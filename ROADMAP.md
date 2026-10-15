# CommSCraft roadmap

v1.0 must match the classic Skype for Business Windows Full Client for supported on-premises server features and policy configurations. Progress means verified implementation, never a version label.

| Phase | Required result |
| --- | --- |
| M0 Source import | Entire historical v0.14.0 workspace, RFC/MS evidence and test fixtures imported; Linux/Windows CI |
| M1 Mutual authentication | Real TLS/SSPI/GSS security association and signed REGISTER interop |
| M2 Presence/contacts/IM | Bidirectional interoperability and policy negative cases |
| M3 Content/file transfer | Every Windows reference transfer feature |
| M4 Audio/video/Enterprise Voice | SDP/ICE/STUN/TURN/RTP/SRTP and call controls |
| M5 Conferencing and sharing | Meetings, roster, presentation, screen/application sharing and control |
| M6 Office and Windows integration | Outlook, presence integration, delegate flows, accessibility and notifications |
| M7 Release qualification | Entire Windows Full Client inventory independently signed off |

The prepared local baseline tracks 122 requirements and 93 reference cases; neither number is evidence of implementation. See docs/windows-parity.md.