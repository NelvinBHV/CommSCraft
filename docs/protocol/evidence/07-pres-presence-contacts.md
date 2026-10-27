# Evidence: MS-PRES – enhanced presence, contacts, containers, delegation, unified contact store

Source: [MS-PRES] "Presence Protocol", edition 20250520, rev. 13.0, 257 pp. Applies up to SfB 2021 / LTSC 2024 (§11, p. 247).

## Model

| ID | Statement | Evidence |
|---|---|---|
| PR-01 | Enhanced presence (`application/msrtc-event-categories+xml`): publication of individual **categories** instead of one document; identity of a publication = (publisher, container, category, instance) + server-side version | §1.3, pp. 16–17; §1.3.1.1, pp. 17–19 |
| PR-02 | expireType: `static`, `endpoint` (while the endpoint is registered; `endpointId` = UUID from +sip.instance), `user`, `time` (with `expires`) | §1.3.1.1, p. 19; §2.2.2.1, p. 33 |
| PR-03 | Container = ACL: members `user`/`domain`/`sameEnterprise`/`federated`/`publicCloud`/`everyone`; container 0 = default for everyone, membership cannot be changed; the highest matching container number wins; precedence URI > domain > enterprise > federated > public cloud > everyone; no match ⇒ empty value (blocked) | §1.3.1.3.1–1.3.1.3.7, pp. 19–21; §3.2.5.3, p. 92 |
| PR-04 | Private categories (e.g. `routing`, `mwi`, `legacyInterop`, `workingHours`, `dndState`, `linkedPICContacts`) visible only to the own user | §1.3.1.2, p. 19; §2.2.2.7.6–2.2.2.7.12, pp. 66–73 |
| PR-05 | Versioning: create with `version="0"`; update/delete with the current version; conflict ⇒ **409** with `application/msrtc-fault+xml`, `Faultcode Client.BadCall.WrongDelta`, `curVersion` ⇒ the client retries with this version; delete = `expires="0"`; batches are atomic | §1.3.1.6, p. 25; §2.2.2.2.3, pp. 35–36; §3.2.5.1.2, pp. 90–91; §3.2.5.4, pp. 92–93 |
| PR-06 | Re-created instance: accept the notification despite a lower version if `publishTime` is newer | §1.3.1.6.4, p. 25 |
| PR-07 | The server aggregates the state; the client publishes only input data | §1.3.1.7, p. 26; §3.8, pp. 114 ff. |

## Messages (client)

| ID | Message | Content | Evidence |
|---|---|---|---|
| PR-10 | REGISTER precondition | `Supported: msrtc-event-categories` MUST be in the REGISTER | §3.2.5.1.1, p. 89 |
| PR-11 | **Publish** | SERVICE to the own AOR (To = From = `publications uri`), `Content-Type: application/msrtc-category-publish+xml`, `<publish xmlns=".../2006/09/sip/rich-presence"><publications uri=…><publication categoryName instance container version expireType [expires]>`; response 200 with `application/vnd-microsoft-roaming-self+xml` (`roamingData/categories`) | §2.2.2.2.1–2.2.2.2.2, pp. 33–35 |
| PR-12 | Publishing errors | 400 (XML), 403 (From≠To, read-only, unregistered category), 409 (version), 413 (size), 488 (`endpoint` without registration) | §3.2.5.4, pp. 92–93 |
| PR-13 | **Self SUBSCRIBE** | `Event: vnd-microsoft-roaming-self`, Accept/Content-Type `application/vnd-microsoft-roaming-self+xml`, body `<roamingList><roaming type="categories|containers|subscribers"/><roamingEx type="delegates"/>`, with BENOTIFY/piggyback; exactly one dialog per endpoint; the response contains all own publications, containers, subscribers, delegates; every change (also from other own endpoints) is reported ⇒ multi-device synchronisation | §1.3.1.5, p. 24; §2.2.2.3, pp. 36–40; §3.3.5.1–3.3.5.2, pp. 96–97 |
| PR-14 | **Subscribe to contact presence (batch)** | SUBSCRIBE to the own AOR, `Event: presence`, `Accept: application/msrtc-event-categories+xml, application/rlmi+xml, multipart/related`, `Require: adhoclist, categoryList`, `Supported: eventlist`, `Content-Type: application/msrtc-adrl-categorylist+xml`, body `<batchSub xmlns=".../2006/01/sip/batch-subscribe"><action name="subscribe|unsubscribe"><adhocList><resource uri/>…</adhocList><categoryList><category name/>…` | §2.2.2.4.1, pp. 40–42; §3.4.5.1.1, p. 99 |
| PR-15 | Batch response | `multipart/related; type="application/rlmi+xml"`; first part an RLMI rejection list with `state=active|pending|terminated|resubscribe`, `reason`, `statusCode` (413 quotaExceeded), `poolFqdn` (pool GRUU ⇒ new batch SUBSCRIBE there); then one `application/msrtc-event-categories+xml` per contact | §2.2.2.4.2, pp. 43–44; §3.4.5.1.3.1, pp. 102–103 |
| PR-16 | Subscription strategy | one batch dialog per server/pool; external (federated) contacts: `state="resubscribe"` ⇒ **individual SUBSCRIBE** with To = contact (one resource only, `subscribe` only); on termination (NOTIFY Expires 0) move the resources into an existing dialog | §3.4.5.1.1, p. 99; §3.4.5.1.3, pp. 101–102; §3.4.5.1.3.1, pp. 102–103 |
| PR-17 | Polling query | SUBSCRIBE with `Expires: 0` without To tag = one-time snapshot | §3.4.5.1.2, pp. 99–100 |
| PR-18 | Report "added to contact list" | `<context><subscriptionContext xmlns=".../2008/09/sip/SubscriptionContext" majorVersion="1" minorVersion="0"><watcher><contactList/></watcher>[<delegation/>]` in the resource element | §2.2.2.4.1, pp. 41–42; §3.4.5.1.3, p. 101 |
| PR-19 | Acknowledge the subscriber list | SERVICE `application/msrtc-presence-setsubscriber+xml`, `<setSubscribers><subscriber user="…" acknowledged="true"/>`; one at a time only | §1.3.1.4, pp. 23–24; §2.2.2.6, pp. 48–50 |
| PR-20 | Container members | SERVICE `application/msrtc-setcontainermembers+xml`, `<setContainerMembers xmlns=".../2006/09/sip/container-management"><container id version><member action="add|delete" type value/>`; container 0 forbidden; 409 on version conflict | §2.2.2.5.2–2.2.2.5.3, pp. 46–48 |
| PR-21 | Delegation | SERVICE `application/msrtc-setDelegate+xml`, `<setDelegates version><delegate uri action="add|remove|modify"><publishOperation authorization="allow|deny"/><redelegateOperation …/>`; response 200 with `delegates`; 409 on version | §2.2.2.10, pp. 82–85 |
| PR-22 | Federated interop | The server delivers PIDF/RPID or msrtc.pidf to federated subscribers; mapping availability→avail table | §1.3.3, pp. 26–27; §2.2.2.8–2.2.2.9, pp. 77–81; table p. 114 |
| PR-23 | Directory search over SIP (`directorySearch` SOAP) | **OCS 2007/R2 only** – do not use for SfB | §2.2.1, pp. 31–32; <1>, <30> pp. 247–249 |

## Categories a client publishes or evaluates

| ID | Category | Core content | Evidence |
|---|---|---|---|
| PR-30 | `state` | `xsi:type` userState (manual), machineState (keyboard/mouse per endpoint), calendarState, phoneState, aggregateState, aggregateMachineState, presentingState; `availability` classes: 0–2999 unknown, 3000–4499 Online, 4500–5999 Idle, 6000–7499 Busy, 7500–8999 BusyIdle, 9000–11999 DND, 12000–17999 Away, ≥18000 Offline; `activity token`/`custom LCID`, `endpointLocation`, `meetingSubject/Location`, time zone, `device` computer/deskphone/mobile/web | §2.2.2.7.1, pp. 50–53 |
| PR-31 | Where to publish | The client publishes `state` in containers **2 and 3**, `device` + machineState in container 2; add NO members there; the server computes aggregateState/legacyInterop in 100/200/300/400 (different levels of detail) and `services` in 2, 100, 200, 300, 400 | §3.8.5.1.1, p. 117; §3.8.5.1.2.7, pp. 123–125; §3.8.5.2.1, p. 126 |
| PR-32 | Aggregation rules (for display) | aggregateMachineState = most active machineState (lowest availability, otherwise newest), without instances 18500; a manual state discards older ones; maximum of availability; idle machine + Busy ⇒ +1500; activity with the highest minAvailability | §3.8.5.1.2.1–3.8.5.1.2.6, pp. 118–123 |
| PR-33 | `device` | `endpointId`, `capabilities` (like services), `machineName`; do not use `timezone` | §2.2.2.7.2, pp. 53–55 |
| PR-34 | `services` (computed by the server) | capabilities per URI: text, voice, video, calendar, remoteCallControl, gifInk, isfInk, breakthrough, applicationSharing, ucs, containerIntegrity, contentWhiteboard, contentPowerPoint, contentNativeFile, contentPoll, contentSharedNotes; attributes render/capture/publish/version/deviceAvailability/preferredEndpointId | §2.2.2.7.3, pp. 55–60 |
| PR-35 | `userProperties` (server, container 1, read-only) | `lines/line lineType=Uc|UcPrivate|Rcc|Dual`, `telephonyMode` None/Uc/Rcc/Dual/NoAudioVideo (controls A/V functions), address data, `exumEnabled`/`exumURL` (voicemail), `acpInformation` (dial-in numbers) | §2.2.2.7.4, pp. 60–63; §3.2.6.2, p. 95 |
| PR-36 | `contactCard` | identity/name/displayName, email, address, company, department, title, office, url, phone (type, uri, displayString), automaton, type (person/huntgroup/autoattendant/automaton), description, `displayADPhoto`, `photo` (type default/enterprise/exchange/other, uri, hash) | §2.2.2.7.5, pp. 63–66 |
| PR-37 | `note` | `<note xmlns=".../2006/09/sip/note"><body type="personal" uri="">` (also OOF from Exchange, cf. SIPREGE `DisablePresenceNote`) | §2.2.2.1, p. 32; §2.2.2.2.1, p. 34 |
| PR-38 | `routing` (private) | Call forwarding preamble per MS-SIPRE §2.2.9 | §2.2.2.7.7, p. 67 |
| PR-39 | `calendarData` | `mailboxID`, `WorkingHours` (Exchange types namespace), `freeBusy` (Base64; instance ID bits 00 ⇒ 2 bits per block Free/Tentative/Busy/OOF; 01 ⇒ variable width with "Working elsewhere", 384 entries), `startTime`, `granularity`, `encodingVersion` | §2.2.2.7.8, pp. 67–70 |
| PR-40 | `mwi` (private, server) | messageWaiting, unread/readVoiceMailCount | §2.2.2.7.11, p. 72 |
| PR-41 | `linkedPICContacts` (private, client) | linked identities from PIDF NOTIFY (e.g. Skype consumer), primary identity for outgoing INVITEs | §2.2.2.7.12, pp. 72–73; §3.2.5.8, p. 94 |
| PR-42 | Persistent chat categories (Lync 2013/SfB only) | roomSetting, roomUpdate (last read msgid), roomInvitation (LastId), gcFilterSetting; `propertyList` namespace `.../2010/12/sip/propertyList`; notification flags 0x1–0x40 | §1.3.6, p. 28; §2.2.2.7.13, pp. 73–77; <29> p. 249 |

## Contact list and unified contact store

| ID | Statement | Evidence |
|---|---|---|
| PR-50 | The contact/group list is still kept via MS-SIP §3.7 (`vnd-microsoft-roaming-contacts`); MS-PRES extends it with UCS | §1.3.5, pp. 27–28; §3.10, p. 133 |
| PR-51 | UCS (from Lync Server 2013): a client with Exchange access sends `Supported: ms-ucs` (and `ms-ucs-ready` to trigger it) in the contact SUBSCRIBE; `contactList ucsMode="allowed|disabled|migrated"`; read-only during migration; after migration: BENOTIFY `subscription-state: terminated`, `ms-diagnostics-public: 2186`; further SUBSCRIBEs with `ms-ucs` ⇒ **488** with `ms-diagnostics: 2165` ⇒ contacts directly from Exchange; clients without Exchange access send no `ms-ucs` ⇒ the server proxies | §3.10.4, p. 134; §4.14, pp. 197–200; <49> p. 250 |
| PR-52 | Contact with a phone number: `contactExtension/phone/uri tel:` | §4.14.1, p. 198 |

## Open points / follow-up checks

- K-PR-1 (found 2026-10-27): §3.2.5.1.2/§2.2.2.2.3 name the fault code `Client.BadCall.WrongDelta`; the 409 example §4.2.1 (p. 138) prints `Protocol client.BadCall.WrongDelta` (evidently a global replacement of "client" in the document). The example's batch answer §4.5 (p. 179) also lacks the multipart close delimiter, and the contact list example §4.14.1 (p. 197) has a space inside a SIP URI (`+14252057570@ contoso.com`). Decision: accept any fault code ending in `BadCall.WrongDelta`; tolerate a missing close delimiter at the end of a Content-Length-framed body; skip contact entries with unusable URIs (`crates/presence`).

- O-PR-1: Which Exchange interface the client uses for UCS contacts (EWS `GetImItemList`/`AddImContactToGroup` or similar) is not described in MS-PRES → check the Exchange documents.
- PR-60 (checked): The example manifest in MS-SIPREGE §4.2.2 (pp. 99–112) defines containers 0, 100, 200, 300, 400, 32000; role `buddy` ⇒ visible containers, default 100 or 200 (`moveToDefaultContainer`); role `delegate` ⇒ only 300; types user/domain/sameEnterprise/federated/publicCloud in **32000** (with `removeFromContainer`). Hence 32000 is the blocking container (inference from the rule structure, no explicit text). The UI names were not documented in the specifications (O-PR-2).

## Later findings (file 39)
- O-PR-1 **closed** (OB-04, normative: MS-OXWSCOS, evidence file 37).
- O-PR-2 **closed** (OB-05, official: Lync 2013 SDK schema reference). Container 0 Default, 1 Self, 2/3 Aggregation, 100 External Contacts, 200 Colleagues, 300 Workgroup, 400 Friends and Family, 32000 Blocked Contacts.
- Implementation (2026-10-27, `crates/presence`): one-time batch presence queries (PR-14/PR-17, `Expires: 0`), contact list query (SIP-07), publishing the manual userState into containers 2 and 3 (PR-11/PR-31) with `expireType="endpoint"` so it ends with the session, 409 retry with `curVersion` (PR-05). The userState instance 603979776 (0x24000000) is taken from example §4.3.1.1 (p. 143; inference – the specification fixes no value). REGISTER now carries `Supported: gruu-10, adhoclist, msrtc-event-categories` (PR-10, RG-03).
- Correction (2026-10-27): the contact list is fetched with a real subscription, as in MS-PRES §4.14 (p. 197) and MS-SIP §3.7 – the one-time query with `Expires: 0` is documented only for presence (§3.4.5.1.2), and a server may answer it without a document. The CLI subscribes, takes the list from the piggybacked 200 OK or the first NOTIFY (signed, answered with a signed 200), and ends the subscription with an in-dialog SUBSCRIBE `Expires: 0` (RFC 3265 §3.3.4). Dialogs per RFC 3261 §12 in `crates/sip-msg` (`dialog`).
