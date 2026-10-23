# Authenticated registration → client bootstrap: directly verified 2025 Microsoft cross-spec model

**Research tranche:** 2026-10-22. **Primary originals directly reviewed:** owner-provided `catalog/MS-OCSPROT.pdf` / text, **rev 9.0, 2025-05-20, §§2.5.1–2.5.3, PDF pp.27–32**; `catalog/MS-SIPREGE.pdf` / text, **rev 17.1, 2025-08-19, §§2.2.2.1–5, 3.3.1–3.3.2, PDF pp.19–25, 82–83**. Official links: [MS-OCSPROT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c), [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d). Original PDF texts were inspected, not only assessment summaries or historical HTML.

## 1. MS-OCSPROT §2.5.1–3: three distinct use-case boundaries

- **§2.5.1, Fig. 6, pp.27–28:** discovery/connection consumes DNS SRV per MS-CONMGMT; selected hostname/port then transport connection; TLS verification is described as optional in the overview (profile-specific TLS policy must be analyzed in MS-CONMGMT and RFC 5922). Compression after TLS is subject to group policy and MS-SIPCOMP. Postcondition: **connected and ready to sign in**, NOT authenticated.
- **§2.5.2, Fig. 7, pp.28–30:** REGISTER requests GRUU and advertised capabilities (resource-list enhanced presence, presence XML, keepalive). Server challenges with 401/407 (Kerberos/NTLM examples), client responds with authentication token in another REGISTER, server validates token and can return endpoint GRUU, keepalive support, protected server authenticity and presence synchronization. Postcondition: **authenticated and registered**. Section describes a use case, NOT every negotiated mechanism; MS-SIPAE independently defines TLS-DSK and anonymous conference Digest branches.
- **§2.5.3, Fig. 8, pp.30–32:** *after authentication and registration*, client bootstrap requests configuration, contacts/groups, own presence, location, MCU discovery, AV Edge credentials, publishes own presence and subscribes to contacts. Postcondition: **bootstrapped and ready for presence updates and communications**.
- **Crucial clause at end of §2.5.3:** Microsoft expressly allows the information-fetch order to vary; the numbered list is **not a universal mandated chronological transcript**. Treat each data prerequisite, feature guard and authentication boundary explicitly.

## 2. MS-OCSPROT §2.5.3: complete 16-step use-case ledger, role-qualified

| Step | Client request / server response | Governing documents | Scope / conditional caveat |
|---|---|---|---|
| 1–2 | SUBSCRIBE for in-band provisioning; server returns 200 and configuration/policies/ABS/group-expansion URL/desktop-phone settings | MS-SIPREGE | high-level overview assumes response carries provisioning; detailed §2.2.2.2 alternatively permits NOTIFY |
| 3–4 | SUBSCRIBE for Contacts and Groups; response with contact list/groups | MS-PRES, MS-SIPREGE | separate group/contact data from provisioning policy |
| 5–6 | SUBSCRIBE for user's own enhanced presence (contact card, calendar); server response | MS-PRES, MS-SIPREGE | independent state from contacts and endpoint auth |
| 7–8 | SERVICE for VoIP location profile; response | MS-SIPRE, MS-SIPREGE | example assumes EnhancedEmergencyServices and LocationRequired settings |
| 9–10 | SERVICE for MCU/conference-server discovery; response | MS-SIPREGE | not the same as joining an actual conference |
| 11–12 | SERVICE for Media Relay authentication tokens; response | MS-AVEDGEA | only specified example when AV Edge configured and ICE/ICE2 supported; other topologies differ |
| 13–14 | SERVICE to publish own presence; 200 acknowledgment | MS-PRES | policy- and registration-dependent |
| 15–16 | Batch SUBSCRIBE for enhanced presence of contacts; 200 acknowledgment | MS-SIPREGE, MS-PRES | contact set obtained in step 4 |

The overview's request/response simplifications must be reconciled with normative underlying SIP subscriptions (in particular early NOTIFY, `Expires: 0`, and piggyback branches); the overview does not authorize treating SIP 200 as the complete content/authorization semantics of all request types.

## 3. MS-SIPREGE §2.2.2.1: exact in-band provisioning request profile

Client SUBSCRIBE for `Event: vnd-microsoft-provisioning-v2`, `Accept: application/vnd-microsoft-roaming-provisioning-v2+xml`, `Content-Type: application/vnd-microsoft-roaming-provisioning-v2+xml`, `Expires: 0` (no subscription dialog), no Require header, XML `provisioningGroupList`. To URI is user's AoR; From matches on self-request. Delegated request uses delegator To, delegate From and **MUST** include p-session-on-behalf-of matching the delegator; MS-SIPAE authorization applies. Server-entity variant has special FQDN/no-user rules. Optional Supported headers follow Microsoft extensions. Source references RFC 3265 §4.4.3.

**Do not generalize `Expires: 0` to all SUBSCRIBE methods**; this is the *specific* no-dialog in-band provisioning use case.

## 4. MS-SIPREGE §2.2.2.2: resolved provisioning placement fork

This directly settles an ambiguity in older research:

- If the UAC **advertises `ms-piggyback-first-notify`** (MS-SIP §3.4.4.1), the server includes the configuration XML **in the SUBSCRIBE 200 response**.
- Otherwise the configuration is delivered in a **NOTIFY request**.
- In either path the Microsoft `Event`, XML media type, `Expires: 0`, and response body schema are specified in §2.2.2.2; `provisionGroupList/provisionGroup` is the **response** schema (§2.2.2.5), distinct from the request schema (§2.2.2.4).
- The base **RFC 3265** (obsolete under later RFC 6665) is cited by the published 2025 Microsoft profile; replacement requires version-compatibility review, not automatic rewriting.

```mermaid
sequenceDiagram
    participant C as SIP client
    participant R as provisioning server
    Note over C: prerequisite: authenticated and registered per MS-OCSPROT 2.5.2
    C->>R: SUBSCRIBE Event: provisioning-v2, Expires: 0, requested provisioningGroupList
    alt UAC advertises ms-piggyback-first-notify
        R-->>C: 200 SUBSCRIBE containing provisionGroupList XML
    else UAC does not advertise extension
        R-->>C: SIP response; provisioning XML in NOTIFY
        R->>C: NOTIFY Event: provisioning-v2, provisionGroupList XML
        Note over C,R: actual response/NOTIFY ordering, subscription transaction rules need RFC3265 analysis
    end
    Note over C: Validate configuration schema; apply policy per feature before using services
```

**Diagram status:** data placement is **directly normative from §2.2.2.2**, overall client lifecycle from MS-OCSPROT; not a byte-exact transcript or proof of 2025 Windows Full Client negotiated capabilities.

## 5. Cross-document identity and policy boundaries

1. **Connection complete** (§2.5.1) ≠ **REGISTER authenticated/registered** (§2.5.2) ≠ **Bootstrap complete** (§2.5.3). The overview uses different pre/post-conditions.
2. Bootstrap requires independently managed states: provisioning policy, contacts/groups, own presence subscription, location policy, conference service discovery, edge credential acquisition, published presence and batch subscriptions.
3. **ICE/ICE2 and AV Edge** tokens are conditional. A general client state machine must represent absence of edge server and unsupported ICE as alternative paths rather than unconditional failure.
4. **Policy gate**: SIP protocol success does not alone imply the feature is permitted. A location request in the overview has explicit EnhancedEmergencyServices/LocationRequired guard.
5. **Microsoft extension precedence:** MS-OCSPROT's simplified 200-with-body step is not a contradiction with MS-SIPREGE's 200-vs-NOTIFY fork; MS-OCSPROT is an overview use case, while §2.2.2.2 is the processing-specific protocol requirement.

## 6. Evidence and unresolved research register

**Direct original text checked:** MS-OCSPROT rev 9.0 §§2.5.1–2.5.3 including all 16 steps and explicitly flexible ordering; MS-SIPREGE rev 17.1 §§2.2.2.1–2.2.2.2 and referenced §§2.2.2.4–5 (request and response data schema), §3.3 overview.

**Still open:** complete MS-SIP normative `ms-piggyback-first-notify` grammar (§3.4.4.1), RFC 3265/6665 event handling and response/NOTIFY races; entire presence/contacts, ICE, conference/MCU, E911, address book and provisioning XML schemas (incl. per-feature optional/policy rules); privacy/delegation, authenticator binding and inter-version behavior. The 112 user-supplied assessment reports have not been uniformly independently audited.

No implementation changes, real-server tests, main merge or release. **Global RFC/MS protocol corpus analysis NOT COMPLETE.**
