# Microsoft MS-OCSPROT section 2.2.2.1: SIP protocol dependency catalog

Research tranche 2026-10-22. No implementation changes or server tests.

## Source status

Official Microsoft section: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/bdcd1606-11c6-4457-a708-3d1c296259a0 (section 2.2.2.1, page last updated 2021-08-17). The MS-OCSPROT document landing page lists revision 9.0 on 2025-05-20: https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c . **The accessible web section may be older than revision 9.0**; this extraction is not a substitute for verifying the published 2025 PDF. Do not silently equate the two.

## Directly observed inventory in section 2.2.2.1

| Identifier | Documented responsibility | Client relevance |
|---|---|---|
| MS-CONMGMT | Client discovery of server address and persistent ordered transport | Core |
| MS-SIPAE | SIP NTLM, Kerberos, TLS-DSK authentication; asserted identity and Referred-By extensions | Core / supplementary identity |
| MS-SIPREGE | SIP registration extensions and client provisioning during registration | Core |
| MS-SIPRE | Routing extensions and SIMPLE presence-related extensions | Core and presence |
| MS-PRES | SIP presence publication and subscription | Core feature |
| MS-SIPCOMP | Capability negotiation and compressed SIP transport | Negotiated feature |
| MS-SIPAPP | Application-specific SIP extensions | Feature dependent |
| MS-OCER | Client/server diagnostic reporting | Error reporting |
| MS-PSOM | Shared-object messaging for collaboration and web conferencing | Conferencing |
| MS-OCPSTN | SIP PSTN/PBX integration | Calling |
| MS-EUMR | Routing for Exchange Unified Messaging | Optional deployment |
| MS-OCEXUM | Communicator integration with Exchange Unified Messaging | Optional deployment |
| MS-QoE | Audio/video quality-of-experience reporting | Media feature |
| MS-RGSWS | Response Group agent service | Optional deployment |
| MS-OCAUTHWS | Authentication and certificate-enrollment web service | Certificate provisioning |
| MS-E911WS | Location web service for emergency calls | Policy/deployment |
| MS-XCCOSIP | Group chat data carried in SIP INFO | Historical/feature profile |
| MS-OCGCWEB | Persistent chat room management web entry | Historical/feature profile |

The original Microsoft section also references RFC 3261, RFC 3265, RFC 3325, RFC 3892 and RFC 2118. Its use of RFC 3265 must be reconciled with its successor RFC 6665 and the historical Windows client behavior before deciding normative applicability.

## Cross-document interpretation

The authentication and registration path needs **MS-CONMGMT -> SIP transport and RFC3261 transaction handling -> MS-SIPAE -> MS-SIPREGE**, with routing behavior from MS-SIPRE and any applicable SIP application extensions. This is a dependency sketch, NOT a claimed strict on-wire sequence. TLS-DSK is explicitly named in the Microsoft overview, so an inventory limited to only Kerberos and NTLM would be incomplete. Microsoft provisioning is explicitly attached to registration and cannot be treated as an optional post-login afterthought without examining MS-SIPREGE clauses.

Presence and other full-client features require separate branches rather than being inferred from REGISTER success. MS-PRES, MS-SIPRE, MS-SIPAPP, MS-PSOM and media-specific documents must be analyzed individually.

## Normative verification gates

- Obtain the published MS-OCSPROT rev 9.0 PDF/DOCX; cross-check this 2021 subsection and extract all other client-scenario tables.
- Inspect the latest and target-deployment revisions of each 18 named MS documents, including normative references, required/optional feature rules, examples and error paths.
- Extract actual MS-SIPAE auth token and signature specifications and MS-SIPREGE registration/provisioning message traces; the overview does not contain them.
- Trace conference, call, media, contacts, presence and recovery references from the rest of MS-OCSPROT and record applicability.
- Build section-level requirement IDs and multi-document state transitions. Owner alone performs live-server tests.

**Status: first-party overview subsection indexed; complete protocol corpus and full-body verification NOT DONE.**
