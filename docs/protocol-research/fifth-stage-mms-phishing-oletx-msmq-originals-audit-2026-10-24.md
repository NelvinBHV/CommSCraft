# P1-03 – Sechs Exchange-MMS/UM/Phishing- und vier Windows-OleTx/MSMQ-Originalbibliografien

**Untersuchungsdatum:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23`. Diese Etappe umfasst ausschließlich dokumentarische Protokoll- und Normreferenzforschung, keine Protokollimplementierung oder Netzwerktests.

Sechs **vollständige originale PDF-Dateien** stammen aus dem nutzereigenen `Exchange_Protocols.zip`, vier aus `Windows_Protocols.tar.xz`. Zu jeder Quelle wurden die vollständigen PDF-Binärbytes SHA-256-geprüft, PDF-Seitenanzahl, Microsoft-Originaledition `vYYYYMMDD`, Revision und tatsächliche Fundseiten der getrennten Abschnitte **§1.2.1 Normative References** und **§1.2.2 Informative References** festgehalten. Die relevanten Originalreferenzseiten wurden zusätzlich gerendert und bildlich inspiziert. **Keine Binär-PDF-Dateien in Git**, alle Originale bleiben beim Nutzer.

## Originaleditionen mit vollständig überprüfbarem Seiten-/Hashbeleg

| Originalkennung | Edition / Revision | PDF-Seiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | Vollständiger PDF-SHA-256 |
|---|---|---:|---|---|---|
| MS-OXORMMS | `v20250520` / **17.0** | 32 | **13** (8) | **6** (8, 9) | `9fa4bae29d6301b750a8d70ebf334c854ffb11f15525b8a9af5b9adcd64aae1d` |
| MS-OXOSMMS | `v20240820` / **12.0** | 24 | **8** (8) | **4** (8) | `191d622a9aadd80f71a4d4c3a5b50934fac0bfcc2a32657e02973dec8b3c2b6e` |
| MS-OXOUM | `v20250520` / **16.0** | 36 | **15** (7, 8) | **1** (8) | `855cb91f276049fa2118b4f2ffbcb32097c83ccf6ae5e82912e09fabc879d03b` |
| MS-OXPFOAB | `v20160914` / **8.0** | 22 | **14** (7) | **1** (8) | `be0f5d1127d17053c569341ec6c3ba587af989ae80669ed417ee95fa337da976` |
| MS-OXPHISH | `v20250520` / **14.0** | 17 | **6** (5, 6) | **3** (6) | `36928cc268967cafe33429bc9cab4cf25debfe804daec31c18e974447dc68408` |
| MS-OXPSVAL | `v20250520` / **19.0** | 21 | **10** (7) | **1** (7) | `ffb9def94228dcb4ecb7ca91c980e9f4fe8c56dd86111d4d33010706df177c27` |
| MS-CMPO | `v20240729` / **31.0** | 75 | **8** (9, 10) | **4** (10) | `d857cb2c186dc6e8fe6737764c8df689e3d215307632b4cbf3eb4d8b50748328` |
| MS-DTCM | `v20240423` / **14.0** | 64 | **8** (11) | **0** (keine) | `9734496e1d801e88b194cfd0f3013cb2553bc04fc6d8576e9b4b1624c80deace` |
| MC-DTCXA | `v20240423` / **25.0** | 252 | **10** (16) | **1** (16) | `aaa088f51ef472cf597af0581d28cc68193a0b3adc5f85626fe44d028efd3543` |
| MC-MQAC | `v20240423` / **27.0** | 364 | **22** (17, 18) | **7** (18, 19) | `12d90a6c3f2c507f450faa0ce6a9591a4e14e9388a4a90ed074d50e5bb448e26` |
| **SUMME** | 10 originale Quellen | | **114** | **28** | 10 vollständige Originalhashes |

Das [maschinenlesbare Originalaudit-JSON](fifth-stage-mms-phishing-oletx-msmq-originals-audit-2026-10-24.json) bewahrt jede Referenz mit unverändertem Ziel-Rohlabel und der tatsächlichen Quell-PDF-Seite. Der [nachgelagerte gerichtete Normledger](fifth-ring-ms-swn-normative-ledger.json) bindet die 114 neuen Normkanten an Version, Revision, SHA-256, vollständigen Original-Archivpfad, Quell-Seitenzahl und getrennte informative Referenzen. Historische **zitierte Zieloriginaleditionen** bleiben `UNRECONCILED`.

## Wesentliche belegte normativ/informativ getrennte Kanten

1. **MMS/Rights Management:** `MS-OXORMMS` zitiert `MS-CFB`, `MS-OFFCRYPTO`, `MS-OXMSG`, `MS-RMPR`, `RFC1950` und `RFC1951` auf PDF-S.8 **normativ**; `MS-OXOMSG` (S.8), `MS-OXORMDR` und `MS-OXOTASK` (S.9) stehen dagegen **informativ**. Daraus entstehen neue rohe transitive Microsoft-Normziele `MS-CFB` und `MS-OFFCRYPTO`; keine automatische 2019-Clientpflicht oder kryptografische Profilwahl.
2. **Unterschiedliche Exchange-Editionen:** `MS-OXOSMMS` trägt **v20240820 Revision 12.0**, nicht v20250520; `MS-OXPFOAB` ist das nochmals deutlich ältere Original **v20160914 Revision 8.0**. Die bibliografischen Referenzen dieser historischen Originale dürfen weder als nachgewiesen identische 2025er Protokollsemantik noch als konkrete Build-/Policy-Pflicht eines Skype-for-Business-2019-Clients behandelt werden.
3. **Mailbox-Unified-Messaging:** `MS-OXOUM` zitiert `MS-OXORMMS`, `ASF`, `G711`, `GSM610`, `WAVE` sowie `XMLSCHEMA2/2` in **§1.2.1** auf PDF-S.7–8; `MS-OXPROTO` wird ausschließlich **informativ** genannt (S.8). Die editionsspezifischen Standardlabels bleiben unverändert; Audio-/MIME-Unterstützung ist nicht automatisch eine Gesamtheit verpflichtender Clientfunktionen.
4. **Public Folder OAB:** `MS-OXPFOAB` zitiert `MS-OXOAB`, `MS-OXOABKT`, `MS-OXWOAB`, `MS-OXDISCO` und `MS-OXCRPC` normativ auf S.7. Die OAB-Implementierungspflicht eines Skype-Clients, die Zieloriginalausgabe und insbesondere die Aktualität des 2016er Originals sind dadurch noch nicht nachgewiesen.
5. **Spam/Phishing und Identifier:** `MS-OXPHISH → MS-OXCSPAM` ist normativ (S.6), aber `MS-OXPHISH → MS-OXCPRPT` sowie `→ MS-OXOMSG` sind nur informativ (S.6). `MS-OXPSVAL` führt `FIPS180`, `RFC1123`, `RFC2821`, `RFC2822` normativ auf S.7 und `MS-OXPROTO` ausschließlich informativ.
6. **OleTx/RPC Richtungsasymmetrie:** `MS-CMPO → MS-RPCE` ist normativ auf Original-S.10; `MS-CMPO → MS-CMOM`, `→ MS-CMP` und `→ MS-DTCO` sind **nur informativ** (S.10), auch wenn umgekehrte Quellen normativ auf `MS-CMPO` referenzieren. `MS-DTCM → MS-CMPO`, `→ MS-CMP`, `→ MS-DTCO` stehen normativ auf S.11. `MS-DTCM §1.2.2` enthält ausdrücklich **keine** informativen Literaturreferenzen.
7. **Transaction Adapter:** `MC-DTCXA → MS-CMPO`, `→ MS-CMP`, `→ MS-DTCO` sind normativ auf S.16; `MC-DTCXA → MS-CMOM` ist **informativ**. Historische RFC-/ISO-/X/Open-Verweise sind noch nicht editionsgenau als gesamte externe Normrekursion abgearbeitet.
8. **Message Queuing ActiveX:** `MC-MQAC` listet 22 normative Referenzen auf PDF-S.17–18, darunter `MC-MQSRM`, `MS-DTCO`, `MS-MQDMPR`, `MS-MQDSSM`, `MS-MQMP`, `MS-MQMQ`, `MS-MQMR` und `MS-MQQB`. Die sieben Quellen `Box98`, `LDAP`, `MS-MQOD`, `MSDN-ECP`, `MSDN-EC`, `MSDN-MQCOC` und `MSDN-MQEIC` sind ausschließlich informativ. Besonders `MS-MQMP` tritt nun als weitere offene transitive Originalquelle auf.

## Nachgelagerte Zielmengen nach erneuter vollständiger Ledger-Auswertung

Die [ursprüngliche Viertstufen-Zielworkqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält unverändert **88** neue MS-/MC-Rohkennungen nach Abzug zuvor geprüfter Quellidentitäten der Ringe 1–4. Sechs ihrer Originalnormbibliografien wurden in dieser Etappe auditiert. Ergebnis: **67** normativ originalgeprüfte Quellen, **ein** als MS-ADLS-§1.1-Schema separat klassifizierter Sonderfall (seine 13 allgemeinen Literaturangaben bleiben **UNCLASSIFIED**), **20** reguläre Quellen weiterhin offen.

Die zusätzliche [transitive Ziel-Workqueue](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) wurde **aus allen 1.122 gerichteten nachgelagerten Normkanten neu berechnet** und schließt sämtliche zuvor geprüften Quellen, die vollständige ursprüngliche 88er-Queue und bereits downstream-auditierte Quellkennungen aus. Vier zuvor offene transitive Quellen sind geprüft (`MS-CMPO`, `MS-DTCM`, `MC-DTCXA`, `MC-MQAC`). Neuentdeckte Rohlabels unter den 16 aktuell offenen zusätzlichen transitive Quellen sind u. a. `MS-CFB`, `MS-OFFCRYPTO` und `MS-COM`.

**Kombinierter offener Bestand: 20 + 16 = 36 verschiedene noch nicht originalauditiert vorkommende MS-/MC-Rohbezeichnungen.** Diese Kennungen sind *nicht* zitierten historischen Originaleditionen zugeordnet, nicht beweisbar die kürzeste fünfte BFS-Ebene und **kein abgeschlossener normativer Fixpunkt**. Weitere Originalbibliografien und externe Standards können zusätzliche Ziele erzeugen.

## Zähler und Wiederaufnahme

| Forschungsbereich | Gesicherter Stand |
|---|---|
| Erste, zweite und dritte Normreferenzebene | **464 / 246 / 827** Kanten, unverändert |
| Vierte Quellenidentitäts-Ebene | **86/86 identifiziert und klassifiziert**: 85 explizite §1.2.1-Normbibliografien plus ein MS-ADLS-§1.1-Schemakatalog; **1.147** Normkanten unverändert |
| Nachgelagerte originale Normbibliografien | **83** Originalquellen / **1.122** gerichtete normative Kanten / **283** separat informative Literaturverweise |
| Anfangs-Workqueue aus viertstufigen Normzielen | **88** rohe MS-/MC-Zielkennungen; **67** Originalquellen auditiert, ein Schema-Sonderfall klassifiziert, **20** reguläre offen |
| Neu rekalkulierte transitive MS-/MC-Ziele | **16** zusätzliche offene rohe Zielkennungen |
| Kombiniert offen | **36 wörtlich unterschiedliche Rohkennungen** |
| Nicht klassifizierte allgemeine Schema-§1.1-Literatur | 67 ältere Drittstufen- und 13 MS-ADLS-Nennungen weiterhin **UNCLASSIFIED** |
| MS-DMRP | Offizielles historisches Original-PDF `v20170601` Revision 8.2 weiterhin ausstehend; separater 2026-Learn-Webexport ist kein editionsidentischer Original-PDF-Nachweis |
| Phase 1 | **0/12 formelle Abschlussgates PASS** |

**Nächste Gruppe:** Verfügbare Originale aus den **20** bisherigen Kandidaten wie `MS-OXSHARE`, `MS-OXSHRMSG`, `MS-MSRP`, `MS-OXLDAP`, `MS-OXCEXT`, `MS-ODATAJSON` sowie den **16** transitive Rohzielen `MS-CFB`, `MS-OFFCRYPTO`, `MS-MQMP`, `MS-COM`. Vor einer Quellenklassifizierung Original-PDF und seine Revisionshistorie überprüfen, anschließend neue Ziele über den vollständigen Normledger rechnen. Außerdem externe RFC/ISO/ITU/IETF/W3C-Historie und tatsächliche Skype-for-Business-2019-Windows-Full-Client-Feature-/Build-/Policy-/Serverrollenmatrix unabhängig belegen.

**Verbindliche Sperren:** keine Protokollimplementierung, keine Live-Netzwerk-/Interoperabilitätstests, keine kostenpflichtigen CI-Runner, keine TLS-/Authentisierungsabsenkung, kein Merge nach `main`, Tag oder Release. Original-PDFs ausschließlich beim Nutzer.
