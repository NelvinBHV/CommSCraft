# Direct Microsoft normative extraction — SIPAE authenticated-server messages and C3P conferencing

2026-10-22; **documentation only**. Exact publicly accessible section HTML inspected. These HTML pages report update dates 2021-11-16, 2019-02-14 or 2021-08-17; the normative clauses MUST still be crosschecked with the downloadable published MS-SIPAE **11.0** (2025-05-20) and MS-OCSPROT **9.0** (2025-05-20). No assumption of identical revisions, complete protocol support or real-server behavior.

## MS-SIPAE §3.2.5.2 — authenticated messages from the SIP server

**Primary source:** https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c, section 3.2.5.2, HTML last updated **2021-11-16**. Directly inspected requirements, written below in order; client-role instructions rather than invented packet examples.

1. **SA lookup (MUST).** On messages carrying Authentication-Info or Proxy-Authentication-Info, locate an existing security association by authentication protocol, realm and authentication target. The target is unquoted `targetname` for NTLM/TLS-DSK; for Kerberos it is targetname without the `sip/` service descriptor. If no matching SA exists, discard the message and stop. A matching socket or transaction alone is not sufficient.
2. **Ordered signature input (MUST).** Construct a buffer of **angle-bracket-delimited header/parameter values**, preserving syntax and case as received, in this exact order:
   - 1 authentication protocol (`NTLM`, `Kerberos`, `TLS-DSK`);
   - 2 `srand`; 3 `snum`; 4 unquoted `realm`; 5 unquoted `targetname`;
   - 6 `Call-ID`; 7 numeric component of `CSeq`; 8 `CSeq` method;
   - 9 From URI; 10 From tag;
   - 11 To URI **only for authenticating-server protocol version >=3**;
   - 12 To tag;
   - 13 P-Asserted-Identity SIP URI **only for version >=3**;
   - 14 P-Asserted-Identity tel URI **only for version >=3**;
   - 15 Expires; 16 numeric status code **only if a response**.
   For an applicable but absent ordinary field, append `<>`. When a condition is false, **do not append a placeholder**. If condition true and the header absent, append `<>`. Canonicalizing header case/field value before MIC calculation is not permitted merely because SIP headers are semantically case-insensitive.
3. **Verification algorithm (MUST).** Base16-decode `rspauth` (page cites RFC 3548 §6). Verify using GSS_VerifyMic with the SA protocol context for NTLM (MS-NLMP §3.1.4) or Kerberos (RFC 2743 §2.3.2); NTLM SSPI additionally supplies fixed message sequence number **100**. For TLS-DSK derive the HMAC token using the hash function and **server authentication key obtained from the completed TLS-DSK negotiation**, RFC 2104 and the exact constructed buffer. Compare binary MICs. A TLS handshake independent of the protocol security context must not be substituted.
4. **Failed MIC (MUST).** If cryptographic verification fails, discard the message and stop.
5. **Replay check after MIC (MUST).** Reject if `snum` is more than **256 below the highest processed SA sequence number**, or if the sequence has already appeared in the maintained sliding window. If a new maximum is observed, slide the window and record it. Sequence tracking is **per SA**.
6. **SA completion and forbidden responses (MUST).** If verifying an SA not yet established: a verified **403 Forbidden** destroys the SA and triggers a request for different credentials; otherwise transition the SA to **established** and destroy other SAs with the same realm/target.

**Important detail still outstanding:** precise earliest establishment, challenged REGISTER and TLS-DSK key acquisition are in MS-SIPAE §3.2.5.1 and §§2–3; this subsection alone does not define them. The `snum` sliding window should be analyzed alongside transport transaction identity to establish replay lifetime, concurrent responses and reconnect handling. Section wording includes edition footnote 9 on TLS-DSK; its applicability must be traced.

## MS-SIPAE §3.2.6 — SA expiration timer events

**Primary source:** https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/15c359a8-7824-45e7-b8e8-36c48d87b434, section 3.2.6, HTML last updated **2019-02-14**.

- On SA expiration, client **MUST initiate establishment of a new SA** with the authenticating server.
- Client **SHOULD** use REGISTER **without authorization headers**; it **MAY** use another challengeable request (examples INVITE, SUBSCRIBE), and **SHOULD NOT** use ACK or CANCEL for this negotiation.
- On successful new SA, destroy the expired SA. If replacement fails, **SHOULD** preserve expired SA until the transaction's maximum duration; section cites Timer B / Timer F, **32 seconds by default**. Distinguish retry timer, SA expiration, REGISTER binding lifetime and TLS connection lifetime.

This expands sign-in research to long-lived security lifecycle, not merely first successful registration. Exact conditional exception analysis remains pending.

## Inter-spec state reconstruction (interpretation from extracted rules)

```text
SIP message carrying (Proxy-)Authentication-Info
  -> SA lookup by scheme + realm + protocol-adjusted targetname
      not found -> discard
  -> construct ordered signed buffer, preserving raw header value case
  -> verify scheme-specific MIC against SA
      failed -> discard
  -> check per-SA snum duplicate and maximum-256 window
      failed -> discard
  -> SA not established?
        + verified 403 -> destroy, require other credentials
        + otherwise -> mark established, retire competing realm/target SAs
  -> hand off verified result to method-specific registration / feature logic
  -> on SA expiry -> challengeable SA reestablishment (new REGISTER preferred)
```

Only the SA-specific transitions above are drawn directly from §3.2.5.2/§3.2.6. The final handoff to registration/provisioning is an architectural connection requiring MS-SIPREGE clause review. **No code-generated signature test vector is claimed.**

## MS-OCSPROT §2.2.2.2 — C3P and conference dependency inventory

Source: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bf570f65-7429-4df3-8f81-751956ff314f, section 2.2.2.2, HTML last updated **2021-08-17**; not yet checked against rev 9.0 PDF.

| Spec | Published overview responsibility | Graph relationship |
|---|---|---|
| MS-CONFBAS | C3P basic conferencing architecture, signaling, state synchronization | root conference control protocol |
| MS-CONFPRO | Organizer provisions/creates/modifies/deletes conferences | extends CONFBAS |
| MS-CONFIM | Multiparty instant-messaging conferences | extends CONFBAS |
| MS-CONFAV | Multiparty audio/video conferences | extends CONFBAS |
| MS-CONFAS | Application-sharing conferences and RTP use | extends CONFBAS / RFC 3550 |
| MS-XMLMC | SIP INFO XML video control / unused video stream freeze | conferencing media control |

The overview makes a critical architectural distinction: **C3P conference control and state synchronization are separate from actual media transport**. Thus implementability cannot be assessed from SIP INVITE and RTP alone. Further original-body analysis must extract C3P message formats, conference resources, provisioning ACLs, join/leave, focus/session state, AV/IM/application-sharing extensions and media negotiation.

## Source completeness and conflict register

| Item | State |
|---|---|
| MS-SIPAE §3.2.5.2 HTML, field list, algorithm paths, replay, SA state | **Directly inspected (HTML 2021-11-16)** |
| MS-SIPAE §3.2.6 timer handling | **Directly inspected (HTML 2019-02-14)** |
| MS-OCSPROT §2.2.2.2 conference list | **Directly inspected (HTML 2021-08-17)** |
| Equivalence of above HTML to MS-SIPAE rev 11.0 / MS-OCSPROT rev 9.0 | **UNVERIFIED** |
| MS-SIPAE §3.2.5.1 context/key derivation and challenge complete trace | **UNVERIFIED** |
| MS-SIPREGE revision 17.1 detailed provisioning/REGISTER messages | **UNVERIFIED** |
| Conference subdocuments' normative processing rules | **UNVERIFIED** |
| Full client feature matrix, calls, media, presence, recovery | **INCOMPLETE** |

**Repository status:** research file only. No implementations, production traffic, interoperability claims, merges or releases.
