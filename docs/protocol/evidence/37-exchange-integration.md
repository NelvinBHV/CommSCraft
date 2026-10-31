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
