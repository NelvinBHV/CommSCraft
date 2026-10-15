# Normative protocol-to-test evidence

Every requirement must have a stable ID, public specification with revision and subsection, implementation symbol, positive test, adversarial test, actual execution environment and result, and any real server interoperability evidence.

| Area | Normative references |
| --- | --- |
| SIP transactions and framing | [RFC 3261 §§7, 10, 17–18](https://www.rfc-editor.org/rfc/rfc3261) |
| SIP DNS/TLS discovery | [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263); MS-CONMGMT |
| Microsoft authentication | MS-SIPAE §§3.2.4–3.2.5 |
| Registration/GRUU | MS-SIPREGE |
| Presence | MS-PRES; [RFC 6665](https://www.rfc-editor.org/rfc/rfc6665) |
| Media negotiation | [RFC 3264](https://www.rfc-editor.org/rfc/rfc3264), [RFC 8866](https://www.rfc-editor.org/rfc/rfc8866), MS-SDPEXT |
| NAT/media security | [RFC 8445](https://www.rfc-editor.org/rfc/rfc8445), [RFC 3711](https://www.rfc-editor.org/rfc/rfc3711), MS-ICE2 |

Evidence states: missing, source-only, executed-offline, verified-interoperable. An authored unit test without an executed run is NOT a pass. Preserve original source-hash ledgers.