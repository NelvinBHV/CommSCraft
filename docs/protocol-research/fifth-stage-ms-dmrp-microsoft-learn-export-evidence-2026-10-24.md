# P1-03 – MS-DMRP: Quellenprüfung eines 588-seitigen Microsoft-Learn-PDF-Exports

**Datum:** 24.10.2026 · **Quelle vom Nutzer hochgeladen:** `openspecs-windows_protocols-ms-dmrp.pdf`, **11.726.847 Bytes**, **588 PDF-Seiten**, Vollfile-SHA-256 `24c68c1946f176a2ecf257ee3887c9b12ef02343d8015c2c7c647ebcbc44eac3`. PDF-Metadaten: `creator: Microsoft Learn`, `producer: Microsoft Learn PDF 1.0.26193.03`, `creationDate: 2026-08-14T03:02:59Z`.

## Wichtige Provenienz-Unterscheidung

Dies ist **nicht** das unveränderte, von Microsoft am 01.06.2017 veröffentlichte `[MS-DMRP].pdf` mit Revision **8.2**. Der Dateianfang enthält die Open-Specifications- und Windows-Protokolllisten; erst PDF-S.70 eröffnet der MS-DMRP-Webdokumentationsabschnitt. Dessen ursprünglicher PDF-Download wird im dort gedruckten Dokumentregister als **2017-06-01 / 8.2** genannt. Die einzelnen Microsoft-Learn-Webartikel zu §1.2.1 und §1.2.2 tragen dagegen jeweils das Aktualisierungsdatum **30.10.2020**. Die gesamte Exportdatei wurde **2026** erzeugt. Deshalb sind die 2020er Online-Bibliografien als Quelle belegt, jedoch ist die Identität mit der normativen Bibliografie **der 2017er Original-PDF-Edition nicht nachgewiesen**.

**Offizieller Registereintrag:** https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dmrp/19a16e20-072f-4d74-a245-ce4df2f1ecdd. Der auf Microsoft Learn hinterlegte PDF-Download zeigt auf `https://winprotocoldoc.blob.core.windows.net/productionwindowsarchives/MS-DMRP/%5BMS-DMRP%5D.pdf`. Der Binärabruf dieses offiziellen Release-PDFs war in dieser Untersuchung **nicht erfolgreich**; es liegt deshalb kein verifizierter historischer Release-Binärhash vor.

## Tatsächlich belegte Microsoft-Learn-Abschnitte innerhalb des Exports

| Seite **des Exports** | Abschnitt | Verifizierter Befund |
|---|---|---|
| 70 | `[MS-DMRP]`-Registereintrag | Dokumenttitel, veröffentlichte historische Version 2017-06-01 / **8.2** und Versionshistorie |
| 75 | §1 Einleitung, Webstand 2019-02-14 | Setzt DCOM für Remoteverwaltung von Storageobjekten ein; Abschnitte 1.5, 1.8, 1.9, 2 und 3 werden als normativ bezeichnet |
| **86** | **§1.2.1 Normative References**, Webstand **2020-10-30** | **6** Quellen, siehe unten |
| **87** | **§1.2.2 Informative References**, Webstand **2020-10-30** | **7** Quellen, siehe unten |
| 89 | §1.4 Relationship to Other Protocols | `MS-DMRP` stützt sich auf `MS-DCOM`, das `MS-RPCE` als Unterlage nutzt |
| 91 | §1.6 Applicability Statement | Für Anwendungen, die Datenträger und Volumes **remote konfigurieren** müssen |

**§1.2.1, PDF-Export-S.86, sechs normative Rohlabels:**

`[C706]`, `[MS-DCOM]`, `[MS-DTYP]`, `[MS-ERREF]`, `[MS-RPCE]`, `[RFC2119]`.

**§1.2.2, PDF-Export-S.87, sieben informative Rohlabels:**

`[MS-VDS]`, `[MSDN-AccPerms]`, `[MSDN-DefAccPerms]`, `[MSDN-DISKMAN]`, `[MSDN-PARTITIONINFO]`, `[MSDN-STC]`, `[MSDN-VOLMAN]`.

Beide Seiten wurden direkt aus den Binärbytes extrahiert und als Bilder überprüft. Die maschinenlesbare [JSON-Quellenprüfung](fifth-stage-ms-dmrp-microsoft-learn-export-evidence-2026-10-24.json) dokumentiert rohe Referenzen, Webartikel-Datum, Exportseiten und Herkunftsha.

## Graph- und Phasen-Disposition

Die weiterhin gültige Arbeitsliste [fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) enthält `MS-DMRP` als **offenen Originalquellkandidaten** mit einer eingehenden normativen Viertstufenkante **`MS-CMRP → MS-DMRP`** aus `MS-CMRP v20250811`, Original-S.23, Quell-SHA256 `6406d7c7b350596da2ea3464dd487ecdcb4990f1d614c9aa2302b629698a5ee3`. Diese eingehende Kante war bereits geprüft. Die neu gelesenen sechs Microsoft-Learn-Webreferezen werden **nicht als sechs neue, editionsgebundene Original-PDF-Normkanten** in das nachgelagerte Ledger aufgenommen, bevor das echte `[MS-DMRP].pdf` v20170601 / Revision 8.2 vorliegt und seine Bibliografie (einschließlich eigener PDF-Seiten und vollständigem SHA-256) überprüft wurde.

**Zähler unverändert:** Die nachgelagerte Originalquellenarbeit verbleibt bei **33** vollständig auditierten Normbibliografien, **442** normativen Kanten und **111** separaten informativen Referenzen; die Viertstufe bei **86/86** Quellidentitäten und **1.147** Normkanten. In der rohlabelbasierten Arbeitsliste sind weiterhin **54** reguläre Originalquellen offen, darunter MS-DMRP. Drittstufige 67 und MS-ADLS 13 §1.1-Schemaverweise bleiben **UNCLASSIFIED**. Formelle Phase 1: **0/12 Gates PASS**.

**Benötigtes exaktes Original:** Die einzelne Microsoft-Original-Spezifikation `[MS-DMRP].pdf`, Ausgabe **1. Juni 2017 / v20170601 / Revision 8.2** (nicht ein Webseiten-Gesamtexport), kann über den oben verlinkten offiziellen Microsoft-Registereintrag heruntergeladen und hier bereitgestellt werden. Die restlichen Protokoll-PDFs müssen nicht erneut hochgeladen werden.

**Keine Implementierung, keine Echtserver-Tests, keine CI-Kosten, keine TLS-/Authentisierungsabsenkung, kein `main`-Merge und kein Release. Die Original-PDF-Dateien bleiben ausschließlich beim Nutzer.**
