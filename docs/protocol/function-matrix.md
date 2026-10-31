# Function matrix with evidence status

Status: **B** = fully documented (implementable on the protocol side from the sources), **B\*** = documented, with documented conflicts/gaps that need a decision or interop confirmation, **T** = partly documented, **N** = not documented (or not implementable without a licence). Secondary evidence (file 39, kind "secondary") does **not** raise the status; only normative or official evidence does. The implementation status per function is tracked in [`../target-app-parity.md`](../target-app-parity.md).

| # | Function | Specifications | Evidence IDs | Status | Note |
|---|---|---|---|---|---|
| 1 | Server discovery | MS-OCDISCWS, MS-CONMGMT | AD-01…13, CON-10…20 | B | order officially documented (OB-06/OB-07) |
| 2 | TLS connection, keep-alive | MS-CONMGMT | CON-01…04, CON-30…42 | B | |
| 3 | SIP compression (optional) | MS-SIPCOMP | CP-01…08 | B\* | O-CP-2 closed (OB-01); O-CP-1 partly (OB-02) |
| 4 | Sign-in with NTLM | MS-SIPAE, MS-NLMP | AE-20…35, NL-01…05 | B\* | test vectors ✓; SIP signature recomputed on an OCS 2007 R2 capture (OB-11) |
| 5 | Sign-in with Kerberos | MS-SIPAE, MS-KILE | AE-25, AE-30, KE-01…03 | B | RFCs obtained (file 38) |
| 6 | Sign-in with TLS-DSK and client certificate | MS-SIPAE, MS-OCAUTHWS | AE-26/27, AW-01…15 | B\* | K-AE-1/3, K-AW-1; WS-Security header secondary (OB-08) |
| 7 | Registration, GRUU, endpoint ID | MS-SIPREGE, MS-SIPRE | RG-01…32, RE-01…10 | B | EPID→UUID recomputed (K-RE-1) |
| 8 | In-band provisioning | MS-SIPREGE | RG-40…50 | B\* | K-RG-1…3 |
| 9 | Publish and subscribe presence | MS-PRES, MS-SIP | PR-01…42, SIP-01…10 | B | |
| 10 | Contact list (server) | MS-SIP, MS-PRES | PR-50, SIP-07/08 | B | |
| 11 | Contact list (Exchange UCS) | MS-PRES, MS-OXWSCOS, MS-OXWSPERS | PR-51, EX-01…05 | B\* | Exchange authentication secondary (OB-15) |
| 12 | Privacy relationships and blocking | MS-PRES, MS-SIPREGE | PR-03, PR-20, PR-60 | B | container meaning officially documented (OB-05) |
| 13 | Offline address book | MS-ABS | AB-01…09 | B\* | compression checked; K-AB-1/2 |
| 14 | Address book and distribution list search | MS-DLX | DL-01…08 | B\* | K-DL-1; endpoint/SOAPAction secondary (OB-12) |
| 15 | Contact photos | MS-OXWSPHOTO | EX-06 | B | |
| 16 | Calendar and out-of-office state | MS-PRES (format) | PR-37, PR-39 | T | Exchange query secondary (OB-13/OB-14), format official |
| 17 | IM 1:1 | MS-SIPRE, MS-OCSPROT | OCS-UC09, RE-* | B | |
| 18 | Typing indicator | MS-CONFIM | CI-08 | T | body format secondary (OB-16, L-11) |
| 19 | Group IM (IM conference) | MS-CONFBAS, MS-CONFIM | CB-*, CI-01…09 | B\* | K-CI-1/2 |
| 20 | Persistent chat | MS-XCCOSIP, MS-OCGCWEB | XC-01…08 | T | Content-Type/SDP secondary (OB-17, L-16) |
| 21 | Media relay access | MS-AVEDGEA, MS-TURN | AV-01…09, TU-01…29, IE-01/02 | B | turn-08/STUN-02 obtained and consistent (IE-01), confirmed by Wireshark (OB-38); unknown attribute 0x8006 (O-TU-1) |
| 22 | ICE connectivity | MS-ICE2, MS-ICE | IC-01…35, IE-02…05 | B\* | CRC ✓, priorities ✓ (IE-05); duplicate fingerprint secondary (OB-09) |
| 23 | Media negotiation (SDP) | MS-SDPEXT, MS-SIPRE | SD-01…81, RE-65 | B\* | K-SD-1/2 |
| 24 | Media encryption | MS-SRTP, MS-SSRTP | SR-01…07, SS-01…06 | B | test vectors ✓; K-SR-1 decided (HMAC-SHA1) |
| 25 | RTP/RTCP, bandwidth, VSR | MS-RTP | RT-01…27, IE-08 | B | L-05/L-06 closed |
| 26 | Audio codecs G.711, G.722 | MS-RTP, MS-SDPEXT | RT-04/05, SD-21 | B | the codecs themselves are ITU standards |
| 27 | Audio codecs RTAudio, Siren, G.722.1 | MS-SDPEXT, MS-RTP, ITU-T G.722.1 | SD-20, RT-04 | T | G.722.1/Annex C: ITU package obtained (OB-26); Siren7 frame format secondary (OB-34); RTAudio only with a Microsoft licence (OB-27) |
| 28 | Video H.264 UC/SVC | MS-H264PF, MS-SDPEXT | PL-01…07, SD-27/28, IE-08 | B | UCConfig specification obtained via archive search (internal reference only) |
| 29 | Video RTVideo | MS-RTVPF | PL-10…12 | N | VC-1 codec (SMPTE ST 421) only for purchase (OB-29); only RTP RFC 4425 obtained (L-07); not planned (D-05) |
| 30 | DTMF, redundancy | MS-DTMF, MS-RTPRADEX | PL-20, PL-30 | B | |
| 31 | Picture control (freeze/refresh) | MS-XMLMC | PL-50, IE-07 | B | Content-Type documented (IE-07) |
| 32 | Bandwidth management (CAC) | MS-ICE2BWM, MS-TURNBWM | BW-01…16 | B | XOR ✓; O-BW-1 closed (OB-10) |
| 33 | PSTN telephony | MS-OCPSTN, MS-SIPRE | PS-01…09, RE-40…42 | B | |
| 34 | Forwarding, simultaneous ring, team and delegate calls | MS-SIPRE, MS-PRES | RE-50…54, PR-21 | B | |
| 35 | Hold, music on hold | MS-SDPEXT, MS-SIPRE | SD-06/07, RE-68 | B | |
| 36 | Call transfer | MS-SIPAE, MS-SIPRE | AE-14/37, RE-62/64 | B\* | attended/supervised/unattended officially documented (OB-25, UCMA Lync 2013); SfB headers by interop |
| 37 | Call park | MS-SIPAPP | CK-01…06 | B\* | K-CK-1 |
| 38 | Voicemail | MS-OCEXUM, MS-EUMSDP | UM-01…04 | T | message classes documented (OB-23); list retrieval by the client not documented (L-15) |
| 39 | Emergency calls with location | MS-E911WS, MS-SIPRE | E9-01…06, RE-67 | B\* | K-E9-1, L-14 |
| 40 | Schedule meetings | MS-CONFPRO | PV-01…11 | B\* | K-PV-1…3; O-PV-3 closed (OB-24: conference-key deprecated) |
| 41 | Join meetings, lobby, roles | MS-CONFBAS | CB-01…41 | B\* | K-CB-1/2; joining from Linux via the meet URL secondary (OB-33) |
| 42 | A/V conferences, muting | MS-CONFAV | CA-01…23 | B\* | K-CA-1/2 |
| 43 | Desktop and application sharing | MS-CONFAS, MS-RTASPF, MS-RDPBCGR, MS-RDPEMC | CS-*, PL-40…42, RD-* | T | pre-connection/multifrag narrowed (OB-18/19); RemoteFX accepted by the Lync 2013 viewer (OB-35, secondary); L-12 |
| 44 | Whiteboard, PowerPoint, polls | MS-PSOM | PO-01…08 | T | encoding ✓; rendering L-17 |
| 45 | Quality reports and call rating | MS-QoE | QE-01…05 | B\* | O-QE-1 partly: ITU P.562/P.800.1 obtained (OB-28) |
| 46 | Error diagnostics and display | MS-OCER | ER-01…07 | B | |
| 47 | Response group agent | MS-RGSWS | WS-01 | B | optional |
| 48 | UCWA web services | MS-CVWREST, MS-ECREST, MS-OCSMP | WS-02 | B | optional (web/mobile) |
| 49 | Conversation history in Exchange | MS-OXWSCDATA, MS-OXWSSRCH | EX-07, EX-09 | T | storage location/item type officially documented (OB-22); field contents open (O-EX-3) |
| 50 | Sign-out | MS-SIPREGE, MS-CONFBAS | RG-29, CB-14, CB-21 | B | |
| 51 | Audio codec SILK | MS-SDPEXT, draft-vos-silk-02, draft-spittka-silk-payload-format-00 | SD-22, IE-10 | T | reference code available, patent licence excluded; `useinbandfec` not in the draft; K-RT-1 |
| 52 | File transfer 1:1 | – (not found in the Microsoft package) | OB-21 | N | Content-Type only secondary (OB-21); normatively only the device type `data` in x-devicecaps (OB-31) |

**Totals (52 functions):** B = 24, B\* = 17, T = 9, N = 2 (function 29 RTVideo; function 52 file transfer, secondary only).
