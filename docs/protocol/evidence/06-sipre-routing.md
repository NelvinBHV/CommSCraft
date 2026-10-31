# Evidence: MS-SIPRE – SIP routing extensions

Source: [MS-SIPRE] "Session Initiation Protocol (SIP) Routing Extensions", edition 20250520, rev. 13.0, 116 pp.

## Endpoint identification

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| RE-01 | The client MUST use EPID, +sip.instance or GRUU to identify the endpoint (SfB practice per SIPREGE: all three) | MUST | §3.1, p. 37 |
| RE-02 | `epid` = ≤ 16 hex characters (64-bit random or MAC), persistent across restarts; the ABNF allows 1–16 tokenchars | SHOULD | §2.2.7, p. 25; §3.2, §3.2.3.1, p. 38 |
| RE-03 | `epid` in **every** From header of every request; remote epid in the dialog state; in follow-up requests in the To header | MUST | §3.2.4.1, pp. 38–39 |
| RE-04 | Incoming request with a foreign `epid` in To ⇒ drop without response | MUST | §3.2.5.1, p. 39 |
| RE-05 | Targeted call to a specific endpoint: AOR in Request-URI/To + `epid` in To (epid from an earlier dialog or the presence document) | SHOULD | §3.2.4.1, p. 39 |
| RE-06 | `+sip.instance="<urn:uuid:…>"` in the Contact of every message that needs a Contact; UUID URN only | MUST | §2.2.5, pp. 23–24; §3.3, §3.3.4.1, pp. 40–41 |
| RE-07 | If EPID is used as well, the instance UUID MUST be **name-based, derived from the EPID**; namespace UUID `fcacfb03-8a73-46ef-91b1-e5ebeeaba4fe`; input = ASCII of the epid value | MUST | §3.3.3.1, pp. 40–41 |
| RE-08 | Registrar/proxy check this derivation and otherwise answer 400 | MUST/SHOULD | §3.3.5.1–3.3.5.2, pp. 41–42 |
| RE-09 | GRUU only from the registrar (MS-SIPREGE); GRUU in the Contact of dialog-creating requests, **not** in the REGISTER Contact; optional `grid` parameter; requests with a GRUU go to the proxy of the GRUU domain; same epid as at registration | MUST | §3.4, §3.4.3.1, §3.4.4.1, pp. 42–43 |
| RE-10 | GRUU forms: user `sip:aor;opaque=user:epid:<b64>;gruu`, voicemail `opaque=app:voicemail`, location profile `opaque=app:locationprofile:get` (+`default` or `phone-context`), conference `opaque=app:conf:{focus|audio-video|chat|meeting|phone-conf}:id:<id>`, server `sip:<fqdn>@<domain>;gruu;opaque=srvr:{HomeServer|MediationServer|MRAS|QoSM}:<id>` | syntax | §2.2.2, pp. 20–21; §2.2.3, pp. 22–23; §3.4.5.1, pp. 43–45 |

### Recomputed (EPID → +sip.instance)
Example §4.2 (p. 81): epid `01010101` → `4b1682a8-f968-5701-83fc-7c6741dc6697`. Our recomputation: **SHA-1**(namespace as little-endian GUID bytes `03 fb ac fc 73 8a ef 46 91 b1 e5 eb ee ab a4 fe` ‖ `"01010101"`), version nibble 5, variant 10, output as a little-endian GUID gives exactly this value. SHA-256 gives `c033428a-…` and does not match.
- **K-RE-1 (conflict, documented):** §3.3.3.1 step 2 (p. 41) requires SHA-256; the normative example §4.2 (p. 81) shows and yields SHA-1. The implementation must use SHA-1 to agree with the example (and with servers that check the derivation, RE-08). Interop test by the owner required.

## NAT/firewall help and header parameters

| ID | Requirement | Evidence |
|---|---|---|
| RE-20 | Contact with the own IP (host or `maddr`) ⇒ append `;proxy=replace`; the server replaces address/port with those of the connection and adds `ms-received-cid` | §2.2.5, p. 23; §3.5.4.1, p. 47; §3.5.5.1, pp. 47–48 |
| RE-21 | Via parameters `received`, `ms-received-port`, `ms-received-cid`; Contact/Route parameters `ms-opaque`, `ms-fe`, `ms-route-sig`, `ms-key-info`, `ms-identity`, `ms-role-rs-to/from`, `ms-ent-dest` must be taken over unchanged by the client into the dialog state/route | §2.2.2.1–2.2.2.3, pp. 21–22; §2.2.6, pp. 24–25; §3.6, pp. 48–50 |
| RE-22 | The proxy rejects requests with outdated HMAC-protected routes with 481 (or 430) | §3.6.5.1, pp. 50–51 |

## Dialog recovery (from Lync 2010)

| ID | Requirement | Evidence |
|---|---|---|
| RE-30 | Enable recovery for valuable dialogs (calls): `Supported: ms-dialog-route-set-update` in all requests of the dialog; negotiate a session timer (RFC 4028); not for presence subscriptions | §2.2.19, p. 31; §3.7.3.1, §3.7.4.1, p. 52 |
| RE-31 | 430 Flow Failed with `P-Dialog-Recovery-Action`: `Registration-Route-Set-Update` ⇒ first a REGISTER refresh, then recover all dialogs; `Dialog-Route-Set-Update` ⇒ target refresh (UPDATE or SUBSCRIBE), take route set/remote target from the response, resend the original; `Wait-For-Session-Update` ⇒ recovery timer; give up after 2 failed attempts | §2.2.18, p. 30; §3.7.5.1, pp. 52–53; §3.7.5.3.1, pp. 53–54; §3.7.5.3.4, pp. 54–55 |
| RE-32 | REGISTER refresh with `presence-state: register-action="added"/"fixed"` ⇒ recover all recoverable dialogs, rebuild the others | §3.7.5.3.2, p. 54 |
| RE-33 | Incoming target-refresh requests: update route set/remote target | §3.7.5.3.3, p. 54 |
| RE-34 | A recovered dialog ⇒ related dialogs (e.g. conference) as well | §3.7.5.3.4, p. 55 |

## Dial rules (location profile)

| ID | Requirement | Evidence |
|---|---|---|
| RE-40 | Default profile name from in-band provisioning (`ucLocationProfile`/`ucUserLocationProfile`) | §3.8.3.1, p. 56; SIPREGE §2.2.2.5.1 p. 27 |
| RE-41 | SERVICE to `sip:<aor>;opaque=app:locationprofile:get;default;gruu` (or `phone-context=`), From = own AOR, `Accept: application/ms-location-profile-definition+xml` | §3.8.4.1, p. 56 |
| RE-42 | Response: `<Name>`, `<Rule>` (Pattern, Translation – .NET regex per [MC-RegEx]; InternalEnterpriseExtension, ApplicableForDeviceDialing), ExternalAccessPrefix, OptimizeDeviceDialing; errors 404/485 | §2.2.8, pp. 25–26; §3.8.5.1, pp. 56–57; schema §7, p. 100 |

## Call forwarding/ring groups (routing preamble)

| ID | Requirement | Evidence |
|---|---|---|
| RE-50 | The client publishes the preamble in the presence category `routing` (MS-PRES §2.2.2.7.7): `<routing name="rtcdefault" version="1|2"><preamble>` with `flags name="clientflags"`, `list` (forwardto, simultaneous_ring, team, delegates, breakthrough), `wait` (total, user, team2; 0–1200 s) | §2.2.9, pp. 26–28; §3.9, pp. 57–58; §3.9.5.1.1–3.9.5.1.4, pp. 59–61 |
| RE-51 | Flags: block, work_hours (calendar working hours), forward_immediate, simultaneous_ring, enablecf, delegate_ring, team_ring, skip_primary, forward_audio_app_invites, e911active | §3.9.5.1.2, pp. 59–60 |
| RE-52 | Server logic (for the UI): default 20 s ringing, then voicemail; total default 15 s; forwarding timer 60 s; DND ⇒ voicemail; 183 with `Ms-Forking: Active`, 101/181 status messages to the caller; CANCEL with `Reason: …;ms-acceptedby=` | §3.9.2, pp. 58–59; §3.9.5.2.2, pp. 62–63; §3.9.5.6, §3.9.5.9, p. 65 |
| RE-53 | `Ms-Sensitivity: normal|private|normal-no-diversion|private-no-diversion` controls diversion | §2.2.10, p. 28; §3.9.5.2.1, pp. 61–62 |
| RE-54 | `History-Info` with `ms-retarget-reason` (forwarding/team-call/delegation), `ms-line-type=private`, `ms-target-phone` to display "forwarded from" | §2.2.17, p. 30; §3.9.5.10, pp. 65–67 |

## Other client-relevant extensions

| ID | Requirement | Evidence |
|---|---|---|
| RE-60 | `ms-edge-proxy-message-trust` (ms-source-type, ms-source-network federation/publiccloud …) ⇒ mark external/federated/PIC contacts; the client must never set the header | §2.2.15, p. 29; §3.10, pp. 68–70 |
| RE-61 | `ms-user-logon-data: RemoteUser` in the REGISTER response ⇒ use external web URLs (DLX, ABS, calendar), do not assume a direct media connection | §2.2.16, pp. 29–30; §3.11.5.2, p. 71 |
| RE-62 | `Ms-Correlation-Id` (UUID) for related dialogs (B2BUA, transfer) | §2.2.12, p. 28; §3.12, pp. 72–73 |
| RE-63 | Conversation note `application/ms-conversation-context+xml` in the INVITE (id, from, to, participants, date, conversationId, dataFormat=text/plain, contextData, mode audio/video/im/applicationSharing) | §2.2.20, pp. 31–35; §3.13.5.1, pp. 74–75 |
| RE-64 | Safe transfer: `Supported: ms-safe-transfer` in INVITE and 200; take `Ms-Sensitivity` from Refer-To | §2.2.19, p. 31; §3.14, pp. 75–76 |
| RE-65 | **INVITE body with SDP**: `multipart/alternative` (ICEv6 SDP with `Content-Disposition: …;ms-proxy-2007fallback` + ICEv19 SDP as the last part), with a conversation note one level deeper in `multipart/mixed`; prefer ICEv19 when receiving; on 415 resend only the ICEv6 SDP | §3.15, pp. 76–78; §2.2.14, p. 29; examples §4.12, pp. 89–93 |
| RE-66 | Agent anonymity: `P-Agent-On-Behalf-Of` when calling on behalf; `Ms-Call-Info: rgs.anonymization` | §2.2.21–2.2.22, pp. 35–36; §3.16, pp. 78–79 |
| RE-67 | **Emergency call**: INVITE with `Priority: emergency`, `Geolocation` header, `Supported: geolocation`, PIDF-LO (RFC 3863/4119/5139, `method=Manual` for an unverified address) + `msftE911PidfExtn` (LocationPolicyTagID); before that a time-limited `routing` publication with `e911active` | §2.2.23, p. 36; §3.17.5.1, p. 80; schema §9, p. 102; example §4.14, p. 94 |
| RE-68 | Music on hold: `sip.rendering` feature tag (RFC 4235) together with MS-SDPEXT §3.1.5.27 | §2.2.5, p. 24 |
| RE-69 | `Content-Disposition` types render/session/icon/alert | §2.2.14, p. 29 |
