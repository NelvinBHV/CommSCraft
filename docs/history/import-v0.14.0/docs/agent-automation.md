# Automation and agent-driven workflows

The [WordCraft](https://github.com/storytold/wordcraft) project demonstrates a useful principle: one set of application commands should work consistently from the GUI, CLI and automation tools. CommSCraft may adopt that architectural pattern while implementing its own secure communications behavior.

## Proposed command surface (NOT IMPLEMENTED)

- read-only local diagnostics: inspect synthetic SIP, SDP, transaction traces, protocol evidence
- explicit configuration and safe connection status reporting
- future contact/presence/IM operations only after fully authenticated and policy-checked sessions
- future call/conference commands only after user consent and server authorization
- optional MCP adapter only after read-only sandboxing, secret handling and approval semantics are reviewed

## Security boundaries

Agents must NOT be allowed to retrieve or export Kerberos/NTLM tokens, silently connect to corporate servers, record calls, inspect another user's chats or bypass server permissions.

Define an allow-list with per-operation consent and auditable actions. Do not expose live mutation operations until authorization, privacy and interoperability testing is complete.

Every command must use the same core state machine as the desktop app; no separate permissive implementation for agents. Source fixtures and diagnostics should be deterministic and usable offline.

## Acceptance evidence

Command IDs; input validation; tests for missing consent and malformed data; explicit negative permissions; behavior parity with the Windows Full Client where applicable; Microsoft/RFC references for all protocol effects.

Status: design proposal only. The previous local sfb-inspect tool is a limited offline diagnostic CLI and is not a general command bus.
