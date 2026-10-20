# Automation and agent-driven workflows

The [WordCraft](https://github.com/storytold/wordcraft) project demonstrates a useful principle: one set of application commands should work consistently from the GUI, CLI and automation tools. CommSCraft may adopt that architectural pattern while implementing its own secure communications behavior.

## Proposed command surface (only one offline diagnostic implemented)

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

Status: one core-only offline command gate implemented in `crates/commscraft-core/src/offline_commands.rs`. Stable ID `diagnostics.inspect-sdp.v1` inspects caller-provided SDP bytes locally and returns a bounded structural summary. A caller must explicitly set both `user_consented` and `policy_allows`; failure is fail-closed. Command names other than the single allowlisted ID are rejected. Rust unit tests cover deterministic output, consent denial, policy denial, unknown IDs, malformed SDP and input size. No network commands, CLI/UI/MCP adapter, authenticated session, interop evidence or feature-parity upgrade exists. This is an incremental implementation of issue #5, not its completion.

## Offline command catalog (development)

The core module `offline_commands.rs` now exports immutable `COMMAND_CATALOG` metadata and `lookup_command(id)`. Only `diagnostics.inspect-sdp.v1` is registered. Each descriptor explicitly declares `read_only=true`, `offline_only=true` and the maximum accepted input size. The existing `execute_offline` path uses the allowlisted catalog lookup and still independently checks user consent, policy permission, input size and SDP validity. Lookup does not authorize execution, trigger network access or expose credentials. Unit tests check unique IDs, denied unknown versions and the offline-only descriptor invariant. This is not a general command bus, executable remote adapter or new verified Windows parity feature.
