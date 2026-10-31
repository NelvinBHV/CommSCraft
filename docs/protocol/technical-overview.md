# How a Skype for Business client works – technical overview (with evidence)

Status: 2026-10-26. Basis: 865 Microsoft Open Specifications PDFs (edition 20250520 and others), registered in [`sources-ms.tsv`](sources-ms.tsv). Every statement refers to an **evidence ID** (e.g. `AE-29`) in [`evidence/`](evidence/); there you find document, section and physical PDF page. Conflicts, gaps and open points: [`conflicts-gaps-open-points.md`](conflicts-gaps-open-points.md). Function overview with evidence status: [`function-matrix.md`](function-matrix.md). The cited IETF foundations (192 RFCs, 20 drafts) and the H.264 UCConfig specification found by archive search are evaluated in [`evidence/38-ietf-foundations-and-archive-sources.md`](evidence/38-ietf-foundations-and-archive-sources.md) ([`sources-ietf.tsv`](sources-ietf.tsv)); the search for the open points (official Microsoft documentation, web archives, ITU, open-source implementations) in [`evidence/39-evidence-for-open-points.md`](evidence/39-evidence-for-open-points.md).

**Evidence principle:** "documented" = stated like this in the sources; "recomputed" = a calculation example/test vector of the document itself was checked; "inference" = technically justified but not literally documented (always marked); "open" = only an interop test can clarify it; "secondary" = confirmed only by an open-source implementation or observation.

## 1. Architecture in four levels

| Level | Content | Evidence |
|---|---|---|
| Signalling | SIP over TLS (TCP) to the Front End/Director/Edge; MS extensions for sign-in, registration, routing, presence, conference control | files 02–07, 10, 18–22, 24, 31, 32 |
| Web services | HTTPS: Lync Autodiscover, web ticket/certificate, address book, distribution lists, location, Exchange EWS | files 08, 09, 27–29, 34, 37 |
| Media | ICE (MS-ICE2) over UDP/TCP with TURN relay (MS-TURN, token via MS-AVEDGEA), RTP/RTCP with MS extensions, SRTP/SSRTP, payload formats | files 11–17, 23 |
| Content | Screen sharing = RDP over RTP (MS-RTASPF); conference content over PSOM (own TLS connection) | files 17, 22, 33, 36 |

Overall picture and use cases UC01–UC21: file 01 (MS-OCSPROT §2.5).

## 2. Sign-in (start until "signed in")

1. **Find the server**: Lync Autodiscover via `lyncdiscoverinternal.<domain>` / `lyncdiscover.<domain>` first (AD-01…AD-13, OB-06/OB-07; returns the SIP access points `SipClientInternal/ExternalAccess` AD-11 and web service URLs), then DNS SRV `_sipinternaltls._tcp`, `_sip._tls` (and `_sipinternal._tcp`, `_sip._tcp`) + DHCP option 120 + fallback hosts `sipinternal`, `sip`, `sipexternal` (CON-10…CON-20).
2. **Transport**: TCP with TLS on top; verify the server certificate (FQDN of the proxy) (CON-01…CON-04, CON-16/17).
3. **Optional compression** (TLS only): `NEGOTIATE` as the first SIP packet, then MPPC 8K compression (CP-01…CP-08; file 10).
4. **Negotiate keep-alive**: `ms-keep-alive: UAC;hop-hop=yes`, CRLFCRLF every 2/3 of the timeout (CON-30…CON-36).
5. **REGISTER** without Authorization (with `epid`, `+sip.instance` = SHA-1 name-based UUID from the EPID – recomputed, RE-02/RE-06/RE-07; `Supported: gruu-10, adhoclist, msrtc-event-categories …` RG-03…RG-11) → **401/407** offering NTLM / Kerberos / TLS-DSK (AE-10, AE-51).
6. **Security association** (MS-SIPAE): connectionless NTLM (AE-24; NTLMv2 test vectors recomputed NL-01…NL-05; real OCS 2007 R2 exchange recomputed OB-11), Kerberos `sip/<fqdn>` (AE-25, KE-01) or **TLS-DSK** with a client certificate (AE-26/27): certificate via web ticket (WS-Trust, AW-01…AW-06) and `GetAndPublishCert` (AW-10…AW-15). Afterwards **every** message is signed/verified: buffer of header fields, NTLM MIC with fixed sequence number 100, `cnum/crand`, `snum/srand`, replay window (AE-29…AE-35).
7. **200 OK to REGISTER**: take the GRUU, refresh before expiry over the same connection (RG-20…RG-28); error/deregistration messages (RG-27, RG-30/31).
8. **In-band provisioning**: `SUBSCRIBE Event: vnd-microsoft-provisioning-v2` with groups ServerConfiguration, meetingPolicy, ucPolicy, locationPolicy, mediaConfiguration, presencePolicyV2, persistentChatConfiguration … (RG-40…RG-50). Delivers among others `mrasUri`, `qosUri`, `focusFactoryUri`, `absInternal/ExternalServerUrl`, `dlx*`, `lisInternalUrl`, `callParkServerUri`, `voiceMailUri`, port ranges, encryption policy (file 04).
9. **Bootstrap** (OCS-UC03): contact/group subscription + self subscription (PR-13, SIP-07), batch subscription of contact presence (PR-14…PR-16), location profile via SERVICE (RE-40…RE-42), MRAS token (AV-01…AV-09), publish own presence (PR-11, PR-30…PR-39), location via the E911 web service (E9-01…E9-06), load the address book file (AB-01…AB-09).
10. **Error handling**: evaluate/show `ms-diagnostics` codes, optional `reportError` via SERVICE (ER-01…ER-07).

## 3. Presence, contacts, search

- Enhanced presence: categories `state`, `note`, `contactCard`, `calendarData`, `device`, `routing` …; containers 0/100/200/300/400/32000 as access levels with officially documented meaning (PR-01…PR-42, PR-60, OB-05); versioning/409 handling (PR-05, PR-12).
- Contact list: classic `vnd-microsoft-roaming-contacts` (PR-50, SIP-07/08) or **UCS in Exchange** (`ms-ucs`, PR-51) with the EWS operations GetImItemList, AddImContactToGroup … (EX-01…EX-05).
- Search: offline address book (AB-*), web search/distribution lists MS-DLX (DL-01…DL-08), photos via EWS GetUserPhoto (EX-06).

## 4. Chat

- 1:1: INVITE (SDP `m=message`) → MESSAGE in the dialog (OCS-UC09, file 01; routing rules file 06).
- Several participants = IM conference: focus + IM MCU, `Ms-Sender`, IMDN on 202, history of the first 40 s (CI-01…CI-09). **Typing indicator**: transport via INFO documented; body `application/xml` `<KeyboardActivity><status status="type|idle"/></KeyboardActivity>` only **secondary evidence** (OB-16, L-11).
- Persistent chat (optional): XCCOS over INFO (XC-01…XC-08); entry `ocschat@<domain>`, Content-Type `text/plain` only **secondary evidence** (OB-17, L-16).

## 5. Audio/video 1:1

1. Get the MRAS token (AV-*), TURN allocations (UDP 3478 / TCP 443 with pseudo-TLS) incl. MESSAGE-INTEGRITY (TU-01…TU-29), optional bandwidth check in the Allocate requests (BW-01…BW-16, XOR examples recomputed; policy server = TURN server, OB-10).
2. SDP offer per MS-SDPEXT: multipart (ICE-19 + ICE-06 version for old peers, RE-65), candidates (SD-40…SD-47), `a=crypto`/`cryptoscale` AES_CM_128_HMAC_SHA1_80 (SD-10…SD-14), codecs/x-caps/x-ssrc-range (SD-20…SD-30, SD-60…SD-66), early media (SD-50/51).
3. ICE checks per MS-ICE2 (IC-01…IC-18; CRC table checked, duplicate fingerprint OB-09), final offer from the caller (IC-07), keep-alive ≤19 s (IC-05).
4. Media: RTP/RTCP with MS profile extensions, VSR/PLI/DSH feedback, bandwidth estimation (RT-01…RT-27); SRTP profile (SR-01…SR-07, K-SR-1: HMAC-SHA1 confirmed by test vector); payload H.264UC/SVC (PL-01…PL-07), RED (PL-20), DTMF RFC 4733 (PL-30).
5. End: BYE, release TURN (OCS-UC15), then a **QoE report** via SERVICE (QE-01…QE-05).
6. Dialog recovery after connection loss (`ms-dialog-route-set-update`, 430, RE-30…RE-34).

**Codec situation (decision D-05):** G.711, G.722, H.264 (SVC extension and UCConfig modes 0/1/2q/2s/3 documented, IE-08) and RFC DTMF can be implemented openly. **SILK** is described by draft-vos-silk-02 with reference code (IE-10; patent licence explicitly excluded). **G.722.1 including Annex C (Siren14)** is freely available from the ITU-T with reference code (OB-20/OB-26). **RTAudio** needs a Microsoft licence (OB-27); **RTVideo** (VC-1 based) and **Siren7** are not available as public bitstream specifications (L-03, L-07); SILK/OPUS support is documented contradictorily (K-RT-1).

## 6. Telephony

- Dialling with location profile/`phone-context`, gateway detection `isGateway`, `ms-early-media`, `ms-bypass`, multiple SDP with Content-ID (PS-01…PS-09).
- Forwarding/simultaneous ring/team/delegate via the routing preamble (RE-50…RE-54).
- **Hold**: `a=inactive` + `sip.rendering="no"` (SD-06); **park/retrieve**: MS-SIPAPP (CK-01…CK-06, requires INVITE/REFER with Replaces); **transfer**: attended/supervised/unattended per the official UCMA description (OB-25), SfB-specific headers partly documented (L-13).
- Voicemail: voicemail GRUU + `Ms-Exchange-Command: skip-pin`, play on phone (UM-01…UM-04).
- Emergency call: PIDF-LO/`Geolocation` in the INVITE (RE-67) with the location from MS-E911WS (E9-*).

## 7. Conferences

- Schedule via the focus factory using SERVICE (PV-01…PV-11; the conference key is deprecated, OB-24); join via link (simple join, CB-40/41, OB-33) or URI.
- Focus dialog: INVITE with C3P `addUser`, role, lobby (`lobby-capable`), session timer (CB-10…CB-18); roster via SUBSCRIBE `conference` with the first full notification ≤32 s (CB-20…CB-27); control commands via INFO with 202/final response (CB-30…CB-37).
- Modalities: A/V MCU (CA-01…CA-23, mute via modifyEndpointMedia), IM MCU (CI-*), sharing MCU (CS-01…CS-11), web conferencing MCU with PSOM (PO-01…PO-08, encodings recomputed).

## 8. Screen/application sharing

`m=applicationsharing` over ICE TCP (SD-80/81, CS-02…CS-08), RDP data stream as RTP PT 127 without RDP encryption (PL-40…PL-42), multiparty channel `encomsp` (RD-03…RD-05, GroupId 3766552189 CS-09), RDP connection sequence (RD-01). Narrowed but open: pre-connection PDU and graphics codecs (O-RD-1/2; RemoteFX accepted by the Lync 2013 viewer, OB-35).

## 9. Sign-out

Conferences: BYE to the MCUs and the focus (CB-14, CI-09); end subscriptions (`Expires: 0`, CB-21); registration: REGISTER `Expires: 0` (RG-29); discard the SA.

## 10. Legal notes (from the documents themselves)

Every document contains the "Intellectual Property Rights Notice" (p. 1): copies for developing implementations are permitted; schemas/IDLs/code examples may be distributed in implementations; Microsoft patents may apply (Open Specifications Promise/Community Promise per document). An open-source project must check the patent situation/promise coverage for each protocol used – **not legal advice**, only a note from the sources. The source package metadata forbids redistributing the PDFs in a public repository; this repository therefore contains only our own descriptions with citations.
