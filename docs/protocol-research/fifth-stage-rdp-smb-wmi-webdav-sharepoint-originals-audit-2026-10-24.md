# P1-03 – Originalaudit: RDP, SMB/SMB Direct, WMI, WebDAV-ACL, SharePoint Meetings und FSSHTTP

**Originalprüfdatum:** 24.10.2026 · **Arbeitsbranch:** `draft/protocol-spec-analysis-2026-10-23` · **Scope:** originalgebundene Bibliografie- und Normreferenzforschung, keine Protokollimplementierung.

Sechs Quellen aus der ursprünglich aus viertstufigen normativen MS-/MC-Zitaten abgeleiteten 88er-Rohzielqueue wurden originalauditiert: `MS-PCCRC`, `MS-RDPEGT`, `MS-RSVD`, `MS-SMBD`, `MS-WMI`, `MS-XWDVSEC`. Vier weitere Originalquellen stammen aus den erst nachgelagert entdeckten transitiven Zielen: `MS-MEETS`, `MS-WDVSE`, `MS-FSSHTTP`, `MS-WSSFO2`.

Die **zehn kompletten Microsoft-Original-PDF-Binärdateien** stammen aus `Windows_Protocols.tar.xz`, `Exchange_Protocols.zip` oder `SharePointProtocols.zip`; jede wurde mit vollständigem SHA-256, Original-Edition `vYYYYMMDD`, Revision, Gesamtseitenzahl und genau abgegrenztem `§1.2.1 Normative References` sowie `§1.2.2 Informative References` geprüft. Beide Literaturstartseiten wurden gerendert und visuell begutachtet. Wiederholte PDF-Fußzeilen `[MS-ID] - vYYYYMMDD` sind ausgeschlossen. **Original-PDF-Binärdateien verbleiben beim Nutzer und werden nicht in Git gespeichert.**

## Quelleneditionsnachweise und Originalseiten

| Original / Originalarchiv | Edition und Revision | PDF-Seiten | §1.2.1 normativ: Anzahl (PDF-S.) | §1.2.2 informativ: Anzahl (PDF-S.) | Vollständiger Original-PDF-SHA-256 |
|---|---|---:|---|---|---|
| `MS-PCCRC` / `Windows_Protocols.tar.xz` | `v20240423` / **Rev. 11.0** | 30 | **9** (7) | **2** (8) | `a2107ddf2ce7fb0ab86f0b5f207ec9715755b61149810b6e2a57dfd8b3c7a030` |
| `MS-RDPEGT` / `Windows_Protocols.tar.xz` | `v20240423` / **Rev. 11.0** | 23 | **6** (5, 6) | **0** (keine) | `6a9a76313dc38bad751222365b95cf51411fb833323804bf95952b6cc1cb3f28` |
| `MS-RSVD` / `Windows_Protocols.tar.xz` | `v20240916` / **Rev. 17.0** | 106 | **6** (8) | **4** (8) | `c2950832ecf2e58d426ee9998e297ddec5f6223fd070c87ef9a13afea0152e23` |
| `MS-SMBD` / `Windows_Protocols.tar.xz` | `v20250811` / **Rev. 16.0** | 39 | **2** (6) | **12** (6) | `e1bea41e4738866d536687c4293acd8ac74f19abc66e5f715d0741f2091a50e2` |
| `MS-WMI` / `Windows_Protocols.tar.xz` | `v20240423` / **Rev. 32.0** | 234 | **14** (13, 14) | **5** (14) | `0b76963cd4c456ae6de0e1500c68fbfc742b252a81f4d184dfbb5cca0e72fe9d` |
| `MS-XWDVSEC` / `Exchange_Protocols.zip` | `v20170919` / **Rev. 6.1** | 35 | **14** (8) | **1** (8) | `b92363affc30fbfc18b6f58655e38010dd2df4349d069512f50be83129f4f72b` |
| `MS-MEETS` / `SharePointProtocols.zip` | `v20240820` / **Rev. 13.0** | 63 | **10** (9) | **2** (9) | `ea64694ee8737e8b5e31b94aeab38d1e3b30d184d470f6edf30c8f6104f8b202` |
| `MS-WDVSE` / `Windows_Protocols.tar.xz` | `v20240916` / **Rev. 14.0** | 27 | **7** (8) | **4** (8, 9) | `62431916dcca4fa08c022d63c2211b16e1202d5b9491a54d7bf96ec7ba6b6d4e` |
| `MS-FSSHTTP` / `SharePointProtocols.zip` | `v20250218` / **Rev. 17.2** | 179 | **19** (12, 13) | **3** (13) | `839b683df6e6a387f035d6d1a108d390a9745bbbd607b23b6c9547b17b8b2913` |
| `MS-WSSFO2` / `SharePointProtocols.zip` | `v20171212` / **Rev. 9.3** | 406 | **12** (26) | **3** (26) | `4eb96a49d6b8f3ae16a6ac3f2eefde17302d773df06af6d40cb7e1a448701ecf` |
| **SUMME** | 10 vollständige Originaleditionen | | **99** | **36** | 10 Vollfile-Hashes |

Das [maschinenlesbare Originalaudit-JSON](fifth-stage-rdp-smb-wmi-webdav-sharepoint-originals-audit-2026-10-24.json) erhält jedes tatsächliche Norm-/Informativ-Quellrohlabel und die zugehörige Original-PDF-Seite. Der [nachgelagerte normative Kantenledger](fifth-ring-ms-swn-normative-ledger.json) verzeichnet je gerichteter Normkante Quelle, Ziel, Literaturklasse, Fundseite, Edition, Revision, Archivmitglied, Originaldateibytes und Vollfile-SHA-256. **Historische Ausgaben der zitierten *Ziel*-Dokumente sind separat weiterhin `UNRECONCILED`.**

## Quellentreue Befunde

1. **RDP-Remoting und neue transitive Ziele:** `MS-PCCRC v20240423 Rev.11.0` enthält auf Original-PDF-S.7 **neun normative** Literaturangaben, darunter `MS-PCCRD`, `MS-PCCRR`, `MS-PCCRTP`, `MS-PCHC`, `RFC2104` und `RFC2119`; `MC-BUP` und `MSDN-BITS` sind **nur informativ** (S.8). Die vier erstmals belegten MS-Ziele bleiben getrennt in der transitiven Rohziel-Queue – ohne Beleg für eine verpflichtende Skype-Client-RDP-Rolle.
2. **RemoteFX-Transport:** `MS-RDPEGT v20240423 Rev.11.0` hat auf Original-S.5–6 **sechs normative** Verweise, darunter `MS-RDPBCGR`, `MS-RDPEDYC`, `MSDN-WindowsGDI` und `RFC2119`. `§1.2.2` enthält **keine** informativen Literaturangaben. Die §1.2.1-Fundstellen auf PDF-S.5–6 sind bewusste Haupttext-/Originalseiten, keine Inhaltsverzeichnis-Fundstellen.
3. **Shared Virtual Disk und SMB Direct:** `MS-RSVD v20240916 Rev.17.0` zitiert `MS-FSCC`, `MS-SMB2`, `MS-SRVS`, `RFC2119` und `UNICODE` normativ (PDF-S.8), aber zwei CVE-/KB-Nennungen sowie `SPC-3` informativ. `MS-SMBD v20250811 Rev.16.0` hat **nur zwei normative** bibliografische Ziele (`IANAPORT`, `RFC2119`, S.6), während `MS-SMB2`, `MS-ERREF`, vier weitere RDMA-RFCs, `ROCE`, `IBARCH` usw. zur **zwölfteiligen informativen** Literatur gehören. `MS-SMBD → MS-SMB2` darf folglich aus diesem Original **nicht** als Normkante im Graph erscheinen.
4. **WMI:** `MS-WMI v20240423 Rev.32.0` dokumentiert **14 normative** Referenzen auf Original-S.13–14, darunter `MS-DCOM`, `MS-OAUT`, `MS-RPCE`, `MS-WMIO`, `DMTF-DSP0004` und `IEEE754`; fünf MSDN-/WMI-Nachschlagequellen stehen in §1.2.2 (S.14) und bleiben informativ. `MS-WMIO` tritt neu in der ergänzenden transitive Rohzielqueue auf.
5. **Ältere Exchange-WebDAV-Sicherheitsedition:** `MS-XWDVSEC v20170919 Rev.6.1` listet vierzehn Normreferenzen auf S.8, u. a. `MS-ADA1`, `MS-ADA3`, `MS-OXCFOLD`, `MS-SAMR`, `RFC2068`, `RFC2518`, `XMLSCHEMA1/2` und `XMLSCHEMA2/2`; `MS-OXPROTO` nur informativ. Die 2017er Quellversion darf nicht ohne Editionsabgleich einem modernen Exchange- oder Skype-Client-Profil zugerechnet werden.
6. **SharePoint-Meetings:** `MS-MEETS v20240820 Rev.13.0` führt **zehn normative** Quellen auf S.9, darunter `RFC2445`, `SOAP1.2-1/2007`, `WSDL`, `XMLSCHEMA1`, `XMLSCHEMA2`; `RFC2818` und `SOAP1.2-2/2007` sind ausdrücklich **informativ**. Ähnlich geschriebene historische SOAP-Ausgaben dürfen nicht ohne Beleg zusammengeführt werden.
7. **WebDAV Search:** `MS-WDVSE v20240916 Rev.14.0` führt **sieben ausschließlich externe normative RFC-Zitate** auf S.8, darunter `RFC2068`, `RFC2246`, `RFC2291` und `RFC4918`. `RFC2518`, `RFC2818`, `MSASP` und `MSDN-FP` sind **informativ** (S.8–9). Keine MS-/MC-Normziele aus dieser Quelle bedeutet **keinen** vollständigen RFC-Fixpunkt.
8. **SharePoint FSSHTTP:** `MS-FSSHTTP v20250218 Rev.17.2` besitzt **19 normative** §1.2.1-Verweise auf S.12–13: `MS-FPSE`, `MS-FSSHTTPB`, `MS-LISTSWS`, `MS-SHDACCWS`, `MS-VERSS`, `RFC2119`, `SOAP1.2-MTOM`, `XMLINFOSET`, `XOP10` usw. **Nur informativ** sind `MS-OCPROTO`, `SOAP1.2-1/2007` und `SOAP1.2-2/2007`. Diese neuen SharePoint-MS-Ziele sind **offene transitive Kandidaten**, keine nachgewiesenen Pflichtfunktionen des Skype-for-Business-2019-Windows-Full-Clients.
9. **Historische SharePoint-Dateiformate:** `MS-WSSFO2` liegt als **v20171212 Rev.9.3** vor, ein Original mit **406 Seiten**. Sein §1.2.1 listet zwölf normative Ziele auf S.26, darunter `MC-FPSEWM`, `MS-FPSE`, `MS-TDS`, `RFC1950`, `RFC2660`, `TSQL-Ref`. `MS-WSSO`, `RFC2518`, `RFC2616` sind **nur informativ** auf S.26. Diese ältere SharePoint-Spezifikation gehört nicht ungeprüft zur gleichen historischen Zielausgabe wie FSSHTTP von 2025.

## Aktualisierte und strikt getrennte Quellarbeitslisten

Die [ursprüngliche viertringbasierte MS-/MC-Workqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält **88** aus der vierten §1.2.1-Normquellebene rohe Zielbezeichnungen nach Ausschluss in früheren Ringen geprüfter Quellenidentitäten. Von diesen sind nun **84 Originalnormbibliografien vollständig auditiert**, **eine** `MS-ADLS`-§1.1-Schema-Bibliografie separat klassifiziert (ihre 13 Angaben bleiben **UNCLASSIFIED**); genau **drei reguläre Originalquellen** sind hier noch offen: `MS-DMRP`, `MS-ODATAJSON`, `MS-SQLRS`. Ihre historischen Original-PDFs wurden in den vorhandenen vier Archiven nicht nachgewiesen; der Microsoft-Learn-2026-Webexport zu `MS-DMRP` ist **nicht identisch** mit dem benötigten offiziellen 2017er Original v20170601, Rev.8.2.

Die [zusätzliche transitive MS-/MC-Rohzielqueue](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) wird **neu aus allen 1.472 ausdrücklich normativen gerichteten Kanten der nachgelagerten Originalbibliografien** berechnet. Dabei werden Quellenidentitäten der ersten vier Ringe, die **gesamte ursprüngliche 88er-Liste** sowie alle bereits geprüften nachgelagerten Quellen ausgeschlossen. Vier zuvor offene transitive Quellen `MS-MEETS`, `MS-WDVSE`, `MS-FSSHTTP`, `MS-WSSFO2` sind nun geprüft, während beispielsweise `MS-PCCRD`, `MS-PCCRR`, `MS-PCCRTP`, `MS-PCHC`, `MS-FSSHTTPB`, `MS-LISTSWS`, `MS-SHDACCWS`, `MS-VERSS`, `MS-WMIO`, `MS-TDS` und `MC-FPSEWM` als neue ungeprüfte wörtliche Normzielkennungen auftreten. Der aktuelle Bestand dieser separaten Liste beträgt **28** Quellenrohlabels.

**Kombiniert: 3 + 28 = 31 unterschiedliche offene rohe MS-/MC-Quellkennungen.** Diese wörtlichen Labels sind **nicht** historisch-editionskanonisiert, nicht als kürzeste fünfte BFS-Stufe nachgewiesen, **kein vollständiger normativer Fixpunkt** und erst recht keine belegte verpflichtende Client-Featureliste.

## Kontrollierte Gesamtbilanz

| Forschungsbereich | Stand nach Originalaudit |
|---|---|
| Normative Bibliografiekanten Ringe 1 / 2 / 3 | **464 / 246 / 827**, unverändert |
| Vierte Originalquellen-Identitätsebene | **86/86** klassifiziert, davon 85 echte §1.2.1-Bibliografien plus ein MS-ADLS-§1.1-Schema-Sonderfall; **1.147** normative Kanten unverändert |
| Nachgelagerte Originalquellenbibliografien | **113** |
| Nachgelagerte ausdrücklich normative gerichtete Kanten | **1.472** (**1.373 + 99**) |
| Nachgelagerte strikt separate informative Literaturnennungen | **420** (**384 + 36**) |
| Ursprüngliche viertringbasierte 88er-Rohzielqueue | 84 originalauditiert, 1 MS-ADLS-Schema-Sonderfall, **3** reguläre ungeprüfte |
| Zusätzliche transitive rohe MS-/MC-Zielquelle-Kennungen | **28** ungeprüft |
| Kombiniert rohe MS-/MC-Quellkennungen offen | **31** |
| Allgemeine Schema-§1.1-Literatur | 67 frühere Drittstufen-Angaben und 13 MS-ADLS-Angaben weiterhin **UNCLASSIFIED** |
| Offizielles MS-DMRP-Original | `v20170601` Revision 8.2 PDF nicht verifiziert; der vorliegende 2026-Learn-Webexport wird weiterhin nicht als Original gezählt |
| Formeller Phase-1-Abschluss | **0/12 Gates PASS** |

**Nächste tatsächlich zu prüfende Gruppe:** mindestens aus den 28 zusätzlichen transitive Rohlabels Originale wie `MS-PCCRD`, `MS-PCCRR`, `MS-PCCRTP`, `MS-PCHC`, `MS-FSSHTTPB`, `MS-LISTSWS`, `MS-SHDACCWS`, `MS-VERSS`, `MS-WMIO`, `MC-FPSEWM`, nach Prüfung der Archivverfügbarkeit. Der reguläre 88er-Rohzielpool ist **erst dann vollständig**, wenn die drei genannten fehlenden Original-PDFs gesondert vorliegen. Historische Zielausgaben, externe Standardsrekursion (RFC/IETF/ISO/ITU/W3C), echte Skype-2019-Windows-Full-Client-Build-/Rollen-/Policy-/Featurepflichten und normativer Graphfixpunkt bleiben OPEN.

**Verbindliche Sperren bis zum formellen Abschluss:** keine Protokollimplementierung, keine Live-Server-/Interop-Tests, keine bezahlten CI-Runner, keine Abschwächung der TLS-/Authentisierungssicherheit, kein Merge nach `main`, Tag oder Release; keine Original-PDF-Binärdateien im Git-Repository.
