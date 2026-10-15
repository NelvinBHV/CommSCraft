# Offline SIP fixtures

These are **synthetic** protocol examples, constructed from RFC 3261 and
[MS-SIPREGE's published examples](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/bc4b0245-339d-490c-ba67-2410e1513ed9).
They do not contain real users, passwords or server configuration and are not
promised to be accepted by a deployed Microsoft server.

- `register.sip` – unauthenticated, simplified REGISTER request
- `trying.sip` – correlated provisional response
- `unauthorized.sip` – correlated 401 final response (no authentication logic)
- `challenge.sip` – synthetic MS-SIPAE multiple WWW-Authenticate challenges (no tokens)

Tests parse each response from bytes, exercise every possible stream split,
and verify that receiving 401 does **not** establish an authenticated session.

- `register-423.sip`: synthetic RFC 3261 §10.3 `Min-Expires` response.
- `register-ms-gruu-200.sip`: synthetic MS-SIPREGE `gruu`/`+sip.instance`/`expires` 200 response (not authenticated).

## MS-SIPAE synthetic MIC fixtures (v0.5)

- `signed-register-ok.sip`: **synthetic** `200 OK` server Authentication-Info
  with HMAC-SHA256 over MS-SIPAE §3.2.5.2 fields. Fixed public server key:
  32 bytes, each `0x5a` (do not use for real authentication).
- `signed-register-invalid-mic.sip`: same fields, deliberately invalid MIC.

These fixtures are for protocol regression tests only. They are **not**
server-generated traffic, real credentials, or proof of interoperability.
