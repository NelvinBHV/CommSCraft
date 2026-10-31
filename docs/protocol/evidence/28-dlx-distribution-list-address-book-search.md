# Evidence: MS-DLX – distribution list expansion and address book web search

Source: [MS-DLX] "Distribution List Expansion Protocol", edition 20250520, 70 pp. URL from in-band provisioning `dlxInternalUrl`/`dlxExternalUrl`/`dlxEnabled` or `abwqInternalURL`/`abwqExternalURL` (MS-SIPREGE, file 04).

| ID | Requirement | Evidence |
|---|---|---|
| DL-01 | SOAP over **HTTPS** (TLS only); server address from another channel (SIP provisioning); the client is a pure pass-through (no state/timers) | §1.4–1.5, pp. 10–11; §2.1, p. 12; §3, p. 15 |
| DL-02 | Authentication: Kerberos/NTLM via HTTP Negotiate (RFC 4559) – WSDL binding `AddressBookWinNegotiate`; additionally binding **`AddressBookWebTicketBearer`** (web ticket from MS-OCAUTHWS, file 09) | §5.1, p. 57; WSDL §6, pp. 63–64 |
| DL-03 | Namespace `DistributionListExpander`; SOAPActions `DistributionListExpander/IAddressBook/SearchAbEntry`, `…/SearchSkypeDirectory`, `…/ProvideSkypeSearchFeedback`, ExpandDistributionList (see K-DL-1) | WSDL §6, pp. 58–65 |
| DL-04 | **ExpandDistributionList**(groupMailAddress) ⇒ DlxGroup: ResponseStatus (Invalid, Success, MemberCountLimitExceeded (default >100), NotAuthorized, NotFound, SimultaneousRequestLimitExceeded (50 external/100 internal)), Users, NestedGroups each ActiveDirectoryObjectInfo (displayName, mail, mailNickname, sipUri) | §3.1.4.1–3.1.4.1.4.1, pp. 17–21; <4>/<6> pp. 66–67 |
| DL-05 | **SearchAbEntry** (from Lync 2010): exactly one of BasicSearch (SearchList, Value, Verb Equals/BeginsWith; comparison ignoring case and accents), ChangeSearch (≤100: AbEntryHash, PhotoHash, SearchOn, Value – only changes returned), OrgSearch (EntryId, OrgHash – org chart, position 0/−1/1/2) + Metadata (FromDialPad with key mapping, MaxResultNum default 20, ReturnList) | §3.1.4.2–3.1.4.2.4.1, pp. 22–31 |
| DL-06 | Mandatory attributes on the server side: EntryId, AbEntryHash, PhotoRelPath (relative to the ABS folder, file 27), PhotoSize, PhotoHash, OrgHash; otherwise AD attribute names | §3.1.1, pp. 15–16 |
| DL-07 | Response AbEntryResponse: Items/AbEntry (Attributes Name+Value or Values, EntryId, Position, SourceNetwork) + Metadata (ResponseCode Succeeded/NoEntryFound/InternalError/InvalidArgumentError/DatabaseError/CorruptionEntryError, MessageText) | §2.2.5.1, p. 13; §3.1.4.2.3.8–3.1.4.2.3.15, pp. 27–30 |
| DL-09 | Client request form (**inference** from the WSDL, document/literal): the body element is the wrapper `SearchAbEntry` with child `AbEntryRequest`; answer `SearchAbEntryResponse/SearchAbEntryResult`. `SearchAbEntryRequest.SearchMetadata` has `FromDialPad` and `MaxResultNum` without `minOccurs="0"` (required); `ReturnList` missing ⇒ server SHOULD answer InvalidArgumentError. Secondary: [SIPE] `sipe-svc.c` l. 406–426 sends exactly this form | schema §6, pp. 58–65; §3.1.4.2.3.6, p. 26 |
| DL-10 | HTTP authentication for the NTLM case (binding `AddressBookWinNegotiate`, DL-02): MS-NTHT – scheme `NTLM`, the first 401 carries no data, the client sends NEGOTIATE then AUTHENTICATE base64-encoded in `Authorization: NTLM …`; any status other than 401 means authentication succeeded. Connection-oriented NTLM: MIC = HMAC_MD5(ExportedSessionKey, NEGOTIATE ‖ CHALLENGE ‖ AUTHENTICATE) | [MS-NTHT] 20240423 §2.2.1–2.2.2, pp. 10–11; example §4, p. 12; [MS-NLMP] 20260330 §3.1.5.1.2, p. 47 |
| DL-08 | SearchSkypeDirectory/ProvideSkypeSearchFeedback (Skype consumer directory) only from SfB Server 2015 | §3.1.4.3–3.1.4.4, pp. 31 ff.; <9>/<10> p. 67 |

## Conflicts / gaps
- K-DL-1: The SOAPAction for ExpandDistributionList is printed with a space ("DistributionListExpander/ExpandDistribution List", p. 63), formed differently from the others (`…/IAddressBook/…`). Determine the correct string by interop test.
- K-DL-2: The security chapter (p. 57) names only Kerberos/NTLM, the WSDL additionally contains a web ticket binding – support both.
- K-DL-3: The examples of §4.4–4.6 (pp. 43–45) show `AbEntryRequest` directly in the body without the `SearchAbEntry` wrapper and without the required `FromDialPad`/`MaxResultNum`; the WSDL requires both. The client sends the WSDL form (DL-09) and accepts both answer forms. Confirm in an interop run.
- O-DL-1: The concrete endpoint URL/path and the mapping of the `dlx*`/`abwq*` URLs to operations are not specified explicitly (only "another channel").

## Later finding (file 39)
- O-DL-1 **secondary evidence** (OB-12): endpoint = the provisioned `dlxInternalUrl`/`dlxExternalUrl`; SOAPAction `DistributionListExpander/IAddressBook/SearchAbEntry`.
- O-DL-1 **closed** (OB-44, normative): the provisioned `dlx*`/`abwq*` URLs are the MS-DLX endpoint (MS-SIPREGE §2.2.2.5.1, pp. 27–28). K-DL-1 resolved: the `AddressBookWinNegotiate` binding (p. 65) prints `DistributionListExpander/ExpandDistributionList` without the space.
