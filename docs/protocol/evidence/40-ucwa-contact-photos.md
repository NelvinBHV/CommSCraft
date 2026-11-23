# Evidence: contact photos over UCWA 2.0

Sources: [MS-OCDISCWS] 20250520 (Lync Autodiscover Web Service Protocol) §2.2.5.5 (pp. 16–17),
§3.1.5.5 (pp. 21–22); the official UCWA 2.0 reference by Microsoft ("Applies to: Skype for
Business 2015"; source repository OfficeDev/skype-docs, commit d6b0925, pages dated 2022-04-11;
rank 2, official Microsoft documentation): "Authentication in UCWA", "Create an application",
applications, application, me, photo, contact, contactPhoto, myContacts, people, search,
distributionGroup, expandDistributionGroup.

| ID | Requirement | Evidence |
|---|---|---|
| UW-01 | Without a token the OAuth resource (Autodiscover `OAuth` link, a Root link) answers 401 with `WWW-Authenticate: MsRtcOAuth href="…/WebTicket/oauthtoken",grant_type="…"` (next to a `Bearer` challenge) | MS-OCDISCWS §2.2.5.5, §3.1.5.5; "Authentication in UCWA", "Create an application" |
| UW-02 | Token: `POST` form `grant_type=urn:microsoft.rtc:windows` (Windows authentication of the request; `password` would send the password as a form field); answer JSON `access_token` (`cwt=…`), `token_type` `Bearer`, `expires_in`; lifetime 8 h for authenticated users | "Authentication in UCWA" |
| UW-03 | Every UCWA request carries `Authorization: Bearer <access_token>` | "Authentication in UCWA" |
| UW-04 | The OAuth (User) resource with the token names the UCWA service: links `Internal/Ucwa`, `External/Ucwa` (`…/ucwa/oauth/v1/applications`) | MS-OCDISCWS §2.2.5.5; "Create an application" step 4 |
| UW-05 | `POST applications` with `culture`, `endpointId`, `userAgent` (each ≤ 100 characters; XML `input` in the UCWA namespace) → 201 with the application resource | applications (POST), "Create an application" |
| UW-06 | XML representation: `resource` (rel, href) with `link`, `property`, `propertyList` and embedded `resource` elements, namespace `http://schemas.microsoft.com/rtc/2012/03/ucwa`; the application embeds `me` (link `photo`) and `people` (links `myContacts`, `search`) | applications, application, contact (XML examples) |
| UW-07 | `contact` carries `contactPhoto`; `GET contactPhoto?size=HR{n}X{n}` (default HR48X48), `Accept: image/jpeg` → `image/jpeg`; 403 when the administrator disabled photos; the user's own photo is `photo` on `me` | contact, contactPhoto, photo |
| UW-08 | `GET search?query=…&limit=1…100` (query ≤ 256 characters) → `contact` and `distributionGroup` resources | search |
| UW-09 | `distributionGroup` (uri, name) carries `expandDistributionGroup`; its GET returns the members as `contact` resources (each with `contactPhoto`) and nested groups; 403 `TooManyContacts` | distributionGroup, expandDistributionGroup |
| UW-11 | The User resource may answer with a `Redirect` link to the Root of the user's home pool; discovery (and the token) start again there, its Root naming the `OAuth` resource. `AccessLocation` (`internal`/`external`) says which of `Internal/Ucwa` and `External/Ucwa` serves the client | [MS-OCDISCWS] 20250520 §2.2.5.1, §2.2.5.5, §3.1.5.5, §4.1 (pp. 24–26); "Create an application" (discovery and UCWA on different hosts, `lyncdiscover` → `lyncweb`) |
| UW-12 | Password grant: `POST` form `grant_type=password&username=…&password=…` ("mandatory username and password parameters … commonly used in forms-based authentication"; the body URL-encoded); same answer as UW-02. The challenge lists the grants offered (`urn:microsoft.rtc:windows,urn:microsoft.rtc:anonmeeting,password`) | "Authentication in UCWA" (grant types; Password grant type; note "The body content must be URL-encoded") |
| UW-10 | Links in answers are server-relative (`/ucwa/…`) or absolute on the service | all examples; design: followed only on the same host and port |

## Implementation (2026-11-21)
- `crates/ucwa` (sans-I/O): challenge, token answer (bounded flat JSON), User links, application
  request and resource (XML through `commscraft-xml`), photo links of contacts, search results and
  expanded groups, the expand link of a group, queries, `resolve` (same host only).
- `client::ucwa::Session`: Autodiscover → OAuth challenge → token with the Windows grant over
  NTLM → User resource → application; photos from the contact list (`myContacts`), from an
  expanded distribution group (when the group is opened) or from a search; the own photo from
  `me`. Used by the desktop client when the directory and Exchange paths give no photo.
- `docs/protocol/evidence/08-ocdiscws-autodiscover.md` AD-06: the discovery state machine now hands
  on the Root's links (`OAuth`) with the outcome.
- 2026-11-22 (owner: no photos in a production environment any more): the session follows a
  `Redirect` of the User resource (at most three, only to hosts in the SIP domain or trusted, a new
  token per pool) and takes the UCWA service of the access location first, trying the next one
  when it cannot be reached (UW-11); before, the internal service was always first, which a client
  outside the company network cannot reach. Redirect and external access `tested-offline` (the
  lab has one front end and no edge); the lab path still `tested-in-lab`.
- Status: `tested-offline`, `tested-in-lab` (token over NTLM, application, own photo, a contact's
  photo, a search hit's photo, members of an expanded distribution group; the server took about
  20 s to create the application).
- 2026-11-23 (owner: outside the company network, without VPN, no contact photos any more;
  "as in the Android app"): the token comes with the Windows grant first and, where that fails
  and the token service offers it, with the password grant as the mobile clients sign in (UW-12;
  a reverse proxy outside the company network often does not pass NTLM through). A token the
  server refuses (401) opens the session again with a new token. Photos shown are kept per
  account (`$XDG_CACHE_HOME/CommSCraft/photos/<account>/shown/`, 0600) and shown at once on the
  next start, asked for again after a day; a photo that could not be fetched is asked for again
  later (2 minutes, doubled, at most an hour, eight times) instead of never in that session.
  Password grant, application and photos `tested-in-lab` (Windows grant skipped with
  `COMMSCRAFT_UCWA_GRANT=password`); the external path needs the owner's confirmation.
