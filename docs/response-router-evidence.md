# v0.13.0 — Non-INVITE response router: RFC/MS traceability

**Evidence class:** authored source, synthetic Rust tests, Python source/manifest/hash checks. **Not a compiled Rust test report and not Skype for Business interoperability certification.** RFC and Microsoft pages reviewed on 2026-10-15. These components DO NOT transmit packets or authenticate users.

| ID | Normative source and section | Requirement / implemented subset | Code | Test name and execution status |
|---|---|---|---|---|
| ROUTER-RFC-01 | [RFC 3261 §17.1.2](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.2) | Distinguish non-INVITE provisional 1xx and final 2xx-6xx; 401/407 terminate their original transaction; new REGISTER retry uses a new transaction | `commscraft-core/src/transaction.rs`, `commscraft-router/src/lib.rs` | `concurrent_transactions_route_out_of_order`, `challenge_is_final_and_retries_need_new_identity` — Rust **NOT RUN** |
| ROUTER-RFC-02 | [RFC 3261 §17.1.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.3), [§8.1.3.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-8.1.3.3) | Match top Via branch and CSeq method. This implementation additionally enforces Call-ID, CSeq number, sent-by and Via transport (strict subset for locally authored requests). Do not confuse TLS/TCP | `commscraft-core/src/transaction.rs::TransactionKey::from_message`, `commscraft-router/src/lib.rs::ResponseRouter::response` | `rejects_wrong_cseq_call_id_branch_and_transport_without_consumption`, `malformed_response_does_not_consume_tracked_request` — Rust **NOT RUN** |
| ROUTER-RFC-03 | [RFC 3261 §17.1.2.2](https://www.rfc-editor.org/rfc/rfc3261.html#section-17.1.2.2) | Timer F remains 64*T1 even after provisional responses; reliable connections use no Timer E retransmissions; final response terminates immediately (Timer K = 0) | `commscraft-core/src/transaction.rs`, `commscraft-router/src/lib.rs::tick` | `timer_f_is_not_reset_by_provisional_responses`, `late_final_is_rejected_without_tick` — Rust **NOT RUN** |
| ROUTER-RFC-04 | [RFC 3261 §18.4](https://www.rfc-editor.org/rfc/rfc3261.html#section-18.4) | Explicit disconnect terminates all pending transactions, with no router reuse on a new TLS peer | `commscraft-router/src/lib.rs::disconnect` | `disconnect_terminates_all_and_prevents_peer_reuse` — Rust **NOT RUN** |
| ROUTER-MS-01 | [MS-SIPAE §4.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/b3989651-d013-4481-8bb0-c4fd1678300a) | Microsoft's REGISTER → 401 Kerberos challenge is modeled as a final response to the first transaction, **not authentication completion** | `commscraft-router/src/lib.rs`, `apps/commscraft-inspect/src/main.rs::router_demo` | `challenge_is_final_and_retries_need_new_identity`, `--router-demo` — Rust **NOT RUN** |
| ROUTER-MS-02 | [MS-OCSPROT §2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-ocsprot/45e475a0-b534-469a-892e-6bd6ae42fc6e) and [MS-SIPREGE §3.1.2.5.1](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/8b6cbbb4-b398-4b55-847d-950b9a8d3010) | Registry/SA and provisioning remain **outside** the router. No `200 OK` routed event can become trusted sign-in without signed response validation, transport trust and registrar binding | `commscraft-router/src/lib.rs` (no live transport API), existing `commscraft-core/src/registration.rs` | Source-contract tests and release block — Python can run; Windows SfB **NOT RUN** |
| ROUTER-RFC-05 | [RFC 3261 §§7.5,18.3](https://www.rfc-editor.org/rfc/rfc3261.html#section-18.3) | Independently frame fragmented/coalesced replies before dispatch | `commscraft-core/src/stream.rs`, `commscraft-router/tests/offline_wire.rs` | `framed_tls_like_stream_routes_two_replies_without_interleaving`, `duplicate_final_from_coalesced_stream_is_not_delivered_twice` — Rust **NOT RUN** |
| ROUTER-DEFENSE-01 | Implementation defense-in-depth, **not an RFC-mandated limit** | At most 64 in-flight, 128 recently completed identities, monotonic timer input and fail-closed duplicate responses | `commscraft-router/src/lib.rs` | `capacity_is_bounded_and_recovers`, `exceeding_recent_history_evicts_oldest` — Rust **NOT RUN** |

## Compatibility and evidence caveats

- The extra Call-ID, CSeq number, Via sent-by and transport checks are **stricter than RFC §17.1.3**; Microsoft deployments that rewrite these fields can require compatibility changes after real packet-capture comparison. Do not claim universal RFC transaction-layer compatibility.
- `MAX_RECENTLY_CLOSED` is bounded recent-history protection, **not** a globally persistent anti-replay guarantee and **not** a substitute for Microsoft SIPAE sequence/MIC checking.
- This router is a standalone, offline component; it is **not yet wired to `TlsSipReceiver` for authenticated sending** and does not handle server-initiated NOTIFY, INVITE/ACK transactions, dialog creation, NAT or keep-alive policy.
- Runtime SSL certificate verification, Windows Kerberos, real registrar negotiation, Microsoft signed REGISTER, presence and IM, calls and all Windows feature-parity tests are outstanding.
- The archived v0.11 and existing v0.12 hashes remain unmodified, including `commscraft-core/src/lib.rs` and the original GitHub workflow. A new, separate CI workflow runs the router on Linux and Windows.

## Reproducible commands

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check_v011_evidence.py
python3 scripts/check_v012_evidence.py
python3 scripts/check_v013_evidence.py
cargo test -p commscraft-router --all-targets
cargo run -p commscraft-inspect -- --router-demo
python3 scripts/check_release.py --release  # MUST EXIT NONZERO
```
