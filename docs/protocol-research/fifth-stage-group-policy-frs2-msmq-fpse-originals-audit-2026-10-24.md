# P1-03 – Acht nachgelagerte MS-/MC-Originalbibliografien: FRS2, Gruppenrichtlinien, MSMQ und FrontPage

**Forschungsdatum:** 24. Oktober 2026 · **Branch:** `draft/protocol-spec-analysis-2026-10-23`. Die Original-PDFs wurden aus den vom Nutzer gehaltenen Archiven `Windows_Protocols.tar.xz` (sieben) und `SharePointProtocols.zip` (ein `MS-FPSE`) entnommen. Die **vollständigen PDF-Dateibytes** wurden mit SHA-256 geprüft; Originaledition/Fußzeile `vYYYYMMDD`, Revision und die Literaturabschnitte `§1.2.1 Normative References` und `§1.2.2 Informative References` mit tatsächlichen Original-PDF-Seiten verifiziert. Die Kopfseiten dieser Abschnitte sind zusätzlich gerendert und visuell überprüft. Die Binär-PDFs bleiben beim Nutzer und sind **nicht** Teil von Git.

## Editionsgenau geprüfte Originalquellen

| ID | Revision / Originaledition | PDF-Seiten | Normativ §1.2.1 (PDF-S.) | Informativ §1.2.2 (PDF-S.) | SHA-256 der vollständigen PDF |
|---|---|---:|---|---|---|
| MS-FPSE | **23.0** / `v20240416` | 80 | **12** (9, 10) | **0** (keine) | `90c52f29a126e262990b0f5f2a894cd408d454e31b22e951fd6b09888d9acb09` |
| MS-FRS2 | **30.0** / `v20240423` | 123 | **19** (11, 12) | **1** (12) | `cced0b75dea731698290443fc95aea18599141b79ce32a023c308a0ef37ab03f` |
| MS-GPFR | **15.0** / `v20240423` | 34 | **5** (9) | **1** (9) | `49a9ab1826e071daed334b3577e308de968203050fcf7b2c237f4d874147299e` |
| MS-GPIPSEC | **27.0** / `v20240423` | 84 | **15** (8, 9) | **10** (9) | `8469b89ae467b735a66e22a4014f907e73438c764a674b3bfa19ae6c6e83d94b` |
| MS-GPSCR | **25.0** / `v20240423` | 30 | **5** (7) | **3** (7) | `ffda6d8b94bb87f043153ec64ebb5c42ce83155a639f62f0aad3d512d27dd3d9` |
| MS-GPSI | **26.0** / `v20240423` | 64 | **14** (11) | **5** (11, 12) | `62e58f0da7ea858dd4012c873618ca8d1e5dae435cbe0e8f10621de76927d26a` |
| MS-MQMR | **18.0** / `v20240423` | 37 | **10** (8, 9) | **3** (9) | `365e2e7476e36b61dda1573b689d749a25562b04c5359d81bd34ba11994aad4d` |
| MS-MQRR | **29.0** / `v20251121` | 96 | **15** (11) | **4** (11) | `2fbb9f469902204a92ca050302be730312c749f48dace4e55d5ce3d5326e9adf` |
| **SUMME** | acht getrennte Originale | | **95** | **27** | acht vollständige SHA-256 |

Archivzuordnung pro Original: `MS-FPSE` aus `SharePointProtocols.zip` / `[MS-FPSE].pdf`; sieben Windows-Protokolle aus `Windows_Protocols.tar.xz` / `Windows_Protocols/[ID].pdf`. Der vollständige [maschinenlesbare Audit](fifth-stage-group-policy-frs2-msmq-fpse-originals-audit-2026-10-24.json) enthält jede Referenz mit Rohlabel und Quell-PDF-Seite. Die **95** zusätzlichen gerichteten normativen Literaturkanten werden im [nachgelagerten Kantenledger](fifth-ring-ms-swn-normative-ledger.json) mit Ausgabe, Revision, originalem SHA-256 und Archivmitglied geführt. Informative Nennungen sind separat dokumentiert.

## Besonders wichtige Befunde und Grenzen

1. **MS-MQRR mit tatsächlicher Selbstreferenz:** `MS-MQRR v20251121` Revision **29.0** nennt auf **PDF-S.11 in §1.2.1** `[MS-MQRR]` ausdrücklich zwischen `[MS-MQQP]` und `[MS-RPCE]` als **normative Quelle**. Dies ist **kein automatisch eingesammelter PDF-Kopf oder -Fuß**. Die Selbstkante bleibt mit `explicit_bibliography_self_citation: true` im Kantenledger und muss bei rekursiver Traversierung als bereits besuchter Knoten behandelt werden – nicht als endloser Auflösungsbedarf oder als Beweis eigener Konformität. Microsoft veröffentlicht die Ausgabe [MS-MQRR Revision 29.0, 21.11.2025](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-mqrr/9edbc8fa-02ad-4c79-804f-6bb8f430aac1). Normative Bibliografie ≠ vollständige Skype-for-Business-Clientpflicht.
2. **MS-FPSE** v20240416 rev **23.0** enthält **12** normative Originalreferenzen aus §1.2.1, aber in **§1.2.2 auf PDF-S.10 ausdrücklich „None.“**. Die Protokollspezifikation bezieht sich auf serverseitige HTTP-Erweiterungen zur Website-Dateiverwaltung und ist keine allgemeine Skype-Full-Client-Funktionspflicht. [Offizielle Edition und Vorgängerversionen](https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-fpse/1bbae50a-7de9-45ea-830d-4e9caf8dfccd); [Microsoft empfiehlt für die Anwendung WebDAV statt FPSE](https://learn.microsoft.com/en-us/openspecs/sharepoint_protocols/ms-fpse/b47e6cf8-2ec4-4d69-b0ac-fb90aade3a8a).
3. **Dateireplikation MS-FRS2:** normativ `MS-FRS2 → MS-FRS1`, `→ MS-BKUP`, `→ MS-XCA` (PDF-S.11–12); `MS-FRS2 → MSDN-RPC` ist **informativ** (S.12). `MS-ADLS` wird normativ referenziert, aber sein eigenes §1.1-Allgemeinverzeichnis mit **13 UNCLASSIFIED** Literaturangaben wird nicht in eine normative §1.2.1-Bibliografie umgewertet.
4. **Gruppenrichtlinien:** `MS-GPFR → MS-GPOL` und `→ MS-SMB` (S.9), `MS-GPIPSEC → MS-GPOL` (S.9), `MS-GPSCR → MS-GPOL` (S.7) sowie `MS-GPSI → MS-GPOL` (S.11) sind normativ. Aus ihnen folgt nicht, dass ein Skype-for-Business-2019-Windows-Client alle Administrations-, Installations- oder Gruppenrichtlinien-Erweiterungen selbst implementieren muss.
5. **Historische IPsec-/IKE-Ausgaben:** `MS-GPIPSEC` nennt `RFC2408`, `RFC2409` und `RFC2412` normativ (PDF-S.9), jedoch `RFC2401`, `RFC2402`, `RFC2406`, `RFC2407` und `RFC3947` nur informativ. Microsoft beschreibt den Mechanismus als Gruppenrichtlinienerweiterung zur Konfiguration lokaler IPsec/IKE-Komponenten, nicht als Freigabe von Schutzabsenkungen. [Offizielle Dokumentzuordnung](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gpipsec/dc377790-7429-47fc-9b55-59589e7a586b).
6. **MSMQ Queues:** `MS-MQMR → MS-MQMQ`, `→ MS-MQDMPR` und `→ MS-MQQB` sind normativ (S.8); `MS-MQMR → MS-MQOD` lediglich informativ (S.9). `MS-MQRR` referenziert außerdem `MC-MQSRM`, `MS-MQQP` und `MS-DTCO` normativ (S.11), während `MS-MQBR` und `MS-MQOD` informativ bleiben. Das ergibt neue transitive Zielkandidaten, keine sofortige Funktionserweiterungspflicht.
7. **Editionen und Versionsgrenze:** `MS-MQRR v20251121` ist jünger als die meisten hier geprüften `v20240423`-Windows-Spezifikationen; `MS-FPSE` trägt `v20240416`. Die zitierten historischen Zieloriginaleditionen sind **UNRECONCILED**, und die Quelldokumentedition darf nicht rückwirkend als Stand eines konkreten Skype-for-Business-Windows-Builds von 2019 gewertet werden.

## Kontrollierte Zähler

| Prüfebene | Nach diesem Audit |
|---|---|
| Frühere Ringe 1–3 | **464 / 246 / 827** gerichtete Normkanten – unverändert |
| Vierte Quellebene | **86/86 Quellidentitäten klassifiziert**; **85** explizite §1.2.1-Normbibliografien plus ein `MS-ADLS`-§1.1-Schema; **1.147** Normkanten – unverändert |
| Nachgelagerte MS-/MC-Originalquellen | **33** Normbibliografien aus Original-PDF geprüft, **442** normativ gerichtete Kanten (**347 + 95**), **111** informative Zitate (**84 + 27**) |
| MS-/MC-Workqueue aus viertstufigen Normzielen | **88** Rohlabel-Kandidaten, davon **33** als §1.2.1-Originalquellen geprüft, **1** MS-ADLS-Schema-Sonderfall klassifiziert, **54** reguläre noch offen |
| Schema-Bibliografien | 67 ältere Drittstufen- und 13 MS-ADLS-Verweise bleiben **UNCLASSIFIED** |
| Phase-1-Schlussprüfung | **0/12 formelle Gates PASS** |

**Nächster Wiederaufnahmepunkt:** 6–10 weitere tatsächlich verfügbare Originalquellen aus der [aktuellen Workqueue](fifth-stage-fourth-target-ms-mc-workqueue-2026-10-24.json) prüfen. Besonders wichtig sind die aus MSMQ entstandenen neuen Ziele `MS-MQDMPR`, `MS-MQQP` und `MC-MQSRM`; diese müssen zuerst mit sämtlichen früheren Quellen-IDs abgeglichen werden, bevor sie einer neuen transitive Ebene zugeordnet werden. Die ursprüngliche Workqueue der 88 Viertstufenziele ist **keine vollständige Fixpunkt-Workqueue**, sondern bleibt auf diesen Basisring begrenzt. Zusätzlich externe IETF/RFC/ISO/ITU/W3C-Ausgaben historisch auflösen, unabhängige Referenzabschlussprüfung erstellen und tatsächlichen 2019-Windows-Full-Client-Build-/Serverrollen-/Policy-Scope belegen.

**Sperren bleiben:** Keine Protokollimplementierung, keine Live-Netzwerk- oder Interoperabilitätstests, kein Einsatz kostenpflichtiger Runner, keine TLS-/Authentisierungsschutzabsenkung, kein Merge nach `main`, Tag oder Release. Alle Original-PDFs beim Nutzer.
