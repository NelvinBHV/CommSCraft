# Windows-Client-Funktionsparität: verbindliches Ziel für v1.0

**Katalog aus v0.6.0 (67 aggregierte Funktionen).** Die ergänzte, verbindliche
Matrix mit 122 Funktionsanforderungen und 93 unabhängigen Windows-Referenzfällen
steht in `FEATURE_PARITY.json` und `WINDOWS_REFERENCE_CASES.json`. Details:
`docs/windows-release-validation.md`.

**Kein v1.0-Freigabestatus.** Diese Tabelle ist der Abnahmekatalog, kein Feature-Versprechen.
Der Referenzclient ist der klassische vollständige **Skype for Business Windows Desktop Client**
gegen einen konfigurierten, lizenzierten **on-premises Skype for Business Server**.
Funktionen hängen von Produktvariante, Administratorrichtlinien und bereitgestellten Diensten ab.
Vor der Freigabe müssen exakte Windows-Buildnummer und die Serverversionen im Testprotokoll fixiert werden.

**Offizielle Ausgangsbasis:** [Microsoft Full Windows Client (Server 2019/SE) feature comparison](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison),
[Microsoft Skype for Business Features](https://learn.microsoft.com/en-us/office365/servicedescriptions/skype-for-business-online-service-description/skype-for-business-online-features),
[Microsoft Service Feature Matrix](https://learn.microsoft.com/en-us/office365/servicedescriptions/skype-for-business-online-service-description/skype-for-business-online-service-description),
[MS-OCSPROT](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/90f4bc22-eae7-480e-a6f8-74d705ed2b2c).
Die Service-Matrix ist **keine** zeitgemäße Liste aller real noch verfügbaren Online-Dienste;
beispielsweise ist Skype for Business Online eingestellt. Unterstützung wird pro realem
on-premises Szenario geprüft und nicht anhand historisch beworbener Online-Dienste behauptet.

**Status:** `offline_subset` = ausschließlich Parser/Simulation/Mock, `not_started` = noch kein
funktionaler Code, `verified` = komplette Implementierung mit realem Nachweis.

## Discovery, Anmeldung und Sicherheit

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Internal/external server discovery (`dns-discovery`) | `offline_subset` | Connect through internal and external DNS and HTTPS autodiscover paths |
| Kerberos, NTLM and TLS-DSK authentication (`auth`) | `offline_subset` | Complete mutual authentication with negative/replay tests |
| SIP registration, refresh and sign-out (`register`) | `offline_subset` | Sign in, renew, sign out and recover after failures with real server |
| In-band provisioning and endpoint identity (`provision`) | `not_started` | Fetch and interpret provisioning safely against server |
| Certificate and service identity lifecycle (`certificates`) | `not_started` | Validate server/certificate, handle renewal and refusal of wrong identity |
| Federated communication where server supports it (`federation`) | `not_started` | Federated SIP domain messaging and presence interop |
| Modern Authentication (when supported by deployment) (`modern-auth`) | `not_started` | Complete supported modern-auth flows with exact server/client policies |

## Kontakte, Präsenz und Integration

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Address book search and directory cards (`contact-search`) | `not_started` | Search and display correct on-prem address book results |
| Contact list and groups (`contact-groups`) | `not_started` | Add, remove, reorder and sync contacts and groups |
| Presence visibility and privacy containers (`contact-privacy`) | `not_started` | Honor privacy containers and personal contact relations |
| Subscribe to enhanced presence categories (`presence-sub`) | `offline_subset` | Receive NOTIFY/BENOTIFY categories across multiple endpoints |
| Publish presence, availability, status, location and note (`presence-publish`) | `not_started` | Correct versioned publications and visibility by container |
| Calendar-driven availability and out-of-office (`calendar-presence`) | `not_started` | Integrate with configured Exchange/Outlook services |
| Contact card photos and profile configuration (`photos`) | `not_started` | Display profile photos and edits with policy enforcement |
| Distribution group expansion (`distribution-groups`) | `not_started` | Expand permitted distribution lists |
| Alerts when tagged contacts change presence (`presence-alerts`) | `not_started` | User can enable and receive permission-respecting status notifications |
| Outlook and Exchange contact integration (`outlook-contacts`) | `not_started` | Find and use Outlook contacts within enabled organization |
| Response Group search (`response-group-search`) | `not_started` | Resolve on-premises response groups |

## Messaging und Dateien

| Feature | Stand | Akzeptanztest |
|---|---|---|
| One-to-one IM and conversation dialogs (`im-1to1`) | `offline_subset` | Send/receive with invite/ack/message/bye and real interop |
| Multiparty messaging (`im-multiparty`) | `not_started` | IM MCU conference lifecycle and roster interop |
| Rich text, emoji and supported messaging formats (`im-richtext`) | `not_started` | Render accepted media types without HTML injection |
| Conversation history and missed messages (`im-history`) | `not_started` | Show local/server-supported conversation history subject to policy |
| File transfer with policy and malware checks (`im-file`) | `not_started` | Transfer files and handle AV/policy/permission constraints |
| Persistent chat rooms where deployed (`persistent-chat`) | `not_started` | Join and manage supported persistent chat |
| Multiple conversations and tabbed history (`im-tabs`) | `not_started` | Switch safely between concurrent conversations |
| Message priority and high importance (`im-importance`) | `not_started` | Send/receive supported importance indication |
| Outlook conversation history integration (`outlook-im-history`) | `not_started` | Write and retrieve history under consent and policy |

## Telefonie und Enterprise Voice

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Peer-to-peer audio calls (`audio-call`) | `not_started` | Place and accept two-way authenticated encrypted calls |
| Enterprise Voice PSTN dialing (`pstn`) | `not_started` | Dial PSTN and receive inbound calls with proper normalization |
| Call hold/resume and mute (`hold`) | `not_started` | Correct hold SDP transitions and microphone mute |
| Attended and unattended transfer (`transfer`) | `not_started` | Blind and consultative transfer interop |
| Call forwarding, simultaneous ring and DND routing (`forwarding`) | `not_started` | Configure and respect server routing and policies |
| Delegation, team call, pickup and shared line (`delegation`) | `not_started` | Manage delegates and shared call scenarios |
| Call park, retrieval and call queues (`callpark`) | `not_started` | Park and retrieve calls with configured server feature |
| Emergency calling and location controls (`emergency`) | `not_started` | Policy-compliant emergency flows and location handling |
| Voicemail and missed call indicators (`voicemail`) | `not_started` | Voicemail integration according to deployment |
| Audio devices, ring and switching (`devices`) | `not_started` | Hot plug, switch, ringtone, microphone and speaker control |
| Call history and call logs (`call-history`) | `not_started` | Accurate history with retention and privacy policies |
| Call monitor and compact in-call UI (`call-monitor`) | `not_started` | In-call UI functions with switching and incoming alerts |
| Response group telephony (`response-group-calls`) | `not_started` | Call on-prem response groups and handle expected queues |

## Medien, Geräte und NAT

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Peer-to-peer video with camera controls (`video`) | `not_started` | Negotiated protected camera streaming and device settings |
| ICE/STUN/TURN and NAT traversal (`ice`) | `not_started` | Media connectivity and fallback through NAT with test lab |
| SRTP key agreement, encryption and RTCP (`srtp`) | `not_started` | Secure media with authenticated key negotiation and replay checks |
| Microsoft-compatible SDP and codecs (`codecs`) | `offline_subset` | Negotiate appropriate audio/video codecs across server versions |
| Bandwidth adaptation, QoS and telemetry (`qos`) | `not_started` | Quality metrics, reconfiguration and network policy |

## Konferenzen und Freigaben

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Scheduled and ad hoc conference join (`conference-join`) | `not_started` | Join, leave and rejoin focus correctly |
| Conference roster and permissions (`conference-roster`) | `not_started` | Participants, lobby, roles and presenter controls |
| Audio/video conferencing and MCU (`conference-audio`) | `not_started` | Join mixed AV conferences with secure media |
| Conference IM messaging (`conference-im`) | `not_started` | Send and receive conference IM/roster data |
| Video-based screen sharing (`sharing-vbss`) | `not_started` | Share and view VbSS sessions where supported |
| Desktop/application sharing via RDP (`sharing-rdp`) | `not_started` | Interactive application share and viewer interop |
| Whiteboard and presentation collaboration (`presentation`) | `not_started` | Present and view supported collaboration content |
| Meeting invitation, scheduling and controls (`meeting-controls`) | `not_started` | Create meetings, invite participants and control meeting state |
| Meeting recording and controls (`meeting-recording`) | `not_started` | Record allowed sessions with privacy and policy safeguards |
| Polls and Q&A in supported meetings (`meeting-polls`) | `not_started` | Participants can view and use enabled meeting polls and Q&A |
| PowerPoint presentation sharing (`powerpoint-sharing`) | `not_started` | Share and view slides in supported meetings |
| Meeting lobby, admission and restrictions (`meeting-lobby`) | `not_started` | Join lobby and handle presenter admission rights |
| Meeting invitations and Outlook scheduling (`meeting-invite`) | `not_started` | Join from and create calendar meeting invitations |

## Windows-Client und Releasequalität

| Feature | Stand | Akzeptanztest |
|---|---|---|
| Feature-complete native desktop client UX (`windows-ui`) | `offline_subset` | Equivalent supported Windows desktop controls and settings |
| Toasts, tray behavior and call notifications (`notifications`) | `not_started` | Reliable user notifications, focus and accessibility |
| Enterprise client policy and privacy enforcement (`enterprise-policy`) | `not_started` | Enforce server restrictions and data retention |
| Keyboard and screen reader operation (`accessibility`) | `not_started` | Pass accessibility audit and Windows usability testing |
| Reconnect, crash recovery and resource limits (`reliability`) | `not_started` | Sustained operation and reconnect under faults |
| Signed Windows installation and update (`packaging`) | `not_started` | Reproducible signed MSI/MSIX and upgrade/rollback tests |
| Windows client baseline compatibility (`enterprise-interop`) | `not_started` | Published parity test matrix against reference Windows client |
| Diagnostics, error reports and support logs (`logging`) | `not_started` | Generate privacy-safe actionable diagnostics |
| Legal hold, archiving and compliance integration (`compliance`) | `not_started` | Respect retention, recording, and compliance policy |

## Striktes Freigabeverfahren

1. Jede Funktion vollständig implementieren (keine Demo-Daten oder simulierte Anmeldung).
2. Spezifikationsverweise und RFCs pro Transaktions-, Medien- und Fehlermodell nachvollziehbar dokumentieren.
3. Unit-, Integrations- und Negativtests einschließlich SIP/Auth-Replay, TLS-MITM, NAT und Fehlerwiederaufnahme automatisieren.
4. Gegen einen rechtmäßig betriebenen SfB-Server samt Windows-Referenzclient und echten Anwenderfunktionen vergleichen.
5. Testnachweise in `release/evidence/` und signiertes, datensparsames `release/interop-signoff.json` prüfen.
6. `python3 scripts/check_release.py --release` darf nur bei vollständiger Verifikation bestehen.
7. Mindestens unabhängige Sicherheitsprüfung sowie Windows-Paketierung/Accessibility-Freigabe.

Die Funktion `check_release.py --release` ist eine **formale Schranke**, keine echte automatische Prüfung
der Microsoft-Interoperabilität. Ausgefüllte Evidenz ist notwendig, aber nicht hinreichend.

