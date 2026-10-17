# Skype for Business Windows Full Client – aktueller Paritätsstand

**Stand: 17. Oktober 2026. Basis: Feature-Register auf PR #35; keine echte Microsoft-Interoperabilität nachgewiesen.**

| Kennzahl | Wert |
| --- | ---: |
| Erfasste Full-Client-Funktionsanforderungen | **122** |
| Davon als Offline-Teilimplementierung registriert | **7 (5,7 %)** |
| Als noch nicht begonnen registriert | **115 (94,3 %)** |
| Vollständig verifiziert registriert | **0 (0 %)** |
| Unabhängige Windows-Referenzfälle | **93** |
| Funktionen mit hinterlegten Interoperabilitätsbelegen | **0** |
| Referenzfälle mit hinterlegten Interoperabilitätsbelegen | **0** |

**Wichtig:** 5,7 % bedeutet *Anteil der Funktionsanforderungen mit registrierter Offline-Teilfunktion*, **nicht** „5,7 % funktionale Gleichwertigkeit zu Skype for Business“. Die Zahl der verifizierten Funktionen ist **0/122**. Ein grüner Rust-CI-Lauf bedeutet keine Anmeldung an einem echten Microsoft-Server.

| Funktionsbereich | Gesamt | Offline-Teilfunktion | Verifiziert |
| --- | ---: | ---: | ---: |
| Identität / Anmeldung | 9 | 3 | 0 |
| Kontakte / Präsenz | 22 | 1 | 0 |
| Messaging | 17 | 1 | 0 |
| Sprache / Telefonie | 23 | 0 | 0 |
| Video / Medien | 5 | 1 | 0 |
| Konferenzen / Freigaben | 35 | 0 | 0 |
| Client / Plattform | 11 | 1 | 0 |
| **Gesamt** | **122** | **7** | **0** |

## Reproduzierbare Fortschrittsmessung

- Quelle: `FEATURE_PARITY.json` und `WINDOWS_REFERENCE_CASES.json`; Statusdefinitionen bleiben wie im Repository.
- Dynamischer Bericht: `python3 scripts/report_feature_parity.py --markdown`.
- Maschinenlesbare Zahlen: `python3 scripts/report_feature_parity.py --json`.
- Regressionstests: `python3 -m unittest discover -s scripts -p "test*.py"`, inklusive `test_report_feature_parity.py`.
- Diese Markdown-Datei ist eine datierte Momentaufnahme. Der Python-Bericht berechnet bei **jedem Aufruf** den aktuellen Stand der Matrix. Ändert sich das Register, ohne dass diese Datei aktualisiert wird, hat der Python-Bericht Vorrang.

## Interpretation und notwendige Nachweise

Quellcode zu einem Feature kann existieren, obwohl das konservative Matrix-Register es noch als `not_started` führt. Für `verified` genügt weder vorhandener Rust-Code, synthetisches MIC, ein 200-OK-Parser noch die offline getestete RFC-5922-SAN-Prüfung. Erforderlich sind unabhängige echte Windows-Full-Client-Referenzfälle, richtige und verweigerte Policies, Sicherheit und Recovery; `docs/release-policy.md` bleibt verbindlich. Die 93 Referenzszenarien sind **nicht** proportional auf die 122 Feature-Einträge zu verteilen. Keine frei erfundene Gesamt-Prozentparität.

Referenz: [Microsoft Windows Full Client feature comparison](https://learn.microsoft.com/en-us/skypeforbusiness/plan/feature-comparison); `docs/release-policy.md`; `ROADMAP.md`.
