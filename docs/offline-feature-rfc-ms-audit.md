# Audit of the 25 offline partial features — 2026-10-17

**Outcome: inventory linkage completed; full normative RFC/MS section-level verification NOT completed.** The machine-readable [audit ledger](../release/offline-feature-spec-audit.json) links all 25 claimed partials to code locations, test-module candidates, public specification families, publication metadata where checked and explicit review gaps. It is *not* a conformity certificate. The original seven baseline entries and 18 development overrides remain separate: their different evidence maturity must not be concealed by the aggregate 25/122 (20.5%) number.

## Protocol references with externally checked publication metadata

- [MS-SIPAE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-sipae/ba3e9821-fa85-4e0f-a80c-5a4c720a00bd): revision **11.0**, published **2025-05-20**. Needed for Windows Kerberos/NTLM authentication; exact applicable provisions still need source-and-fixture audit.
- [MS-SIPREGE](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-siprege/21acf797-984c-48ce-97e3-c7df5c776b3d): revision **17.1**, published **2025-08-19**. REGISTER/in-band provisioning protocol, not an authenticated registration proof.
- [MS-PRES](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-pres/741d682d-590b-4600-916e-8deeee426a5c): revision **13.0**, published **2025-05-20**. Used only for protocol context for the local presence drafts.
- [MS-CONFIM](https://learn.microsoft.com/en-us/openspecs/office_protocols/ms-confim/6f14d1ec-c854-4360-bdda-a6c61a655a60): revision **11.0**, published **2025-05-20**. Multiparty local roster models do not implement the conference protocol.
- IETF references include [RFC 2782 (February 2000) §3](https://www.rfc-editor.org/rfc/rfc2782#section-3), [RFC 2743 (January 2000) §2](https://www.rfc-editor.org/rfc/rfc2743#section-2), [RFC 3261 (June 2002) §§10–12](https://www.rfc-editor.org/rfc/rfc3261), [RFC 3264 (June 2002) §5](https://www.rfc-editor.org/rfc/rfc3264#section-5) and [RFC 6665 (July 2012) §4.1](https://www.rfc-editor.org/rfc/rfc6665#section-4.1). These are **review pointers**, not section-by-section declarations of passing tests.

## Per-feature evidence classification

The seven historical partials (`dns-discovery`, `auth`, `register`, `presence-sub`, `im-1to1`, `codecs`, `windows-ui`) have a public document family and an initial source mapping, but fixture-level review and exact MS section pinning are explicitly **open**. The machine ledger marks these `unverified_mapping`.

The 18 development partials (`contact-groups`, `presence-publish`, `im-history`, `contact-search`, `contact-privacy`, `im-richtext`, `calendar-presence`, `photos`, `distribution-groups`, `im-multiparty`, `im-file`, `audio-call`, `hold`, `forwarding`, `transfer`, `conference-roster`, `devices`, `call-history`) are **local model / bounded validation only**. Their Microsoft specification references are *context only*. A value `not-applicable-local-only` is deliberate: no Microsoft wire requirement was implemented. Details and module test families are in the ledger; full end-to-end Windows behavior and security-policy enforcement are absent.

## Release impact and next steps

1. Every future protocol-bearing change must pin the **exact Microsoft edition, its section and a normative requirement**, plus test fixture identifiers and observed CI results. `UNVERIFIED` must never be promoted to a verified status by assumption.
2. The offline test job runs `python3 -m unittest discover -s scripts -p "test*.py"`, including `test_offline_feature_spec_audit.py`. This checks completeness and honesty of mappings, **not** the truth of a Microsoft specification's content.
3. Windows and Ubuntu CI are evidence only for the commands actually executed at the **same HEAD SHA**. Real-server tests require an expressly authorized lab, sanitized artifacts and independent review. No v0.30.0 or v1.0 release gate was overridden.

Historical original import SHA-256 manifests and `FEATURE_PARITY.json` are unchanged.
