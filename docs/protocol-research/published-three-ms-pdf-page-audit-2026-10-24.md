# P1-01 – Original-PDF-Bibliografien MS-EUMSDP / MS-TURN / MS-RTP (24.10.2026)

**FACT:** Die drei offiziellen Microsoft-Original-PDF-Ausgaben wurden als öffentlich zugängliche PDF-Webansicht mit datiertem Editions-Footer `v20250520`, Text und Screenshots der Norm- und Informationsreferenzseiten geprüft. Revisionsnummer und Veröffentlichungsdatum jeweils gegen das offizielle Microsoft-Published-Version-Inventar gehalten. **Keine vollständigen PDF-Binärbytes übernommen, keine SHA-256-Prüfung der neuen drei Originale.**

| Dokument | Veröffentlicht / Revision | Umfang | §1.2.1 / §1.2.2 (PDF-S.) | Normativ / informativ |
| --- | --- | ---: | --- | ---: |
| [MS-EUMSDP](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-EUMSDP/%5BMS-EUMSDP%5D.pdf) | 20.05.2025 / 9.0 | 17 | 6 / 6 | 3 / 8 |
| [MS-TURN](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-TURN/%5BMS-TURN%5D.pdf) | 20.05.2025 / 21.0 | 61 | 9–10 / 10 | 11 / 7 |
| [MS-RTP](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTP/%5BMS-RTP%5D.pdf) | 20.05.2025 / 17.0 | 68 | 9–10 / 10 | 10 / 15 |

## Quellenbelegte direkte Referenzen

### MS-EUMSDP – Revision 9.0 (2025-05-20)

- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-EUMSDP/%5BMS-EUMSDP%5D.pdf) **§1.2.1, S. 6**: 3 normative Referenzkennungen: `MS-SDPEXT`, `RFC2119`, `RFC4566`.
- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-EUMSDP/%5BMS-EUMSDP%5D.pdf) **§1.2.2, S. 6**: 8 *informative* Referenzen: `MS-DTMF`, `MS-ICE2`, `MS-ICE`, `MS-QoE`, `MS-RTP`, `MS-SIPRE`, `MS-SRTP`, `MS-TURN`.
- **Revisionsprovenienz:** [offizieller Microsoft-Learn-Eintrag](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-eumsdp/c01cded2-02a3-467a-a959-5ebdc91e542b), aktuelle Revision 9.0, Veröffentlichung 20.05.2025, PDF-Footer `v20250520`. Vollfile-Hash, Versionsdiff und festgelegte 2019-Client-/Server-/Policy-Anwendbarkeit bleiben **OPEN**.

### MS-TURN – Revision 21.0 (2025-05-20)

- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-TURN/%5BMS-TURN%5D.pdf) **§1.2.1, S. 9–10**: 11 normative Referenzkennungen: `draft-ietf-behave-rfc3489bis-02`, `draft-rosenberg-midcom-turn-08`, `RFC1321`, `RFC2119`, `RFC2246`, `RFC2616`, `RFC2818`, `RFC3489`, `RFC4346`, `RFC5246`, `RFC6156`.
- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-TURN/%5BMS-TURN%5D.pdf) **§1.2.2, S. 10**: 7 *informative* Referenzen: `MS-AVEDGEA`, `MS-ICE2`, `MS-ICE`, `RFC3261`, `RFC4566`, `RFC768`, `RFC793`.
- **Revisionsprovenienz:** [offizieller Microsoft-Learn-Eintrag](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-turn/9e434b27-eb13-4249-b031-2d15c3835c8b), aktuelle Revision 21.0, Veröffentlichung 20.05.2025, PDF-Footer `v20250520`. Vollfile-Hash, Versionsdiff und festgelegte 2019-Client-/Server-/Policy-Anwendbarkeit bleiben **OPEN**.

### MS-RTP – Revision 17.0 (2025-05-20)

- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTP/%5BMS-RTP%5D.pdf) **§1.2.1, S. 9–10**: 10 normative Referenzkennungen: `MS-H264PF`, `MS-H26XPF`, `MS-SDPEXT`, `MSFT-H264UCConfig-v1.1-2011`, `RFC2119`, `RFC3550`, `RFC3551`, `RFC4585`, `RFC5285`, `RFC5506`.
- **FACT**, [Original-PDF](https://officeprotocoldoc.z19.web.core.windows.net/files/MS-RTP/%5BMS-RTP%5D.pdf) **§1.2.2, S. 10**: 15 *informative* Referenzen: `abs-send-time`, `MS-DTMF`, `MS-ICE2`, `MS-ICE`, `MS-RTASPF`, `MS-RTPRADEX`, `MS-RTVPF`, `MS-SIPRE`, `MS-SRTP`, `MS-TURNBWM`, `MS-TURN`, `RFC3389`, `RFC4571`, `RFC4733`, `RFC5761`.
- **Revisionsprovenienz:** [offizieller Microsoft-Learn-Eintrag](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-rtp/26dda764-6e96-456b-810b-9ae0275af564), aktuelle Revision 17.0, Veröffentlichung 20.05.2025, PDF-Footer `v20250520`. Vollfile-Hash, Versionsdiff und festgelegte 2019-Client-/Server-/Policy-Anwendbarkeit bleiben **OPEN**.

## Erkenntnisse / offene normative Abhängigkeiten

- **FACT:** MS-TURN §1.2.1 zitiert `draft-ietf-behave-rfc3489bis-02` und `draft-rosenberg-midcom-turn-08` ausdrücklich normativ; RFC 3261 wird erst in §1.2.2 informativ aufgeführt. Keine automatisierte Ersetzung der historischen IETF-Drafts durch spätere RFCs.
- **FACT:** MS-EUMSDP §1.2.1 enthält nur MS-SDPEXT, RFC 2119 und RFC 4566; seine acht weiteren MS-Protokollverweise stehen §1.2.2 informativ.
- **FACT:** MS-RTP §1.2.1 nennt MS-H26XPF und `MSFT-H264UCConfig-v1.1-2011`, während MS-TURN dort in §1.2.2 informativ referenziert ist.
- **INFERENCE:** Die Bibliografien begründen gerichtete Quellkanten, aber keine automatische clientweite Aktivierung aller Medien-/TURN-Funktionen.
- **OPEN:** Der direkte HTTPS-Binärdownload schlug aufgrund DNS-Auflösung des Microsoft-PDF-Hosts fehl; Bytezahl und SHA-256 der drei Volloriginale wurden **nicht** ermittelt. Vorrevisionsdifferenzen, rekursive Unterreferenzen, Errata, Clientbuild und Server-/Policy-Bezug sind weiterhin ungelöst.

**Validierung der drei §1.2.1-Zielmengen:** Bereits im `normative-reference-ledger.json` enthaltene 3+11+10=**24** gerichtete normative Kanten stimmen exakt mit den Originalausgaben überein; keine neuen Kanten erzeugt. Fortschritt **19/46** erste MS-Ebene per aktueller veröffentlichter PDF-Referenzseite, **27 HTML-only**; zweite MS-Ebene **4/12**. Gesamtkanten weiterhin **460 + 246**; formelle Phase-1-Gates **0/12 PASS**. Phase 1 **NICHT ABGESCHLOSSEN**. Keine Protokollimplementierung, Live-/Interoperabilitätstests, `main`-Merge oder Release.
