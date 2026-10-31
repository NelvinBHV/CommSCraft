# Evidence: MS-CONFAV – audio/video conference (AVMCU)

Source: [MS-CONFAV] "Centralized Conference Control Protocol: Audio-Video Extensions", edition 20250520, 108 pp. Extends MS-CONFBAS (file 18). Dependencies MS-SDPEXT, MS-SIPRE, MS-ICE/ICE2, MS-RTP, MS-RTPRADEX (§1.4, p. 12). Product list p. 102 up to SfB LTSC 2024.

## Joining the AVMCU (client)

| ID | Requirement | Evidence |
|---|---|---|
| CA-01 | AVMCU URI = `conf-uris` entry with purpose `audio-video`; available conference media from `entity-view`(AVMCU)/`entity-state`/`media`/`entry` (label e.g. `main-audio`, `main-video`, `panoramic-video`) | §3.1.5.2, pp. 26–27; example MS-CONFBAS p. 128–129 |
| CA-02 | Explicit addUser dial-in **not needed**: an INVITE to the AVMCU conference URI creates an implicit addUser at the focus; if explicit, exactly one endpoint with exactly one of epid / endpoint-uri / sip-instance, optionally exactly one language | §3.1.5.1, p. 26; §2.2.3.2, pp. 23–24 |
| CA-03 | The AVMCU maps an INVITE to a dial-in: GRUU ⇒ endpoint-uri, otherwise EPID + From URI, otherwise +sip.instance; no match ⇒ **404**; no mappable/acceptable media line ⇒ **488** | §3.2.7.1.2, pp. 44–45; §3.2.7.1.2.1, p. 45 |
| CA-04 | SDP offer (MS-SDPEXT, file 14): per media type no more m= lines than conference media (excess ⇒ port 0); with `multi-view-capable=true` several m=video per SDPEXT §3.1.5.34/35 and the MIME structure of MS-SIPRE §3.15.4.1; optional `a=label:` with the conference label | §3.1.5.2.1, p. 27; §2.2.2.2.1.5, p. 22 |
| CA-05 | `Accept-Language` (exactly one language) ⇒ language of the entry/exit announcements | §3.1.5.2, p. 26 |
| CA-06 | Mapping conference medium ↔ user medium ↔ m= line via label, type (audio/video), status ↔ a=sendrecv/sendonly/recvonly/inactive; direction = intersection of status and SDP | §3.2.1.1–3.2.1.2, pp. 28–30; §3.2.5.1.3–3.2.5.1.4, pp. 36–38 |
| CA-07 | Re-INVITE: new m= lines are mapped, existing mappings stay fixed; port 0 ⇒ medium "Rejected" | §3.2.5.1.2, p. 36; §3.2.7.1.3, pp. 45–46 |

## AVMCU roster contents

| ID | Requirement | Evidence |
|---|---|---|
| CA-10 | entity-capabilities `msav:capabilities` (supports-audio, supports-video); namespace `http://schemas.microsoft.com/rtc/2005/08/avconfinfoextensions` | §2.2.1.1.3, p. 16; §3.2.3.1.1.1, p. 30 |
| CA-11 | entity-state: video-parameters/video-mode `dominant-speaker-switched` (default) or `manual-switched` (+ intended-primary-presenter-source); entry-exit-announcements (enabled, modifiable; `…/2009/03/commonmcuextensions`); mediaFiltersRules / conf-media-filters-rules (mayModifyOwnFilters, initialFilters per role, type audio/video/panoramic-video); presentation-mode-capable; multi-view-capable; video-presentation-mode-capable | §2.2.1.1.2–2.2.1.1.5, pp. 15–19; §2.2.2.2.1, pp. 21–23 |
| CA-12 | User endpoint (AVMCU): media with label, src-id (= SSRC), media-ingress-filter / media-egress-filter (block/unblock, MCU view), media-source-id (MSI, from Lync 2013), source-name (a=x-source) | §2.2.2.1, pp. 19–20; <4>/<5> pp. 102–103 |
| CA-13 | If the client supports video source requests (MS-RTP VSR, file 16), the MCU does **not** choose the video source itself; otherwise by dominant speaker or manual source | §2.2.1.1.2, p. 16; <2> p. 102 |

## AV control commands (C3P via the focus, file 18 CB-30 ff.)

| ID | Requirement | Evidence |
|---|---|---|
| CA-20 | **modifyEndpointMedia** (mute): mediaKeys (confEntity, userEntity, endpointEntity, mediaId) + media with media-ingress-filter/-egress-filter; "unblock" only if mayModifyOwnFilters of the role is true, otherwise unauthorized; a status change ⇒ the MCU sends a **re-INVITE** with the new direction; errors UserDoesntExist/EndpointDoesntExist/OtherFailure | §2.2.3.3, p. 24; §3.2.5.4, pp. 39–41 |
| CA-21 | **modifyConferenceAnnouncements** (enabled mandatory); only if modifiable=true, otherwise notSupported | §2.2.3.4, pp. 24–25; §3.2.5.5, p. 41 |
| CA-22 | **modifyConference** to the AVMCU (presenters only): mediaFiltersRules (e.g. mute all participants) – **without** re-INVITE, roster change only; video-parameters (manual-switched/spotlight) from Lync 2013 | §3.2.5.6–3.2.5.6.2, pp. 41–43; <19>/<20> pp. 103–104 |
| CA-23 | addUser dial-out to the AVMCU (e.g. add a phone number): exactly one endpoint, at most one of epid/endpoint-uri/sip-instance; the MCU sends an INVITE with an SDP offer, "pending", after 200 OK "success" | §2.2.3.1, p. 23; §3.2.5.2, p. 38; §3.2.7.1.1, p. 44 |

## Conflicts / gaps
- K-CA-1: MS-CONFBAS §3.11 (pp. 110–113) describes joining the MCU **with** a prior addUser dial-in by the client; MS-CONFAV §3.1.5.1 (p. 26) declares it **unnecessary** for A/V (implicit through the INVITE). Both paths are documented; implement the direct INVITE, explicit dial-in as a fallback (interop test).
- K-CA-2: Filter values are called `block`/`unblock` in the schema (§2.2.1.1.1.1, p. 15), "blocked"/"unblocked" in §3.2.5.6.1 (p. 42). Authoritative: the schema enumeration.
- L-10: SDP content beyond type/label/direction is "implementation-specific" (§3.2.5.1.1, p. 35) – MS-SDPEXT (file 14) is authoritative.
