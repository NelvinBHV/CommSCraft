# Discovery → secure transport → authentication → REGISTER: clause analysis v0.1

**Status: PARTIALLY ANALYZED.** Source of each quoted rule is a *live Microsoft Learn section* whose **precise content-to-published-revision binding has not yet been verified against the release PDF**. Landing revisions are independently known: MS-CONMGMT 10.0 (2025-05-20), MS-SIPAE 11.0 (2025-05-20), MS-SIPRE 13.0 (2025-05-20), MS-SIPREGE 17.1 (2025-08-19), MS-OCSPROT 9.0 (2025-05-20). The statements below are explicitly tagged **LIVE-SECTION / PENDING-EDITION-CHECK**, not complete requirements for that revision.

## D-001: Discover candidate outbound SIP proxies

**Source:** [MS-CONMGMT §3.1.5 Message Processing Events and Sequencing Rules](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3), live section; [MS-OCSPROT §2.2.2.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bdcd1606-11c6-4457-a708-3d1c296259a0). 

**Facts (normative language in the section):**

1. Obtain domain from SIP URI (§3.1.4 referenced; its input/normalization rules **not yet examined**); **MUST** query four DNS SRV names **in parallel**: `_sipinternaltls._tcp.<domain>` (TLS); `_sipinternal._tcp.<domain>` (TCP); `_sip._tls.<domain>` (TLS); `_sip._tcp.<domain>` (TCP).
2. Returned SRV records **MUST** be sorted by priority within each query; groups are ordered: internal TLS; DHCP/TLS; internal TCP; DHCP/TCP; external TLS; external TCP. Completion timing can change when returned groups may be emitted (§3.1.5); do not substitute naive "wait for all" without reviewing early completion rules.
3. DHCP SIP discovery is **SHOULD**, uses DHCP INFORM [RFC2131] §4, Option 120 [RFC3361] §3, preferably Option 60 value `MS-UC-Client` [RFC2132] §9.13. Broadcast to 255.255.255.255 is **MUST**; ignore returned IP addresses as directed in context and note the section's distinction between DHCP outputs and address resolution. When DHCP entries lack ports, TLS uses 5061, TCP uses 5060.
4. For SRV TLS targets, client **SHOULD** ignore domains/subdomains outside SIP domain; client **MUST** use the transport indicated by record group. This is not a statement that any TCP fallback is safe under every security policy.
5. If entries were not already included, **SHOULD** append: `sipinternal.<domain>:443` TLS, `sipinternal.<domain>` TCP, `sip.<domain>:443` TLS, `sip.<domain>` TCP, `sipexternal.<domain>:443` TLS, `sipexternal.<domain>` TCP.
6. Sequential candidate connection attempts and fresh A lookup before each entry are **SHOULD**. DNS A lookup failure or TCP failure for all candidate IPs leads to next entry (**SHOULD**); **all other failures** or exhausted entries → discovery failure (**MUST**). Record this separation: a TLS certificate validation failure is **not** equivalent to "TCP could not connect"; do not silently select plaintext as a supposed recovery.

**Cross-spec dependencies:** RFC 2782 SRV ordering/weights (algorithm not yet verified here), DNS RFCs 1034/1035, DHCP RFCs 2131/2132/3361, MS-CONMGMT §§3.1.4/3.1.6 and TLS certificate sections (OPEN), RFC 3263 only where applicable (do not impose IETF standard discovery in preference to the explicit Microsoft sequence).

**Security checkpoint:** DNS data is not an identity proof. A candidate endpoint must not be promoted to trusted authenticated proxy until connection and identity checks pass. Certificate SAN / FQDN binding, redirects, TLS version/ciphers, chain/revocation and policy variation require MS-CONMGMT certificate clauses + current X.509/TLS authoritative review. Do not assert a specific SAN grammar or TLS version here.

**Failure branches:** DHCP timeout/unavailable, empty SRV sets, duplicate or out-of-domain results, resolution failure, TCP connect failure, TLS/security failure, all candidates exhausted. Every branch's timeout/retry budget remains OPEN until full state/timer sections are evaluated.

## R-001: Authenticated registration handshake (overview)

**Source:** [MS-OCSPROT §2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/45e475a0-b534-469a-892e-6bd6ae42fc6e), live subsection (scenario, **not** exhaustive state machine). Prerequisite: protocol client already connected.

**FACT:** Client sends REGISTER requesting GRUU, enhanced-presence resource-list/category capabilities and keep-alive capability. Server responds with authentication challenge such as **401** or **407**, offers Kerberos or NTLM in this *example use case*; client sends a **new REGISTER** containing the selected authentication token according to MS-SIPAE. Server validates token and may return generated GRUU, keep-alive support, encrypted server-authenticity proof and presence synchronization hints.

**Important scope limit:** The scenario's illustrative offer of Kerberos/NTLM does **not** prove TLS-DSK or alternative supported workflows are impossible. MS-SIPAE published landing and MS-OCSPROT §2.2.2.1 also identify TLS-DSK, which requires separate full investigation. A 200 response alone cannot be equated with server-authenticated sign-in; SA proof, transaction binding and trusted transport are separate checks. Current CommSCraft docs describe that distinction, but are **not independent protocol evidence**.

## R-002: Registrar's specific validation and endpoint binding

**Source:** [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010), live subsection; crosses [MS-SIPAE §3.2] and [RFC3261 §§8.2.2, 10.3](https://www.rfc-editor.org/info/rfc3261).

**FACTS (from **registrar**, not necessarily a client's sending obligations):**

| Condition/event | Explicit result/processing | Required further examination |
| --- | --- | --- |
| New / refresh REGISTER | Registrar **MUST** establish and maintain NTLM or Kerberos SA with UAC according to MS-SIPAE §3.2; validate REGISTER by MS-SIPREGE §2.2.1.1 | Request/header grammar, SA lifetimes, product behaviors |
| Missing Epid and SIP.INSTANCE | Example special response **400**, ErrorId **4010** | Exact trigger conjunction, ms-diagnostics format MS-OCER §2.2.1 |
| Event header not `registration` | **489**, ErrorId **4055** | All acceptable Event/Supported combinations |
| `msrtc-event-categories` without `gruu-10` in Supported | **421**, ErrorId **2057** | Negotiation versions, capability fallback policy |
| New endpoint identity | Registrar uses Epid and SIP.INSTANCE and generates GRUU according to MS-SIPRE §2.2.3 | Required attribute formats and GRUU trust |
| Existing endpoint identity | Registrar handles REGISTER refresh per RFC 3261 §10.3; SIP.INSTANCE is primary key for replacing bindings | Expiry policy, collision and concurrency |
| Server absence of Presence Service, UAC not survivable | Registrar SHOULD reject **503**, public ErrorId **4164** | Product behavior notes 158–163 |
| Accepted REGISTER | **200 OK** per RFC 3261 §10.3, GRUU per the **draft-ietf-sip-gruu-10 §8.2** reference; Presence-State header SHOULD be set | GRUU quoting, Contact data, signed fields, exact 200 body |
| Presence-State / Register-Action | For new endpoint `added`; newly reroutable endpoint `fixed`; refreshed endpoint `refreshed` | Actual client response transitions |
| Survivable UAC, Presence-Service-Available true→false | `NOTIFY` `userservices-unavailable` SHOULD be generated | §2.2.1.13 details and event subscription |
| Optional device report | Ms-Device-Info MAC/version/vendor SHOULD be read | Privacy/data minimization |

**Normative precision:** Microsoft uses SHOULD on multiple server responses; avoid treating optional fields as guaranteed on every deployment. Error codes above are from explicitly enumerated cases, **not a complete error taxonomy**.

## A-001: Security association and trust boundaries (work breakdown)

- [ ] **MS-SIPAE rev 11.0** §§2–3: exact headers/ABNF, NTLM/Kerberos/TLS-DSK negotiation, all versions, tokens, GSS/SSPI context completion, message canonicalization/MIC, anti-replay, key usage, clock/sequence and all failure transitions.
- [ ] **MS-OCAUTHWS**: certificate enrollment/authentication web-service interactions; compare user credential + certificate paths and enrollment trust assumptions.
- [ ] **MS-KILE/MS-NLMP and RFC 4120/2743**: credential acquisition, SPN/service identity, mutual-auth semantics, token signing, NTLM attack surface, connection binding.
- [ ] **MS-CONMGMT rev 10.0**: TLS certificate field constraints, validation including target-identity precedence, persistent transport, keep-alive negotiation/timers/retry, disconnect and reconnect.
- [ ] **MS-SIPREGE rev 17.1**: all client behavior, 423/Min-Expires handling, sip.instance / epid / GRUU, provisioned configuration payloads, expiration/unregistration and server-authentication proof.
- [ ] **Crossed versions**: MS-SIPREGE 17.0 vs 17.1 substantive difference, 2024 vs 2025 MS-CONMGMT/SIPAE, deployed SfB 2019/SE policy variants.

### Minimal proposed state labels — **INFERENCE**, not normative wire spec

`START → CANDIDATES → TRANSPORT_CONNECTING → TLS_PEER_VERIFIED → SIP_UNAUTHENTICATED → AUTH_NEGOTIATING → SA_VERIFIED → REGISTER_ACCEPTED → PROVISIONING → READY`.

`REGISTER_ACCEPTED` is never promoted from an unverified final SIP message; validation states must distinguish signed/authenticated/provisioned/usable. Error paths return to `FAILED_LOCKED` or `REAUTH_REQUIRED` according to *future* clause review, not assumed automatic retry. See [flows-and-states.md](flows-and-states.md).
