# P1-03 – Zehn Originalbibliografien aus Exchange, MSMQ und OleTx

**Forschungsdatum:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23` · **Status:** dokumentarische Quellenforschung, keine Protokollimplementierung.

Sechs unveränderte Original-PDFs aus dem nutzereigenen `Exchange_Protocols.zip` und vier aus `Windows_Protocols.tar.xz` wurden vollständig gelesen. Für jedes Original wurden **ganzer Binärdatei-SHA-256**, Archiv-Mitglied, `vYYYYMMDD`-Edition, Revisionshistorie, echte Original-PDF-Fundseite und §1.2.1/§1.2.2-Bibliografien erfasst. Die ursprünglichen Referenzseiten wurden zusätzlich gerendert und visuell kontrolliert. Die PDF-Binärdateien bleiben beim Nutzer, **keine PDF-Binärdatei wurde in Git abgelegt**.

## Editions- und Seitenbeweise

| Original | Revision / Edition | PDF-S. gesamt | Normativ §1.2.1 (Fundseite) | Informativ §1.2.2 (Fundseite) | SHA-256 der kompletten Original-PDF |
|---|---|---:|---|---|---|
| MS-OXOJRNL | **13.0** / `v20250520` | 23 | **9** (7) | **2** (7, 8) | `d6d0344a2fab5b34a3f98190d03fb44aeb0ef57f31cd13038e8fdf72ebfea858` |
| MS-OXONOTE | **12.0** / `v20250520` | 21 | **8** (7) | **4** (7) | `ea78ad7812e23e5726ef3f026ca4936d1e5ab23cb9c0f6f7bf77e48fa3b70e43` |
| MS-OXOPFFB | **6.4** / `v20160914` | 35 | **13** (7, 8) | **2** (8) | `011e2715f86e7161d9090dbd260574ef79af5706243ece391c4d67ea4c32ffab` |
| MS-OXOPOST | **13.0** / `v20250520` | 20 | **7** (7) | **2** (7) | `76672c08bf0ec18a7a0dcff2048a3f3a7df176e219c51acc99fd47f9be5d56cb` |
| MS-OXORSS | **13.0** / `v20250520` | 25 | **9** (7) | **4** (7, 8) | `172abcd971405176636292bd8a149184d604b7e1fcb2b4da6c8cc96fcf93b815` |
| MS-OXRTFEX | **15.0** / `v20250520` | 34 | **6** (6, 7) | **7** (7) | `dfceffb2bbee6e0a9b90d155be70a2b5a06fddae71b1d0fa434922ac2460a557` |
| MS-MQCN | **18.0** / `v20240423` | 42 | **11** (8, 9) | **1** (9) | `c6561376eaa6aa8d6f569dc3bed2931dead329df5d9f578b8b9049cf69ca51f7` |
| MS-MQQP | **25.0** / `v20240423` | 60 | **13** (8) | **1** (8) | `6195f88fa881e45986485a2d4b7e76adfcac09852a93f9d4b88d320f862a3719` |
| MS-CMOM | **27.0** / `v20240423` | 73 | **14** (9, 10) | **0** (keine) | `ee5899ed46f4eb571b60ba71bd99ac6d1ff895353980846e4816f88f818b4052` |
| MS-CMP | **25.0** / `v20240423` | 39 | **6** (8) | **1** (8) | `4aad23b1e5586e2007a6fe9a0bf781ae46403fec05f4964d71305e1b6ecf6ddc` |
| **SUMME** | zehn originale Bibliografien | | **96** | **24** | 10 vollständige SHA-256 |

**Einzelnachweise und nachvollziehbare Graphkanten:** [Originalquellen-Audit-JSON](fifth-stage-exchange-legacy-freebusy-msmq-oletx-audit-2026-10-24.json) · [nachgelagerter normativer Kantenledger](fifth-ring-ms-swn-normative-ledger.json). Der Kantenledger bewahrt Zielrohlabel, tatsächliche Original-PDF-Fundseite, Quelledition, Quellrevision, vollständigen Binärhash und Archivpfad. Informative Quellen werden **nicht** normativ gezählt. Die exakten **zitierten Zieloriginaleditionen** sind weiterhin `UNRECONCILED`.

## Belegte Referenzbeziehungen und Grenzen

1. **Echte §1.2.1-Selbstkante bei MSDTC OleTx:** `MS-CMP`, Original `v20240423` Revision **25.0**, nennt auf **PDF-S.8** `[MS-CMP]` als **expliziten normativen Literaturverweis**, getrennt von der wiederholten Original-PDF-Fußzeile. Die gerichtete Bibliografie-Selbstkante `MS-CMP → MS-CMP` bleibt im Ledger mit `explicit_bibliography_self_citation:true`, darf aber bei künftiger Rekursion keine unendliche Traversierung auslösen. Bereits bekannte echte Selbstkanten `MS-MQRR → MS-MQRR` und `MS-OXOCNTC → MS-OXOCNTC` bleiben bestehen.
2. **Historische Exchange-Free/Busy-Ausgabe statt stiller Aktualisierung:** `MS-OXOPFFB` liegt **v20160914 Rev.6.4** vor. Sein §1.2.1-Verzeichnis enthält **13 Normquellen** auf PDF-S.7–8, darunter `MS-OXODLGT`, `MS-OXOSFLD`, `RFC1279` und `RFC2119`; informativ auf S.8 sind ausschließlich `MS-OXPROTO` und `MS-OXWAVLS`. Die Ausgabe von **2016** ist kein Beweis gleicher Normeditionen wie die 2025er Exchange-Spezifikationen oder der tatsächlichen Unterstützung von Public-Folder-Free/Busy durch einen Skype-for-Business-2019-Windows-Client.
3. **Journal/Notiz/Post und informative ROP-Nennung:** `MS-OXOJRNL` nennt auf S.7 `MS-OXCFOLD`, `MS-OXCMSG`, `MS-OXOSFLD` und `MS-OXRTFCP` normativ, aber `MS-OXCROPS` (S.7) und `MS-OXPROTO` (S.8) informativ. `MS-OXONOTE` nennt `MS-OXRTFEX` **informativ** auf S.7, `MS-OXOPOST` wiederum `MS-OXCROPS` und `MS-OXPROTO` informativ auf S.7.
4. **RSS als gesondertes Quellenprofil:** `MS-OXORSS` zitiert auf S.7 `RFC4287` und `RSS2.0` **normativ** neben sechs Exchange-Protokollen und RFC2119; `MS-OXBBODY`, `MS-OXCPRPT`, `MS-OXCROPS` und `MS-OXPROTO` sind hingegen ausschließlich informativ auf S.7–8. Die historischen RFC- und Feed-Spezifikationen müssen separat editionsgenau geprüft werden.
5. **RTF-Exchange-Quelle:** `MS-OXRTFEX` zitiert `HTML`, `MSFT-RTF`, `MS-DTYP`, `MS-OXCMSG`, `RFC2119` und `RFC5234` normativ (PDF-S.6–7); `MS-OXRTFCP` nur **informativ** (S.7), obwohl andere Quellbibliografien `MS-OXRTFCP` normativ zitieren.
6. **MSMQ-Bibliografien:** `MS-MQCN → MS-MQBR`, `→ MS-MQDMPR`, `→ MS-MQDSSM`, `→ MS-MQDS`, `→ MS-MQQB` stehen normativ auf S.8. `MS-MQQP` nennt auf S.8 `MC-MQAC`, `MS-MQDMPR`, `MS-MQDSSM`, `MS-MQDS`, `MS-MQMP` und `MS-MQRR` normativ. Beide Quellen behandeln `MS-MQOD` hingegen ausschließlich informativ. Dies erzeugt neue **transitive Rohzielkandidaten**, aber keine belegte Skype-Client-MSMQ-Komplettfunktion.
7. **MSDTC-Unterlagen:** `MS-CMOM` `v20240423` Rev.27.0 nennt `MC-DTCXA`, `MS-CMPO`, `MS-CMP`, `MS-DTCM`, `MS-DTCO` und `MS-SCMR` normativ (PDF-S.9–10). Sein informatives §1.2.2-Verzeichnis enthält **„None.“**. `MS-CMP` nennt `MS-CMPO`, `MS-DTCM`, `MS-DTCO` und `MS-ERREF` normativ (S.8), aber `MS-TPSOD` nur informativ.
8. **Keine Quellenüberinterpretation:** Normative Literaturzitate sind gerichtete Dokumentationsreferenzen. Sie beweisen für sich weder eine erforderliche Funktion des **Skype for Business 2019 Windows Full Client** noch einen konkreten Softwarebuild, Serverrollenfall, Policyzustand, lückenlosen normativen Fixpunkt oder gleichwertige historische Zielausgaben.

## Abhängigkeitsfrontier auf Gesamtkantenbestand neu berechnet

Die ursprüngliche [viertringbasierte MS-/MC-Rohziel-Workqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält weiterhin **88** von der vierten Ebene abgeleitete unterschiedliche Rohkennungen. Sechs dieser Originale wurden jetzt auditiert: **61** originalgeprüfte Normbibliografien aus dieser Queue, **1** separat als §1.1-Schema (MS-ADLS) klassifizierte Quelle mit **13 weiterhin UNCLASSIFIED** Verweisen, **26** reguläre Kandidaten verbleiben.

Die [separate transitive Ziel-Workqueue](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) wurde **neu aus allen 1.008 normativen gerichteten downstream-Kanten** abgeleitet, bei Ausschluss der bereits geprüften Quellen der Ringe 1–4, sämtlicher ursprünglicher 88 Rohlabels und aller bereits geprüften downstream-Originalquellen. Vier bisher offene transitive Quellen (`MS-MQCN`, `MS-MQQP`, `MS-CMOM`, `MS-CMP`) sind jetzt auditiert, aber Folgeziele wie `MS-CMPO` (drei neue normative Eingangskanten), `MS-DTCM` (zwei), `MC-DTCXA`, `MC-MQAC`, `MS-DTCLU` und `MS-MQMP` sind hinzugekommen. Die gesonderte transitive Queue umfasst aktuell **17** offene Rohkennungen.

**Gesamt noch offene wörtliche MS-/MC-Quellkennungen = 26 + 17 = 43.** Diese 43 Kennungen sind **nicht** versionskanonisierte Dokumentidentitäten, nicht als kürzeste fünfte BFS-Ebene bewiesen und **kein** vollständig geschlossener Referenzgraph.

## Kontrollierte Zähler / Wiederaufnahmepunkt

| Forschungsbereich | Stand |
|---|---|
| Frühere normative Ringe 1, 2 und 3 | **464 / 246 / 827** gerichtete Kanten, unverändert |
| Vierte Referenz-Quellidentitäts-Ebene | **86/86 klassifiziert**, 85 §1.2.1-Bibliografien plus MS-ADLS-§1.1-Schema-Sonderfall, **1.147** normative Kanten, unverändert |
| Nachgelagerte Originalbibliografien | **73** vollständige Original-PDF-§1.2.1-Bibliografien, **1.008** normative Kanten (**912 + 96**) und **255** separate informative Literaturverweise (**231 + 24**) |
| Ursprüngliche 88er-Rohzielqueue | **61** auditiert, **1** MS-ADLS-Schemafall, **26** reguläre offen |
| Separate transitive Quellebene | **17** weitere offene Rohquellkennungen |
| Kombiniert offener wörtlicher Quellenbestand | **43** MS-/MC-Rohlabels |
| Schema-§1.1-Literatur | 67 ältere Drittstufen-Nennungen sowie 13 MS-ADLS-Nennungen **UNCLASSIFIED** |
| MS-DMRP | 2017er Originalausgabe v20170601 rev8.2 noch ausstehend; separater Microsoft-Learn-Webexport von 2026 kein binär identischer Originalnachweis |
| Phase 1 | **0/12 Abschlussgates bestanden** |

**Nächste sinnvolle Originalgruppe:** offene Quellen der initialen Queue `MS-OXORMMS`, `MS-OXOSMMS`, `MS-OXOUM`, `MS-OXPFOAB`, `MS-OXPHISH` und `MS-OXPSVAL`; transitive Ziele `MS-CMPO`, `MS-DTCM`, `MC-DTCXA`, `MC-MQAC`. Auswahl nach tatsächlicher Archivverfügbarkeit. Parallel bleiben die externe RFC/ISO/ITU/IETF/W3C-Originaleditionen, die kanonische historische Zielauflösung und die unabhängige SfB-2019-Windows-Full-Client-Build-/Policy-/Serverrollen-Pflichtmatrix OPEN.

**Verbindliche Sperren bis zum formellen Phase-1-Abschluss:** Keine Protokollimplementierung, keine Live-Netzwerk-/Interop-Tests, keine bezahlten CI-Runner, keine Abschwächung von TLS-/Auth-Sicherheit, kein Merge nach `main`, Tag oder Release. Original-PDF-Binärdateien verbleiben ausschließlich beim Nutzer.
