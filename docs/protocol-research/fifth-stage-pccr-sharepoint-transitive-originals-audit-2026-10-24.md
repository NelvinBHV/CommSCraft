# P1-03 – Weitere zehn transitive Originalquellen (§1.2.1 und §1.2.2)

**24. Oktober 2026 · Branch `draft/protocol-spec-analysis-2026-10-23` · Ausgangs-HEAD `6070875822ae64f50459d30528a59b3f3a929222`.** Vor der Fortsetzung wurde festgestellt, dass der zuvor als „uncommitted“ bezeichnete Batch im unmittelbar vorherigen Commit bereits veröffentlicht war. Die zehn vorherigen PDFs wurden aus den Nutzer-ZIP-Originalen mit den im Repo gespeicherten vollständigen SHA-256-Prüfsummen gegengeprüft (10/10). Daher keine Doppelbuchung der vorherigen 99/36.

Der hier **neue** Zehnerbatch basiert auf Original-Binärdateien in `CommSCraft_Gesamtanalyse_Teil_01–04` (nicht in Git); die ganzen SHA-256 sind direkt aus den PDF-Binärdateien berechnet. Die nachfolgenden Seiten sind **PDF-Seiten (1-basiert)**, nicht gedruckte Inhaltsverzeichnisseiten. Einträge wurden ausschließlich im wirklichen `1.2.1 Normative References` bis `1.2.2 Informative References` bzw. `1.3` ausgelesen; sich wiederholende `[MS-...] - vYYYYMMDD`-Fußzeilen wurden ausgeschlossen. Die durch gegenseitige Zitate entstehenden Zyklen, insbesondere `MS-PCCRD ↔ MS-PCCRR`, werden als gerichtete Einzelkanten beibehalten, aber nicht unkontrolliert expandiert. Zitierten historischen **Ziel**editionen wurden nicht stillschweigend normalisiert.

| Original | Edition | Revision | Seiten | §1.2.1 Normativ | §1.2.2 Informativ | Vollständiger SHA-256 der Original-PDF |
|---|---|---:|---:|---:|---:|---|
| MC-FPSEWM | v20190319 | 8.5 | 135 | 11 | 2 | `8a6574e3bdf11c28492a0937b1fdfcf8d4dce498282ea8b648717db5b102e347` |
| MS-PCCRD | v20240423 | 15.0 | 32 | 11 | 3 | `b84075276e143e8942f0312806b65314d4712932bb353084e47c6f53c03132c9` |
| MS-PCCRR | v20240423 | 15.0 | 43 | 7 | 2 | `95427f2f489c5fc59c92deb671246c7f567cf86f5c7068a6c4074f3a9abb3f70` |
| MS-PCCRTP | v20240423 | 13.0 | 21 | 4 | 2 | `d47d451152a7da7c8a89ed094e44d7af63f5dfb7010d80f5ab553f585e74539c` |
| MS-PCHC | v20240423 | 12.0 | 31 | 9 | 4 | `f34f8c983f7c958ca55d35d55bc3386da884ae274ef437bafd25beb82bfcc951` |
| MS-FSSHTTPB | v20240820 | 24.0 | 117 | 5 | 1 | `353845858dd84af3408a2bc33b63ce1c47eac8ba1872c4524c9482c9781ea3b4` |
| MS-LISTSWS | v20240820 | 18.0 | 215 | 25 | 1 | `1ebf0d650aed389de8253907ec3413025c3bec85fe382c18924c36f78090bd51` |
| MS-SHDACCWS | v20240820 | 14.0 | 21 | 12 | 1 | `b0965d1109f1de657dca6251778a0403591427ccab2c793526e35c2aaad6b5d6` |
| MS-VERSS | v20220215 | 10.1 | 40 | 13 | 1 | `8148d9a8c4e6307b93d0112d76729d3f445aef5761301cdda64d55b3986ec8e5` |
| MS-WMIO | v20240423 | 18.0 | 74 | 7 | 1 | `ef8357da5e82fb3a5d165efd0cceb5d75e6064efcb2efee9f4da261154ff2b06` |
| **Gesamt neu** | | | | **104** | **18** | |

## Wörtliche gerichtete Kanten mit Fundseiten

### MC-FPSEWM · v20190319 · Rev. 8.5

Original im Nutzerarchiv: `PDFs/SharePoint/[MC-FPSEWM].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_01.zip`), 3383344 Byte. Normativ §1.2.1: `MS-DTYP` (PDF-S. 12), `MS-WDVSE` (PDF-S. 12), `MSDN-ThemeDef` (PDF-S. 12), `RFC1123` (PDF-S. 12), `RFC1341` (PDF-S. 13), `RFC2119` (PDF-S. 13), `RFC2279` (PDF-S. 13), `RFC2616` (PDF-S. 13), `RFC2854` (PDF-S. 13), `RFC3986` (PDF-S. 13), `RFC4234` (PDF-S. 13). Informativ **nur** §1.2.2: `MS-FPSE` (PDF-S. 13), `MS-WDVME` (PDF-S. 13).

### MS-PCCRD · v20240423 · Rev. 15.0

Original im Nutzerarchiv: `PDFs/Windows/[MS-PCCRD].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_03.zip`), 1240341 Byte. Normativ §1.2.1: `MS-PCCRC` (PDF-S. 8), `MS-PCCRR` (PDF-S. 9), `RFC2119` (PDF-S. 9), `RFC3986` (PDF-S. 9), `RFC768` (PDF-S. 9), `SOAP-UDP` (PDF-S. 9), `SOAP1.2-1/2003` (PDF-S. 9), `WS-Discovery` (PDF-S. 9), `WSDL` (PDF-S. 9), `XMLNS` (PDF-S. 9), `XMLSCHEMA2/2` (PDF-S. 9). Informativ **nur** §1.2.2: `MC-BUP` (PDF-S. 9), `MSDN-BITS` (PDF-S. 9), `WS-Addr-Core` (PDF-S. 9).

### MS-PCCRR · v20240423 · Rev. 15.0

Original im Nutzerarchiv: `PDFs/Windows/[MS-PCCRR].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_03.zip`), 1655953 Byte. Normativ §1.2.1: `FIPS197` (PDF-S. 8), `MS-DTYP` (PDF-S. 8), `MS-PCCRC` (PDF-S. 8), `MS-PCCRD` (PDF-S. 8), `RFC2119` (PDF-S. 9), `RFC2616` (PDF-S. 9), `SP800-38A` (PDF-S. 9). Informativ **nur** §1.2.2: `MC-BUP` (PDF-S. 9), `MSDN-BITS` (PDF-S. 9).

### MS-PCCRTP · v20240423 · Rev. 13.0

Original im Nutzerarchiv: `PDFs/Windows/[MS-PCCRTP].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_03.zip`), 886276 Byte. Normativ §1.2.1: `MS-PCCRC` (PDF-S. 6), `RFC2119` (PDF-S. 6), `RFC2616` (PDF-S. 6), `RFC793` (PDF-S. 6). Informativ **nur** §1.2.2: `MC-BUP` (PDF-S. 6), `MSDN-BITS` (PDF-S. 6).

### MS-PCHC · v20240423 · Rev. 12.0

Original im Nutzerarchiv: `PDFs/Windows/[MS-PCHC].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_03.zip`), 1308446 Byte. Normativ §1.2.1: `MS-DTYP` (PDF-S. 8), `MS-PCCRC` (PDF-S. 8), `MS-PCCRR` (PDF-S. 8), `MS-SPNG` (PDF-S. 8), `RFC2119` (PDF-S. 8), `RFC2616` (PDF-S. 8), `RFC2743` (PDF-S. 8), `RFC2818` (PDF-S. 8), `RFC4559` (PDF-S. 8). Informativ **nur** §1.2.2: `MC-BUP` (PDF-S. 8), `MS-BPCR` (PDF-S. 8), `MSDN-BITS` (PDF-S. 8), `MSDOCS-WinINet` (PDF-S. 8).

### MS-FSSHTTPB · v20240820 · Rev. 24.0

Original im Nutzerarchiv: `PDFs/SharePoint/[MS-FSSHTTPB].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_02.zip`), 4343531 Byte. Normativ §1.2.1: `MS-ERREF` (PDF-S. 9), `MS-FSSHTTP` (PDF-S. 9), `MS-PCCRC` (PDF-S. 9), `RFC2119` (PDF-S. 9), `RFC2822` (PDF-S. 9). Informativ **nur** §1.2.2: `MS-FSSHTTPD` (PDF-S. 9).

### MS-LISTSWS · v20240820 · Rev. 18.0

Original im Nutzerarchiv: `PDFs/SharePoint/[MS-LISTSWS].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_02.zip`), 5635198 Byte. Normativ §1.2.1: `ISO-8601` (PDF-S. 15), `MS-LCID` (PDF-S. 15), `MS-OUTSPS` (PDF-S. 15), `MS-PRSTFR` (PDF-S. 15), `MS-VIEWSS` (PDF-S. 15), `MS-WSSCAML` (PDF-S. 15), `MS-WSSFO2` (PDF-S. 15), `MS-WSSFO3` (PDF-S. 15), `MS-WSSTS` (PDF-S. 15), `MS-XSSK` (PDF-S. 15), `RFC1123` (PDF-S. 15), `RFC2045` (PDF-S. 15), `RFC2119` (PDF-S. 15), `RFC2396` (PDF-S. 16), `RFC2616` (PDF-S. 16), `RFC2822` (PDF-S. 16), `RFC4648` (PDF-S. 16), `SOAP1.1` (PDF-S. 16), `SOAP1.2-1/2007` (PDF-S. 16), `SOAP1.2-2/2007` (PDF-S. 16), `WSDL` (PDF-S. 16), `XMLNS` (PDF-S. 16), `XMLSCHEMA1/2` (PDF-S. 16), `XMLSCHEMA1` (PDF-S. 16), `XMLSCHEMA2/2` (PDF-S. 16). Informativ **nur** §1.2.2: `RFC2818` (PDF-S. 16).

### MS-SHDACCWS · v20240820 · Rev. 14.0

Original im Nutzerarchiv: `PDFs/SharePoint/[MS-SHDACCWS].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_02.zip`), 953140 Byte. Normativ §1.2.1: `MS-LISTSWS` (PDF-S. 6), `MS-WSSCAML` (PDF-S. 6), `RFC2119` (PDF-S. 6), `RFC2616` (PDF-S. 6), `SOAP1.1` (PDF-S. 6), `SOAP1.2-1/2007` (PDF-S. 6), `SOAP1.2-2/2007` (PDF-S. 6), `WSDL` (PDF-S. 6), `XMLNS` (PDF-S. 6), `XMLSCHEMA1/2` (PDF-S. 7), `XMLSCHEMA1` (PDF-S. 7), `XMLSCHEMA2/2` (PDF-S. 7). Informativ **nur** §1.2.2: `RFC2818` (PDF-S. 7).

### MS-VERSS · v20220215 · Rev. 10.1

Original im Nutzerarchiv: `PDFs/SharePoint/[MS-VERSS].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_02.zip`), 1323874 Byte. Normativ §1.2.1: `ISO-8601` (PDF-S. 8), `MS-LISTSWS` (PDF-S. 8), `MS-WSSTS` (PDF-S. 8), `RFC2119` (PDF-S. 8), `RFC2616` (PDF-S. 8), `SOAP1.1` (PDF-S. 8), `SOAP1.2-1/2007` (PDF-S. 8), `SOAP1.2-2/2007` (PDF-S. 8), `SOAP1.2/1` (PDF-S. 8), `WSDL` (PDF-S. 8), `XMLNS` (PDF-S. 8), `XMLSCHEMA1` (PDF-S. 9), `XMLSCHEMA2` (PDF-S. 9). Informativ **nur** §1.2.2: `RFC2818` (PDF-S. 9).

### MS-WMIO · v20240423 · Rev. 18.0

Original im Nutzerarchiv: `PDFs/Windows/[MS-WMIO].pdf` (Archivgruppe `CommSCraft_Gesamtanalyse_Teil_04.zip`), 2259510 Byte. Normativ §1.2.1: `DMTF-DSP0004` (PDF-S. 9), `IEEE754` (PDF-S. 10), `MS-DCOM` (PDF-S. 10), `MS-WMI` (PDF-S. 10), `RFC2119` (PDF-S. 10), `RFC4234` (PDF-S. 10), `UNICODE` (PDF-S. 10). Informativ **nur** §1.2.2: `DMTF` (PDF-S. 10).

## Auswertung und Grenzen

- Die 104 gerichteten §1.2.1-Normzitate werden in `fifth-ring-ms-swn-normative-ledger.json` aufgenommen, 18 informative §1.2.2-Verweise ausschließlich in dessen `informative_refs`-Register. Gesamt jetzt **123 originalgeprüfte Quellbibliografien / 1.576 normative gerichtete Kanten / 438 informative Verweise**.
- Die transitive zusätzliche MS-/MC-Rohzielqueue wurde **über sämtliche 1.576 Kanten neu aufgebaut**. Abzüge: Quellen der Ebenen 1–4, die ursprünglichen 88 wörtlichen Rohzielkennungen sowie bereits geprüfte downstream-Quellen. Neu offenes Kollektiv: **25** Rohkennungen; zusammen mit den 3 noch offenen ursprünglichen regulären Quellen **28** unterschiedliche Rohkennungen. Neue Ziele gegenüber der vorherigen 28er-Liste: `MS-OUTSPS`, `MS-PRSTFR`, `MS-VIEWSS`, `MS-WSSCAML`, `MS-WSSFO3`, `MS-WSSTS`, `MS-XSSK`.
- Ursprüngliche 88er-Queue unverändert: **84** vollständige Normbibliografien geprüft, **1** MS-ADLS-Schema-§1.1-Sonderfall (13 Referenzen bleiben UNCLASSIFIED) und **3** reguläre Originale fehlen: MS-DMRP, MS-ODATAJSON, MS-SQLRS. 67 ältere allgemeine Schema-§1.1-Verweise bleiben ebenfalls **UNCLASSIFIED**.
- `MS-DMRP v20170601 Rev. 8.2` liegt unverändert nicht als identisches historisches Original vor; 2026er Learn-Webexport ist kein Ersatz. Externe RFC/IETF/ISO/ITU/W3C-Quellen, genaue zitierte Zielausgaben und tatsächliche Skype-for-Business-2019-Windows-Full-Client-Rollen-/Build-/Policy-Anwendbarkeit bleiben offen. Die Kanten sind **Literaturbeziehungen, keine automatischen Implementierungspflichten**. **Kein normativer Fixpunkt, keine kürzeste-Ebenen-Behauptung; Phase 1: 0/12 Gates PASS.**

**Nächster Wiederaufnahmepunkt:** Zuerst GitHub-HEAD, diese Datei, das Audit-JSON, Normledger, initiale und transitive Queue sowie Phase-1-Gates erneut lesen. Dann die in der neuen Transitqueue aufgeführten und in den vier Nutzer-ZIPs vorhandenen Originale weiter prüfen (insbesondere `MS-WSSCAML`, `MS-WSSTS`, `MS-WSSFO3`, `MS-SPNG` nach Verfügbarkeitsprüfung); ausschließlich fehlende Editionen gezielt nachfordern. Nur Forschungsdokumente im Draft-Branch committen; kein Code, Test, CI-Upgrade, Merge, Tag oder Release.
