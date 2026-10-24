# P1-03 – Acht weitere nachgelagerte Originalbibliografien: CSDL/EDMX, DFS, Distributed Link Tracking, EMF und Ereignisprotokolle

**Untersuchung:** 24.10.2026. Quelle: acht originale PDFs aus dem nutzereigenen `Windows_Protocols.tar.xz`, exaktes Member je Quelle `Windows_Protocols/[ID].pdf`. Vollständige Binärdateien wurden SHA-256-geprüft. Alle Editionen, Revisionsnummern, `§1.2.1 Normative References`, `§1.2.2 Informative References` und tatsächliche PDF-Fundseiten wurden getrennt aus dem Original überprüft, die ersten Literaturseiten zusätzlich als Bilder gerendert. **Keine PDF-Binärdatei in Git**.

## Einzelquellen und beweisbare Zahlen

| Original | Revision / Originaledition | Seiten | §1.2.1 Normative (PDF-S.) | §1.2.2 Informative (PDF-S.) | Vollfile SHA-256 |
|---|---|---:|---|---|---|
| MS-XCA | **10.0** / `v20240423` | 30 | **3** (5, 6) | **1** (6) | `03f2a632b84773c4e660382c87ae3ca50c17c9595c85065ece680a44cc1d729a` |
| MC-CSDL | **14.0** / `v20190313` | 126 | **9** (11) | **2** (11) | `dc357012d2aa3cf83e7ec467e4607e9c32f8598a29cdcbb41c6226f08fe7f199` |
| MC-EDMX | **9.0** / `v20190313` | 15 | **3** (5) | **2** (6) | `9b310aed5a70b77b20b787d235d76dbd370ba7f82fae554d815d66d8d591c15f` |
| MS-BKUP | **12.0** / `v20240423` | 22 | **5** (7, 8) | **4** (8) | `723bf75826c4cfbb10bbbfd5838f679707924906bdf6102f9ea795d54242bdd9` |
| MS-DFSRH | **17.0** / `v20240423` | 87 | **20** (10, 11) | **3** (11) | `911118f689d556f5577041fad3813e591195b11e83c7f299dc433f3997dcebb9` |
| MS-DLTW | **17.0** / `v20240423` | 38 | **9** (9) | **1** (9) | `b00fb17a43331083d05243bb55454d41a364b27ed136f39c17dd4e3d0b40aaa7` |
| MS-EMF | **18.0** / `v20240423` | 282 | **5** (17) | **7** (17) | `c0681914dc75a8672496d76103b31df78e5d6aeb950a1561e666ecc61d691241` |
| MS-EVEN | **26.0** / `v20240423` | 81 | **18** (9, 10) | **10** (10, 11) | `bda2097a5bb7204954f90f4bd0721d7ddf4aa3e92c85000f6397185b32edf327` |
| **SUMME** | acht getrennte Originale | | **72** | **30** | acht Vollfile-Hashes |

Alle einzelnen normativen Rohverweise mit PDF-Seitenzahl, Quelledition/Revision, Binärprüfsumme und Archivpfad stehen im [Originalaudit-JSON](fifth-stage-data-services-dfs-dltw-emf-event-audit-2026-10-24.json) und gerichtet im [nachgelagerten Normledger](fifth-ring-ms-swn-normative-ledger.json). Die 30 informativen Quellen bleiben getrennt von den Normkanten.

## Belastbare gerichtete Befunde

- **CSDL/EDMX-Beziehung:** `MC-CSDL → MC-EDMX` (PDF-S.11) sowie `MC-EDMX → MC-CSDL` (S.5) sind **beide normativ**. `MC-CSDL → MS-ODATA` ist normativ, `MC-EDMX → MS-ODATA` hingegen **nur informativ** (S.6). Dieses Detail schließt eine stillschweigende Gleichsetzung der beiden Rückrichtungen aus.
- **Historische XML-Editionen:** `MC-CSDL` nennt `XML1.0`, `XMLNS-2ED`, `XMLSCHEMA1` normativ auf PDF-S.11. `MS-DFSRH` nennt davon getrennt `XML10`, `XMLSCHEMA0`, `XMLSCHEMA1` und `XMLSCHEMA2` auf S.11. Unterschiedliche Rohlabels werden **nicht** ungeprüft kanonisiert oder durch neuere Editionen ersetzt.
- **Dateisicherungen:** `MS-BKUP → MS-FRS1`, `→ MS-FRS2`, `→ MS-FSCC` sind normativ (S.7); `MS-BKUP → MS-DLTW` ist **nur informativ** (S.8). Aus dem Verweis folgt weder eine Sicherungsserver- noch eine DLT-Implementierungspflicht des SfB-2019-Clients.
- **DFS Replication Helper:** `MS-DFSRH → MS-ADLS` ist normativ (S.10). `MS-ADLS` selbst enthält keine gesonderte §1.2.1-Literaturbibliografie, sondern §1.1 mit **13 weiterhin UNCLASSIFIED** Quellen; seine normativ definierten Schemaattribute/-klassen stehen davon unberührt. `MS-DFSRH → MS-FRS2` und `→ MS-RPCE` sind normativ (S.10), `→ LDAP-ERR` nur informativ (S.11).
- **Distributed Link Tracking:** `MS-DLTW → MS-DLTM` ist **nur informativ** (PDF-S.9), während `MS-DLTW → MS-RPCE`, `→ MS-SMB`, `→ MS-SMB2` und `→ RFC1088` normativ sind. Eine Übersicht über beide DLT-Protokolle ist kein zusätzliches normatives Rückzitat. Externe Beschreibung: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dltw/fc649f0e-871a-431a-88b5-d5b2f80e9cc9.
- **Wichtiges editionsgeprüftes Zusatzoriginal:** Neben dem archivierten **MS-DLTW v20240423, Revision 17.0**, PDF-S.9, SHA-256 `b00fb17a43331083d05243bb55454d41a364b27ed136f39c17dd4e3d0b40aaa7` lag das zuvor separat vom Nutzer bereitgestellte **MS-DLTW v20210625, Revision 16.0**, **38 PDF-Seiten**, SHA-256 `28a8b2a3e57e2594c0d96501679cf413841c432975364cb2ad87a85a09fe8a15` vor. Die exakten **neun normativen** und **ein informatives** Rohlabel sind auf PDF-S.9 in beiden Ausgaben **identisch**. Es wird deshalb nur **eine** Quelle im aktuellen nachgelagerten Ledger gezählt; die Editionen werden **nicht** als identische Binärdateien oder als beweisbar identische Protokollsemantik behandelt.
- **Metafile-Referenzen:** `MS-EMF → MS-WMF` steht normativ (PDF-S.17); `MS-EMF → MS-EMFPLUS` steht **nur informativ** (S.17). In der zuvor geprüften Quelle `MS-WMF` war `MS-WMF → MS-EMF` ebenfalls nur informativ. Keine normative Rückkante aus diesen informativen Nennungen ableiten.
- **Event Log Remoting:** `MS-EVEN → MS-EVEN6`, `→ MS-RPCE`, `→ MS-SMB` normativ (PDF-S.10). `MS-EVEN` enthält zehn separate informative Verweise, darunter `MSFT-CVE-2021-31958` (S.11). Microsoft beschreibt `MS-EVEN6` als Ersatz von `MS-EVEN`; dies macht nicht das gesamte Ereignisprotokoll zur Skype-for-Business-Full-Client-Pflicht: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-even/f9f6ba31-0ba4-4524-bb31-39a5ef4667ee.
- **Algorithmus MS-XCA:** `MS-XCA` zitiert `IEEE-MRC`, `RFC2119` und `UASDC` normativ (PDF-S.5–6). `MS-DTYP` steht dagegen in §1.2.2 **nur informativ** auf S.6. Ein Algorithmusverweis allein belegt keine Clientfunktion.

## Abweichung gegenüber dem geplanten Batch

`MS-DMRP` ist weiterhin ein **offener** Quellkandidat, aber in keinem der aktuell vorhandenen vier PDF-Archive mit diesem Dokumentnamen zu finden. Er wurde nicht als geprüft markiert oder durch eine erfundene PDF-Fassung ersetzt. Stattdessen wurde der **ebenfalls offene und tatsächlich im Windows-Archiv vorhandene** Kandidat `MS-EVEN` geprüft. `MS-DLTM v20170601` liegt ebenfalls separat beim Nutzer vor, ist jedoch kein regulär offenes Quellziel dieses Batches; seine PDF-Binärdateien bleiben ungeändert außerhalb von Git.

## Reproduzierbare Bilanz und offene Forschungsfragen

| Ebene / Workstream | Bestand |
|---|---|
| Erste / zweite / dritte normative Ringe | **464 / 246 / 827** Kanten, unverändert |
| Vierte Quellenidentitäts-Ebene | **86/86 klassifiziert**, davon **85** §1.2.1-Normbibliografien und ein MS-ADLS-§1.1-Schema; **1.147** normative gerichtete Kanten, unverändert |
| Nachgelagerte Microsoft-/MC-Originalbibliografien | **25** bisher geprüft, **347** Normkanten (275 + **72**), **84** separate informative Literaturzitate (54 + **30**) |
| Rohziel-Workqueue aus vierter Ebene | **88** Kennungen; **25** normative Originalbibliografien geprüft, **1** MS-ADLS-Sonderfall §1.1 schon klassifiziert, **62** reguläre Kandidaten offen |
| Allgemeine Schemareferenzen | 67 früher in der dritten Ebene plus 13 bei MS-ADLS bleiben separat **UNCLASSIFIED** |
| Formelle Phase-1-Gates | **0/12 PASS** |

**Nächster Wiederaufnahmepunkt:** Offene Rohzielkennungen in [der Workqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) weiter untersuchen; z. B. `MS-DMRP` nur mit konkret verfügbarem Original, ansonsten `MS-FPSE`, `MS-FRS2`, `MS-GPFR`, `MS-GPIPSEC`, `MS-GPSCR`, `MS-GPSI`, `MS-MQMR`, `MS-MQRR`. Zusätzlich die historischen *zitierten Zieloriginalversionen* auflösen, die externen RFC/IETF/ISO/ITU/W3C-Referenzen editionsgenau prüfen und die tatsächlichen SfB-2019-Windows-Full-Client-Build-/Policy-/Serverrollenpflichten unabhängig belegen.

**Methodische Einschränkung:** Die 88 Rohziele sind keine kanonisierte fünfte BFS-Ebene und kein normativer Fixpunkt; häufige normative Literaturverweise beweisen keine umfassende Implementierungspflicht. Bis zum formellen Abschluss von Phase 1: **keine Protokollimplementierung, keine Live-Tests, keine bezahlte CI, keine TLS-/Auth-Abschwächung, kein Merge nach `main`, kein Release**. PDF-Originale bleiben beim Nutzer.
