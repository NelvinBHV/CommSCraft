# P1-03 – Dritte normative Microsoft-/MC-Referenzebene (24. Oktober 2026)

**Stand: neue Quelldaten erhoben, normativer Fixpunkt und Produktanwendbarkeit OFFEN.** Dieser Bericht ist rein quellenbezogene Forschung zu CommSCraft und keine Freigabe für Protokollimplementierung, Netzwerk-Interoperabilitätstests, Merge oder Release.

## Quellen und Vorgehen

Die **52** MS-/MC-Zielkennungen des bestehenden [zweiten Microsoft-Fronters](transitive-frontier.json) wurden gegen vollständige nutzerbereitgestellte PDF-Originaldateien geprüft: **46** aus `Windows_Protocols.tar.xz`, **fünf** aus `Exchange_Protocols.zip` und **eine** aus `SharePointProtocols.zip`. Die PDFs sind beim Nutzer archiviert. In Git verbleiben **nur** die je Datei berechnete SHA-256, die PDF-Fußzeilen-Ausgabe `vYYYYMMDD`, die PDF-Seitennummern, extrahierte Referenzkennungen und der Forschungsstatus.

**FACT:** 48/52 Quell-PDFs enthalten einen exakt überschriebenen Abschnitt `§1.2.1 Normative References`. Die Ausnahme ist die **Active-Directory-Schema-Dokumentfamilie** `MS-ADA1`, `MS-ADA2`, `MS-ADA3`, `MS-ADSC`: Diese vier verwenden stattdessen `§1.1 References` **ohne getrennte normative/informative Klassifikation**. Die dort gelisteten Referenzen werden nicht zu normativen Kanten hochgestuft.

Die Seiten wurden lokal mit PyMuPDF extrahiert; die Abschnittsgrenzen wurden anhand der realen Überschriften (nicht der Inhaltsverzeichniszeilen) erkannt und an gerenderten Originalseiten verschiedener Familien visuell kontrolliert (MS-ADA1, MS-ADTS, MS-NRPC, MS-SPNG, MC-NBFSE, MS-SPS2SAUTH, MS-OXWSCORE und MS-RDPEPS). PDF-Fußzeilen/Selbsterwähnungen wurden nicht als eigene bibliographische Kanten gezählt. Jeder unterschiedliche bibliographische Referenzeintrag ist innerhalb der jeweiligen Quell-Spezifikation genau einmal erfasst. **Die automatische normtextliche Interpretation einzelner Clientpflichten ist nicht erfolgt.**

## Ergebnis der 52 Quellbibliografien

| Nachweistyp | Quellen | Direkte Erwähnungen |
|---|---:|---:|
| Exakt `§1.2.1` **normativ** | **48** | **827** |
| `§1.1 References` **unklassifiziert** | **4** | **67** |
| **Gesamt** | **52** | **894** |

Die **827 gerichteten normativen Kanten** zerfallen in **398** MS-/MC-Verweise, **267** RFC-Verweise und **162** externe beziehungsweise historische Referenzen. Das sind *neue direkte Kanten der dritten Untersuchungsebene*, keine 827 neuen Spezifikationen. Die 67 Bibliografie-Erwähnungen aus den vier Schema-PDFs werden im TSV ausdrücklich als `unclassified_reference` geführt und **nicht** in die normative Kantenanzahl aufgenommen.

**Zielunion (exakte Rohkennungen, vor semantischer Kanonisierung):** Die 827 normativen Kanten verweisen auf **342** unterschiedliche Ziel-Labels. **106** davon sind bereits in den ersten beiden Ringen (345 bekannte Labels) enthalten, **236** zusätzliche sind neu: **86 MS-/MC-Spezifikationen**, **62 RFCs** und **88 externe, historische oder noch nicht kanonisierte Labelvarianten**. Die Raw-Label-Zielunion über drei Ebenen ist damit **345 + 236 = 581**. Gerade Standards wie `C706`, `ITU...`, `X690` oder historische IETF-Draft-Aliase können vor der abschließenden Quellversionsauflösung nicht stillschweigend vereinheitlicht werden. Dies sind keine Zahlen für Client-Pflichtfeatures.

Zu den besonders verzweigungsreichen Ausgaben gehören `MS-ADTS v20260824` (**97** normative Label), `MS-NRPC v20260209` (**41**), `MS-SAMR v20260427` (**41**), `MS-CSRA v20260824` (**39**) und `MS-OXWSCDATA v20250520` (**39**). Sie werden **nicht automatisch als Clientpflicht** interpretiert; AD/Kerberos/Windows-Server-/Exchange-Teilnetze können durch Rollen, Richtlinien und Client-Funktionspfade bedingt sein.

## Wiederverwendbare, verifizierte Registerdateien

- **[52-Quellen-Inventar](third-ring-source-inventory-2026-10-24.txt):** jede MS-/MC-Kennung mit Original-PDF-Edition, SHA-256, Startseite, Bibliografie-Typ N/U und Referenzanzahl.
- **[Vollständige Drittstufen-Referenzkanten](third-ring-bibliography-edges-2026-10-24.tsv.gz):** GZIP-komprimierte UTF-8-TSV-Tabelle (894 Zeilen + Header), `referrer, target, pdf_page, classification, kind`. GZIP-SHA-256 `3910b571cf8f11c5287fa7415e46cde9b1904b177f700d4879f30ebd40590de5`; Dekompression ergibt TSV-SHA-256 `f8503123b6cde9b810fda0e24a3701f37f4a756b3c4a80fb50ecefaecb4bc503`. Git-Blob-SHA `1001663179629981a0f0f7005f2fa178da616eef` wurde beim Upload abgeglichen.
- **[Neue vierte MS-/MC-Frontier](fourth-ring-ms-frontier-2026-10-24.json):** die **86** bislang nicht als Ziel erfassten MS-/MC-IDs mit `OPEN`-Status. Vor neuer Datei-Anforderung sind die **bereits vorhandenen drei Quellarchive** anhand ihrer Dateilisten zu durchsuchen.

Diese Dateien enthalten keine vollständigen Microsoft-PDF-Binärbytes und keine geheimen Netzzugangsdaten. Die gesamte, unkomprimierte 52-Sourcen-/894-Referenzen-JSON-Evidenz ist zusätzlich als Gesprächsdatei für den Nutzer verfügbar, muss aber für die Git-Weiterarbeit nicht erneut aus PDFs gewonnen werden, solange TSV und SHA-Inventar erhalten bleiben.

## Wesentliche offene Arbeiten (nicht als PASS zählen)

1. **P1-03 / P1-10:** Die 86 neu gefundenen MS-/MC-Ziele und die 62 RFCs plus 88 externen/varianten Ziel-Labels müssen nach Edition, Veröffentlichung und vor allem *tatsächlicher Client-/Serverrollen-/Policy-Anwendbarkeit* bewertet werden. Rekursion gegebenenfalls bis zum relevanten Fixpunkt wiederholen; bei Unanwendbarkeit eine belegte Ausschlussentscheidung dokumentieren.
2. **Schema-Sonderfall:** Für `MS-ADA1/2/3/ADSC` sind alle 67 allgemeinen Referenzen *unklassifiziert*. Nicht pauschal normativ behandeln; deren Schemaobjekte, tatsächlicher Produkt- und Nutzungspfad, und Verweisauslegung bleiben getrennt zu prüfen.
3. **P1-04, P1-05, P1-07:** RFC-Update-/Errata-/Obsolescence-Graph, historische IETF-Drafts, reale externe Normeditionen und frühere Microsoft-Ausgaben bleiben offen. Alte TLS-/SSL-/Krypto-Bibliografien erlauben keine unsichere Implementierung.
4. **P1-06:** Exakter Skype for Business 2019 Windows Full Client-Build, relevante Serverversion/Serverrolle, Feature-, Recovery- und Policy-Profile nicht hinreichend belegt.
5. **Quellhaltung:** Die PDFs liegen vereinbarungsgemäß **beim Nutzer**. Es besteht **keine** Forderung mehr, die vollständigen PDF-Dateien in GitHub zu archivieren. Falls eine bestimmte Originalausgabe für Nachprüfung fehlt, gezielt beim Nutzer anfordern.
6. **Formelle Phase-1-Abschlussgates:** **0/12 PASS**. Das bloße Erfassen von §1.2.1-Bibliografien ist nicht gleichbedeutend mit einem bewiesenen normativen Fixpunkt, einer unabhängigen Review oder vollständiger Windows-Client-Funktionsparität.

**Keine Protokollcode-Änderungen, keine Live-/Interoperabilitätstests, keine Tests mit kostenpflichtigen CI-Runnern, kein Merge nach `main`, kein Tag und kein Release.**
