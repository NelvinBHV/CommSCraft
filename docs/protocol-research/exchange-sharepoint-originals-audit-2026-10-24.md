# Exchange-/SharePoint-Originalquellen — exakter PDF-Abgleich vom 24.10.2026

**FACT:** Die vom Auftraggeber bereitgestellten Archive `Exchange_Protocols.zip` und `SharePointProtocols.zip` wurden lokal geprüft. Es liegen **129 + 168 = 297** vollständige PDF-Dateien mit gültigem PDF-Kopf und unabhängig berechneten **SHA-256-Prüfsummen** vor (268,0 + 552,2 MB entpackte PDF-Bytes). Die ZIP-SHA-256 sind `e3729e5712fd11b087a2c3560d5490d54afed04f2ca80562cbeb6020373c9f88` (Exchange) und `0b3f3ccfe190a2988edc0d1d74a47cb1f81106c13e30dc9ab60bebf672974fc1` (SharePoint). Die vollständigen Einzeldateimanifeste sind als lokale Forschungsartefakte verfügbar; die Original-PDFs wurden **nicht** in das öffentliche Git-Repository hochgeladen.

## Zwei neue exakte Microsoft-§1.2.1-Originale der zweiten Ebene

| Original | Revision / Veröffentlichungsdatum | PDF-Fundstelle | SHA-256 Vollfile | Normativ / Informativ |
|---|---|---|---|---|
| [MS-OAUTH2EX](https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-oauth2ex/56105233-7ca7-4b4b-821d-84bb55bfc2cf) | **8.0 / 20.05.2025** | **§1.2.1, S. 5**; 19 Seiten | `fd0c0291abb6c3d696872855243ef8b59bf8e413c55c0b9303c4644d9117cc74` | 7 / 3 |
| [MS-OXWUMS](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwums/79ace092-b793-4fa5-82b0-dd30d12c14c2) | **16.0 / 20.05.2025** | **§1.2.1, S. 10–11**; 107 Seiten | `61447ba6fdbb82a4cd11ad3d433aef369d764da286294acb7e65b92e91d54f1d` | 16 / 3 |

Beide Originalseiten wurden als PDF-Text extrahiert und **visuell gerendert**. Revisions-/Ausgabedatum sind in den Revisionstabellen (MS-OAUTH2EX **S. 2**, MS-OXWUMS **S. 3**) und den `v20250520`-PDF-Fußzeilen ausgewiesen. Die **7+16=23** normativen Zielkennungen entsprechen exakt den vorhandenen Einträgen in `transitive-reference-ledger.json`; **keine** neue Kante und keine Umklassifizierung der informativen Referenzen.

### MS-OAUTH2EX 8.0: historischer Draftkonflikt jetzt originalbelegt

**FACT – MS-OAUTH2EX rev 8.0, 20.05.2025, §1.2.1 S. 5:** Die Spezifikation nennt `draft-ietf-oauth-json-web-token-32`, `MS-SPS2SAUTH`, RFC 2119, 2818, 3986, 793 sowie **`draft-ietf-oauth-v2-22` im Zitattext**, allerdings verweist der Hyperlink auf **`draft-ietf-oauth-v2-23`**. Die Editionsbeschaffung ist jetzt nachgewiesen; **die beabsichtigte normative OAuth-Draft-Revision bleibt OPEN**, darf weder zu -22 noch -23 noch RFC 6749 geraten werden. §1.2.2 S. 5 führt MS-OXPROTO, MS-XOAUTH und RFC2616 *informativ*. Die tatsächliche SfB-Windows-Client-Anwendbarkeit bleibt OPEN.

### MS-OXWUMS 16.0: originaler Exchange-UM-Normverweis

**FACT – MS-OXWUMS rev 16.0, 20.05.2025, §1.2.1 S. 10–11:** Normativ sind MS-OXDSCLI, MS-OXWSCDATA, MS-OXWSCORE, MS-OXWSCVTID, MS-OXWSSRCH, RFC 2045/2119/2818/3066/3261/3966 sowie SOAP 1.1, WSDL 1.1, XML Namespaces Third Edition **2009**, XML Schema Parts 1 und 2 **First Edition 2001**. Letztere dürfen **nicht** ohne Editionsvergleich durch die 2004er Second Edition ersetzt werden. §1.2.2 S. 11 listet MS-OXPROTO, MS-OXWCONFIG, MS-OXWSADISC als *informativ*.

## Weitere sechs neue, bislang nicht normativ abgeschlossene Originalquellen

Das Normregister verweist unmittelbar auf die folgenden Microsoft-Dokumente, deren vollständige Original-PDF-Binärdateien ebenfalls in den ZIPs liegen. Editionsdatum und Revision stammen aus PDF-Fußzeile und Revisionstabelle; ein Vergleich mit dem aktuellen offiziellen Veröffentlichungsregister und die jeweilige vollständige §1.2.1-Analyse bleiben offen.

| ID | Ausgabe | Vollfile SHA-256 |
|---|---|---|
| MS-OXDSCLI | 24.1 / 2025-08-19 | `0aefc91d1ce02affc1c54cbd5dbe08627df88b0826cc847aa860a2630894a516` |
| MS-OXWSCDATA | 21.0 / 2025-05-20 | `25f8654d818681d03567684d5302b6bb567ec03aa97507fc1389e7898f2ba0d6` |
| MS-OXWSCORE | 18.1 / 2025-08-19 | `9198ac4d1610a3408a9be2b64a1a5f4ee573d206608a2f51dd62390778fb0f21` |
| MS-OXWSCVTID | 13.0 / 2025-05-20 | `aa0124e5354993db61be7bad8cd3ea9ac7f297aa3695f41bcda882e63285694d` |
| MS-OXWSSRCH | 18.0 / 2025-05-20 | `c553ef009263ad3ba52c640387f2c8855d27f4538e8ec3bb116111e16d4f675d` |
| MS-SPS2SAUTH | 5.0 / 2025-05-20 | `b353607b9fef5b955067ad24872ffea08380b8c320e5f277604546c25389590f` |

## Aktualisierter Nachweisstand und Grenzen

- Erste Microsoft-Ebene: **46/46** aktuelle §1.2.1-PDF-Nachweise, **464** direkte Normkanten – unverändert seit dem vorherigen ZIP-Audit.
- Zweite Microsoft-Ebene: **6/12** aktuelle §1.2.1-PDF-Nachweise statt 4/12, **246** bereits dokumentierte Normkanten bleiben unverändert; sechs ungeprüfte Originale: MS-H26XPF, MS-RDPBCGR, MS-SDP, MS-WCCE, MS-WSPOL, MS-WSTEP.
- Bibliografische Kette für die sechs zusätzlich akquirierten Referenzquellen, historische Versionsdifferenzen, externe ISO/W3C/ITU-Ausgaben, RFC-Updates/Errata, Produktbuild und Server-/Policy-Anwendbarkeit sind **OPEN**.
- **0/12 formale Phase-1-Abschlussgates PASS.** Nur Forschung, keine Protokollimplementierung, keine Live-Tests, kein Merge nach `main` und kein Release.
- **Archivierungs-/Rechtegate:** Der vollständige Microsoft-Original-PDF-Hinweis gestattet Kopien zur Implementierungsentwicklung und die Weitergabe von Teilen, aber keine eindeutig erlaubte pauschale öffentliche Voll-PDF-Weiterverteilung. Das GitHub-Repository ist öffentlich; deshalb wurden keine vollständigen PDF-Binärdateien dorthin committet. Privates zulässiges Binärarchiv weiterhin offen.
