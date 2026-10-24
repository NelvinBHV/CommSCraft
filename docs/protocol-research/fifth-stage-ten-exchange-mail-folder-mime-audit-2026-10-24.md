# P1-03 – Zehn nachgelagerte originale Exchange-Bibliografien: Mail, MIME, Ordner, Nachrichten und Tabellen

**Forschungsdatum:** 24.10.2026 · **Nur autorisierter Draft-Branch:** `draft/protocol-spec-analysis-2026-10-23`.

Aus `Exchange_Protocols.zip` wurden zehn unveränderte PDF-Originale an ihren einzelnen Archivmitgliedern `[MS-ID].pdf` überprüft. Für jede vollständige Originaldatei wurden SHA-256, Dateigröße, Seitenzahl, die Originaledition **v20250520**, Revisionshistorie, `§1.2.1 Normative References` und `§1.2.2 Informative References` geprüft. Die jeweiligen Literaturstartseiten wurden zusätzlich gerendert. Die PDF-Binärdateien verbleiben ausschließlich beim Nutzer; Git enthält nur bibliografische Belege und Metadaten.

## Editions- und seitengebundener Originalaudit

| Original-ID | Revision / Edition | Seiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | Vollständiger PDF-SHA-256 |
|---|---|---:|---|---|---|
| MS-OXBBODY | **12.0** / `v20250520` | 18 | **10** (6) | **1** (6) | `f04f5885d8563d64856a77d9b2c43a0c080a3a00f08449ef40c78cd01929cb97` |
| MS-OXCFOLD | **25.0** / `v20250520` | 72 | **14** (11) | **3** (11) | `607a1cacc44178cdb88fb5b0ac3545d85d6ff8a3e8c2d933c87ac93b67136e10` |
| MS-OXCICAL | **15.0** / `v20250520` | 247 | **19** (11) | **4** (12) | `b7871e921240959f7f027e4ad91d6cb1d68a8e0f599cca0ec022ed65ac5c0e39` |
| MS-OXCMAIL | **24.0** / `v20250520` | 125 | **55** (14, 15, 16, 17) | **9** (17) | `bb2b732ae272b09275b46b340078b1af1e4eefdb9b6f286337af7c6b05a8163c` |
| MS-OXCMSG | **27.0** / `v20250520` | 91 | **28** (14, 15) | **12** (15, 16) | `4209a820e454e623abf47d24c2b4f374ac37762b85de03810d2b14de5244c53b` |
| MS-OXCPRPT | **19.0** / `v20250520` | 66 | **8** (10, 11) | **3** (11) | `b0917a7bd885f818b5ea8bc672592fa220796a3f12110c8d89dc4214e9b5b7ca` |
| MS-OXCSPAM | **12.0** / `v20250520` | 27 | **9** (7, 8) | **5** (8) | `65c1abdf9b525925eae90f959ec6959a07f6cd509e693113ccc0dc0455977fcc` |
| MS-OXCTABL | **23.0** / `v20250520` | 64 | **8** (9) | **7** (9) | `5acf0b1664ffc13f7e986e55d9e90a62ab6283ce142d4f35a6b18c8a36ce6a27` |
| MS-OXOABKT | **16.0** / `v20250520` | 53 | **6** (7, 8) | **2** (8) | `b8fe871574a9da522e6e020f0b8edab4d5a910733dc5e023f52cc38ec8e584fb` |
| MS-OXOCFG | **16.0** / `v20250520` | 103 | **22** (12, 13) | **4** (13) | `ac5fe44d72e92048568188b856252d4242b653c750d7846077e02251679e7971` |
| **GESAMT** | 10 Originalquellen | | **179** | **50** | 10 vollständige Hashes |

**Rohbelege:** [Audit-JSON](fifth-stage-ten-exchange-mail-folder-mime-audit-2026-10-24.json) bewahrt alle Norm- und Informativ-Einzelziele mit exaktem PDF-Originalseitenbezug, Edition, Revisionsnummer und Dateihash. Die 179 gerichteten Normkanten stehen im [nachgelagerten Normledger](fifth-ring-ms-swn-normative-ledger.json); die 50 informativen Zitate erscheinen nur im getrennten informativen Bereich. Editionsfremde Zitatzieloriginale wurden nicht stillschweigend kanonisiert.

## Erkenntnisse und Klassifikationskontrollen

1. **MIME/Mail:** `MS-OXCMAIL` enthält **55 normative** Ziele auf PDF-S.14–17, darunter `MS-OXBBODY`, `MS-OXCICAL`, `MS-OXOCAL`, `MS-OXRTFEX`, `MS-OXTNEF`, `RFC1738`, `RFC1740`, `RFC2045`, `RFC2822` und `RFC5234`. Neun weitere Ziele sind ausschließlich **informativ**, etwa `RFC1521`, `RFC2046`, `RFC2048` und `RFC2049`. Eine historische Normbibliografie ist kein Nachweis, dass Skype for Business 2019 jeden Exchange-Mail-Codec unterstützen muss.
2. **Terminologie, Rohlabels und Originaleditionen:** `MS-OXCICAL` zitiert `RFC2445` und `RFC2446` normativ (S.11), aber `RFC2447` nur informativ (S.12). `MS-OXOCFG` zitiert `XMLBase`, `XMLNS` und `XMLSCHEMA2` normativ (S.13). Diese konkreten historischen Rohlabels bleiben unverändert, bis ihre exakten Zieloriginaleditionen geklärt sind.
3. **Tatsächliche asymmetrische Referenzrichtung:** `MS-OXCTABL → MS-OXCMSG` steht **informativ** auf PDF-S.9; `MS-OXCMSG → MS-OXCTABL` steht dagegen **normativ** (S.15). `MS-OXCFOLD → MS-OXCRPC` ist ebenfalls nur informativ (S.11), während `MS-OXCMSG → MS-OXCRPC` normativ ist (S.14). Der Graph bildet jede Richtung mit der Klasse ihrer eigenen Originalquelle ab.
4. **Offline Address Book:** `MS-OXOABKT → MS-OXOAB` ist ausschließlich **informativ** (S.8), wohingegen `MS-OXOABKT → MS-OXOABK`, `→ MS-NSPI` und `→ MS-OXNSPI` normativ (S.7) sind. Der neue normative Rohzielverweis auf `MS-NSPI` ist ausdrücklich Teil der getrennten transitiven Folgearbeitsliste.
5. **Spam / Table / Permission:** `MS-OXCSPAM → MS-OXPHISH` ist normativ (S.8). `MS-OXCSPAM → MS-OXCPRPT` hingegen informativ (S.8); die getrennte Quelle `MS-OXCPRPT` hat wiederum acht eigene normative Abhängigkeiten, unter anderem `MS-OXCFOLD`, `MS-OXCFXICS` und `MS-OXCSTOR` (S.10). `MS-OXCTABL → MS-OXCFXICS` steht lediglich im informativen Literaturabschnitt (S.9).
6. **Zyklische Quellbibliografie-Referenzen:** `MS-OXBBODY → MS-OXCMSG` ist normativ (S.6) und `MS-OXCMSG → MS-OXBBODY` normativ (S.14). `MS-OXCICAL → MS-OXCMAIL` (S.11) und `MS-OXCMAIL → MS-OXCICAL` (S.15) ebenso. Zyklen verlangen eine besuchte-Knoten-Strategie bei der künftigen Referenzrekursion, nicht wiederholte Implementierung derselben Dokumente.
7. **Quellen-PDF-Fußzeilen ausgeschlossen:** Wiederholte `[MS-OXCMAIL] - v20250520`-, `[MS-OXCMSG] - v20250520`- und ähnliche Fußzeilen sind keine zusätzlichen normativen Literaturziele. Sie wurden bei der Extraktion explizit erkannt und ausgeschlossen.

## Neue transitive MS-/MC-Ziele außerhalb der ursprünglichen Rohziel-Liste

Die ursprüngliche [88er-Arbeitsliste](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) basiert **ausschließlich auf den normativen Rohzielen der vierten Ebene**, nach Abzug bereits früher geprüfter Quellen. Darin sind nach dieser Prüfung **43** Quellen bibliografisch überprüft, **1** MS-ADLS-Schema-§1.1-Sonderfall klassifiziert und **44** reguläre Originalquellen weiterhin offen.

Der aktuell aggregierte **nachgelagerte Normgraph mit 621 Kanten** nennt aber weitere Rohziele, die in den ersten vier Ebenen noch nicht als Quelle auditiert wurden und **nicht** in der ursprünglichen 88er-Liste stehen. Die gesonderte [Transitivziel-Ergänzung](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) dokumentiert **15** solche bisher nicht geprüften MS-/MC-Rohkennungen samt eingehender Normkante, PDF-Fundseite und Hash. Dazu gehören `MS-NSPI` (drei normative Eingänge), `MS-MQDMPR` und `MS-MQQB` (je zwei), `MC-MQSRM`, `MS-EVEN6`, `MS-MEETS` und andere.

**Der kombinierte rohe offene Quellenbestand beträgt somit vor Editionskanonisierung mindestens 44 + 15 = 59 unterschiedliche Bezeichnungen.** Das sind weder 59 bewiesene zusätzliche Skype-Client-Pflichten noch 59 editionsnormalisierte Dokumentidentitäten; weitere Audits können neue Ziele erzeugen.

## Kontrollierte Gesamtbilanz

| Abgegrenztes Prüffeld | Belegter Stand |
|---|---|
| Erste / zweite / dritte normative Quellregister | **464 / 246 / 827** gerichtete Kanten; unverändert |
| Vierte Quellenidentitäts-Ebene | **86/86 klassifiziert**: 85 explizite §1.2.1-Bibliografien plus ein MS-ADLS-Schema-§1.1-Sonderfall; **1.147** Normkanten unverändert |
| Nachgelagerter Originalquellen-Audit | **43** Quellen mit §1.2.1-Bibliografien; **621** gerichtete Normkanten (**442 + 179**) und **161** informative Zitate (**111 + 50**) |
| Anfangs-Rohzielqueue aus vierter Ebene | **88** Bezeichnungen insgesamt; 43 geprüft, 1 Schema klassifiziert, **44** ungeprüfte reguläre Quellen |
| Zusätzlich entstandene transitive Rohzielqueue | **15** weitere ungeprüfte MS-/MC-Zielkennungen, außerhalb der ursprünglichen 88 |
| Schema-§1.1-Zitate | Alte 67 Drittstufen-Nennungen sowie 13 MS-ADLS-Nennungen weiterhin separat **UNCLASSIFIED** |
| Microsoft-Learn-MS-DMRP-Nachtrag | Nur 2026-Webexport geprüft; 2017er Original-PDF rev8.2 noch nicht binär verifiziert und nicht in den 43 Quellen gezählt |
| Formelle Phase-1-Gates | **0/12 PASS**, keine normative Vollabschlussbehauptung |

**Nächster Schritt:** Zuerst die [44 ungeprüften Ursprungs-Kandidaten](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) und die [15 zusätzlichen transitiven Quellenrohziele](fifth-stage-additional-transitive-ms-mc-targets-2026-10-24.json) anhand tatsächlich vorhandener Originale, historischer Zielausgaben und echter PDF-Fundseiten weiter auflösen. Kandidaten umfassen u. a. `MS-OXOCNTC`, `MS-OXODOC`, `MS-OXOFLAG`, `MS-OXOSMIME`, `MS-OXTNEF`, `MS-NSPI` und `MS-MQDMPR`. `MS-DMRP` bleibt bis zur Verifikation des separaten Original-PDFs v20170601/8.2 offen.

**Verbindliche Projektgrenzen:** Kein vollständiger normativer Fixpunkt, keine aufgelöste RFC-/IETF-/ISO-/ITU-/W3C-Zielrekursion, keine verifizierte Produkt-/Build-/Policy-Anwendbarkeit des Skype-for-Business-2019-Windows-Full-Clients. **0/12 Phase-1-Gates PASS**. Keine Protokollimplementierung, keine Live-Netzwerk-/Interoperabilitätstests, keine bezahlten CI-Runner, keine Auth-/TLS-Abschwächung, kein Merge nach `main` und kein Release. Alle Original-PDFs verbleiben beim Nutzer.
