# P1-03 – Zehn Originalbibliografien: Active Directory, Druck, Dateireplikation und RDP

**Stand:** 24.10.2026. **Originalbasis:** zehn unveränderte Windows-Original-PDFs aus dem vom Nutzer bereitgestellten `Windows_Protocols.tar.xz`. Die gesamten PDF-Bytes wurden lokal SHA-256-geprüft; jeweilige `vYYYYMMDD`-Originalfußzeile, Revisionshistorie, **§1.2.1 Normative References** und **§1.2.2 Informative References** ausgelesen. Erste Referenzseiten sind zusätzlich gerendert und visuell geprüft. Keine PDF-Binärdateien in Git. Rohlabel, exakte PDF-Seite, Quellrevision und Quell-Hash je gerichteter Kante im [Ledger](fourth-ring-normative-reference-ledger.json) und in [Einzelbeleg-JSON](fourth-ring-rdp-print-directory-originals-audit-2026-10-24.json).

## Vollständig geprüfte Originalquellen

| Quelle | Originaledition / Revision | PDF-Seiten | §1.2.1 normativ (Seiten) | §1.2.2 informativ (Seiten) | SHA-256 komplette PDF |
|---|---|---:|---|---|---|
| MS-FRS1 | `v20240916` / **29.0** | 250 | **20**, 14, 15 | **1**, 15 | `00fd7df3ae4d063d34387dcf47b514f9c552815eb8e83dfc4acfdc7ca00eab7f` |
| MS-RCMP | `v20240423` / **16.0** | 24 | **14**, 7, 8 | **4**, 8 | `2ece3a1047ed7d51a87eec4208eb10412115363f6ef5acdd917f48c94215d235` |
| MS-RDPEAR | `v20240423` / **10.0** | 70 | **20**, 8, 9 | **6**, 9 | `a4795c1eff7659fd7053e2fef93c6112793533b8320abe4b1e429feef227b16a` |
| MS-RDPEDC | `v20170601` / **8.0** | 35 | **3**, 7 | **3**, 7, 8 | `35eab722ef79a702b9cb781eeed355623a5081fef818a038299d681a59bcd795` |
| MS-RDPEDYC | `v20240423` / **21.0** | 62 | **6**, 9 | **0** („None.“ bzw. keine Einträge) | `68a3a8a32e482c7dae7413c36158084efd2542dc171446fb54823485b67ec0c5` |
| MS-RDPEGFX | `v20260511` / **19.0** | 145 | **9**, 9 | **3**, 9 | `0acb3b2454deb101aee96619ef8586eca0143b9c06342c42919ba41c257f37c9` |
| MS-RDPEUDP2 | `v20240423` / **6.0** | 37 | **2**, 6 | **0** („None.“ bzw. keine Einträge) | `09a87ff1fb8c5ed2ba0eaadae0b251e1c46a95b5ce576333a27cbe6bbea147ae` |
| MS-RDPEVOR | `v20240423` / **14.0** | 28 | **9**, 5, 6 | **0** („None.“ bzw. keine Einträge) | `521f06c513441df83c748c4ecc2307b26f9b01f3e9a25d0c0f0577a74e10a9c3` |
| MS-RPRN | `v20250811` / **39.0** | 449 | **29**, 22, 23 | **22**, 23, 24 | `e5bb06ca3574f90cb9a0dad7ffa688b37b75323680ac0c41973048ccaf029c29` |
| MS-SAMS | `v20240729` / **18.0** | 38 | **17**, 9, 10 | **0** („None.“ bzw. keine Einträge) | `c59a11780125a3a7314d15dff20d34074948e28094b7657cf4b815f582e59b99` |
| **SUMME** | 10 verschiedene Originalquellen | | **129** | **39** | alle zehn vollständigen Dateien geprüft |

**Archivpfad je Quelle:** `Windows_Protocols/[ID].pdf` innerhalb `Windows_Protocols.tar.xz`. Die genauen Archivmitglieder stehen für jeden Beleg im JSON. Die Original-PDFs werden vom Nutzer verwahrt.

## Verifizierte Beziehungen und Abgrenzungen

1. **Zyklus ist Literaturzitat, kein implementierbarer Endlosrekursionsauftrag:** `MS-RDPEDYC v20240423 §1.2.1 → MS-RDPEGFX` auf PDF-S.9 und `MS-RDPEGFX v20260511 §1.2.1 → MS-RDPEDYC` auf PDF-S.9. Die jüngere Grafik-Ausgabe vom **11.05.2026** (Revision 19.0) belegt weder rückwirkend eine Pflicht noch ein vorhandenes 2019-Client-Feature. Frühere Zielversionen und Windows-Build-Kontext bleiben **OPEN**.
2. **Druck und File Replication nennen Schema-Originale normativ:** `MS-FRS1 v20240916 §1.2.1 → MS-ADLS` (S.15) sowie `MS-RPRN v20250811 §1.2.1 → MS-ADLS` (S.22). Die **13 zuvor extrahierten allgemeinen `MS-ADLS §1.1 References`** bleiben dennoch **UNCLASSIFIED**, weil MS-ADLS keinen eigenen normativen §1.2.1-Abschnitt besitzt. Dessen Quellidentität verbleibt im offenen Frontier.
3. **RDP-Abhängigkeiten:** `MS-RDPEAR → MS-RDPEDYC` (S.9), `MS-RDPEVOR → MS-RDPEGFX` (S.6) und `MS-RDPEUDP2 → MS-RDPEUDP` (S.6) sind normative bibliografische Verweise. Das belegt **keine** allgemeine RDP-Video-, Grafik- oder Transportpflicht im Skype-for-Business-Windows-Full-Client.
4. **Informative Referenzen bleiben getrennt:** `MS-RDPEAR → MS-RDPBCGR` steht nur im informativen Abschnitt (PDF-S.9), anders als `MS-RDPEAR → MS-RDPEDYC`. `MS-RPRN` enthält **22** nur informative Quellen, darunter `MS-PAN` (S.24). `MS-RDPEUDP2`, `MS-RDPEVOR`, `MS-RDPEDYC` und `MS-SAMS` enthalten kein informatives Literaturziel.
5. **Historische Normeditionen nicht stillschweigend vereinheitlichen:** `MS-RDPEGFX` nennt u. a. `ITU-H.264-201201`, `ITU-BT.709-5` und `RFC2119` normativ. `MS-RPRN` nennt `RFC8010` und `RFC8011`; `MS-RCMP` verweist normativ auf `RFC4556`, `RFC5280` und `X690`. Zielversionen bleiben bis zur separaten Prüfung als **UNRECONCILED** gespeichert.

## Nicht in diesen Viertstufen-Batch einsortierte Originale

Die zusätzlich verfügbaren Original-PDFs `[MS-DLTM]-170601.pdf` / `[MS-DLTM]-170601(1).pdf` (byteidentisch, SHA-256 `46c0c9acb95e710beb2a8b61f6a0c635ae91b41456e919ad1c92b1e754db90f8`) und `[MS-DLTW]-210625.pdf` (SHA-256 `28a8b2a3e57e2594c0d96501679cf413841c432975364cb2ad87a85a09fe8a15`) sind **keine der neun offenen Viertstufenquellidentitäten**. Sie wurden daher nicht künstlich in den 86er-Fortschrittszähler aufgenommen. Etwaige normative Erreichbarkeit auf einer nachgelagerten Ebene ist gesondert festzustellen.

## Forschungszähler nach Prüfung

| Referenzebene | Gerichtete normative Kanten | Status |
|---|---:|---|
| Erste | 464 | unverändert |
| Zweite | 246 | unverändert |
| Dritte | 827 | unverändert; 67 ältere AD-Schema-Nennungen weiterhin UNCLASSIFIED |
| **Vierte** | **1.061** | **77/86** Quellen mit geprüften Original-§1.2.1-Verzeichnissen; **9** Quell-IDs offen, darunter Schema-Sonderfall MS-ADLS |
| Fünfte | 6 | erste separat geprüfte MS-SWN-Quelle, kein Gesamt-Fixpunkt |

**Dieses Batch:** 129 neue normative Kanten, 39 separat informative Zitate. Offen bleiben die acht anderen unvollständig geprüften vierten Quellen `MC-NBFX`, `MS-CTA`, `MS-DPSP`, `MS-EMFPLUS`, `MS-MQMQ`, `MS-N2HT`, `MS-PAN`, `MS-RAI` und der bekannte `MS-ADLS`-Schemaabschluss. Die Quellenliste ist im [Frontier-JSON](fourth-ring-ms-frontier-2026-10-24.json) maßgeblich.

**Sperren:** 0/12 formelle Phase-1-Abschlussgates PASS. Kein vollständiger normativer Fixpunkt, keine editionsgenau abgeschlossene externe RFC-/ISO-/ITU-Rekursion und keine geklärte Build-/Gegenserver-/Policy-Funktionspflicht des Skype-for-Business-2019-Windows-Full-Clients. Keine Implementierung, keine Netz-/Echtserver-Interoperabilitätstests, keine kostenpflichtige CI, keine Authentisierungs-/TLS-Abschwächung, kein `main`-Merge, Tag oder Release.
