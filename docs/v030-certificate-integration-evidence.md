# v0.30.0 combined SIP SAN security integration

**Status: source-authored, new-head CI pending; no Microsoft interoperability established.**

Dedicated branch `integration/v030-sip-san-six-prs` starts from PR #31 and explicitly folds in code/test safeguards from PRs #26–#30 without merging to main. All source changes must be reviewed against original PR diffs before merge.

| Public normative document, version and section | Source → offline fixture | Status |
| --- | --- | --- |
| RFC 3261 (June 2002) §25.1, PR #26 header syntax | `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host` → `coordinated_sip_san_negative_and_positive_matrix`: malformed header pairs vs valid `hname=hvalue` | Authored; CI pending |
| RFC 3261 §25.1, PR #27 encoded controls | Same path/fixture: encoded CR, LF, NUL, TAB, DEL rejected; visible escaped header accepted | Authored; CI pending |
| RFC 5922 (June 2010) §§7.1–7.3, RFC 5280 (May 2008) §4.2.1.6, PR #28 | `sip_certificate.rs::ascii_sip_domain`, `crates/commscraft-transport/src/lib.rs::dns_name` → numeric IPv4 negatives in both Rust modules; DNS name positive | Authored; CI pending |
| RFC 3261 §§19.1.1,25.1, PR #29 | `sip_uri_host` → malformed URI parameters negative, valid distinct parameters positive | Authored; CI pending |
| RFC 5280 §4.2.1.6, PR #30 | `sip_uri_host` → Unicode IA5 SAN suffix negative, escaped ASCII positive | Authored; CI pending |
| RFC 3261 §§19.1.1,19.1.4 and RFC 3986 (January 2005) §2.1, PR #31 | `sip_uri_host`, `match_sip_sans` → `percent_encoded_unreserved_duplicate_parameters_are_rejected` | PR #31 individually CI-passed, **combined SHA pending** |
| [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c), [MS-CONMGMT §3.1.5](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-conmgmt/40d79c10-2275-4e52-bb4b-67c703d092c3) | Independent Windows server trust/SPN/MIC/replay context: no changes to authentication or production network | Real interoperability **unverified**, exact Microsoft revision to be pinned at implementation/review |

RFC links: [3261](https://www.rfc-editor.org/rfc/rfc3261), [5922](https://www.rfc-editor.org/rfc/rfc5922), [5280](https://www.rfc-editor.org/rfc/rfc5280), [3986](https://www.rfc-editor.org/rfc/rfc3986).

Required proof: consolidated exact SHA under standard `ubuntu-latest` and `windows-latest`, strict Rust tests/Clippy, Python evidence checks and `scripts/check_standard_runners.py`. Existing Issue #9 formatting/provenance limitations remain explicit. Historical SHA-256 ledgers remain untouched. No real networks, secrets, paid runners, merges to `main` or releases. v1.0 **BLOCKED**.


## Cross-feature combination regression — 2026-10-17

The consolidated fixture `crates/commscraft-transport/src/sip_certificate.rs::tests::coordinated_sip_san_negative_and_positive_matrix` now additionally asserts rejection from both `sip_uri_host` and `match_sip_sans` for: (1) equivalent encoded duplicate `transport/%74ransport` **with otherwise well-formed URI headers**; (2) valid `lr` parameter **with malformed trailing header pair**; (3) valid `transport` parameter **with percent-encoded CR in a header value**. Code source commit `d36cffe391847c958fce5a83e63095782a9956ac`.

**Normative mapping:** [RFC 3261 (June 2002) §§19.1.1, 19.1.4, 25.1](https://www.rfc-editor.org/rfc/rfc3261); [RFC 3986 (January 2005) §2.1](https://www.rfc-editor.org/rfc/rfc3986#section-2.1); [RFC 5922 (June 2010) §§7.1–7.2](https://www.rfc-editor.org/rfc/rfc5922#section-7.1); and [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6). Escaped-control rejection is a documented conservative local trust-boundary restriction, not a claim that every control escaped in generic SIP contexts is forbidden by RFC 3261. Microsoft context [MS-SIPAE §3.2.5.2](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/26fd795e-5567-4d33-a09b-9c501f7ff01c) requires independent security association evidence, not satisfied here.

**Result:** source authored; CI results for this new integrated head **pending**, including Linux/Windows tests. Previous integration head `51acf037de3aa0493ca1915ee908200cf66f6bba` showed running/queued checks, with successful presence and router jobs at the last inspection but no complete 11/11 result. Nothing is represented as verified Microsoft interoperability. Preserve historical SHA-256 and source provenance; no nonstandard runners, networks, releases or merges.


## Bounded SAN-URI parsing — 2026-10-17

The integration branch adds *conservative local limits* of 4096 bytes per SIP URI SAN and 64 URI parameters / 64 URI header pairs, in `crates/commscraft-transport/src/sip_certificate.rs::sip_uri_host`. This bounds the existing repeated parameter-name comparisons and avoids excessive per-certificate work even below the separate 64 KiB DER ceiling. Both `sip_uri_host` and `match_sip_sans` must reject over-limit names; `bounded_sip_certificate_uri_components_fail_closed` checks an oversized SAN URI and excessive parameter/header counts while preserving an ordinary URI.

**Normative context:** [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6) defines certificate SAN uniformResourceIdentifier encoding, [RFC 5922 (June 2010) §§7.1–7.2](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) uses those identities for SIP domain matching, and [RFC 3261 (June 2002) §§19.1.1, 25.1](https://www.rfc-editor.org/rfc/rfc3261) defines SIP URI and component grammar. **4096 and 64 are implementation safety limits, NOT RFC-mandated maxima.** Any oversized but otherwise syntactically valid URI will be conservatively rejected at certificate identity boundary. Source commit `f744761ac6385eba2a19583c16e84c5f8944f9c5`.

**Execution:** Tests authored; combined new-head Ubuntu and Windows CI **not yet confirmed**. No real Microsoft interoperability, network connection, trust bypass, credential operation or version release. Prior v0.30.0 integration SHA `9efb7e0264f2fcb3fd6c74bf48043aa1a1bfafae` passed **11/11** Actions workflows; newer CI runs remain queued/in progress and must not inherit that status.


## Oversized URI SAN and DNS fallback — 2026-10-17

`crates/commscraft-transport/src/sip_certificate.rs::match_sip_sans` now refuses an entire SAN identity match if any URI SAN exceeds the 4096-byte local parsing limit, **even when a DNSName SAN otherwise matches**. Previously the oversized URI was silently ignored in `sip_uri_host`, creating a DNS fallback despite a potentially conflicting oversized SIP identity. `oversized_uri_san_cannot_fall_back_to_matching_dns_san` tests oversized conflicting SIP URI + matching DNS, plus ordinary conflicting valid URI + matching DNS. This conservative fail-closed rule is narrower than full RFC grammar and does not assert that RFC 5922 mandates the local 4096-byte limit.

Context: [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) SIP URI SAN precedence, [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6) subjectAltName. Source commit `ebe0c65530b17c4588004e3a684775735ee715ed`. Test **authored, not yet verified by CI at this SHA**; no live trust or sign-in claims.


## Malformed SIP-scheme URI SAN cannot trigger DNS fallback

`crates/commscraft-transport/src/sip_certificate.rs::match_sip_sans` refuses to authorize a domain using a matching DNS SAN when a co-present `sip:` URI SAN is malformed or outside the conservative supported grammar. Before commit `18cb192b40403e91280a84bfba90cf8ab11fc11b`, such malformed SIP URI identities were ignored, allowing a DNS SAN to authorize the connection. The check also preserves valid matching SIP URI behavior and permits an unrelated `https:` URI SAN to coexist with a matching DNS SAN. Offline fixture: `malformed_sip_uri_san_does_not_silently_enable_dns_fallback`.

Normative context: [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1) (SIP domain identity from subjectAltName), [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6) (URI SAN), [RFC 3261 (June 2002) §§19.1.1, 25.1](https://www.rfc-editor.org/rfc/rfc3261) (URI grammar). **Conservative local trust policy:** rejecting an invalid SIP URI even with a matching DNS SAN is a deliberate security restriction, not a claim that RFC 5922 mandates this exact fallback rule. New-head CI and independent interoperability **pending**.

## CI regression resolved: old userinfo-to-DNS expectation

GitHub Actions run [39005540326](https://github.com/NelvinBHV/CommSCraft/actions/runs/39005540326) failed on both standard Windows and Ubuntu jobs with `sip_certificate::tests::uri_scheme_no_userinfo_parameters_and_exact_dns_comparison`. The older assertion expected a matching DNS SAN to authorize `sip:alice@example.com`, but the current conservative RFC 5922 certificate identity subset forbids a SIP URI SAN containing userinfo and forbids fallback to DNS when a malformed `sip:` identity is present. Commit `4f1fa353810289eb490b524190d23c908fe685d8` corrects the obsolete positive assertion into a negative security regression. Source: `crates/commscraft-transport/src/sip_certificate.rs`. Normative context: [RFC 5922 (June 2010) §7.1](https://www.rfc-editor.org/rfc/rfc5922#section-7.1), [RFC 3261 (June 2002) §19.1.1](https://www.rfc-editor.org/rfc/rfc3261). This is a deliberate conservative trust-boundary policy, not a claim that the RFC universally forbids URI userinfo. **Re-run status: pending** until the new combined SHA passes. No real sign-in demonstrated.


## Multi-SIP-SAN ordering regression — 2026-10-17

New source fixture `crates/commscraft-transport/src/sip_certificate.rs::tests::one_valid_sip_uri_does_not_hide_an_invalid_sip_uri` covers certificates containing both a valid matching SIP URI SAN and a malformed, conflicting SIP URI SAN. Both SAN orderings must reject the name match, regardless of an additional matching DNS SAN. Positive cases with two valid SIP URI SANs retain correct exact-name matching, and a third unrelated name is denied. The corresponding implementation already refuses malformed SIP-scheme entries in `match_sip_sans`; this change adds missing regression coverage rather than expanding the protocol parser. Source commit `8abd9536031179ca98f48b7a6d56aca8a44c9b7e`.

Normative context: [RFC 5922 (June 2010) §§7.1–7.2](https://www.rfc-editor.org/rfc/rfc5922), [RFC 5280 (May 2008) §4.2.1.6](https://www.rfc-editor.org/rfc/rfc5280#section-4.2.1.6), and [RFC 3261 (June 2002) §19.1.1](https://www.rfc-editor.org/rfc/rfc3261). Conservative local fail-closed behavior is not claimed as a universal RFC requirement. **Result: added on integration branch; exact-head CI pending.** No authenticated Microsoft interoperability claimed.
