# 122-feature × 93 Windows-case traceability matrix — SOURCE INVENTORY, NOT NORMATIVE VERIFICATION

**Generated from exact repository documents (read-only) on 2026-10-23:** `FEATURE_PARITY.json` Git blob `ee17d3ab5d049acf1b5ba8ff7a3cdb46d19759fe`; `WINDOWS_REFERENCE_CASES.json` Git blob `a8138aa87ffa67c6f4b9a0101618b63b9190a42c`; `release/offline-feature-spec-audit.json` Git blob `de0a104e913df7e080fa62d823a671b83aff7d3d`. Original files are not modified. This matrix is a *snapshot* and must be regenerated/reviewed if the source blobs change.

All **122** feature IDs were enumerated; all **93** scenario IDs were cross-referenced and all links resolved to existing features. **31** features have no linked Windows reference scenario in the current baseline. `status` describes the **repository's claimed offline source-development stage**, NOT functionality or proven protocol conformance. Every normative audit gate below remains OPEN; no automatic upgrade based on source metadata, tests or a broad MS-* identifier.

**Columns:** Source `specifications` are **candidate** references as recorded in the inventory, not verified normative clauses. `Cases` are the exact IDs currently linked in `WINDOWS_REFERENCE_CASES.json`; `—` means no baseline scenario. Every row must ultimately be expanded to an exact [MS-*] revision/date/§, RFC edition/§, wire field, state, timeout, error, security rule, server-policy denial, recovery and evidence crosswalk.

| Area | Feature ID | Repository status | Candidate sources | Windows scenario IDs | Normative review |
| --- | --- | --- | --- | --- | --- |
| identity | `dns-discovery` | offline_subset | MS-CONMGMT, RFC2782 | — | OPEN (full rev/§, security, errors, timers) |
| identity | `auth` | offline_subset | MS-SIPAE, RFC2743 | `kerberos-login` | OPEN (full rev/§, security, errors, timers) |
| identity | `register` | offline_subset | MS-SIPREGE, RFC3261 | — | OPEN (full rev/§, security, errors, timers) |
| identity | `provision` | not_started | MS-SIPREGE | — | OPEN (full rev/§, security, errors, timers) |
| identity | `certificates` | not_started | MS-SIPAE, MS-CONMGMT | — | OPEN (full rev/§, security, errors, timers) |
| identity | `federation` | not_started | MS-SIPRE, MS-PRES | `federated-im` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contact-search` | not_started | MS-ABCH | `corporate-address-book` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contact-groups` | not_started | MS-PRES | `contacts-crud`, `contact-group-edit` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contact-privacy` | not_started | MS-PRES | `privacy-relationships` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-sub` | offline_subset | MS-PRES, RFC6665 | `status-watch` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-publish` | not_started | MS-PRES | `status-publish` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `calendar-presence` | not_started | MS-PRES | — | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `photos` | not_started | MS-PRES | — | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `distribution-groups` | not_started | MS-ABCH | `directory-group-expansion` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-1to1` | offline_subset | MS-SIPRE, MS-OCSPROT, RFC3261 | `message-or-email-action` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-multiparty` | not_started | MS-CONFIM, MS-CONFBAS | — | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-richtext` | not_started | MS-SIPRE | — | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-history` | not_started | MS-SIPRE | — | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-file` | not_started | MS-SIPRE | — | OPEN (full rev/§, security, errors, timers) |
| messaging | `persistent-chat` | not_started | MS-PCHAT | `persistent-room-client` | OPEN (full rev/§, security, errors, timers) |
| voice | `audio-call` | not_started | MS-SDPEXT, MS-RTP, MS-SRTP | `make-voice-call` | OPEN (full rev/§, security, errors, timers) |
| voice | `pstn` | not_started | MS-OCPSTN | — | OPEN (full rev/§, security, errors, timers) |
| voice | `hold` | not_started | MS-SDPEXT, RFC3264 | — | OPEN (full rev/§, security, errors, timers) |
| voice | `transfer` | not_started | MS-SIPRE, RFC3515 | `transfer-call` | OPEN (full rev/§, security, errors, timers) |
| voice | `forwarding` | not_started | MS-SIPRE | `configure-forwarding` | OPEN (full rev/§, security, errors, timers) |
| voice | `delegation` | not_started | MS-SIPRE | `team-call-configuration`, `delegate-configuration` | OPEN (full rev/§, security, errors, timers) |
| voice | `callpark` | not_started | MS-SIPRE | `parking-and-retrieval` | OPEN (full rev/§, security, errors, timers) |
| voice | `emergency` | not_started | MS-OCPSTN | `e911-routing` | OPEN (full rev/§, security, errors, timers) |
| voice | `voicemail` | not_started | MS-PRES | `voice-mail-greeting` | OPEN (full rev/§, security, errors, timers) |
| voice | `devices` | not_started | MS-SDPEXT | — | OPEN (full rev/§, security, errors, timers) |
| voice | `call-history` | not_started | MS-SIPRE | — | OPEN (full rev/§, security, errors, timers) |
| video_media | `video` | not_started | MS-SDPEXT, MS-RTP | `meeting-video` | OPEN (full rev/§, security, errors, timers) |
| video_media | `ice` | not_started | MS-ICE2, MS-TURN | — | OPEN (full rev/§, security, errors, timers) |
| video_media | `srtp` | not_started | MS-SRTP, RFC3711 | — | OPEN (full rev/§, security, errors, timers) |
| video_media | `codecs` | offline_subset | MS-SDPEXT, RFC3264 | — | OPEN (full rev/§, security, errors, timers) |
| video_media | `qos` | not_started | MS-SDPEXT | — | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `conference-join` | not_started | MS-CONFBAS, MS-CONFPRO | — | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `conference-roster` | not_started | MS-CONFBAS | `complete-roster` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `conference-audio` | not_started | MS-CONFAV, MS-CONFBAS | `meeting-audio`, `meeting-video` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `conference-im` | not_started | MS-CONFIM, MS-CONFBAS | `group-im` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `sharing-vbss` | not_started | MS-SDPEXT, MS-CONFAS | `video-screen-sharing` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `sharing-rdp` | not_started | MS-CONFAS, MS-RDPBCGR | `desktop-share` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `presentation` | not_started | MS-CONFBAS | — | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-controls` | not_started | MS-CONFPRO | `presenter-controls` | OPEN (full rev/§, security, errors, timers) |
| client_platform | `windows-ui` | offline_subset | MS-OCSPROT | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `notifications` | not_started | MS-OCSPROT | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `enterprise-policy` | not_started | MS-SIPREGE | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `accessibility` | not_started | WCAG | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `reliability` | not_started | MS-CONMGMT | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `packaging` | not_started | Windows | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `enterprise-interop` | not_started | MS-OCSPROT | — | OPEN (full rev/§, security, errors, timers) |
| identity | `modern-auth` | not_started | MS-SIPAE | `modern-auth-flow` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-alerts` | not_started | MS-PRES | `tag-alerts` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `outlook-contacts` | not_started | MS-ABCH | `outlook-contact-search` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `response-group-search` | not_started | MS-SIPRE | `response-group-directory` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-tabs` | not_started | MS-SIPRE | `tabbed-conversations` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-importance` | not_started | MS-SIPRE | `priority-im` | OPEN (full rev/§, security, errors, timers) |
| messaging | `outlook-im-history` | not_started | MS-SIPRE | `outlook-im-log`, `exchange-im-archive` | OPEN (full rev/§, security, errors, timers) |
| voice | `call-monitor` | not_started | MS-SIPRE | — | OPEN (full rev/§, security, errors, timers) |
| voice | `response-group-calls` | not_started | MS-OCPSTN | `dial-response-group` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-recording` | not_started | MS-CONFBAS | `record-av-sharing` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-polls` | not_started | MS-CONFBAS | `polls` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `powerpoint-sharing` | not_started | MS-CONFBAS | `ppt-present` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-lobby` | not_started | MS-CONFBAS | — | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-invite` | not_started | MS-CONFPRO | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `logging` | not_started | MS-CONMGMT | — | OPEN (full rev/§, security, errors, timers) |
| client_platform | `compliance` | not_started | MS-SIPRE | `media-record-archive` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-notes` | not_started | MS-PRES, MS-OCSPROT | `status-oof` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-location` | not_started | MS-PRES, MS-OCSPROT | `custom-location` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `presence-note-edit` | not_started | MS-PRES, MS-OCSPROT | `custom-note` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `photos-public-url` | not_started | MS-PRES, MS-OCSPROT | `external-photo-url` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contact-prepop` | not_started | MS-OCSPROT, MS-PRES | `initial-contacts` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `recent-contacts` | not_started | MS-OCSPROT, MS-PRES | `recent-contact-list` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `current-conversations` | not_started | MS-OCSPROT, MS-PRES | `active-chat-list` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contact-layout` | not_started | MS-OCSPROT, MS-PRES | `alternative-contact-layouts` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contacts-sorting` | not_started | MS-OCSPROT, MS-PRES | `sort-by-group-relationship-new` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `contacts-availability-sort` | not_started | MS-OCSPROT, MS-PRES | `sort-by-presence` | OPEN (full rev/§, security, errors, timers) |
| contacts_presence | `exchange-contacts` | not_started | MS-OCSPROT, MS-PRES | `exchange-address-book` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-email-action` | not_started | MS-OCSPROT, RFC3261 | `message-or-email-action` | OPEN (full rev/§, security, errors, timers) |
| messaging | `spellcheck` | not_started | MS-OCSPROT, RFC3261 | `spell-check` | OPEN (full rev/§, security, errors, timers) |
| messaging | `skills-sharepoint-search` | not_started | MS-OCSPROT, RFC3261 | `sharepoint-skill-search` | OPEN (full rev/§, security, errors, timers) |
| messaging | `persistent-chat-escalate` | not_started | MS-OCSPROT, RFC3261 | `chatroom-escalation` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-inline-photos` | not_started | MS-OCSPROT, RFC3261 | `chat-profile-images` | OPEN (full rev/§, security, errors, timers) |
| messaging | `im-ink-receive` | not_started | MS-OCSPROT, RFC3261 | `ink-message-display` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `conference-gallery` | not_started | MS-OCSPROT, MS-CONFBAS | `gallery-view` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `app-sharing` | not_started | MS-OCSPROT, MS-CONFBAS | `application-share` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-anonymous-invite` | not_started | MS-OCSPROT, MS-CONFBAS | `invite-anonymous` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-dial-in` | not_started | MS-OCSPROT, MS-CONFBAS | `dialin-conference` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-meet-now` | not_started | MS-OCSPROT, MS-CONFBAS | `meet-now` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `powerpoint-navigation` | not_started | MS-OCSPROT, MS-CONFBAS | `ppt-navigation` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `onenote-notes` | not_started | MS-OCSPROT, MS-CONFBAS | `onenote-meeting` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `whiteboard` | not_started | MS-OCSPROT, MS-CONFBAS | `whiteboard` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-file-upload` | not_started | MS-OCSPROT, MS-CONFBAS | `conference-upload` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-scheduling` | not_started | MS-OCSPROT, MS-CONFBAS | `schedule-meeting` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-qa` | not_started | MS-OCSPROT, MS-CONFBAS | `qa-manager` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-disable-video` | not_started | MS-OCSPROT, MS-CONFBAS | `disable-participant-video` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-disable-im` | not_started | MS-OCSPROT, MS-CONFBAS | `disable-conference-im` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-mute-audience` | not_started | MS-OCSPROT, MS-CONFBAS | `mute-all` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-demote-all` | not_started | MS-OCSPROT, MS-CONFBAS | `demote-all` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-broadcast` | not_started | MS-OCSPROT, MS-CONFBAS | `meeting-broadcast` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-delegate-scheduling` | not_started | MS-OCSPROT, MS-CONFBAS | `schedule-for-delegator` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `outlook-delegate-sync` | not_started | MS-OCSPROT, MS-CONFBAS | `outlook-delegate-sync` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-video-spotlight` | not_started | MS-OCSPROT, MS-CONFBAS | `spotlight-video` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-remote-control` | not_started | MS-OCSPROT, MS-CONFBAS | `remote-control` | OPEN (full rev/§, security, errors, timers) |
| voice | `click-to-call` | not_started | MS-OCSPROT, RFC3261 | `contact-click-to-call` | OPEN (full rev/§, security, errors, timers) |
| voice | `e911-sip-notify` | not_started | MS-OCSPROT, RFC3261 | `e911-notify-sip` | OPEN (full rev/§, security, errors, timers) |
| voice | `e911-distribution-notify` | not_started | MS-OCSPROT, RFC3261 | `e911-notify-group` | OPEN (full rev/§, security, errors, timers) |
| voice | `missed-calls` | not_started | MS-OCSPROT, RFC3261 | `missed-call-alert` | OPEN (full rev/§, security, errors, timers) |
| voice | `delegate-outbound` | not_started | MS-OCSPROT, RFC3261 | `call-on-behalf` | OPEN (full rev/§, security, errors, timers) |
| voice | `delegate-inbound` | not_started | MS-OCSPROT, RFC3261 | `answer-delegated-call` | OPEN (full rev/§, security, errors, timers) |
| voice | `group-call-pickup` | not_started | MS-OCSPROT, RFC3261 | `group-pickup` | OPEN (full rev/§, security, errors, timers) |
| voice | `location-routing` | not_started | MS-OCSPROT, RFC3261 | `location-based-routing` | OPEN (full rev/§, security, errors, timers) |
| voice | `response-group-management` | not_started | MS-OCSPROT, RFC3261 | `response-team-administration` | OPEN (full rev/§, security, errors, timers) |
| messaging | `public-im` | not_started | MS-OCSPROT, MS-SIPRE | `public-im` | OPEN (full rev/§, security, errors, timers) |
| messaging | `external-calling` | not_started | MS-OCSPROT, MS-SIPRE | `external-audio-calls` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `meeting-recording-artifacts` | not_started | MS-OCSPROT | `record-collaboration-content` | OPEN (full rev/§, security, errors, timers) |
| meetings_sharing | `recording-resolution` | not_started | MS-OCSPROT | `record-resolution` | OPEN (full rev/§, security, errors, timers) |
| identity | `mfa` | not_started | MS-SIPAE | `multi-factor-auth` | OPEN (full rev/§, security, errors, timers) |
| identity | `certificate-auth` | not_started | MS-SIPAE | `certificate-login` | OPEN (full rev/§, security, errors, timers) |
| client_platform | `recording-archive-artifacts` | not_started | MS-OCSPROT | `record-file-archive` | OPEN (full rev/§, security, errors, timers) |
| client_platform | `sign-in-logs` | not_started | MS-OCSPROT | `client-signin-logs` | OPEN (full rev/§, security, errors, timers) |
| voice | `high-volume-calls` | not_started | MS-OCSPROT | `high-call-volume` | OPEN (full rev/§, security, errors, timers) |

## Separate unresolved research obligations

- For each of the 31 feature rows with no mapped Windows scenario, determine whether it is legitimately covered by another scenario, whether an actual Windows full client case is missing, or whether it is non-protocol local functionality. Do not silently discard.
- Investigate all scenario conditions, including disabled policy, privilege restrictions, missing server role, unsupported media/network capabilities, race and recovery. The reference-case list itself states that the precise Windows build remains **UNPINNED**.
- The original source/audit-ledger files are historic evidence and may have content from different development eras. Research status here overrides **none** of their existing verification gates.
- Reconcile candidate source IDs against the complete official [MS-OCSPROT] 9.0 version-pinned cross-reference graph; add exact Microsoft revision/date/section and RFC original/update edition **per row**, not per broad capability label.
- Only after source analysis, map to a **future** source-driven implementation plan (not code or tests on this draft).
