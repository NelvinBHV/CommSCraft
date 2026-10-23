# MS-XMLMC 10.0 — CVP-to-video-source SIP INFO picture_freeze, picture_fast_update, RTCP and SDP independence

**Research-only 2026-10-23. Source-specific selected 20-page original audit, not a captured Full Client media trace.** Microsoft official [MS-XMLMC] *XML Schema for Media Control Extensions*, **revision 10.0, release 2025-05-20, v20250520**, original locally SHA-256 verified `71d684c0cdcec8c6698a86b0fbdad4bca0c70943a8f799716cb5e4d348382a16`; original PDF https://officeprotocoldocs-f5hpbjgea6b8gneq.b02.azurefd.net/files/MS-XMLMC/%5BMS-XMLMC%5D.pdf . Read original §§1.3–1.7,2.1–2.2.1,3.1.1–3.1.7,3.2.1–3.2.7,§4 example, §5 security. Original PDF pp7,10,11,12 rendered and crosschecked. Direct normative original refs: [RFC2976](https://www.rfc-editor.org/rfc/rfc2976) (2000 SIP INFO), [RFC3264](https://www.rfc-editor.org/rfc/rfc3264) (2002 SDP offer/answer), [RFC4566](https://www.rfc-editor.org/rfc/rfc4566) (2006 SDP), [RFC2119](https://www.rfc-editor.org/rfc/rfc2119) requirement words, **IETFDRAFT-XMLSMC-12** (historical XML media control schema, §4/5/7.2/9); current RFC5168 XML Media Control (2008) and SIP INFO package [RFC6086](https://www.rfc-editor.org/rfc/rfc6086) are **edition comparisons, not silent replacement**. Adjacent sources [MS-RTP] rev17, [MS-H264PF] rev16, [MS-RTVPF] rev11, [MS-CONFAV] rev13 and [MS-CONFBAS]. **P** direct Microsoft requirement, **S** cross-protocol implication, **U** unresolved deployment/source edition.

## 1. Wire message and actor split

~~~xml
<?xml version="1.0" encoding="utf-8" ?>
<media_control>
  <vc_primitive>
    <to_encoder>
      <picture_freeze/>
    </to_encoder>
  </vc_primitive>
</media_control>
~~~

**This is directly based on [MS-XMLMC] §4's example, whose semantics/outer container come from IETFDRAFT-XMLSMC-12, not standalone standalone SIP/RTP.** The original §2.2.1 expands the base `to_encoder` XML choice from `picture_fast_update` to include `picture_freeze`. It does **not** define a new SIP method, network port, RTP payload type or negotiated SDP direction bit.

| ID | Source | Normative or role-qualified behavior |
|---|---|---|
| XMLMC-01 | §§1.3,2.1–2.2.1 | Control is **SIP INFO with XML body** per RFC2976, using IETFDRAFT-XMLSMC-12 `media_control/vc_primitive/to_encoder`. Microsoft adds **`picture_freeze`** alongside **`picture_fast_update`**; nothing about conference scheduling HTTP or a new RTP codec. |
| XMLMC-02 | §1.6 | Scope **only centrally processed multiparty RTP video**: sender **Central Video Processor (CVP, e.g. video MCU)**, receiver **originating video source (OVS)**. Not generic point-to-point calls, audio tracks, appsharing RDP, or PSOM data. |
| XMLMC-03 | §3.1 introductory | An OVS **SHOULD NOT send `picture_freeze`**. CVP owns source selection decision. Receiver OVS's client implementation may still not support extension; feature applicability must be negotiated/assumed from companion profile, not faked. |
| XMLMC-04 | §3.1.1 | Extension **stateless on wire** with no synchronized SDP offer/answer state. OVS **MUST respond consistently** to *all* SIP INFO messages carrying picture_freeze, even if currently sending no RTP. Source says no implicit persistent state from received message. |
| XMLMC-05 | §3.1.5.1 | On valid picture_freeze, if currently sending RTP video, OVS **SHOULD suspend RTP video packets**; if already not sending, **takes no action**. Source says OVS **SHOULD NOT retain/persist a record of the picture_freeze message**. This is media transmission adaptation, not SIP BYE. |
| XMLMC-06 | §3.1.5.1 | OVS that **implements picture_freeze MUST also implement `picture_fast_update`** because it is the resume/refresh counterpart; feature cannot be modelled as a permanently latched STOP command. |
| XMLMC-07 | §3.1.5.1 | If RTP video packets are suspended in response to freeze, the OVS **MUST continue RTCP packets**. RTCP continuity is explicit even though video media packets stop; RTCP packets alone do not prove camera frames are being delivered. |
| XMLMC-08 | §3.1.5.2 | For **well-formed** picture_freeze, OVS **MUST NOT return error** even if freeze is irrelevant under current SDP negotiated media state or no RTP video is active. This is different from malformed XML parser failures referenced in IETFDRAFT-XMLSMC-12 §7.2. |
| XMLMC-09 | §§3.1.2,3.1.6,3.2.2,3.2.6 | MS-XMLMC **defines no additional timers** for CVP/OVS in this extension. SIP RFC3261 transaction timers, RTCP report intervals and ICE consent continue from their own source. Don't invent a 5s or 30s picture-freeze expiry timer. |
| XMLMC-10 | §3.2.1 | CVP controls only input streams actively routed to watchers. For switch from A to B, CVP may send **`picture_fast_update` to B and `picture_freeze` to A** in **either order**, though prudent CVP waits for B video before freezing A. This is an example, not a mandatory single ordered exchange. |
| XMLMC-11 | §3.2.1.1 | Base draft has optional `stream_id`. As CVP assumes all participant streams started/stopped together, CVP **SHOULD NOT include optional `stream_id`** in either picture_freeze or picture_fast_update. Don't invent per-SSRC freeze scope from that optional field. |
| XMLMC-12 | §3.2.4 | CVP triggers freeze when internal video selection decides video not routed; **no prescribed external trigger**. If no freeze or fast_update sent, irrelevant clients **continue sending RTP even while CVP discards it**, wasting bandwidth but not failing SIP. |
| XMLMC-13 | §§1.3,3.1.1,3.1.5 | **S:** picture_freeze ≠ SDP `sendonly/recvonly/inactive` change, ≠ RTCP receiver report BYE, ≠ ICE nomination restart, ≠ H264 PACSI layer removal, ≠ server-side MS-RTP video loss feedback. Each has its own actor and state. |
| XMLMC-14 | §§1.6,3.1.5.1,3.2.1 | **S:** A source paused by CVP can remain **SIP-connected, SDP-video-negotiated, RTCP-active, and RTP-video-silent** simultaneously. When resume fast_update arrives, codec IDR/references and actual network delivery are separate validation; don't report frame visible until received/decoded. |
| XMLMC-15 | §§1.7,1.8 | Source **does not define extension-specific capability negotiation** or vendor-extensible fields. This absence **does not establish universal SfB Windows client support**; exact client build/server MCU video functionality still requires product edition evidence. |
| XMLMC-16 | §5 + RFC3261/RFC2976 | **S security boundary:** Only authenticated/trusted conference video processor may control a camera sender; don't accept spoofed arbitrary SIP INFO from a nonmember as authoritative CVP command. Exact SIPAE per-hop MIC/authenticity and conference dialog routing must be read in original related sources. |

## 2. Independent RTP/RTCP/video state model (S, source constrained)

~~~mermaid
stateDiagram-v2
 [*] --> SDPVideoNegotiated
 SDPVideoNegotiated --> RTPVideoSending: Source/encoder active
 RTPVideoSending --> RTPVideoPaused: CVP SIP INFO picture_freeze, SHOULD stop RTP
 RTPVideoPaused --> RTPVideoPaused: further well-formed picture_freeze (no error)
 RTPVideoPaused --> RTPVideoSending: CVP SIP INFO picture_fast_update, resume/refresh
 RTPVideoSending --> RTPVideoSending: picture_fast_update refresh
 state RTPVideoPaused {
  [*] --> RTCPStillRunning
  RTCPStillRunning --> RTCPStillRunning: send periodic RTCP MUST
 }
 SDPVideoNegotiated --> SDPChanged: independent SIP/SDP renegotiation
 RTPVideoSending --> SDPChanged: independent SIP/SDP renegotiation
 RTPVideoPaused --> SDPChanged: independent SIP/SDP renegotiation
~~~

**Not a captured packet trace:** CVP source selection and OVS source processing are different agents; this state diagram does not invent resume packet timing, camera LED state, encoder IDR request responses or ICE policy.

## 3. RFC and Microsoft crosswalk + unresolved evidence

| Control | Source | Wire message / actor | Distinguishing state |
|---|---|---|---|
| `picture_freeze` | MS-XMLMC rev10 §2.2.1;§3.1.5 | **CVP→OVS SIP INFO XML** | RTP stop SHOULD, **RTCP must continue** |
| `picture_fast_update` | IETFDRAFT-XMLSMC-12 §4 via MS-XMLMC rev10 §3.1.5 | **CVP→OVS SIP INFO XML** | refresh/resume video packet source; no guaranteed decoded frame |
| SDP `a=inactive` or `a=recvonly` | RFC3264 §§6–8; RFC4566 §6 | **SIP offer/answer** | renegotiated media direction, unrelated to ephemeral freeze |
| RTCP loss/feedback | RFC3550 + MS-RTP rev17 | **RTP control channel** | sender adaptation/network quality, not CVP camera selection |
| Video packet NAL/FEC | MS-H264PF rev16 and MS-RTVPF rev11 | **RTP media stream** | codec frame data, independent of SIP INFO |
| Conference role/source roster | MS-CONFAV + MS-CONFBAS | **Focus/C3P and MCU** | meeting membership and selected video source, not video freeze timer |
| SIP and media security | MS-SIPAE and MS-SRTP/SSRTP | **SIP MIC / SDES SRTP security context** | authenticated control and media encryption separate |

**Open normative issues:** (a) exact 2006/2008 historic `IETFDRAFT-XMLSMC-12` XSD and RFC successor section comparison; (b) SIP INFO package use/negotiation [RFC6086] vs historical [RFC2976] (not automatically upgraded); (c) OVS support and CVP topology for 2015 Windows binaries; (d) SIP INFO malformed XML responses, deadlines, injection/replay threats; (e) latest MS-CONFAV/RTCP codec feedback interactions and video switch duplicate/late INFO races. All remain **U**, no missing details invented.

**Research status NOT COMPLETE; no code, server communications, interoperability tests, main merge or release.**
