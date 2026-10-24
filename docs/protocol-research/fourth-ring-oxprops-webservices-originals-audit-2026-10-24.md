# P1-03 – Exchange-Property- und Web-Service-Originalbibliografien (Batch 7)

**Belegdatum:** 24. Oktober 2026. **Quellen:** zehn unveränderte PDF-Binärdateien aus dem bereits vom Nutzer bereitgestellten `Exchange_Protocols.zip`. Die Dateibytes, PDF-Fußzeilen `v20250520`, Revisionsübersichten, §1.2.1 und §1.2.2 wurden direkt aus den vollständigen Originalen ausgelesen. Die Referenzstartseiten wurden außerdem gerendert und anhand einer Seitenübersicht visuell geprüft. **Keine PDF-Datei wurde oder wird ins Git-Repository übertragen.**

## Exakte Originale und Ergebnisse

| Dokument | Revision | PDF-Seiten | Vollständiger SHA-256 des Originals | Normativ §1.2.1 | Informativ §1.2.2 |
|---|---:|---:|---|---|---|
| MS-OXPROPS | **30.0** | 377 | `8b8c308ada6bb961f49182707932762f1a97486301e355d9c23f6f7b27cc3a0e` | **59**, S.32–34 | **2**, S.34 |
| MS-OXWSBTRF | **10.0** | 35 | `93e09a5ce1bc28b2fa3853b84ded1518549695b72683fb8fe5caf7bbef184b5b` | **11**, S.6–7 | **3**, S.7 |
| MS-OXWSCEXT | **10.0** | 56 | `34365ed085b52eda0d2152f8f174ae66fb99c1259e9a4c1b3819e398b7255ff3` | **12**, S.7–8 | **3**, S.8 |
| MS-OXWSCONT | **16.0** | 79 | `ab7fd68eecd878934ec1170f43113ca2585c0fe59e350fffa5dbe065055a69f0` | **15**, S.9 | **3**, S.9–10 |
| MS-OXWSCONV | **13.0** | 56 | `63d1fe3cfdc50b5e22681df6cd0ecfca266ed7c4575bdff374cd2e71dd7985df` | **15**, S.6–7 | **2**, S.7 |
| MS-OXWSDLGM | **8.0** | 55 | `bd4fb5915ae3e9374db2bf46e9a070fd59024093aae09fb935305ad6b4e62ad6` | **10**, S.9 | **3**, S.9 |
| MS-OXWSDLIST | **8.0** | 36 | `127513abc5280b8bdac40b9b1672e3ea7f72ef5a67e071d55907f7c232d38bca` | **13**, S.7–8 | **4**, S.8 |
| MS-OXWSEDISC | **9.0** | 74 | `28c1f9bece68a96d18bf01b0d4e11e43192a6673f30a8fa7267e31b7a79efab4` | **16**, S.8 | **4**, S.8–9 |
| MS-OXWSFOLD | **14.0** | 90 | `e261b850cb031200b66d46c2b358abadc7bc579107eaec7c23539f975b77815d` | **19**, S.9–10 | **4**, S.10 |
| MS-OXWSGNI | **8.0** | 37 | `833ea4ac9c85aa0b80b0f9e89e809d758708328f3dcf204ae75da338a00f0a68` | **14**, S.6–7 | **3**, S.7 |
| **Gesamt** | alle `v20250520` / 20.05.2025 | | **10 getrennte Vollfile-Hashes** | **184** | **31** |

[Maschinenlesbare Originalbibliografien mit Quellen-PDF-Seite je Rohreferenz](fourth-ring-oxprops-webservices-originals-audit-2026-10-24.json) · [Viertstufen-Ledger, 690 gerichtete Normkanten](fourth-ring-normative-reference-ledger.json).

## Präzise gerichtete Beziehungen

- **MS-OXPROPS ist nicht nur ein allgemeines Eigenschaftenverzeichnis:** Die `v20250520`-Ausgabe benennt in §1.2.1 auf PDF-S.32–34 **59 explizite normative Referenzen**, darunter `MS-OXABREF`, `MS-OXCRPC`, `MS-OXWAVLS`, `MS-OXRTFEX` und `ISO/IEC8802-3`. Die separaten **informativen** Einträge sind `MS-OXPROTO` und `RFC5234`.
- **Normative gegenseitige Verweise:** `MS-OXWSCONT → MS-OXWSDLIST` (S.9) und `MS-OXWSDLIST → MS-OXWSCONT` (S.7) sind beide normativ. `MS-OXWSCONT → MS-OXWSFOLD` (S.9) und `MS-OXWSFOLD → MS-OXWSCONT` (S.9) ebenfalls. Ein bibliografischer Zyklus ist **keine** Begründung für endlose Implementierungsrekursion.
- **Asymmetrischer Verweis:** `MS-OXWSGNI → MS-OXWSEDISC` steht normativ in MS-OXWSGNI S.6; die umgekehrte Nennung `MS-OXWSEDISC → MS-OXWSGNI` ist in MS-OXWSEDISC S.9 **informativ**. Die beiden Richtungen dürfen nicht gleichgesetzt werden.
- **XML- und Normeditionen:** MS-OXWSEDISC v20250520 nennt `XMLSCHEMA1/2`, `XMLSCHEMA1`, `XMLSCHEMA2/2`, `XMLSCHEMA2` ausdrücklich als verschiedene Rohlabels. MS-OXPROPS enthält außerdem die konkrete ältere Referenz `ISO/IEC8802-3`. Die zitierten Originaleditionen bleiben bis zur separaten Ausgabenprüfung **UNRECONCILED**; es erfolgt keine automatische Ersetzung durch neuere Normen.
- **Produktgeltung offen:** Diese Exchange-Web-Service-Originale aus 2025 beweisen keine universelle Funktionspflicht in einem Skype for Business **2019 Windows Full Client**. Entscheidend sind der damalige Client-Build, die tatsächlich genutzten Server-/Exchange-Features, Richtlinien und unterstützten Versionen. Normbibliografie und operative Client-Mindestanforderung sind getrennt.

## Zähler, offene Evidenz und formelle Gates

Direkt vorher **37/86** Quellen und **506** Viertstufen-Normkanten; nach diesen zehn Quellen **47/86**, **690** gerichtete Viertstufen-Normkanten und **39 offene** Bibliografien. **31** informative Quellen separat gespeichert, nicht normativ. Die vorherigen drei Ringzahlen **464 / 246 / 827** und der getrennte erste Fünftstufen-Beleg `MS-SWN` mit **6** Normkanten bleiben unverändert. Vier AD-Schema-§1.1-Verzeichnisse mit **67 UNCLASSIFIED** Referenznennungen bleiben außerhalb des Normregisters.

**Stand der Schlussprüfung:** 0/12 formelle Phase-1-Gates bestanden; keine editionsgeprüfte vollständige externe RFC-/IETF-/ISO-Rekursion, kein unabhängiger Norm-Fixpunkt, keine belegte vollständige Windows-Full-Client-Produktpflicht. Keine Protokollimplementierung, Live-/Interoperabilitätstests, bezahlten Runner, Änderungen an `main`, Release/Tag oder Schwächung von TLS und Authentisierung. Alle Original-PDFs bleiben beim Nutzer.
