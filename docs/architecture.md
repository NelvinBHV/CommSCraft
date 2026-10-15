# Architecture

CommSCraft uses WordCraft-like project organization, but implements distinct SIP, media and Windows integration components.

| Layer | Workspace path (historical v0.14.0) | Scope |
| --- | --- | --- |
| Desktop | apps/sfb-desktop | egui demonstration, not authenticated |
| Diagnostics | apps/sfb-inspect, apps/sfb-probe | Offline protocol examples and opt-in DNS/TLS probes |
| Protocol | crates/sfb-core | SIP parser, transaction and registration state, MIC/replay, offline presence/IM |
| Routing | crates/sfb-router | Bounded Non-INVITE client response dispatcher |
| Transport | crates/sfb-transport | Verified TLS receive-only transport |
| Future | Authenticated sessions / media / conferences | Only after separate RFC/MS review and live tests |

Trust boundary: TLS validation does not establish MS-SIPAE authentication. A SIP 200 does not establish sign-in. MIC verification must occur before replay state changes, and unsigned GRUU/Contact routing metadata needs independent channel trust.

Historical source tree is prepared but not yet uploaded here.