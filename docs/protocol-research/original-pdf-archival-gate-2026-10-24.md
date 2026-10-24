# Original-PDF-Archivierung: Repository- und Rechte-Gate (2026-10-24)

**Verbindliche Evidenzregel:** Eine local gehashte Original-PDF darf **nicht** als "im Git-Repository archiviert" markiert werden, solange weder der exakte Git-Blob-Hash noch der Commit/Branch und ein wiederabrufbarer Repository-Pfad für *diese Binärdatei* verifiziert wurden. SHA-256 des Original-PDFs und Git-Blob-SHA sind unterschiedliche Prüfsummen.

## Warum die 74 PDFs aktuell nicht im öffentlichen CommSCraft-Git sind

Die userbereitgestellte `OfficeProto.zip` wurde lokal komplett entpackt, 74 PDFs mit SHA-256 geprüft. Die Original-Dateien enthalten Microsofts Copyright-Hinweis, welcher Kopien für die **Entwicklung von Implementierungen** erlaubt, jedoch für die Weitergabe ausdrücklich **Teile** in Implementierungen bzw. Implementierungsdokumentation nennt. Ein pauschales Recht auf **öffentliche Weiterverteilung vollständiger PDFs** steht dort nicht. `NelvinBHV/CommSCraft` ist öffentlich. Die Originalbinaries wurden deshalb **nicht** in diesen öffentlichen Draft-Branch hochgeladen. Zudem kann die vorhandene GitHub-Connector-Schreibschnittstelle nur explizite Inhaltsstrings übernehmen; sie nimmt keine bereits lokal entpackten Binärpfade entgegen.

## Empfohlene richtlinienkonforme Ablage, sofern die Nutzungsrechte dies erlauben

1. Einen **privaten** Quellen-Repository-Remote verwenden, auf den nur die berechtigten Mitwirkenden Zugriff haben. Nicht aus dem privaten Quellenarchiv automatisch nach `CommSCraft/main` veröffentlichen.
2. `OfficeProto.zip` unverändert lokal sichern; ZIP-SHA-256: `27f31dc1020d284159a44cecef5233aee5856814271cbf20f4f7ee1b8dd452f8`. Alle 74 PDFs nach SHA-256 mit dem maschinell erzeugten lokalen `SHA256SUMS`-Manifest prüfen.
3. Unter `protocol-originals/MS-*/vYYYYMMDD/[MS-...].pdf` revisionsfeste Binärdateien ablegen. Historische Revisionen (**MS-ABS 10.0**, **MS-SIPREGE 17.0**, **MS-SSRTP 11.0**) strikt von den aktuelleren Veröffentlichungen trennen. ZIP-Digests bei MS-AVEDGEA/SIPAE/OCSPROT nicht über frühere abweichende Digests schreiben.
4. Den **privaten** Git-Commit und alle 74 Binärpfade zurücklesen, Git-Blob-SHA und tatsächliche PDF-SHA-256 gegentesten, dann öffentliche Forschungsdokumente höchstens mit IDs/Revisionen/Hashes/zulässigen URLs annotieren.
5. Falls ausdrücklich und rechtswirksam die Erlaubnis zur vollständigen öffentlichen Weiterveröffentlichung eingeholt wird, kann die Publikationsentscheidung separat dokumentiert werden. Bis dahin ist "public full PDF archive" **OPEN**, nicht `COMPLETE`.

Für künftige Quellfortsetzungen: **Original-PDF lokal akquirieren → vollständiges SHA-256 + Edition + §-Auszug prüfen → Lizenz/Reposichtbarkeit prüfen → im zulässigen Binär-Repository committen und dort erneut verifizieren → erst dann als archiviert bezeichnen.** Der GitHub-Quellnachweis bleibt strikt von der inhaltlichen Client-/Protokollnorm-Prüfung getrennt.

**Keine Implementierung, echten Serverkontakte, Interoperabilitätstests, `main`-Merges oder Releases.**

## Ergänzung 2026-10-24: Windows-Originalarchiv und 822 bisher geprüfte PDFs

Der Nutzer lieferte `Windows_Protocols.tar.xz` (SHA-256 `52f87efc17c9854f70b0b14650c877a572b1a99c7ed9df4f6c9a96387720c652`). 451 Original-PDFs wurden validiert/gehasht, damit **822** einzelne PDF-Dateien über alle drei Archivlieferungen. Die lokale SHA256SUMS-Datei enthält 451 Einträge (eigene Manifest-SHA-256 `e81077d69bea8493947e7588130425ef4237789fcf2a035d0ad4fd8b6aaadc27`). Das zusätzlich herunterladbare Forschungsnachweis-ZIP beinhaltet Prüfsummen, JSON-Metadaten und normative Kantenkontrollen, **nicht** die vollständigen PDFs. Der Umstand, dass die 12 direkten Microsoft-Zweitring-PDF-Ausgaben jetzt vollständig verifiziert sind, schließt weder das Rechte-/Privatarchiv-Gate noch den Norm-Fixpunkt. Bei MS-NLMP, MS-RDPEMC und MS-SIP verschiedene Hashes gleicher Editionen separat protokollieren; frühere Evidenzbytes nicht überschreiben. Auch künftig keine Behauptung „PDF liegt im Repository“ ohne tatsächlichen Git-Binär-Blob und Rechtefreigabe.
