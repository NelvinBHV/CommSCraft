# Evidence: MS-OCDISCWS – Lync Autodiscover web service

Source: [MS-OCDISCWS] "Lync Autodiscover Web Service Protocol", edition 20250520, rev. 10.0, 34 pp. Applies from Lync 2013 up to SfB LTSC 2024 (§8, p. 31). Depends on MS-OCAUTHWS (§1.4, p. 7).

Classification: second, HTTP-based discovery path next to DNS SRV (MS-CONMGMT). Delivers SIP access points (`SipClientInternalAccess`/`SipClientExternalAccess`) and web service URLs (AuthBroker, UCWA).

| ID | Requirement | Strength | Evidence |
|---|---|---|---|
| AD-01 | DNS CNAMEs `lyncdiscoverinternal.<sipdomain>` and `lyncdiscover.<sipdomain>` | precondition | §1.5, p. 7 |
| AD-02 | The client tries up to 4 URLs: http/https `lyncdiscoverinternal.<d>`, then http/https `lyncdiscover.<d>`; 1+2 and 3+4 in parallel; 3/4 only after a response from 1 **and** 2; take the first successful response | MUST/MAY | §3.2.1.1, pp. 22–23 |
| AD-03 | Root resource `https://<fqdn>/autodiscover/autodiscoverservice.svc/root?sipuri=<uri>`; SfB omits the `sip:` prefix | MUST | §2.2.3.1, p. 9; §3.1.5.2, p. 19; <1> p. 31 |
| AD-04 | Accept `application/vnd.microsoft.rtc.autodiscover+xml;v=1` or `+json;v=1` (no Accept ⇒ JSON; unknown ⇒ 406); UTF-8 without BOM | MUST | §2.1, p. 8; §3.1.5.1, pp. 18–19 |
| AD-05 | Response `AutodiscoverResponse` (AccessLocation internal/external) with exactly one of Root/User/Domain; links `token`+`href` | MUST | §2.2.4.1–2.2.4.6, pp. 10–15; §2.2.5, pp. 15–17 |
| AD-06 | Tokens: Internal/External Autodiscover, Internal/External AuthBroker (SOAP+MEX, MS-OCAUTHWS), Internal/External Ucwa (REST, MS-OCSMP), Redirect, User, Domain, OAuth (the example additionally `Self`) | – | §2.2.5.5, pp. 16–17; example p. 24 |
| AD-07 | The HTTP root always answers with a redirect; a foreign user ⇒ redirect to the next hop | MUST | §3.1.5.2, p. 19 |
| AD-08 | User resource only with `X-Ms-WebTicket`; without ⇒ 401 + `X-Ms-WebTicketUrl` (web ticket service, MS-OCAUTHWS); unknown user ⇒ 404; another home pool ⇒ User with a single Redirect | MUST | §2.2.2.2–2.2.2.3, p. 9; §3.1.5.3, pp. 19–20 |
| AD-09 | OAuth resource with `Authorization: Bearer …`; without ⇒ 401, invalid ⇒ 403 | MUST | §2.2.2.1, p. 8; §3.1.5.5, pp. 21–22; example pp. 24–25 |
| AD-10 | Domain resource without authentication, always 200, links + SipAccess if present | MUST | §3.1.5.4, pp. 20–21 |
| AD-11 | SipAccess elements `fqdn`/`port`; the client uses `SipClientInternalAccess` or `SipClientExternalAccess` | – | §2.2.4.5, p. 13 |
| AD-12 | At most 10 redirects or loop detection | MUST | §3.2.1.2, p. 23 |
| AD-13 | AccessLocation decides internal/external links; cache the links until an error; on a network change probe the root | SHOULD/MAY | §3.2.1.3, p. 23 |

## Open points
- O-AD-1: The specification does not say in which order a **SIP desktop client** combines DNS SRV (CONMGMT) and Lync Autodiscover. Both paths are documented; the priority in the Windows client is not.
- O-AD-2: The example (p. 24) uses `http://lyncdiscover.contoso.com/?sipuri=…`, §3.1.5.2 the path `/autodiscover/autodiscoverservice.svc/root?sipuri=` – both forms are to be checked.

## Later findings (file 39)
- O-AD-1 **closed** (OB-06, official): lyncdiscoverinternal → lyncdiscover → _sipinternaltls._tcp → _sip._tls → sipinternal → sip → sipexternal; Autodiscover always has priority.
- O-AD-2 **closed** (OB-07): the root URL `https://lyncdiscover[internal].<domain>/?sipuri=` is for automatic discovery, the path form `…/autodiscoverservice.svc/Root` for manual configuration.
