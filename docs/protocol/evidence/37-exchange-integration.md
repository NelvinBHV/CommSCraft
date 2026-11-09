# Evidence: the client's Exchange integration

Sources (Exchange package, 129 documents, edition 20250520). **Only five** Exchange documents name the SfB/Lync client explicitly in their product list: MS-OXWSCOS, MS-OXWSPERS, MS-OXWSPHOTO, MS-OXWSSRCH, MS-OXOCNTC (machine search over all 129 texts). All other Exchange functions are **not documented** for the SfB client.

## Documented

| ID | Requirement | Evidence |
|---|---|---|
| EX-01 | **Contact list in Exchange (UCS)** from Exchange 2013 + Lync Server 2013 (SIP side: MS-PRES `ms-ucs`, file 07 PR-51); with Exchange 2010/Lync Server 2010 contact objects per MS-OXOCNTC instead | OXOCNTC <1>, p. 70 |
| EX-02 | UCS = folder **ImContactList** (groups as personal distribution lists, `IPF.Contact.MOC.ImContactList`) and **QuickContacts** (contacts, `IPF.Contact.MOC.QuickContacts`); group types `IPM.DistList.MOC.Tagged`, `.OtherContacts`, `.UserGroup`, `.DG`, `.Favorites` (at most one each of Tagged/OtherContacts/Favorites) | OXWSCOS §1.3, p. 11; §2.2.4.2, pp. 14–15; OXOSFLD folder table |
| EX-03 | EWS SOAP 1.1 (HTTPS SHOULD), namespaces `…/exchange/services/2006/types|messages`, SOAPAction `http://schemas.microsoft.com/exchange/services/2006/messages/<Operation>`; headers RequestServerVersion, MailboxCulture, ExchangeImpersonation; authentication by the web server | OXWSCOS §1.7, p. 12; §2.1–2.2.1, p. 13; §3.1.4.6, pp. 40–44 |
| EX-04 | Operations: GetImItemList (everything), GetImItems, AddImGroup, SetImGroup, RemoveImGroup, AddImContactToGroup, AddNewImContactToGroup, AddNewTelUriContactToGroup, RemoveImContactFromGroup, RemoveContactFromImList, Add/RemoveDistributionGroup(To/From)ImList, SetImListMigrationCompleted | OXWSCOS §3.1.4, pp. 17–18; §3.1.4.1–3.1.4.13, pp. 18–72 |
| EX-05 | Contacts in the result as **personas** (FindPeople, GetPersona) | OXWSCOS §2.2.4.3, p. 15; OXWSPERS §1.3, p. 8; §3.1.4.1–3.1.4.2, pp. 30–40 |
| EX-06 | **Photos**: HTTPS GET `https://<EWS>/ews/Exchange.asmx/s/GetUserPhoto?email=<address>&size=HR48x48…HR648x648`; base URL `ExternalPhotosUrl` via Autodiscover GetUserSetting (MS-OXWSADISC); ETag/If-None-Match ⇒ 304; 404 = no picture; authentication per **MS-OCAUTHWS** | OXWSPHOTO §1.5, p. 6; §3.1.5–3.1.5.1.1, pp. 8–10; example §4, p. 11 |
| EX-07 | Mailbox search FindFolder/FindItem (e.g. conversation history) | OXWSSRCH §3.1.4.1–3.1.4.2, pp. 44–49 |
| EX-08 | EWS endpoint via Autodiscover **SOAP (MS-OXWSADISC)** or **POX (MS-OXDSCLI)** | OXWSCOS §1.4–1.5, pp. 11–12; OXWSPERS §1.4, p. 8 |
| EX-09 | Folder "Conversation History" as distinguished folder `conversationhistory` (likewise `quickcontacts`, `imcontactlist`) | OXWSCDATA DistinguishedFolderIdNameType (§2.2.5.10) |

## Not documented (inference only)
- O-EX-1: Free/busy for the presence category `calendarData` (file 07 PR-39) – the format matches Exchange types (`WorkingHours`, MergedFreeBusy), but querying via **MS-OXWAVLS GetUserAvailability** is **not** documented for the SfB client (product list Exchange/Outlook only).
- O-EX-2: Out-of-office note as presence note – **MS-OXWOOF** names only Outlook.
- O-EX-3: Saving the conversation history (CreateItem in `conversationhistory`) – folder documented (EX-09), the SfB client's procedure not described.
- O-EX-4: Voicemail list/settings (MS-OXWUMS) – only Outlook named.
- O-EX-5: How the client authenticates to Exchange (NTLM/Negotiate, Basic, OAuth, MS-OCAUTHWS ticket) is stated only for photos (OCAUTHWS); otherwise "the web server authenticates".
These points are dispensable for a first Linux client or need an interop test.

## Later findings (file 39)
- O-EX-1/O-EX-2 **secondary evidence** (OB-13/OB-14): GetUserAvailability (DetailedMerged, 15 min) and GetUserOofSettings; the presence format is officially documented (Lync 2013 SDK).
- O-EX-5 **secondary evidence** (OB-15): Exchange Autodiscover (POX), then Negotiate/NTLM, Basic only as the last fallback.
- O-EX-3 **partly** (OB-22): stored in each participant's "Conversation History" folder as a mail item (Lync 2010 SDK, official); server-side recording from Lync 2013 via `EnableServerConversationHistory` (MS-SIPREGE pp. 55/127).
- O-EX-4 **narrowed** (OB-23): voicemail message classes per MS-OXOUM §2.2.2.1, p. 10.
- O-EX-1, O-EX-2, O-EX-4, O-EX-5 **closed** (OB-48, normative): Autodiscover gives `ASUrl`, `EwsUrl` and `AuthPackage` (MS-OXDSCLI pp. 18–23); the EWS operations are server interfaces. O-EX-3 closed by design (OB-49).

## Implementation (2026-11-09, photos from Exchange)
- **Autodiscover (EX-08):** `ews::user_settings_request` asks GetUserSettings ([MS-OXWSADISC] §3.1.4.4, p. 49; example §4.3, pp. 80–82) for `InternalEwsUrl`, `ExternalEwsUrl`, `InternalPhotosUrl`, `ExternalPhotosUrl` at U1 `https://autodiscover.<domain>/autodiscover/autodiscover.svc`, then U2 `https://<domain>/…` (§1.5, pp. 11–12). The answer is read with the ErrorCode values of §3.1.4.4.4.2 (p. 75): RedirectAddress and RedirectUrl (HTTPS only) are followed at most three times (design), other codes are errors. Authentication: NTLM, as for the other web services (inference from OB-48/O-EX-5; the server offers Negotiate and NTLM).
- **Photos (EX-06):** `/photo` with the photo policy `AllPhotos` (RG-52) first asks Exchange GetUserPhoto (size HR240x240) for the contact's mail address from the address book search, then falls back to the directory photo (file 28). Sent back: the ETag in If-None-Match; 304 keeps the cached photo, 404 means no Exchange photo. The photo is kept in the user's cache directory (0700/0600), JPEG and PNG only, within `MaxPhotoSizeKB`.
- **K-EX-1 (found 2026-11-09):** [MS-OXWSPHOTO] §3.1.5.1.1 (p. 9) names the URL `…/ews/Exchange.asmx/s/GetUserPhoto` and says the photos URL holds "the location of the Exchange.asmx HTTP handler", without saying whether it ends in `/s`. Exchange returns the photos URL with `/s` (`tested-in-lab`); `ews::photo_url` adds `/s` only when it is missing, so the EWS URL works as well.
- **Server behaviour (`tested-in-lab`, no details):** without a photo stored in the mailbox, GetUserPhoto returns the directory thumbnail; with one, the stored photo. No ETag was sent, so the 304 path is `tested-offline` only.
- **Status:** `tested-offline`, `tested-in-lab`.

## Implementation (2026-11-09, contact list in Exchange)
- **Operations (EX-04):** `ews::ucs` builds GetImItemList (§3.1.4.6, pp. 40–44), AddImGroup (§3.1.4.3, p. 27), SetImGroup (§3.1.4.12, p. 65), RemoveImGroup (§3.1.4.11, p. 61), AddNewImContactToGroup (§3.1.4.4, p. 31), AddImContactToGroup (§3.1.4.2), RemoveImContactFromGroup (§3.1.4.10, p. 57) and RemoveContactFromImList (§3.1.4.8, p. 48) as SOAP 1.1 with RequestServerVersion (examples §4, pp. 73–75), and checks ResponseClass and ResponseCode of every answer. Groups (ImGroupType, §2.2.4.2) and contacts (personas, EX-05) are read bounded.
- **CLI:** when the list moved (file 07, PR-51), `/contacts` shows the list from Exchange with numbered groups; `/contact add|remove|move` and `/contactgroup add|rename|remove` become the operations above, then the list is read again. Authentication: NTLM, as for Autodiscover.
- **K-EX-2 (found 2026-11-09):** a group's MemberCorrelationKey (§2.2.4.2) names the contact items behind the personas – `Attributions/Attribution/SourceId` of [MS-OXWSPERS] – not the PersonaId; these items are also the ContactId the operations take (`tested-in-lab`). An IM address may come without `sip:` (a list moved from the server); the client adds it and checks the address.
- **Status:** `tested-offline`, `tested-in-lab` (move of the list, read, add/rename/remove group, add/move/remove contact).

## Implementation (2026-11-09, calendar and out-of-office state, function 16)
- **Reading (O-EX-1/O-EX-2, OB-13/OB-14/OB-48):** `ews::calendar` asks GetUserAvailability ([MS-OXWAVLS] example §4.1, pp. 46–47) for the user's own mailbox with RequestedView DetailedMerged and 15-minute blocks over 384 blocks from the start of the UTC day (MS-PRES §2.2.2.7.8, p. 70: "SHOULD contain 384 entries"), and GetUserOofSettings ([MS-OXWOOF] example §4.1, p. 28).
- **Merged string:** the digits 0–4 (§3.1.4.1.4.1, pp. 42–43) become the 2-bit codes of MS-PRES; "no data" counts as free.
- **Ongoing meeting:** a Busy event at the current time. Tentative and out-of-facility events are left out (inference). Subject and location are taken only from events that are not private.
- **Automatic replies:** on while Enabled, or Scheduled with the current time in the duration. The internal reply is reduced to plain text (Exchange keeps it as HTML).
- **K-AW-3 (found 2026-11-09):** the request's time zone is UTC: bias 0 and both transitions without offset. With two equal transition dates the server refuses the zone as invalid. With the transition dates of the example §4.1 it answers (`tested-in-lab`).
- **Publishing (PR-30, PR-37, PR-39):** `presence::calendar` follows the Lync 2013 SDK page "Publication behaviors of category instances" (official, secondary) for instances and containers:
  - calendarData instance 0 (static): working hours into containers 1, 200, 300 and 400, empty into 100 and 32000; the same as the example MS-PRES §4.3.3 (pp. 154–156).
  - calendarData instance from a hash of the mailbox: the free/busy value into 200, 300 and 400, empty into 1, 100 and 32000.
  - calendarState `0x4…` (6500, `in-a-meeting`, subject and location) and the `out-of-office` activity `0x6…` (without availability, so it does not change the aggregated availability, §3.8.5.1.2.2) into containers 2 and 3.
  - The note (type `OOF`) into 200, 300 and 400, empty into 100 and 32000.
  - Endpoint-bound publications, unlike the SDK's time-bounded and static ones, so nothing stale remains after sign-out (design; §4.3.3 does the same for free/busy).
  - Only changed groups are published (§3.8.5.3, p. 129); an ended meeting or reply is deleted with `expires="0"`; 409 conflicts are retried with the versions they name (PR-05).
- **Server settings ([MS-SIPREGE] §2.2.2.5.6, pp. 50–52):** what the endpointConfiguration settings disable is neither read nor published:
  - `DisableCalendarPresence`: free/busy, meeting and working hours.
  - `DisableFreeBusyInfo`: the free/busy value.
  - `DisableMeetingSubjectAndLocation`: subject and location.
  - `DisablePresenceNote`: the automatic replies.
  - `CalendarStatePublicationInterval` replaces the 15-minute interval.
- **Contacts:** the aggregateState's meetingSubject and meetingLocation (§3.8.5.1.2.5, p. 122) and an OOF note are shown.
- **Status:** `tested-offline`, `tested-in-lab` (publication accepted; a watcher sees busy, in a meeting and the out-of-office note).
