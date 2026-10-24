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

## 2026-10-23 – MS-ABS revisionsgenauer historischer PDF-Seitenvergleich (P1-01/P1-07)

**FACT:** Die offiziellen veröffentlichten **MS-ABS 9.0 (2024-08-20)** und **10.0 (2025-05-20)** sind in den gerenderten Original-PDF-Seiten §1.2.1 **S. 9** geprüft. Beide nennen dieselben drei normativen Ziele **E164-2005, MC-RegEx, RFC2119**, während UASDC und UNICODE separat **informativ** in §1.2.2 bleiben. Der **10.0-Anhang D §8 S.74** nennt SfB 2019 Client, Server 2019 und Server Subscription Edition; **§9 S.76** meldet Änderungen an Glossar und Produktliste. Genaue Belege: [PDF-Editionsaudit](ms-abs-edition-audit-2026-10-23.md) und [maschinenlesbarer Editionsnachweis](ms-abs-edition-evidence-2026-10-23.json).

**OPEN:** Aktuell veröffentlichte **MS-ABS 10.1 (2025-08-19)** konnte weiterhin nicht aus dem umgeleiteten PDF-Download gelesen oder lokal gehasht werden. Ein 9.0/10.0-Vergleich ist **kein** Nachweis für 10.1, kein vollständiger Versionsdiff und kein normativer Client-Paritätsnachweis. **13/46** aktuelle original-PDF-§1.2.1-Belege, **33** HTML-only, **460** erste Normkanten, **246** zweite Kanten, **146** offene neue Ziele und **0/12** PASS-Gates **unverändert**. Phase 1 unvollständig; keine Implementierung/Live-Tests/Änderung an main/Release.

## 2026-10-23 – weiterer RFC-/OAuth-Quellenabgleich auf dem Draft-Branch

**FACT (P1-04/P1-12):** Die originalen [RFC 3263 §11/§12](https://www.rfc-editor.org/rfc/rfc3263.html) und [RFC 3264 §14/§15](https://www.rfc-editor.org/rfc/rfc3264.html) wurden aus offiziellen RFC-Editor-HTML-Originalausgaben eingesehen. **10** getrennte ausgehende Normkanten (**9** unterschiedliche RFC-Ziele), **8** ausdrücklich *informative* Bezüge, nachfolgende Updates/Obsoletes aus RFC-Editor-Infofeldern geprüft. In RFC 3264 ist RFC 3261 **informativ**, in RFC 3263 **normativ**. [Neuer RFC-Bericht](rfc3263-rfc3264-original-reference-audit-2026-10-23.md) / [JSON](rfc3263-rfc3264-reference-audit-2026-10-23.json). RFC 3268: §29-Angabe „June 2002“ stimmt mit dem Original-Dokumentkopf überein, RFC-Editor-**Publikationsmonat** ist „July 2002“; Unterschied zwischen Kopf-/Katalogdatum, kein erwiesener Wire-Konflikt.

**FACT/OPEN (P1-11):** IETF `draft-ietf-oauth-v2-22` (22.09.2011) und `-23` (21.01.2012) sind verschiedene historische Originale; MS-OAUTH2EX führt `-22` im Zitattitel und `-23` in der URL. [Quellenabgleich](ms-oauth2ex-oauth-draft-conflict-2026-10-23.md) dokumentiert zwei Kandidaten; die normative Absicht der aktuellen MS-OAUTH2EX-8.0-Ausgabe bleibt **OPEN**. Aktuelle Original-PDFs/Binärhashes weiterhin nicht verfügbar. Aktuelle Metriken nachgerechnet: **46** Microsoft-Quellen, **460** erste Normkanten/**199** Ziele, **246** zweite Kanten/**183** Ziele, **146** zweite-Ring-Neuziele, **345** Zielunion, **113** RFC-Seeds, **13** exakte aktuelle MS-PDF-Referenzseiten, **33** HTML-only, formelle **0/12 PASS**, Phase 1 **NICHT ABGESCHLOSSEN**. Keine Implementierung/Live-Tests/main-Merge/Release.

## 2026-10-24 – Acht Original-PDFs hochgeladen und nach SHA-256 geprüft

**FACT:** Vollständige PDF-Binärdateien mit Editionskopf, Gesamtbytezahl und SHA-256 belegt: MS-OCSPROT 9.0, MS-SIPAE 11.0, MS-SIPREGE 17.1, MS-ABS 10.1, MS-SIP 8.0, MS-KILE 49.0, MS-NLMP 37.0, MS-RDPEMC 16.0. Die sieben §1.2.1-Normlisten umfassen insgesamt **122** Quellenkanten und stimmen in ihren normierten Zielkennungen mit bestehenden Registerkanten überein. Siehe [vollständiger Evidenzbericht](uploaded-eight-ms-pdf-source-audit-2026-10-24.md) und [SHA-256-Manifest](uploaded-eight-ms-pdf-provenance-2026-10-24.json). Die PDF-Dateien selbst wurden **nicht** committet.

**Evidenzfortschritt:** Erste MS-Ebene jetzt **16/46** editionsgenau per PDF-§1.2.1 (30 HTML-only), zweite Ebene **4/12** per PDF-§1.2.1 (8 unbestätigte HTML-Quellen). MS-RDPEMC ist keine partielle Suchindex-Normliste mehr. Acht von 58 direkten Microsoft-Spezifikationen mit originalem Vollfile-Hash belegt (7 neu + historisch MS-AVEDGEA); MS-OCSPROT als gesondertes, gehashtes Überblicksoriginal. **Keine** weitere Normkante addieren (460/246 unverändert). **Phase 1 weiterhin NICHT ABGESCHLOSSEN, 0/12 Gates**, wegen fehlender anderer Editionsoriginale, Versionendiffs, RFC/externem Referenzgraph und Client-Build/Policy-Anwendbarkeit. Keine Implementierung, Live-Tests, main-Merge oder Release.


## 2026-10-24 – Drei weitere aktuelle Microsoft-§1.2.1-PDF-Seiten geprüft (P1-01)

**FACT:** Die veröffentlichten Original-PDF-Webansichten von **MS-EUMSDP 9.0 (17 S., 20.05.2025, §1.2.1 S. 6)**, **MS-TURN 21.0 (61 S., §1.2.1 S. 9–10)** und **MS-RTP 17.0 (68 S., §1.2.1 S. 9–10)** wurden einschließlich Seiten-Screenshots eingesehen. Die Editionen tragen jeweils `v20250520`. Die **3 + 11 + 10 = 24** dort normativ gelisteten Referenz-IDs entsprechen exakt bereits vorhandenen Erststufen-Kanten; **460** Kanten unverändert. Belege, offizielle URLs und §1.2.1/§1.2.2-Trennung: [Quellenbericht](published-three-ms-pdf-page-audit-2026-10-24.md), [JSON-Evidenz](published-three-ms-pdf-page-audit-2026-10-24.json).

**FACT / Scope:** MS-TURN nennt die historischen Entwürfe `draft-ietf-behave-rfc3489bis-02` und `draft-rosenberg-midcom-turn-08` **normativ**, RFC 3261 dagegen **informativ** (§1.2.1/§1.2.2 S. 9–10). MS-EUMSDP führt MS-SDPEXT, RFC 2119 und RFC 4566 **normativ**, acht andere MS-Protokolle **informativ** (§1.2.1/1.2.2 S. 6). MS-RTP führt unter anderem MS-H26XPF und `MSFT-H264UCConfig-v1.1-2011` **normativ**, MS-TURN dagegen **informativ** (§1.2.1/1.2.2 S. 9–10). Das begründet **keine** universelle Funktionspflicht des 2019-Windows-Clients.

**Evidenzgrenze:** Die Web-PDF-Seiten wurden gelesen, aber der direkte Binärdownload scheiterte an der DNS-Auflösung: **keine neu erworbene oder gehashte vollständige PDF-Datei**, **kein** vollständiger Vorversionsdiff. Aktuell **19/46** erste MS-Ebene mit editionsgenauer PDF-§1.2.1-Seite (**27 HTML-only**), **4/12** zweite Ebene, **460+246** bestehende Kanten, **146** weitere offene Zielkennungen, **0/12 Phase-1-Gates PASS**. **Phase 1 unvollständig**. Keine Implementierung, Live-/Interoperabilitätstests, `main`-Merge, Tag oder Release.


## 2026-10-24 – Weitere vier originalveröffentlichte Microsoft-§1.2.1-Seiten geprüft (P1-01)

**FACT:** Die Microsoft-Original-PDFs **MS-SRTP 14.0** (19 S., §1.2.1 S.7), **MS-SDPEXT 16.0** (82 S., §1.2.1 S.12–13), **MS-H264PF 16.0** (33 S., §1.2.1 S.7–8) und **MS-SIPCOMP 10.0** (24 S., §1.2.1 S.6–7) tragen `v20250520` (20.05.2025). Die jeweils normative und informative Bibliografie wurde anhand der offiziellen PDF-Textseiten und Seiten-Screenshots getrennt geprüft. Ihre **5+30+8+6 = 49** normativen Zielkennungen stimmen mit bereits erfassten Erststufen-Kanten exakt überein, **0 neue Kanten**. [Prüfbericht](published-four-ms-pdf-section-audit-2026-10-24.md), [maschinenlesbarer Beleg](published-four-ms-pdf-section-audit-2026-10-24.json).

**FACT/OPEN:** MS-SDPEXT §1.2.1 zitiert sowohl die historischen ICE-Drafts -06 und -19 als auch den ICE-TCP-Draft -07 normativ; `MS-ICE2` normativ, `MS-ICE` dagegen §1.2.2 informativ. MS-H264PF nennt exakt ISO/IEC 14496-10:2014 normativ (Originalstandard/Zugriff noch OPEN). MS-SIPCOMP nennt RFC 4346 (TLS 1.1) historisch normativ, was **keine** Freigabe zur TLS-Rückstufung oder eine generelle Client-TLS-1.1-Pflicht bedeutet. MS-SRTP nennt NIST FIPS 180-4 und RFC 3711 normativ; die Runtime-Anforderungen daraus bleiben separat auszuwerten.

**Evidenzgrenze:** Die Original-PDF-Webseiten wurden visuell bestätigt, der **vollständige Binärdownload/Dateihash ist nicht gelungen** (DNS-Auflösung des Microsoft-Hosts). Kein vollständiger Vorversionsdiff, Errata-Abschluss oder belegte Windows-Client-Build-/Server-/Policy-Anwendbarkeit. **Jetzt 23/46** ausgabengenaue erste PDF-§1.2.1-Nachweise, **23/46 HTML-only**; zweite Ebene **4/12**, 460 erste / 246 zweite Microsoft-Normkanten unverändert, **0/12 Phase-1-Gates PASS**. **Phase 1 NICHT ABGESCHLOSSEN**: keine neuen Protokollimplementierungen, keine Live-/Interoperabilitätstests, kein `main`-Merge oder Release.


## 2026-10-24 – Nutzerarchiv `OfficeProto.zip`: erste PDF-Quellebene vollständig akquiriert, E911-Graphfehler behoben

**FACT:** Das ZIP wurde lokal überprüft (SHA-256 `27f31dc1020d284159a44cecef5233aee5856814271cbf20f4f7ee1b8dd452f8`): **74 gültige PDF-Originale**, **180.967.975 entpackte PDF-Bytes**; alle 74 Vollfile-Hashes, Seitenzahlen und PDF-Editionen ermittelt. Sämtliche **46** MS-OCSPROT-Erststufen-Quell-IDs sind enthalten, davon **43** in der katalogisierten aktuellen Edition. **MS-ABS 10.0, MS-SIPREGE 17.0, MS-SSRTP 11.0** sind gegenüber 10.1 / 17.1 / 11.1 historische ZIP-Versionen und werden nicht stillschweigend zu den aktuellen umetikettiert. Die vorher **23 HTML-only**-§1.2.1-Quellen sind in ihren exakten veröffentlichten PDF-Ausgaben nun als vollständige gehashte Originaldateien eingesehen; zusammen mit früherem editionsgenauen PDF-Seiten-/Originalnachweis sind **46/46** aktuellen Erststufen-§1.2.1-Abschnitte belegt, **0 HTML-only**. Dies ist **keine** abgeschlossene Vollanalyse oder Versionsvergleich.

**NEUER QUELLENFEHLER (FACT, MS-E911WS 9.0 / 20.05.2025 §1.2.1 PDF S. 8–9):** Der vorhandene Quellgraph führte nur **8** normative Kanten; in der Original-PDF sind **12** separat normative Referenzen. Vier fehlende, auf S. 9 sichtbar verifizierte Verweise **[WSDL]**, **[XMLNS]**, **[XMLSCHEMA1/2]**, **[XMLSCHEMA2/2]** wurden mit der zutreffenden W3C-Ausgabe ergänzt. Der ursprüngliche WSA1.0-WSDL-Binding-Verweis bleibt separat bestehen. Die Erststufen-Normkanten betragen nun **464** (vorher 460), die vier Zusätze gehören zu den externen Normen (**87** statt 83), **199** verschiedene Erststufen-Zielkennungen unverändert; zweite MS-Ebene weiter **246** Kanten. Die übrigen Quellabschnitts-**Anzahlen** stimmen überein; die vollständige semantische Normziel-Kanonisierung insbesondere für 22 neue Quellen ist noch **OPEN**.

**PROVENIENZ:** In derselben Edition/Dateilänge weichen neue ZIP-SHA-256 von historisch protokollierten Hashes für MS-AVEDGEA 12.0, MS-SIPAE 11.0 und MS-OCSPROT 9.0 ab; ohne frühere Binärdateien kein inhaltlicher Diff-Nachweis. **PDFs sind noch NICHT im öffentlichen GitHub-Repository abgelegt.** Der Copyright-Hinweis erlaubt Kopien zur Implementierungsentwicklung und die Veröffentlichung von *Teilen* in Implementierungsdokumentation, erteilt aber keine eindeutige allgemeine Erlaubnis zur öffentlichen Weiterverbreitung der vollständigen PDFs. Ein privates, zugriffsbeschränktes Quellenarchiv und Rechteklärung bleiben für diese Nutzeranforderung nötig. Forschungsstand, Register und Prüfsummen sind als Text dokumentiert, ohne unzulässige Public-Originalveröffentlichung vorzutäuschen.

[Quell-Audit und neue SHA-256-Tabelle](officeproto-zip-source-audit-2026-10-24.md). **Noch offen:** acht Quelloriginale außerhalb MS-OCSPROT (MS-H26XPF, MS-OAUTH2EX, MS-OXWUMS, MS-RDPBCGR, MS-SDP, MS-WCCE, MS-WSPOL, MS-WSTEP), historische Editionsdiffs, vollständiger RFC-/Draft-/ISO-/W3C-Referenz-Fixpunkt, Produktbuild/Serverrole/Policy und sichere Binärarchivierung. Formelle **0/12 Gates PASS** trotz PDF-Quellenerwerb; **Phase 1 INCOMPLETE**. Keine Protokollimplementierung, Echtserver-/Interoperabilitätstests, `main`-Merge, Tag oder Release.


## 2026-10-24 – Zwei weitere Microsoft-Originale aus Exchange-/SharePoint-Archiven (P1-02/P1-03/P1-11)

**FACT:** Die Nutzer-Archive `Exchange_Protocols.zip` und `SharePointProtocols.zip` enthalten **129** beziehungsweise **168** einzeln als PDF verifizierte und vollständig SHA-256-gehashte Originaldateien (zusammen **297** PDF-Dokumente, **820.238.729** entpackte PDF-Bytes). Archiv-SHA-256: Exchange `e3729e5712fd11b087a2c3560d5490d54afed04f2ca80562cbeb6020373c9f88`, SharePoint `0b3f3ccfe190a2988edc0d1d74a47cb1f81106c13e30dc9ab60bebf672974fc1`. Die vollständigen PDF-Binärdateien wurden lokal geprüft, **nicht** in das öffentliche Git-Repository übertragen.

**FACT – exakte Normabschnitte:** [MS-OAUTH2EX rev 8.0, 20.05.2025, §1.2.1 PDF-S.5](exchange-sharepoint-originals-audit-2026-10-24.md) (19 S., SHA-256 `fd0c0291abb6c3d696872855243ef8b59bf8e413c55c0b9303c4644d9117cc74`) bestätigt **7** vorhandene Normkanten. [MS-OXWUMS rev 16.0, 20.05.2025, §1.2.1 PDF-S.10–11](exchange-sharepoint-originals-audit-2026-10-24.md) (107 S., SHA-256 `61447ba6fdbb82a4cd11ad3d433aef369d764da286294acb7e65b92e91d54f1d`) bestätigt **16** vorhandene Normkanten. Beide Originalausgaben einschließlich Revisionstabelle, Referenzseiten-Text und Seitenansicht geprüft. Kein neuer Normverweis. **Zweite Ebene nun 6/12** ausgabengenaue originale §1.2.1-PDF-Quellen (**6 noch ungeprüft**), **246** Kanten unverändert; erste Ebene **46/46**, **464** Kanten unverändert.

**FACT/OPEN – OAuth-Konflikt:** MS-OAUTH2EX **Originalrevision 8.0** §1.2.1 S.5 selbst druckt den historischen OAuth-Draft `-22` im Quellennamen und verlinkt auf die unterschiedliche Ausgabe `-23`. Das bisherige *Original-PDF-Zugriffsproblem* ist gelöst; die **Normabsicht bleibt OPEN**, keine stille Ersetzung durch eine neue Draft-/RFC-Version. MS-OXWUMS referenziert XML-Schema 1/2 **First Edition 2001**, nicht pauschal die 2004er Second Edition.

**Zusätzlich erworbene, noch nicht normativ überprüfte direkte Microsoft-Referenzquellen:** MS-OXDSCLI, MS-OXWSCDATA, MS-OXWSCORE, MS-OXWSCVTID, MS-OXWSSRCH (Exchange) und MS-SPS2SAUTH (SharePoint). Alle sechs Binärhashes und exakte PDF-Ausgaben im [neuen Quellenbericht](exchange-sharepoint-originals-audit-2026-10-24.md). Noch fehlende Zweitring-PDF-Originale: **MS-H26XPF, MS-RDPBCGR, MS-SDP, MS-WCCE, MS-WSPOL, MS-WSTEP**. Die rechtlich zulässige **private** Binärarchivierung ist weiterhin offen; keine Behauptung, die PDFs lägen bereits im öffentlichen Repository.

**Forschungsgrenzen:** Produkt-Build, Serverrollen/-Policies, Normrekursion, historische Versionendiffs und externe Referenzen nicht geschlossen; **0/12 Phase-1-Abschlussgates PASS, Phase 1 INCOMPLETE**, keine Implementierung, realen Server-/Interoperabilitätstests, `main`-Merge, Tag oder Release.


## 2026-10-24 – Windows_Protocols.tar.xz: zweite Original-PDF-Ebene nun vollständig editionsgeprüft

**FACT – Vollquellen:** Das Nutzerarchiv `Windows_Protocols.tar.xz`, SHA-256 `52f87efc17c9854f70b0b14650c877a572b1a99c7ed9df4f6c9a96387720c652`, enthält **451 gültige PDF-Dateien** (**424 MS / 27 MC**) und **1.334.078.644** entpackte PDF-Bytes. Sämtliche Dateibytes wurden lokal validiert und nach SHA-256 gehasht, ohne Voll-PDFs im öffentlichen GitHub zu veröffentlichen. Vollständige pro-Datei Prüfsummen, Metadaten und 145 Quellenlabel-/Normtarget-Abgleiche liegen als herunterladbares lokales Nachweispaket vor; SHA-256 der drei Prüfnachweise sind im [Windows-Originalquell-Audit](windows-originals-2026-10-24.md) protokolliert.

**FACT – Ausgaben/Abschnitte:** Die sechs zuletzt fehlenden aktuellen Microsoft-PDFs **MS-H26XPF 1.3 (2017-06-01), MS-RDPBCGR 62.0 (2026-03-09), MS-SDP 1.0 (2017-06-01), MS-WCCE 55.0 (2026-08-24), MS-WSPOL 8.0 (2019-03-13), MS-WSTEP 15.0 (2024-04-23)** sind exakt vorhanden; Versionsgeschichte, PDF-Footer, §1.2.1-Normverzeichnisse und Seiten-Screenshots geprüft. Ihre **4+37+7+61+20+16 = 145** bestehenden normativen Quellenkanten stimmen nach belegter Zitationslabel-/Editions-Normalisierung vollständig mit dem zweiten Register überein, **0 neue Kanten**. **Zweite Ebene jetzt 12/12** aktuelle Original-§1.2.1-PDF-Ausgaben (**0 editionsungeprüfte Quellen**); **246** Normkanten im zweiten Ring und **464** im ersten bleiben unverändert.

**FACT – neue Abhängigkeitsoriginale:** Alle **52** im zweiten MS-Normring neu entdeckten MS-/MC-Zielkennungen haben Original-PDFs in den bisherigen drei Archivlieferungen (46 Windows/MC, sechs Exchange/SharePoint). **Nur Dateibeschaffung**, weder rekursiver Norm-Fixpunkt noch verpflichtendes Windows-Full-Client-Feature. Bei **MS-NLMP 37.0, MS-RDPEMC 16.0 und MS-SIP 8.0** unterscheiden sich neue Binär-SHA-256 von früheren gehashten Quellen trotz gleicher Edition. Frühere Hashes werden *nicht überschrieben*; ohne alte Originalbytes keine Aussage über genaue textliche Differenzen. MS-KILE 49.0 stimmt binär mit dem historisch protokollierten SHA-256 überein.

**Grenzen:** Insgesamt **822 PDFs aus allen drei Nutzerlieferungen** dateibezogen gehasht (74 Office, 297 Exchange/SharePoint, 451 Windows; Duplikate/ältere Ausgaben möglich). Vollständige Original-PDFs **nicht** ins öffentliche GitHub übertragen, Rechte-/privates Quellenarchiv weiter offen. P1-01 Quellnormziel-Semantik der ersten Ebene, dritte und weitere Normrekursion, RFC-/IETF-Draft-/ISO-/W3C-Historie/Errata, substantielle Vorversionsdiffs sowie fixierter Windows-2019-Client-/Server-/Policy-Stand bleiben offen. **0/12 formale Gates PASS; Phase 1 INCOMPLETE; keinerlei Code, Live-Netztests, `main`-Merge, Tag oder Release.**


## 2026-10-24 – Dritte normative MS-/MC-Ebene quellenbezogen erfasst (P1-03, noch OPEN)

**FACT:** Die **52** zuvor neu entdeckten MS-/MC-Spezifikationen wurden aus den nutzerbereitgestellten Microsoft-PDF-Originalarchiven auf vollständigen Binär-Hash (SHA-256), `vYYYYMMDD`-Fußzeile und Literaturabschnitte untersucht. **48** Quellen besitzen ein explizites `§1.2.1 Normative References` mit zusammen **827** normativen, quellspezifischen gerichteten Verweisen: **398 MS-/MC, 267 RFC, 162 externe/historische**. **Vier Active-Directory-Schemabände** (`MS-ADA1`, `MS-ADA2`, `MS-ADA3`, `MS-ADSC`) haben dagegen einen allgemeinen `§1.1 References`-Abschnitt mit **67 nicht normativ klassifizierten Nennungen**. Diese werden **NICHT** den normativen 827 Kanten zugeschlagen. Abschnittsgrenzen wurden ohne Inhaltsverzeichnis-Treffer erkannt, Fußzeilen-/Selbstzitate ausgeschlossen und Original-PDF-Abschnitte familienübergreifend visuell kontrolliert.

**Rohziel-Fortschritt:** Die 827 Kanten betreffen **342** unterschiedliche Ziel-Labels; davon **106** bereits in der bisher bekannten Zielunion der ersten beiden Ebenen (**345**), somit **236** neue Rohkennungen (**86** MS-/MC, **62** RFC, **88** externe/historische/aliasartige Kennungen), **581** Raw-Label-Ziele über drei Ebenen. Rohkennungen sind **noch nicht historisch normiert/semantisch kanonisiert**, insbesondere historische IETF-/W3C-/ISO-/ITU-Editionen und Namensvarianten. Dies beweist **keine** 581 clientseitig verpflichtenden Funktionen oder einen normativen Fixpunkt. Die **464 + 246** vorhandenen direkten ersten/zweiten Ring-Kanten bleiben unverändert und werden durch den **gesonderten** dritten Satz von **827** Kanten ergänzt.

**Git-Provenienz (keine Original-PDFs):** [Drittstufenbericht](third-ring-original-bibliography-audit-2026-10-24.md), [52 Original-Editionen/SHA-256](third-ring-source-inventory-2026-10-24.txt), [vollständige Referenzkanten/Seiten-Nachweise als GZIP-TSV](third-ring-bibliography-edges-2026-10-24.tsv.gz) und [86 weitere MS-/MC-Frontier-Ziele](fourth-ring-ms-frontier-2026-10-24.json). Der komprimierte TSV-Git-Blob wurde bitgenau gegen den lokalen SHA-1-Git-Blob-Wert kontrolliert; es sind **keine** Microsoft-PDF-Binärdateien committet.

**PDF-Haltung gemäß Nutzer:** Der Nutzer behält die Originale. Es besteht **keine Pflicht zu privatem/öffentlichem Git-PDF-Archiv**; falls eine konkrete Ausgabe oder Nachkontrolle nötig wird, deren PDF gezielt beim Nutzer anfordern. Offene Arbeiten: Prüfung/ggf. Einengung der **86** weiteren MS-/MC-Abhängigkeiten, kanonisierter RFC-/Draft-/Externgraph, Schema-Bibliografie-Auslegung, Vorversions-Diffs, Windows-2019-Client-Build/Serverrollen/Richtlinien und unabhängig überprüfter relevanter Fixpunkt. **0/12 formelle Phase-1-Abschlussgates PASS, Phase 1 NICHT ABGESCHLOSSEN.** Keine Protokollimplementierung, Echtserver-Tests, `main`-Merge, Tag oder Release.

### Archiv-Abdeckung der nächsten 86 MS-/MC-Quellen (2026-10-24)

Die Dateinamen aller **86** bisher neuen vierten-MS-/MC-Zielkennungen wurden gegen **alle vier** nutzerbereitgestellten Archivpakete abgeglichen. **85/86** Original-PDF-Dateinamen sind bereits lokal verfügbar: **47** Windows, **37** Exchange und **eine** SharePoint-Datei. **Einziger derzeit fehlender Original-PDF-Dateiname: `MS-DLTM`**. Die offizielle [Microsoft-Veröffentlichungsseite](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dltm/a4a88aac-3bc5-4d5a-8136-52a3bfb979ef) nennt *Distributed Link Tracking: Central Manager Protocol* **Rev. 9.1, 01.06.2017** als veröffentlichte Ausgabe. Dieser Dateinamenabgleich bestätigt **nicht** Editionsaktualität oder normative §1.2.1-Verifikation der 85 vorhandenen Spezifikationen. Die fehlende PDF nur bei tatsächlichem Bedarf gezielt vom Nutzer anfordern; keine allgemeine erneute Archivlieferung erforderlich.


## 2026-10-24 – Nutzerquelle MS-DLTW v20210625: dokumentiert, MS-DLTM weiterhin fehlend

**FACT:** Der Nutzer lieferte `[MS-DLTW]-210625.pdf` (**Distributed Link Tracking: Workstation Protocol**, Rev. **16.0**, Ausgabe **25.06.2021** `v20210625`, **38 PDF-Seiten**, **1.300.722 Bytes**, SHA-256 `28a8b2a3e57e2594c0d96501679cf413841c432975364cb2ad87a85a09fe8a15`). Die historische PDF-Seite **9**, §1.2.1/§1.2.2, wurde als Text und Seitenbild gelesen: **neun normative Ziele** (C706, MS-DTYP, MS-ERREF, MS-FSCC, MS-RPCE, MS-SMB2, MS-SMB, RFC1088, RFC2119), **MS-DLTM ausschließlich §1.2.2 informativ**. [Belegbericht](user-ms-dltw-historical-upload-2026-10-24.md), [JSON-Nachweis](user-ms-dltw-historical-upload-2026-10-24.json).

**Editionsgrenze:** Im bereits gehashten `Windows_Protocols.tar.xz` liegt eine **neuere MS-DLTW-Ausgabe `v20240423`** mit 38 Seiten, **1.287.869 Bytes**, SHA-256 `b00fb17a43331083d05243bb55454d41a364b27ed136f39c17dd4e3d0b40aaa7`. Beide Ausgaben getrennt halten; kein Inhalts-/Vorversionsdiff erfolgt. **Das für P1-03 noch fehlende Original `MS-DLTM` wurde NICHT geliefert.** Es ist ein anderes Protokoll (*Central Manager*, katalogisierte Rev. 9.1 vom 01.06.2017), normativ in der Drittstufenquelle **MS-RPCE** zitiert. Die Viertstufen-Dateiabdeckung bleibt **85/86**; 1 Original fehlt, keine neue Normkante. **Kein PDF im Repo** gemäß Nutzerwunsch; binäre Originale bleiben beim Nutzer. Formell **0/12 Gates PASS, Phase 1 OPEN**, keine Implementierung, Live-Tests oder Merge.


## 2026-10-24 – MS-DLTM rev 9.1 Original erhalten; Viertstufen-Dateiabdeckung 86/86

**FACT:** Der Nutzer lieferte die zuvor fehlende Original-PDF `[MS-DLTM]-170601(1).pdf` (Distributed Link Tracking: Central Manager Protocol), Rev. 9.1, Ausgabe `v20170601`, 01.06.2017, 63 Seiten, 1.759.744 Bytes, SHA-256 `46c0c9acb95e710beb2a8b61f6a0c635ae91b41456e919ad1c92b1e754db90f8`. Norm-§1.2.1 PDF-S.9 im Text und visuell geprüft: **10 normative** Zielkennungen `C706`, `MS-ADTS`, `MS-DLTW`, `MS-DTYP`, `MS-ERREF`, `MS-RPCE`, `MS-SAMR`, `MS-SMB`, `MS-SPNG`, `RFC2119`. §1.2.2 PDF-S.9–10 führt **6 informative** Referenzen. [Exakter Bericht](fourth-ring-ms-dltm-original-audit-2026-10-24.md), [JSON-Provenienz](fourth-ring-ms-dltm-original-audit-2026-10-24.json), [separater vierter Normkanten-Ledger](fourth-ring-normative-reference-ledger.json).

**Unverwechselbar:** Die frühere `MS-DLTW`-2021-Ausgabe führt `MS-DLTM` nur *informativ*, während `MS-DLTM → MS-DLTW` hier **normativ** ist. `MS-RPCE → MS-DLTM` ist die zuvor festgestellte normative Drittstufenreferenz. Neu im vierten Ring: 10 gerichtete Kanten, 9 zuvor bekannte Ziele plus der Rohzielname `MS-DLTW` (PDF bereits aus Nutzerdaten vorhanden). **464/246/827** Normkanten der ersten drei Ringe bleiben unverändert.

**Status:** Nun sind **86/86** Viertstufen-MS-/MC-Originaldokumentidentitäten dateibezogen verfügbar, aber erst **1/86** aktuelle Viertstufen-§1.2.1-Bibliografien individuell geprüft. Für die übrigen **85** sind exakte Edition, Normalisierung und Produktanwendbarkeit OPEN. MS-DLTM §1.4/§1.6 S.12 beschreibt domänengebundene Datei-/Domänencontroller-Funktionen, keine nachgewiesene allgemeine Skype-for-Business-2019-Windows-Client-Pflicht. Der Nutzer behält die PDF-Datei; im Git sind nur Quellenbelege und Metadaten. **0/12 Phase-1-Gates PASS, Phase 1 offen, keine Implementierung, Live-Tests, `main`-Merge oder Release.**
