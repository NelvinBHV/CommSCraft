# P1-03 – Nachgelagerte Microsoft-/MC-Quellen: acht Originalbibliografien und reproduzierbare Rohziel-Workqueue

**Stand:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23`. Die vierte Quellenidentitäts-Ebene ist bereits abgeschlossen und wurde **nicht erneut gezählt**. Diese neue Etappe untersucht acht aus ihren normativen Zielkennungen abgeleitete, bisher nicht als Originalbibliografie geprüfte Microsoft-/MC-Spezifikationen. Die erste bereits geprüfte nachgelagerte Quelle `MS-SWN` bleibt separat erhalten.

## Editionsgenaue Belege aus vorhandenen Nutzeroriginalen

Originalarchive `Exchange_Protocols.zip` (sechs Quellen) und `Windows_Protocols.tar.xz` (zwei Quellen), jeweils `[MS-ID].pdf` im passenden ZIP bzw. `Windows_Protocols/[MS-ID].pdf` im TAR. PDF-Binärdateien verbleiben beim Nutzer, nicht im Repository. Für jede **vollständige** PDF wurden SHA-256, Revisionszeile und `vYYYYMMDD`-Fußzeile geprüft; die Abschnitte `§1.2.1 Normative References` und `§1.2.2 Informative References` wurden getrennt gelesen, die normativen Referenzseiten zusätzlich gerendert und visuell kontrolliert.

| Originalquelle | Revision und Edition | PDF-Seiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | Vollfile SHA-256 |
|---|---|---:|---|---|---|
| MS-OXWSADISC | **18.0** / `v20250520` | 101 | **15** (S. 10, 11) | **1** (S. 11) | `76a978279642b5c6fe8f0fa00c67e66852b8a0de2b19e332056aa029be9c6105` |
| MS-DFSNM | **33.0** / `v20240423` | 162 | **19** (S. 14, 15) | **5** (S. 15) | `0639dbafea212c81d778e299c55264552c3c9901b68810c7d4f6247817fe62d6` |
| MS-OXCROPS | **26.0** / `v20250520` | 236 | **14** (S. 17, 18) | **3** (S. 18) | `72f59d674452ba2d25b41fab33209b1ce203d0a7a00fed9b90f69d18f5c21251` |
| MS-OXOABK | **20.0** / `v20250520` | 67 | **28** (S. 11, 12) | **1** (S. 13) | `adee5d069dc6299acc983b051227a980e712429ecfdae11af622aaa32e354598` |
| MS-GPREG | **30.0** / `v20240423` | 81 | **6** (S. 7, 8) | **2** (S. 8) | `d2937f537fc03732904714af832cface36cc6f26a43ec7dbaf7d6bbd868cb96f` |
| MS-OXCDATA | **20.0** / `v20250520` | 137 | **27** (S. 12, 13) | **4** (S. 13) | `110f70deb4425db226b7fb5a9754c143a5b5d97235c64d18e8b7e73cc912915c` |
| MS-OXCFXICS | **26.0** / `v20250520` | 213 | **14** (S. 16) | **8** (S. 16, 17) | `f7245fe634a32a4ad9d07cceb4f2c6bc4e3c9ff264401c3ed5cb35fd80358dff` |
| MS-OXCNOTIF | **23.0** / `v20250520` | 38 | **9** (S. 7, 8) | **4** (S. 8) | `a62a58ebbbeb11bbe41d96538a516f6e47c6a3019927d479444ec3c6c44aa240` |
| **Gesamt** | 8 vollständige Quellen | | **132** | **28** | 8 getrennte geprüfte Hashwerte |

**Primärbelege:** Alle einzelnen Rohlabels mit gerichtetem Absender/Ziel, Original-PDF-Seite, Quellrevision und Hash stehen in [Originalaudit-JSON](fifth-stage-eight-ms-mc-originals-audit-2026-10-24.json) und im fortgeschriebenen [nachgelagerten Normregister](fifth-ring-ms-swn-normative-ledger.json). Die ursprünglichen sechs `MS-SWN`-Normkanten und zwei informativen Quellen wurden nicht neu erzeugt oder überschrieben.

## Fachlich wichtige normative und informative Richtungen

- **Zyklische Exchange-ROP-Abhängigkeiten:** `MS-OXCROPS → MS-OXCDATA` (S.17), `MS-OXCDATA → MS-OXCROPS` (S.12), `MS-OXCROPS → MS-OXCFXICS` (S.18) und `MS-OXCFXICS → MS-OXCROPS` (S.16) sind ausdrückliche normative Bibliografieverweise; auch `MS-OXCDATA` und `MS-OXCFXICS` nennen einander normativ. Diese Zyklen begründen keine Endlosimplementierung oder automatische Voll-Client-Funktionspflicht.
- **Adressbuch-/Exchange-Verweise:** `MS-OXOABK → MS-OXCDATA` (S.11) und `MS-OXCDATA → MS-OXOABK` (S.12) sind ausdrücklich normativ. `MS-OXCDATA → MS-OXOCAL` und `→ MS-OXORMDR` sind dagegen **nur informativ** (S.13). Rohlabels bleiben unverändert und werden nicht ohne Originalnachweis auf andere Editionen umgedeutet.
- **Autodiscover Web Service:** `MS-OXWSADISC v20250520` nennt fünfzehn normative externe RFC-/SOAP-/WS-*- und XML-Ziele, aber `MS-OXPROTO` nur informativ. Die Microsoft-Spezifikation beschreibt die Ermittlung von Konfiguration über eine Autodiscover-Abfrage; das rechtfertigt noch keine automatische Verpflichtung, jeden Web-Service-Endpunkt oder jedes Exchange-Feature im SfB-2019-Windows-Full-Client umzusetzen. Offizielle Referenz: https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwsadisc/b239dc06-c52c-4259-bc49-1ff88d22ae3c
- **DFS Namespace Management:** `MS-DFSNM v20240423 → MS-SMB`, `→ MS-SMB2` und `→ MS-RPCE` normativ (PDF-S.15). Sie beschreiben Protokollbibliografie-Abhängigkeiten von DFS-Verwaltung, keine pauschale Pflicht zur SMB-/DFS-Serverimplementation im Produkt.
- **Registry Group Policy:** `MS-GPREG v20240423 → MS-GPOL` und `→ MS-GPNAP` normativ (S.8), `→ MS-RRP` dagegen nur informativ. Der Mechanismus beschreibt Client-seitige Registrierung und Policy-Verarbeitung von Windows; welche konkreten Einstellungen für Skype for Business 2019 relevant sind, muss build-/policybezogen untersucht werden. Offizielle Referenz: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gpreg/834da877-264f-4589-9b80-b6b012c8edc3
- **Keine automatische Editionsaktualisierung:** `MS-OXWSADISC` unterscheidet u. a. `WS-MetadataExchangeDir`, `WSA10`, `WSADDRCORE`, `WSADDR-Metadata` und `WSAddressing`. `MS-OXCDATA` zitiert die historische Rohkennung `XMLSCHEMA2/2`. Die tatsächliche Normedition eines jeden Zieltextes bleibt **UNRECONCILED**.

## Quellen-Workqueue ab der abgeschlossenen vierten Ebene

Aus dem vierten Register wurden **1.147** gerichtete normative Quellenkanten und ihre Rohziele gegen die zuvor geprüften Quellidentitäten der ersten bis vierten Ebene abgeglichen, einschließlich der **52** Drittstufen-Quellidentitäten (davon vier mit §1.1-Schema-Bibliografien). Dies ergibt **88 MS-/MC-Rohzielkennungen**, die nicht bereits als Quellen der ersten vier Ebenen auditiert waren. Der Abgleich steht mit Rückverweis auf jede eingehende Referenz und ihrem Quellen-PDF-HASH im [reproduzierbaren Workqueue-JSON](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json).

**Status dieser 88 Rohkennungen:** **9** Bibliografien bisher geprüft (die ältere `MS-SWN` und acht neue Originale dieses Batches), **1** Schema-Sonderfall `MS-ADLS` bereits als unklassifizierte §1.1-Bibliografie behandelt, **78** reguläre Ziel-Quellenkennungen weiterhin nicht als Originalbibliografien durchgearbeitet.

**Methodische Einschränkung:** Die 88 sind *aus Viertstufen-Zitaten abgeleitete neue Quellen-Arbeitsziele* und noch **keine mathematisch bewiesene kürzeste fünfte Ebene**. Die Originaleditionen der Zielquellen, redundante Pfade über frühere Stufen und weitere Microsoft- und externe Referenzen sind nicht vollständig kanonisiert. Die Rohzielkennungen können Mehrfach- oder historische Editionsidentitäten umfassen; sie sind weder automatisch 88 erforderliche Clientprotokolle noch 88 neue Downloads.

## Unveränderte ältere Evidenz und formale Sperren

| Bereich | Nach diesem Batch |
|---|---|
| Erste, zweite, dritte normative Ringregister | **464 / 246 / 827 Kanten**, unverändert |
| Vierte Quellebene | **86/86 klassifizierte Identitäten; 85 §1.2.1-Bibliografien; 1.147 gerichtete Normkanten**, unverändert |
| Nachgelagerte MS-/MC-Quellenarbeit | **9 geprüfte Normbibliografien**, **138 gerichtete Normkanten** (bisher 6 + jetzt 132), **30 informative Zitate** (bisher 2 + jetzt 28) |
| Aus Viertstufe neu abzuleitende MS-/MC-Rohziele | **88** Kandidaten insgesamt; **78** reguläre nachgelagerte Quellen noch offen; **1** §1.1-Schema-Ausnahme `MS-ADLS` |
| Frühere Schema-Sonderfälle | Drittring **67** §1.1-Zitate und viertstufiges `MS-ADLS` **13** §1.1-Zitate weiterhin **UNCLASSIFIED** |
| Formelle Phase 1 | **0/12 Gates bestanden** |

**Wiederaufnahmepunkt:** Den nächsten tatsächlich verfügbaren Batch (6–10) aus den **78** unbewerteten MS-/MC-Quellkennungen auswählen, vorzugsweise `MS-OXCPERM`, `MS-OXCSTOR`, `MS-OXNSPI`, `MS-OXOAB`, `MS-OXOCAL`, `MS-OXOMSG`, `MS-OXORMDR`, `MS-WMF`. Zuerst exakte Quelleditionen/Hash/§1.2.1 und §1.2.2 prüfen; danach editionsgeprüft weitere transitive Ziele ableiten. Unabhängig RFC/IETF/ISO/ITU/W3C-Ausgaben und den tatsächlichen 2019-Client-Build-, Serverrollen- und Policystatus verifizieren. **Kein normativer Fixpunkt.**

**Verbote bis zum formellen Abschluss:** keine Protokollimplementierung, keine Live-Netzwerk-/Interop-Tests, keine kostenpflichtigen CI-Runner, keine Abschwächung von TLS/Authentisierung, kein Merge nach `main`, Tag oder Release. Original-PDFs ausschließlich beim Nutzer; Git enthält nur Forschungsbelege und Prüfsummen.
