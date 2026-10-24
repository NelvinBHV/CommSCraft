# MS-OAUTH2EX: OAuth-Draft -22 / -23 bibliographischer Konflikt (23.10.2026)

**P1-11 – präzisiert, aber nicht freigegeben (OPEN).** Diese Prüfung unterscheidet den Microsoft-Quelleneintrag, die tatsächlich adressierte IETF-Draft-Revision und die aktuell veröffentlichte MS-Originalausgabe. Kein Versionsersatz ohne Normtextvergleich.

## Belegter Sachverhalt

- **FACT – Microsoft:** Die offizielle [MS-OAUTH2EX-Landing-Page](https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-oauth2ex/56105233-7ca7-4b4b-821d-84bb55bfc2cf) nennt **Rev. 8.0, 20.05.2025** als veröffentlichte Version. Der in [transitive-reference-ledger.json](transitive-reference-ledger.json) erfasste Microsoft-§1.2.1-HTML-Quellstand (letztes Seitenupdate **20.08.2024**) enthält die bibliographische Normzitation `draft-ietf-oauth-v2-22`, **aber** einen URL-Pfad mit `draft-ietf-oauth-v2-23`. Eine indizierte, *ältere* Microsoft-PDF-Ausgabe **v20210720**, §1.2.1 S.6 bestätigt denselben -22/-23-Konflikt in der damaligen Veröffentlichung; das Indexergebnis ist **keine** vollständige oder gehashte aktuelle 8.0-PDF-Quelle.
- **FACT – IETF:** Die unveränderliche [Draft-Revision -22](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-22) trägt den Dokumentkopf **22.09.2011 / `draft-ietf-oauth-v2-22`**. Die [Revision -23](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-23) hat den Dokumentkopf **21.01.2012 / `draft-ietf-oauth-v2-23`**. Dies sind **zwei eigenständige historische Draft-Snapshots**, später aus derselben Draft-Linie in [RFC 6749](https://www.rfc-editor.org/info/rfc6749) mündend; RFC 6749 ersetzt **keinen** ungeklärten historischen Microsoft-Zitatstand automatisch.
- **INFERENCE:** Der Konflikt ist eine *Bibliographie-/Link-Inkonsistenz* der Microsoft-Zitation. Allein der ausgeschriebene `-22`-Text beweist nicht, dass Microsoft normativ den Inhalt von -22 statt -23 meinte; die `-23`-Adresse beweist genauso wenig die gegenteilige Entscheidung.
- **OPEN:** Original-**MS-OAUTH2EX 8.0** PDF/DOCX, SHA-256, §1.2.1, Änderungs-/Erratahinweise; direkte Microsoft-Klarstellung oder Textvergleich -22/-23 für die tatsächlichen angezogenen Regeln; SfB-2019-Client-/Rollen-/Policy-Anwendbarkeit.

## Disposition

`P1-11 = OPEN_REFERENCE_TARGET_AMBIGUOUS`. In Datenmodellen den zusammengesetzten Konflikt-Key `draft-ietf-oauth-v2-22_TITLE_vs_v2-23_URL` **nicht** stillschweigend in `draft-ietf-oauth-v2-22`, `-23` oder RFC 6749 umbenennen. Beide Original-Draft-URLs neben dem Microsoft-Zitat pflegen; nach Abruf der passenden 8.0-Originalausgabe Versions- und Textdifferenzen prüfen. Die zwölf formalen Phase-1-Gates bleiben **0/12 PASS**.


## 2026-10-24 – Verifikation am aktuellen MS-OAUTH2EX-8.0-Original (Quellenrevision bestätigt, Draftentscheidung bleibt offen)

**FACT:** Das vollständige Nutzer-Original `SharePointProtocols.zip/[MS-OAUTH2EX].pdf` ist `v20250520` / **rev 8.0**, **19 Seiten**, **833.843 Bytes**, vollständige SHA-256 `fd0c0291abb6c3d696872855243ef8b59bf8e413c55c0b9303c4644d9117cc74`. Die veröffentlichte Revisionstabelle **PDF S.2** bestätigt 20.05.2025 / 8.0; die originalgeprüfte Referenzseite **§1.2.1 PDF S.5** führt ausdrücklich den Namen `draft-ietf-oauth-v2-22`, verlinkt jedoch auf `draft-ietf-oauth-v2-23`. Die anderen sechs direkten Normkanten stimmen mit `transitive-reference-ledger.json` überein. **Somit ist die Ausgabe-/Original-Nachweislücke geschlossen.**

**OPEN:** Die widersprüchliche Draft-Zielrevision ist damit **nicht** normativ entschieden. Der Ledger-Konfliktkey `draft-ietf-oauth-v2-22_TITLE_vs_v2-23_URL` bleibt erhalten, bis Original-Draft-Versionsdiff, tatsächliche normative Verwendungsstellen und gegebenenfalls eine Microsoft-Klarstellung ausgewertet sind. Auch die Client-/Server-Anwendbarkeit bleibt ungeklärt. Kein RFC-6749-Ersatz, keine Implementierungsfreigabe. [Neuer Quellenbericht](exchange-sharepoint-originals-audit-2026-10-24.md).
