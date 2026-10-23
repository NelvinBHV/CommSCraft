# MS-OCDISCWS 10.0 — Lync web-service autodiscovery, ticket/OAuth gates, redirect limits and network switching

**Primary-source research 2026-10-23; selected-clause audit, full classical Windows application mapping unverified.** [MS-OCDISCWS] *Lync Autodiscover Web Service Protocol*, **rev10.0, 2025-05-20 (`v20250520`)**, original Microsoft PDF SHA-256 verified `20a89f1b3c490307558888921a5a3fc6633b17d857ed46513f5932c06bc8b099`: https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-OCDISCWS/%5BMS-OCDISCWS%5D.pdf . §§1.3–1.7,2.1–2.2.5,3.1.5.1–.5,3.2.1.1–.3 examined; original PDF pp8,19,21–23 rendered/visually checked. Primary normative source historical RFCs [RFC2616](https://www.rfc-editor.org/rfc/rfc2616) HTTP/1.1, [RFC2818](https://www.rfc-editor.org/rfc/rfc2818) HTTPS and [RFC3986](https://www.rfc-editor.org/rfc/rfc3986) absolute URI; first-party [MS-OCAUTHWS] rev11 for Web Ticket and OAuth auth mechanisms, [MS-OCSMP] rev7 for scheduling API. **P** direct Microsoft requirement, **S** justified composition, **U** missing deployment/build specifics.

## 1. Web-service discovery actor and wire state

| ID | Source | Direct rule/requirement |
|---|---|---|
| OCD-01 | §1.3,§2.1 | This protocol finds **web services and topology for the user's home server**, independent of the **MS-CONMGMT four SIP DNS-SRV queries**. HTTP/HTTPS, usually 80/443, custom ports MAY; content MUST UTF-8 without BOM. |
| OCD-02 | §3.2.1.1 | From user's SIP domain, client MUST construct up to **four root discovery URLs**, in order: `http://lyncdiscoverinternal.<sipdomain>`, `https://lyncdiscoverinternal.<sipdomain>`, `http://lyncdiscover.<sipdomain>`, `https://lyncdiscover.<sipdomain>`. Do not silently substitute Microsoft 365/Teams modern endpoints. |
| OCD-03 | §3.2.1.1 | **Requests 1+2 MAY run concurrently** and 3+4 MAY concurrently, but client **MUST wait for both internal results before sending external requests**. It MUST process first successful result and MAY cancel pending requests. This is not four permanently serialized mandatory probes. |
| OCD-04 | §3.1.5.2;§2.2.3.1 | Every Root request MUST carry `sipuri=<sipuri>` query parameter. Primary root path is `/autodiscover/autodiscoverservice.svc/root`. HTTP Root response MUST 200 with **`Redirect` link to next hop**; this is an explicit source behavior, not invitation to send authentication credentials over HTTP. |
| OCD-05 | §3.1.5.2 | If Root handles the SIP-URI, response 200 Root links to User and Domain and MAY OAuth. Otherwise Root **200 with `Redirect` link**; a 200 can mean **next hop**, not discovery completed. |
| OCD-06 | §3.2.1.2 | Client MUST cap redirect count at **10 across the entire flow**, **or** implement redirect-loop detection. No unbounded `Redirect` chain even if each GET succeeds. |
| OCD-07 | §§2.2.4.1,3.1.5.1 | `AutodiscoverResponse` must contain exactly one logical Root/Domain/User representation and **AccessLocation attribute**. Default/no `Accept` or `*/*` uses `application/vnd.microsoft.rtc.autodiscover+json;v=1`; XML variant `+xml;v=1` accepted. Other unsupported Accept → **HTTP 406 Not Acceptable**. |
| OCD-08 | §§3.1.5.3–.4 | **User URL and Domain URL MUST be followed from the Root response link href**, not guessed via `/user` suffix. Domain exposes topology, User exposes **user's home server** topology. Both may provide Internal/External Autodiscover, AuthBroker and Ucwa links and SipAccess. |
| OCD-09 | §§2.2.2.2–.3,3.1.5.3 | User GET MUST send **X-Ms-WebTicket**. Missing/invalid ticket → server **401**; without ticket response has **X-Ms-WebTicketUrl** pointing to Web Ticket Service URL for [MS-OCAUTHWS]. This is **HTTP Web Ticket**, not SIPAE 401/407 realm challenge nor TURN allocation token. |
| OCD-10 | §3.1.5.3 | With valid User ticket, home server unknown → **404 empty body**; located on different server → **200 User containing only Redirect link**; local → **200 User** with configured service links. An HTTP 200 on User is not always a final home endpoint. |
| OCD-11 | §§2.2.2.1,3.1.5.5 | OAuth GET requires **Authorization** header; absent → **401**, invalid → **403**, valid+unknown home → **404**, valid+remote home → **200 with User/Redirect**, valid+local home → **200 User service links**. Distinct from SIP REGISTER auth and not every deployment offers OAuth resource. |
| OCD-12 | §3.2.1.3 | Final User/Domain/OAuth returns **AccessLocation=internal|external** along with both internal and external links. Client **SHOULD follow matching set**; don't assume the presence of both makes both network sides reachable or safe. |
| OCD-13 | §3.2.1.3 | Client MAY cache internal/external Autodiscover URLs and probe Root after short connectivity loss; service URLs **SHOULD remain cached until connecting fails**. This differs from MS-OCSMP §3.1.5.7/8 **per-user policy/default values that SHOULD NOT be cached**. |
| OCD-14 | §§3.2.1.1–3.2.1.3 | Redirect and web ticket URLs are **trust-boundary inputs**. Cross-origin redirects require authorization/header scope/TLS certificate validation from [MS-OCAUTHWS] and HTTPS security spec; source did not authorize forwarding secret ticket to arbitrary untrusted web domain. |
| OCD-15 | §3.1.5.1 | Unsupported `Accept` → 406, not SIP 488; 401/403/404 have resource-specific meanings and **must not universally trigger SIP deregistration**. |
| OCD-16 | §3.2.1.3 + MS-OCSMP §3.1.1.2.3 | **S integration:** OCSMP scheduling app discovers a Web home endpoint through OCDISCWS before creating its application resource. Authentication Web Ticket/OAuth and resource authorization are distinct from participant admission to later SIP Focus conference. |

## 2. Conditional web discovery and scheduling states (S)

~~~mermaid
flowchart TD
 A[User SIP domain] --> B[Internal HTTP + HTTPS Lyncdiscover requests, both complete]
 B --> C{Valid Root response?}
 C -->|No| D[External HTTP + HTTPS Lyncdiscover probes]
 D --> C
 C -->|Root Redirect| E[Follow redirect up to 10 or loop detection]
 E --> C
 C -->|Root User / Domain / optional OAuth links| F[Select User or OAuth authentication profile]
 F --> G{Authenticated?}
 G -->|User no Web Ticket| H[HTTP 401 with X-Ms-WebTicketUrl to MS-OCAUTHWS]
 G -->|OAuth absent / invalid| I[401 / 403 resource-specific]
 G -->|Valid ticket or Authorization| J{Home resource location}
 J -->|Remote| E
 J -->|Local| K[AutodiscoverResponse AccessLocation and service links]
 K --> L[Select internal/external link; cache topology until connection fails]
 L --> M[Optional MS-OCSMP scheduled meeting REST resource]
 M --> N[Separate SIP Focus/C3P join and AV or sharing MCU]
~~~

**S constraints:** Original explicit HTTP probe URLs are historic discovery behavior, not recommendation to send Web Tickets/secrets over an unencrypted channel. The graph does not assume that an HTTP Root redirect makes a destination trustworthy or that the target SfB desktop builds use this web discovery for every SIP REGISTER.

## 3. Protocol boundaries, edition and gaps

- **OCD-U01** Distinguish `lyncdiscoverinternal/lyncdiscover` HTTP service URLs from **MS-CONMGMT `_sipinternaltls._tcp` DNS SRV** and DHCP 120; they have separate discovery, credential and failure paths. They may use one user SIP domain, **not** a single network transaction.
- **OCD-U02** Source exposes HTTP Root redirect and optional user OAuth links but cross-domain URI origin/certification checking, ticket audience, replay/downgrade and redirect policy require full [MS-OCAUTHWS] and platform TLS implementation normative review. Never invent allowlist or silent HTTP credential forwarding.
- **OCD-U03** Server returns OAuth/User/Domain auth failures; distinguish OAuth 403 from wrong SIPAE MIC and MS-SIPREGE survivable registration state. Registration need not be discarded on Web scheduling failure.
- **OCD-U04** Historic client build/version/timing and implemented OAuth vs Web Ticket branch remain U. The 2025 spec describes service family, not proof all full Windows client releases expose all Web scheduling.
- **OCD-U05** Full 34-page schemas, all `Link` token types, Root/Domain redirect loops with multiple origins, exact 401 HTML body and modern TLS/X.509 security need further normative clause review; no live server validation.

**NOT COMPLETE: research only. No protocol code, server connections, interoperability tests, merge or release.**
