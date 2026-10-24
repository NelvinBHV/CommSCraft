# P1-03 – Exchange Web Services und historische Windows-Protokolle: zehn normative Originalbibliografien + ein separates AD-LDS-Schema

**Forschungsdatum: 24.10.2026.** Branch `draft/protocol-spec-analysis-2026-10-23`. **Originale des Nutzers** aus `Exchange_Protocols.zip` und `Windows_Protocols.tar.xz`, nicht Git. Vollständige PDF-Bytes per SHA-256 geprüft; für jede Quelle §1.2.1 und §1.2.2 getrennt, einschließlich Originalveröffentlichung, Revisionshistorie, PDF-Seite und Rohlabel. Die Referenzstartseiten wurden als Bilder gerendert und visuell gegengeprüft. Der [maschinelle Originalaudit](fourth-ring-mixed-oxws-windows-originals-audit-2026-10-24.json) enthält **jede** gerichtete Referenz samt Fundseite.

## Zehn reguläre §1.2.1-Originalverzeichnisse

| Original-PDF (ID) | Edition / Revision | Seiten | Normativ §1.2.1 (Originalseite) | Informativ §1.2.2 | SHA-256 der vollständigen PDF |
|---|---|---:|---|---|---|
| MS-OXWSPSNTIF | v20250520 / **8.0** | 27 | **12**, S.7–8 | 3, S.8 | `e2e4c133e0dbde6d639aac9925b47b1c6096e6140fc3f2ce35f8bb176e81d5e3` |
| MS-OXWSRSLNM | v20250520 / **9.0** | 30 | **12**, S.7–8 | 3, S.8 | `feb751338bd82b2fc8f2eba1774918869247b72232d9e6524a083bbe9d11cf46` |
| MS-OXWSTASK | v20250520 / **9.0** | 44 | **11**, S.7–8 | 4, S.8 | `484fd3466f3765d0fa1ea6f4020173590174719443b3cc03da218d92a3bb33d6` |
| MS-OXWSURPT | v20250520 / **6.0** | 33 | **11**, S.7 | 6, S.7–8 | `8f722442979091773740d996a67b668428fe5ab50f9f4e00065316f6bf999fa5` |
| MS-OXWSUSRCFG | v20250520 / **11.0** | 53 | **13**, S.8–9 | 3, S.9 | `317578adf18dfb2cf2f16c221d802266580bbb4c30e1d776a8e435358c29ef25` |
| MS-EERR | v20240423 / **16.0** | 20 | **5**, S.7 | 1, S.7 | `1c6902f609980d4424216844be2b76a75f7a581633f93663f7d6f5813dd015ee` |
| MS-LCID | v20240423 / **16.0** | 61 | **6**, S.6 | 1, S.6 | `f24791d7bd0690fae57351467286c9fcd66ce74cbd6c9db99be1ddc8cea446d5` |
| MS-ODATA | v20190313 / **21.0** | 290 | **27**, S.12–14 | 2, S.14 | `e8c1e07d0a86b6d323bec48d69c8c950aa25dc32a0af7e660b54341f35615b48` |
| MS-RPCL | v20170601 / **12.1** | 77 | **18**, S.11–12 | **0 – „None.“**, S.12 | `f9f410e9a50a5ba25bfb7e006544b224ecfd82a5156160589ebce7c714a4d9f3` |
| MS-W32T | v20240423 / **23.0** | 62 | **7**, S.8–9 | 5, S.9 | `ec0cc8508b6ccd60b483792c6fae87e5d95837451561ac225eb14c5ffa03b915` |
| **GESAMT** | 10 verschiedene Editionen | | **122** | **28** | vollständige Originale verifiziert |

**Herkunft:** Die fünf `MS-OXWS*`-Quellen stammen aus `Exchange_Protocols.zip` mit Archivmitglied `[MS-ID].pdf`. Die anderen fünf stammen aus `Windows_Protocols.tar.xz` unter `Windows_Protocols/[MS-ID].pdf`. Exakte Zuordnung aller Dateien auch in [PDF_ARCHIVE_LOOKUP.md](PDF_ARCHIVE_LOOKUP.md).

## Wichtige Einzelbelege und sicherheitsrelevante Grenzen

- `MS-OXWSPSNTIF §1.2.1 → MS-OXWSNTIF` (S.7) ist normativ. Dies ist ein Quellenverweis im Push-Notifications-Web-Service, **kein pauschaler Beweis** für benötigte Push-Dienste im SfB-Windows-Full-Client.
- `MS-OXWSURPT` führt `MS-OXWSCONV`, `MS-OXWSCORE`, `MS-OXWSFOLD` ausdrücklich **informativ** (§1.2.2, S.7–8). Sie dürfen nicht in die Normkanten dieser Quelle aufgenommen werden.
- `MS-EERR → C706`, `MS-EERR → ISO/IEC-8859-1` und `MS-EERR → MS-RPCE` stehen normativ auf S.7; der `MS-EERR → MS-RPCH`-Verweis ist nur **informativ**. Ausgabe und Reichweite des ISO-/DCE-Ziels benötigen eigene Prüfung.
- `MS-LCID` nennt `ISO-15924`, `ISO-3166`, `ISO-639` und `RFC5646` normativ (S.6). Sprach-/Locale-Referenzen sind keine automatisch belegte Pflicht zur Implementierung sämtlicher Microsoft-Windows-Locale-Dienste in der Anwendung.
- `MS-ODATA` rev 21.0 `v20190313` führt 27 normative Originalziele, u. a. `MC-CSDL`, `MC-EDMX`, `MS-ODATAJSON`, `ODataJSON4.0` und historisch `RFC2616`, `RFC2617`. Eine historische OData-Normbibliografie belegt nicht die vollständige Nutzung von OData durch Skype for Business 2019.
- `MS-RPCL` rev **12.1 / 01.06.2017** enthält 18 normative Referenzen einschließlich `C705`, `C706`, `MS-ADA1`, `MS-ADA3`, `MS-MAIL` und `MS-RPCE`; §1.2.2 auf S.12 lautet ausdrücklich **„None.“**. Die PDF-Fußzeile `[MS-RPCL] - v20170601` ist **keine** normative Selbstreferenz. Microsoft kennzeichnet RPC Location Services Extensions ausdrücklich als Legacy, deprecated und nicht mehr von Microsoft genutzt: [offizielle Spezifikation](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rpcl) und [Einführung](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rpcl/0f2ef67f-7237-48ed-bc68-d4c43cf62809). **Keine Wiederbelebung unsicherer Legacy-Funktionen** allein aufgrund der Normbibliografie.
- `MS-W32T` nennt `MS-RPCE`, `MS-SMB` und `RFC1305` normativ. Auch dies ist nicht gleichzusetzen mit einer Skype-Client-spezifischen Zeitdienst-Implementierungspflicht.

## Besonderheit: MS-ADLS ist ein Schema ohne §1.2.1

Zusätzlich wurde das Original `[MS-ADLS].pdf` aus dem Windows-Archiv direkt gelesen: `v20180912`, Revision **21.1**, **170 PDF-Seiten**, SHA-256 `aa4aee04c2084a25a26863537e67d7635f0f5d36380c32e776fe6bec4e9b8890`. Der gesamte Literaturabschnitt auf PDF-S.**13–14** trägt die Überschrift **„§1.1 References“**, **nicht** „§1.2.1 Normative References“. Es werden **13** Rohreferenzen genannt: `JFIF`, `MS-ADOD`, `MS-ADTS`, `MS-DTYP`, `MSDN-ExtUserIntDirObj`, `MSDN-GroupType`, `MSFT-ADSCHEMA`, `RFC2251`, `RFC2849`, `RFC3280`, `RFC822`, `X121`, `X500`.

Diese 13 Einträge stehen ausschließlich als **UNCLASSIFIED_GENERAL_SCHEMA_BIBLIOGRAPHY** im maschinellen Audit; **null** daraus im gerichteten Normledger. MS-ADLS bleibt vorerst im noch offenen Viertstufen-Frontier, gekennzeichnet als Sondertyp zur separaten Schema-Abschlussprüfung. Die früheren **67** allgemein klassifikationsoffenen Schema-Nennungen aus der **dritten** Ebene bleiben **unverändert**; sie werden nicht stillschweigend mit den 13 zusätzlichen MS-ADLS-Rohzielen vermischt. [Microsofts MS-ADLS-Dokumentregister](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-adls/94279943-25ab-4c13-9bf2-6d411cc2f796) bestätigt die Ausgabe 2018-09-12 / 21.1.

## Kontrollierte Bilanz / nächste Untersuchung

| Ebene | Normative Kanten | Stand |
|---|---:|---|
| Erste | 464 | unverändert |
| Zweite | 246 | unverändert |
| Dritte | 827 | zusätzlich 67 alte Schema-§1.1-Zitate UNCLASSIFIED |
| **Vierte** | **932** | **67/86 echte §1.2.1-Verzeichnisse**, 19 IDs im offenen Frontier inkl. MS-ADLS-Schema |
| Fünfte | 6 | nur separat geprüftes MS-SWN |

Aus **57/86 und 810** Kanten wurden **67/86 und 932** Kanten. **122** explizite §1.2.1-Normkanten neu; **28** §1.2.2-informative Referenzen separat. Zusätzlich 13 §1.1-Schema-Nennungen mit offener Klassifikation.

**Sperren bleiben verbindlich:** Kein vollständiger normativer Fixpunkt, keine geschlossene externe RFC-/ISO-/IETF-Rekursion, keine editionsgeprüfte SfB-2019-Windows-Full-Client-Funktionspflicht, **0/12 Phase-1-Gates PASS**. Kein Protokollcode, keine Live-Tests, kein unautorisierter Runner, kein `main`-Merge, Release oder TLS-/Authentisierungsschutz-Downgrade. Original-PDFs bleiben ausschließlich beim Nutzer.
