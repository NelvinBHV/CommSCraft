# Evidence: MS-SIPREGE – registration, sign-out, in-band provisioning, version check

Source: [MS-SIPREGE] "Session Initiation Protocol (SIP) Registration Extensions", file `[MS-SIPREGE]-250819.pdf`, edition 20250819, rev. 17.1, 134 pp.
Dependencies: RFC 3261 §10 (REGISTER), RFC 3265 (SUBSCRIBE/NOTIFY), draft-ietf-sip-gruu-10, RFC 950, MS-SIPRE, MS-SIPAE, MS-CONMGMT, MS-PRES, MS-OCER, **MS-SIP** (contact/group subscription, batch subscribe, piggyback) – §1.4, p. 13.

## REGISTER request (client)

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| RG-01 | SIP over TCP or TLS; IPv6 from Lync 2013 | MUST | §2.1, p. 14; <1> p. 119 |
| RG-02 | At most one `Event: registration` (implicit subscription to the registration state) | MAY/MUST | §2.2.1.1, p. 14 |
| RG-03 | `Supported`: `gruu-10`, `adhoclist` (batch subscribe), `com.microsoft.msrtc.presence`, `msrtc-event-categories` (enhanced presence), `ms-userservices-state-notification` (survivable mode), `ms-cluster-failover`; do not send `ms-forking` | SHOULD | §2.2.1.5, pp. 15–16 |
| RG-04 | `ms-keep-alive` at most once | MAY | §2.2.1.1, p. 14 |
| RG-05 | REGISTER must fulfil MS-SIPRE §3.4 (GRUU/endpoint); create EPID and `+sip.instance` before the first REGISTER and use them | MUST | §2.2.1.1, p. 14; §3.1.1.3–3.1.1.4.1, p. 75 |
| RG-06 | `ms-subnet: <IPv4 subnet address of the TCP connection>` | SHOULD/MUST | §2.2.1.6, p. 16; §3.1.1.4.1, p. 75 |
| RG-07 | `Ms-Device-Info: MAC=<aa-bb-…>,vendor=…,version=…` (from Lync 2013) | SHOULD | §2.2.1.7, p. 16; §3.1.1.4.1, p. 75; <8> p. 120 |
| RG-08 | `User-Agent` with product version `Major.Minor.Build(≥4 digits).QFE`, e.g. `UCCAPI/3.5.6889.0 OC/3.5.6889.0` | SHOULD | §3.4.1.5.1, p. 84 |
| RG-09 | No wildcard Contact to remove all bindings | SHOULD NOT | §2.2.1.10, p. 17 |
| RG-10 | Exactly one active REGISTER dialog per EPID; establish the SA per MS-SIPAE | MUST | §3.1.1, p. 74 |
| RG-11 | Contact example: `methods="INVITE, MESSAGE, INFO, OPTIONS, BYE, CANCEL, NOTIFY, ACK, REFER, BENOTIFY";proxy=replace;+sip.instance="<urn:uuid:…>"` | example | §4.1.1, p. 94 |

## Processing the REGISTER response

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| RG-20 | Take the GRUU from Contact (`gruu="sip:alice@…;opaque=user:epid:…;gruu"`) and use it later | SHOULD | §3.1.1.5.1, p. 75; example p. 95 |
| RG-21 | Take the expiry from the 200, set the refresh timer **shorter**; refresh over **the same TCP connection** | MUST | §3.1.1.5.1, p. 76; §3.1.1.6, p. 77 |
| RG-22 | `ms-keep-alive` of the response → keep-alive timer (CONMGMT) | SHOULD | §3.1.1.5.1, p. 76 |
| RG-23 | `presence-state: register-action="added"|"refreshed"|"fixed"`; on refresh with "added"/"fixed" ⇒ registration error, all dialogs ended, register again | MUST/SHOULD | §2.2.1.4, p. 15; §3.1.1.5.1, p. 76 |
| RG-24 | `primary-cluster-type` central/remote, `is-connected-to-primary` yes/no, `user-services-state=unavailable` ⇒ survivable mode (self subscription only) | SHOULD/MUST | §3.1.1.5.1, p. 76 |
| RG-25 | After the first 200: set up the contact/group subscription (MS-SIP §3.3) and the self subscription (MS-PRES §3.3) | SHOULD | §3.1.1.5.1, p. 76 |
| RG-26 | Self subscription fails ⇒ REGISTER Expires 0; outside survivable mode retry after a random interval | SHOULD | §3.1.1.5.1, p. 76 |
| RG-27 | Server errors: 400/4010 missing endpoint ID; 489/4055 wrong event; 421/2057 `msrtc-event-categories` without `gruu-10`; 503/4164 presence service gone and no survivable mode | server | §3.1.2.5.1, p. 78 |
| RG-28 | The registrar grants an expiry ≥ 30 s; bindings are replaced via `+sip.instance` | server | §3.1.2.2, p. 77; §3.1.2.5.1, p. 78 |
| RG-29 | Sign-out: REGISTER with `Expires: 0` | – | §4.1.2, p. 95 |

## Server-initiated deregistration / survivable mode

| ID | Requirement | Evidence |
|---|---|---|
| RG-30 | Deregister NOTIFY in the REGISTER dialog (Call-ID/tags of the last REGISTER): `Event: registration-notify`, `Content-Type: text/registration-event`, `subscription-state: terminated;expires=0`, `ms-diagnostics-public` | §2.2.1.11, pp. 17–18; §3.2.2.4.1, p. 81; example §4.1.3, p. 96 |
| RG-31 | Body values: `deregistered;event=unregistered` (4140, endpoint quota), `rejected` (4141, user disabled), `deactivated` (4142, move), `userservices-unavailable` (4165), `preferred-registrar-change` (4166/4167, new registrar in `P-Preferred-Registrar`); `registered;event=userservices-unavailable` (survivable mode, `subscription-state: active`) | §2.2.1.13, pp. 18–19; §2.2.1.12, p. 18 |
| RG-32 | Client: with `P-Preferred-Registrar`, register there | §3.2.1.5, p. 81 |

## In-band provisioning (SUBSCRIBE `vnd-microsoft-provisioning-v2`)

| ID | Requirement | Evidence |
|---|---|---|
| RG-40 | SUBSCRIBE to the own AOR; `Event: vnd-microsoft-provisioning-v2`; `Accept` and `Content-Type: application/vnd-microsoft-roaming-provisioning-v2+xml`; `Expires: 0` (no dialog); delegation: From = delegate + `p-session-on-behalf-of` | §2.2.2.1, pp. 19–20 |
| RG-41 | Answer in the 200 (if `Supported: ms-piggyback-first-notify`), otherwise via NOTIFY | §2.2.2.2, p. 20; example §4.2, pp. 97–98 |
| RG-42 | Request body `<provisioningGroupList xmlns=".../2006/09/sip/provisioninggrouplist" subnet="…">` with groups: ServerConfiguration, meetingPolicy, ucPolicy, publicationGrammar, userSetting, endpointConfiguration, locationPolicy, mediaConfiguration, presencePolicyV2, privacyPublicationGrammar, publicProviders, persistentChatConfiguration | §2.2.2.4, pp. 20–22; §2.2.2.5.11, p. 71; example p. 98 |
| RG-43 | Response `<provisionGroupList xmlns=".../2006/09/sip/provisiongrouplist-notification">` | §2.2.2.5, p. 22; example p. 98 |
| RG-44 | Example request headers: `Supported: com.microsoft.autoextend`, `ms-benotify`, `Proxy-Require: ms-benotify`, `ms-piggyback-first-notify` | §4.2.1, pp. 97–98 |

### Provisioning values that matter for the client (selection, per group)

**ServerConfiguration** (§2.2.2.5.1, pp. 22–29): `absInternalServerUrl`/`absExternalServerUrl` (address book files, MS-ABS), `abwqInternalURL`/`abwqExternalURL` (address book web query), `dlxInternalUrl`/`dlxExternalUrl`/`dlxEnabled` (MS-DLX), `lisInternalUrl` (MS-E911WS), `mrasUri` (MS-AVEDGEA), `qosUri` (MS-QoE), `focusFactoryUri` (create conferences), `voiceMailUri`, `ucLocationProfile` (dial rules, MS-SIPRE §2.2.8), `ucPC2PCAVEncryption` (Support/Require/DoNotSupport), `ucMaxVideoRateAllowed` (HD720P-1.5M, VGA-600K, CIF-250K), port ranges (`ucPortRangeEnabled`, `ucMin/Max{Media,SipDynamic,Audio,Video,AppSharing,FileTransfer}Port`), `responseGroupService*URL` (MS-RGSWS), `callParkServerUri`, `botSipUriForTestCall`, `cwaInternal/ExternalUri` (dial-in page), `requireFIPSCompliantMedia`, `enableBWPolicyCheck`, `enableInCallQoS`, `inCallQoSIntervalSeconds`, `hybridConfigService*Url`. Several values explicitly "Ignored".

**meetingPolicy** (§2.2.2.5.2, pp. 29–32): AllowIPAudio, AllowIPVideo, EnableDataCollaboration, EnableAppDesktopSharing, AllowAnonymousParticipants, AllowExternalUserControl, AllowRecording, AllowFileTransfer, EnableP2PFileTransfer, EnableP2PVideo, EnableMultiViewJoin, MaxConferenceVideoResolution, AudioBitRate/VideoBitRate/AppSharingBitRate/FileTransferBitRate, TotalReceiveVideoBitRate, AllowPolls, AllowQna, AllowOfficeContent, AllowSharedNotes, MeetingSize and others.

**ucPolicy** (§2.2.2.5.3, pp. 32–33): AllowSimultaneousRinging, AllowCallForwarding, EnableCallPark, EnableCallTransfer, EnableDelegation, EnableMaliciousCallTrace, EnableTeamCall, enableBWPolicyOverride, VoiceDeploymentMode (OnPrem/Online/OnlineBasic/OnPremOnlineHybrid).

**publicationGrammar / privacyPublicationGrammar** (§2.2.2.5.4, pp. 33–48; §2.2.2.5.10, pp. 59–71): `containerManifestList` (container IDs, member types user/domain/sameEnterprise/federated/publicCloud/everyone, roles delegate/buddy, occurrenceConstraint, resolutionRules, blockRules for invites/subscriberPrompt by container/network/availability) and `categoryPublicationManifest` (rules bootstrap/transformation (XSL)/cleanup per category+container, expireType endpoint/user/static/time). The client SHOULD create containers according to the manifest and abort parsing on a namespace or major-version mismatch (XSD comments pp. 35, 41, 43).

**userSetting** (§2.2.2.5.5, pp. 48–49): disableFileTransfer, ucUserLocationProfile, commonAreaPhone.

**endpointConfiguration** (§2.2.2.5.6, pp. 49–56): among others AbsUsage (WebSearchAndFileDownload/WebSearchOnly/FileDownloadOnly), AllowUnencryptedFileTransfer, BlockConversationFromFederatedContacts, CalendarStatePublicationInterval, ConferenceIMIdleTimeout, DisableCalendarPresence, DisableFreeBusyInfo, DisableHtmlIm, DisableRTFIM, DisableEmoticons, DisablePresenceNote (OOF), DisablePhonePresence, EnableAppearOffline, EnableIMAutoArchiving, EnableCallLogAutoArchiving, EnableContactSync, EnableServerConversationHistory, EnableURL, MaximumNumberOfContacts, MaximumDGsAllowedInContactList, MaxPhotoSizeKB, PhotoUsage (NoPhoto/PhotosFromADOnly/AllPhotos), P2PAppSharingEncryption (0/1/2), EnableHighPerformance{P2P,Conferencing}AppSharing (frame-rate table by AppSharingBitRate), WebServicePollInterval, MapiPollInterval, SearchPrefixFlags (bit mask), EnableSkypeUI, CallViaWorkEnabled, RateMyCall*, ImLatency*, EnableTracing/TracingLevel.

**locationPolicy** (§2.2.2.5.7, pp. 56–58): EnhancedEmergencyServicesEnabled, LocationPolicyTagID, LocationRequired (yes/no/disclaimer), UseLocationForE911Only, EmergencyDialString, EmergencyDialMask, EmergencyNumbers (format `112-911;999`, takes precedence over EmergencyDialString), NotificationUri, ConferenceUri, ConferenceMode (oneway/twoway), EmergencyServiceDisclaimer, LocationRefreshInterval.

**mediaConfiguration** (§2.2.2.5.8, p. 58): bypassEnabled, internalBypassMode, externalBypassMode, bypassId.

**presencePolicyV2** (§2.2.2.5.9, pp. 58–59): EnablePrivacyMode, AutoInitiateContacts, PublishLocationDataDefault, DisplayPublishedPhotoDefault, PersonalNoteHistoryDepth, SubscribeToCollapsedDG.

**persistentChatConfiguration** (§2.2.2.5.11, p. 71): DefaultPersistentChatPoolUri, EnablePersistentChat, PersistentChatPoolUris, PersistentChatWebManagerUriInt/Ext.

**publicProviders** (§2.2.2.5.12, pp. 71–73): Name, ProxyFqdn, IconUrl, NameDecorationDomain, NameDecorationRoutingDomain (`user@domain` → `user(domain)@Routing`), NameDecorationExcludedDomainList.

## Version check on REGISTER (warning codes)

| ID | Statement | Evidence |
|---|---|---|
| RG-50 | 200 = allowed; 403 = blocked; `Warning` with agent `lcs.microsoft.com`: 310 (text, incompatible), 311 (403 + static URL), 312 (dynamic base URL with `folder`, the client adds `lang`, `arch`, `flavor`, `build`; OCS 2007 R2 only), 313 (200 + static URL), 314 ("Check Microsoft Update…") | §3.4.2.5.1–3.4.2.5.9, pp. 88–92; <168>, <174>, <175> p. 129 |

## Gaps/conflicts

- O-RG-1 (found 2026-10-27): No document of the package describes how a registrar sends a client to its home pool after authentication (full-text search for 301/302/"Moved" in MS-SIPREGE, MS-SIPRE, MS-CONMGMT without result). Secondary evidence: SIPE (`sip-transport.c` l. 1154–1196) handles a 301 to REGISTER by reading the Contact `sip:host[:port];transport=…`, closing the connection and registering at that host; RFC 3261 §8.1.3.4 and §21.3.2 define the generic redirect. Implementation: accepted only when signed by the SA, host checked, TLS only, foreign domain marked (`crates/register`). Interop test IT-17.
- K-RG-1: Contact/group subscription, batch subscribe (`adhoclist`) and `ms-piggyback-first-notify` are referred to **[MS-SIP]** (pp. 15, 20, 76, 97). MS-SIP is available (Windows package) → separate evidence file 05.
- K-RG-2: §3.1.1 (p. 74) names only NTLM or Kerberos for the SA; MS-SIPAE additionally allows TLS-DSK (from Lync 2010). Not a contradiction in practice, but the text is outdated.
- K-RG-4 (found 2026-10-27): Several examples are not well-formed and element names are spelled inconsistently. (a) §4.2.2 (p. 111) closes `</ requireFIPSCompliantMedia>` with a space – not well-formed XML; (b) the names differ in case between schema, prose and example: `EnableInCallQoS` (schema p. 25) / `enableInCallQos` (prose p. 29) / `enableInCallQoS` (example p. 110), `InCallQoSIntervalSeconds` / `inCallQoSIntervalSeconds`, `abwqExternalUrl` (schema, example) / `abwqExternalURL` (prose p. 27); (c) all three NOTIFY examples §4.1.3–4.1.5 (pp. 96–97) have `To: sip:alice@fabrikam.com>` without the opening `<`, §4.1.5 has `P-Preferred-Registrar: <sip:…;transport=tls">` with a stray quote and `Content-Length: 41` for a 45-byte body; (d) `PersistentChatPoolUris` (p. 111) contains a space inside a SIP URI (`GroupChat: l_7T…`). Decision: XML must be well-formed (fail closed); provisioning names are compared case-insensitively; malformed headers are refused. Tests: `crates/provisioning`, `crates/register`.
- K-RG-3: The text extraction of the XSD fragments (e.g. pp. 24–25, 50, 60–67) is only partly readable; the element names are documented by the tree view and description, the exact XSD types must be checked against the original PDF during implementation.

## Later finding (file 39)
- OB-22: `EnableServerConversationHistory` (endpointConfiguration, p. 55; footnote <124>, p. 127: not supported before Lync 2013) lets the IM server record conversation history.
- Found 2026-10-27 (implementation): the REGISTER example §4.1.1 (p. 94) carries a real NTLM AUTHENTICATE_MESSAGE in `gssapi-data`. Decoded it is **NTLMv1** (24-byte LM and NT responses) with flags 0x42908255 (DATAGRAM, IDENTIFY, KEY_EXCH, VERSION; **no** EXTENDED_SESSIONSECURITY, no 128-bit) from Windows 5.2.3790. CommSCraft refuses such an offer (OB-43); whether current servers offer ESS/128/KEY_EXCH is interop test IT-14. Test: `crates/register` `siprege_example_authenticate_is_ntlm_v1`.
