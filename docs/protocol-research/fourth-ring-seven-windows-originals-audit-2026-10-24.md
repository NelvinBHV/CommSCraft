# P1-03 – Sieben weitere Windows-Originalbibliografien der vierten Microsoft-Referenzebene (24.10.2026)

**FACT: tatsächliche vollständige, bereits vom Nutzer gelieferte Original-PDFs** wurden aus `Windows_Protocols.tar.xz` eingelesen. Die Revisionstabellen, `vYYYYMMDD`-Fußzeilen, PDF-§1.2.1/§1.2.2-Texte und gerenderte Referenzseiten wurden geprüft; jede vollständige Datei wurde SHA-256-gehasht. **Die Original-PDFs werden nicht in Git archiviert und verbleiben beim Nutzer.** Dies ist Quellenforschung, kein Interoperabilitätstest.

## Exakt geprüfte Quellenausgaben

| ID | Release Rev. / Ausgabe | Vollfile SHA-256 | §1.2.1 Normativ | §1.2.2 Informativ |
|---|---|---|---|---|
| MS-FSCC | **60.0**, 2025-11-21 / `v20251121`, 243 Seiten | `0828799f05cce35c95dc972bce1c29548602ca4ff171d4aa8c0ff93018491704` | **11**, S.14 | **24**, S.15 ff. |
| MS-GPCAP | **9.0**, 2024-04-23 / `v20240423`, 26 Seiten | `e5f6320761417042cfd39f36404921c38934cb451dee4d8e1f10e8319d685295` | **8**, S.8–9 | **2**, S.9 |
| MS-GPOL | **39.0**, 2024-04-23 / `v20240423`, 85 Seiten | `c1151ad9b40abff7bf4ec60340459024d2b86acd8de05115ec554c248983a176` | **28**, S.14–15 | **9**, S.15 |
| MS-GPSB | **25.0**, 2024-04-23 / `v20240423`, 57 Seiten | `e48d5e2744153e4c9d6d84a614c9803a637784dfdfc3de16bb58b6542c8e59c0` | **14**, S.9 | **2**, S.9 |
| MS-RPCH | **22.0**, 2024-07-29 / `v20240729`, 154 Seiten | `3c6d080a7c04e8514d584a44672cbb0fcd347620b53ddc39c950f9461d91e828` | **17**, S.15 | **5**, S.16 |
| MS-SMB2 | **88.0**, 2026-09-28 / `v20260928`, 499 Seiten | `fef3a06507679130a8cfdcaeaed581482d7f235435dd904f6c794f113ff04bd1` | **36**, S.20–21 (Abschnittsbeginn S.19) | **34**, S.21 ff. |
| MS-TLSP | **18.0**, 2026-09-28 / `v20260928`, 19 Seiten | `3c64be5d95e1867d86f636f4813b578c73bf201363a6a2cd226dbc6c7c9f0372` | **21**, S.5–7 | **14**, S.7 |

**Normative Summe 11+8+28+14+17+36+21 = 135 neue gerichtete Quellkanten.** Informative Summe **90**, bewusst getrennt und **nicht** in den Normgraphen eingerechnet. Alle normativen Einzelkennungen samt individueller PDF-Seite, Quellrevision und Binär-SHA-256 stehen im aktualisierten [Viertstufen-Normledger](fourth-ring-normative-reference-ledger.json). Alle Quellen und beide Klassen stehen zusätzlich im [strukturierten Quellenbericht](fourth-ring-seven-windows-originals-audit-2026-10-24.json).

## Nachgebesserte Original-Provenienz aus dem früheren Exchange-Batch

Die vier bereits zuvor exakt an ihren Microsoft-Originalseiten bibliografisch geprüften `v20250520`-PDFs wurden nun **als vollständige Dateibytes aus `Exchange_Protocols.zip`** geprüft, ohne deren bestehende **54** normative Kanten zu verändern:

| Quelle | Vollständiger PDF-SHA-256 |
|---|---|
| MS-OXCMAPIHTTP 15.0 | `246bf51780a5ed370422e060dd98785bae76325f88587d2ec708bcbb8a2c8d70` |
| MS-OXWSSYNC 10.0 | `c83791a19fdc70c733b04d0fd1bd20dd3b413bb3fc3f828f61ebbdf16f7f3311` |
| MS-OXWSXPROP 13.0 | `0ea61a56ce4332871ed9543d9a799873d2b950389b76865a71667d979e69443d` |
| MS-OXWSMSG 15.0 | `e8371f4b6ea07ae0d7afbac18154a7bb96146d7b921d1cb7e6267e3e35092fc7` |

Damit werden aus 5/86 **12/86** exakt geprüfte Viertstufen-Normverzeichnisse, aus 64 **199** direkte Viertstufen-Normkanten und **74** Dokumentbibliografien bleiben offen. Die drei vorgelagerten separaten Ringzahlen **464 / 246 / 827** sowie die 67 `UNCLASSIFIED`-AD-Schemazitate bleiben unverändert.

## Offene Zielkanonisierung, historische Sicherheit und Produktgeltung

- **FACT:** MS-TLSP rev 18.0 führt ausdrücklich historische TLS RFC 2246/4346/5246, TLS 1.3 RFC 8446 sowie die **genau bezeichneten** Draft-Versionen `DRAFT-IETF-TLS-MLDSA-05`, `DRAFT-IETF-TLS-MLKEM-09` und `IETFDRAFT-TLS-ECDHE-MLKEM-04`. Nicht als gleichwertig oder automatisch aktualisiert interpretieren. Ein historischer TLS-Referenzeintrag ist **keine Freigabe für TLS 1.0/1.1, SSL, unsichere Cipher oder eine Zertifikatsprüfung-Abschwächung**.
- **FACT:** MS-SMB2 rev 88.0 ist eine **2026er** Ausgabe und referenziert u.a. RFC 8446 und RFC 9000; Revisionen von 2026 sind **nicht** ohne produktbezogene Begründung für einen Skype for Business 2019 Windows Full Client bindend. Die historischen 2019-Editionen/Diffs bleiben OPEN.
- **FACT:** MS-GPCAP und MS-GPSB betreffen zentrale Zugriffsrichtlinien bzw. Security-Gruppenrichtlinien. Bibliografische Erreichbarkeit allein beweist nicht, dass der zu untersuchende SfB-Windows-Full-Client selbst jede solche Administrative-Funktion implementieren muss.
- **OFFEN:** Klassifikation/Edition jedes neu benannten RFC-/IETF-Draft-/ISO-/W3C-Ziels, genaue Erstgeltung nach Windows-Client- und Server-Build, rekursiver Norm-Fixpunkt, vollständige normbezogene Exklusionen. Rohe Referenzkennungen werden vorerst als `RAW_LABEL_ONLY_UNRECONCILED_TARGET_EDITION` geführt, nicht stillschweigend auf aktuelle RFCs umbenannt.
- Das bereits im Bestand befindliche `MS-GKDI` wurde zusätzlich als PDF erkannt, **nicht** als neuer Quellbeleg der aktuellen 86er Viertstufen-Frontier gezählt, weil diese Dokument-ID dort **nicht** enthalten ist. Dies vermeidet eine falsche Überhöhung der 12/86.
- Die Quellen sind beim Nutzer zu finden über `PDF_ARCHIVE_LOOKUP.md`: sieben Windows-Originale in `Windows_Protocols.tar.xz` / aufgeteilten Gesamtarchiven, vier Hashnachweise in `Exchange_Protocols.zip`.

**Abschluss:** Formelle Phase-1-Gates **0/12 PASS**, Phase 1 **INCOMPLETE**. Keine Implementierung, keine Live-/Interoperabilitätstests, kein `main`-Merge, kein Tag und kein Release.
