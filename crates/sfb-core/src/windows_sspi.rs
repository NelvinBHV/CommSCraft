//! Windows Kerberos SSPI token source, NOT a Skype for Business login.
//!
//! Uses only the current Windows logon session's Kerberos credential. Tokens
//! are opaque. Never falls back to Negotiate/NTLM, never prompts for passwords.
//! The application must independently verify MS-SIPAE server Authentication-Info
//! MIC before declaring REGISTER successful.
//!
//! References: Microsoft InitializeSecurityContext (Kerberos),
//! AcquireCredentialsHandle (Kerberos); MS-SIPAE 3.2.5.1; RFC 2743, RFC 4121.
#![cfg(windows)]

use crate::kerberos::{KerberosError, KerberosProvider, KerberosStep, MAX_GSS_TOKEN};
use crate::kerberos_mic::{KerberosMicBackend, KerberosMicError, MAX_MIC_INPUT, MAX_MIC_TOKEN};
use std::ffi::c_void;
use std::fmt;
use std::ptr::{null, null_mut};

const SEC_E_OK: i32 = 0;
const SEC_I_CONTINUE_NEEDED: i32 = 0x0009_0312;
const SEC_I_COMPLETE_NEEDED: i32 = 0x0009_0313;
const SEC_I_COMPLETE_AND_CONTINUE: i32 = 0x0009_0314;
const SECPKG_CRED_OUTBOUND: u32 = 2;
const ISC_REQ_MUTUAL_AUTH: u32 = 0x0000_0002;
const ISC_REQ_REPLAY_DETECT: u32 = 0x0000_0004;
const ISC_REQ_SEQUENCE_DETECT: u32 = 0x0000_0008;
const ISC_REQ_INTEGRITY: u32 = 0x0001_0000;
const ISC_REQ_IDENTIFY: u32 = 0x0002_0000;
const REQUIRE: u32 = ISC_REQ_MUTUAL_AUTH | ISC_REQ_REPLAY_DETECT |
    ISC_REQ_SEQUENCE_DETECT | ISC_REQ_INTEGRITY | ISC_REQ_IDENTIFY;
const SECBUFFER_VERSION: u32 = 0;
const SECBUFFER_TOKEN: u32 = 2;
const SECBUFFER_DATA: u32 = 1;
const SECPKG_ATTR_SIZES: u32 = 0;
const SECURITY_NATIVE_DREP: u32 = 0x10;
const MAX_NATIVE_TOKEN: usize = 64 * 1024;

#[repr(C)]
#[derive(Default)]
struct SecHandle { lower: usize, upper: usize }
#[repr(C)]
struct SecBuffer { size: u32, kind: u32, data: *mut c_void }
#[repr(C)]
struct SecBufferDesc { version: u32, count: u32, buffers: *mut SecBuffer }
#[repr(C)]
#[derive(Default)]
struct SecPkgContextSizes {
    max_token: u32, max_signature: u32, block_size: u32, security_trailer: u32,
}

#[link(name = "Secur32")]
unsafe extern "system" {
    fn AcquireCredentialsHandleW(principal: *const u16, package: *const u16,
        usage: u32, logon: *const c_void, auth: *const c_void,
        get_key: *const c_void, key_arg: *const c_void,
        credential: *mut SecHandle, expiry: *mut i64) -> i32;
    fn FreeCredentialsHandle(credential: *mut SecHandle) -> i32;
    fn InitializeSecurityContextW(credential: *mut SecHandle, old_context: *mut SecHandle,
        target: *mut u16, flags: u32, reserved: u32, data_rep: u32,
        input: *mut SecBufferDesc, reserved2: u32, new_context: *mut SecHandle,
        output: *mut SecBufferDesc, attributes: *mut u32, expiry: *mut i64) -> i32;
    fn CompleteAuthToken(context: *mut SecHandle, token: *mut SecBufferDesc) -> i32;
    fn DeleteSecurityContext(context: *mut SecHandle) -> i32;
    fn QueryContextAttributesW(context: *mut SecHandle, attribute: u32,
        output: *mut c_void) -> i32;
    fn MakeSignature(context: *mut SecHandle, qop: u32,
        message: *mut SecBufferDesc, sequence: u32) -> i32;
    fn VerifySignature(context: *mut SecHandle, message: *mut SecBufferDesc,
        sequence: u32, qop: *mut u32) -> i32;
}

/// All values are deliberately opaque; `Debug` never prints an SSPI token,
/// service principal or a native pointer. Native handles are RAII-owned.
pub struct WindowsKerberosProvider {
    credential: SecHandle,
    context: SecHandle,
    has_credential: bool,
    has_context: bool,
    pinned_spn: Option<String>,
    finished: bool,
    failed: bool,
}
impl fmt::Debug for WindowsKerberosProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowsKerberosProvider")
            .field("has_credential", &self.has_credential)
            .field("has_context", &self.has_context)
            .field("finished", &self.finished)
            .field("failed", &self.failed)
            .finish()
    }
}
impl WindowsKerberosProvider {
    /// Obtain the current user's Kerberos credential handle from SSPI.
    /// This does NOT authenticate a remote server or transmit credentials.
    pub fn new() -> Result<Self, KerberosError> {
        let mut provider = Self {
            credential: SecHandle::default(), context: SecHandle::default(),
            has_credential: false, has_context: false, pinned_spn: None,
            finished: false, failed: false,
        };
        let package: Vec<u16> = "Kerberos\0".encode_utf16().collect();
        let mut expiry = 0_i64;
        // SAFETY: The package string is NUL terminated; the output handles
        // and expiry are initialized and owned by this object. Null args ask
        // SSPI for the existing interactive logon-session credentials.
        let result = unsafe { AcquireCredentialsHandleW(null(), package.as_ptr(),
            SECPKG_CRED_OUTBOUND, null(), null(), null(), null(),
            &mut provider.credential, &mut expiry) };
        if result != SEC_E_OK { return Err(KerberosError::ProviderUnavailable); }
        provider.has_credential = true;
        Ok(provider)
    }

    /// Consume a genuine server GSS context token from a validated, pinned
    /// SIP transaction. Caller MUST NOT invoke this for arbitrary network data.
    /// Passing None starts a context; Some continues an existing context.
    /// The function itself never changes an application "logged in" status.
    pub fn step(&mut self, trusted_spn: &str, server_token: Option<&[u8]>)
        -> Result<KerberosStep, KerberosError>
    {
        if self.failed || self.finished || !self.has_credential ||
            crate::kerberos::KerberosNegotiation::new(trusted_spn).is_err() {
            return Err(KerberosError::State);
        }
        if self.pinned_spn.as_deref().is_some_and(|pinned| pinned != trusted_spn) {
            self.failed = true;
            return Err(KerberosError::UntrustedChallenge);
        }
        if self.has_context != server_token.is_some() {
            return Err(KerberosError::State);
        }
        if server_token.is_some_and(|t| t.is_empty() || t.len() > MAX_GSS_TOKEN) {
            self.failed = true;
            return Err(KerberosError::InvalidToken);
        }
        self.pinned_spn.get_or_insert_with(|| trusted_spn.to_owned());
        let mut target: Vec<u16> = trusted_spn.encode_utf16().chain(std::iter::once(0)).collect();
        let mut outgoing = vec![0_u8; MAX_NATIVE_TOKEN];
        let mut out_buffer = SecBuffer {
            size: outgoing.len() as u32, kind: SECBUFFER_TOKEN,
            data: outgoing.as_mut_ptr().cast(),
        };
        let mut out_desc = SecBufferDesc {
            version: SECBUFFER_VERSION, count: 1, buffers: &mut out_buffer,
        };
        let mut inbound = server_token.unwrap_or_default().to_vec();
        let mut in_buffer = SecBuffer {
            size: inbound.len() as u32, kind: SECBUFFER_TOKEN,
            data: inbound.as_mut_ptr().cast(),
        };
        let mut in_desc = SecBufferDesc {
            version: SECBUFFER_VERSION, count: 1, buffers: &mut in_buffer,
        };
        let old_context = if self.has_context { &mut self.context as *mut SecHandle }
            else { null_mut() };
        let input_desc = if server_token.is_some() { &mut in_desc as *mut SecBufferDesc }
            else { null_mut() };
        let mut attrs = 0_u32;
        let mut expiry = 0_i64;
        // SAFETY: All descriptors/buffers remain alive and writable throughout
        // the call; the output buffer is preallocated, so SSPI does not own it.
        // On subsequent steps SSPI explicitly permits the same old/new context
        // handle. Every owned handle is freed once in Drop.
        let status = unsafe { InitializeSecurityContextW(
            &mut self.credential, old_context, target.as_mut_ptr(), REQUIRE, 0,
            SECURITY_NATIVE_DREP, input_desc, 0, &mut self.context,
            &mut out_desc, &mut attrs, &mut expiry,
        ) };
        // Some unsuccessful SSPI calls leave an allocated security context;
        // retaining it for RAII cleanup is safer than losing its handle.
        self.has_context = self.context.lower != 0 || self.context.upper != 0;
        if matches!(status, SEC_I_COMPLETE_NEEDED | SEC_I_COMPLETE_AND_CONTINUE) {
            // SAFETY: SSPI owns the context; token descriptor is still live.
            let complete = unsafe { CompleteAuthToken(&mut self.context, &mut out_desc) };
            if complete != SEC_E_OK { self.failed = true; return Err(KerberosError::ProviderFailed); }
        } else if !matches!(status, SEC_E_OK | SEC_I_CONTINUE_NEEDED) {
            self.failed = true;
            return Err(KerberosError::ProviderFailed);
        }
        if out_buffer.size as usize > outgoing.len() ||
            out_buffer.size as usize > MAX_GSS_TOKEN ||
            (attrs & ISC_REQ_INTEGRITY) == 0 ||
            (attrs & ISC_REQ_IDENTIFY) == 0 {
            self.failed = true;
            return Err(KerberosError::ProviderFailed);
        }
        let complete = matches!(status, SEC_E_OK | SEC_I_COMPLETE_NEEDED);
        if complete && (attrs & ISC_REQ_MUTUAL_AUTH) == 0 {
            self.failed = true;
            return Err(KerberosError::ProviderFailed);
        }
        if out_buffer.size == 0 && !complete {
            self.failed = true;
            return Err(KerberosError::InvalidToken);
        }
        outgoing.truncate(out_buffer.size as usize);
        self.finished = complete;
        Ok(KerberosStep { token: outgoing, complete, integrity: true })
    }

    pub fn context_complete(&self) -> bool { self.finished && !self.failed }
}
impl WindowsKerberosProvider {
    fn mic_signature_capacity(&mut self) -> Result<usize, KerberosMicError> {
        if !self.context_complete() || !self.has_context || self.pinned_spn.is_none() {
            return Err(KerberosMicError::IncompleteContext);
        }
        let mut sizes = SecPkgContextSizes::default();
        // SAFETY: A completed, owned SSPI context and a C-layout output buffer.
        let status = unsafe { QueryContextAttributesW(&mut self.context,
            SECPKG_ATTR_SIZES, (&mut sizes as *mut SecPkgContextSizes).cast()) };
        if status != SEC_E_OK || sizes.max_signature == 0 ||
            sizes.max_signature as usize > MAX_MIC_TOKEN {
            return Err(KerberosMicError::NativeFailure);
        }
        Ok(sizes.max_signature as usize)
    }

    fn native_get_mic(&mut self, message: &[u8]) -> Result<Vec<u8>, KerberosMicError> {
        if message.is_empty() || message.len() > MAX_MIC_INPUT {
            return Err(KerberosMicError::InvalidInput);
        }
        let capacity = self.mic_signature_capacity()?;
        // SSPI descriptors use mutable memory. A copy avoids casting away
        // const on caller-owned message bytes and limits the native boundary.
        let mut signed = message.to_vec();
        let mut token = vec![0_u8; capacity];
        let mut buffers = [
            SecBuffer { size: signed.len() as u32, kind: SECBUFFER_DATA,
                data: signed.as_mut_ptr().cast() },
            SecBuffer { size: token.len() as u32, kind: SECBUFFER_TOKEN,
                data: token.as_mut_ptr().cast() },
        ];
        let mut desc = SecBufferDesc { version: SECBUFFER_VERSION,
            count: buffers.len() as u32, buffers: buffers.as_mut_ptr() };
        // The SIPAE cnum/snum is authenticated inside the MS-SIPAE MIC buffer.
        // The transport does not separately supply SSPI sequence numbers.
        // SAFETY: Writable C buffers live for the duration of this call.
        let status = unsafe { MakeSignature(&mut self.context, 0, &mut desc, 0) };
        if status != SEC_E_OK || buffers[1].size == 0 ||
            buffers[1].size as usize > capacity ||
            buffers[0].size as usize != message.len() || signed != message {
            return Err(KerberosMicError::NativeFailure);
        }
        token.truncate(buffers[1].size as usize);
        Ok(token)
    }

    fn native_verify_mic(&mut self, message: &[u8], token: &[u8])
        -> Result<(), KerberosMicError>
    {
        if message.is_empty() || message.len() > MAX_MIC_INPUT ||
            token.is_empty() || token.len() > MAX_MIC_TOKEN {
            return Err(KerberosMicError::InvalidInput);
        }
        let capacity = self.mic_signature_capacity()?;
        if token.len() > capacity { return Err(KerberosMicError::InvalidInput); }
        let mut signed = message.to_vec();
        let mut token_copy = token.to_vec();
        let mut buffers = [
            SecBuffer { size: signed.len() as u32, kind: SECBUFFER_DATA,
                data: signed.as_mut_ptr().cast() },
            SecBuffer { size: token_copy.len() as u32, kind: SECBUFFER_TOKEN,
                data: token_copy.as_mut_ptr().cast() },
        ];
        let mut desc = SecBufferDesc { version: SECBUFFER_VERSION,
            count: buffers.len() as u32, buffers: buffers.as_mut_ptr() };
        let mut qop = 0_u32;
        // SAFETY: A complete SSPI context, bounded mutable input buffers,
        // and an initialized QOP out parameter. No raw material is logged.
        let status = unsafe { VerifySignature(&mut self.context, &mut desc, 0, &mut qop) };
        if status != SEC_E_OK || qop != 0 || signed != message {
            return Err(KerberosMicError::NativeFailure);
        }
        Ok(())
    }
}

impl KerberosMicBackend for WindowsKerberosProvider {
    fn complete(&self) -> bool { self.context_complete() }
    fn target_spn(&self) -> Option<&str> { self.pinned_spn.as_deref() }
    fn get_mic(&mut self, message: &[u8]) -> Result<Vec<u8>, KerberosMicError> {
        self.native_get_mic(message)
    }
    fn verify_mic(&mut self, message: &[u8], token: &[u8]) -> Result<(), KerberosMicError> {
        self.native_verify_mic(message, token)
    }
}

impl KerberosProvider for WindowsKerberosProvider {
    fn initiate(&mut self, trusted_spn: &str) -> Result<KerberosStep, KerberosError> {
        self.step(trusted_spn, None)
    }
}
impl Drop for WindowsKerberosProvider {
    fn drop(&mut self) {
        // SAFETY: These are the only owners of their respective SSPI handles.
        // Free the context before the credential. No tokens are logged.
        unsafe {
            if self.has_context { let _ = DeleteSecurityContext(&mut self.context); }
            if self.has_credential { let _ = FreeCredentialsHandle(&mut self.credential); }
        }
        if let Some(spn) = &mut self.pinned_spn { unsafe {
            let bytes = spn.as_mut_vec();
            for byte in bytes { std::ptr::write_volatile(byte, 0); }
        } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sspi_token_memory_layout_matches_c() {
        assert_eq!(std::mem::size_of::<SecHandle>(), 2 * std::mem::size_of::<usize>());
        assert_eq!(std::mem::size_of::<SecBuffer>(), if cfg!(target_pointer_width="64") {16} else {12});
        assert_eq!(std::mem::size_of::<SecPkgContextSizes>(), 16);
    }
}
