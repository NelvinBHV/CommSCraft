# CommSCraft – verbindlicher Forschungsstand und Wiederaufnahmepunkt

**Stand: 23.10.2026 · Repository:** [NelvinBHV/CommSCraft](https://github.com/NelvinBHV/CommSCraft) · **Arbeitsbranch:** `draft/protocol-spec-analysis-2026-10-23` · **Phase 1: NICHT ABGESCHLOSSEN**

> **WICHTIG FÜR SPÄTERE ARBEITEN:** Dieser Text ist der Einstiegspunkt für die Fortsetzung. Er fasst den belegten Stand der Dokumentation zusammen, ersetzt aber **nicht** die verlinkten Quellenregister. Die vollständig enumerierte **erste Referenzebene (46/46)** ist **keine vollständig aufgelöste normative Abhängigkeitskette** und erst recht keine Aussage über Client-Funktionalität oder Protokollkonformität.

## 0. Auftrag, Schutzvorgaben und Geltungsbereich

Ziel ist die vollständige, revisionsgenaue Analyse **aller für den klassischen Skype for Business 2019 Windows Full Client** und seine Funktion mit Skype for Business Server 2019/Subscription Edition relevanten RFCs, Microsoft Open Specifications, Normverweise, Nachrichten-/Zustandsmodelle, Sicherheits- und Fehlermechanismen. **Erst nach belegtem Abschluss der Dokumentations- und Analysearbeit** ist eine spätere Implementierungsplanung zulässig.

- **Verboten bis zur expliziten Freigabe:** neue Protokollimplementierungen oder deren Umsetzung; Echtserververbindungen; eigenständig ausgeführte Interoperabilitätstests; Merge nach `main`; Tag/Release. **Nur der Auftraggeber führt Live-/Interoperabilitätstests durch.**
- Keine Authentifizierungs-, TLS- oder Protokolldetails erfinden. Jede relevante Aussage mit **Dokument-ID, offizieller URL, Revision, Veröffentlichungsdatum und genauem Abschnitt** sowie `FACT`, `INFERENCE` oder `OPEN` kennzeichnen.
- Frühere Offline-Test-/MIC-/SIP-200-Evidenz ist kein unabhängiger Nachweis für eine erfolgreiche authentifizierte Skype-Anmeldung.
- Schutzregeln aus [`AGENTS.md`](../../AGENTS.md) einhalten: Zertifikatsprüfung nicht abschalten, keine stillschweigende Klartext-Rückstufung, keine Authentifizierungsgeheimnisse protokollieren, Fremdeingaben als feindlich behandeln.
- **Produktvarianten:** der tatsächliche Windows-Full-Client-Build, die Server-Policy-Konfiguration und die konkrete Serverrolle sind weiterhin **nicht abschließend festgelegt**; nicht ohne Begründung von einer Funktion auf universelle Clientpflicht schließen.

## 1. Letzter nachgewiesener Basisstand

Vor diesem Status-Handoff: Draft-Commit `d460ef52a7eeb80d4e7bc4cc1355ae490f7d6102`; unveränderter beobachteter `main`-Commit `fd4e48b0b993ed170308943adf3cf8cf1ae8f6a0`. **Diese Werte sind historische Anker; bei jeder Fortsetzung den neuen Branch-HEAD und `main` erneut prüfen.** Die Git-Blob-SHAs der fünf wichtigsten Quellregister wurden gesondert in [`research-checkpoint.json`](research-checkpoint.json) erfasst, damit spätere Änderungen nachvollziehbar bleiben.

| Thema / Metrik | Verifizierter Snapshot | Bedeutung und Einschränkung |
| --- | ---: | --- |
| MS-OCSPROT-Protokollfamilien | **6/6** | Überblick rev **9.0**, veröffentlicht **2025-05-20**; diese Listen allein sind nicht die gesamte Normenkette |
| Einträge in diesen sechs Familien | **46** | Jeweils ID, offizielle Microsoft-Seite und aktuell veröffentlichte Revision/Datum katalogisiert |
| Direkte §1.2.1-Normverweislisten in irgendeiner Quellenform erfasst | **46/46** | **Erststufen-Inventur** abgeschlossen, nicht vollständige Release-Revisionsprüfung |
| §1.2.1 durch datierte Original-PDF-Referenzseiten unterstützt | **13** | Davon **12** aus indizierten PDF-Seitenausschnitten, **1** vollständige hochgeladene Original-PDF |
| §1.2.1 nur in Live-HTML gesehen | **33** | Aktuelle veröffentlichte PDF-Ausgabe nicht durchgängig abgeglichen; HTML kann Jahre älter sein |
| Vollständige Original-PDFs mit Dateihash nachgewiesen | **1** | Nur MS-AVEDGEA 12.0, Nutzereingabe; die Originaldatei ist **nicht Teil des Git-Repositories** |
| Dokumentierte normative direkte Quell-Ziel-Kanten | **460** | Davon 120 Microsoft-, 228 RFC-, 29 historische IETF-Draft- und 83 externe Normverweise |
| Verschiedene normative Referenz-Zielkennungen | **199** | **Nicht rekursiv abgeschlossen**, enthält auch W3C/ISO/andere Dokumente |
| Erfasste RFC-Kennungen (Kandidaten + direkt) | **113** | Davon **89** in bereits eingesehenen MS-Normverweislisten unmittelbar genannt; RFC-Normverweise und Updates-Ketten offen |
| Zusätzlich erkannte Microsoft-Dokumente außerhalb der sechs Tabellen | **12** | Neueste Versionen/Daten bekannt, anwendbare historische Revisionen und ihre Normverweise nicht vollständig geklärt |
| Ursprüngliche Windows-Funktionsinventur | **122 Funktionen / 93 Szenarien** | Bestandskennungen, **keine** normativ nachgewiesene Vollabdeckung; **31 Funktionen** ohne direkt verknüpftes Windows-Referenzszenario |

Die Zählwerte sind aus [`ms-ocsprot-inventory.json`](ms-ocsprot-inventory.json), [`normative-reference-ledger.json`](normative-reference-ledger.json), [`rfc-inventory.json`](rfc-inventory.json), [`transitive-microsoft-inventory.json`](transitive-microsoft-inventory.json) und [`full-122x93-traceability.md`](full-122x93-traceability.md) zum Snapshot 2026-10-23 abgeleitet. **Bei späteren Ergänzungen Werte neu berechnen und nicht einfach übernehmen.**

## 2. Gesicherter Fortschritt: MS-AVEDGEA 12.0

Das bisher fehlende aktuelle Original wurde vom Nutzer als `[MS-AVEDGEA].pdf` bereitgestellt und überprüft:

| Provenienz | Verifizierter Wert |
| --- | --- |
| Offizielle Spezifikation | **MS-AVEDGEA – Audio Video Edge Authentication Protocol** |
| Veröffentlichte Ausgabe | **v20250520**, **Revision 12.0**, **20.05.2025** |
| Umfang | **40 Seiten**, **1.562.486 Bytes** |
| SHA-256 der hochgeladenen Datei | `baa3cddc1e1bfb4b0c2b0a998c71b802f80d4e5e6bb30adc0d8d42ebbbb0c8ff` |
| Normative Quellen | **§1.2.1, S. 7: 5** – `draft-deason-sip-soap-00`, RFC 2119, XML Namespaces Third Edition (2009), XML Schema Part 1 und Part 2, Second Edition (2004) |
| Informative Quellen | **§1.2.2, S. 8: 5** – MS-ICE2, MS-ICE, MS-SIPRE, MS-TURN, XML 1.0 Third Edition (2004) |
| Produktbezug | **Anhang B / §7, S. 36** nennt Skype for Business 2019, Skype for Business Server 2019 und Server Subscription Edition |
| Änderungshistorie | **Revisionstabelle S. 3**, **§8 S. 38**; Versionsvergleich 11.0→12.0 **nicht** vollständig als Binär-/Inhaltsdiff abgeschlossen |

**Wesentliche Klassifikationsregel:** MS-ICE, MS-ICE2, MS-SIPRE und MS-TURN sind **informative Referenzen in §1.2.2**, auch wenn sie technische Abhängigkeiten darstellen. Sie dürfen **nicht** fälschlich zu normativen §1.2.1-Kanten umklassifiziert werden. Die 5 Normkanten und 5 separaten informativen Kanten sind in der Datenbank eingetragen.

Quellen und weitere Einzelnachweise: [`ms-avedgea-12-source-audit.md`](ms-avedgea-12-source-audit.md), offizielle [MS-AVEDGEA-Versionseintragung](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-avedgea/56860f6a-2fcf-4cad-8ee5-466d8f382a45).

**Provenienzrisiko:** Die PDF wurde **nicht nach GitHub committet**. Die Datei kann bei einem zukünftigen Auftrag außerhalb dieses Chats fehlen. Dann das Original erneut zur Verfügung stellen oder aus einer autoritativen Quelle beziehen und seinen SHA-256-Wert abgleichen. Nie ohne Original so tun, als sei die neue Datei erneut verifiziert worden.

## 3. Stand je Arbeitsphase

| Phase | Status | Was erreicht ist / was fehlt |
| --- | --- | --- |
| **1 – vollständige Quellenakquisition** | **NICHT ABGESCHLOSSEN** | MS-OCSPROT: 46/46 direkte Normlisten in irgendeiner Form eingesehen, aber nur 13 durch datierte PDF-Referenzseiten belegt; **33 HTML-only**; vollständige aktuelle Originale, Querverweis-Rekursion, RFC-Updates/Normverweise, History-Diffs, Fremdstandards und relevante Client-Build-Profile offen |
| **2 – Vollanalyse jedes Dokuments** | **TEILWEISE / ausgewählte Klauseln** | Einzelanalysen zu MS-CONMGMT, MS-SIPREGE, MS-SIPAE, RFC3261, RFC2782, MS-AVEDGEA; **nicht** alle Header, Algorithmen, Zustandsautomaten, Timer, Fehler und Negativfälle aller Dokumente |
| **3 – Protokollgesamtsystem** | **VORLÄUFIG** | Szenario-/Zustandsdiagramme für Start, TLS, Auth, REGISTER, Präsenz/IM, Medien/Konferenzen und Reconnect vorhanden; viele Diagrammübergänge explizit als Inferenz/ungeprüft markiert |
| **4 – Vollständige Auswertung / Implementierungsplan** | **OFFEN** | Kein freigegebener quellengestützter Implementierungsplan, keine vollständige Anforderungsvalidierung |

**Kein Gesamtabhebezustand oder Client-Paritätsnachweis darf allein aus Quellennamen, Offline-Tests oder Beispielabläufen abgeleitet werden.**

## 4. Noch zu erledigende Arbeit – priorisiert und eindeutig

1. **P1-01 – Revisionsgenauigkeit:** Für die **33 HTML-only** MS-§1.2.1-Seiten die jeweils **veröffentlichte aktuelle Original-PDF/DOCX-Ausgabe** beschaffen, Version/Datum/Abschnitt abgleichen und jede Abweichung dokumentieren. Der Abschluss der Erststufenerfassung bleibt erhalten; **nicht** fälschlich als Release-Prüfung markieren.
2. **P1-02 – Vollständige Quellenerfassung:** Relevante Microsoft-Originaldokumente inklusive Vorversionen beschaffen, Quellen-URL, Dateiformat, Größe, Prüfsumme, Ausgabedatum, Revision, Änderungsverlauf und Zugriffshindernisse festhalten. **Nicht nachgewiesene** Quellen explizit OPEN lassen. Bisher nur die eine MS-AVEDGEA-Binärdatei verifiziert.
3. **P1-03 – Microsoft-Abhängigkeiten rekursiv:** Für die **12** direkt gefundenen zusätzlichen Windows-/Exchange-/SharePoint-Dokumente und weitere neu entstehende Knoten die normativen Verweise mit exakten Ausgaben auflösen; jede clientrelevante Kante weiterverfolgen. Auslassungen als begründete *not applicable*-Entscheidung belegen statt stillschweigend zu ignorieren. Einstieg: [`transitive-microsoft-inventory.md`](transitive-microsoft-inventory.md).
4. **P1-04 – RFC-Referenzgraph:** Für alle einschlägigen RFCs Originaldatum, normative References, Errata, `Updates / Updated by`, `Obsoletes / Obsoleted by` prüfen. Auch **historische IETF-Drafts** (z. B. SIP-GRUU, ICE, SIP-Outbound, SIP-SOAP, OAuth) versionsgenau erfassen; keine moderne RFC an Stelle des tatsächlich von Microsoft zitierten Drafts setzen.
5. **P1-05 – Externe Standards:** ISO 3166-1:2013, ISO 8601:2004, ITU E.164:2005 sowie weitere W3C-/OASIS-/IEEE-/SMPTE-/ISO-Spezifikationen mit exakter Ausgabe, normativer Relevanz und ggf. Bezahl-/Zugriffsblocker dokumentieren.
6. **P1-06 – Produktversion und optionale Funktionen:** Exakten Windows-Client-Build, Server-Edition, Rollen, Konfiguration und Richtlinienbezug festlegen; Zuordnung jeder relevanten Funktion/Quelle zu den **122 Features und 93 Referenzszenarien** prüfen. Insbesondere alle 31 Funktionen ohne direktes Szenario entscheiden.
7. **P1-07 – Versionskonflikte:** Versionen seit MS-OCSPROT 9.0 (u. a. MS-ABS 10.1, MS-SIPREGE 17.1, MS-SSRTP 11.1), historische RFCs und MS-Spezifikationen einschließlich relevanter **2026-Windows-Protokollrevisionen** auf produktbezogene Unterschiede prüfen.
8. **P1-08 – Nachweisbarer Abschluss:** Referenzgraph bis zum relevanten Fixpunkt prüfen; alle offenen Zugriffe/Fragen und dokumentierten Ausschlüsse entscheiden; Vollständigkeit mit Ausgangsdateien und Änderungsnachweisen kontrollieren. **Nur danach Phase 1 als abgeschlossen markieren**, und erst nach den späteren Phasen Implementierungsplanung erwägen.

Diese Punkte beschreiben ausschließlich die **noch ausstehende Forschungs-/Dokumentationsarbeit**, **keinen** Auftrag zur Protokollentwicklung oder zu Tests.

## 5. Quellen- und Arbeitsdateien: wo künftige Arbeiten beginnen

| Datei | Funktion |
| --- | --- |
| [`README.md`](README.md) | Forschungsregeln, Scope und Gesamtstruktur |
| [`research-checkpoint.json`](research-checkpoint.json) | Maschinenlesbare Momentaufnahme, Baseline-Commit/Blob-SHAs, Status und Arbeitsblocker |
| [`phase1-audit.md`](phase1-audit.md) | Formelles Phase-1-Gate mit offener Quellen- und Versionsprüfung |
| [`source-catalog.md`](source-catalog.md) | Zentrale verlinkte Microsoft-/RFC-Quellen- und Versionsübersicht |
| [`ms-ocsprot-inventory.json`](ms-ocsprot-inventory.json) | 46 Microsoft-Protokolldokumente, offizielle Landings und Einzelquellenstatus |
| [`normative-reference-ledger.json`](normative-reference-ledger.json) | 460 direkte normative Kanten und 5 separat gespeicherte MS-AVEDGEA-informative Kanten |
| [`rfc-inventory.json`](rfc-inventory.json) | 113 RFC-Originalkennungen, Quellen-URLs, direkter/indirekter Kandidatenstatus |
| [`transitive-microsoft-inventory.json`](transitive-microsoft-inventory.json) | 12 zusätzliche Microsoft-Protokolldokumente mit aktuellen Publikationsmetadaten |
| [`ms-avedgea-12-source-audit.md`](ms-avedgea-12-source-audit.md) | Prüfprotokoll des hochgeladenen, vollständig gehashten Original-PDFs |
| [`full-122x93-traceability.md`](full-122x93-traceability.md) | Bestandsverbindung der 122 Features zu 93 Windows-Referenzfällen |
| [`discovery-registration-analysis.md`](discovery-registration-analysis.md), [`sipae-authentication-clause-review.md`](sipae-authentication-clause-review.md), [`rfc3261-rfc2782-clause-review.md`](rfc3261-rfc2782-clause-review.md), [`flows-and-states.md`](flows-and-states.md) | Bisherige **teilweise** technische Auswertung, keine vollständige Normenprüfung |
| [`research-log.md`](research-log.md) | Chronologische Arbeitsetappen; **jüngster Eintrag gilt vor älteren Zwischenzahlen** |

## 6. Wiederaufnahmeanleitung (nächster Auftrag)

1. `git fetch`, dann auf **`draft/protocol-spec-analysis-2026-10-23`** arbeiten. Zuerst `AGENTS.md`, **diese Datei**, `phase1-audit.md`, `research-checkpoint.json` und die letzten Einträge im `research-log.md` lesen. **Nicht** mit `main` überschreiben oder mergen.
2. Vor jeder Änderung den tatsächlichen Branch-HEAD und die JSON-Kataloge prüfen. Die oben stehenden Kennzahlen aus den Dateien **neu berechnen**, insbesondere normative vs. informative Kanten, eindeutige MS-IDs/RFC-IDs und Revisionen.
3. Den **höchstpriorisierten unbelegten** Punkt aus §4 bearbeiten: exakte Primärquelle finden/lesen, Beleg bis zu relevanter Abschnittsebene erfassen, Zweifelsfälle offen benennen; weiterhin keine Protokollimplementierungen oder Live-Tests.
4. Pro Quellenabschnitt den Status getrennt aktualisieren: *veröffentlichtes Metadatum*, *HTML sichtbar*, *publizierte PDF-Seiten geprüft*, *Originaldatei gehasht*, *Normverweise geschlossen*, *Client-Anwendbarkeit entschieden*, *Versionsvergleich durchgeführt*. Eine niedrigere Evidenzstufe nie pauschal zu einer höheren aufwerten.
5. Die Quellregister + `phase1-audit.md` aktualisieren, diesen Status/Checkpoint bei Zahlenänderungen anpassen, einen datierten Eintrag im `research-log.md` und einen **reinen Dokumentations-Commit** auf dem Draft-Branch erstellen. Keine Ergebnisse als vollständig melden, bevor die Abschlussbedingungen erfüllt sind.
6. Für Interoperabilität/realen Serverkontakt ausschließlich **vom Auftraggeber bereitgestellte und ggf. bereinigte Prüfergebnisse** verwenden; keine eigenen Verbindungen oder Tests auslösen.

### Maßstab für spätere Statusmeldungen

- **46/46 Erststufen-Normlisten eingesehen** = **Ja**.
- **Alle Normquellen revisionsgenau archiviert und vollständig rekursiv geprüft** = **Nein**.
- **Phase 1 vollständig** = **Nein**.
- **Phase 2–4 vollständig** = **Nein**.
- **Neue Protokollimplementierung / Merge / Release erlaubt** = **Nein**.

**Der nächste Auftrag muss am belegten Stand hier anschließen und darf weder vorhandene Beleglücken stillschweigend schließen noch das vorhandene Forschungsprotokoll als Interoperabilitätsnachweis interpretieren.**


## Forschungsfortsetzung 23.10.2026 – zweite Microsoft-Referenzebene (neuester Prüfstand)

**Dieser Abschnitt ergänzt und aktualisiert den obigen historischen Snapshot, ersetzt dessen damalige Zahlen aber nicht rückwirkend.** Nach dem vorherigen Handoff wurden die **zwölf** bereits identifizierten Microsoft-Spezifikationen außerhalb der sechs MS-OCSPROT-Tabellen auf direkte §1.2.1-Referenzen untersucht: **elf vollständige offizielle HTML-Normlisten sichtbar**, **MS-RDPEMC nur als partieller offizieller Suchauszug** (Seitenabruf fehlgeschlagen). Der daraus gesondert erfasste zweite Ring enthält **246** gerichtete Normverweise zu **183** Zielkennungen. Im Vergleich zu den **199** bereits erfassten Zielkennungen des ersten Rings werden **146** neue Quellkennungen sichtbar: **52 Microsoft-/MC-**, **51 RFC-** und **43 andere Norm-/historische Draft-Kennungen**. Zusammen ergeben beide Inventare **345 verschiedene Zielkennungen**. **460 Erststufenkanten bleiben unverändert; 246 weitere Kanten sind nicht mit vollständiger Normrekursion oder clientverbindlicher Implementierung gleichzusetzen.**

- **Neue Primärdokumentation:** [`phase1-transitive-research-2026-10-23.md`](phase1-transitive-research-2026-10-23.md) (offizielle Quellen, Versionskonflikte, Interpretation und Aufgaben).
- **Maschinenlesbarer zweiter Ring:** [`transitive-reference-ledger.json`](transitive-reference-ledger.json) – 246 Kanten mit Quell-§, URL, Art, HTML-Beweisstatus und offener PDF-Versionsprüfung.
- **Offene dritte Ebene:** [`transitive-frontier.json`](transitive-frontier.json) – die 146 gegenüber dem bisherigen MS-OCSPROT-Zielgraphen neuen Kennungen mit rückverfolgbaren eingehenden Kanten; keine vorschnelle Clientpflicht.
- **Status der zwölf Ursprungsdokumente:** [`transitive-microsoft-inventory.json`](transitive-microsoft-inventory.json) – elf vollständige HTML-Listen, eine nur teilweise belegte Suchindexliste, keine revisionsgenau überprüfte Original-PDF-Ausgabe in dieser Etappe.

**Neue dokumentierte Streitpunkte:** MS-OAUTH2EX nennt den historischen OAuth-Draft als `-22`, verweist aber per URL auf `-23`; MS-H26XPF verweist auf das zusätzliche MS-RTPME; MS-RDPEMC muss gegen die gesamte veröffentlichte Originalausgabe geprüft werden. Die neue 2026-MS-KILE-Normliste darf nicht ohne passenden Editionsnachweis als das 2019-Client-Profil ausgegeben werden. Die von MS-RDPBCGR referenzierten RDP/TLS/OAuth- und ITU-T-Standards sind vor einer Einordnung als Clientfunktion auf MS-RTASPF/MS-CONFAS und konkrete Produktvarianten zurückzuführen.

**Kein Phase-1-Abschluss:** 33 erste HTML-only-Listen weiterhin ohne bestätigten Vergleich zur datierten Microsoft-PDF-Ausgabe; 12 zusätzliche Microsoft-Listen ebenfalls ohne PDF-Editionsvergleich (eine davon nur partiell); 146 weitere Knoten benötigen Quelle/Relevanz/Edition/Nachfolgeprüfung; RFC-Normverweise und Standards bleiben offen. **Keine Protokollimplementierung, keine Serververbindung, kein Interoperabilitätstest, kein Merge/Release.**

### Zusätzlicher RFC-Editionen-Check (2026-10-23)

Die offiziellen RFC-Editor-Metadaten wurden für **10 ausgewählte RFC-Originalausgaben** und deren direkte `Updates`-/`Obsoletes`-Beziehungen kontrolliert: **RFC 3265, 6665, 4566, 8866, 5245, 8445, 6125, 9525, 3548, 4648**. Nachweise mit RFC-Editor-Original-URLs, Richtung der Beziehung, Status und offenen Errata-/Normreferenzgates: [`rfc-version-relations-reviewed.json`](rfc-version-relations-reviewed.json). Die in den RFC-Zielkatalog eingebrachten Metadaten **ändern die 113 RFC-Kennungen nicht**. Dieser Editionsprüfungsschritt ist **keine vollständige RFC-Normverweis- oder Errata-Analyse**. Historisch referenzierte RFC-/Draft-Versionen bleiben von neueren Nachfolgern strikt unterschieden.

## 2026-10-23 – RFC 3261/2782 original bibliography continuation (P1-04)

**FACT:** Official original RFC Editor [RFC 3261 June 2002, §29](https://www.rfc-editor.org/rfc/rfc3261.html) enumerates **26 explicitly normative RFC reference edges**. Official original [RFC 2782 February 2000, unnumbered “References” p.10](https://www.rfc-editor.org/rfc/rfc2782.html) enumerates **7 distinct RFC IDs plus 2 historical Work-in-Progress entries**, but **does not classify its bibliography as normative/informative**; these 9 are strictly **UNCLASSIFIED** in the new [rfc-first-hop-reference-ledger.json](rfc-first-hop-reference-ledger.json). There are **23 RFC target IDs not present in the frozen 113-seed** (19 from RFC 3261, 4 from RFC 2782). Combined 35 source-specific references, 34 target keys. These do NOT add to the existing **460/246 Microsoft normative edges** and do not imply SfB Windows client mandatory behavior.

**FACT:** RFC Editor info headers identify RFC 3261 updated by 21 later RFCs and obsoleting RFC 2543; RFC 2782 updated by RFC 6335/8553 and obsoleting RFC 2052. RFC 3261 official errata search displayed **12 Verified**, but individual errata and other statuses were not fully reviewed; RFC 2782 errata access blocked/rate-limited. New [rfc-first-hop-audit-2026-10-23.md](rfc-first-hop-audit-2026-10-23.md) contains original section/date/URL references and open issues.

**P1-01 unresolved:** Attempts to fetch published MS-ABS and MS-SIPAE release PDFs failed in browser; container DNS failed for download host. No current release PDF file/hash or §1.2.1 edition upgrade, **33 HTML-only** still open. First-tier 46/460/199, second-tier 246/183/146, RFC seed 113, full PDF binaries 1, Phase 1 **INCOMPLETE**, code/live tests/main merge/release prohibited.

## 2026-10-23 – Vollständigkeit angefordert, 12 formale Abschlussgates geprüft (verbindlicher neuester Nachweis)

Der Versuch, Phase 1 anhand der realen JSON-Register als **vollständig** abzunehmen, ist **nicht bestanden: 0/12 Gates PASS**. [Phase-1-Abschlussaudit](phase1-closure-audit-2026-10-23.md) und [58-Quellen-/12-Gates-Matrix](phase1-closure-gates-2026-10-23.json) nennen die nicht nachgewiesenen Abschnitte und offiziellen Quellen für jede unmittelbare Microsoft-Quelle. Nachgezählt: **46** MS-OCSPROT-Dokumente, **13/46** §1.2.1-PDF-Ausgabenbelege, **33** HTML-only, **1** historisch vollständig gehashte Benutzer-PDF (in dieser Sitzung nicht erneut verfügbar), **460** erste Normkanten und **199** Ziele; **12** zweite MS-Quellen mit **0/12** PDF-Editionenbelegen, davon **11** HTML-Listen und **1** Suchauszug; **246** zweite Kanten, **146** neue offene Knoten, **113** RFC-Seed-IDs.

Neuer Versionsbefund: [RFC 3261, Juni 2002, §29 [6]](https://www.rfc-editor.org/rfc/rfc3261.html) zitiert **RFC 3268 / Juni 2002**, während der [RFC-Editor-Veröffentlichungseintrag für RFC 3268](https://www.rfc-editor.org/info/rfc3268) **Juli 2002** ausweist. Dies ist ein **offener Quell-Bibliografie-Widerspruch**, kein behaupteter Wire-Diff. Weitere RFC-Editor-Metadaten zu [RFC 2396](https://www.rfc-editor.org/info/rfc2396), [RFC 2234](https://www.rfc-editor.org/info/rfc2234) und [RFC 768](https://www.rfc-editor.org/info/rfc768) geprüft: historische Normen und spätere Updates bleiben getrennt, keine automatische 2019-Client-Pflicht.

**Phase 1 NICHT ABGESCHLOSSEN, neue Implementierungen verboten, Produkt-Build-/Server-Rollen-/Policy-Nachweis fehlt, externe Standards und Quellrekursion offen, kein Merge/Tag/Release.** Die schon archivierte MS-AVEDGEA-Datei ist **nicht** als in dieser Sitzung erneut gehasht dargestellt.
