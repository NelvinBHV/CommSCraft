# RFC 3261 / RFC 2782 Originalquellen: Referenz-Audit 2026-10-23

**P1-04, TEILWEISE. Phase 1 NICHT ABGESCHLOSSEN.** Die originalen RFC-Editor-HTML-Texte und Versions-Metadaten wurden geprüft, keine RFC-Datei binär gehasht. Kein Windows-Client- oder Server-Interoperabilitätsbeweis.

## FACT – Originalausgaben und Referenztypen

- **RFC 3261, SIP: Session Initiation Protocol, Juni 2002, §29, Druckseiten 261–262**, [Original](https://www.rfc-editor.org/rfc/rfc3261.html): 26 explizit normative Referenzen [1]–[26]. §30 ist davon ausdrücklich separat **informativ**. Exakte 26 Referenzkennungen: RFC2327, RFC2119, RFC2822, RFC3263, RFC2396, RFC3268, RFC2279, RFC2616, RFC2806, RFC2234, RFC2046, RFC1750, RFC3264, RFC768, RFC761, RFC2960, RFC2617, RFC2183, RFC3204, RFC1123, RFC2277, RFC1847, RFC2630, RFC2633, RFC2246, RFC2401.
- **RFC 2782, DNS SRV, Februar 2000, unnummerierter Abschnitt “References”, Druckseite 10**, [Original](https://www.rfc-editor.org/rfc/rfc2782.html): 7 verschiedene RFC-IDs (RFC1700, RFC1034, RFC1035, RFC974, RFC2119, RFC2181, RFC2219) und zwei historische Work-in-Progress-Quellen: **ARM** (“Discovering LDAP Services with DNS”) und **KDC-DNS** (“Distributing Kerberos KDC and Realm Information with DNS”). RFC 2119 wird im Original zweimal gelistet und hier einmal gezählt. **Keine normative/informative Zweiteilung im Quellabschnitt; daher alle 9 als unklassifiziert** markiert. Unbekannte Draft-Revisionen bleiben OPEN.

**INFERENCE / Quellgraph:** 26 + 9 = 35 Einträge auf Quell-Dokumentebene mit 34 verschiedenen Zielkennungen (RFC 2119 ist gemeinsames Ziel). Im historischen 113-RFC-Seed fehlen 19 RFC-Kennungen aus RFC 3261 sowie vier weitere aus RFC 2782, insgesamt **23 neue RFC-Zielkandidaten**. Ein Zitat allein verpflichtet den SfB Windows Full Client nicht zur Implementierung. Der ursprüngliche Katalog mit 113 RFC-IDs und die 460/246 Microsoft-Normkanten bleiben unverändert. Maschinelle Kanten: [rfc-first-hop-reference-ledger.json](rfc-first-hop-reference-ledger.json).

## FACT – Versions-/Errata-Metadaten, noch keine Änderungsfolgenanalyse

- [RFC 3261 info](https://www.rfc-editor.org/info/rfc3261): Original Juni 2002, **obsoletes RFC 2543**; **updated by** 21 RFCs: RFC3265, RFC3853, RFC4320, RFC4916, RFC5393, RFC5621, RFC5626, RFC5630, RFC5922, RFC5954, RFC6026, RFC6141, RFC6665, RFC6878, RFC7462, RFC7463, RFC8217, RFC8591, RFC8760, RFC8898, RFC8996. Richtung nicht umkehren.
- [RFC 2782 info](https://www.rfc-editor.org/info/rfc2782): Original Februar 2000, **obsoletes RFC 2052**, **updated by RFC 6335, RFC 8553**. Kein Ersatz der von Microsoft tatsächlich zitierten historischen Ausgabe ohne Diff.
- [RFC 3261 official errata search](https://errata.rfc-editor.org/search/?rfc_number=3261&presentation=records): am 23.10.2026 **12 Verified** im Register sichtbar; beispielhaft ID 316 zu §25.1. **OPEN:** individuelle Bewertung aller Errata einschließlich anderer Status, Auswirkungen auf konkrete Versionsprofile. [RFC 2782 errata search](https://errata.rfc-editor.org/search/?rfc_number=2782&presentation=records) beim Zugriff rate-limited. **Keine Null-Errata-Aussage.**

## OPEN – P1-01/P1-02 Zugriff und nächste Forschung

- MS-ABS, veröffentlichte Revision **10.1 / 2025-08-19**, [Microsoft-Ausgabe](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-abs/ddc59abf-1f42-4935-9b72-5314665dbcff). Kandidat https://officeprotocoldoc.z19.web.core.windows.net/files/MS-ABS/%5BMS-ABS%5D.pdf war im Browser nicht abrufbar; Container-DNS konnte Host nicht auflösen. **Keine Originaldatei, kein Hash, kein §1.2.1-Editionenvergleich**.
- MS-SIPAE, veröffentlichte Revision **11.0 / 2025-05-20**, [Microsoft-Ausgabe](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd). Kandidat https://officeprotocoldoc.z19.web.core.windows.net/files/MS-SIPAE/%5BMS-SIPAE%5D.pdf ebenfalls unzugänglich; vorhandene HTML-Abschnittsfassung ist nicht als PDF-revisionsidentisch belegt.
- **Nächste Schritte:** diese aktuellen PDF/DOCX-Ausgaben in einem autorisierten Zugriffskanal beziehen, Edition/Abschnitt und SHA-256 vergleichen; 23 zusätzliche RFC-Zielkandidaten gegen offizielle RFC-Editor-Originale, normative Folgebezüge, Updates, Obsoletes und Errata prüfen; historische ARM/KDC-DNS-Drafts exakt identifizieren; konkrete SfB-2019-Build-/Server-Policy-Relevanz mit Belegen entscheiden.

**Unveränderte Grenzen:** Keine Protokollimplementierung, keine eigenständigen Live-/Interoperabilitätstests, kein Merge nach main, kein Tag oder Release.
