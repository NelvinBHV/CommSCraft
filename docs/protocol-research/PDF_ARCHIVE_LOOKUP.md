# Archivverzeichnis für alle 824 CommSCraft-Original-PDF-Dateiinstanzen

**Quellenstand:** 24.10.2026 · **Keine PDF-Binärdateien in Git**. Die PDF-Dateien liegen nach Nutzerwunsch beim Auftraggeber.

## Vollständiges Dateiverzeichnis

- **[JSON mit 824 expliziten Pfadzuordnungen](pdf-archive-file-locations-2026-10-24.json)** – je Datei: Dokument-ID, ursprüngliches Archiv, exakter Pfad im Originalarchiv, zugehöriges Teil-ZIP und Pfad im Teil-ZIP.
- **[CSV mit denselben 824 Zuordnungen](pdf-archive-file-locations-2026-10-24.csv)** – schnell per Dokument-ID durchsuchbar, z. B. `MS-SMB2`.
- **Schlüssel:** Die Kombination aus Archivname und Member-Pfad, nicht allein die Dokument-ID. Das ältere separat gelieferte `MS-DLTW` und die andere Windows-Edition bleiben getrennt. 824 Instanzen entsprechen **823** verschiedenen Dokument-IDs.
- **Vollständige PDF-SHA-256-Werte** liegen in den früher an den Nutzer übergebenen Paketmanifesten (z. B. `CommSCraft_Gesamtanalyse_Index_und_Pruefsummen.zip / META/SHA256SUMS.txt` und `META/MANIFEST.json`). Das hiesige Git-Verzeichnis dient der **Zuordnung**; es behauptet keine erneute Inhaltsanalyse aller PDFs.

## Ursprüngliche Nutzerarchive

| Original-Eingang | PDF-Dateien |
|---|---:|
| `OfficeProto.zip` | 74 |
| `Exchange_Protocols.zip` | 129 |
| `SharePointProtocols.zip` | 168 |
| `Windows_Protocols.tar.xz` | 451 |
| Zwei einzeln gelieferte Dateien: `[MS-DLTW]-210625.pdf`, `[MS-DLTM]-170601.pdf` | 2 |
| **Gesamt** | **824** |

## Vier unabhängig geprüfte Teilarchive

| Teilarchiv | PDFs | Prüfsumme (SHA-256 des ZIP) |
|---|---:|---|
| `CommSCraft_Gesamtanalyse_Teil_01.zip` | 263 | `ed76010502fddd2281f1b7dbab3113da88f211ce0cee5f8419cb501752432deb` |
| `CommSCraft_Gesamtanalyse_Teil_02.zip` | 224 | `878daa4151bb99db5742d5ddabc0c6bdcc481ef9b3b8d5bcd82854c209115f4b` |
| `CommSCraft_Gesamtanalyse_Teil_03.zip` | 251 | `9d19feab933d7953cb6b1ed87c7dc5670904d1b4e98e5b14b37064cf32fc81f9` |
| `CommSCraft_Gesamtanalyse_Teil_04.zip` | 86 | `8a0b9a602941537cfb4b58bc7e59a90b694848482c55a6c74e3d08e528d5c1f8` |

**Prüfmethode:** Alle vier ZIP-Dateien erneut gelesen, SHA-256 gegen den vorhandenen Paketindex abgeglichen, vollständigen ZIP-CRC-Test durchgeführt. **824/824 Einträge** sind sowohl einem Originalarchiv als auch einem Teil-ZIP eindeutig zugewiesen. Der aktuelle Content-Hash der PDF-Datei ist jeweils im separaten Nutzer-Manifest dokumentiert.

### Wichtiger Integritätsbefund zum großen Gesamt-ZIP

Die derzeit verfügbare Datei `CommSCraft_Gesamtanalyse_824_PDF_Originale.zip` ist **unvollständig/kein lesbares ZIP**: derzeit **133.465.902** Bytes statt **1.181.972.130** Bytes im früheren Erstellungsindex. Den damaligen erwarteten SHA-256 `82c968aaeabc819fe3dda3a4c50fe7cead07b1d3aa50a51fd85690702f984788` **nicht** dem unvollständigen Archiv zuschreiben. **Verwende bis zur erneuten, bestandenen Prüfung nur die vier Teilarchive.** Dieser Befund ist ein Artefaktproblem, keine Änderung der wissenschaftlichen PDF-Provenienz.

## Konkrete Beispiele für die nächste Forschungsetappe

| PDF-Kennung | Originalarchiv | Verifiziertes Teilarchiv / Pfad |
|---|---|---|
| `MS-SMB2` | `Windows_Protocols.tar.xz` | Teil 03 · `PDFs/Windows/[MS-SMB2].pdf` |
| `MS-FSCC` | `Windows_Protocols.tar.xz` | Teil 03 · `PDFs/Windows/[MS-FSCC].pdf` |
| `MS-RPCH` | `Windows_Protocols.tar.xz` | Teil 03 · `PDFs/Windows/[MS-RPCH].pdf` |
| `MS-TLSP` | `Windows_Protocols.tar.xz` | Teil 04 · `PDFs/Windows/[MS-TLSP].pdf` |
| `MS-GPOL` | `Windows_Protocols.tar.xz` | Teil 03 · `PDFs/Windows/[MS-GPOL].pdf` |
| `MS-GKDI` | `Windows_Protocols.tar.xz` | Teil 03 · `PDFs/Windows/[MS-GKDI].pdf` |
| `MS-OXCMAPIHTTP` | `Exchange_Protocols.zip` | Teil 01 · `PDFs/Exchange/[MS-OXCMAPIHTTP].pdf` |
| `MS-OXWSMSG` | `Exchange_Protocols.zip` | Teil 01 · `PDFs/Exchange/[MS-OXWSMSG].pdf` |

**Arbeitsregel:** Zuerst die JSON-/CSV-Datei nach Kennung durchsuchen, dann das konkrete Teilarchiv und die exakte PDF-Edition verwenden. Der Archivbesitz allein ersetzt keine normative §1.2.1-Prüfung und belegt keine Windows-Full-Client-Implementierungspflicht. Phase 1 bleibt **0/12 formelle Abschlussgates PASS**; keinerlei Protokollcodeänderung, Live-/Interoperabilitätstests, `main`-Merge, Tag oder Release.
