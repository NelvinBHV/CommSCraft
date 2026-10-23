# MS-PSOM rev11 — separate TLS data-MCU channel, identity token, interface hash exchange, objects and file transfer

**Research-only original source audit: 2026-10-23. INCOMPLETE as a full protocol/client conformance assessment.** Primary Microsoft **[MS-PSOM]**, *PSOM Shared Object Messaging Protocol*, revision **11.0**, official publication **2025-05-20** (`v20250520`), local original PDF SHA-256 **6eee9f7f42014a0806d887448bb5837a68eb56a2a5048ebfa9fd9a4814b6e049**; official https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-PSOM/%5BMS-PSOM%5D.pdf . Verified first-party original §§2.1–2.2.2.2, 3.1.1, 3.1.4.1.12–3.1.4.1.17, **3.2.2–3.2.3.1.4**, §§3.2.4.1.8–13, 3.3.2,3.3.4.2–3.3.4.4,5.1. Original **PDF page 53** rendered and visually verified (other clauses extracted from the indexed 111-page source). Document-direct bibliography from the original archive: **[MS-CONFBAS]** rev15 (notably §§2.2.3.17–18 for AddUser pwrpc, sAuthId and proxyHeader), **[RFC2119](https://www.rfc-editor.org/rfc/rfc2119)** requirement terms and **[RFC2616](https://www.rfc-editor.org/rfc/rfc2616)** HTTP/1.1. Generic TLS and HTTPS transport have external standards not automatically counted as [MS-PSOM] directly cited RFCs. The exact §1.2 normative/informative classification and deployed TLS/certificate policy remain to audit. **P** direct primary obligation/role; **S** derived connection dependency; **U** unverified target Windows build / unresolved.

## 1. Authorization and independent transport: not SIP or RTP

| ID | First-party clause | Exact, role-scoped evidence |
|---|---|---|
| PSOM-01 | §2.1 | **All PSOM protocol communication MUST use a TLS channel**, with server address and port obtained from outside PSOM. Thus SIP Focus/INFO 202 and MS-RTP transport are not proof PSOM authenticated or available. |
| PSOM-02 | §3.2.3.1.1.1 | Client obtains **sAuthId from C3P AddUser response** ([MS-CONFBAS] §2.2.3.17–18); if proxy/load-balancer mode `pwrpc.modes=fwdtls`, also **proxyHeader**. For `pwrpc.modes=tls`, only sAuthId necessary. Authorization token is scoped to DataMCU join, not SIPAE SSPI Kerberos, media SRTP key, or arbitrary file token. |
| PSOM-03 | §3.2.3.1.1.1 | In direct `tls` mode, client MUST connect to **FQDN:port in pwrpc.pwsURI**. If AddUser `alternativeName` present in that mode, MUST use it in place of X.509 certificate subject for FQDN validation as specified. This text does **not** authorize skipping CA trust, treating untrusted TLS as good, or blindly reinterpreting modern SAN checks; all must be reconciled with certificate platform docs and product profile. |
| PSOM-04 | §3.2.3.1.1.1 | In `fwdtls` mode, client MUST choose a **random proxy[i] FQDN/port** among provisioned entries and try others if connect fails; client MUST send **32-bit network-order proxyHeader length and proxyHeader** immediately after connect. Do not confuse proxyHeader with MS-SIPAE Authorization or HTTP header. |
| PSOM-05 | §§3.2.3.1.1.1–2 | Client then MUST transmit **12-byte fixed/length header** `Signature=0x70773200; AuthenticationVersion=0x00000000; AuthenticationTokenLen=uint32_BE` + byte encoding of `sAuthId`. Server MUST **terminate immediately** on invalid authorization; valid handshake server replies fixed four-byte **0x70773200** and **only then interface versioning begins**. |
| PSOM-06 | §3.2.3.1.2 | Client MUST set Channel=0 with **SetChannel** before root ConnMgr RPC version negotiation; must call `version`, **`addProtocol` once for each supported distributed object**, then `doneProtocols`; after `doneProtocols` client MUST NOT send any of those three again. No negotiated version can be inferred merely from received handshake bytes. |
| PSOM-07 | §3.2.3.1.2 | ConnMgr exchanges **per-interface hashes**; any mismatch ⇒ client **MUST terminate connection immediately** (may send Break first). Version negotiation complete **only when client has sent and received doneProtocols**. Thus successful TLS and sAuthId does not imply application shared object channels are ready. |
| PSOM-08 | §§3.2.3.1.3–4 | ConnMgr **client interface hash 8322047979521208965** (example interface defined in source), with `version`, `addProtocol`, `doneProtocols`, `ping`. After version success client MUST create **application PSOM channel 2** by `RPCOpen(0x37)` with 32-bit channel 2 and valid embedded lookup RPC, then **SetChannel=0x00000002**. Channel 0 reserved for protocol negotiation, channel 2 for application messages. |
| PSOM-09 | §§3.2.2,3.3.2 | **Both PSOM client and server** have a **30-second** ConnMgr `ping()` keepalive timer. This is **separate** from SIP-CONMGMT per-hop CRLF, RTP/RTCP idle, ICE 5s consent probes, SIP registration refresh and TURN allocation. |
| PSOM-10 | §§2.2.1–2.2.1.1 | Records start with **one-byte type**, followed by type-defined length/body, lengths as **uint32 big-endian** where present. `Close=0x00` (no body), `SetChannel=0x04` (+uint32 BE ChannelId), `Break=0x06` (+uint32 BE reason len, ASCII code), `RpcMessage=0x16` (+uint32 BE body length), `RPCOpen=0x37` (+channel id+embedded RPC length/body). |
| PSOM-11 | §2.2.1.1.1–3 | Graceful **Close** MUST occur during disconnection, **other PSOM channels must close before channel 0**; close transitions connected distributed objects to disconnected. **Break** has same teardown behavior but indicates error with length-prefixed ASCII diagnostic. Free-text Break reason is troubleshooting text, not an authenticated access-control command. |
| PSOM-12 | §§2.2.2–2.2.2.2 | Distributed object `ProxyId` is **signed Int32 represented as GenericInt**; root proxy id 0. Initiating side increments positive IDs from +1, remote peer maps to **negative** corresponding id; receiver MUST negate the received ProxyId to route to local counterpart. Misusing unsigned ProxyId silently cross-targets objects. |
| PSOM-13 | §§2.2.2.1.1–2 | `OP_CONNECT=0x84` includes parent proxy-id, **PartName PSOM string**, 64-bit **interface hash** and target child; incoming hash MUST match object's interface. `OP_CLOSE=0x86` closes **that object**, not automatically all child objects (they remain connected unless separately disconnected). |
| PSOM-14 | §2.2.2.2 | `OP_DATA` is RPC method call with **GenericInt ProxyId, one-byte signed MethodIndex and serialized payload**; type disambiguation from 0x84/0x86 uses GenericInt encoding guarantee. Do not treat arbitrary RPC payload as an HTTP or SIP XML packet. |
| PSOM-15 | §§3.1.4.1.12–17,3.2.4.1.8–13 | Whiteboard, PPT, annotations, polls, shared notes, Q&A and links are separate **distributed-object interfaces**. Annotation `cAddAnnotationBatch` and `cChangePropertiesBatch` use **parallel IDs/generations/owners and propertyCounts** to bind flattened property arrays, whereas `cChangeTextBatch` carries **textVersions** and deltaCounts. Ignoring generation numbers can mix stale annotations and new content. |
| PSOM-16 | §§3.3.4.2,5.1 | File Web Server downloads can use HTTP or HTTPS by source profile, **files stored on the Web Server MUST be AES-encrypted** with a **key/IV conveyed in PSOM messages**. Source §5.1 states **SHA-1 is used for file deduplication/download optimization, NOT for security**. Do not interpret the SHA-1 file hash as an authentication MAC or confuse with disputed SRTP HMAC. |
| PSOM-17 | §3.3.4.3–4 | Alternate decrypted download and single-file HTTPS Form POST use separate UploadManager methods. For browser-style POST, client `sRequestWebUploadBlob` → server **cAcceptWebUpload/cRejectUpload** → HTTPS Form POST → HTTP success/error (200 or non-200) → client **MUST call sUploadFinished(cookie, cancel=true if failed)**. SIP 202 or HTTPS 200 alone does not complete upload accounting. |
| PSOM-18 | §§3.2.3–3.3.4 | **Independent authorization boundaries:** SIP Focus membership/AddUser token ≠ PSOM TLS server identity ≠ DataMCU token accepted ≠ distributed-object version hash compatible ≠ content permission/upload accepted. This is a **synthesized state requirement (S)**, not a claim the MS-OCSPROT flow is always strictly serialized beyond original conditions. |

## 2. Precise combined sequence (S, source-clause gated)

~~~mermaid
sequenceDiagram
 participant C as SfB Windows client
 participant F as SIP Focus/C3P
 participant D as DataMCU or provisioned PSOM TLS proxy
 C->>F: Join Focus / separate C3P AddUser for content
 F-->>C: sAuthId, pwrpc.modes/pwsURI and optional proxyHeader
 C->>D: TLS connect to provisioned FQDN/port, certificate identity validation
 opt Forward TLS proxy mode
  C->>D: uint32 BE proxyHeader length then bytes
 end
 C->>D: 70773200 00000000 uint32 BE sAuthId len + sAuthId bytes
 alt token/authorization invalid
  D--xC: terminate PSOM connection
 else valid
  D-->>C: 70773200 ack
  C->>D: SetChannel 0; ConnMgr version/addProtocol*/doneProtocols
  D-->>C: ConnMgr versions/doneProtocols
  alt Interface hashes incompatible
   C--xD: Break optional; terminate connection MUST
  else version handshake both done
   C->>D: RPCOpen application channel 2 then SetChannel 2
   C-->>D: PSOM object RPC / annotation changes
  end
 end
 Note over C,D: Independent ConnMgr ping every 30 seconds
~~~

*This diagram is a conditional specification composition, not an exact client network capture. Data MCU roles and C3P AddUser preceding PSOM are described in MS-CONFBAS.*

## 3. Explicit blockers

1. **PSOM-U1:** §3.2.3.1.1.1 `alternativeName` certificate-name validation needs exact target Windows certificate-chain/SAN/hostname handling and stronger contemporary TLS policy sources. No insecure accept-any-certificate fallback inferred.
2. **PSOM-U2:** Authenticated sAuthId lifetime, token binding to meeting/participant and when to refresh on reconnect require detailed [MS-CONFBAS] §2.2.3.17–18/§3 and admin policy, not just this connection handshake.
3. **PSOM-U3:** All complete 111-page distributed-object method signatures/type encodings, version hashes and error mapping **not fully audited** in this tranche. Schema extraction needed, including File URL validation/retention and malicious object method/replay/error transitions.
4. **PSOM-U4:** Source-allowed HTTP download with AES data-at-rest does **not itself prove integrity, origin authenticity or safe crypto mode**; details of key exchange/cipher mode must be independently read from exact source before any later implementation planning.
5. **PSOM-U5:** PSOM source's 30s application ping is not SIP/ICE consent. TLS connection loss/rejoin does not automatically imply Focus leave, end media or account deregistration.
6. **PSOM-U6:** Determine actual client product/build feature availability from [MS-OCSPROT] §4 notes; 2025 protocol text does not establish all historical 2015 Windows binaries have all whiteboard/PP/Q&A features.

**Gate remains CLOSED: no protocol implementation, no genuine server connections or tests, no merge or releases.**
