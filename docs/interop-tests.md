# Interop test cases for the owner

> **Last reviewed:** 2026-10-31 · **Last updated:** 2026-10-31 · **Change:** minor (M3 call tests IT-23 … IT-26, NTLM channel bindings IT-27)

After the fourth search ([`protocol/evidence/39-evidence-for-open-points.md`](protocol/evidence/39-evidence-for-open-points.md),
OB-39 … OB-51) no open point needs more document research. What is left is the observable behaviour
of a real server or of the Windows client. Only the owner runs these tests, on authorized systems
(T-03, D-08). Agents do not connect to servers; they turn a sanitized result into evidence and code.

Each case states what the sources already fix, so a capture only has to decide the remaining
question. Until a case is run, the client follows the **default** column. That keeps every case
non-blocking.

## How to hand in a result

Results of interop runs (Autodiscover answers, host names, captures) and credentials are **never**
committed or written into issues or pull requests. They stay with the owner; agents keep notes only
in the local, git-ignored `plan/` directory and turn them into generic evidence without any data
of the tested deployment.

- Capture with the client's own debug log or a TLS key log on your own machine. Do not commit
  captures made by Microsoft software unless you made them on an authorized system and sanitized
  them (CLAUDE.md, clean room).
- Sanitize before sharing: replace user names, SIP URIs, host names, IP addresses, GRUUs, epids,
  tags, Call-IDs, tokens, signatures, NTLM/Kerberos blobs and SRTP keys with placeholders. Keep the
  structure, header names, attribute types and lengths.
- Record server version and cumulative update, client version, date and test case ID. The result
  goes into the evidence file named in the table and moves the status to `verified-interoperable`.

## Cases

| ID | Point | Already fixed by the sources | Question for the capture | Default until tested | Needed for |
|---|---|---|---|---|---|
| IT-01 | O-CB-1 | In-band request `Accept: Application/vnd.microsoft.lync.meeting+xml;ver=15.0` and XML response format (MS-CONFBAS §2.2.6, p. 55; CB-41) | Does the Join Manager return the XML for a `User-Agent` that does not claim Windows? If not, which part of the header does it check? | Fetch the meet URL; fall back to the browser HTML page and take the focus URI from it (OB-33) | M4 |
| IT-02 | O-RD-1 | MS-RDPEMC §1.5 (p. 10) names a pre-connection PDU (MS-RDPEPS §2.2.1) as precondition; SIPE sends none (OB-18) | Does the Skype for Business sharer expect a pre-connection PDU from the viewer, and with which Id/blob? | Send none; configurable | M6 |
| IT-03 | O-RD-2 | The sharer must send only what the viewer's capabilities allow (MS-RDPBCGR §3.3.5.3.13.2, p. 322) | Does the sharer deliver a picture to a viewer that advertises no RemoteFX/NSCodec, only bitmap updates? | Advertise only the codecs implemented | M6 |
| IT-04 | L-11 | INFO in the IM dialog; the MCU forwards the body without interpreting it (MS-CONFIM §3.4.1.4.1, p. 26) | Content-Type and body of the typing indicator sent by the Windows client (secondary: `application/xml`, `<KeyboardActivity>`, OB-16) | Send and parse the secondary format; ignore unknown bodies | M2 |
| IT-05 | L-16 | XCCOS payloads (MS-XCCOSIP §4) | Request-URI, SDP of the INVITE and Content-Type of the INFO towards the persistent chat server (secondary: OB-17) | Use the secondary values; feature behind a switch | M7 |
| IT-06 | L-13 | Transfer flows (OB-25, official UCMA reference); `ms-referee-uri`, `ms-safe-transfer`, `Ms-Correlation-Id` are documented one by one | Which of these headers does the Windows client send in blind and consultative transfer, and does the server require them? | RFC 3515/3891/3892 plus the documented MS headers | M4 |
| IT-07 | K-RE-1 | MS-SIPRE §3.3.3.1 (p. 41) says SHA-256, the example §4.2 (p. 81) yields SHA-1 | Which hash does the server accept for the derivation (RE-08)? | SHA-1 (matches the example) | M1 |
| IT-08 | K-CA-1 | MS-CONFBAS describes dial-in via addUser, MS-CONFAV makes it unnecessary for A/V | Is the direct INVITE to the A/V MCU accepted without a prior addUser? | Direct INVITE; addUser as fallback | M4 |
| IT-09 | K-CS-2 | MS-CONFAS ties AttendeesCanShare to server-mode 13, which MS-CONFPRO declares obsolete | Does the server report AttendeesCanShare, and with which value? | Treat a missing value as false | M6 |
| IT-10 | K-E9-1 | The schema defines `urn:schema:Microsoft.Rtc.WebComponent.Lis.2010`, the examples omit it | Does the location service accept requests with the schema namespace? | Schema namespace | M4 |
| IT-11 | K-CK-1 | The schema and the park response use `http://schemas.microsoft.com/rtc/2008/12/callpark`, the request example omits it | Does the call park service accept requests with the namespace? | Schema namespace | M4 |
| IT-12 | O-CP-1 (optional) | Compression is negotiated only by the client (MS-SIPCOMP §3.1.5, p. 11); MPPC is tested offline | Byte order of the 16-bit "Uncompressed size" and bit order of the MPPC stream | Compression off; never negotiated | optional |
| IT-14 | O-NL-1 design (OB-43) | MS-NLMP defines all variants; the MS-SIPREGE example (p. 94) shows NTLMv1 without ESS from OCS 2007 times | Do current servers offer EXTENDED_SESSIONSECURITY, 128-bit and KEY_EXCH in the SIP NTLM CHALLENGE (flags of the base64 `gssapi-data`)? | Refuse weaker offers (fail closed) | M1 |
| IT-15 | RG-08, RG-50 | Version filter answers 403 + Warning 310/311 (MS-SIPREGE §3.4.2.5) | Does the server's client version policy accept the User-Agent `CommSCraft/<version>`? Which rule matches? | Report `VersionBlocked` to the user | M1 |
| IT-16 | AD-10, D-10 | The Domain resource needs no authentication and contains SipAccess "if present" (MS-OCDISCWS §3.1.5.4) | Does the unauthenticated Domain resource of the server contain `SipClientInternalAccess`/`SipClientExternalAccess`? Does `https://lyncdiscover[internal].<domain>` answer with a valid certificate? | Fall back to DNS SRV | M1 |
| IT-17 | O-RG-1 | Generic SIP redirect (RFC 3261); SIPE handles a 301 to REGISTER (secondary) | Does a pool that is not the user's home pool answer the authenticated REGISTER with 301 and a Contact `sip:host:port;transport=tls`, signed? | Follow only a signed TLS redirect; ask for foreign domains | M1 |
| IT-18 | TLS ([SFB-TLS], evidence 02) | TLS 1.2 from SfB 2015 CU9 / 2019 CU1; ECDHE + AES-GCM suites in the supported order | Does the TLS handshake with your pool succeed (version and suite)? `commscraft-cli` prints the version | No TLS 1.0/1.1, no fallback | M1 |
| IT-20 | Invitation INVITE of the Windows client (K-CI-3) | MS-SIPRE §3.9.5.1 (p. 58) names `application/ms-conf-invite` and an `audio` element; the rest of the body is only secondary (SIPE) | When a Windows client adds you to a conversation: does an INVITE with `application/ms-conf-invite+xml` arrive (element names `Conferencing`, `focus-uri`), or does the IM MCU call you (dial-out, `Ms-Focus-Uri`)? `interactive` prints "invitation N" or a new conversation | Invitation is only reported; joining with `/join N` | M2 |
| IT-21 | Group IM against the conferencing server | addConference, addUser join/dial-out, roster, IM MCU, IMDN (CB-10 … CB-37, CI-01 … CI-12) | `interactive`, `/group sip:A sip:B`: is the conference created (`openAuthenticated`), do the invitees get the conversation, are messages delivered (202 + IMDN)? | – | M2 |
| IT-22 | STUN-02 checks with MI and FINGERPRINT (O-IC-2) | Length rule of RFC 5389 §15.4 applied to the legacy format | During a call with a client that sends IMPLEMENTATION-VERSION < 3: do our Binding Requests get Binding Responses (not 431)? | Length up to the end of MI | M3 |
| IT-23 | Audio call 1:1 (M3) | OCS-UC12; SDP, ICE, TURN, SRTP, G.722/G.711 tested offline (loopback test with real sockets) | `interactive`, `/call sip:…` and `/answer`: does the call connect in both directions (inside the network and through the Edge), can both sides hear each other, do `/dtmf` digits arrive, does `/hangup` end it on both sides? `audio-check` first verifies the local devices | Encryption as provisioned (`ucPC2PCAVEncryption`), no plaintext fallback | M3 |
| IT-24 | Audio call with UDP blocked (TCP relay through the Edge on 443) | MS-ICE2 regular mode with TCP relayed candidates, pseudo-TLS, RFC 4571 framing (tested against a TURN test server) | From a network that blocks UDP: does `/call` connect and carry audio? | UDP first, TCP relay as additional candidates | M3 |
| IT-25 | Hold and resume in both directions | MS-SDPEXT §3.1.5.23/§3.1.5.27, RFC 4235 §5.2 (tested offline) | Does the peer show the call as held after `/hold`, does audio come back after `/resume`, is a hold (with music) from the peer shown? | No audio sent while held | M3 |
| IT-26 | Long audio call (≥ 5 min) | MS-RTP RTCP profile: packet pairs/trains, audio healer, "MS-EVT", TURN bandwidth (tested offline) | Does the call stay up, does the peer show call quality normally, any errors in the client log? | No media without SRTP authentication | M3 |
| IT-27 | Address book search (NTLM over HTTP) with channel bindings | MS-NLMP §3.1.5.1.2, RFC 4121, RFC 5929 (recomputed offline; issue #42) | Does `interactive` → `/search` still work; if the web server requires Extended Protection, is it accepted? Which signature algorithm does the server certificate use (RSA/ECDSA with SHA-2 is covered, PSS/Ed25519 sends no binding)? | Never sent over a connection other than the one the challenge came on | M2 |
| IT-28 | Call with early media | MS-SDPEXT §3.1.5.12–3.1.5.13, RFC 3262 (tested offline) | `/call` to a number or service that plays an announcement before answering: is it heard before the answer, does the call connect normally after it? With `--verbose`: PRACK after a reliable 183? | No speech sent before the 200 | M3 |
| IT-19 | Whole M1 flow | Everything in IT-07, IT-14 … IT-18 | `commscraft-cli sign-in … --authorized`: registration, keep-alive, provisioning summary, sign-out | – | M1 |
| IT-13 | O-EX-3 (optional) | Folder and item type of the conversation history (OB-22) | Body layout written by the Windows client, so that both clients' entries look alike | Own layout | optional |

## Running the command-line tool (IT-19)

```
cargo build --release -p commscraft-cli
# 1. Discovery only (IT-16): no password, no sign-in.
./target/release/commscraft-cli sign-in --aor sip:alice@contoso.com --discover-only --authorized
# 2. Complete run (IT-19); the server is discovered, or given with --server pool1.contoso.com:5061.
./target/release/commscraft-cli sign-in --aor sip:alice@contoso.com --user alice --domain CONTOSO \
    [--ca-file enterprise-root.pem] [--verbose] --authorized
```

Discovery order (OB-06, D-10): `https://lyncdiscoverinternal.<domain>`, then
`https://lyncdiscover.<domain>` (Root → Domain resource, no credentials), then DNS SRV
`_sipinternaltls._tcp` and `_sip._tls` via the name servers in `/etc/resolv.conf`, then
`sipinternal`, `sip`, `sipexternal.<domain>:443`. Every step is printed. A target that Autodiscover names outside
your SIP domain (hosted deployments) is used only after you trust its domain – interactively (the
answer is remembered in `~/.config/commscraft/trusted-domains`) or with `--trust-domain DOMAIN`,
like `TrustModelData` of the Windows client. DNS SRV targets outside the SIP domain are never used. If Autodiscover uses a
certificate from an enterprise CA that is not in the system trust store, add `--ca-file`.

The password is asked without echo. `--verbose` prints the SIP messages with every
authentication header and all bodies redacted; review the output before sharing it anyway (host
names, user names and IP addresses remain). The EPID is kept in `~/.config/commscraft/epid`
(mode 0600). Without `--authorized` the tool refuses to connect.

## Not a test case

Behaviour that the sources fix completely is tested offline, not here. That includes NTLM flag
handling (OB-43), unknown TURN attributes (OB-41), the web ticket header (OB-39), the DLX
endpoint (OB-44), QoE without MOS values (OB-47) and the Exchange interfaces (OB-48). A failure
against a real server in these areas is a bug report with a capture, not an open point.
