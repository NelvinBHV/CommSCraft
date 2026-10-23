# MS-SIPREGE 17.1 – direct PDF registration, provisioning and registrar lifecycle audit

**Research 2026-10-22.** Directly checked owner-supplied original `catalog/MS-SIPREGE.pdf` with its extracted `catalog/MS-SIPREGE.txt`, version banner **v20250819, 134 pages**, revision **17.1, published 2025-08-19**. Official original: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d. The matching assessment is `assessments/microsoft-sip-registration-provisioning.md` and is treated as a hypothesis source, not independent proof. No implementation or real-server tests.

## Directly checked primary clauses

| Requirement ID | Direct original section | Evidence and interpretation | Status |
|---|---|---|---|
| REGE17-01 | §2.2.2.4, pp.24–25 | Provisioning **request** XSD uses **`provisioningGroupList`** root with **`provisioningGroup`** children. Note XSD namespace and case are normative identity, not an interchangeable XML tag spelling. | checked PDF text |
| REGE17-02 | §2.2.2.5, pp.25ff | Provisioning **response** uses **`provisionGroupList`** root with **`provisionGroup`** children; source points to preceding XSD and enumerates group names, including `ServerConfiguration`, `meetingPolicy`, `ucPolicy`, `publicationGrammar`, and `userSetting`. This is a real request/response asymmetry; do **not** normalize both into one field name. | checked PDF text |
| REGE17-03 | §3.1.2.2, PDF p.77 | Registrar **MUST** give binding expiration **not less than 30 seconds**, and on REGISTER for a binding **MUST** refresh the associated expiration timer. This is a registrar-side rule; do not misstate as a universal client-side 30-second default. | checked PDF text |
| REGE17-04 | §3.1.2.5.1, pp.77–79 | Server's REGISTER 200 OK behavior is based on RFC 3261 §10.3 plus Microsoft response extensions. Registration and presence service availability are distinct conditions. | inspected relevant text; full case matrix pending |
| REGE17-05 | §3.1.2.5.1 and §§2.2.1.12–13 | Registrar can issue `NOTIFY` on registration status transitions. `P-Preferred-Registrar` can designate a new home registrar on `preferred-registrar-change`; authenticated notification and identity context are required before trusting migration information. | corroborated source clauses; detailed client processing pending |
| REGE17-06 | §3.1.2.5.1 and §4.1.4 | `userservices-unavailable` identifies a service degradation. For survivable mode a registered endpoint can remain registered but lose presence services; for an endpoint not supporting survivable mode, the example and server rules describe deregistration. | corroborated; profile-dependent processing pending |
| REGE17-07 | §4.1.3, example p.96 approx | Example shows `registration-notify`, `subscription-state: terminated;expires=0`, diagnostics **4141** and body `deregistered;event=rejected` for disabled user. Diagnostic does not by itself prove bad password; use full state+body+identity. | checked example, not unconditional client norm |
| REGE17-08 | §4.1.2 example | Specific-Contact `REGISTER` with `Expires: 0` and response 200 models basic unregistration. Must not be equated with closing TLS, destroying all SAs or unregistering every endpoint of one AoR. | confirmed via earlier source review; independent complete normative cleanup unresolved |

## Provisioning message lifecycle — confirmed base versus unresolved processing

The assessment identifies a SUBSCRIBE to the client's AoR, `Expires: 0`, `Event: vnd-microsoft-provisioning-v2` and `application/vnd-microsoft-roaming-provisioning-v2+xml`. **Treat this as assessment-reported until the exact §3.3.1 and §2.2.2 normative clauses are independently line-checked**, not as an invented definitive trace.

```mermaid
sequenceDiagram
    participant C as Windows SIP Client
    participant R as Registrar / Provisioning Server
    C->>R: SIP REGISTER (RFC 3261 + MS-SIPREGE)
    R-->>C: Correlated REGISTER outcome
    Note over C: Authenticated SA / MIC is separate from binding and provisioning state (MS-SIPAE)
    C->>R: Provisioning SUBSCRIBE (assessment-reported; exact clause check outstanding)
    R-->>C: Provisioning data in response or NOTIFY (negotiated-mode check outstanding)
    Note over C: Parse response provisionGroupList; validate complete policy snapshot
    R-->>C: Registration-state NOTIFY (when applicable)
    Note over C: Distinguish registered, service-unavailable, server migration and deregistered
```

This is a **dependency and conditional scenario**, not a mandatory packet count or a proven single server configuration. The integrity check on provisioning/NOTIFY and exact response placement depend on separate MS-SIPAE and MS-SIPREGE clauses.

## Cross-specification interpretation

- **RFC 3261 §10** establishes registration binding lifetimes, Contact scope and REGISTER semantics; MS-SIPREGE adds Microsoft's provisioning, notification, availability and registrar control.
- **MS-SIPAE §§3.2.4.1–3.2.5.2** supplies outgoing request integrity, SA lookup and received MIC/anti-replay. A response 200 or a provisional registration event is not sufficient alone to infer authenticated fully usable client state.
- **MS-PRES** endpoint-scope publication/subscription lifetimes must be reconciled with MS-SIPREGE registration changes; degrade in user services is not equivalent to an immediate generic transport disconnect.
- **MS-CONMGMT** server discovery and verified TLS identity must be independently re-evaluated if a preferred registrar changes; a notification cannot authorize a TLS-validation bypass.
- Distinguish **registered contact**, **authenticated security context**, **provisioning snapshot**, **presence service availability**, **SIP transport**, and **user feature entitlement** as separate state variables.

## Version applicability and remaining work

The uploaded rev 17.1 text contains legacy product notes, examples and different client/version profiles; for each extracted branch determine applicability to the actual targeted Skype for Business Windows Full Client generation. Validate the rest of §2.2.2 XSD/schema groups and §3.3, subscription lifetime, piggyback response, deployment scenarios, and every client-side timer and failure path. Verify all 112 provided assessment files separately and systematically; this examination does not endorse their overall “complete” claims.

**Evidence level:** the listed first-party fields and binding-timer passage are checked against the supplied v20250819 extraction. Some sequence/feature claims are explicitly assessment-derived or inferred. Whole protocol system: **NOT COMPLETE**. No code changes, live-server tests, main merge or release.
