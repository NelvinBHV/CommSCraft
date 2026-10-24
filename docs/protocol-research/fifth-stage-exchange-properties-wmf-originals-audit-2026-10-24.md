# P1-03 – Acht weitere nachgelagerte Exchange-/WMF-Originalbibliografien

**Forschungsdatum:** 24.10.2026. **Draft-Branch:** `draft/protocol-spec-analysis-2026-10-23`. Acht unveränderte Original-PDFs wurden direkt aus den vom Nutzer gehaltenen Archiven `Exchange_Protocols.zip` (sieben Quellen) und `Windows_Protocols.tar.xz` (MS-WMF) geprüft. Die vollständigen Originalbytes wurden mit SHA-256 gehasht, Revisionen und `vYYYYMMDD`-Fußzeilen geprüft, Literaturabschnitte `§1.2.1` und `§1.2.2` getrennt ausgelesen und deren Seitenköpfe gerendert und visuell kontrolliert. **Original-PDFs bleiben beim Nutzer, keine Binärdateien in Git.**

## Originaleditionen, Seiten und getrennte Referenzklassen

| Quelle | Edition / Revision | PDF-Seiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | SHA-256 komplette Original-PDF |
|---|---|---:|---|---|---|
| MS-OXCPERM | `v20250520` / **17.0** | 35 | **10** (7, 8) | **2** (8) | `e8ebcfd832161ad9c2886a4eb380438de085a0c734166501c12ca91952d15a92` |
| MS-OXCSTOR | `v20250520` / **27.0** | 63 | **19** (9, 10) | **1** (10) | `14b33c47cde13c34e79b64a720379e7b3ca2b20dbb4733e92d695525062e554e` |
| MS-OXNSPI | `v20250520` / **15.0** | 94 | **12** (10) | **7** (10, 11) | `daf17ba2c8b49fe05c7b6669d637eee2a785e7960eab525181f78ff9229a9dfc` |
| MS-OXOAB | `v20250520` / **16.0** | 75 | **19** (9, 10) | **2** (10) | `1db7c4e06b82872943b58215bb2c600c7790444debbb9d41e7097a373501eabb` |
| MS-OXOCAL | `v20250819` / **22.1** | 185 | **26** (15) | **2** (16) | `07c286b4a1a4e6a1b6574d66e03f2aa51eb136091c1e19feb948f2e3234dec3c` |
| MS-OXOMSG | `v20250520` / **24.0** | 101 | **29** (15, 16) | **5** (16) | `f8476dc4d37972a331e583e7d7185a6c8a25b9bdfc88594f198261f85ea29c3f` |
| MS-OXORMDR | `v20250520` / **16.0** | 43 | **16** (9) | **3** (9) | `75a0e73d8e8f6dd6cfc1f01c0d067e07013cb4e223fe4da9eb037c243e9432e2` |
| MS-WMF | `v20240423` / **18.0** | 213 | **6** (15, 16) | **2** (16) | `e518ebfd6473d9c80291135499cbbe92ed4da791698785661f466555abc8064a` |
| **Summe** | acht Quellen | | **137** | **24** | acht individuelle Vollfile-Hashes |

**Prüfspur:** [Jeder Originaleintrag mit Rohlabel, PDF-Seite und Dateihash](fifth-stage-exchange-properties-wmf-originals-audit-2026-10-24.json) sowie [fortgeschriebenes nachgelagertes Normledger](fifth-ring-ms-swn-normative-ledger.json). ZIP-Dateipfad `[ID].pdf` bzw. TAR-Dateipfad `Windows_Protocols/[MS-WMF].pdf`; alternative Versionen nicht gleichgesetzt.

## Normative Gerichtetheit, asymmetrische Hinweise und Ausgaben

- **Kalender/Message/Remainder:** `MS-OXOCAL → MS-OXOMSG` (PDF-S.15) und `MS-OXOMSG → MS-OXOCAL` (S.15) sind normativ; `MS-OXOCAL → MS-OXORMDR` (S.15) und `MS-OXORMDR → MS-OXOCAL` (S.9) ebenfalls. Solche Zyklen sind **bibliografisch** und keine Aussage über eine allgemeine Client-Funktion.
- **Eigenschafts-/Speicherabhängigkeiten:** `MS-OXCSTOR → MS-OXCFXICS` und `→ MS-OXCROPS` (S.10), `MS-OXCPERM → MS-OXNSPI` (S.8) sowie `MS-OXOAB → MS-OXNSPI` (S.9) sind explizite §1.2.1-Quellenkanten.
- **Informativ nicht normativ:** `MS-OXOMSG → MS-OXOAB` und `MS-OXOMSG → MS-OXCFOLD` (S.16) stehen nur in **§1.2.2**. `MS-OXORMDR → MS-DTYP` (S.9) ist ebenfalls ausschließlich informativ, selbst wenn andere Dokumente `MS-DTYP` normativ nennen. `MS-WMF → MS-EMF` (S.16) ist **informativ**, nicht normativ.
- **Adressbuch und historische Standards:** `MS-OXNSPI` verweist auf `RFC2119` normativ, aber auf `RFC1510`, `RFC4120`, `RFC4511` nur informativ (PDF-S.10–11). `MS-OXOAB` nennt `ITU-X.690-2008`, `ISO/IEC8802-3`, `RFC2315` und `RFC2459` normativ (S.9–10). Zitierte Editionen dürfen nicht stillschweigend durch moderne RFCs ersetzt werden.
- **Quellversionsabweichung:** `MS-OXOCAL` liegt als `v20250819` / Revision **22.1** vor; die sechs anderen Exchange-Originale haben `v20250520`. `MS-WMF` ist `v20240423` / Revision **18.0**. Diese Quellversionen sind exakt bewahrt. Die **zitierten Zieloriginaleditionen** bleiben **UNRECONCILED**.

## Kontrollierte Bilanz und weitere Arbeit

| Quellebene | Stand |
|---|---|
| Erste bis dritte | **464 / 246 / 827** normative Kanten, unverändert |
| Vierte | **86/86 Quellenidentitäten klassifiziert**, davon 85 normative §1.2.1-Verzeichnisse plus MS-ADLS-Sonderfall; **1.147** gerichtete Normkanten unverändert |
| Nachgelagerte MS-/MC-Quellarbeit (traditionell als fünfte Ebene geführt) | Vorher 9 Originalbibliografien / 138 Normkanten / 30 informative Zitate. **Jetzt 17 / 275 / 54.** |
| Aus Viertstufen-Zielen ermittelte MS-/MC-Rohquellliste | **88** neue Kennungen nach Abzug bereits früher auditierten Quellen. **17** normbibliografisch geprüft, **1** MS-ADLS-Schema-Sonderfall bereits klassifiziert, **70** reguläre Kandidaten offen |
| Sonderreferenzlisten | Alte 67 AD-Schema-§1.1-Zitate und 13 MS-ADLS-§1.1-Zitate weiterhin separat **UNCLASSIFIED** |

Die 88 Rohquellen sind **keine mathematisch bewiesene fünfte Ebene mit kürzesten Entfernungen** und keinesfalls 88 nachgewiesene Implementierungspflichten des Skype-for-Business-2019-Windows-Full-Clients. Auch der Zielversionsabgleich und die transitive externe RFC-/ISO-/IETF-/ITU-/W3C-Rekursion sind offen. Quellenreferenz ≠ Client-Feature-Pflicht.

**Nächster Wiederaufnahmepunkt:** [Aktualisierte Workqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) – 70 ungesichtete reguläre MS-/MC-Rohzielquellen. Je 6–10 exakte Nutzeroriginale prüfen, beginnend mit `MS-XCA`, `MC-CSDL`, `MC-EDMX`, `MS-BKUP`, `MS-DFSRH`, `MS-DLTW` (bereits als eigenständiges Original v20210625 beim Nutzer vorhanden), `MS-DMRP`, `MS-EMF` oder einer archivseitig verfügbaren, zusammenhängenden Gruppe. PDF-Vollhash/Revision/§1.2.1 vs §1.2.2-Seite erfassen, Editionsgeltung und Client-Build/Policy unabhängig belegen.

**Phase 1 nicht abgeschlossen: 0/12 formelle Gates bestanden.** Keine Implementierung, keine Live-Netzwerk-/Interoperabilitätstests, keine kostenpflichtigen CI-Runner, keine Auth-/TLS-Sicherheitsabsenkung, kein Merge nach `main` und kein Release.
