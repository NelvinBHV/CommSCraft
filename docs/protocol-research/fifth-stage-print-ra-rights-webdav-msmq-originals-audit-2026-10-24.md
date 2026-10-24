# P1-03 – Originalbibliografien Print, Remote Assistance, Rights Management, SharePoint, Calendar/WebDAV, Eventlog und MSMQ

**Forschungsetappe:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23` · **Nur Quellenforschung**. Diese Etappe verändert keine Protokollimplementierung, keine Netzwerk-/Testumgebung und kein Release.

## Belegmethode und Provenienz

Fünf bislang offene Rohquellen der ursprünglichen Viertstufen-Arbeitsliste (`MS-PAR`, `MS-RA`, `MS-RMPR`, `MS-WSSHP`, `MS-XWDCAL`) und fünf bislang offene transitive Rohquellen (`MS-EVEN6`, `MS-GPNAP`, `MS-MQSD`, `MS-WDV`, `MS-TIPP`) wurden jeweils anhand einer vollständigen Microsoft-Original-PDF aus den bereits vorhandenen nutzereigenen Archiven `Windows_Protocols.tar.xz`, `SharePointProtocols.zip` oder `Exchange_Protocols.zip` untersucht.

Für alle zehn vollständigen PDF-Dateibytes wurden **SHA-256**, Byteanzahl, Microsoft-Edition `vYYYYMMDD`, Quellrevision und Gesamtseitenzahl erfasst. Getrennte **§1.2.1 „Normative References“** und **§1.2.2 „Informative References“** wurden gegen die **wirklichen Original-PDF-Seiten** abgegrenzt und die jeweiligen Einstiegsseiten zusätzlich gerendert und visuell geprüft. Mehrfach wiederholte PDF-Fußzeilen `[MS-ID] - vYYYYMMDD` wurden verworfen; eine explizite Selbstreferenz **innerhalb der eigentlichen normativen Literaturliste** wurde erhalten.

**Original-PDF-Binärdateien bleiben beim Nutzer.** In Git gelangen ausschließlich eigenständige Forschungsberichte und maschinenlesbare Referenzbelege, niemals die Original-PDFs. Zitierte **Zieldokumenteditionen** sind weiterhin `UNRECONCILED`; auch identische Rohkennungen in verschiedenen historischen Quelleditionen werden nicht automatisch als inhaltlich identische Implementierungspflichten behandelt.

## Exakte Originaleditionen und bibliografische Fundstellen

| Originaldokument / Archiv | Edition / Revision | PDF-Gesamtseiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | SHA-256 der *vollständigen* Original-PDF |
|---|---|---:|---|---|---|
| `MS-PAR` / `Windows_Protocols.tar.xz` | `v20250811` / **19.0** | 129 | **8** (13) | **14** (13, 14) | `3f1e313df78044bdbbd8b1c4202a7dc7064dd38708d38d238dfae035e8c0717e` |
| `MS-RA` / `Windows_Protocols.tar.xz` | `v20260113` / **18.0** | 90 | **10** (10) | **12** (11) | `03dd4aebc6c55296df3129f736cb607200f50501c5e7a2699e53e382e32347ea` |
| `MS-RMPR` / `Windows_Protocols.tar.xz` | `v20240423` / **41.0** | 220 | **36** (16, 17, 18) | **5** (18) | `37306e4ae6d5239924cffc8058a6fc7ad85df12323d78094c316c375c05ef060` |
| `MS-WSSHP` / `SharePointProtocols.zip` | `v20240820` / **17.0** | 33 | **11** (8) | **1** (8) | `c36cb2a014173a076b13f087f9136681e3b0120fedb8fbd97a8cec8ad548aff2` |
| `MS-XWDCAL` / `Exchange_Protocols.zip` | `v20170620` / **4.7** | 76 | **25** (11, 12) | **7** (12, 13) | `0beeac415e5d287f4c9d729286180f8452312013f3eee9be7ed1483fff375355` |
| `MS-EVEN6` / `Windows_Protocols.tar.xz` | `v20240423` / **25.0** | 161 | **21** (10, 11) | **26** (11, 12) | `37ba28499c3dc0b37d712ce1199a58f9e97a7e7c3f49ca5d8bcd2a99baefbb94` |
| `MS-GPNAP` / `Windows_Protocols.tar.xz` | `v20170601` / **7.0** | 35 | **16** (8, 9) | **10** (9) | `829ace9e3e4ebdc9bf56e988f17c0c14fed5a49a4dd0cd7d52f16eb633ccb9df` |
| `MS-MQSD` / `Windows_Protocols.tar.xz` | `v20170601` / **13.0** | 29 | **9** (8) | **2** (8) | `a058ed3014e6641957aeb8fc29accc09557e7c222362a3315ebc96201247835d` |
| `MS-WDV` / `Windows_Protocols.tar.xz` | `v20240423` / **21.0** | 34 | **6** (8) | **2** (8) | `5841a14158967422fba507efa7a66a0a3a54295f1b5702799dbc35685d3741a2` |
| `MS-TIPP` / `Windows_Protocols.tar.xz` | `v20240423` / **18.0** | 85 | **3** (10) | **1** (10) | `dfcd196db47592043528a48a26625163930d6dfc91f4901dfdf8c0e73bbe0058` |
| **SUMME** | zehn getrennte Originaleditionen | | **145** | **80** | zehn vollständige Vollfile-Prüfsummen |

[Maschinenlesbare Referenzeinträge](fifth-stage-print-ra-rights-webdav-msmq-originals-audit-2026-10-24.json) enthalten für jede normative oder informative Quellnennung das unveränderte Literatur-Rohlabel, die echte Quell-PDF-Fundseite sowie gemeinsame Edition, Revision, Archivmitglied, vollständige Dateigröße und SHA-256. Der [nachgelagerte gerichtete Normledger](fifth-ring-ms-swn-normative-ledger.json) bewahrt die exakt 145 neuen **normativen** Quellkanten mit denselben Belegfeldern, während die 80 **informativen** Nennungen ausschließlich separat geführt werden.

## Wichtige Einzelbefunde

1. **Normative Selbstzitate wirklich im Literaturabschnitt:** `MS-RMPR v20240423 Rev41.0 §1.2.1 → MS-RMPR` ist eine echte explizite Literaturangabe auf **Original-PDF-S.17**; eine zusätzliche PDF-Fußzeile mit gleicher Kennung wurde ausgeschlossen. `MS-MQSD v20170601 Rev13.0 §1.2.1 → MS-MQSD` steht ebenfalls ausdrücklich auf **Original-S.8** und ist kein extrahierter Footer. Beide sind bibliografische Selbstkanten und bei künftiger Referenzrekursion mit `visited`-Strategie zu behandeln. Bereits früher belegte echte Selbstzitate in `MS-MQRR`, `MS-OXOCNTC` und `MS-CMP` bleiben unverändert erhalten.
2. **WebDAV-Klassen sind nach Quellrichtung verschieden:** `MS-WSSHP v20240820 → MS-WDV` ist **normativ** in §1.2.1 auf S.8; auf derselben Seite sind `MS-FSSHTTP` und `MS-WSSFO2` ebenfalls normativ. `MS-XWDCAL v20170620 → MS-WDV` dagegen steht **nur informativ** in §1.2.2 auf S.12, während `MS-XWDCAL → MS-WDVSE`, `→ MS-WDVME`, `→ MS-MEETS` normativ sind (S.11–12). Deshalb niemals aus einer anderen Quelle auf eine gewünschte Kantenklasse schließen.
3. **SharePoint-Headers-Original hat besondere Abschnittsüberschrift:** `MS-WSSHP v20240820 Rev17.0` verwendet im anschließenden Abschnitt `§1.3 Protocol Overview (Synopsis)` anstatt eines einfachen `§1.3 Overview`. Die Abgrenzung von §1.2.2 endet deshalb ausdrücklich an der tatsächlichen Überschrift. Es gibt **elf** normative und **einen** informativen Bibliografieeintrag: `MC-FPSEWM` ist ausschließlich informativ (Original-S.8). Trotz SharePoint-Provenienz folgt daraus keine automatische Skype-Windows-Full-Client-Pflicht.
4. **Rights Management + Webservices:** `MS-RMPR v20240423 Rev41.0` enthält **36 normative** Literaturangaben auf S.16–18 einschließlich `MS-RMPRS`, `MS-RMSI`, `MS-MWBE`, `MS-MWBF`, `XRML`, `SOAP1.2/1` und `SOAP1.2/2` sowie **5 informative** auf S.18. `MS-ADTS` und `MS-LSAT` werden dort **nur informativ** zitiert. Die 2024er Source-Edition legt weder 2019er Client-Build noch Policy-/Serverrollenumfang fest.
5. **Remote Assistance mit Ausgabe von 2026:** `MS-RA` liegt als vollständige **v20260113 Revision 18.0** vor, mit zehn normativen Referenzen auf PDF-S.10 (`MS-PNRP`, `MS-RAIOP`, `MS-RAI`, `MS-RDPBCGR`, `MS-RDPEGDI`, `MS-RDPEMC` sowie RFCs) und **12 ausschließlich informativen** Verweisen auf S.11. Die 2026er Revision ist nicht automatisch ein Beleg für den Skype-for-Business-2019-Windows-Full-Client-Featureumfang.
6. **Eventlog und CVE-Literatur:** `MS-EVEN6 v20240423` besitzt **21 normative** Quellen auf S.10–11 und **26 informative** auf S.11–12; darunter viele `MSDN-`- und `MSFT-CVE-`-Referenzen. `MS-EVEN6 → MS-GPSI` ist normativ (S.10); `MSFT-CVE-2021-31958` ist hingegen nur informativ (S.12). Sicherheitsdokument-Links sind keine zusätzlich normative Literaturkante.
7. **Network Access Protection (historisches Original):** `MS-GPNAP v20170601 Rev7.0` zitiert `MS-DHCPN`, `MS-GPOL`, `MS-GPREG`, `MS-HCEP`, `MS-PEAP`, `MS-TSGU` sowie verschiedene historische RFCs **normativ** (S.8–9). Die zehn NAP-/IPsec-/DHCP-Dokumentationsverweise unter §1.2.2 auf S.9 sind **informativ**. Die Protokollverweise belegen weder, dass heutige Client-Builds NAP unterstützen, noch eine Freigabe alter Kryptoprofile.
8. **MSMQ-Directory-Service-Historie:** `MS-MQSD v20170601 Rev13.0` referenziert `MS-MQDMPR`, `MS-MQDSSM`, `MS-MQDS`, `MS-MQMP` und sich selbst normativ (S.8). `MS-MQOD` dagegen ist ausschließlich informativ. Die historische Ausgabe von 2017 darf nicht mit neueren MSMQ-Zielausgaben ohne Revisionsbeleg gleichgesetzt werden.
9. **WebDAV-Client-Extensions und OleTx:** `MS-WDV v20240423 Rev21.0` enthält sechs normative **RFC**-Zitate (S.8), keinerlei normative MS-/MC-Kennung; die beiden Nachschlagequellen `MSASP` und `MSDN-STC` sind informativ. `MS-TIPP v20240423 Rev18.0 → MS-DTCO` sowie `→ RFC2119` und `→ RFC2371` stehen normativ (S.10), `RFC2372` nur informativ. Das Ende der Microsoft-Kennungen bei `MS-WDV` ist **kein** Beweis für abgeschlossene externe normative RFC-Rekursion.
10. **Printing-Änderungen:** `MS-PAR v20250811 Rev19.0` nennt `MS-RPRN`, `MS-SPNG`, `MS-RPCE`, `RFC2781` und weitere auf S.13 normativ; die 14 Literaturangaben zu Drucker-/CVE-/MSDN-Kontext sind ausschließlich informativ (S.13–14). Hieraus folgt keine Verpflichtung zur Druckerschnittstellen-Implementierung im Skype-Client.

## Zwei getrennte und erneut vollständig berechnete Quellen-Frontiers

Die ursprüngliche [viertstufenbasierte Zielworkqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält insgesamt **88 neue MS-/MC-Rohlabels** nach Ausschluss bereits in den ersten vier Quellenebenen geprüfter Quellidentitäten. **Fünf** weitere Originalbibliografien aus dieser Arbeitsliste sind jetzt vollständig geprüft. Aktuell: **78** vollständige §1.2.1-Originalbibliografien auditiert, **eine** MS-ADLS-§1.1-Schemaausnahme separat klassifiziert (ihre 13 Literaturangaben bleiben **UNCLASSIFIED**), **9** reguläre Quellen aus dieser ursprünglichen Arbeitsliste noch offen.

Die separate [transitive Rohzielqueue](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) ist **aus allen 1.373 gerichteten normativen downstream-Kanten neu aufgebaut**. Sie entfernt Quellen der früheren Original-Ringe, die vollständige ursprüngliche 88er-Queue und sämtliche inzwischen geprüften nachgelagerten Originalquellen. Fünf frühere transitive Rohlabels (`MS-EVEN6`, `MS-GPNAP`, `MS-MQSD`, `MS-WDV`, `MS-TIPP`) sind originalgeprüft. Zugleich führen die neuen Bibliografien zusätzliche Normziele ein (`MS-DHCPN`, `MS-FSSHTTP`, `MS-HCEP`, `MS-MWBE`, `MS-MWBF`, `MS-PNRP`, `MS-RAIOP`, `MS-RMPRS`, `MS-RMSI`, `MS-WDVME`, `MS-WSSFO2` usw.), deshalb verbleiben nun **21** zusätzlich transitive MS-/MC-Rohkennungen ohne eigenen Originalbibliografie-Audit.

**Kombiniert: 9 + 21 = 30 wörtlich verschiedene offene MS-/MC-Rohlabels.** Das sind keine editionskanonischen Originalidentitäten, keine überprüfte kürzeste fünfte BFS-Stufe und kein transiter vollständiger normativer Fixpunkt. Der offene Pool kann trotz Fortschritt wachsen.

## Kontrollierte Zähler und Wiederaufnahme

| Prüffeld | Status |
|---|---|
| Normative Kanten Ringe 1, 2 und 3 | **464 / 246 / 827**, unverändert |
| Vierte Quellidentitätsebene | **86/86** klassifiziert, davon 85 explizite §1.2.1-Originalnormbibliografien plus ein MS-ADLS-§1.1-Schemasonderfall; **1.147** Normkanten unverändert |
| Nachgelagerte Originalnormbibliografien | **103** vollständig auditiert (**93 + 10**) |
| Nachgelagerte gerichtete Normkanten | **1.373** (**1.228 + 145**) |
| Nachgelagerte separate informative Zitate | **384** (**304 + 80**) |
| Ursprüngliche 88er-Workqueue | **78** Originalnormquellen auditiert, **1** Schemaausnahme, **9** reguläre ungeprüfte |
| Zusätzlich transitive MS-/MC-Rohquellen | **21** ungeprüft |
| Kombinierte noch offene wörtliche Rohbezeichnungen | **30** |
| Generische Schema-§1.1-Zitate | 67 ältere Drittstufen + 13 MS-ADLS, weiterhin **UNCLASSIFIED** |
| MS-DMRP | 2017er Originalausgabe `v20170601` Rev8.2 weiterhin ausstehend; 2026er Microsoft-Learn-Webexport nur ergänzende Webfassung |
| Formelle Phase 1 | **0/12 Abschlussgates PASS** |

**Weiterarbeit:** Zuerst die neun verbliebenen ursprünglichen Kandidaten (insbesondere verfügbare `MS-PCCRC`, `MS-RDPEGT`, `MS-RSVD`, `MS-SMBD`, `MS-WMI`, `MS-XWDVSEC`) und ausgewählte der 21 transitiven Kandidaten (z. B. `MS-MEETS`, `MS-WDVSE`, `MS-FSSHTTP`, `MS-WSSFO2`) nach exakter PDF-Verfügbarkeit prüfen. MS-DMRP-v20170601-Original, MS-ODATAJSON und MS-SQLRS weiterhin nicht durch Webseitenexports ersetzen. Externe RFC/IETF/ISO/ITU/W3C-Zieloriginale sowie die echte Skype-for-Business-2019-Windows-Full-Client-Funktions-/Build-/Policy-/Rollenmatrix bleiben OPEN.

**Sperren bis zum formellen Phase-1-Abschluss:** Keine Protokollimplementierung, Live-Netzwerk-/Interop-Tests, bezahlten CI-Runner, TLS-/Authentisierungsabsenkung, Merge nach `main`, Tag oder Release. Original-PDF-Binärdateien beim Nutzer; nur Forschungstexte im Draft-Branch.
