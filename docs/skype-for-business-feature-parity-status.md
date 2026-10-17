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
