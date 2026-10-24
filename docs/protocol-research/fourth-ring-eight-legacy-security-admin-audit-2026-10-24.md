# P1-03 – Acht weitere Originalbibliografien: NetBIOS, Remote Mailslot, RAP, GKDI, EFSR, SCMR, GPEF und CMRP

**Stand:** 24.10.2026. **Quellenbasis:** acht vom Nutzer bereits bereitgestellte vollständige Microsoft-Original-PDFs aus `Windows_Protocols.tar.xz`, `Windows_Protocols/[ID].pdf`. **Alle acht** PDF-Dateien wurden binär SHA-256-geprüft, ihre Fußzeilen `vYYYYMMDD` und Revisionshistorienspalten gelesen, normative `§1.2.1` und informative `§1.2.2` separat extrahiert und die ersten Referenzseiten visuell mit gerenderten Originalseiten abgeglichen. Git enthält **nur Forschungsbelege**, keine PDF-Binärdateien. Der Nutzer verwahrt die Originale.

## Quellen/Editionen und §1.2-Referenznachweis

| Originaldokument | Revision / PDF-Ausgabe | Originalseiten | Normativ §1.2.1 | Informativ §1.2.2 | Vollfile SHA-256 |
|---|---|---:|---|---|---|
| [MS-MAIL](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-mail/8ea19aa4-6e5a-4aed-b628-0b5cd75a1ab9) Remote Mailslot | **18.0**, `v20240729` | 26 | **9**, PDF-S. 6–7 | **2**, S. 7 | `83bb6aae4914b3ed8db2cb7f5034fe49af33f6105ec7912cd96b181b209e6565` |
| [MS-NBTE](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nbte/3461cfa8-3d28-4fa3-8163-131bf1046fa3) NBT Extensions | **22.0**, `v20240423` | 31 | **6**, S. 7 | **5**, S. 7–8 | `ea0309d784d119ad412d59600b6cf3f05026cef85cd2e7927fd6814939a13764` |
| [MS-RAP](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rap/ca5d2be3-efe8-4045-a3c8-22d80e265b62) Remote Administration | **33.0**, `v20240916` | 133 | **9**, S. 10 | **5**, S. 10 | `5d6d7d789ae13a4ef145cc84180d92963692b018a42fe599d0b04d258af87607` |
| [MS-GKDI](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gkdi/943dd4f6-6b80-4a66-8594-80df6d2aad0a) Group Key Distribution | **10.0**, `v20240423` | 42 | **19**, S. 8–9 | **6**, S. 9–10 | `ab7911712753bfc11ec7e8b04d211f1cddbd5a69fd12dbe527bd81a35dbb766f` |
| [MS-EFSR](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-efsr) EFSRPC | **32.0**, `v20240916` | 94 | **15**, S. 12 | **14**, S. 12–13 | `e40d0f6f58cde506c423a0c860f739a86b263563dcf6a1a5da43957f640f45b1` |
| [MS-SCMR](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-SCMR/705b624a-13de-43cc-b8a2-99573da3635f) Service Control Manager Remote | **35.0**, `v20250811` | 178 | **8**, S. 11 | **10**, S. 11–12 | `18035f0f40837ac08cd4b725c6c7130cd013bd91eb3f96f7f697930cc688cd4c` |
| [MS-GPEF](https://learn.microsoft.com/en-us/openspecs/windows_protocols/MS-GPEF/14d3fd83-7537-41a2-af39-8e52c19ef0e3) Group Policy EFS Extension | **25.0**, `v20240423` | 37 | **7**, S. 10 | **2**, S. 10 | `2cc2dd0b8f64c351f7b0e838606e79591a3dcf1ce825831a05bdb77384398982` |
| [MS-CMRP](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cmrp/ba4117c0-530e-4e70-a085-4b4cf5bbf193) Failover Cluster ClusAPI | **44.0**, `v20250811` | 662 | **13**, S. 23–24 | **3**, S. 24 | `6406d7c7b350596da2ea3464dd487ecdcb4990f1d614c9aa2302b629698a5ee3` |
| **Summe** | | | **86** | **47** | |

Jeder der **86 gerichteten §1.2.1-Verweise** ist mit seiner exakten PDF-Seite, Referenzlabel, Referenzklasse, Quellrevision, PDF-Erscheinungsdatum und SHA-256 im aktualisierten [Viertstufen-Normledger](fourth-ring-normative-reference-ledger.json) gespeichert. Die **47 §1.2.2-Referenzen** liegen separat mit Quellseiten im [maschinellen Audit](fourth-ring-eight-legacy-security-admin-audit-2026-10-24.json); sie wurden **nicht** zu normativen Kanten erklärt. Fußzeilen-Eigenkennungen wie `[MS-GKDI] - v20240423` wurden beim Extrahieren nicht als Bibliografieziel interpretiert.

## Erkenntnisse und Abgrenzungen

**FACT – explizite Quellverweise:**
- `MS-MAIL → MS-BRWS`, `MS-MAIL → MS-CIFS` und `MS-MAIL → MS-SMB` sind normativ; `MSLOT` und `PIPE` sind im Original `MS-MAIL §1.2.2` **informativ**.
- `MS-RAP → MS-CIFS`, `MS-RAP → MS-SMB`, `MS-RAP → MS-SRVS` normativ; die Produkt-Voraussetzung für RAP umfasst nach `§1.5` ein geeignetes SMB-Dialekt-/IPC$-Szenario. Das belegt **nicht**, dass ein moderner Skype for Business Windows Client das historische Remote Administration Protocol implementieren muss.
- `MS-GKDI → MS-ADTS`, `MS-GKDI → MS-RPCE`, `MS-GKDI → SP800-108` und `MS-GKDI → FIPS180-3` sind normative Originalreferenzen. **Editionstreue beachten:** `FIPS180-3`, `FIPS198-1`, `SP800-56A` und andere historische Normlabels nicht ohne Versionsrecherche in neuere Standards überführen.
- `MS-GPEF → MS-EFSR` und `MS-GPEF → MS-GPOL` sind normativ. Die Group Policy EFS Extension verwaltet EFS-bezogene Richtlinien; `§1.4` ordnet sie dem Group-Policy-Kernprotokoll zu. Eine durchsetzbare Client-Funktion muss separat über konkreten Windows-/Skype-Client-Build, Rolle und Richtlinienstatus belegt werden.
- `MS-CMRP → MS-SCMR`, `MS-CMRP → MS-RPCE`, `MS-CMRP → MS-NLMP` normativ. Die ClusAPI-Spezifikation beschreibt Remote-Verwaltung von Failover-Clustern; das ist **keine** unmittelbar aus der Literaturliste ableitbare Skype-Client-Featurepflicht.
- `MS-SCMR` ist eine RPC-Schnittstelle zur entfernten Dienststeuerung; ohne produktbezogene Belege keine Universalanforderung an die Client-Anwendung.

**Sicherheitsgrenze:** `MS-MAIL §1.6` bezeichnet Remote Mailslot als ungerichtet/unzuverlässig und ohne Authentisierung. Aus bibliografischen Kanten folgt keinerlei Freigabe für ungesicherte Remote-Verarbeitung, SMB1-Erzwingung, NTLM-/Kerberos-Geheimnislogging oder TLS-Zertifikatsumgehung. Vor allem sind Quelle, unterstütztes OS-Verhalten und tatsächliche SfB-2019-Windows-Full-Client-Applikationspflicht streng zu trennen.

**STATUS:** Vorher **19/86** geprüfte Viertstufen-§1.2.1-Verzeichnisse, **300** Kanten; nach diesen acht **27/86**, **386** Kanten, **59** weitere Originalbibliografien offen. Frühe Ebenen **464/246/827** bleiben unverändert; erste getrennte Fünftstufenquelle `MS-SWN`: **6** Normkanten unverändert. Vier AD-Schema-§1.1-Quellen: **67 UNCLASSIFIED** Literaturerwähnungen, weiterhin nicht normativ. Rohlabel-Kanonisierung, historische Zielversionen, RFC-/Draft-/externe Normrekursion, Client-Feature-/Serverrollenbeweis und unabhängiger normativer Fixpunkt bleiben **OPEN**.

**Formal:** Phase 1 **INCOMPLETE, 0/12 Gates PASS**. Keine Protokollimplementierung, keine Echtserver-Interoperabilitätstests, keine kostenpflichtigen CI-Runner, keine TLS-Schutzabsenkung, kein `main`-Merge/Tag/Release. Quellen-PDFs bleiben beim Nutzer; in Git stehen ausschließlich textuelle Belege.
