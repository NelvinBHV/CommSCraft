# P1-03: Acht Quellen der vierten MS-/MC-Referenzebene (24.10.2026)

Base-HEAD: 1868ec97ee463b19fa370aead6fb8e3b4c864d40. Vier aktuelle v20250520-Microsoft-Exchange-PDFs sind an §1.2.1 und §1.2.2 im PDF-Seitenbild sowie PDF-Text geprüft. Vier Windows-Dokumente sind nur editionsbezogen vorgeprüft, weil deren PDFs nicht zuverlässig erreichbar waren.

## Exakte Originaleditionen, Seiten und Prüfsummen

| Quelle | Revision / Veröffentlichung | Normative §1.2.1 | Informative §1.2.2 | Vollfile SHA-256 |
| --- | --- | --- | --- | --- |
| [MS-OXCMAPIHTTP](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxcmapihttp/d502edcf-0b22-42f2-8500-019f00d60245) | 15.0 / v20250520 / 2025-05-20, 96 PDF-S. | 12 / S. 9 | 2 / S. 9 | OPEN: vollständige PDF-Bytes nicht verfügbar |
| [MS-OXWSSYNC](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwssync/9ac26095-b7fc-430e-94cf-0dc34b9a09a0) | 10.0 / v20250520 / 2025-05-20, 45 PDF-S. | 20 / S. 7,8 | 3 / S. 8 | OPEN: vollständige PDF-Bytes nicht verfügbar |
| [MS-OXWSXPROP](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwsxprop/1e0ae5e6-c516-4ad1-ad12-c19fee04860a) | 13.0 / v20250520 / 2025-05-20, 24 PDF-S. | 9 / S. 5,6 | 4 / S. 6 | OPEN: vollständige PDF-Bytes nicht verfügbar |
| [MS-OXWSMSG](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwsmsg/74051353-1b3f-4298-95c0-e3a54a8512ca) | 15.0 / v20250520 / 2025-05-20, 46 PDF-S. | 13 / S. 7,8 | 2 / S. 8 | OPEN: vollständige PDF-Bytes nicht verfügbar |
| [MS-FSCC](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/efbfe127-73ad-4140-9967-ec6500e66d5e) | 60.0 / 2025-11-21 (offizielle Editionstabelle) | OPEN, PDF nicht geprüft | OPEN | OPEN |
| [MS-SMB2](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-smb2/5606ad47-5ee0-437a-817e-70c366052962) | 88.0 / 2026-09-28 (offizielle Editionstabelle) | OPEN, PDF nicht geprüft | OPEN | OPEN |
| [MS-GPOL](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gpol/62d12924-6252-4052-996f-161d2b9019f4) | 39.0 / 2024-04-23 (offizielle Editionstabelle) | OPEN, PDF nicht geprüft | OPEN | OPEN |
| [MS-TLSP](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tlsp/58aba05b-62b0-4cd1-b88b-dc8a24920346) | 18.0 / 2026-09-28 (offizielle Editionstabelle) | OPEN, PDF nicht geprüft | OPEN | OPEN |

Die angegebene SHA-256 ist in allen vier neuen Exchange-Quellen ausdrücklich NULL, nicht ein Hash der Webseite und nicht ein früherer Archivhash. Früher verifizierte MS-DLTM SHA-256 im alten Ledger bleibt unangetastet. Alte Nutzer-Archiv-PDFs müssen erst revisionsgenau gegen die neuen 2026-Ausgaben abgeglichen werden.

## Pro Original normativ versus informativ

### MS-OXCMAPIHTTP / rev 15.0 / v20250520

**§1.2.1 (ausschließlich normative Kanten):** PDF-S. 9: [MS-LCID], [MS-OXABREF], [MS-OXCDATA], [MS-OXCNOTIF], [MS-OXCROPS], [MS-OXCRPC], [MS-OXDSCLI], [MS-OXNSPI], [MS-OXOABK], [RFC2119], [RFC2616], [RFC2818].

**§1.2.2 (informativ, KEINE Normkanten):** PDF-S. 9: [MS-OXPROTO], [RFC896].

PDF-Quelllink: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OXCMAPIHTTP/%5BMS-OXCMAPIHTTP%5D.pdf. Client-/Serverrollen-/Policy-Anwendbarkeit für Skype for Business 2019 Windows Full Client bleibt OPEN.

### MS-OXWSSYNC / rev 10.0 / v20250520

**§1.2.1 (ausschließlich normative Kanten):** PDF-S. 7: [MS-OXWSCDATA], [MS-OXWSCONT], [MS-OXWSCORE], [MS-OXWSDLIST]; PDF-S. 8: [MS-OXWSFOLD], [MS-OXWSMSG], [MS-OXWSMTGS], [MS-OXWSPOST], [MS-OXWSSRCH], [MS-OXWSTASK], [RFC2119], [RFC2616], [RFC2818], [RFC3066], [SOAP1.1], [WSDL], [WSIBASIC], [XMLNS], [XMLSCHEMA1], [XMLSCHEMA2].

**§1.2.2 (informativ, KEINE Normkanten):** PDF-S. 8: [MS-OXDSCLI], [MS-OXPROTO], [MS-OXWSADISC].

PDF-Quelllink: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OXWSSYNC/%5BMS-OXWSSYNC%5D.pdf. Client-/Serverrollen-/Policy-Anwendbarkeit für Skype for Business 2019 Windows Full Client bleibt OPEN.

### MS-OXWSXPROP / rev 13.0 / v20250520

**§1.2.1 (ausschließlich normative Kanten):** PDF-S. 5: [MS-DCOM], [MS-DTYP], [MS-OAUT]; PDF-S. 6: [MS-OXWSCDATA], [RFC2119], [WSDL], [XMLNS], [XMLSCHEMA1], [XMLSCHEMA2].

**§1.2.2 (informativ, KEINE Normkanten):** PDF-S. 6: [MS-OXPROTO], [MS-OXWSCORE], [MS-OXWSFOLD], [MS-OXWSSRCH].

PDF-Quelllink: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OXWSXPROP/%5BMS-OXWSXPROP%5D.pdf. Client-/Serverrollen-/Policy-Anwendbarkeit für Skype for Business 2019 Windows Full Client bleibt OPEN.

### MS-OXWSMSG / rev 15.0 / v20250520

**§1.2.1 (ausschließlich normative Kanten):** PDF-S. 7: [MS-OXDSCLI], [MS-OXWSADISC], [MS-OXWSCDATA], [MS-OXWSCORE], [MS-OXWSFOLD], [RFC2119], [RFC2616]; PDF-S. 8: [RFC2818], [SOAP1.1], [WSDL], [XMLNS], [XMLSCHEMA1], [XMLSCHEMA2].

**§1.2.2 (informativ, KEINE Normkanten):** PDF-S. 8: [MS-OXPROTO], [MS-OXWSSRCH].

PDF-Quelllink: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OXWSMSG/%5BMS-OXWSMSG%5D.pdf. Client-/Serverrollen-/Policy-Anwendbarkeit für Skype for Business 2019 Windows Full Client bleibt OPEN.

## Referenzgraph / Editionsdisziplin

- 54 neue gerichtete explizite §1.2.1-Kanten im getrennten fourth-ring-normative-reference-ledger.json; Kategorien: {"MS_MC":28,"RFC":11,"EXTERNAL":15}; 33 verschiedene rohe Zielbezeichner innerhalb dieser vier Quellen. Pro Kante stehen raw_label, normalisiertes Ziel, konkrete Quelledition, Original-PDF-URL, PDF-Seite, §, bibliografisch zitierte RFC/W3C-Edition (soweit exakt benannt) und SHA-256=NULL.
- 11 informativ zitierte §1.2.2-Ziele nur in getrennten Objekten gespeichert. Normativ ist keine automatische tatsächliche Full-Client-Featurepflicht.
- Explizite historische Fremdstandards: RFC 2119 (1997), RFC 2616 (1999), RFC 2818 (2000), RFC 3066 (2001), SOAP 1.1 (2000), WSDL 1.1 (2001), WS-I Basic Profile 1.0 (2004), XML Namespaces Third Edition (2009) und XML Schema Part 1+2 First Edition (2001). Keine stillschweigende Ersetzung durch neuere Normeditionen.
- Neue Rohzielnamen noch NICHT gegen die komplette komprimierte Drittstufenbibliografie reconciliiert. Die erste-bis-dritte-Ring-Rohunion 581 bleibt nur als historischer Wert. Kein erfundener globaler Viert-Ring-Union-Zähler.
- Ring-1 Kanten 464, Ring-2 246, Ring-3 explizit 827, die vier AD-Schema-§1.1-Listen mit 67 UNCLASSIFIED bleiben unverändert. Ring-4 10 + 54 = 64; 1 + 4 = 5 von 86 Referenzbibliografien geprüft, 81 offen. Vollbinary-SHA256 neu berechnet: 0.

## Wiederaufnahmepunkt

Konkrete fehlende vier Original-PDF-Dateien, falls die offiziellen Downloads weiter nicht zugänglich sind: [MS-FSCC].pdf Rev 60.0 (2025-11-21), [MS-SMB2].pdf Rev 88.0 (2026-09-28), [MS-GPOL].pdf Rev 39.0 (2024-04-23), [MS-TLSP].pdf Rev 18.0 (2026-09-28). Jeweils Original-Edition, reale §1.2.1-/§1.2.2-Seiten, Prüfsumme, neue Zielnamen und Produktrollen prüfen. Danach MS-RPCH, MS-OXWSFOLD und MS-OXWSATT. Original-PDFs verbleiben beim Nutzer; keine PDFs in Git. Phase-1-Gates 0/12 PASS, keine Implementierung, Live-Tests, Merge nach main, Release oder bezahlte CI-Runner.
