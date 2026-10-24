# CommSCraft – Prompt zur Fortsetzung der Protokollforschung

> Neuer P1-03-Fortschritt NACH Handoff 24.10.2026: [Batch-Report](fourth-ring-p1-03-batch-2026-10-24.md) – vier weitere originalv20250520 PDF-Bibliografien, jetzt 5/86 geprüft, 64 vierte Normkanten, 81 offen; vier weitere Windows-IDs nur Revisionen geprüft. Nächste genaue Originaleditionen: MS-FSCC 60.0, MS-SMB2 88.0, MS-GPOL 39.0, MS-TLSP 18.0. Alte Zahlen unten sind historischer Handoff. Gates 0/12 PASS.

Setze die **CommSCraft-Protokollforschung (Phase 1, Quellenanalyse)** im GitHub-Repository `NelvinBHV/CommSCraft` auf dem **bestehenden Branch `draft/protocol-spec-analysis-2026-10-23`** fort. Nutze den tatsächlichen aktuellen Branch-HEAD, nicht blind eine alte Commit-ID.

**Unbedingt zuerst lesen und abgleichen:** `AGENTS.md`, `docs/protocol-research/CURRENT_STATUS.md`, `research-checkpoint.json`, `phase1-closure-gates-2026-10-23.json`, `fourth-ring-ms-frontier-2026-10-24.json`, `fourth-ring-normative-reference-ledger.json`, `third-ring-original-bibliography-audit-2026-10-24.md`, `phase1-audit.md` und die letzten Einträge aus `research-log.md`. Prüfe den Branch-Stand, bevor Du änderst.

**Letzter überprüfter fachlicher Stand (24.10.2026, vor Erstellung dieses Übergabecommits):**
- Erste Ebene: 46/46 Microsoft-§1.2.1-Quellenabschnitte belegt, **464** direkte Normkanten.
- Zweite Ebene: 12/12 originalveröffentlichte §1.2.1-Quellenabschnitte belegt, **246** direkte Normkanten.
- Dritte Ebene: 52 MS-/MC-Originale untersucht; **48** explizite normative Referenzabschnitte mit **827** Kanten (398 MS/MC, 267 RFC, 162 extern/historisch). Vier `MS-ADA1`/`MS-ADA2`/`MS-ADA3`/`MS-ADSC`-§1.1-Schemabibliografien enthalten **67 UNCLASSIFIED** Referenznennungen, **nicht** automatisch Normkanten.
- 236 neue rohe Referenzlabels aus Ebene 3: 86 MS/MC, 62 RFC und 88 externe/historische. Raw-Label-Union der ersten drei Ebenen **581** – keine kanonisierte Produkt-Pflichtmenge.
- Vierte MS-/MC-Ebene: **86/86 Dokumentidentitäten beschafft**, aber **nur 1/86** Ausgabenbibliografien individuell geprüft. `MS-DLTM` rev **9.1**, PDF `v20170601`, §1.2.1 S.9 besitzt **10** normative Kanten (ein zusätzliches rohes Ziel `MS-DLTW`, bereits als PDF vorhanden); §1.2.2 S.9–10 besitzt **6** informative Quellen. Die übrigen **85** Bibliografien sind offen. In einem separaten `fourth-ring-normative-reference-ledger.json` stehen derzeit **10** Viertstufenkanten.
- Formaler Phase-1-Abschluss: **0/12 Gates PASS**, kein normativer Fixpunkt, keine bewiesene Voll-Client-Featurepflicht.

**Nächste konkrete Aufgabe – P1-03:** Untersuche **eine handhabbare erste Gruppe von ca. 6–10 der verbleibenden 85 Viertstufen-MS-/MC-Dokumente** in `fourth-ring-ms-frontier-2026-10-24.json`. Priorisiere Referenzquellen mit hoher Quervernetzung und nachvollziehbarer Relevanzhypothese, z. B. `MS-SMB2`, `MS-FSCC`, `MS-RPCH`, `MS-TLSP`, `MS-GPOL`, `MS-OXCMAPIHTTP` oder `MS-OXWSMSG`. **Vorher** nachsehen, ob ID/Version noch offen ist und ob die Originaldatei im aktuellen Chat zugänglich ist.

Für **jede** überprüfte Quelle:
1. Identität, exakte Originaledition/Revision/Datum, vollständige SHA-256 falls Originalbytes vorhanden, Bibliografieseite(n) und offiziellen Quelllink festhalten.
2. `§1.2.1 Normative References` und `§1.2.2 Informative References` strikt trennen; bei abweichenden Dokumentstrukturen wie `§1.1 References` die Normativität **ungeklärt** markieren.
3. Normative gerichtete `Quelle → Ziel`-Kanten mit rohem Zitatlabel, normalisiertem Ziel, Ausgabe und Seitenprovenienz **nur dem getrennten Viertstufen-Ledger hinzufügen**, nicht die Kanten früherer Ebenen überschreiben. Dubletten und historische Fassungen editionsgetreu erhalten.
4. Jeden Zielnamen gegen die bisherigen Ledgers und Frontier-IDs abgleichen; neue MS/MC, RFC, IETF-Drafts und externe Normen separat als nächste offene Frontier klassifizieren. Nicht aus bloßer Zitierung auf eine Pflicht des Skype for Business 2019 Windows Full Client schließen.
5. Belegte Client-/Serverrollen-/Policy-Anwendbarkeit und Ausschlüsse dokumentieren; ohne konkreten Produktnachweis den Status **OPEN** lassen.
6. Maschinell nachrechnen (Quellanzahlen, gerichtete Kanten, Rohlabel-Mengen, neue Frontier, 0/12 Gates) und konsistent `CURRENT_STATUS.md`, `research-checkpoint.json`, `phase1-audit.md`, `research-log.md`, die passenden Frontier-/Ledger-JSON-Dateien sowie einen exakten Quellenbericht aktualisieren.

**Original-PDF-Regel:** Der Nutzer **behält alle PDF-Binärdateien selbst**. **Keine PDFs in ein öffentliches oder privates Git-Repository übertragen**, keine erneute Massenupload-Bitte. Wenn eine Originalausgabe im aktuellen Arbeitskontext nicht mehr verfügbar ist, **nur die tatsächlich benötigte(n) konkrete(n) PDF-Datei(en)** beim Nutzer anfordern, vorher Repo-Quellnachweise und bereits verfügbare Dateien prüfen. Nicht behaupten, Vollbinary-SHA oder Original-PDF-Seiten selbst verifiziert zu haben, wenn nur ein alter Hash-/Seitenbeleg vorliegt.

**Sicherheits- und Arbeitsgrenzen:** Keine Protokollimplementierung/Quellcodeänderungen, keine Echtserver- oder Interoperabilitätstests, keine bezahlten CI-Runner, keine TLS-/Zertifikatsicherheitsabsenkung, keine GitHub-Merges nach `main`, keine Tags oder Releases. Der bestehende Draft-Branch ist der einzige Schreibziel-Branch. Keine Phase-1-Freigabe ohne erfüllte und unabhängig geprüfte formelle Gates.

**Zum Abschluss derselben Sitzung:** Änderungen in einem nachvollziehbaren Dokumentations-Commit auf dem Draft-Branch speichern; den finalen HEAD, geänderte Dateien und aktualisierte Kennzahlen über GitHub zurücklesen und verifizieren. Den neuen Stand in `CURRENT_STATUS.md` als klaren Wiederaufnahmepunkt festhalten. Antworte auf Deutsch, nenne Quellenbelege, offene Fragen und den konkreten nächsten Batch. Arbeite so weit wie mit vorhandenen Quellen tatsächlich möglich, statt bloß einen weiteren Plan zu schreiben.
