# P1-03 – Sechs Exchange- und vier MSMQ/OleTx-Originalbibliografien

**Untersuchungsdatum:** 24.10.2026 · **Draft-Branch:** `draft/protocol-spec-analysis-2026-10-23` · **Forschungsstatus, keine Produktimplementierung**.

Die Original-PDFs wurden aus den beim Nutzer verbleibenden Archiven `Exchange_Protocols.zip` (sechs) und `Windows_Protocols.tar.xz` (vier) entnommen. Jeweils wurden die vollständigen PDF-Bytes SHA-256-geprüft, Originaledition/Fußzeile und Revisionshistorie gelesen, die Abschnitte **§1.2.1 Normative References** und **§1.2.2 Informative References** einzeln erfasst und deren Seitenbilder visuell inspiziert. **Kein Original-PDF-Binary wurde in Git abgelegt.**

## Vollständige Quellenidentitäten, Literaturseiten und Binärhashes

| Original-PDF | Revision / Edition | PDF-Seiten | Normativ §1.2.1: Anzahl (PDF-S.) | Informativ §1.2.2: Anzahl (PDF-S.) | SHA-256 der Originalbytes |
|---|---|---:|---|---|---|
| MS-OXODLGT | **16.0** / `v20250520` | 40 | **19** (8) | **1** (8) | `a6dd75da34c8079340dd49e334adcdc5a11dc3e25a2984ebde85936ae5cbf359` |
| MS-OXORULE | **23.0** / `v20250520` | 60 | **15** (10) | **2** (10) | `53c27a60ca041d6db076572515b180d997e5cc1395372cf72db6718470a42059` |
| MS-OXOSFLD | **17.0** / `v20250520` | 40 | **13** (9) | **15** (9, 10) | `212981eb234d81355dc3bdc8ca2d02f71425a3c539788d3a02a37d442e83684b` |
| MS-OXOSRCH | **15.0** / `v20250520` | 42 | **18** (8) | **2** (8) | `2442095b8c8029322e9db4072dd160295dc027e59206bd4984111280e3904e5c` |
| MS-OXRTFCP | **15.0** / `v20250520` | 46 | **4** (7) | **2** (7) | `33e2262f7f253ff07ee2a8d8e06c6345ee05d276fd50ab8506c96843259abc2f` |
| MS-OXVCARD | **9.0** / `v20250520` | 30 | **11** (7) | **2** (7) | `211728f2fe112dd9f1622cc84f261d5323cacb33719f23c7e64b861e5e300faf` |
| MS-MQDS | **24.1** / `v20170601` | 238 | **14** (13) | **3** (14) | `afd590c3c79e32c1ca0f20f8cef9e268ca9bf20867edc098589e25c14528fb23` |
| MS-MQDSSM | **24.0** / `v20240423` | 136 | **13** (10, 11) | **2** (11) | `ca6c4bb31f62f08fd09faad6dad251497416255e9542d35067acfe8c5da5dfd0` |
| MS-DTCO | **32.0** / `v20240729` | 475 | **18** (34, 35) | **8** (35) | `9e5c1f26e6a099b79a44e4dd8cbae5c96ee70997bb2cb25c01642d74809ba9db` |
| MS-MQBR | **18.0** / `v20240423` | 28 | **6** (7, 8) | **3** (8) | `5612ce6812af9c316266999253ca426d1ef3ef3a72bbe1d2395c3a79d3ca9b9a` |
| **SUMME** | zehn getrennte Originale | | **131** | **40** | zehn vollständige SHA-256 |

**Exakte Einzelbelege:** [maschinenlesbarer Originalaudit](fifth-stage-exchange-rules-msmq-dtc-originals-audit-2026-10-24.json) und [nachgelagerter Normgraph](fifth-ring-ms-swn-normative-ledger.json). Alle einzelnen gerichteten Normverweise enthalten Quellrevision, Edition, Original-PDF-Seite, Binär-SHA-256, vollständige Originalseitenzahl und Archivpfad. Informative Verweise werden in einem eigenen Datenfeld geführt. Zitierte **Zieloriginaleditionen** bleiben `UNRECONCILED`.

## Belegte Richtungen und Editionskonflikte

1. **MSMQ-Quellversion von 2017 ist real und nicht umgedeutet:** `MS-MQDS` liegt als **v20170601 Revision 24.1** vor (Original-PDF-S.13–14). Die [offizielle Microsoft-Versionsliste](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-mqds) führt ebenfalls 01.06.2017 / 24.1 als veröffentlichte Originalausgabe. `MS-MQDS → MS-MQCN`, `→ MS-MQDMPR` und `→ MS-MQDSSM` sind auf PDF-S.13 **normativ**; `→ MS-MQOD` nur informativ auf S.14. Dieses historische Quellen-PDF wird nicht mit den 2024er MSMQ-Editionen gleichgesetzt.
2. **MSMQ-zyklische Originalbibliografie:** `MS-MQDSSM v20240423 → MS-MQDS` (S.10) und `MS-MQDS v20170601 → MS-MQDSSM` (S.13) sind beide als Bibliografieeinträge normativ, aber **verschieden datierte Originalquellenausgaben**. `MS-MQBR → MS-MQDMPR`, `→ MS-MQDSSM` und `→ MS-MQQB` normativ (S.7); `MS-MQBR → MS-MQDS` nur informativ (S.8). Diese Richtungsunterschiede bleiben im Graph.
3. **OleTx/Transaktionskoordination:** `MS-DTCO` ist das **475 Seiten** umfassende Original v20240729 rev32.0. §1.2.1 nennt normativ u. a. `MS-CMOM`, `MS-CMPO`, `MS-CMP`, `MS-CMRP` (S.34), `MS-TIPP`, `MS-WSRVCAT`, `RFC2371` und `RFC4122` (S.35). `MS-DTCO → MS-MQRR` ist dagegen **nur informativ** (S.35). Die [öffentliche Microsoft-Einführung](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtco/430845a4-6a4a-4b26-a483-45cd424a1718) beschreibt OleTx für verteilte Transaktionskoordination. Weder die Existenz eines Bibliografiezitats noch seine Einbettung in den Graph beweist eine vollständige Skype-for-Business-2019-Windows-Full-Client-Rolle.
4. **Exchange-Suchordner:** `MS-OXOSRCH → MS-OXOJRNL` und `→ MS-OXONOTE` sind normativ (S.8), `MS-OXOSRCH → MS-OXCSTOR` steht dagegen ausschließlich in §1.2.2 (S.8). Die Original-Suche nach Nachrichtentypen stellt noch keinen konkreten Full-Client-Funktionsnachweis dar.
5. **Exchange-Regel-/Sonderordner-Abhängigkeiten:** `MS-OXORULE → MS-OXOSFLD` ist normativ (S.10), aber `MS-OXOSFLD → MS-OXORULE` nur **informativ** (S.10). `MS-OXOSFLD` nennt insgesamt **13 normative und 15 informative** Referenzen, getrennt über PDF-S.9–10. Eine automatische bidirektionale Kante wäre sachlich falsch.
6. **Delegation und Speicherrollen:** `MS-OXODLGT → MS-OXDISCO`, `→ MS-OXCSTOR` und `→ MS-OXOTASK` sind normativ auf PDF-S.8. `MS-OXODLGT → MS-OXPROTO` ist dort ausschließlich informativ.
7. **Konvertierungen mit kleinen eigenen Referenzmengen:** `MS-OXRTFCP` hat vier Normziele einschließlich `RFC2119` und `RFC5234` (S.7); `MSFT-RTF` ist lediglich informativ. `MS-OXVCARD` zitiert `RFC2045`, `RFC2047`, `RFC2426` und `X520` normativ (S.7), `MS-OXCMAIL` nur informativ. Daraus folgt kein automatischer Zwang, alle dargestellten Formate im Skype-for-Business-Client umzusetzen.

## Revidierte MS-/MC-Zielworkqueues

Die [anfängliche viertstufenbasierte Arbeitsliste](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält unverändert **88 MS-/MC-Rohlabel-Kandidaten** nach Abzug bereits in Ebenen 1–4 geprüfter Quellenidentitäten. Sechs neue Exchange-Originalbibliografien aus dieser Liste wurden geprüft. Damit sind **55** aus dieser Liste normbibliografisch originalgeprüft, **ein** MS-ADLS-§1.1-Schemafall separat klassifiziert und **32** reguläre Originalbibliografien offen.

Zusätzlich wurden vier bisher offene Quellen der [transitiven Ergänzungsliste](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) geprüft: `MS-MQDS`, `MS-MQDSSM`, `MS-DTCO` und `MS-MQBR`. Nach erneutem Ausschluss früherer Ringe, anfänglicher 88 Rohkennungen und sämtlicher bereits geprüfter downstream-Quellen aus **allen 912 downstream-Normkanten** umfasst diese ergänzende Liste aktuell **16** offene Rohziele. Neu auffällig sind `MS-CMOM`, `MS-CMP`, `MS-CMPO`, `MS-TIPP` und `MS-WSRVCAT`; die Kanten lassen sich auf `MS-DTCO` S.34–35 zurückführen.

Der **kombinierte noch offene Bestand beträgt 32 + 16 = 48 verschiedene MS-/MC-Rohkennungen**, nicht die Zahl bewiesener eigener Implementierungspflichten. Diese Menge ist **nicht** historisch-editionskanonisiert, nicht mathematisch als kürzeste fünfte Ebene bewiesen und nicht normativ vollständig; bei weiteren Quellen können weitere Ziele auftauchen.

## Nachweisbare Bilanz und offener Wiederaufnahmepunkt

| Prüffeld | Zustand nach Audit |
|---|---|
| Frühere normative Ringe 1–3 | **464 / 246 / 827** Kanten, unverändert |
| Vierte Quellebene | **86/86** Quellidentitäten klassifiziert (85 explizite §1.2.1-Bibliografien + ein MS-ADLS-Schema-§1.1-Sonderfall), **1.147** Normkanten unverändert |
| Nachgelagerte Originalbibliografien | **63** Quellen auditiert, **912** gerichtete normative Kanten (**781 + 131**) und **231** informative Verweise (**191 + 40**) |
| Ursprüngliche viertstufige Rohzielqueue | **88** Rohlabels, **55** Quellen originalgeprüft, 1 Schema-Ausnahme, **32** reguläre offen |
| Separate transitive MS-/MC-Rohzielqueue | **16** noch nicht auditiert |
| Beide Quellenlisten, wörtliche Rohkennungen | **48** offene Kandidaten |
| Schema-§1.1-Angaben | Alte 67 Drittstufen-Nennungen und 13 MS-ADLS-Nennungen weiterhin **UNCLASSIFIED** |
| MS-DMRP | Historische Originalausgabe 01.06.2017 rev8.2 bleibt ausstehend; 2026 Microsoft-Learn-Webexport bleibt Zusatzquelle, nicht Originalnormbibliografie |
| Phase 1 | **0/12 formelle Abschlussgates bestanden** |

**Nächste tatsächlich mögliche Gruppe:** Aus der anfänglichen Liste `MS-OXOJRNL`, `MS-OXONOTE`, `MS-OXOPFFB`, `MS-OXOPOST`, `MS-OXORSS`, `MS-OXRTFEX`; aus der transitiven Liste `MS-MQCN`, `MS-MQQP`, `MS-CMOM`, `MS-CMP` – vorausgesetzt, ihre Originale sind vorhanden. Zusätzlich erfordern historische zitierte Zielausgaben sowie RFC/IETF/ISO/ITU/W3C-Standards noch systematischen Versionsnachweis; die tatsächlich erforderlichen Rollen, Builds und Policystati des Skype-for-Business-2019-Windows-Full-Clients bleiben ungeklärt.

**Verbindliche Sperren bis zum Abschluss von Phase 1:** keine Protokollimplementierung, keine Live-Netzwerk-/Interop-Tests, keine bezahlte CI, keine Sicherheitsabsenkung bei TLS oder Authentisierung, kein Merge nach `main`, Tag oder Release. Original-PDFs nur beim Nutzer.
