# P1-03 – Zehn weitere Originalbibliografien zu Exchange Web Services: Kalender, Termine, Benachrichtigungen und Personendaten

**Quellenprüfung: 24.10.2026.** Quelle sind zehn vollständige Nutzer-Original-PDFs aus `Exchange_Protocols.zip` (gleicher Originalpfad im Archiv: `[MS-ID].pdf`). Alle zehn PDF-Binärdateien wurden mit SHA-256 geprüft; Revisionshistorie, Release-Fußzeile `vYYYYMMDD`, exakte `§1.2.1 Normative References` und `§1.2.2 Informative References` wurden aus dem jeweiligen Original ausgelesen. Erste Norm- und Informationsreferenzseiten wurden gerendert und visuell überprüft. **Kein PDF-Binary im Git-Repository** – der Nutzer hält die Originale.

## Editionsgenaue Quellenübersicht

| ID | Revision / Originaledition | PDF-Seiten | Normative §1.2.1-Belege | Informative §1.2.2 | Vollfile-SHA-256 |
|---|---|---:|---|---|---|
| MS-OXWSGTRM | **10.1 / v20250819** | 33 | **10**, S.7–8 | **3**, S.8 | `211869af38050e08226da07a76042909e3fcb0f97358d03a7d7ce1a77f09bf30` |
| MS-OXWSGTZ | **9.0 / v20250520** | 41 | **10**, S.7–8 | **12**, S.8 | `4c0f2637270fa5e1d9d88242d73e71e75951ccaa0ebb000f4b1051c7fc0e8457` |
| MS-OXWSMSHR | **13.0 / v20250520** | 44 | **11**, S.7–8 | **3**, S.8 | `58a8a24311bf826e9e5285edbcf75ff1f0c768ca3f0840128737b2431ee6b3f1` |
| MS-OXWSMTGS | **14.0 / v20250520** | 106 | **18**, S.9–10 | **3**, S.10 | `19f5c34887c2686b5aa1dc7d297d89b4c3566d0919373cce1a9dab1e77cb0268` |
| MS-OXWSMTRK | **13.0 / v20250520** | 50 | **9**, S.7–8 | **3**, S.8 | `b11ec8472ce49d25f59775ef0c3efb9362100292616d204ae6718a74a33274eb` |
| MS-OXWSNTIF | **12.0 / v20250520** | 56 | **12**, S.8–9 | **4**, S.9 | `ed7ab47fb006b7ac30a482aeba0a96209f3933ee9932f1d30c9ff7d6f6d7c918` |
| MS-OXWSPED | **7.0 / v20250520** | 25 | **10**, S.6–7 | **3**, S.7 | `2dcfa67fa5e16d20d219aa9283db07c283a06e8e92212292d16c3b778e6d4015` |
| MS-OXWSPERS | **7.0 / v20250520** | 58 | **16**, S.7 | **3**, S.7 | `0154856d6f82c77b44aaaeea61a9cb80abc803defbdbc1ebddd32af02c13e9f4` |
| MS-OXWSPHOTO | **11.0 / v20250520** | 15 | **10**, S.5 | **2**, S.5 | `b4f87bc8f6ceab4e94f5bc26fba34a591a4432cb569d4444a6e851f6b236ca35` |
| MS-OXWSPOST | **10.0 / v20250520** | 38 | **14**, S.7–8 | **2**, S.8 | `b8011011566ef29da6cda8fe231795b5d66b981cee11214aec11c553c922fb1d` |
| **SUMME** | | | **120** | **38** | **10 vollständig gehashte Originale** |

Genaue einzelne Rohlabels und die jeweilige Original-PDF-Seite stehen im [Audit-JSON](fourth-ring-oxws-calendaring-notifications-originals-audit-2026-10-24.json); die **120 neuen gerichteten** §1.2.1-Kanten stehen außerdem im [Viertstufen-Normledger](fourth-ring-normative-reference-ledger.json). Referenz-URLs: [offizielles Microsoft-Exchange-Web-Service-Protokollverzeichnis](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxproto/e91d2bc8-687f-4ca2-bc68-2b3854eb5113); [Originaleditionsregister für MS-OXWSGTRM](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxwsgtrm/46852f67-8163-4569-82f6-5c91c9cf2b72).

## Gerichtete Beziehungen – normativ versus informativ

- **MS-OXWSMTGS §1.2.1 → MS-OXWSGTZ und → MS-OXWSPERS:** beides ausdrücklich normativ (S.9). **MS-OXWSPERS §1.2.2 → MS-OXWSMTGS** steht dagegen **nur informativ** (S.7). Eine Rückrichtung wird nicht automatisch normativ.
- **MS-OXWSGTZ enthält zwölf ausschließlich informative MS-/MC-Verweise** unter §1.2.2, einschließlich `MS-OXWSATT`, `MS-OXWSCONT`, `MS-OXWSDLIST`, `MS-OXWSMTGS`, `MS-OXWSPOST`, `MS-OXWSSYNC` und `MS-OXWSTASK`. Demgegenüber ist `MS-OXWSGTZ → MS-OXWSCDATA` normativ. Das Referenzverzeichnis beweist nicht, dass jeder Verbraucher seiner Zeitinformationen als normative Vorgabe in GTZ stünde.
- **MS-OXWSNTIF §1.2.2 → MS-OXWSPSNTIF:** informativ, nicht normativ (S.9). Die tatsächlich einschlägige Push-/Pull-/Streaming-Abhängigkeit muss aus konkreter Operation, Produktrolle und Policy belegt werden.
- **MS-OXWSPHOTO §1.2.2 → MS-OCAUTHWS:** informativ (S.5); die bloße Erwähnung beweist keine Autorisierungs- oder Photo-Service-Funktion im SfB-2019-Client.
- **MS-OXWSPOST §1.2.1 → RFC850:** roh zitiert normativ (S.8). Das historische RFC-Label bleibt als solches erhalten, ohne den Zielstandard oder seine möglichen Errata/Obsoletes stillschweigend zu aktualisieren.
- **MS-OXWSPERS** nennt die zwei exakt ausgabenbezogenen Rohlabels `XMLSCHEMA1/2` und `XMLSCHEMA2/2` normativ (S.7). Sie bleiben von `XMLSCHEMA1` und `XMLSCHEMA2` getrennt.
- **MS-OXWSGTRM – wichtige Versionsabweichung:** Original `v20250819`, Revision **10.1** (Microsoft verzeichnet 19.08.2025). Die **übrigen neun** Originale sind `v20250520`. Der Verweisbaum wird nicht fälschlich auf einen einheitlichen 2025-05-20-Stichtag nivelliert.

Fünf rohe Norm-Zielkennungen waren im bislang separaten **Viertstufen-Normledger** noch nicht als Ziele registriert: `MS-OXSHRMSG`, `MS-OXWSBTRF`, `MS-OXWSDLGM`, `MS-OXWSPERS`, `RFC850`. Dies bedeutet **nur „neu als Ziel in diesem Ledger“**, nicht „weltweit neu“, „noch nicht beschafft“, „fünftstufig neu“ oder „SfB-Client-pflichtig“; Quell- und Zielversionen müssen gegen sämtliche früheren Ebenen/Archividentitäten geprüft werden.

## Rechnerische Trennung der Referenzebenen

| Stufe | Normative gerichtete Kanten | Quellen-/Auditumfang |
|---|---:|---|
| Erste | 464 | bisher unverändert |
| Zweite | 246 | bisher unverändert |
| Dritte | 827 | davon vier AD-§1.1-Schema-Verzeichnisse mit 67 separaten **UNCLASSIFIED** Nennungen |
| **Vierte** | **810** | **57/86** geprüfte Quellen, **29** offen |
| Fünfte | 6 | MS-SWN separat; keine abgeschlossene Fünftstufenfrontier |

Für diese zehn neuen Quellen **120 normative Kanten + 38 informative Nennungen**, getrennt. **Die späteren (2025er) Exchange-Spezifikationen sind kein hinreichender Beleg für vollständige Anwendbarkeit auf einen Skype for Business 2019 Windows Full Client.** Build, Gegenserver, Deployment, angebotene Funktionen, historische Ausgaben sowie Client-/Serverrollen und Policy-Beschränkungen bleiben OPEN. Kein rekursiver Norm-Fixpunkt und kein Abschluss einer externen RFC-/IETF-/ISO-Wirkungskette nachgewiesen.

**Formeller Status:** Phase 1 **INCOMPLETE, 0/12 PASS**. Keine Protokollimplementierung, keine Live-Interop-/Echtserver-Netztests, keine GitHub Actions mit kostenpflichtigen Runnern, keine TLS-/Kerberos-/NTLM-Schutzabsenkung, kein Merge nach `main`, keine Tags/Releases. Nutzer verwahrt sämtliche PDF-Originaldateien; GitHub speichert ausschließlich Forschungstext, Metadaten und SHA-256.
