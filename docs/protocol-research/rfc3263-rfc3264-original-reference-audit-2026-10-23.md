# RFC 3263/3264 – Originalbibliographien und Sicherheitsgrenzen (23.10.2026)

**P1-04/P1-12 – Teilfortschritt, Phase 1 NICHT ABGESCHLOSSEN.** Belegt werden die originalen RFC-Editor-HTML-Bibliographien, **nicht** alle Unterreferenzen/Errata/Versionseffekte, keine Interoperabilität und keine anwendbaren vollständigen 2019-Clientpflichten. [Maschinenlesbares Kantenregister](rfc3263-rfc3264-reference-audit-2026-10-23.json).

## Originalausgaben und disjunkte Referenztypen

| FACT: Quell-Edition | Normativer Abschnitt | Normative Referenz-IDs | Separater informativer Abschnitt |
| --- | --- | --- | --- |
| [RFC 3263](https://www.rfc-editor.org/rfc/rfc3263.html), originaler Dokumentskopf **Juni 2002** | **§11, Druck-S. 14–15: 5** | RFC 3261, 2782, 2915, 2119, 2434 | **§12, S.15: 3** — RFC 2543 und zwei undatierte historische *Work in Progress* (DDDS Part One/Part Three) |
| [RFC 3264](https://www.rfc-editor.org/rfc/rfc3264.html), originaler Dokumentskopf **Juni 2002** | **§14, Druck-S.23–24: 5** | RFC 2327, 2119, 3108, 1889, 1890 | **§15, S.24: 5** — RFC 2974, **3261**, 2326, 2833, 2543 |

**FACT:** Insgesamt **10** originäre, quellspezifische normative RFC-Kanten mit **9** unterschiedlichen normativen Zielen (RFC 2119 doppelt); außerdem **8** strikt separat informative Verweise. **RFC 3261 ist in RFC 3264 §15 informativ, in RFC 3263 §11 normativ.** Die Quellenrolle darf nicht durch einen globalen Zieltyp überschrieben werden. RFC 3263 §12 nennt zwei historische DDDS-Drafts ohne genau aufgelöste Ausgabe. Kein Final-RFC-Ersatz ohne Versionsvergleich.

**FACT / Metadaten aus offizieller [RFC-3263-Infoseite](https://www.rfc-editor.org/info/rfc3263):** *Updated by* RFC 7984 und 8553, *obsoletes* RFC 2543. [RFC-3264-Infoseite](https://www.rfc-editor.org/info/rfc3264): *Updated by* RFC 6157, 8843 und 9143; *obsoletes* RFC 2543. **Achtung Editionsangabe:** die unveränderlichen RFC-Originalkopfzeilen nennen jeweils *Juni 2002*, die Editor-Publikationsdatenbank nennt jeweils *Juli 2002*. Verwende **Kopf-Datum und Katalog-Publikationsdatum als getrennte Felder**, nie als erfundene zwei RFC-Ausgaben.

## Ausgewählte normative Verhaltensklauseln – noch keine vollständige Analyse

- **FACT (RFC 3263, Juni-2002-Original §4.1–§4.3):** Der DNS-Ablauf zur Auswahl des SIP-Zielservers berücksichtigt Transport, Port und Adressauflösung mit SRV/NAPTR. **OPEN:** Welche konkreten Pfade für den klassischen SfB-2019-Full-Client mit konkreter Policy/Serverrolle tatsächlich gelten, ist nicht entschieden.
- **FACT (RFC 3264, Juni-2002-Original §4):** Offer/Answer ist ein atomarer Austausch. Ein Agent darf kein neues Angebot erzeugen, solange sein voriges Angebot noch auf Antwort/Ablehnung wartet oder ein empfangenes Angebot offen ist. Dies ist kein Beweis, dass sämtliche SfB-Medienvarianten identisch implementiert werden.
- **FACT (RFC 3264, Juni-2002-Original §11):** Das übergeordnete Signalisierungsprotokoll braucht Mittel zur End-zu-End-Authentisierung/Integrität sowie zur Erkennung und Ablehnung veralteter Angebote und Antworten; Verschlüsselung der Nachrichtenkörper wird als **SHOULD**, nicht als MUST formuliert. **OPEN:** die konkrete Sicherheitsbindung in SfB-2019/MS-SIPAE/SRTP bleibt nachzuweisen.

## RFC 3268 – geklärte Natur des bibliographischen Monatsunterschieds

**FACT:** [RFC 3261 Original, Juni 2002 §29, Eintrag [6]](https://www.rfc-editor.org/rfc/rfc3261.html) bezeichnet RFC 3268 mit „June 2002“. Der **unveränderliche [RFC-3268-Originaltext](https://www.rfc-editor.org/info/rfc3268)** zeigt tatsächlich selbst im Dokumentkopf **„June 2002“**, während das offizielle RFC-Editor-Feld **„Date published: July 2002“** ausweist. **INFERENCE:** Dies ist ein Unterschied zwischen *gedrucktem Dokumentmonat* und *Katalog-Publikationsmonat*, nicht ein Beleg für zwei verschiedene RFC-3268-Editionen oder unterschiedliche Cipher-Suites. **OPEN:** der administrative Redaktions-/Veröffentlichungsgrund ist nicht separat durch eine RFC-Editor-Historienquelle nachgewiesen. Das ursprüngliche Audit lag richtig bei der beobachteten Datumsabweichung, konnte deren zwei Quellenfelder aber noch nicht unterscheiden.

## Grenzen und offene Knoten

1. **P1-04:** Für alle neun Norm-Ziel-IDs der zwei RFCs: originale Bibliographien, Editionsketten, Errata und Microsoft-Anwendbarkeit weiterprüfen. Nur die erste ausgehende Referenzebene ist hier aus den Original-§§11/14 erfasst.
2. **P1-12:** Nachfolge-RFCs 7984/8553 und 6157/8843/9143 *nicht automatisch* als Pflicht für SfB 2019 substituieren; Versions-/Produktdifferenzen prüfen. Offizieller Errata-Suchzugriff für RFC 3263/3264 in dieser Forschungssitzung nicht erreichbar; kein behauptetes „keine Errata“.
3. **Quellenakquisition:** Keine Original-RFC-Datei in dieser Sitzung binär lokal bezogen/gehasht. DNS-Auflösung im Container für externe Quellen fehlgeschlagen; RFC-Editor-HTML ließ sich im Forschungsbrowser lesen.
4. **Inventartrennung:** Die bestehenden **460** ersten und **246** zweiten Microsoft-Normverweise, die **113** RFC-Seed-IDs, die **146** offenen neuen Ziele und die **0/12** formalen PASS-Gates bleiben unverändert. Das neue Kantenregister ist **eine dritte, gesonderte, noch nicht rekursiv geschlossene Ebene**, keine Erhöhung der Microsoft-Kanten.

**Keine Implementierung, kein Live-Serverkontakt, keine selbständigen Interoperabilitätstests, kein Merge/Release.**
