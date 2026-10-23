# Original-PDF-Abgleich: Konferenz-IM und Präsenz-Endpunktlebensdauer

**Datum:** 2026-10-22. **Forschungsstand:** Abschnittsweise am Originaltext verifiziert, kein vollständiger Protokollabschluss. **Ausschließlich Dokumentation.** Primärquellen: die vom Projektinhaber bereitgestellten PDFs `catalog/MS-CONFIM.pdf` (v20250520, 81 Seiten) und `catalog/MS-PRES.pdf` (v20250520, 257 Seiten), jeweils mit gleichnamiger Textextraktion. Die normativen Abschnitte unten wurden in den Originaltexten nachgeschlagen. Sie sind nicht aus einer älteren HTML-Kurzfassung übernommen.

Offizielle Ausgangsquellen: https://learn.microsoft.com/en-us/openspecs/office_protocols/ und MS-OCSPROT v9.0 §2.5 zur Rolle von Präsenz, IM und Konferenzen.

## 1. MS-CONFIM: Konferenz-Nachrichten sind zweistufige Zustellvorgänge

| Anforderung | Originalabschnitt | Prüfung / Zustandsfolge |
|---|---|---|
| CONFIM-01 | §3.3.1.5.1 | Bei **200 OK** auf das vom Konferenzteilnehmer gesendete MESSAGE **MUST** der Client die Weiterleitung an alle übrigen Konferenzteilnehmer als erfolgreich behandeln. Dies ist die protokollspezifische Bedeutung, nicht die allgemeine Behauptung tatsächlicher Empfang durch jeden Client. |
| CONFIM-02 | §3.3.1.5.1 | Bei **202 Accepted** **MUST** der Client die `Message-Id` aus der Antwort lesen. Fehlt sie, gilt der definierte Standardwert **0**. Der Client startet `sendMessageTimer` und wartet auf IM Delivery Notification (IMDN). |
| CONFIM-03 | §3.3.1.5.1 | Stimmt die Kennung der IMDN mit jener des 202 überein, zeigt der Client den anhand des IMDN-Inhalts abgeleiteten Weiterleitungsstatus an und beendet den noch laufenden Timer. Eine fremde IMDN-ID darf nicht als Abschluss dieser Nachricht dienen. |
| CONFIM-04 | §§3.3.1.5.1, 3.3.1.6 | Nach Timerablauf wird ein fehlgeschlagener Zustellbericht mit Status **408** für alle Empfänger der betreffenden Nachricht angenommen. `sendMessageTimer` ist **MUST** und hat standardmäßig **10 Sekunden**. Nicht mit dem RFC-3261-Clienttransaktionstimer verwechseln. |
| CONFIM-05 | §3.3.1.5.2 | Für ein vom MCU weitergeleitetes MESSAGE sendet der Client **MUST** 200 OK. Der Absender ist, falls vorhanden, aus `Ms-Sender` zu entnehmen; die `From`-URI enthält die Konferenz-URI und darf nicht als ursprünglicher persönlicher Absender fehlgedeutet werden. |
| CONFIM-06 | §3.3.2.5.1 | MCU-seitig bei nur einem Teilnehmer: **MUST** 200 OK (ohne spätere Zustellbenachrichtigung). Mit weiteren verbundenen Teilnehmern: **SHOULD** 202. In beiden Fällen **MUST** der MCU eine `Message-Id` mit aktuellem Zähler angeben und diesen erhöhen; Weiterleitung nutzt dieselbe Kennung. |
| CONFIM-07 | §§3.3.2.1, 3.3.2.5.1 | MCU hält Konferenznachrichtenhistorie während der ersten **40 Sekunden** nach Erzeugung vor und kann frühe Nachrichten an spät beitretende Teilnehmer verteilen; Status-/Inhaltsformat und Feature-Unterstützung (u. a. `Ms-Sender`, multipart/alternative) beeinflussen die Weiterleitung. |

**Prüfgrenze:** §3.3.2.5.1 schreibt MCU-Verhalten vor, nicht automatisch gleiches Clientverhalten. Die exakte IMDN-XML-Syntax in MS-CONFIM §§2.2.2 und 6.2 ist für die vollständige Byte- und Zustandsanalyse noch zu extrahieren. 202 bestätigt die Annahme zur Weiterleitung, **nicht** ihren endgültigen Erfolg.

### Ablaufmodell (normative Teilfolge)

```mermaid
sequenceDiagram
 participant C as Client
 participant M as Konferenz-MCU
 participant P as Andere Teilnehmer
 C->>M: MESSAGE
 alt nur Client verbunden
 M-->>C: 200 OK + Message-Id
 Note over C: Als erfolgreich weitergeleitet behandeln
 else weitere Teilnehmer verbunden
 M-->>C: 202 Accepted + Message-Id
 C->>C: sendMessageTimer = 10s (Standard)
 M->>P: MESSAGE + Message-Id
 alt passende IMDN vor Timeout
 M-->>C: IMDN mit derselben Message-Id
 C->>C: Zustellstatus aus IMDN; Timer stoppen
 else Timeout
 C->>C: Zustellung je Empfänger als 408-Fehler behandeln
 end
 end
```

**Noch offen:** Nachrichtenbearbeitung bei 4xx/5xx, MIME-Unterstützung, Forward-Status aller Empfänger, erneute MCU-Verbindung und MCU-Leistungsmerkmale nach historischem Windows-Client-Profil.

## 2. MS-PRES: Registrierung steuert Präsenzinstanzen und Benachrichtigungen

Direkt geprüfter Originalabschnitt **MS-PRES v20250520 §3.2.5.5.1 (User Deregistration), PDF-Seite 93**:

- Nach der Abmeldung des **letzten Endpunkts** eines Benutzers erhält der Presence-Server eine Benachrichtigung, um veraltete Veröffentlichungen aufzuräumen.
- Für Container und Kategorien sucht er die betroffenen Kategorieinstanzen mit Ablaufart `user` beziehungsweise `endpoint`, letztere mit der Identität des abgemeldeten Endpunkts.
- Der Endpoint-Identifier verweist auf den UUID-Wert des `+sip.instance`-Parameters in Contact nach **MS-SIPRE §2.2.5**.
- Nach der Bereinigung **MUST** der Server den Abonnenten die Änderungen mitteilen, über NOTIFY oder BENOTIFY gemäß **MS-PRES §3.2.5.3** und **MS-SIP §3.5.1.1**.

**Wichtig für den Client:** Es ist nicht zulässig, aus `Expires: 0` alleine eine vollständige präsenzseitige Bereinigung aller Benutzergeräte abzuleiten. Die Originalregel unterscheidet ausdrücklich den letzten Endpunkt, endpointgebundene Instanzen und `user`-Lebensdauern. Die Serveraktion und die clientseitige Verarbeitung der folgenden NOTIFY/BENOTIFY sind getrennte Zustandsautomaten.

## 3. Gemeinsame Abhängigkeitsanalyse

- **RFC 3261 / MS-SIPREGE**: Registrierung, spezifischer Contact und Bindungslebensdauer.
- **MS-SIPRE §2.2.5**: SIP-Endpunktkennung aus `+sip.instance`; übergreifende Zuordnung zur Präsenz.
- **MS-PRES §3.2.5.5.1**: serverseitiger Cleanup abhängig von Registrierung und Kategorieablaufart; anschließend Benachrichtigung.
- **MS-CONFBAS / MS-CONFIM**: eigenständiger MCU-Nachrichtenzustand einschließlich `Message-Id`, IMDN und 10-Sekunden-Zustell-Timer, unabhängig von REGISTER-Timern.
- **MS-SIPAE**: signierte Client-/Servernachrichten und SA-Zustände dürfen nicht zu einer allgemeinen `MESSAGE 202 = angekommen`-Annahme verkürzt werden.

### Konflikt- und Versionsregister

1. **EV-01:** Der aktuelle Microsoft-Originaltext wurde für beide hier analysierten Protokolle verwendet, aber noch nicht alle referenzierten anderen Dokumente und Produktversionen gegengeprüft.
2. **EV-02:** 202 Accepted im Konferenz-IM ist nur MCU-Annahme und erfordert gegebenenfalls eine spätere Zustellmeldung; RFC-Transaktion und IM-Zustellung sind getrennt.
3. **EV-03:** `From` einer MCU-Weiterleitung enthält Konferenz-URI, nicht automatisch ursprüngliche Benutzeridentität; bei `Ms-Sender` gelten gesonderte Regeln.
4. **EV-04:** Registrierungsänderung und Präsenz-Cleanup dürfen nicht als ein einziger synchroner Zustand behandelt werden; NOTIFY/BENOTIFY hat eigene Verarbeitungsregeln.
5. **EV-05:** `+sip.instance` muss versionstreu aus MS-SIPRE §2.2.5 mit MS-PRES abgeglichen werden; Multi-Endpunkt- und Datenschutzfolgen stehen aus.

## Nächste normative Forschungsschritte

Die vollständigen MSCONFIM-IMDN-XSD-Regeln (§§2.2.2, 6.2) und MCU-Fortschreibungsregeln (§3.3.2.5.2); MS-PRES §3.2.5.3 und §3.2.5.5.2 (Endpoint Deregistration/Expiration); MS-SIP §3.5.1.1; sowie MS-SIPRE §2.2.5 müssen gemeinsam ausgewertet werden. Ebenso fehlen die vollständigen Windows-Full-Client-Feature-Matrix, Medien- und Konferenzfälle, Versionskonflikte und Gesamtzustandsdiagramme.

**Status: zwei fachlich zusammenhängende Original-PDF-Abschnitte geprüft, globale RFC/MS-Analyse NICHT abgeschlossen.** Keine Protokollcodeänderungen, keine echten Servertests, kein Merge und kein Release.
