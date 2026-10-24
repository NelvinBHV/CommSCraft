# P1-03 – Vierte Microsoft-/MC-Referenzebene: Quellbibliografien vollständig klassifiziert

**Auditdatum:** 24.10.2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23` · **Umfang:** finale acht explizite normative Originalbibliografien und abschließende Behandlung des Schemasonderfalls `MS-ADLS`. **Originaldateien verbleiben beim Nutzer**, keine PDF-Binärdateien in GitHub.

## Abschluss der Quellenidentitäten – kein Abschluss der gesamten Normrekursion

Der ermittelte Ausgangspunkt war **77/86** Originalbibliografien mit explizitem normativen §1.2.1 und **1.061** gerichteten Normkanten. Die letzten acht §1.2.1-Listen erzeugen **86** neue Normkanten und **29** separat informative Literaturnennungen. Damit sind **85/85 Originale mit expliziten §1.2.1-Normbibliografien** geprüft. Die letzte, 86. Quelle, `MS-ADLS`, hat **kein §1.2.1**; ihr §1.1-Referenzverzeichnis wurde separat und abschließend als **UNCLASSIFIED_GENERAL_SCHEMA_BIBLIOGRAPHY** nach Quellenklasse eingeordnet. Es entstehen daraus **keine** gerichteten normativen Literaturkanten. **86/86 Quellenidentitäten klassifiziert; 1.147 gerichtete normative Viertstufenkanten; 0 Quelleidentitäten ohne Audit-Disposition.**

**Das ist nur die Quellbibliografie-Frontier:** Ziel-Originaleditionen, externe RFC-/ISO-/IETF-Normen, weitere Microsoft-/MC-Stufen, Produkteinsatz, Build, Serverrolle, Policy und ein unabhängig nachvollzogener Norm-Fixpunkt sind weiterhin **OPEN**.

## Acht neue originalverifizierte §1.2.1-Bibliografien

Die Originaldateien stammen aus dem vom Nutzer gehaltenen `Windows_Protocols.tar.xz` (exakter Member jeweils `Windows_Protocols/[ID].pdf`). Ganze PDF-Bytes wurden mit SHA-256 geprüft; Revisionshistorie, Fußzeilen `vYYYYMMDD`, §1.2.1 und §1.2.2 sowie Referenz-Fundseiten aus dem Original extrahiert und die normativen Übersichtsseiten gerendert und bildlich gegengeprüft.

| Dokument | Originaledition / Revision | Seiten | Normativ §1.2.1, PDF-Seite(n) | Informativ §1.2.2, PDF-Seite(n) | SHA-256 Vollfile |
|---|---|---:|---|---|---|
| MC-NBFX | `v20230207` / **9.0** | 57 | **10** (7) | **5** (8) | `b23784a43d5848916760c29ce7bd10ec9679893b2cfc7240fd10467e62f1a4a3` |
| MS-CTA | `v20240423` / **8.0** | 20 | **2** (5) | **0** (keine) | `e3d33b8bc96123f7e758be3f325bf240cbfd7985efd70a9ba72f294d48d127bb` |
| MS-DPSP | `v20240423` / **16.0** | 23 | **9** (7, 8) | **4** (8) | `a137e8a85afffeb145398f2652ef8192ba74a94948bd318dbe79002a29cbd4c0` |
| MS-EMFPLUS | `v20240423` / **20.0** | 262 | **14** (18, 19) | **5** (19) | `cb9324b2746706a9f428ef141b9eae2ae92876507789b8b4021ca1ce6fcbf435` |
| MS-MQMQ | `v20240423` / **26.0** | 133 | **22** (15, 16) | **6** (16) | `0060b8c8fd2ed1a4c85af1b41b61445d2265cc1b96819b694c60e50dad5b482d` |
| MS-N2HT | `v20240423` / **14.0** | 18 | **5** (6) | **0** (keine) | `929665158ac447a92f68af56d9c9bb2b60f826148bbefca1bc75285f92121794` |
| MS-PAN | `v20240423` / **19.0** | 91 | **15** (12) | **8** (13) | `4585347bc2e63a433c1bbd7829cae5ee88040f1646eb4a8007d84f6d27775b48` |
| MS-RAI | `v20240423` / **12.0** | 47 | **9** (8, 9) | **1** (9) | `7b7510df5c0f2c1308ceb39ec230d4e8b1417d8e338120cb7b7f23a4a9c912b8` |
| **SUMME** | acht Originale | | **86** | **29** | acht unabhängige Binärdatei-Hashes |

Jede neue gerichtete Kante enthält Rohlabel, Originalseitenzahl, Source-Ausgabe/Revision, PDF-Datei-SHA-256 und Archivpfad im [Viertstufen-Normledger](fourth-ring-normative-reference-ledger.json). Vollständige informative Original-Referenzlisten finden sich **nur separat**, nicht als normative Kanten, im [Audit-JSON](fourth-ring-final-eight-source-bibliographies-audit-2026-10-24.json).

## Gesonderter AD-LDS-Abschlussbefund

- **Original:** `MS-ADLS` – *Active Directory Lightweight Directory Services Schema*, **v20180912 / Revision 21.1**, **170 PDF-Seiten**, SHA-256 **`aa4aee04c2084a25a26863537e67d7635f0f5d36380c32e776fe6bec4e9b8890`**; Archiv: `Windows_Protocols.tar.xz`, Member: `Windows_Protocols/[MS-ADLS].pdf`. Original binär erneut gelesen, Hash gegengeprüft.
- **PDF-S.13–14, §1.1 „References“:** **13 allgemeine Referenzen**, exakt `JFIF`, `MS-ADOD`, `MS-ADTS`, `MS-DTYP`, `MSDN-ExtUserIntDirObj`, `MSDN-GroupType`, `MSFT-ADSCHEMA`, `RFC2251`, `RFC2849`, `RFC3280`, `RFC822`, `X121`, `X500`. Sie sind in der Quelle *nicht* in §1.2.1 normativ versus §1.2.2 informativ unterteilt. Ihre bibliografische Bedeutung bleibt **UNCLASSIFIED**, nicht etwa „13 normative Kanten“ und nicht „13 informative Kanten“.
- **Wichtig:** Das Dokument enthält in **§2 und §3 normative Definitionen der Schemaattribute und Schemaklassen**. Dass die Literaturangaben selbst nicht klassifiziert sind, darf **nicht** als Aussage „MS-ADLS enthält keine normativen Schemaanforderungen“ interpretiert werden.
- **Quellenidentitäts-Status:** `REVIEWED_SCHEMA_ONLY_BIBLIOGRAPHY_UNCLASSIFIED`; aus der Menge offener Quell-IDs herausgenommen, ohne das Dokument fälschlich als normale §1.2.1-Bibliografie zu zählen. Die alten **67** generalisierten §1.1-Verweise aus der **dritten** Ebene bleiben separat **UNCLASSIFIED** und werden nicht mit den 13 ADLS-Verweisen zusammenaddiert.

## Technische und editionsbezogene Beobachtungen

1. **Normative Referenzen sind gerichtet:** `MS-EMFPLUS → MS-EMF` (PDF-S.19), `MS-EMFPLUS → MS-LCID` (S.19), `MS-MQMQ → MS-MQMR` und `MS-MQMQ → MS-MQRR` (S.15), `MS-RAI → MS-RA` und `MS-RAI → MS-RDPBCGR` (S.9) sind explizite Normverweise. Bibliografische Erreichbarkeit ist kein Nachweis einer allgemeinen Skype-for-Business-2019-Funktionspflicht.
2. **Informationsrichtungsfalle bei Druck:** `MS-PAN → MS-RPRN` steht **nur informativ** in MS-PAN §1.2.2, PDF-S.13. Die frühere `MS-RPRN → MS-PAN`-Nennung ist dort ebenfalls informativ. Keine normative Kante aus diesen beiden Bibliografieverweisen konstruieren.
3. **Mailslots, HTTP-Authentisierung und alte Normeditionen:** `MS-DPSP` nennt `RFC2617`, `RFC2829`, `RFC2831` normativ und `DRAFT-DIGESTBIND` nur informativ; `MS-N2HT` nennt `RFC2617` und `RFC4559` normativ, aber **keine** informative Quelle. Historische Authentisierungstexte dürfen nicht ohne Zielversionsprüfung zu einer heute implementierbaren Sicherheitsfreigabe werden.
4. **MC-NBFX v20230207:** historische Labels `IEEE854`, `RFC2279`, `RFC2781`, `RFC3548`, `XML1.0` in §1.2.1 sowie `IEEE754`, `MC-NBFS` und `MC-NBFSE` nur §1.2.2. **Keine** stillschweigende Normersetzung oder Aufwertung informativer Verweise.
5. **MS-CTA und MS-N2HT:** `MS-CTA` listet nur `ISO/IEC-9899` und `RFC2119` normativ; `MS-N2HT` hat fünf normative Verweise und keine informativen. Die PDF-Fußzeilen mit eigenem `[ID] - v...` wurden ausdrücklich ausgeschlossen, nicht als Selbstkanten gespeichert.
6. **Historische Produktgeltung:** Die Edition `MC-NBFX v20230207` und sieben 2024er Windows-Originale enthalten auch später aktualisierte Protokoll- und Standardstände. Eine versionsfremde Rückprojektion auf den Windows Full Client 2019 oder dessen Serverdeployment ist nicht zulässig.

## Ebenenbilanz und Fortsetzung nach Quellabschluss

| Stufe | Gerichtete Normkanten | Quellen-/Auditstatus |
|---|---:|---|
| Erste | 464 | unverändert |
| Zweite | 246 | unverändert |
| Dritte | 827 | unverändert; 67 eigenständige §1.1-Schema-Nennungen UNCLASSIFIED |
| **Vierte** | **1.147** | **85** §1.2.1-Bibliografien geprüft + **1** Schema-§1.1-Dokument eigens klassifiziert = **86/86 Quellidentitäten** |
| Fünfte | 6 | erste separat geprüfte Quelle MS-SWN, restliche Nachfolgefrontier **nicht** abgeschlossen |

**Nächster Wiederaufnahmepunkt:** keine neuen Viertstufen-Quellenidentitäten mehr anfordern. Stattdessen systematisch die **Ziel-Originaleditionen** (inklusive fehlender historischer Microsoft-/MC-, ISO-/RFC-/IETF-/W3C-Editionen) auflösen, die fünfte und weitere transitive Referenzebenen untersuchen, gegebenenfalls die unklassifizierten Schemaverweise methodisch entscheiden und eine unabhängig reproduzierbare normative Abschlussprüfung aufbauen. Parallel die tatsächlich im Skype for Business 2019 Windows Full Client erforderlichen Funktions-/Policy-/Build-Profile mit unabhängigen Quellen belegen. Bibliografiegraph ≠ Client-Spezifikation.

**Formale Sperren:** Phase 1 **0/12 Gates PASS**, kein Norm-Fixpunkt, kein Produktpflichtbeweis. Keine Protokollimplementierung, keine Live-Netz- oder Interoperabilitätstests, keine kostenpflichtigen CI-Runner, kein TLS-/Authentisierungsdowngrade, kein Merge nach `main`, kein Tag oder Release. PDF-Binärdateien bleiben beim Nutzer.
