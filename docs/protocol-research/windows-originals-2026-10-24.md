# Windows Open Specifications – vollständiges 451-PDF-Quellarchiv und 6 weitere exakte zweite Microsoft-Ausgaben (24.10.2026)

**FACT:** Vom Nutzer bereitgestelltes Originalarchiv `Windows_Protocols.tar.xz` auf sicher zugreifbare Pfade, PDF-Signaturen und vollständige Datei-Hashes geprüft. **451 gültige PDFs** (**424 MS- und 27 MC-Spezifikationen**), zusammen **1.334.078.644 entpackte PDF-Bytes**. Archiv-SHA-256: `52f87efc17c9854f70b0b14650c877a572b1a99c7ed9df4f6c9a96387720c652`. Die Originalbinaries wurden lokal gelesen, jedoch **nicht in das öffentliche GitHub-Repository committet**. Öffentliche Voll-PDF-Weiterverbreitungsrechte sind weiterhin nicht nachgewiesen; Zugriff auf ein zulässiges privates Quellenarchiv ist offen.

## Exakt geprüfte zweite Microsoft-Ebene

| Microsoft Original | Revision / Datum / Footer | Vollständiges PDF | SHA-256 | §1.2.1: normative Kennungen |
| --- | --- | --- | --- | ---: |
| MS-H26XPF | 1.3 / 2017-06-01 / `v20170601` | 26 S., 1129519 Bytes | `c3ec281d1b8b4c4bd74b751427c957067574c225a1f4d0256d2e49cdd4adef81` | 4 (ab PDF-S. 6) |
| MS-RDPBCGR | 62.0 / 2026-03-09 / `v20260309` | 444 S., 14529191 Bytes | `63530e1feb089fe442387c2162eb7f7d84eea98bdd4d6bf46223aa1ed3ede0ac` | 37 (ab PDF-S. 20) |
| MS-SDP | 1.0 / 2017-06-01 / `v20170601` | 25 S., 1074616 Bytes | `f58d4d3d73bcdd1836670b6827dca917d8751bcc9ee33c4a3518c310012d6848` | 7 (ab PDF-S. 6) |
| MS-WCCE | 55.0 / 2026-08-24 / `v20260824` | 267 S., 6694053 Bytes | `7cee4f03124e0682cced08f6895ad4c05eabc0729c777bb15399b5d4dfa2640b` | 61 (ab PDF-S. 22) |
| MS-WSPOL | 8.0 / 2019-03-13 / `v20190313` | 36 S., 1360651 Bytes | `7927d6a498e235134c6d3713abb05eaf875349e94f588307760d9d4c5a482008` | 20 (ab PDF-S. 7) |
| MS-WSTEP | 15.0 / 2024-04-23 / `v20240423` | 43 S., 1374899 Bytes | `dd2f0a760a254c48da4d31c65c798f4156e04744ee40c4b067c24b913a64e5d5` | 16 (ab PDF-S. 7) |

**FACT:** Ausgaben anhand der Originalrevisionstabellen und PDF-Fußzeilen geprüft, die jeweiligen §1.2.1-Abschnitte extrahiert und ihre Anfangsseiten als PDF gerendert und visuell geprüft. Die normativen Zitationskennungen aus den aktuellen Originalausgaben wurden gegen die bereits normalisierten `transitive-reference-ledger.json`-Einträge verglichen. Es stimmen **4 + 37 + 7 + 61 + 20 + 16 = 145** bestehende gerichtete Quellreferenzen überein; **keine** neue Normkante. Beim Abgleich wurden PDFs wiederholt zitierende Footer-Selbstnennungen ausdrücklich nicht als bibliografische Kanten gewertet.

**Editionstreue:** In MS-WSPOL §1.2.1 (2019) ist **XML Namespaces Second Edition 2006** normativ; MS-WSTEP §1.2.1 (2024) nennt hingegen **Third Edition 2009**, und beide zitieren XML Schema Part 1 **First Edition 2001**. MS-RDPBCGR rev 62.0 (2026) zitiert zugleich ältere TLS-RFCs und RFC8446 sowie `draft-ietf-oauth-pop-key-distribution-07`, was **keine** Freigabe für SSL3, unsichere TLS-Downgrades oder universelle 2019-Client-Pflichten impliziert. MS-WCCE rev 55.0 (2026) führt zahlreiche Certificate/AD/Kerberos-Abhängigkeiten. Editionsversionsstand der 2026er Dokumente und der tatsächliche SfB-2019-Client-/Server-/Policy-Stand sind getrennt zu klären.

## Dritte Ebene: Originaldateien vorhanden, Normrekursion offen

Von **52** neuen Microsoft-/MC-Zielkennungen des zweiten Ring-Fronters liegen die **46 Windows-/MC-Originale** in diesem Archiv; die weiteren **sechs Exchange-/SharePoint-Originale** wurden bereits im vorherigen ZIP-Audit bezogen. Damit ist die *Dateibeschaffung für diese 52 IDs* gelungen. Keine dieser 52 Zielkennungen wird dadurch automatisch zu einem verpflichtenden Windows-Full-Client-Feature, und ihr Rekursions-/Editionenabschluss ist **nicht** nachgewiesen.

## Kollisionen bei früheren Originaldatei-Prüfsummen

**OPEN – historische Evidenz unbedingt erhalten:** Die folgenden in Windows_Protocols.tar.xz vorliegenden PDFs zeigen die gleiche veröffentlichte Versionskennung wie frühere hochgeladene Originaldateien, jedoch abweichende SHA-256 und Dateigrößen:

- **MS-NLMP 37.0:** Windows-Archiv `dd8c9d6bcd3562a903ce66ed03eea51ec17645b9d293e7e37c2aceb01efc2c86` (3021682 Bytes), historischer Originalnachweis `00fe14a058134aabf6a6db1bcc1c47b8d45f0656ef7de610c03712906a945a27` (3032203 Bytes).
- **MS-RDPEMC 16.0:** Windows-Archiv `4649fa49ddb48301452c64745de56a8289f9ca76cb689f9f38dbccd28dd401b0` (1680715 Bytes), historischer Originalnachweis `17658ee28cef61d0ba11beb69be0e1fbe9e0b0f6aa7c146d5bc4100f64c0985b` (1698642 Bytes).
- **MS-SIP 8.0:** Windows-Archiv `97d866690b82cda1df018007cd297d96eb8142d5b95f50825e2f6b8aeedb7ea5` (2809897 Bytes), historischer Originalnachweis `6ee1c16c922b7c154373572d5a3341139aaf6d91922a35468581a209f68e9e52` (2709399 Bytes).

**MS-KILE 49.0** ist ein Gegenbeispiel: neuer Archiv-SHA-256 `8d172e6fb7b760812fba2e41b1bb591dc80fcfce441419153cc93622456e7a01` stimmt mit dem früheren Hash überein. Ohne frühere Originalbytes kein substantieller Binär-/Inhaltsdiff der drei Konfliktquellen; **keine** alten Ledger-Hashes überschreiben.

## Nachweise, nicht zu überinterpretieren

- **12/12** zweite Microsoft-Ebene: aktuelle exakte PDF-§1.2.1-Edition und ursprüngliche Normzielmengen belegt; vorher 6/12. **246** gerichtete zweite Ebene, **464** erste Ebene unverändert.
- **451** vollständige Windows-PDF-Dateihashes im lokalen Prüfsummenmanifest `Windows_Protocols_SHA256SUMS.txt` (Manifest-SHA-256 `e81077d69bea8493947e7588130425ef4237789fcf2a035d0ad4fd8b6aaadc27`). Vollständiges JSON-Metadatenmanifest SHA-256 `57f84d8b8ac58a17d99e573a746ce994e2abc786f68ed539c97a54dd72c5bc29` und detaillierte 145-Kanten-Korrespondenz SHA-256 `6ba40e44c67c6c7f13f1271900161b13f813eca38f80d4e4a37719f471aea624`. Diese vollständigen Einzeldateimanifeste sind als Gesprächsartefakte abrufbar; Git enthält zunächst die belastbaren Ausschnittsnachweise und den Manifest-Fingerprint.
- **OFFEN:** Alle RFC-/IETF-Draft- und externen Normenrekursionen, 52 dritte-MS-Ebene normative Originalbibliografien, vollständige Vorrevisions-Diffs, exakt fixierter 2019-Windows-Client-/Server-Rollen-/Policy-Stand, Rechte-/privates Originalbinary-Archiv, unabhängige Review. **0/12 formale Phase-1-Gates PASS; Phase 1 INCOMPLETE.**
- **Keine** Protokollimplementierung, echte Netzinteroperabilität, GitHub-Actions-Runner-Änderungen, Merge nach `main`, Tag oder Release.
