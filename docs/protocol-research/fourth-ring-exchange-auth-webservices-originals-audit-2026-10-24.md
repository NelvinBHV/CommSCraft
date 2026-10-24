# P1-03 – Zehn Originalbibliografien: OLE-Automation, Forms Based Auth, Exchange RPC und Web Services

**Stand 24.10.2026.** Dokumentierte vierte Microsoft/MC-Referenzebene. Die kompletten PDF-Dateibytes stammen aus den vom Nutzer bereitgestellten Originalarchiven `Windows_Protocols.tar.xz` (MS-OAUT), `SharePointProtocols.zip` (MS-OFBA) und `Exchange_Protocols.zip` (acht Exchange-Quellen). Die PDF-Fußzeilen, Revisionsübersichten, `§1.2.1 Normative References` und `§1.2.2 Informative References` wurden aus der jeweiligen Originalausgabe ausgelesen; die Referenzseiten wurden auch gerendert und visuell gegengeprüft. Die Binärdateien verbleiben beim Nutzer, nicht auf GitHub.

## Exakte geprüfte Quelle, Revision, SHA-256 und Referenzklassen

| MS/MC-ID | Revision | Original-PDF-Ausgabe | Vollständige PDF-SHA-256 | Normative §1.2.1 (PDF-Seite) | Informative §1.2.2 |
|---|---:|---|---|---|---|
| MS-OAUT | 22.0 | v20260714, 196 Seiten | `1fde3088df7b5fc675e06edaf028df30c331ae54c41bc81059b79cb60ac70cb1` | 9 (S.14) | 8 (S.14–15) |
| MS-OFBA | 11.0 | v20240820, 23 Seiten | `bde1630aff99562ae6b0e5435c114fcac7c42a7895f26872cff3e92ca422c5c1` | 9 (S.6) | 5 (S.7) |
| MS-OXCRPC | 25.0 | v20250520, 94 Seiten | `d6149dcd57c9aa12e03438b92052fe279d4e032e53065fcec141e14cf19c4aed` | 11 (S.10) | 6 (S.10) |
| MS-OXDISCO | 19.0 | v20250520, 26 Seiten | `5bbff811e5644fedb18ea192d6349bcc1723bae5ca25634ae5b73cf0d569f0e5` | 15 (S.8) | 1 (S.9) |
| MS-OXWSATT | 16.0 | v20250520, 53 Seiten | `82a8c19cbc5b25aa518faaee7eb72aef7ac49639376907c0d3f34b9c8a5143fd` | 20 (S.8) | 1 (S.9) |
| MS-OXWAVLS | 17.0 | v20250520, 64 Seiten | `6f57dd83f9c3db4a60069660dcee6086b57ce16ca58c15bf93ec665849b98b53` | 14 (S.8) | 4 (S.8–9) |
| MS-OXWOAB | 13.0 | v20250520, 18 Seiten | `dd5cf73f65bed8fde38fbb19bbc0133bd66066155ed4afaad35d375e79d4308c` | 10 (S.6–7) | 1 (S.7) |
| MS-OXWOOF | 17.0 | v20250520, 40 Seiten | `36076e523afcc197113b303b8e37e7c7477ae966c6b18fba0822295c852d5ecb` | 10 (S.8) | 3 (S.8) |
| MS-OXWSARCH | 10.0 | v20250520, 31 Seiten | `531a498d1367b5968415e393e595edb0e1051f818817e371cb9f9d71073fd885` | 16 (S.6–7) | 3 (S.7) |
| MS-OXABREF | 15.0 | v20250520, 22 Seiten | `84664ea27aed0e9dc8710bf6526aa25b01139d130223aefd3983c4fcf072e37d` | 6 (S.6–7) | 3 (S.7) |
| **Gesamt** | | **10 unterschiedliche Quell-IDs** | **Vollfile-Hashes vorhanden** | **120** | **35** |

Die [exakten Originalreferenzlisten je Quelle (JSON)](fourth-ring-exchange-auth-webservices-originals-audit-2026-10-24.json) und das [fortgeschriebene, gerichtete Normregister](fourth-ring-normative-reference-ledger.json) enthalten die Rohlabels unverändert einschließlich ihrer jeweiligen PDF-Seite. Es werden keine Zitate in moderne, vermeintlich gleichwertige Editionen umgeschrieben.

## Beobachtungen zur Quellsemantik

- **Asymmetrische RPC-Beziehung:** MS-OXABREF §1.2.1 PDF-S.6 enthält die normative Kante `MS-OXABREF → MS-OXCRPC`. Die Gegenrichtung `MS-OXCRPC → MS-OXABREF` steht in **§1.2.2 Informative References (PDF-S.10)**. Die informative Richtung ist keine normative Kante.
- **Quellenreferenz auf sich selbst:** MS-OAUT v20260714 §1.2.1 PDF-S.14 listet tatsächlich `[MS-OAUT]` auf. Diese Rohreferenz bleibt **als Normbibliografie-Selbstnennung** erhalten und wird nicht ohne weitere Prüfung zu einer zusätzlich zu implementierenden Funktion oder zu einer unendlichen Rekursionspflicht umgedeutet.
- **Web-Service-Kette:** `MS-OXWSATT → MS-OXWSMSG`, `MS-OXWAVLS → MS-OXPROPS` und `MS-OXWSARCH → MS-OXWSFOLD` sind normative Quellverweise. Die jeweils genannten Ziel-Originalausgaben und die tatsächliche Relevanz für den Skype-for-Business-2019-Windows-Full-Client bleiben separat zu belegen.
- **Exakt zu unterscheidende XML-Editionen:** MS-OXWSARCH enthält `XMLSCHEMA1/2`, `XMLSCHEMA1`, `XMLSCHEMA2/2` und `XMLSCHEMA2`. MS-OXWOOF nennt `XMLSCHEMA2/2` zusätzlich zu `XMLSCHEMA2`. Dies sind in den PDFs verschiedene Labels für XML-Schema-Ausgaben; sie dürfen nicht stillschweigend zusammengelegt werden.
- **Besonders vorsichtige zeitliche Geltung:** MS-OAUT ist eine **2026** veröffentlichte Fassung, acht Exchange-Dokumente sind **2025er** und MS-OFBA eine **2024er** Ausgabe. Eine Zitierung dieser Ausgaben beweist nicht die Clientimplementierung oder Funktionsgleichheit einer SfB-2019-Installation.
- **Noch offen und nicht gezählt:** Die **MS-OXPROPS**-Original-PDF liegt vor, enthält aber ein umfangreiches gesondertes Verzeichnis; sie wurde in diesem **10er-Batch nicht vollständig auditiert** und verbleibt ausdrücklich im offenen 49er-Frontier. Kein Normkanten-Count dafür erhöht.

## Formelle Zähler und Sperren

| Forschungsring | Quellenstand | Normative gerichtete Kanten |
|---|---|---:|
| Erster | 46/46 (früherer Quellstand) | 464 |
| Zweiter | 12/12 | 246 |
| Dritter | 52 Originalquellen, davon 48 explizite §1.2.1 | 827 |
| **Vierter** | **37/86, noch 49 offen** | **506** |
| Fünfter (separat) | Erste MS-SWN-Quelle | 6 |

Unverändert: 67 AD-Schema-§1.1-Nennungen **UNCLASSIFIED**, keine belastbar abgeglichene Zielfassungsliste, kein vollständiger normativer Fixpunkt, keine vollständige Client-Feature-/Serverrolle-/Policy-Beweiskette. Phase 1 bleibt **0/12 Gates PASS**. Die PDF-Originale liegen beim Nutzer. Keine Protokollimplementierung, echten Netzwerk-/Interop-Tests, kostenpflichtigen Runner, TLS-/Auth-Sicherheitsabsenkung, `main`-Merge, Tags oder Releases.
