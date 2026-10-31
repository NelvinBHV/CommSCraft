# CommSCraft parity with the Skype for Business client

> **Last reviewed:** 2026-10-31 · **Last updated:** 2026-10-31 · **Change:** minor (synchronised with the code: M1–M3 building blocks)

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

## Functions and evidence status (2026-10-31)

Evidence status comes from the protocol analysis ([`protocol/function-matrix.md`](protocol/function-matrix.md)): **B** fully documented,
**B\*** documented with conflicts/gaps needing a decision or interop check, **T** partly documented,
**N** not documented or not implementable without a licence. Implementation status: "–" no code;
"parts tested-offline" = building blocks exist and pass offline tests, the function as a whole does not.

| # | Function | Main specifications | Evidence | Implemented |
|---|---|---|---|---|
| 1 | Server discovery | MS-OCDISCWS, MS-CONMGMT | B | tested-offline (Autodiscover over HTTPS only (D-10) with trust model, DNS SRV and fallback host names: `discovery`, `url`, `net`; used by `commscraft-cli`) |
| 2 | TLS connection, keep-alive | MS-CONMGMT | B | tested-offline (TLS 1.2/1.3 adapter with loopback test, stream framing, keep-alive negotiation and CRLF keep-alive: `net`, `sip-msg`, `register`) |
| 3 | SIP compression (optional) | MS-SIPCOMP | B\* | – |
| 4 | Sign-in with NTLM | MS-SIPAE, MS-NLMP | B\* | tested-offline (NTLMv2 incl. MIC, MS-SIPAE signing and verification up to version 4, offline sign-in against a test registrar: `ntlm`, `sipae`, `register`) |
| 5 | Sign-in with Kerberos | MS-SIPAE, MS-KILE | B | – |
| 6 | Sign-in with TLS-DSK and client certificate | MS-SIPAE, MS-OCAUTHWS | B\* | – |
| 7 | Registration, GRUU, endpoint ID | MS-SIPREGE, MS-SIPRE | B | tested-offline (REGISTER with GRUU, refresh, signed 301 redirect to the home pool, deregistration NOTIFY, version block, keep-alive: `register`, `sip-identity`) |
| 8 | In-band provisioning | MS-SIPREGE | B\* | tested-offline (SUBSCRIBE with piggyback, provisionGroupList, validated ServerConfiguration; used for the address book, conference and media relay URIs: `provisioning`, `xml`, `session`) |
| 9 | Publish and subscribe presence | MS-PRES, MS-SIP | B | tested-offline (endpoint and own state publication, long-lived batch subscriptions with NOTIFY, refresh and pool resubscribe: `presence`, `session`) |
| 10 | Contact list (server) | MS-SIP, MS-PRES | B | parts tested-offline (read the contact list with a live subscription and delta updates: `presence`, `session`); adding/removing contacts and groups – |
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
| 21 | Media relay access | MS-AVEDGEA, MS-TURN | B | parts tested-offline (MS-TURN client incl. SHA-256, TCP framing and pseudo-TLS: `turn`; STUN codec: `stun`; MS-AVEDGEA credentials fetched and refreshed by the engine: `avedgea`, `session`; UDP and TCP (443) relay candidates in calls: `call-media`); IPv6 relays, multiplexed TURN (not used by SfB) – |
| 22 | ICE connectivity | MS-ICE2, MS-ICE | B\* | parts tested-offline (ICE agent over UDP and TCP (ice-tcp-07 pairing, RFC 4571 framing on the TCP relay): checks in both STUN formats, peer-reflexive candidates, regular nomination, keep-alive, consent freshness: `ice-agent`, `call-media`); TCP-only mode and MS-ICE (2007) – |
| 23 | Media negotiation (SDP) | MS-SDPEXT, MS-SIPRE | B\* | parts tested-offline (audio offer/answer, a=crypto, ICE-19 attributes, remote-candidates: `sdp`; call signalling with multipart offers, ringing, early media as the caller (100rel/PRACK, 199), answer, final offer, CANCEL, BYE: `session`, `call-media`); a=cryptoscale, early answer as the callee, early media of forked calls, video – |
| 24 | Media encryption | MS-SRTP, MS-SSRTP | B | parts tested-offline (SRTP/SRTCP contexts with MKI, replay window, ROC – RFC 3711 B.2/B.3 and libsrtp reference packets – used in calls: `srtp`, `media`; SSRTP reception from the A/V MCU with `a=cryptoscale` negotiation – MS-SSRTP §4 vectors: `srtp`, `sdp`, `media`); SSRTP sending (server flavor, not needed by a client) – |
| 25 | RTP/RTCP, bandwidth, video source requests | MS-RTP | B | parts tested-offline (RTP header, sender, reception statistics, SR/RR/SDES/BYE, all MS-RTP profile extensions, SDES "MS-EVT", probe/pair/train packets: `rtp`; audio media core with jitter buffer and concealment, packet pair and train sending rates, audio healer metrics, TURN bandwidth to the peer: `media`); bandwidth estimation from received pairs, peer info exchange, video feedback (PLI, VSR) – |
| 26 | Audio codecs G.711, G.722 | MS-RTP, MS-SDPEXT, ITU-T G.711, G.722 | B | tested-offline (G.711 against Tables 1–6; G.722 ADPCM bit-exact against all ITU digital test sequences, QMF round trip; used in calls: `g711`, `g722`, `media`) |
| 27 | Audio codecs RTAudio, Siren, G.722.1 | MS-SDPEXT, MS-RTP, ITU-T G.722.1 | T | – |
| 28 | Video H.264 UC/SVC | MS-H264PF, MS-SDPEXT | B | – |
| 29 | Video RTVideo | MS-RTVPF | N | not planned |
| 30 | DTMF, redundancy | MS-DTMF, MS-RTPRADEX | B | parts tested-offline (DTMF per RFC 4733 within MS-DTMF, sending and receiving, CLI `/dtmf`: `rtp`, `media`, `call-media`); redundancy (MS-RTPRADEX) – |
| 31 | Picture control (freeze/refresh) | MS-XMLMC | B | – |
| 32 | Bandwidth management (call admission control) | MS-ICE2BWM, MS-TURNBWM | B | parts tested-offline (Check/Commit/Update attributes and responses, path rules: `turn-bwm`; bandwidth requests on the TURN allocation: `turn`; refused sites, 274/275: `ice-agent`; audio calls: check while gathering as the callee, commit, updates, cancel, policy bandwidth to the peer: `call-media`, CLI with `enableBWPolicyCheck`); video streams, PSTN failover – |
| 33 | PSTN telephony | MS-OCPSTN, MS-SIPRE | B | parts tested-offline (dialling numbers with the location profile as phone-context, E.164, incoming gateway calls with asserted number, anonymous callers, call back, 605 loop rejection, Content-ID/ms-accepted-content-id, early media from gateways: `session`, `provisioning`, CLI `/call 0401234` `/callback`); media bypass, client-side dial rules – |
| 34 | Forwarding, simultaneous ring, team and delegate calls | MS-SIPRE, MS-PRES | B | – |
| 35 | Hold, music on hold | MS-SDPEXT, MS-SIPRE | B | parts tested-offline (hold/resume re-INVITE with inactive/sendrecv and `+sip.rendering="no"`, incoming hold and music on hold recognised, no RTP while held: `session`, `sdp`, `media`, `call-media`; CLI `/hold` `/resume`); streaming our own music on hold – |
| 36 | Call transfer | MS-SIPAE, MS-SIPRE | B\* | – |
| 37 | Call park | MS-SIPAPP | B\* | – |
| 38 | Voicemail | MS-OCEXUM, MS-EUMSDP | T | – |
| 39 | Emergency calls with location | MS-E911WS, MS-SIPRE | B\* | – |
| 40 | Schedule meetings | MS-CONFPRO | B\* | – |
| 41 | Join meetings, lobby, roles | MS-CONFBAS | B\* | parts tested-offline (join by focus URI, invitation or meeting link with the join manager document, lobby wait and admission, granted role, roster, leave, presenter commands: admit/deny from the lobby, roles, lock, remove: `conf`, `session`, CLI `/join` `/admit` `/promote` `/lock` …); scheduling is function 40, admission policy changes, recording – |
| 42 | A/V conferences, muting | MS-CONFAV | B\* | parts tested-offline (audio in the A/V MCU (SRTP sent, SRTP or SSRTP received), participants' audio state, muting ourselves and others (modifyEndpointMedia), muting the audience (mediaFiltersRules): `conf`, `session`, `call-media`, CLI `/confaudio` `/confmute` `/muteuser` `/muteall`); announcements language, dial-out of phone numbers, video – |
| 43 | Desktop and application sharing | MS-CONFAS, MS-RTASPF, MS-RDPBCGR, MS-RDPEMC | T | – |
| 44 | Whiteboard, PowerPoint, polls | MS-PSOM | T | – |
| 45 | Quality reports and call rating | MS-QoE | B\* | – |
| 46 | Error diagnostics and display | MS-OCER | B | – |
| 47 | Response group agent | MS-RGSWS | B | – |
| 48 | UCWA web services | MS-CVWREST, MS-ECREST, MS-OCSMP | B | – |
| 49 | Conversation history in Exchange | MS-OXWSCDATA, MS-OXWSSRCH | T | – |
| 50 | Sign-out | MS-SIPREGE, MS-CONFBAS | B | tested-offline (REGISTER with Expires 0, unsubscribe, ending conversations, group conversations and calls: `register`, `session`) |
| 51 | Audio codec SILK | MS-SDPEXT, draft-vos-silk-02 | T | – |
| 52 | File transfer 1:1 | none public | N | needs captures |

**Totals (2026-10-31, measured from this table):** evidence B 24 · B\* 17 · T 9 · N 2 · implemented: **10** tested-offline as a whole, **14** in parts, **26** not started, 2 not planned/needs captures. Nothing is `verified-interoperable` in this document; interop runs belong to the owner ([`interop-tests.md`](interop-tests.md)).
