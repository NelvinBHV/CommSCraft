# Phase 1 – Rekursive normative Querverweise, zweiter Microsoft-Ring (Stand 2026-10-23)

**Status: UNVOLLSTÄNDIG.** Das bestehende MS-OCSPROT-Basisregister enthält weiterhin genau **460** normative Erstreferenz-Kanten aus **46/46** Hauptlisten. Die neue, **eigenständige** [zweite Ebene](transitive-reference-ledger.json) ergänzt **246** direkt von **12** zusätzlich benannten Microsoft-Spezifikationen ausgehende Referenzen (nicht als bereits in den 460 enthalten markieren). Davon wurden **11** §1.2.1-Listen direkt im HTML der offiziellen Microsoft-Open-Specifications-Seiten eingesehen; bei **MS-RDPEMC** nur ein offizieller Suchauszug, da der Seitenabruf fehlschlug. Keine der zwölf neu untersuchten Listen ist bisher gegen den **passenden veröffentlichten PDF-/DOCX-Originalstand** revisionsgenau abgeglichen.

## Quellen, Veröffentlichungen und Prüfstatus

**Revision/Datum** stammen aus dem bisher angelegten [Metadatenkatalog](transitive-microsoft-inventory.json), **HTML-Update-Datum** von der jeweiligen verlinkten offiziellen §1.2.1-Seite. **Beides darf nicht zu einer belegten identischen Textausgabe verschmolzen werden**; Windows-Spezifikationen wurden vielfach später aktualisiert als der klassische SfB-2019-Client.

| Spezifikation | Zuletzt veröffentlichte Revision (Metadaten) | Tatsächlich eingesehener offizieller Normabschnitt | HTML zuletzt aktualisiert | Erfasste Kanten | Evidenz |
| --- | --- | --- | --- | ---: | --- |
| MS-SIP | 8.0 (2017-06-01) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sip/241bd813-787f-4b49-a5c7-e28b1270f9ea) | 2024-10-30 | 16 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-H26XPF | 1.3 (2017-06-01) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-h26xpf/08c8711d-9d6e-44da-93a6-8712344199c3) | 2024-10-30 | 4 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-SDP | 1.0 (2017-06-01) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sdp/41a2961e-b14c-4d50-8637-84bc4b7eaedb) | 2024-10-30 | 7 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-KILE | 49.0 (2026-09-28) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-kile/b7219d26-dbc7-4f3a-adfe-dcc31f90d92a) | 2026-09-25 | 43 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-NLMP | 37.0 (2026-03-30) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nlmp/3c88d453-b53a-4004-9a66-2ded619dcd27) | nicht gesichert | 15 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-RDPBCGR | 62.0 (2026-03-09) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/a75b2f6b-a048-4273-95dc-2d246b2bfec8) | 2024-04-23 | 37 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-RDPEMC | 16.0 (2024-04-23) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpemc/90a92b56-fae6-4af8-9592-4c9bb0918582) | 2024-04-23 | 4 | Suchauszug, noch unvollständig verifiziert |
| MS-WCCE | 55.0 (2026-08-24) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wcce/6b0fd42e-d685-4169-a6dd-a230ae244e74) | 2024-04-23 | 61 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-WSTEP | 15.0 (2024-04-23) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wstep/9d3c9a95-a923-4ad7-9c13-60485acf5cfb) | 2024-04-23 | 16 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-WSPOL | 8.0 (2019-03-13) | [§1.2.1](https://learn.microsoft.com/kk-kz/openspecs/windows_protocols/ms-wspol/a3a07257-01be-47ae-9a4b-2cc11957ea09) | 2024-10-30 | 20 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-OAUTH2EX | 8.0 (2025-05-20) | [§1.2.1](https://learn.microsoft.com/de-de/openspecs/sharepoint_protocols/ms-oauth2ex/7d87259b-2577-4355-a64a-dacb3fc2f69c) | 2024-08-20 | 7 | HTML sichtbar, PDF-Ausgabe ungeprüft |
| MS-OXWUMS | 16.0 (2025-05-20) | [§1.2.1](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwums/a93a66e2-a6d9-47d5-bcb1-4aceed0a5fdc) | 2024-04-16 | 16 | HTML sichtbar, PDF-Ausgabe ungeprüft |

**Anmerkung:** Der Microsoft-Katalogstand aus 2026 darf nicht pauschal als diejenige Version behandelt werden, die zum Windows Full Client 2019 passt. Besonders deutlich ist das bei MS-KILE, MS-NLMP, MS-RDPBCGR und MS-WCCE. Zusätzlich sind einige HTML-Abschnitte selbst älter als ihre zuletzt veröffentlichte Revision. Zu klären ist jeweils die **tatsächlich anzuwendende Edition** samt Server-/OS-Produktverhalten.

## Maschinell abgeglichener neuer Referenzgraph

| Bedeutung | Zahl |
| --- | ---: |
| Dokumente der zweiten Ebene | **12** |
| Sichtbare vollständige HTML-§1.2.1-Listen | **11** |
| Nur im offiziellen Suchauszug vorliegende Listen | **1** |
| Neue gerichtete Kanten (zweite Ebene) | **246** |
| Verschiedene Zielkennungen innerhalb der zweiten Ebene | **183** |
| Davon Microsoft/MC-Kanten | **81** |
| Davon RFC-Kanten | **104** |
| Davon andere Norm-/Draft-Kanten | **61** |
| **Neue** Zielkennungen gegenüber der ersten Ebene | **146** |
| Neue Microsoft-/MC-Ziele | **52** |
| Neue RFC-Ziele | **51** |
| Neue externe Standards/historische Entwürfe | **43** |
| Eindeutige Zielkennungen über beide Ebenen | **345** |

Die neue [offene Zielmenge](transitive-frontier.json) enthält **146** Einzelkennungen mit rückverfolgbarer eingehender Referenz. **Das ist keine vollständige transitive normative Hülle**: weitere Auflösung, Versionsprüfung und belegte Ausschlüsse fehlen. Auch folgt aus einer normativen Referenz in einer breiten **server- oder plattformbezogenen** Spezifikation **nicht**, dass jede referenzierte Funktion im Windows Full Client implementiert werden muss.

## Neu sichtbare technische Abhängigkeitszweige – quellenbelegte Zuordnung, keine Implementierung

1. **SIP/Kerberos/NTLM.** [MS-SIP §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sip/241bd813-787f-4b49-a5c7-e28b1270f9ea) nennt u. a. MS-ADA1–3, MS-ADSC, MS-KILE und MS-NLMP sowie RFC 1964, 3261–3265 und PIDF RFC 3863. [MS-KILE §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-kile/b7219d26-dbc7-4f3a-adfe-dcc31f90d92a) erweitert die Quellkette um AD/KDC/PAC/PKINIT/SPNEGO, RFC 3961/3962/4120/4121/8009 und ältere Kerberos-Drafts. [MS-NLMP §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nlmp/3c88d453-b53a-4004-9a66-2ded619dcd27) referenziert insbesondere MD4/MD5/HMAC sowie Windows-Authentifizierungsdienste. Ob AD-/Server-only-Komponenten eine direkte Clientanforderung sind, bleibt OFFEN.
2. **Videotransport/SDP.** [MS-H26XPF §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-h26xpf/08c8711d-9d6e-44da-93a6-8712344199c3) nennt das bisher nicht erfasste **MS-RTPME** sowie RFC 2032 und RFC 2190. [MS-SDP §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-sdp/41a2961e-b14c-4d50-8637-84bc4b7eaedb) führt zurück zu MS-SIP sowie den historischen RTP/RTCP/SDP-Editionen RFC 3550, 3605, 3611, 4566.
3. **Desktop-/Anwendungsteilung.** [MS-RDPBCGR §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpbcgr/a75b2f6b-a048-4273-95dc-2d246b2bfec8) nennt RDP-Virtual-Channel-/Grafik-/Credential-Specs und u. a. TLS-RFCs, JWS/JWT/OAuth-RFCs und kostenpflichtige ITU-T T.122/123/124/125/128. Dies beweist nur die **RDPBCGR-Abhängigkeiten**; der für SfB-Anwendungsteilung tatsächlich genutzte Subset muss mit MS-RTASPF/MS-CONFAS und Produktverhalten abgeglichen werden.
4. **Zertifikatsregistrierung und Webdienste.** [MS-WCCE §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wcce/6b0fd42e-d685-4169-a6dd-a230ae244e74), [MS-WSTEP §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-wstep/9d3c9a95-a923-4ad7-9c13-60485acf5cfb) und [MS-WSPOL §1.2.1](https://learn.microsoft.com/kk-kz/openspecs/windows_protocols/ms-wspol/a3a07257-01be-47ae-9a4b-2cc11957ea09) erschließen Certificate Templates, PKCS/CMS, SOAP, WS-Trust, WS-Security, .NET framing, historische TLS- und OCSP-Ausgaben. **Keines** der damit genannten Algorithmen wird hier pauschal als zulässige Sicherheitspolitik empfohlen.
5. **OAuth/Exchange.** [MS-OAUTH2EX §1.2.1](https://learn.microsoft.com/de-de/openspecs/sharepoint_protocols/ms-oauth2ex/7d87259b-2577-4355-a64a-dacb3fc2f69c) enthält einen ausdrücklichen **Versionswiderspruch**: OAuth-v2-Draft als **-22** bezeichnet, Ziel-URL endet **-23**. Diesen Konflikt nicht stillschweigend vereinheitlichen. [MS-OXWUMS §1.2.1](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwums/a93a66e2-a6d9-47d5-bcb1-4aceed0a5fdc) nennt fünf weitere Exchange-WS-Dokumente; direkte Windows-Client-Relevanz bleibt zu entscheiden.

## Quellenqualität und Zugriffslücken

- **11 vollständige HTML-§1.2.1-Seiten** sind lesbar, stehen aber nicht als revisionsgebunden heruntergeladene PDFs zur Verfügung. Seiten mit Hinweisen zur erforderlichen Anmeldung lieferten teilweise dennoch Referenztext; das berechtigt nicht zu Aussagen über vollständige geschützte Inhalte.
- **MS-RDPEMC:** offizieller [Suchtreffer zum §1.2.1](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpemc/90a92b56-fae6-4af8-9592-4c9bb0918582) zeigte MS-ERREF, MS-RDPBCGR, MS-RDPEPS und RFC 2119. Vollseitenabruf scheiterte; **nur partieller Suchindex-Evidenzstatus**, keine Behauptung vollständiger Liste.
- Direkte Microsoft-PDF-URLs für MS-ABS und MS-DLX waren im Browser nicht zugänglich; aus der Containerumgebung war ein DNS-/Netzwerkzugriff auf Microsoft-PDF und RFC-Server nicht möglich. **Keine PDF-Datei wurde in dieser Etappe heruntergeladen oder gehasht.**
- Die bestehenden **460** Erststufenkanten aus [normative-reference-ledger.json](normative-reference-ledger.json) und die bereits hochgeladene MS-AVEDGEA-Originaldatei bleiben als getrennter Provenienzstand erhalten.
- **Sicherheitskritischer Editionskonflikt:** RFC 2246, 4346, 5246, 8446 u. a. tauchen nebeneinander in historischen Microsoft-Referenzlisten auf. Die Referenz ist **kein Freibrief**, obsolete TLS-Versionen/Algorithmen zu aktivieren.

## Nächste normativ notwendige Arbeit

1. **Priorität A:** Exakte PDF-/DOCX-Ausgaben der zwölf zweiten-Ebene-Dokumente samt Veröffentlichung/Revision und deren §1.2.1-Referenzlisten verifizieren. Bei MS-RDPEMC vorrangig gesamte aktuelle Referenzseite beschaffen. **11 HTML-Listen ≠ 11 PDF-verifizierte Ausgaben.**
2. **Priorität B:** Neue [52 Microsoft-/MC-Knoten](transitive-frontier.json) nach *direkter Funktion für den klassischen Windows Client* vs. *server-/plattformbezogene, nur indirekte Abhängigkeit* klassifizieren, mit fundierter Ein-/Ausschlussbegründung. Danach relevante Ketten rekursiv weiterverfolgen.
3. **Priorität C:** Die **51** gegenüber der bisherigen ersten Referenzebene neuen RFC-Zielkennungen und die übrigen referenzierten RFCs mit Editor-Metadaten, Errata, ursprünglichen Normverweisen, Updates/Obsoletes und exakten historischen Editionen prüfen.
4. **Priorität D:** Parallel die bisherigen **33 nur als HTML belegten Erststufenlisten** gegen die datierte Microsoft-Veröffentlichung abgleichen; abweichende Versionen und Zugriffsbarrieren dokumentieren.

**Phase 1 insgesamt weiterhin UNVOLLSTÄNDIG.** Kein Code, keine Serververbindungen, keine eigenen Interoperabilitätstests, keine Merges oder Releases.
