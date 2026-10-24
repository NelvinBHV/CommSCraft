# P1-03 – Originalquellen-Audit: Exchange-Extensions/LDAP/Sharing, Windows-MSRP, MSMQ, CFB/COM und DTCLU

**Datum:** 24.10.2026 · **Arbeitsbranch:** `draft/protocol-spec-analysis-2026-10-23` · **Dokumentationsstatus:** Quelle originalgeprüft, keine Protokollimplementierung.

Fünf vollständige Original-PDFs wurden aus dem nutzereigenen `Exchange_Protocols.zip`, fünf aus `Windows_Protocols.tar.xz` extrahiert. Für jede originale Binärdatei wurden vollständiger SHA-256, Edition `vYYYYMMDD`, Revision, Original-PDF-Seitenzahl, getrennter `§1.2.1 Normative References`- und `§1.2.2 Informative References`-Abschnitt ermittelt. Die Seiten mit Referenzüberschriften wurden gerendert und visuell kontrolliert. Wiederholte `[MS-ID] - vYYYYMMDD`-PDF-Fußzeilen wurden nicht als Bibliografieeinträge gezählt. **Alle Binär-PDFs verbleiben ausschließlich beim Nutzer; Git enthält nur Quellennachweise.**

## Editions- und Originalseitenbelege

| Originalquelle | Revision / Edition | PDF-Seiten | Normativ §1.2.1 (Originalseiten) | Informativ §1.2.2 (Originalseiten) | SHA-256 der vollständigen PDF |
|---|---|---:|---|---|---|
| MS-OXCEXT | **8.0** / `v20250520` | 63 | **17** (8) | **3** (8, 9) | `621a04766324faaedd585a73bb12dc6a677746b9ea069b821758f9028336a9e9` |
| MS-OXLDAP | **16.0** / `v20250520` | 25 | **14** (7) | **1** (8) | `10914e84a63a8150ab669156d9253768f170244ba6c69e5116f9296e2b3bef3a` |
| MS-OXMSG | **18.0** / `v20250520` | 38 | **6** (8) | **3** (8) | `662ba0fe02c56c8f2815df4826b8f4f43775638924856f4a0a43a0d583074167` |
| MS-OXSHARE | **10.0** / `v20250520` | 37 | **10** (7) | **3** (8) | `63853b71b2a474d314029b035b471e533e03464c0abecad0e1e1573619eb2c6d` |
| MS-OXSHRMSG | **14.0** / `v20250520` | 21 | **8** (6) | **1** (6) | `f53b82f03377e828c80ff72d19b6e45f7cac6078b72532d5a59910ff694717b1` |
| MS-MSRP | **10.3** / `v20170601` | 43 | **13** (7, 8) | **2** (8) | `ef57d7f2573f73e0b9798fad6912aad0ca353f5f641695c98a1cdf5e1db278e0` |
| MS-MQMP | **31.0** / `v20240423` | 134 | **20** (11, 12) | **3** (12) | `4968c56195eb5e704107157f415e356e14c4799a4092ceed1830c77c5e40d9dc` |
| MS-CFB | **12.0** / `v20240423` | 46 | **4** (9) | **4** (9) | `75427bf664fe863c881f214a6a35fe9312c9610cfd7776d3131269120a74f88d` |
| MS-COM | **12.0** / `v20240423` | 87 | **9** (12) | **1** (12) | `25c85e0f6dc79bf1692ca005ff5f39e951b0b1e2ced86b51d4b99bf6528d4adf` |
| MS-DTCLU | **17.0** / `v20240423` | 199 | **5** (16) | **0** (keine) | `3debf8b0bdf7c7d956f3738aefdf2c880bec92976ba6ae6585a2bd39201ed1f4` |
| **GESAMT** | 10 Originale | | **106** | **21** | 10 vollständige SHA-256 |

**Maschinenlesbare Einzelreferenzen:** [Originalaudit-JSON](fifth-stage-exchange-sharing-msrp-mqmp-com-audit-2026-10-24.json) enthält je Literaturangabe Referenztyp, exaktes Ziel-Rohlabel und PDF-Fundseite. [Nachgelagerter normativer Kantenledger](fifth-ring-ms-swn-normative-ledger.json) enthält die gerichteten Normverweise, Quelledition, Revision, vollständigen PDF-Dateihash, Originalseitenzahl und Archivmitglied; separat informative Verweise werden **nicht** zu normativen Kanten umklassifiziert. Alle zitierten **historischen Zieloriginaleditionen** bleiben `UNRECONCILED`.

## Wesentliche verifizierte Unterschiede

1. **Exchange-Extended Properties:** `MS-OXCEXT v20250520` besitzt **17 Normquellen** (§1.2.1, Original-S.8), darunter `MS-OWEMXML`, `MS-OXWSCEXT`, `MS-OXWSCORE`, `XMLSCHEMA1/2` und `XMLSCHEMA2/2`; die drei Quellen `MS-OWEXML` (S.8), `MS-OXPROTO` (S.8) und `MSDN-JavaScriptApiOffice` (S.9) sind dagegen **nur informativ**. `MS-OWEMXML` taucht damit als weiteres ungeprüftes transitives MS-Rohziel auf. Moderne Webservices-/JavaScript-Dokumentationsbezüge begründen noch keine nachgewiesene Skype-for-Business-2019-Full-Client-Funktion.
2. **Exchange LDAP:** `MS-OXLDAP` nennt vierzehn Normquellen auf Original-S.7, darunter `RFC1274`, `RFC2696`, `RFC2798`, `RFC2891`, `RFC4511`, `RFC4512`, `RFC4519`, `RFC4523`, `RFC4524`; `MS-OXPROTO` ist ausschließlich informativ auf S.8. Der Auflösungsgrad historischer RFC-Versionen bleibt offen.
3. **Exchange MSG und CFB:** `MS-OXMSG → MS-CFB` steht normativ auf PDF-S.8. Das nun separat geprüfte **`MS-CFB v20240423`** führt selbst `MS-DTYP`, `RFC2119`, `UNICODE3.0.1` und `UNICODE5.0.0` **normativ** (S.9), aber `MS-OLEDS`, `MS-OLEPS`, `MSDN-SS`, `MSDN-STGMC` **informativ** (S.9). Die beiden ausdrücklich unterschiedlichen Unicode-Ausgabe-Rohlabels dürfen nicht als identische editionslose Standardausgabe zusammengeführt werden.
4. **Sharing:** `MS-OXSHARE` zitiert `MS-OXCFOLD`, `MS-OXCMSG`, `MS-OXCPERM`, `MS-OXCPRPT`, `MS-OXOSFLD` normativ (S.7), aber `MS-OXCROPS`, `MS-OXODLGT` und `MS-OXPROTO` sind lediglich informativ (S.8). `MS-OXSHRMSG` nennt `MS-OXWSMSHR`, `RFC2445`, `XMLENC`, `XMLSCHEMA1`, `XMLSCHEMA2` und `XML` normativ auf S.6; nur `MS-OXPROTO` informativ. Originalausgaben der referenzierten XML-/W3C-Normen sind noch nicht abgeglichen.
5. **Historischer Windows-MSRP-Beleg:** Die hier vorliegende vollständige Originalausgabe **`MS-MSRP v20170601 Revision 10.3`** ist datiert auf 2017, also nicht dieselbe Zeitebene wie die 2025er Exchange-Originale. Sie führt auf Original-S.7–8 insgesamt 13 Normquellen, darunter `MS-CIFS`, `MS-MAIL`, `MS-NBTE`, `MS-SMB`, `RFC1001`, `RFC1002`, `RFC768`; nur `MSKB-330904` und `PIPE` sind informativ. Aus der Kennung `MS-MSRP` wird hier **keine Behauptung über Skype-SIP-MSRP-Messagingpflichten** abgeleitet.
6. **MSMQ-Message-Manager-Abhängigkeiten:** `MS-MQMP` listet **20 normative** Literaturquellen auf Original-S.11–12, u. a. `MS-MQDMPR`, `MS-MQDS`, `MS-MQQB`, `MS-MQQP`, `MS-MQRR`; `MS-MQDSSM`, `MS-MQOD` und `MSDN-MQEIC` bleiben dagegen ausdrücklich **informativ** (S.12). Selbst andere normative Referenzrichtungen heben `MS-MQMP → MS-MQDSSM` nicht zu normativ an.
7. **COM-Kette / OleTx:** `MS-COM` nennt `MS-DCOM`, `MS-DTCO`, `MS-NRPC`, `MS-OAUT` und `MS-RPCE` normativ auf S.12, `MSDN-DTC` nur informativ. `MS-DTCLU` führt `IBM-LU62Guide`, `MS-CMPO`, `MS-CMP`, `MS-DTCO` und `RFC2119` normativ auf S.16; sein `§1.2.2`-Abschnitt hat **keine** informativen Literaturquellen. Das ist keine Aussage über notwendige COM-/OleTx-Anwendungsrollen des SfB-Windows-Clients.

## Vollständige Aktualisierung der zwei MS-/MC-Quellenarbeitslisten

Die ursprüngliche [88er-Viertstufenzielqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält jetzt **73** Quellen mit originalgeprüfter §1.2.1-Normbibliografie, **einen** MS-ADLS-Schema-§1.1-Sonderfall mit 13 **UNCLASSIFIED** allgemeine Literaturangaben und **14** noch nicht originale reguläre Quellen. In diesem Batch wurden die fünf Exchange-Originale und `MS-MSRP` daraus auditiert.

Die separate [transitive Quellziel-Liste](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) ist **neu aus sämtlichen 1.228 nachgelagerten gerichteten Normkanten** aufgebaut; sie entfernt bereits geprüfte Quellkennungen der früheren Ringe, sämtliche 88 anfängliche Kennungen sowie schon downstream-originalauditierten Quellen. Die vier früher transitiv offenen `MS-MQMP`, `MS-CFB`, `MS-COM` und `MS-DTCLU` sind neu auditiert; hinzu kommt die rohe Kennung `MS-OWEMXML`. Daher verbleiben **13** zusätzliche wörtliche ungeprüfte MS-/MC-Kennungen.

**Kombinierter Bestand: 14 + 13 = 27 offene wörtliche MS-/MC-Quellrohlabels.** Diese sind **keine editionsnormalisierten Originalidentitäten**, keine als shortest-path-fifth-ring-BFS bewiesene Ebene und **kein normativer Fixpunkt**. Neue bibliografische Audits können zusätzliche Ziele hervorbringen.

## Reproduzierbare Zähler

| Abgrenzung | Ergebnis |
|---|---|
| Normative Kanten Ringe 1–3 | **464 / 246 / 827**, unverändert |
| Vierte ursprüngliche Quellenreferenzebene | **86/86 Quellenidentitäten klassifiziert**, davon 85 explizite §1.2.1-Bibliografien + ein MS-ADLS-§1.1-Schemafall; **1.147** normative Kanten unverändert |
| Nachgelagerte normativ-originalaudierte Quellen | **93** vollständige §1.2.1-Bibliografien, **1.228** normative Kanten (**1.122 + 106**) und **304** informative Belege (**283 + 21**) |
| Anfänglich 88 MS-/MC-Rohziele aus Ebene 4 | **73** Originalnormbibliografien auditiert, 1 Schema-Sonderfall klassifiziert, **14** reguläre Originalquellen offen |
| Separat neu berechnete transitive MS-/MC-Rohzielqueue | **13** offen |
| Kombiniert offene wörtliche rohe MS-/MC-Quellkennungen | **27** |
| Unklassifizierte generische §1.1-Schema-Zitate | **67** ältere Drittstufen- und **13** MS-ADLS-Nennungen bleiben **UNCLASSIFIED** |
| Historische MS-DMRP-Ausgabe | Offizielles `[MS-DMRP].pdf` v20170601 Revision 8.2 weiterhin nicht original-verifiziert; Microsoft-Learn-Gesamtexport von 2026 bleibt eine sekundäre Webquellenfassung |
| Formelle Phase 1 | **0/12 Abschlussgates PASS** |

**Nächster sinnvoller Batch:** Verfügbare Anfangsquellen wie `MS-PAR`, `MS-RA`, `MS-RMPR`, `MS-WSSHP` und `MS-XWDCAL` sowie zusätzliche transitive Quellen `MS-EVEN6`, `MS-GPNAP`, `MS-MQSD`, `MS-WDV`, `MS-OWEMXML` (nur bei tatsächlich verfügbarem Original). **Gezielter fehlender Originalbedarf:** `MS-DMRP` v20170601 Rev.8.2, `MS-ODATAJSON`, `MS-SQLRS` und `MS-OFFCRYPTO` waren in den vier vorhandenen Originalarchiven nicht auffindbar; nur falls genau diese Quellen für die jeweilige nächste Prüfung erforderlich werden, ihr originales PDF vom Nutzer anfordern.

**Verbindliche Sperren:** Keine Protokollimplementierung, keine Live-Netzwerk-/Interop-Tests, keine Nutzung kostenpflichtiger CI, keine TLS-/Auth-Sicherheitsabsenkung, kein Merge nach `main`, Tag oder Release. Die vollständigen PDF-Binärdateien verbleiben beim Nutzer.
