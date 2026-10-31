# Evidence: additional web services – MS-RGSWS, MS-CVWREST, MS-ECREST, MS-OCSMP

Sources (edition 20250520): [MS-RGSWS] "Response Group Service Web Service Protocol" 46 pp.; [MS-CVWREST] "Unified Communications Call Via Work Protocol" 46 pp.; [MS-ECREST] "Unified Communications Event Channel Protocol" 32 pp.; [MS-OCSMP] "Microsoft Online Conference Scheduling and Management Protocol" 93 pp.

| ID | Requirement | Evidence |
|---|---|---|
| WS-01 | **Response group agent** (RGS): SOAP over HTTPS, address from in-band provisioning (`responseGroupService*URL`, MS-SIPREGE §2.2.2.5.1, file 04), authentication per MS-OCAUTHWS (web ticket, file 09); operations IsAgent, GetAgent, GetGroups, GetActiveResponseGroups, SignIn/SignInMultiple, SignOut/SignOutMultiple | RGSWS §1.3–1.5, pp. 9–11; §2.1, p. 12; §3.1.4.1–3.1.4.8, pp. 16–33 |
| WS-02 | **UCWA family** (CVWREST "call via work": conversations, phoneAudioInvitation, startPhoneAudio/stopPhoneAudio; ECREST event channel `events`, namespace `http://schemas.microsoft.com/rtc/2012/03/ucwa`; OCSMP meeting scheduling myOnlineMeetings, onlineMeetingDefaultValues/EligibleValues/Policies, phoneDialInInformation): REST over HTTP(S), discovered via **Lync Autodiscover** (MS-OCDISCWS, file 08), authentication MS-OCAUTHWS | CVWREST §1.3–1.4, pp. 7–8; §3.1.5, pp. 21–26; ECREST §1.3–2.1, pp. 7–8; §3.1.5, p. 16; OCSMP §1.3–1.4, pp. 10–11; §3.1.5, pp. 31–56 |

## Classification
- According to the overview the UCWA services are meant for **web and mobile applications** ("A typical scenario … web or mobile application", OCSMP §1.3, p. 10). The Windows desktop client uses SIP for the same functions (meeting scheduling via MS-CONFPRO, file 19). For CommSCraft they are **optional**; they can, however, offer a simpler alternative for meeting scheduling/dial-in data.
- Not SfB-related (contained in the Office package, not evaluated further): Groove protocols (MS-GRV*), PowerPoint/OneNote web services (MS-PW*, MS-OMPWHP, MS-OMWWH), Office Broadcast (MS-OBPAS/MS-OBPRS), MS-IMESYN, MS-INFODCF, MS-MERX, MS-STWEB, MS-UDCX, MS-OCPROTO (Office client overview, without SfB protocols).
