# Evidence: MS-CONFAS – application/desktop sharing in conferences (ASMCU)

Source: [MS-CONFAS] "Centralized Conference Control Protocol: Application Sharing Extensions", edition 20250520, 50 pp. Extends MS-CONFBAS (file 18). Dependencies MS-SDPEXT (file 14, SD-80/81), MS-RTP, MS-ICE/ICE2, MS-RTASPF (file 17, PL-40 ff.), **MS-RDPBCGR**, **MS-RDPEMC** (§1.3–1.4, pp. 8–9). Product list p. 47 from OC 2007 R2 up to SfB LTSC 2024.

| ID | Requirement | Evidence |
|---|---|---|
| CS-01 | ASMCU URI = conf-uris purpose `applicationsharing`; INVITE to this URI (dial-in); dial-out is **not** supported by the ASMCU; Session-Expires 600 s | §2.2.2.1, p. 12; §3.1, p. 14; §3.2.5.1.1, p. 15 |
| CS-02 | SDP offer: at most **one** m=applicationsharing line per role; role mandatory `a=x-applicationsharing-role:sharer` or `viewer`; the MCU answers with the reversed role and `a=x-applicationsharing-session-id` | §3.2.5.1.2, p. 15; §3.3.5.2.1, p. 19 |
| CS-03 | Optional m=video for video-based screen sharing (SfB server from 2015): `a=label:applicationsharing-video`, sendonly (sharer) or recvonly (viewer); `a=x-applicationsharing-contentflow` sendonly/inactive (sharer) or recvonly/inactive (viewer); inactive only with an active m=video; violations ⇒ **488** | §3.2.5.1.2, pp. 15–16; <1>, <8> pp. 47–48 |
| CS-04 | Neither m=applicationsharing nor m=video in the initial offer ⇒ 488 | §3.2.5.1.2, p. 15 |
| CS-05 | ICE completion with final offer/answer (ICE re-INVITE) mandatory | §3.2.5.1.3, p. 16; §3.3.5.2.2, p. 20 |
| CS-06 | Role change/renegotiation: new m=applicationsharing, old line port 0 | §3.2.5.1.4, p. 16; §3.3.5.2.3, p. 20 |
| CS-07 | New sharer ⇒ the previous sharer gets a **BYE** with `ms-diagnostics-public:21000;reason="New sharer joined conference"` | §3.2.5.1.5, p. 16; §3.3.5.6, p. 24 |
| CS-08 | `a=x-capabilities:name="sendrecv|sendonly|recvonly|none";…` (names: lower case, _ , -); format error ⇒ 488; the server mirrors them as media-capabilities in the roster | §3.2.5.1.6, pp. 16–17 |
| CS-09 | RDP level: Attendee Connected PDU **GroupId = 3766552189**; FriendlyName in the Participant Created PDU = user@host of the endpoint URI | §3.2.5.2.1–3.2.5.2.2, p. 17; §3.3.5.3.1, p. 20 |
| CS-10 | Roster: media type `applicationsharing`, status sendonly (sharer) / recvonly (viewer), media-state `joining`→`connected`, session-id from entity-state/session-ids (`http://schemas.microsoft.com/rtc/2005/08/asconfinfoextensions`); label `applicationsharing` / `applicationsharing-video`; medium disconnected ⇒ media element missing | §2.2.1.1.1–2.2.1.2.1.2, pp. 11–12; §3.3.5.4–3.3.5.5, pp. 20–24 |
| CS-11 | Policies: permission `AllowUserToScheduleMeetingsWithAppSharing` false ⇒ INVITE rejected (ms-diagnostics-public 21001); `AttendeesCanShare` (attendee as sharer ⇒ 21002); capabilities `supports-application-desktop-sharing` desktop/singleApplication/none (with singleApplication the client must not share the desktop), `video-screen-sharing-enabled` | §3.3.5.7.1–3.3.5.7.3, p. 26; §3.3.5.4, p. 21 |

## Conflicts / gaps
- K-CS-1: §3.2.5.1.2 (pp. 15–16) uses both "sharer/viewer" and "sharing/viewing" for x-applicationsharing-role. The MS-SDPEXT ABNF (§3.1.5.18.2) allows only `sharer` | `viewer` ⇒ authoritative.
- K-CS-2: §3.3.5.7.2 (p. 26) ties "AttendeesCanShare" to server-mode = 13, MS-CONFPRO §2.2.1.2 declares server-mode obsolete (behaviour always like 14). For SfB, AttendeesCanShare = false is to be expected, if the server reports it at all – interop test.
- K-CS-3: The example on p. 22 uses the namespace `urn:params:xml:ns:conference-info-separator` (missing "ietf") and typographic quotes – example error.
- L-12: The actual screen/application transfer needs a complete **RDP implementation** (MS-RDPBCGR, MS-RDPEMC, possibly MS-RDPEGFX/RDPRFX); these Windows specifications are available and documented separately (file 36).
