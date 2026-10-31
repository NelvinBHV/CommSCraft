# Evidence: MS-QoE – quality reports and call rating

Source: [MS-QoE] "Quality of Experience Monitoring Server Protocol", edition 20250520, 175 pp. Schemas for OCS 2007 up to SfB Server (§6.1–6.5, pp. 88–151). Based on RFC 3611/4737, draft-sipsoap-00 (obtained, file 38).

| ID | Requirement | Evidence |
|---|---|---|
| QE-01 | After the end of every SIP session with media: **SERVICE** to the QoE URI (from in-band provisioning `qosUri`, file 04; example `sip:…;gruu;opaque=srvr:QoS:…`), alternatively the pool URI or Request-URI = To; response **202 Accepted** (= accepted, not processed); on error the client may resend | §1.3, pp. 11–12; §2.2, p. 13; §3.1–3.2, p. 79; example p. 82 |
| QE-02 | `Content-Type: application/vq-rtcpxr+xml`, root `<VQReportEvent xmlns="ms-rtcp-metrics">` (extensions ms-rtcp-metrics.v2/v3/v4) with VQSessionReport (SessionId = dialog ID ≤755 characters), Endpoint (Name ≤256, OS, CPU …), DialogInfo (attributes CallId, FromTag, ToTag, Start, End; FromURI, ToURI, Caller, Local/RemoteContactURI, Local/RemoteUserAgent, LocalPAI, ConfURI …), per m line a MediaLine (Label, Description: connectivity/ICE, security, transport, addresses, devices; Inbound/OutboundStream with Network (jitter, packet loss, burst gap loss, delay, utilisation), Payload (audio/video/app sharing), QualityEstimates (MOS)) | §2.2.1–2.2.1.29, pp. 13–75; example §4.1, pp. 82–84 |
| QE-03 | Call rating: `application/ms-cqf+xml`, `<CallQualityFeedbackReport xmlns="ms-cqf" CallId FromTag ToTag [Start End]>` with ReportingUserURI, Rating 1–5, optional Feedback/Text (≤4000, LanguageTag), Tokens (Id 1–5 audio, 21–25 video; Value 0/1) | §2.2.2–2.2.2.4, pp. 75–78; example §4.2, pp. 84–85 |
| QE-04 | Error responses: 606 (unknown report type), 400 (schema/values), **401 if From ≠ LocalPAI or FromURI** (anonymous: focus URI ≠ ConfURI), 415 (Content-Type), 413 (>300 KB) | §3.2, pp. 79–80 |
| QE-05 | Strings over the length limit are truncated (exceptions: SessionId/Name/CallId ⇒ rejection); format values culture-invariant | §2.2, p. 13; §2.2.1.2.2, p. 16; §2.2.1.3.2, p. 17 |
| QE-06 | Implementation (crate `qoe`, `session::qoe`, `media::Audio::quality`): after every call with media the client sends the VQReportEvent with one VQSessionReport: Endpoint Name, DialogInfo (CallId, From/To tags as the call's first INVITE had them, Start = when the call became active, End, From/To URIs, Caller, contacts, user agents, **LocalPAI = own address** so the From check of QE-04 also holds for incoming calls), the `main-audio` MediaLine with Security, Offerer, Transport, Local/RemoteAddr of the selected path and both streams (Network: Jitter/InterArrival(Max) in ms, PacketLoss/LossRate(Max), Delay/RoundTrip(Max) from LSR/DLSR (RFC 3550 §6.4.1), Utilization/Packets; Payload/Audio with codec and sample rate; no MOS, OB-47); element order of the schema; empty blocks left out. Call rating `/rate`: CallQualityFeedbackReport with ReportingUserURI, Rating 1–5, optional Feedback/Text (≤ 4000). 202 is quiet, other codes are reported | §2.2.1–2.2.2, pp. 13–78; §3.2, p. 79; RFC 3550 §6.4.1 |

Status: tested-offline (report and rating documents, session flow; network figures measured from RTP/RTCP).

## Conflicts / gaps
- K-QE-1: The Tokens schema (p. 77) defines only Id/Value (+xs:any); the example (pp. 84–85) additionally contains `<Tag>` (e.g. DistortedSpeech) – allowed via xs:any, meaning not specified. Also a schema typo `axOccurs` (p. 77, instead of maxOccurs).
- O-QE-1: Many metrics (MOS estimates, signal levels, echo) require own audio analysis; their computation is not specified (references to ITU-T P.562/P.800.1). For a Linux client only network-measurable values (jitter, loss, RTT, packets) can be filled reliably; the QoE report is optional (the server accepts partial reports per the schema – mandatory fields see QE-02).

## Later finding (file 39)
- O-QE-1 **partly** (OB-28): ITU-T P.562 and P.800.1 obtained; the SfB-specific MOS method remains undocumented.
- O-QE-1 **closed** (OB-47, normative): `QualityEstimates` and the MOS elements are optional (§2.2.1.10.1, p. 33; p. 61); the client sends network metrics only.
