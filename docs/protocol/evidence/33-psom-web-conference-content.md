# Evidence: MS-PSOM – web conference content (PowerPoint, whiteboard, polls, Q&A, files)

Source: [MS-PSOM] "PSOM Shared Object Messaging Protocol", edition 20250520, 111 pp. Connection data from the MS-CONFBAS addUser dial-in response for the web conferencing MCU (`data-conf`), file 18 CB-36.

## Transport and connection setup

| ID | Requirement | Evidence |
|---|---|---|
| PO-01 | Own **TLS TCP connection** (not SIP); target: `pwrpc.modes=tls` ⇒ `pwrpc.pwsURI` (`fqdn:port`, certificate check against `alternativeName`); `fwdtls` ⇒ a random `proxy[i].FQDN:Port`, send length + proxyHeader first | §2.1, p. 19; §3.2.3.1.1.1, p. 53 |
| PO-02 | Sign-in: `70 77 32 00` ‖ `00 00 00 00` ‖ length(sAuthId) ‖ sAuthId (ASCII, big endian); the server confirms with `70 77 32 00`, otherwise abort | §3.2.3.1.1.1–3.2.3.1.1.2, pp. 53–54; example §4.1.1–4.1.2, pp. 92–93 |
| PO-03 | Record level (1 byte type, lengths 32 bits BE): 0x00 Close, 0x04 SetChannel(u32), 0x06 Break(len, ASCII reason), 0x16 RpcMessage(len, body), 0x37 RPCOpen(channel, len, RPC body) | §2.2.1–2.2.1.1.5, pp. 19–21 |
| PO-04 | Operations: 0x84 Connect (ParentProxyId, PartName, Hash), 0x86 Close (ProxyId), otherwise RPC = ProxyId (GenericInt) ‖ MethodIndex (1 byte) ‖ parameters; **negate** received ProxyIds; root object ProxyId 0 | §2.2.2–2.2.2.2, pp. 21–25 |
| PO-05 | Types: GenericInt (−112…127 one byte, otherwise 0x80/0x88 + (n−1) with n ∈ {1,2,3,4,6,8} bytes BE), String (2-byte length BE + UTF-8, XORed backwards with a running −17), Boolean, Byte, Double (IEEE 754 BE), Array (GenericInt count + elements), object reference (null = 0x8C) | §3.1.1.1, pp. 26–27; appendix A §6, pp. 102–104 |
| PO-06 | Flow: SetChannel 0 → ConnMgr `version(stubHash)`, per object `addProtocol(name, versions[], hashes[] = client + server hash)`, `doneProtocols` (both sides) → RPCOpen channel 2 + `Lookup` → SetChannel 2 → the server connects the root **Meeting** (`cSetUrlBase` before `cMeetingReady`) and its children | §3.2.3.1.2–3.2.3.1.4, pp. 54–56; §3.2.4.1.1, p. 56; examples §4.1.3–4.3, pp. 93–100 |
| PO-07 | Keep-alive: `ping()` on ConnMgr every **30 s**; connection loss ⇒ complete rebuild; hash mismatch ⇒ abort | §3.1.2, p. 27; §3.2.2, p. 52; §1.3.1, p. 15 |
| PO-08 | Application objects with complete interface definitions (name, version, hashes, methods): Meeting, ContentManager (contents, presenter baton), ContentUserManager, UploadManager/UploadStream (upload, packet scheme pp. 60–66), Content, NativeFileOnlyContent, PptContent (slides, navigation), AnnotationContainer, WhiteboardContent, PollContent, SharedNotesContent, QnaContent, SharedLinksContent | §3.1.4.1.4–3.1.4.1.17, pp. 30–51; client §3.2.4.1, pp. 56–78; server §3.3.4.1, pp. 81–91 |

### Recomputed (own script)
- GenericInt table 0/255/−255/256 ✓; `version` hash 8322047979521208965 ⇒ `87 73 7d da 8b 97 1e 72 85` ✓; sum −8221414758688209204 + 8322047979521208965 = 100633220832999761 ⇒ `87 01 65 85 61 94 5a b9 51` ✓ (pp. 94–95).
- String "pptdemo2.pptx" ⇒ `00 0d 53 44 31 32 02 15 e6 a8 85 cc bd aa 97` ✓ (p. 104).
- Example bytes decoded: `Microsoft.Rtc.Server.DataMCU.Meeting.Pod.ConnMgr`, `Microsoft.Rtc.Server.DataMCU.Meeting.Meeting`; lookup placeholders "TODO-ryanfa-remove"/"NotUsed"; length fields 0x40 and 0x3C match.
- Method indices in the examples (version=1, addProtocol=2, doneProtocols=3, Lookup=5) follow the **declaration order**.

## Conflicts
- K-PO-1 (documented): §3.1.4.1.1 (p. 28) "first declared method = index 1, then consecutive"; §3.1.4.1.2.1 (p. 30) "alphabetical order". The example bytes confirm the **declaration order**.
- K-PO-2: The "irregular" GenericInt values for −2^31 (`88 00`) and −2^63 (`8d` + 6 zero bytes) (p. 102) do not follow from the pseudo code (which yields `8b 80 00 00 00` or `8f 80 00…`); probably overflow effects of the original implementation. Make the decoder tolerant.
- L-17: The content result (PowerPoint rendering) requires Office file formats (ECMA-376/ISO 29500, cited normatively, p. 12) – showing it on Linux needs a renderer; from Lync 2013, PowerPoint rendering usually happens via Office Web Apps (MS-WOPI) – to be checked separately.
