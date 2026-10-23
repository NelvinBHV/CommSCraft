# MS-EUMSDP rev9 — Exchange UM audio-only offer/answer versus general conferencing SDP

**Original-source research 2026-10-23, specific full normative section reviewed; real client/build conformance NOT established.** Microsoft original **[MS-EUMSDP] Exchange Unified Messaging Session Description Protocol Extension**, **rev9.0, v20250520, May 20 2025**, 17-page archived original SHA-256 **d9c8c1b658fb25530fdd4032b7a97a45fe199feb12ef9950ef0d50a07c18c3d6**, official Microsoft https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-EUMSDP/%5BMS-EUMSDP%5D.pdf. All core protocol-specific sections **§§1.3–1.4,2.1–2.2,3.1.1–3.1.7** assessed, PDF **page10** rendered from original. Direct Microsoft dependencies **[MS-DTMF]**, **[MS-ICE]**, **[MS-ICE2]**, **[MS-QoE]**, **[MS-RTP]**, **[MS-SDPEXT] rev16**, **[MS-SIPRE] rev13**, **[MS-SRTP] rev14**, **[MS-TURN]**. RFC originals **[RFC2119](https://www.rfc-editor.org/rfc/rfc2119)** and **[RFC4566](https://www.rfc-editor.org/rfc/rfc4566)** SDP. **P** original normative fact, **S** derived inter-protocol boundary, **U** incomplete client product-specific profile.

| ID | Section | Role-qualified requirement or limitation |
|---|---|---|
| UMSDP-01 | §§1.3,1.4,2.1 | UM uses existing **MS-SDPEXT offer/answer** and SIP/ICE/RTP/DTMF/SRTP transports, **not a new media signaling transport**. Call-in voicemail and Play-On-Phone are source examples. |
| UMSDP-02 | §3.1.5 (override MS-SDPEXT §3.1.5.18) | **Only `m=audio` is supported**; any `m=video`, `m=applicationsharing` or other media types are **rejected**. UM call is **not a video/conference session** merely because the user's existing app supports video. |
| UMSDP-03 | §3.1.5 (override MS-SDPEXT §3.1.5.19) | SDP `o=` **`sess-version` MUST be numeric**, **SHOULD be ignored on receive**, and protocol **server SHOULD increment** on later offers. Generic offer/answer version assumptions from other modalities do not override this UM-specific rule. |
| UMSDP-04 | §3.1.5 (override MS-SDPEXT §3.1.5.12.3) | If provisional **18x** response already contains an SDP answer, final response answer from **same SIP fork assumed identical**, and **differences ignored**. Branch/fork identity is relevant; “first differing final SDP replaces media” is wrong for this UM profile. |
| UMSDP-05 | §3.1.5 (override MS-SDPEXT §3.1.5.25) | UM profile explicitly **ignores `a=x-bypassid`, `a=x-bypass`, `a=x-mediasettings`**. Contrast **MS-OCPSTN rev11 §3.6**: a gateway may support `ms-bypass` for other PSTN calls; supporting bypass generally **does not mean Exchange UM path uses it**. |
| UMSDP-06 | §3.1.5 (override MS-SDPEXT §3.1.5.30) | Ignore **`a=rtcp-rsize`**, **`a=rtcp-fb`** (no RTCP feedback extension in UM profile). Does **not** imply RTP or RTCP absent; base MS-RTP still a dependency. |
| UMSDP-07 | §3.1.5 (overrides MS-SDPEXT §§3.1.5.31–33) | Ignore **`a=x-ssrc-range`, `a=x-source-streamid`, `a=x-source`**. SSRC range allocation, MSI and source labeling across A/V conferencing not supported in this UM variant. |
| UMSDP-08 | §1.4 | Media may traverse ICE, ICE2 and TURN, DTMF and SRTP still required as applicable. **UM audio-only limitation does not bypass NAT checks, negotiated crypto or media authentication.** |
| UMSDP-09 | §§1.3,3.1.5; MS-EUMR §2.2.4 | **Different SDP contexts:** UM *voice-call* m=audio uses MS-EUMSDP, while **proxy→UM notification dialog** uses `m=application 9 SIP *` per MS-EUMR §2.2.4.3. They are distinct channels and rules, not a contradiction about media types. |
| UMSDP-10 | §§1.3,3.1.5 | **Edition gate:** While MS-EUMSDP rev9 is 2025 source, exact 2015/2016 Windows client binary and Exchange UM service edition must be checked. The reference profile does not mandate that all Windows Full Clients expose Play-On-Phone UI. |

## Composite UM media and notification boundaries (S)

~~~mermaid
flowchart TD
 A[User/client call, then proxy UM redirect MS-EUMR] --> B[Voice-mail route over SIP]
 B --> C[MS-EUMSDP audio-only offer-answer]
 C --> D[ICE/ICE2/TURN and SRTP as negotiated]
 D --> E[Audio RTP plus optional DTMF, independently of video]
 B --> F[UM proxy call event notification path separate]
 F --> G[MS-EUMR special INVITE SDP m=application 9 SIP *]
 G --> H[SIP INFO XML missed/answered/forbidden]
 H --> I[UM sends email outside Windows call media]
~~~

**Pending:** Do not assume `x-bypass` ignored for **all** PSTN calls (it is UM-specific). Exact response/fork identifiers and early 183 media behavior require reconciled MS-SIPRE and MS-SDPEXT; original media SHA1/SHA256 inconsistency remains OPEN. All SDP unknown attrs and malformed numeric values need lower RFC/SDPEXT handling and edition fit.

**Gate CLOSED. No protocol implementation, real-server connections, assistant interoperability tests, merges or releases.**
