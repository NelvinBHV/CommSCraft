# P1-03 – Sechs Exchange-Originale und vier neue transitive Windows-Originalbibliografien

**Forschungsstand:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23` · **Dokumentationsarbeit ohne Implementierung**.

Sechs vollständige Original-PDFs stammen aus dem vom Nutzer bereitgestellten `Exchange_Protocols.zip`; vier weitere aus `Windows_Protocols.tar.xz`. Für jedes Original wurden sämtliche PDF-Dateibytes per SHA-256 geprüft, Originaledition `vYYYYMMDD`, Revision, `§1.2.1 Normative References` und `§1.2.2 Informative References` inklusive Originalseiten ausgewertet. Die Literaturstartseiten wurden zusätzlich gerendert und visuell auf korrekte Abschnittsgrenzen überprüft. **Die PDF-Binärdateien verbleiben beim Nutzer und werden nicht in Git gespeichert.**

## Originalquellen und bibliografischer Befund

| Originalkennung | Revision / Edition | PDF-Seiten | §1.2.1 normativ (Original-S.) | §1.2.2 informativ (Original-S.) | SHA-256 komplette PDF |
|---|---|---:|---|---|---|
| MS-OXOCNTC | **21.0** / `v20250520` | 74 | **16** (10, 11) | **2** (11) | `7c5f2a564b7c48706e688b5ed88c965776cf16ebc69f8838cbc2fbafaabee496` |
| MS-OXODOC | **13.0** / `v20250520` | 24 | **6** (7) | **2** (7) | `082f9a15774d1971f51dbdeb90e52823bfb5c2421fd2eed38bb6ffb2ae3f5573` |
| MS-OXOFLAG | **15.0** / `v20250520` | 41 | **11** (8) | **1** (8) | `b4aab19824d81a18ae6cbd45976e4a198de1d47a41960dd650526d4131289123` |
| MS-OXOSMIME | **15.0** / `v20250520` | 23 | **10** (7, 8) | **7** (8) | `93962010c6ffb4fd0b3247bad479144b95f849c2f51a6b1764281512f2d69274` |
| MS-OXTNEF | **15.0** / `v20250520` | 51 | **10** (8) | **8** (8) | `1011dece1c231d3d5cdb4b3e952a3e3105d9c7af1e53811cf289d8489e86c3e2` |
| MS-OXOTASK | **16.0** / `v20250520` | 40 | **12** (8) | **3** (8) | `232bde17edc3cd95f325c54b7c1deb0adfc61cf002d74fe6dbfa0597ae94595c` |
| MS-NSPI | **16.0** / `v20240423` | 103 | **12** (9, 10) | **0** (None) | `cfd80777a37a1134ed8d1f2e0722dbec4aaee6043e7378fd3eddcd448c0927b9` |
| MS-MQDMPR | **32.0** / `v20240423` | 138 | **29** (12, 13, 14) | **3** (14) | `66513c84b868d3ae179413eaacbbd41d7c99875b9cedd48c0a3f69c57da5c956` |
| MS-MQQB | **33.0** / `v20240423` | 139 | **25** (11, 12) | **2** (12) | `2f95df5b5eb5db5c96c6f4eef75dd239aca8dc57af0e66f8dde3efa5228a4a7c` |
| MC-MQSRM | **24.0** / `v20240423` | 109 | **29** (10, 11, 12) | **2** (12) | `1d53db156d2ef581dd9d2efcf942dc07e5fcc9af0523216f1d0477c0c05e6dbe` |
| **SUMME** | 10 Originale | | **160** | **30** | 10 vollständige Originalhashes |

Normative und informative Rohlabels mit Original-PDF-Seiten, Quellrevision, vollständigem Quell-SHA-256 sowie präzisem Archivmitglied sind im [Originalaudit-JSON](fifth-stage-six-exchange-four-transitive-msmc-originals-audit-2026-10-24.json) enthalten. Das [fortgeschriebene nachgelagerte Normregister](fifth-ring-ms-swn-normative-ledger.json) enthält die **160 neuen gerichteten normativen Literaturkanten**, nicht jedoch die 30 ausschließlich informativen Zitate.

## Bedeutsame belegte Richtungen

1. **Echte Selbstreferenz im Kontaktobjekt:** `MS-OXOCNTC v20250520 §1.2.1` zitiert **`[MS-OXOCNTC]`** auf Original-PDF-S.11 zwischen den anderen normativen Quellen. Ebenfalls auf dieser Seite erscheint eine PDF-Fußzeile `[MS-OXOCNTC] - v20250520`, die **ausgeschlossen** wurde. Der echte eigenständige §1.2.1-Eintrag bleibt als eine normative `MS-OXOCNTC → MS-OXOCNTC`-Selbstkante gekennzeichnet. Kein rekursives Neuladen derselben bereits geprüften Originalquelle erforderlich.
2. **Keine automatische Aufwertung von Referenzen:** `MS-OXODOC → MS-OXCROPS` ist **nur informativ** (S.7); `MS-OXOTASK → MS-OXCFXICS` ist **nur informativ** (S.8). `MS-OXTNEF → MS-OXCFOLD`, `→ MS-WMF` sowie `→ RFC2045` sind ausschließlich informative Nennungen (S.8), obwohl dieselben Kennungen anderswo normativ referenziert werden können. Die Klassifikation ist je Referrer und Zitierabschnitt zu ermitteln.
3. **Historische S/MIME-Standards:** `MS-OXOSMIME` zitiert `RFC1847`, `RFC2045`, `RFC2183`, `RFC3852` und `RFC5751` normativ auf S.8, jedoch `RFC2046`, `RFC2047`, `RFC2048`, `RFC2049`, `RFC2311` und `RFC2822` nur informativ. Kein Editionswechsel oder Schluss auf verpflichtende SFb-2019-Client-Algorithmen ohne Quellenvergleich.
4. **Mailbox/Task-Protokolle:** `MS-OXOTASK → MS-OXODLGT` (S.8) und `MS-OXOFLAG → MS-OXOTASK` (S.8) sind normativ. Die genannten Exchange-Feature-Kanten sind Bibliografieabhängigkeiten, keine Produktfunktionsfreigaben.
5. **NSPI aus neuem transitiven Zielbestand:** `MS-NSPI v20240423 rev16.0` (S.9–10) führt zwölf normative Rohziele, darunter `MS-OXCDATA`, `MS-OXOABKT`, `MS-OXOABK`, `RFC1510` und `RFC4120`. Das informative Verzeichnis §1.2.2 enthält **keine** Literaturziele. Ein PDF-Seitenfuß mit der eigenen Kennung wurde nicht als Eintrag gezählt.
6. **MSMQ-Normzyklen und Klassifizierungsasymmetrie:** `MS-MQDMPR → MC-MQSRM` (S.13), `MC-MQSRM → MS-MQDMPR` (S.10), `MS-MQQB → MS-MQDMPR` (S.11) und `MC-MQSRM → MS-MQQB` (S.11) sind normative Literaturverweise. `MS-MQDMPR → MS-MQQB` steht dagegen **nur informativ** (S.14); deshalb ist auch hier kein wechselseitiges Normzitat ohne Quellprüfung anzunehmen.
7. **Zusätzliche Sicherheitsnormen, historische Ausgaben:** `MS-MQDMPR` nennt `FIPS180-2`, `FIPS186`, `FIPS197`, `FIPS46-3`, `RFC8017`; `MS-MQQB` nennt u. a. `SP800-38A`, `RFC4757` und `X509`; `MC-MQSRM` nennt `RFC2246` und `XMLDSig`. Diese Originalrohlabels sind kein Ersatz für die genaue Editions-/Sicherheitsprofilentscheidung eines Skype-for-Business-2019-Windows-Full-Clients.

## Bereinigte zwei getrennte Folgefrontiers

Die ursprüngliche [vierte-Ring-Zielworkqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) umfasst **88 Rohkennungen**, die aus viertstufigen Normzitaten abgeleitet und vorher noch nicht als Quellen in den Ringen 1–4 erfasst waren. Aus dieser Menge wurden sechs Exchange-Originalquellen jetzt geprüft, sodass **49** Normoriginal-Bibliografien dieser Queue vorliegen, **eine** MS-ADLS-§1.1-Schema-Bibliografie weiterhin separat als **UNCLASSIFIED** gilt und **38** reguläre Quellen dort offen sind.

Die gesonderte [aktualisierte transitive Workqueue](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) betrachtet nun **alle 781** gerichteten normativen Kanten der nachgelagerten Originalbibliografien. Sie zieht die alten Quellenidentitäten der früheren Ringe, die komplette ursprüngliche 88er-Queue und sämtliche bereits geprüften nachgelagerten Quellen ab. Vier vorher offene transitive Kandidaten `MS-NSPI`, `MS-MQDMPR`, `MS-MQQB` und `MC-MQSRM` sind jetzt als Originalquellen geprüft. **Vier neue Quellenrohkennungen** wurden erkannt: `MS-MQDSSM`, `MS-MQBR`, `MS-MQCN`, `MS-MQSD`. Damit stehen in diesem getrennten Folgeinventar weiterhin **15** rohe unbewertete Kennungen.

**Die kombinierte offene Menge beträgt 38 + 15 = 53 unterschiedliche Rohbezeichnungen.** Dies ist keine kanonisierte historische Ziel-Editionenmenge, nicht mathematisch als kürzeste fünfte Ebene bewiesen und ausdrücklich **kein vollständiger normativer Fixpunkt**. Ein Teil der Zielkennungen kann sich auf Serverfunktionen, historische Profile oder nicht belegte Client-Features beziehen.

## Gesamtbilanz

| Forschungsbereich | Stand |
|---|---|
| Erste / zweite / dritte normative Stufe | **464 / 246 / 827** gerichtete Kanten, unverändert |
| Vierte Quellidentitäts-Ebene | **86/86 klassifiziert**, davon **85** explizite §1.2.1-Originalbibliografien + **ein** MS-ADLS-§1.1-Schema-Sonderfall; **1.147** normative Kanten, unverändert |
| Nachgelagerte Originalbibliografien | **53** geprüfte Normbibliografien, **781** gerichtete normative Kanten (**621 + 160**) und **191** getrennte informative Literaturzitate (**161 + 30**) |
| Ursprüngliche Viertstufen-Rohzielqueue | **88** gesamt, **49** Original-Normbibliografien auditiert, **ein** Schema-Sonderfall, **38** offen |
| Zusätzlich nachgelagert auftauchende Ziele | **15** noch nicht als Originalbibliografie geprüfte MS-/MC-Rohlabels |
| Kombinierter offener MS-/MC-Rohzielbestand | **53** (38 + 15), noch nicht editionsnormalisiert |
| Nicht klassifizierte Schema-Nennungen | 67 Drittstufen-§1.1 + 13 MS-ADLS-§1.1, weiterhin **UNCLASSIFIED** |
| MS-DMRP | Historische offizielle PDF-Ausgabe `v20170601` / Revision 8.2 noch nicht binär geprüft; 2026 Microsoft-Learn-Webexport **nicht** als Originalbibliografie gezählt |
| Formelle Phase 1 | **0/12 Gates PASS** |

**Wiederaufnahmepunkt:** aus den **38** ursprünglichen offenen Kandidaten z. B. `MS-OXODLGT`, `MS-OXORULE`, `MS-OXOSFLD`, `MS-OXOSRCH`, `MS-OXRTFCP`, `MS-OXVCARD` untersuchen; separat aus **15** transitiven Rohzielen `MS-MQDS`, `MS-MQDSSM`, `MS-DTCO`, `MS-MQBR` nach Originalverfügbarkeit. Die historischen Zielversionen der RFC-/ISO-/IETF-/ITU-/W3C-Normverweise sowie die reale Skype-for-Business-2019-Windows-Full-Client-Build-/Serverrollen-/Policy-Abdeckung müssen unabhängig nachgewiesen werden.

**Sperren:** Keine Protokollimplementierung, keine Live-Server-/Interop-Tests, keine Nutzung bezahlter CI-Runner, keine TLS-/Authentisierungsabsenkung, kein Merge nach `main`, Tag oder Release; alle Original-PDF-Binärdateien verbleiben beim Nutzer.
