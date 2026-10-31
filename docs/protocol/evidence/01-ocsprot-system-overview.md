# Evidence: MS-OCSPROT – system overview

Source: [MS-OCSPROT] "Skype for Business and Skype for Business Server Protocols Overview", edition 20250520, revision 9.0, 73 pp.
Page numbers = physical PDF page (= the "n / 73" footer).

Nature: informative system document. It does **not** define processing rules (p. 2: "It does not restate the processing rules …"); binding details are in the individual specifications. It does, however, **state which protocols the client uses in which use case** – this is the structure of the function matrix.

## Scope

| ID | Statement | Evidence |
|---|---|---|
| OCS-01 | Applies to OCS 2007 … Lync 2013/Skype for Business, SfB 2016, SfB 2019, SfB Server SE | §4, p. 66 |
| OCS-02 | The client connects via SIP; media then run over media protocols (RTP etc.) | §2.1, p. 13 |
| OCS-03 | Version control: the server checks the `User-Agent` header against a minimum version and otherwise rejects the sign-in | §2.6.1, p. 54 |
| OCS-04 | The server needs AD, DNS (A + SRV), a CA (TLS server certificates), IIS (HTTPS for address book, DL expansion, document sharing) | §2.3.2.1–2.3.2.4, p. 25 |

## Protocol assignment (complete list of member protocols, §2.2, pp. 17–23)

- Directory: MS-ABS, MS-DLX (§2.2.1)
- SIP: MS-CONMGMT, MS-EUMR, MS-OCEXUM, MS-OCPSTN, MS-OCER, MS-PRES, MS-QoE, MS-SIPCOMP, MS-SIPAE, MS-SIPRE, MS-SIPREGE, MS-RGSWS, MS-PSOM, MS-SIPAPP, MS-OCAUTHWS, MS-E911WS, MS-XCCOSIP, MS-OCGCWEB (§2.2.2.1)
- Conferencing: MS-CONFBAS, MS-CONFPRO, MS-CONFIM, MS-CONFAV, MS-CONFAS, MS-XMLMC (§2.2.2.2)
- HTTP: MS-OCSMP, MS-OCDISCWS, MS-CVWREST, MS-ECREST (§2.2.2.3)
- Media: MS-EUMSDP, MS-DTMF, MS-H264PF, MS-RTP, MS-RTPRADEX, MS-RTASPF, MS-RTVPF, MS-SDPEXT, MS-SRTP, MS-SSRTP (§2.2.3.1)
- ICE: MS-AVEDGEA, MS-ICE, MS-ICE2, MS-ICE2BWM, MS-TURN, MS-TURNBWM (§2.2.3.2)
- Other normative references: MS-NLMP, MS-RDPBCGR (image format for application sharing), MS-WOPI, RFC 2118 (MPPC), RFC 3261, 3265, 3325, 3550, 3551, 3892, 6120/6121 (§1.2, pp. 10–12)

## Use cases (skeleton of the client flow)

| ID | Use case | Steps (short) | Protocols | Evidence |
|---|---|---|---|---|
| OCS-UC01 | Find and connect to the server | DNS SRV for the domain of the SIP URI → TCP → optional TLS (verify certificate) → optional compression | CONMGMT, SIPCOMP | §2.5.1, pp. 27–28 |
| OCS-UC02 | Register/authenticate | REGISTER (requests GRUU, resource-list/enhanced-presence support, keep-alive) → 401/407 offering Kerberos/NTLM → REGISTER with token → 200 OK with GRUU, keep-alive confirmation, server signature | CONMGMT, PRES, SIPAE, SIPRE, SIPREGE | §2.5.2, pp. 28–30 |
| OCS-UC03 | Bootstrap | SUBSCRIBE in-band provisioning → SUBSCRIBE contacts/groups → SUBSCRIBE self presence → SERVICE location profile → SERVICE MCU list → SERVICE media relay token → SERVICE publish presence → batch SUBSCRIBE for all contacts; order is free | SIPREGE, PRES, SIPRE, AVEDGEA | §2.5.3, pp. 30–32 |
| OCS-UC04 | Location (E911) | HTTPS SOAP GetLocations with network identifiers → list of civic addresses → user choice, cache | E911WS | §2.5.4, pp. 32–33 |
| OCS-UC05 | Complete sign-in | UC01+UC02+UC03+UC04 | – | §2.5.5, p. 33 |
| OCS-UC06 | Change presence | SERVICE (calendar from Exchange, device/capabilities, activity, contact card) → 200 OK → server NOTIFY to subscribers | PRES | §2.5.6, pp. 33–34 |
| OCS-UC07 | Load address book | Full and delta files from the ABS URL in in-band provisioning; for search and reverse number lookup | ABS | §2.5.7, pp. 34–35 |
| OCS-UC08 | Expand distribution list | HTTPS SOAP to the DLX URL from provisioning; response: SIP URI, mail, alias, display name per member | DLX | §2.5.8, p. 35 |
| OCS-UC09 | Start IM | INVITE → 200 → ACK → MESSAGE → 200 | SIPRE | §2.5.9, pp. 36–37 |
| OCS-UC10 | Add contact | SERVICE → 200; server notifies the other party (subscriber list) and delivers presence via NOTIFY/BENOTIFY | PRES | §2.5.10, p. 37 |
| OCS-UC11 | Multiple endpoints | Server forks the INVITE; endpoints are chosen by published capabilities; server CANCELs the rest | PRES, SIPRE | §2.5.11, pp. 37–38 |
| OCS-UC12 | Start a call | TURN allocation (Edge) → INVITE with SDP offer → other side allocates, 183 with answer → ICE checks on both sides → 200 → RTP/SRTP → re-INVITE with chosen candidates | AVEDGEA, ICE/ICE2, ICE2BWM, OCPSTN, RTP, RTPRADEX, SDPEXT, SIPRE, SRTP, SSRTP, TURN, TURNBWM | §2.5.12, pp. 38–41 |
| OCS-UC13 | Add video | as UC12, plus H264PF, RTVPF | – | §2.5.13, pp. 41–43 |
| OCS-UC14 | Answer a call (MPOP) | as UC12 from the callee's view | – | §2.5.14, pp. 43–45 |
| OCS-UC15 | End a call | BYE → 200; release TURN ports afterwards | SIPRE | §2.5.15, pp. 45–46 |
| OCS-UC16 | QoE report | after the call every client sends its own QoE report to its provisioned server | QoE, SIPRE | §2.5.16, pp. 46–47 |
| OCS-UC17 | Start/join an audio conference | addConference (CONFPRO) → conference URI → INVITE addUser (focus) → SUBSCRIBE conference → 200 with conference document + MCU URI → BENOTIFY roster → INFO getConference → INVITE with SDP to the AV MCU → ICE → RTP → re-INVITE | CONFPRO, CONFBAS, CONFAV, ICE, SDPEXT, SRTP … | §2.5.17, pp. 47–49 |
| OCS-UC18 | Conference events | SUBSCRIBE → 200 with roster → BENOTIFY | CONFBAS | §2.5.18, pp. 49–50 |
| OCS-UC19 | Share desktop | INVITE to the AS MCU with SDP → ICE2 → re-INVITE → media RTP/RTASPF (graphics per MS-RDPBCGR) | CONFAS, ICE2, RTASPF, SRTP … | §2.5.19, pp. 50–51 |
| OCS-UC20 | Whiteboard | INFO addUser to the data MCU → 202 → INFO response with PSOM credentials → PSOM connection | CONFBAS, PSOM | §2.5.20, p. 52 |
| OCS-UC21 | Persistent chat room | INVITE → ACK → INFO cmd:getserverinfo → rpl:getserverinfo → cmd:bjoin → rpl:bjoin → rpl:bccontext (history) → grpchat | XCCOSIP | §2.5.21, pp. 52–54 |

## Security-relevant system statements

| ID | Statement | Evidence |
|---|---|---|
| OCS-SEC1 | DLX only over SSL; authentication Kerberos V5 / NTLM | §2.9.1.2, p. 56 |
| OCS-SEC2 | Media keys (SDP `a=crypto`) only over a TLS-protected SIP connection; SDPEXT does not encrypt them itself | §2.9.1.5, p. 56 |
| OCS-SEC3 | The `ms-diagnostics` header can contain internal information | §2.9.1.4, p. 56 |
| OCS-SEC4 | TURN: long-term credential in digest challenge/response | §2.9.1.7, p. 57 |

## Limits of this document

- Figures (sequence diagrams Fig. 6–35) are only graphics in the PDF; the text extraction contains the step lists reproduced above.
- Nothing about the client's Exchange Web Services integration (calendar, contacts, conversation history); see the separate evidence file 37.
