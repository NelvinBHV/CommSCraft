# P1-03 – Zehn weitere Originalquellen (SharePoint / Windows / Exchange)

**24.10.2026 · Ausgangs-HEAD `4744e4d14796ab2cfbbe3e7f9b2c0921b1a8ebe6` · ausschließlich `draft/protocol-spec-analysis-2026-10-23`.** Die PDF-Binärdateien stammen aus den vier Nutzer-ZIP-Archiven; keine PDF-Binärdateien nach Git. Original-PDF vollständig SHA-256-geprüft. Ausgabe aus Originalfußzeile und Revision aus offizieller Microsoft-Publikationstabelle (gleicher Veröffentlichungstag). §1.2.1 und §1.2.2 streng getrennt, Seiten 1-basiert, kein PDF-Footer als Verweis. Alle Einzelnachweise im JSON und im Norm-/Info-Ledger.

| Quelle | PDF-Edition | Rev. | PDF-Seiten | Normativ §1.2.1 | Informativ §1.2.2 | SHA-256 vollständige PDF |
|---|---|---:|---:|---:|---:|---|
| MS-OUTSPS | v20241112 | 11.1 | 96 | 7 | 5 | `25908296a5e77dbdd0e73484dfbfe0c2e115d11de2b5000395c591901e110812` |
| MS-PRSTFR | v20211005 | 7.0 | 31 | 7 | 4 | `8bcaca88f2c7400f6cca89b4c73e54fa7bf92a4b0465af47ed57d332c431bb51` |
| MS-VIEWSS | v20240820 | 13.0 | 60 | 10 | 2 | `11c68a36d716cec5963cc01f93db33487ee6354c50d74b12a401b609b0060621` |
| MS-WSSCAML | v20240820 | 17.0 | 144 | 15 | 1 | `cdc0233418c51f465521719d748c1f47ab876cbdba1362807f80d39adb52ed54` |
| MS-WSSFO3 | v20170620 | 4.0 | 484 | 13 | 2 | `d444ffcd0081f2fbdcd0b2c4844c0beedc60358265942d7cc2a2976430659d38` |
| MS-WSSTS | v20211005 | 7.0 | 61 | 9 | 1 | `ecbc0a5e1cfb6b15cc0923d275f0a6c901876ddc96715a9e7103598b65d693e2` |
| MS-HCEP | v20170601 | 19.0 | 42 | 20 | 11 | `2317126b54020ec863f77432d2be5915c0cbdad40efe95d5fbeee5b6513574fe` |
| MS-MCI | v20161017 | 7.3 | 12 | 2 | 1 | `58f9ba1e32800db84fd3cefead9e20facf8279fa61f5c0c4c1a4facd9887b285` |
| MS-MWBE | v20171201 | 11.0 | 51 | 18 | 7 | `ed6295a94322f5ce3d236e7db9a2789dcd2f820e2463eabb681c1c174f2cf966` |
| MS-MWBF | v20240423 | 16.0 | 72 | 31 | 12 | `1304d44e2be6281821569f19fea17150efc05300562321dad4ac32ce48fa7423` |

**+10 originale Bibliografien, +132 gerichtete normative Kanten, +46 separat informative Zitate**. Neuer downstream-Gesamtstand: **133 / 1.708 / 484** (zuvor 123 / 1.576 / 438). Sechs bisher neu aus der letzten Rekursion: `MS-OUTSPS`, `MS-PRSTFR`, `MS-VIEWSS`, `MS-WSSCAML`, `MS-WSSFO3`, `MS-WSSTS`; vier weitere bereits offene: `MS-HCEP`, `MS-MCI`, `MS-MWBE`, `MS-MWBF`.

## Abgrenzung und markante Belege

- `MS-OUTSPS → MS-LISTSWS` auf PDF-Seite 9 ist normativ; `MS-OUTSPS → MS-STSSYN` auf Seite 9 ist rein informativ.
- `MS-VIEWSS → MS-WSSCAML` auf Seite 9 ist normativ, `MS-VIEWSS → MS-LISTSWS` auf Seite 9 nur informativ. Die **zwei** originalen §1.2.2-Einträge enden vor §1.3; keine spätere Referenz aus dem Dokumentkörper hinzuzählen.
- `MS-WSSCAML → MS-WSSFO3` (S.16) sowie `MS-WSSTS → MS-WSSCAML` (S.10) sind normativ. Historische `MS-WSSFO3 v20170620 Rev4.0` bleibt eigene Ausgabe.
- `MS-HCEP → MS-WCCE` (S.8) ist normativ; `MS-MWBE v20171201 → MS-MWBF` und `MS-MWBF v20240423 → MS-MWBE` sind normative Zyklen (je S.10), aber keine Editionsgleichsetzung und keine rekursive Endlosschleife.

## Neu berechnete MS-/MC-Rohzielqueue

Alle **1.708** ausdrücklich normativen downstream-Kanten rekursionsrelevant ausgewertet; Ausschluss der ursprünglichen 88 Viertstufen-Rohzielkennungen, 133 bereits auditierten downstream-Quellen und der früheren Ring-Quellen-Ausschlüsse. Zusätzlich transitiv **20** offen; neu `MS-ADFSPIP`, `MS-OAPX`, `MS-WCCE`, `MS-WSSCAP`, `MS-WSSFO`. Zusammen mit 3 ursprünglichen regulären Ausständen **23** wörtlich verschiedene offene rohe Kennungen. Keine historisch editionskanonische Rekursion, kein bewiesener Fixpunkt.

Vorherige 28 offene Roh-IDs: **21** PDFs in Nutzerarchiven vorhanden; **7** fehlen (`MS-DHCPN`, `MS-DMRP`, `MS-ODATAJSON`, `MS-OFFCRYPTO`, `MS-OWEMXML`, `MS-SQLRS`, `MS-XSSK`). Nach diesem Zehnerbatch verbleiben **16/20** transitive Original-PDFs in Archiven, vier transitiv fehlende (`MS-DHCPN`, `MS-OFFCRYPTO`, `MS-OWEMXML`, `MS-XSSK`); drei initial fehlende (`MS-DMRP`, `MS-ODATAJSON`, `MS-SQLRS`). Alle fünf neuen Kandidaten `MS-ADFSPIP`, `MS-OAPX`, `MS-WCCE`, `MS-WSSCAP`, `MS-WSSFO` sind in den ZIP-Archiven vorhanden, aber noch nicht originalbibliografisch geprüft. Historisches `MS-DMRP v20170601 Rev8.2` fehlt weiterhin; 2026er Microsoft-Learn-Export ist kein historischer Originalbeleg.

## Unveränderte Grenzen

Frühere Normkantenebenen **464/246/827/1.147**, vierte Original-Quellidentität **86/86**. Ältere 67 allgemeine Schema-§1.1-Verweise sowie 13 MS-ADLS-Schema-§1.1-Verweise bleiben **UNCLASSIFIED**. Historische Zieldokumenteditionen, externe RFC/IETF/ISO/ITU/W3C-Normrekursion, Skype-for-Business-2019-Full-Client-Build/Feature/Policy/Rollen-Anwendbarkeit **OPEN**. Phase 1 weiterhin **0/12 Gates**, keine Implementierung, Live-Tests, bezahlte CI, Sicherheitsabsenkung, main-Merge, Tags, Release. Der neue Wiederaufnahmeprompt wird auf Wunsch **nur im Chat** ausgegeben und `NEXT_SESSION_PROMPT.md` **nicht verändert**.
