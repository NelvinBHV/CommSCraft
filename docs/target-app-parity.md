# CommSCraft parity with the Skype for Business client

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (first M1 building blocks)

The assessment of how close CommSCraft is to the Skype for Business desktop client for Windows.
[`ROADMAP.md`](../ROADMAP.md) summarizes it; [`gaps.md`](gaps.md) itemizes the shortfalls.

## What we measure against

- **Target client:** Skype for Business 2016/2019/LTSC desktop client for Windows (the "full
  client"), connected to an on-premises Skype for Business Server 2015, 2019 or Subscription
  Edition.
- **Sources:** Microsoft Open Specifications (Office and Windows protocols, editions up to 2025),
  IETF RFCs and Internet-Drafts, ITU-T Recommendations; official Microsoft documentation where the
  specifications are silent. The client binaries are never inspected.
- **Not in scope:** Skype for Business Online (retired by Microsoft), Microsoft Teams, the consumer
  Skype service.

## Functions and evidence status (2026-10-26)

Evidence status comes from the protocol analysis ([`protocol/function-matrix.md`](protocol/function-matrix.md)): **B** fully documented,
**B\*** documented with conflicts/gaps needing a decision or interop check, **T** partly documented,
**N** not documented or not implementable without a licence. Implementation status: "–" no code;
"parts tested-offline" = building blocks exist and pass offline tests, the function as a whole does not.

| # | Function | Main specifications | Evidence | Implemented |
|---|---|---|---|---|
| 1 | Server discovery | MS-OCDISCWS, MS-CONMGMT | B | parts tested-offline (Autodiscover state machine, DNS SRV ordering: `discovery`) |
| 2 | TLS connection, keep-alive | MS-CONMGMT | B | tested-offline (TLS adapter with loopback test, stream framing, keep-alive negotiation: `net`, `sip-msg`, `register`) |
| 3 | SIP compression (optional) | MS-SIPCOMP | B\* | – |
| 4 | Sign-in with NTLM | MS-SIPAE, MS-NLMP | B\* | parts tested-offline (NTLMv2 handshake messages, SIP signing and verification: `ntlm`, `sipae`) |
| 5 | Sign-in with Kerberos | MS-SIPAE, MS-KILE | B | – |
| 6 | Sign-in with TLS-DSK and client certificate | MS-SIPAE, MS-OCAUTHWS | B\* | – |
| 7 | Registration, GRUU, endpoint ID | MS-SIPREGE, MS-SIPRE | B | parts tested-offline (REGISTER flow, GRUU, refresh: `register`, `sip-identity`) |
| 8 | In-band provisioning | MS-SIPREGE | B\* | parts tested-offline (SUBSCRIBE, provisionGroupList, validated ServerConfiguration: `provisioning`, `xml`) |
| 9 | Publish and subscribe presence | MS-PRES, MS-SIP | B | tested-offline (endpoint and own state publication, long-lived batch subscriptions with NOTIFY, refresh and pool resubscribe: `presence`, `session`) |
| 10 | Contact list (server) | MS-SIP, MS-PRES | B | parts tested-offline (read: `presence`) |
| 11 | Contact list (Exchange unified contact store) | MS-PRES, MS-OXWSCOS, MS-OXWSPERS | B\* | – |
| 12 | Privacy relationships and blocking | MS-PRES, MS-SIPREGE | B | – |
| 13 | Offline address book | MS-ABS | B\* | – |
| 14 | Address book and distribution list search | MS-DLX, MS-NTHT | B\* | parts tested-offline (SearchAbEntry basic search with checked answer: `dlx`; NTLM over HTTP on a kept-alive connection: `ntlm`, `net`; CLI `/search`); distribution list expansion – |
| 15 | Contact photos | MS-OXWSPHOTO | B | – |
| 16 | Calendar and out-of-office state | MS-PRES (format) | T | – |
| 17 | Instant messaging 1:1 | MS-SIPRE, MS-OCSPROT | B | tested-offline (incoming and outgoing sessions, queued text, delivery state: `im`, `session`) |
| 18 | Typing indicator | MS-CONFIM | T | parts tested-offline (INFO KeyboardActivity, secondary format: `im`) |
| 19 | Group IM (IM conference) | MS-CONFPRO, MS-CONFBAS, MS-CONFIM | B\* | tested-offline (create, join, roster, IM MCU, dial-out invitations, IMDN, session timers, leave: `conf`, `session`; CLI `/group`, `/invite`, `/join`); invitations of Windows clients secondary only (K-CI-3) |
| 20 | Persistent chat | MS-XCCOSIP, MS-OCGCWEB | T | – |
| 21 | Media relay access | MS-AVEDGEA, MS-TURN | B | parts tested-offline (MS-TURN client incl. TCP framing and pseudo-TLS: `turn`; STUN codec: `stun`); MS-AVEDGEA credentials tested-offline (`avedgea`, engine bootstrap and refresh) |
| 22 | ICE connectivity | MS-ICE2, MS-ICE | B\* | – |
| 23 | Media negotiation (SDP) | MS-SDPEXT, MS-SIPRE | B\* | parts tested-offline (audio offer/answer, a=crypto, ICE-19 attributes: `sdp`) |
| 24 | Media encryption | MS-SRTP, MS-SSRTP | B | tested-offline (SRTP/SRTCP contexts with MKI, replay window, ROC; RFC 3711 B.2/B.3 and libsrtp reference packets: `srtp`) |
| 25 | RTP/RTCP, bandwidth, video source requests | MS-RTP | B | parts tested-offline (RTP header, sender, reception statistics, SR/RR/SDES/BYE, DTMF: `rtp`); MS-RTP profile extensions – |
| 26 | Audio codecs G.711, G.722 | MS-RTP, MS-SDPEXT, ITU-T G.711, G.722 | B | tested-offline (G.711 against Tables 1–6; G.722 ADPCM bit-exact against all ITU digital test sequences, QMF round trip: `g711`, `g722`) |
| 27 | Audio codecs RTAudio, Siren, G.722.1 | MS-SDPEXT, MS-RTP, ITU-T G.722.1 | T | – |
| 28 | Video H.264 UC/SVC | MS-H264PF, MS-SDPEXT | B | – |
| 29 | Video RTVideo | MS-RTVPF | N | not planned |
| 30 | DTMF, redundancy | MS-DTMF, MS-RTPRADEX | B | – |
| 31 | Picture control (freeze/refresh) | MS-XMLMC | B | – |
| 32 | Bandwidth management (call admission control) | MS-ICE2BWM, MS-TURNBWM | B | – |
| 33 | PSTN telephony | MS-OCPSTN, MS-SIPRE | B | – |
| 34 | Forwarding, simultaneous ring, team and delegate calls | MS-SIPRE, MS-PRES | B | – |
| 35 | Hold, music on hold | MS-SDPEXT, MS-SIPRE | B | – |
| 36 | Call transfer | MS-SIPAE, MS-SIPRE | B\* | – |
| 37 | Call park | MS-SIPAPP | B\* | – |
| 38 | Voicemail | MS-OCEXUM, MS-EUMSDP | T | – |
| 39 | Emergency calls with location | MS-E911WS, MS-SIPRE | B\* | – |
| 40 | Schedule meetings | MS-CONFPRO | B\* | – |
| 41 | Join meetings, lobby, roles | MS-CONFBAS | B\* | – |
| 42 | A/V conferences, muting | MS-CONFAV | B\* | – |
| 43 | Desktop and application sharing | MS-CONFAS, MS-RTASPF, MS-RDPBCGR, MS-RDPEMC | T | – |
| 44 | Whiteboard, PowerPoint, polls | MS-PSOM | T | – |
| 45 | Quality reports and call rating | MS-QoE | B\* | – |
| 46 | Error diagnostics and display | MS-OCER | B | – |
| 47 | Response group agent | MS-RGSWS | B | – |
| 48 | UCWA web services | MS-CVWREST, MS-ECREST, MS-OCSMP | B | – |
| 49 | Conversation history in Exchange | MS-OXWSCDATA, MS-OXWSSRCH | T | – |
| 50 | Sign-out | MS-SIPREGE, MS-CONFBAS | B | parts tested-offline (REGISTER Expires 0: `register`) |
| 51 | Audio codec SILK | MS-SDPEXT, draft-vos-silk-02 | T | – |
| 52 | File transfer 1:1 | none public | N | needs captures |

**Totals:** B 24 · B\* 17 · T 9 · N 2 · implemented 0 / 52.
