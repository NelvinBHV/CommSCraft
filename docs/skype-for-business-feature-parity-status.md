# Skype for Business Windows Full Client – aktueller Paritätsstand

**Stand: 17. Oktober 2026. Basis: Feature-Register auf PR #35; keine echte Microsoft-Interoperabilität nachgewiesen.**

| Kennzahl | Wert |
| --- | ---: |
| Erfasste Full-Client-Funktionsanforderungen | **122** |
| Davon als Offline-Teilimplementierung registriert | **10 (8,2 %)** |
| Als noch nicht begonnen registriert | **112 (91,8 %)** |
| Vollständig verifiziert registriert | **0 (0 %)** |
| Unabhängige Windows-Referenzfälle | **93** |
| Funktionen mit hinterlegten Interoperabilitätsbelegen | **0** |
| Referenzfälle mit hinterlegten Interoperabilitätsbelegen | **0** |

**Wichtig:** 8,2 % bedeutet *Anteil der Funktionsanforderungen mit registrierter Offline-Teilfunktion*, **nicht** „5,7 % funktionale Gleichwertigkeit zu Skype for Business“. Die Zahl der verifizierten Funktionen ist **0/122**. Ein grüner Rust-CI-Lauf bedeutet keine Anmeldung an einem echten Microsoft-Server.

| Funktionsbereich | Gesamt | Offline-Teilfunktion | Verifiziert |
| --- | ---: | ---: | ---: |
| Identität / Anmeldung | 9 | 3 | 0 |
| Kontakte / Präsenz | 22 | 3 | 0 |
| Messaging | 17 | 2 | 0 |
| Sprache / Telefonie | 23 | 0 | 0 |
| Video / Medien | 5 | 1 | 0 |
| Konferenzen / Freigaben | 35 | 0 | 0 |
| Client / Plattform | 11 | 1 | 0 |
| **Gesamt** | **122** | **10** | **0** |

## Reproduzierbare Fortschrittsmessung

- Quelle: `FEATURE_PARITY.json` und `WINDOWS_REFERENCE_CASES.json`; Statusdefinitionen bleiben wie im Repository.
- Dynamischer Bericht: `python3 scripts/report_feature_parity.py --markdown`.
- Maschinenlesbare Zahlen: `python3 scripts/report_feature_parity.py --json`.
- Regressionstests: `python3 -m unittest discover -s scripts -p "test*.py"`, inklusive `test_report_feature_parity.py`.
- Diese Markdown-Datei ist eine datierte Momentaufnahme. Der Python-Bericht berechnet bei **jedem Aufruf** den aktuellen Stand der Matrix. Ändert sich das Register, ohne dass diese Datei aktualisiert wird, hat der Python-Bericht Vorrang.

## Interpretation und notwendige Nachweise

Quellcode zu einem Feature kann existieren, obwohl das konservative Matrix-Register es noch als `not_started` führt. Für `verified` genügt weder vorhandener Rust-Code, synthetisches MIC, ein 200-OK-Parser noch die offline getestete RFC-5922-SAN-Prüfung. Erforderlich sind unabhängige echte Windows-Full-Client-Referenzfälle, richtige und verweigerte Policies, Sicherheit und Recovery; `docs/release-policy.md` bleibt verbindlich. Die 93 Referenzszenarien sind **nicht** proportional auf die 122 Feature-Einträge zu verteilen. Keine frei erfundene Gesamt-Prozentparität.

Referenz: [Microsoft Windows Full Client feature comparison](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison); `docs/release-policy.md`; `ROADMAP.md`.

## Neue Teilfunktionen (Quelle, keine Server-Parität)

- `contact-groups`: lokale Kontaktgruppen und Mitgliedschaft, begrenzt auf 128 Gruppen/Mitglieder, negative Tests für Duplikate, fehlende Gruppen und Steuerzeichen.
- `presence-publish`: lokale, validierte Verfügbarkeits-/Notiz-Entwürfe; **kein** SIP PUBLISH oder authentifizierte Zustandsübermittlung.
- `im-history`: flüchtiger, begrenzter Gesprächsverlauf (128 Einträge) mit Löschfunktion und Tests für Überlauf und Steuerzeichen; **keine** Dateispeicherung, Serververbindung oder geräteübergreifende Synchronisierung.

Quellpfad `crates/commscraft-core/src/client_offline.rs`; Testfixtures `client_offline::tests::*`. Normative Orientierung: [RFC 3261 (June 2002) §19](https://www.rfc-editor.org/rfc/rfc3261), [RFC 3856 (August 2004) §§3–4](https://www.rfc-editor.org/rfc/rfc3856), [RFC 3428 (December 2002) §3](https://www.rfc-editor.org/rfc/rfc3428); die lokalen Modelle implementieren **keine** Netzprotokollkonformität. Ergebnisse des neueren SHA sind bis zum CI-Abschluss noch nicht bestätigt.

## Additional offline code under validation — not yet counted in matrix

The following three bounded APIs have been added in `crates/commscraft-core/src/local_features.rs`: local address-card search (`contact-search`), local privacy relation drafts (`contact-privacy`), and HTML-escaped text preview (`im-richtext`). All are offline-only; they do not implement Microsoft server search, privacy enforcement or rich-text messaging. Tests cover malformed data, case-insensitive lookups, duplicate entries and markup escaping. **The central FEATURE_PARITY.json registry is still authoritative and must be updated only when the exact-head CI verifies these tests.** No claim of 13 registered features or live parity before that update.

Context: Microsoft Open Specifications [MS-ABCH](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abch/), [MS-PRES](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/), [MS-SIPRE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipre/). These local-only models implement no normative network behavior. Source commit `3741724aef4efb3cd0c57130de6b23f4a698ea87`; test execution still pending.

## Development overlay: 16 offline subsets (2026-10-17)

Because `FEATURE_PARITY.json` belongs to the immutable historical SHA-256 source ledger, the **original 7/122** entries remain unchanged. `release/offline-feature-development.json` records **9 new source-backed offline partials** separately: `contact-groups`, `presence-publish`, `im-history`, `contact-search`, `contact-privacy`, `im-richtext`, `calendar-presence`, `photos`, and `distribution-groups`. The report script now merges the 7 original partials and 9 development overrides to show **16/122 (13.1%)** while preserving historical evidence. The new `auxiliary_offline.rs` tests verify bounded local calendar slots, signature/size-gated photo draft bytes, and local distribution-list member deduplication. **All 16 are only offline subsets, never complete interoperable functions; verified parity remains 0/122.** Exact-head CI is pending; don't treat code additions as passed tests until completed.

Normative protocol references for later integration (local data models implement none of these network semantics): [RFC 3261 (June 2002) §19](https://www.rfc-editor.org/rfc/rfc3261), [RFC 3856 (August 2004) §§3–4](https://www.rfc-editor.org/rfc/rfc3856), Microsoft [MS-PRES](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/) and [MS-ABCH](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abch/); applicable Microsoft revision must be pinned before implementing wire behavior.

## Development milestone: 20 / 122 local offline subsets

At commit `e0004fd514f613f2364bae0b10782bcb7b20c632`, the original immutable registry identifies 7 offline subsets and `release/offline-feature-development.json` adds 13 distinct local implementations, yielding **20/122 (16.4%)** *source-backed offline subsets*. Four new entries are `im-multiparty` (local roster add/remove), `im-file` (safe file-offer metadata only), `audio-call` (local call-state transitions only), `hold` (mute/hold local control state only) in `crates/commscraft-core/src/interaction_offline.rs`; positive/negative fixtures in `interaction_offline::tests`. There is **no** live SIP multiparty invitation, real file transport or virus checking, media call, audio device, authenticated state, or negotiated hold. **Verified Windows SfB parity remains 0/122.** CI at the latest commit must pass before claiming tested-offline status.

Normative references for future wire implementation, not implemented by these local models: [RFC 3261 (June 2002) §§12–14](https://www.rfc-editor.org/rfc/rfc3261), [RFC 3264 (June 2002) §§5–8](https://www.rfc-editor.org/rfc/rfc3264), [RFC 3428 (December 2002) §3](https://www.rfc-editor.org/rfc/rfc3428); version-specific Microsoft conference and media protocol requirements are out of scope for these offline drafts.

## Development milestone: 25 / 122 offline-only partials (2026-10-17)

The immutable historical 7 `offline_subset` entries plus 18 unique entries in `release/offline-feature-development.json` yield **25/122 (20.5%)** local/offline source-backed partials. The five latest features are `forwarding` (local destination draft), `transfer` (local prepare/commit guard), `conference-roster` (bounded local participant list), `devices` (local device-label selection only), and `call-history` (bounded, erasable local list). Source and negative fixtures: `crates/commscraft-core/src/call_drafts.rs::tests`. No real SIP signaling, authenticated Skype for Business connection, device enumeration, media, phone calls, conference participation, synchronization or server policy enforcement. **Fully verified Windows Full Client parity remains 0/122.** Exact-head CI is pending; tests are authored, not independently verified until workflow completion.

Future wire protocol context (none implemented by these models): [RFC 3261 (June 2002), §§12–14](https://www.rfc-editor.org/rfc/rfc3261), [RFC 3264 (June 2002), §§5–8](https://www.rfc-editor.org/rfc/rfc3264); Microsoft MS-CONFBAS / MS-CONFIM revisions must be pinned before claiming interoperable conference support.

## Development milestone: 35/122 offline-only partials — 2026-10-18

Seven immutable historical entries plus 28 distinct entries in `release/offline-feature-development.json` now yield **35/122 (28.7%) source-backed *offline subset entries***. Newly added `presence-notes`, `presence-location`, `recent-contacts`, `contacts-sorting`, `current-conversations` implement **bounded local presentation drafts only** in `crates/commscraft-core/src/contacts_ui_offline.rs`. Named negative and positive Rust fixtures: `notes_reject_controls_and_clear`, `locations_reject_untrusted_controls`, `recents_are_unique_most_recent_first_and_bounded`, `contacts_sorted_stably_and_duplicates_denied`, `current_conversations_guard_state`. New-head CI results are **pending**; nothing is claimed tested until actual jobs succeed.

Context only: Microsoft MS-PRES (presence extension), MS-ABCH (address book), MS-OCSPROT (general client protocol overview). **No Microsoft wire protocol, version-specific normative section, enterprise policy, authenticated presence publication or contact sync is implemented by these five drafts.** Relevant IETF publication context: [RFC 3856 (August 2004) §§3–4](https://www.rfc-editor.org/rfc/rfc3856), [RFC 3261 (June 2002) §19](https://www.rfc-editor.org/rfc/rfc3261). There is no applicable normative RFC claim for local UI list sorting or notes storage. The exact mapping is in `release/offline-feature-spec-audit.json`. **Microsoft-interoperable verified features: 0/122; full Windows parity: 0%.**

## Development milestone: 40/122 (2026-10-18)

The immutable seven historical entries plus 33 development-only overlays now track **40/122 (32.8%) offline partials**, **0/122 verified Microsoft interoperability**. Five newly authored bounded local prototypes: `contacts-availability-sort` (manual availability rank sorting), `contact-layout` (layout switches), `meeting-invite` (recipient staging only), `spellcheck` (manual flagged words only, **not** a spelling dictionary), and `contact-prepop` (staged contact addresses). Source: `crates/commscraft-core/src/client_preferences_offline.rs`; five discrete positive/negative unit fixtures in `client_preferences_offline::tests`. Every feature maps to an individual record in `release/offline-feature-spec-audit.json`, with source and exact test name; `scripts/test_offline_feature_spec_audit.py` enforces 40 entries and checks file/test-symbol existence. Microsoft MS-PRES, MS-OCSPROT, MS-CONFBAS and MS-ABCH are **contextual families only**, without claimed implementation of any version-specific normative clause. IETF RFC 3261 (June 2002) §19 is URI-related background only; no remote SIP invitation or Microsoft server synchronization is implemented. CI success must be established at the new exact HEAD SHA before marking these models tested. No main merge, external Microsoft connection or release is authorized.

## Development milestone: 45/122 offline entries — 2026-10-18

The historical immutable 7 plus 38 independent source-backed development overlays now total **45/122 (36.9%) local/offline subset entries**. New prototypes are `meeting-qa`, `meeting-disable-video`, `meeting-disable-im`, `meeting-mute-audience`, `conference-gallery`: bounded Q&A list, local media toggles, a local audience mute switch and a de-duplicated gallery roster. All are local models in `crates/commscraft-core/src/meeting_controls_offline.rs` with five named positive/negative test fixtures. `release/offline-feature-spec-audit.json` has five new per-feature source/test traces. Microsoft MS-CONFBAS and MS-CONFIM are **context only**; no specific version-dependent Microsoft normative clause is implemented or verified for these UI drafts. Cf. SIP dialog semantics in [RFC 3261 (June 2002) §12](https://www.rfc-editor.org/rfc/rfc3261#section-12), not implemented by these models. There is no real network meeting join, moderation enforcement, media control or Microsoft interoperability. **Verified Microsoft parity remains 0/122** and v0.30.0 is not approved. New-commit CI results must be separately collected.
