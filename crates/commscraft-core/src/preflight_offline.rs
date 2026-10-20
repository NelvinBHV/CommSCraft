//! Offline preflight-only models. No credentials, TLS handshakes, network I/O or persistence.
const MAX: usize = 64;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError { Invalid, Duplicate, Capacity, Missing }
fn label_ok(s: &str) -> bool { !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control) }
fn host_ok(s: &str) -> bool {
    s.len() <= 253 && !s.is_empty() && s.split('.').all(|part|
        !part.is_empty() && part.len() <= 63 && !part.starts_with('-') && !part.ends_with('-')
        && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}
#[derive(Debug, Default)]
pub struct ProvisioningPreview { host: Option<String> }
impl ProvisioningPreview {
    pub fn stage_host(&mut self, host: &str) -> Result<(), DraftError> {
        if !host_ok(host) { return Err(DraftError::Invalid); }
        self.host = Some(host.to_ascii_lowercase()); Ok(())
    }
    pub fn host(&self) -> Option<&str> { self.host.as_deref() }
}
#[derive(Debug, Default)]
pub struct CertificateMetadataPreview { fingerprints: Vec<String> }
impl CertificateMetadataPreview {
    pub fn stage_sha256_hex(&mut self, fingerprint: &str) -> Result<(), DraftError> {
        if fingerprint.len() != 64 || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(DraftError::Invalid); }
        if self.fingerprints.iter().any(|x| x.eq_ignore_ascii_case(fingerprint)) { return Err(DraftError::Duplicate); }
        if self.fingerprints.len() >= MAX { return Err(DraftError::Capacity); }
        self.fingerprints.push(fingerprint.to_ascii_lowercase()); Ok(())
    }
    pub fn count(&self) -> usize { self.fingerprints.len() }
}
#[derive(Debug, Default)]
pub struct FederationDomainPreview { domains: Vec<String> }
impl FederationDomainPreview {
    pub fn stage(&mut self, domain: &str) -> Result<(), DraftError> {
        if !host_ok(domain) || !domain.contains('.') { return Err(DraftError::Invalid); }
        if self.domains.iter().any(|x| x.eq_ignore_ascii_case(domain)) { return Err(DraftError::Duplicate); }
        if self.domains.len() >= MAX { return Err(DraftError::Capacity); }
        self.domains.push(domain.to_ascii_lowercase()); Ok(())
    }
    pub fn domains(&self) -> &[String] { &self.domains }
}
#[derive(Debug, Default)]
pub struct PersistentChatLocalPreview { messages: Vec<String> }
impl PersistentChatLocalPreview {
    pub fn push(&mut self, body: &str) -> Result<(), DraftError> {
        if !label_ok(body) { return Err(DraftError::Invalid); }
        if self.messages.len() >= MAX { return Err(DraftError::Capacity); }
        self.messages.push(body.into()); Ok(())
    }
    pub fn clear(&mut self) { self.messages.clear(); }
    pub fn messages(&self) -> &[String] { &self.messages }
}
#[derive(Debug, Default)]
pub struct PackagingManifestPreview { entries: Vec<String> }
impl PackagingManifestPreview {
    pub fn stage_name(&mut self, name: &str) -> Result<(), DraftError> {
        if !label_ok(name) || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
            return Err(DraftError::Invalid);
        }
        if self.entries.iter().any(|x| x == name) { return Err(DraftError::Duplicate); }
        if self.entries.len() >= MAX { return Err(DraftError::Capacity); }
        self.entries.push(name.into()); Ok(())
    }
    pub fn entries(&self) -> &[String] { &self.entries }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn provisioning_rejects_invalid_host_and_normalizes() {
        let mut p = ProvisioningPreview::default();
        assert_eq!(p.stage_host("SIP.Example.test"), Ok(()));
        assert_eq!(p.host(), Some("sip.example.test"));
        assert_eq!(p.stage_host("bad..example.test"), Err(DraftError::Invalid));
        assert_eq!(p.stage_host("good.example.test\r"), Err(DraftError::Invalid));
    }
    #[test] fn certificates_reject_bad_sha256_and_duplicate() {
        let mut p = CertificateMetadataPreview::default();
        let hex = "a".repeat(64);
        assert_eq!(p.stage_sha256_hex(&hex), Ok(()));
        assert_eq!(p.stage_sha256_hex(&hex.to_uppercase()), Err(DraftError::Duplicate));
        assert_eq!(p.stage_sha256_hex("not-a-hash"), Err(DraftError::Invalid));
        assert_eq!(p.count(), 1);
    }
    #[test] fn federation_rejects_duplicate_and_bad_domains() {
        let mut p = FederationDomainPreview::default();
        assert_eq!(p.stage("Example.test"), Ok(()));
        assert_eq!(p.stage("example.test"), Err(DraftError::Duplicate));
        assert_eq!(p.stage("-bad.example.test"), Err(DraftError::Invalid));
        assert_eq!(p.domains(), &["example.test"]);
    }
    #[test] fn persistent_chat_is_bounded_and_wipeable() {
        let mut p = PersistentChatLocalPreview::default();
        for _ in 0..MAX { assert_eq!(p.push("hello"), Ok(())); }
        assert_eq!(p.push("overflow"), Err(DraftError::Capacity));
        assert_eq!(p.push("bad\n"), Err(DraftError::Invalid));
        p.clear(); assert!(p.messages().is_empty());
    }
    #[test] fn package_names_reject_paths_and_duplicates() {
        let mut p = PackagingManifestPreview::default();
        assert_eq!(p.stage_name("client_x64"), Ok(()));
        assert_eq!(p.stage_name("../secret"), Err(DraftError::Invalid));
        assert_eq!(p.stage_name("client_x64"), Err(DraftError::Duplicate));
        assert_eq!(p.entries(), &["client_x64"]);
    }
}
