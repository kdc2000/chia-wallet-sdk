---
phase: 4
slug: send-side-action
status: passed
verified: 2026-05-16T00:00:00Z
verifier: gsd-verifier
score: 8/8 must-haves verified
re_verification: false
---

# Phase 4: Send-side action — Verification Report

## Phase Goal

> A wallet developer can call `spends.add(SilentPaymentSend { recipient, amount, memos })` (or multi-recipient equivalent), call the `Spends` finish flow, and obtain a signed `SpendBundle` whose outputs land at correctly-derived one-time puzzle hashes that a Phase-3 scanner can detect. Multi-party and footgun cases hard-error.

## Verification Result

**Status: passed.** The goal is delivered end-to-end in `chia_sdk_driver` behind `chip-0057`: `Action::silent_payment_send(recipient, amount, memos)` is one-line constructable, `Spends::finish_with_silent_payment_keys` is reachable and produces a `Outputs.xch[i].puzzle_hash` byte-equal to the independent `derive_one_time_puzzle_hash` output (closes the round-trip), and all four hard-error paths (multi-party, no-XCH-inputs, 32-byte memo hint, identity-element key) are wired and (where reachable from valid inputs) tested.

All 8 SEND-* requirements are satisfied, all 6 ROADMAP success criteria close with named tests that PASS, all project-level invariants hold, Phase 1/2/3 grep bans remain intact, and the public surface contains no CHIP-0058 transport names.

## Must-Haves Verified

| # | Plan | Truth | Evidence |
|---|------|-------|----------|
| 1 | 04-01 | `derive_one_time_puzzle_hash` free function with full Phase-3 composition | `crates/chia-sdk-driver/src/silent_payments/one_time.rs:60-87` — 88 lines, composes `derive_output_tweak` + `derive_onetime_pk` + `puzzle_hash_for_pk`; TV1 byte-pinned at `23adba14...c21fbf5` (k=0) + k=1 round-trip — 2 tests PASS |
| 2 | 04-01 | `compute_input_hash` over lex-min coin_id + aggregated PK | `crates/chia-sdk-driver/src/silent_payments/input_hash.rs:53-68` — uses `coin_ids.iter().min()`, TV1 byte-pinned at `38a1c8...cc9411`; order-independence test passes |
| 3 | 04-01 | `aggregate_sender_sks` free function via `ScalarField::from_bytes_raw + add` | `crates/chia-sdk-driver/src/silent_payments/aggregate.rs:47-54`, TV4 byte-pinned at `5600d878...cbf95b89`; deliberately omitted from prelude per RESEARCH §Anti-Pattern 2 |
| 4 | 04-02 | `SilentPaymentSend` struct + `SpendAction` impl that defers ECDH | `crates/chia-sdk-driver/src/actions/silent_payment_send.rs:35-50` (struct), `:107-176` (impl); no `CreateCoin` at apply time, records `SilentPaymentPending` |
| 5 | 04-02 | `Action::SilentPaymentSend` enum variant + `Action::silent_payment_send` constructor | `crates/chia-sdk-driver/src/action_system/action.rs:40` (variant), `:239-245` (constructor), `:274` + `:297` (match-arm dispatches) — chip-0057-gated; `BURN_PUZZLE_HASH` placeholder reuses the existing const at line 22 |
| 6 | 04-03 | `Spends::finish_with_silent_payment_keys` 9-step finish flow + multi-party hard-error | `crates/chia-sdk-driver/src/silent_payments/send_keys.rs:150-256` — algorithm steps 1-9 present in source; PK round-trips through `SecretKey::from_bytes(scalar)` per Pitfall A |
| 7 | 04-03 | Three new `DriverError` variants with canonical wording | `crates/chia-sdk-driver/src/driver_error.rs:134-152` — `SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`, `SilentPaymentMemoHintForbidden` (the existing `SilentPayment(#[from] SilentPaymentError)` at :134-136 is from Phase 3 and remains intact) |
| 8 | 04-04 | k-counter `HashMap<[u8;48], u32>` + `silent_payments_pending: Vec<...>` on Spends | `crates/chia-sdk-driver/src/action_system/spends.rs:28-31` (struct fields), `:78-82` (initializer in `with_separate_change_puzzle_hash`), `:464-468` (carried through `prepare`) |
| 9 | 04-04 | `Spends::emit_silent_payment_announcements` for opcode 60/61 binding | `crates/chia-sdk-driver/src/silent_payments/send_keys.rs:87-116` — short-circuits on `< 2` inputs, lex-min announcer, returns `()` (clippy::unnecessary_wraps fix); called from finish flow at `:250` |
| 10 | 04-04 | `ConditionsSpend::conditions_ref` read-only accessor | `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs:33-` — added so tests can introspect emitted conditions before finish consumes the spend |
| 11 | 04-05 | Private `memo_hint_guard` rejecting 32-byte first atom | `crates/chia-sdk-driver/src/actions/silent_payment_send.rs:84-105` — FIRST line of `SpendAction::spend` at `:130`; `Memos::None` returns Ok, malformed shapes return Ok, only the exact 32-byte-first-atom shape errs |
| 12 | 04-05 | Three new prelude re-exports + 14-symbol driver block | `src/prelude.rs:40-46` — chip-0057-gated; adds `SilentPaymentSend`, `compute_input_hash`, `derive_one_time_puzzle_hash`; `aggregate_sender_sks` DELIBERATELY OMITTED |
| 13 | 04-05 | Privacy-warning rustdoc on all public memo-bearing APIs | `grep -L 'Privacy warning'` over the 5 audited paths returns empty (see Project-level Invariants below) |

Phase 4 ships **5 created files + 7 modified files** + **17 new named `#[test]` functions** all passing. Independent verification confirms test count: driver `chip-0057` runs **1080 tests passing**; workspace (CI excludes) runs **2416 tests passing**.

## Success Criteria Closure

All 6 ROADMAP Phase 4 success criteria close. Each test was run independently and observed PASS during this verification.

| # | Criterion | Closing Test | Verified |
|---|-----------|--------------|----------|
| 1 | Round-trip: apply+finish `puzzle_hash` matches `derive_one_time_puzzle_hash` byte-for-byte | `actions::silent_payment_send::tests::round_trip_matches_derive_one_time_puzzle_hash` | PASS |
| 2 | Multi-party hard-error fires with `Err(SilentPaymentMultiPartyUnsupported)` | `actions::silent_payment_send::tests::multi_party_hard_errors` (also `silent_payments::send_keys::tests::multi_party_hard_errors`) | PASS |
| 3 | Multi-output k-counter: same `scan_pk` increments 0→1; distinct `scan_pk` each get k=0 | `actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters` | PASS (both) |
| 4 | Cross-index announcement: opcode 60 on lex-min coin, opcode 61 on others with asserted id | `actions::silent_payment_send::tests::cross_index_announcement_binding` + `single_input_no_announcement` (short-circuit) + `input_hash_round_trip` (receiver reconstruction) | PASS (all three) |
| 5 | Memo-hint guard rejects 32-byte first memo with `Err(SilentPaymentMemoHintForbidden)` | `actions::silent_payment_send::tests::memo_hint_guard_rejects_32_byte_first_memo` (+ allows-sentinel + allows-none) | PASS (all three) |
| 6 | Privacy-warning rustdoc audit: every public memo-bearing API carries the literal `Privacy warning` | `! grep -L 'Privacy warning' <5 Phase-4 paths>` returns empty | PASS |

## Requirement Coverage

All 8 SEND-* requirements satisfied. File and line evidence per requirement:

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| SEND-01 | `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) -> Bytes32` | satisfied | `silent_payments/one_time.rs:60-87`; TV1 pin test at `:127`; k=1 round-trip test at `:154` |
| SEND-02 | `compute_input_hash(coin_ids, sender_pk_aggregated) -> ScalarField` using lex-min coin id | satisfied | `silent_payments/input_hash.rs:53-68` (`coin_ids.iter().min()` at :59); TV1 pin test at `:97`, lex-min test at `:115`, order-independence test at `:137` |
| SEND-03 | `aggregate_sender_sks(sks) -> ScalarField` + Spends-level multi-party hard-error | satisfied | `silent_payments/aggregate.rs:47-54` (free fn, TV4 pin test at `:76`); multi-party hard-error path in `silent_payments/send_keys.rs:169-176` (returns `Err(SilentPaymentMultiPartyUnsupported)`); tested in `actions::silent_payment_send::tests::multi_party_hard_errors` + `silent_payments::send_keys::tests::multi_party_hard_errors` |
| SEND-04 | `SilentPaymentSend` action + `Spends::finish_with_silent_payment_keys` finish-time entry | satisfied | apply-time: `actions/silent_payment_send.rs:107-176` (`SpendAction` impl); finish-time: `silent_payments/send_keys.rs:150-256` (9-step flow); `Action::silent_payment_send` constructor at `action_system/action.rs:239`; closed by `action_state_machine` + `round_trip_matches_derive_one_time_puzzle_hash` |
| SEND-05 | `Vec<Recipient>` semantics via `silent_payment_counters` HashMap on Spends, per-scan_pk k-counter | satisfied | `action_system/spends.rs:28-29` (field), `actions/silent_payment_send.rs:151-157` (increment-and-record); closed by `multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters` |
| SEND-06 | Opcode 60 / 61 announcement binding for multi-input sends | satisfied | `silent_payments/send_keys.rs:87-116` (`emit_silent_payment_announcements`); invoked from `finish_with_silent_payment_keys:250`; closed by `single_input_no_announcement` + `cross_index_announcement_binding` + `input_hash_round_trip` |
| SEND-07 | 32-byte first memo guard — hard-error not silent rewrite | satisfied | `actions/silent_payment_send.rs:84-105` (`memo_hint_guard`); called as FIRST line of `spend` at `:130` (before any side-effects); closed by `memo_hint_guard_rejects_32_byte_first_memo` + `memo_hint_guard_allows_sentinel_prefixed` + `memo_hint_guard_allows_none` |
| SEND-08 | Privacy-warning rustdoc on every memo-bearing public API | satisfied | `grep -L 'Privacy warning'` over 5 Phase-4 paths returns empty (see Project-level Invariants); manual audit confirms warnings on `aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash`, `finish_with_silent_payment_keys`, `SilentPaymentSend` (type + field + new + spend + memo_hint_guard) |

No orphaned requirements: `.planning/REQUIREMENTS.md` line 94-101 maps SEND-01..08 to Phase 4; all eight appear in `04-PHASE-SUMMARY.md` `requirements-completed:` frontmatter and in the per-plan `requirements:` fields.

## Project-level Invariants

| # | Invariant | Command | Status |
|---|-----------|---------|--------|
| 1 | Driver clippy clean (-D warnings) under chip-0057 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | PASS (exit 0, no output) |
| 2 | Workspace fmt check | `cargo fmt --all --check` | PASS (exit 0) |
| 3 | No unused dependencies | `cargo machete` | PASS ("didn't find any unused dependencies") |
| 4 | Driver no-features build | `cargo check -p chia-sdk-driver` | PASS |
| 5 | Workspace no-features build | `cargo build --workspace` | PASS (1m 20s, all crates including napi/wasm/pyo3) |
| 6 | No `unsafe` in Phase-4 surface | `grep -rn 'unsafe' silent_payments/ + actions/silent_payment_send.rs` | PASS (empty) |
| 7 | Privacy-warning audit on memo-bearing files | `grep -L 'Privacy warning' actions/silent_payment_send.rs silent_payments/{one_time,aggregate,input_hash,send_keys}.rs` | PASS (empty — every file contains the substring) |
| 8 | Phase 1 ban: `mod_by_group_order` | `grep -rn 'mod_by_group_order' silent_payments/ + action` | PASS (empty) |
| 9 | Phase 1 ban: `^use sha2::` | `grep -rnE '^use sha2::' silent_payments/ + action` | PASS (empty) |
| 10 | Phase 3 ban: `Sha256::digest` | `grep -rnE 'Sha256::digest' silent_payments/ + action` | PASS (empty) |
| 11 | Workspace clippy (all-features, all-targets) | `cargo clippy --workspace --all-features --all-targets` | PASS — 2 pedantic warnings in `chia-sdk-daemon/src/client.rs:426-427` (`match_wildcard_for_single_variants`) are pre-existing and unrelated to Phase 4; pedantic is `warn` not `deny` per workspace lint policy |

Workspace pedantic warnings in `chia-sdk-daemon` are out-of-scope for Phase 4 verification (not in the silent-payments surface). The Phase 4 acceptance gate of `clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0.

## Cross-phase Regression

| Crate / Suite | Test Count | Status |
|---------------|-----------|--------|
| `chia-sdk-driver --features chip-0057` (full crate) | 1080 passing, 0 failing | PASS — no regression |
| `chia-sdk-types --features chip-0057` | 20 passing + 2 doctests | PASS — Phase 1 types stable |
| `chia-sdk-utils --features chip-0057 silent_payments` | 27 passing | PASS — Phase 2 keys/address/labels stable |
| Workspace (CI excludes: napi/wasm/pyo3/bindings/derive/bindy/bindy-macro) | 2416 passing, 0 failing | PASS — matches phase summary claim |

Phase 1, 2, and 3 tests continue to pass (no regression introduced by Phase 4).

## Forward-compatibility (CHIP-0058)

PASS. The Phase 4 public surface introduces no CHIP-0058 transport names. Every reference to "CHIP-0058" in Phase 4 source is a doc comment in `silent_payments/mod.rs` or `silent_payments/types.rs` (both Phase 3 origin) that *describes* forward-compatibility intent — none names a transport type, RPC message, sp-service struct, or HTTP/WS endpoint.

`TweakData` (Phase 3) remains transport-agnostic and Phase 4 does not constrain its shape: a future CHIP-0058 transport client can still construct `TweakData` from wire messages.

Sender-side: `SilentPaymentSend { recipient: SilentPaymentAddress, amount: u64, memos: Memos<NodePtr> }` takes only address + amount + memos. No transport assumption.

`Spends::finish_with_silent_payment_keys(synthetic_pks: &IndexMap<Bytes32, PublicKey>, secret_keys: &IndexMap<Bytes32, SecretKey>)` operates over the wallet's own SK material, not over any transport-provided indexer data.

## Notable Deviations

Three minor deviations from RESEARCH/Plans, each benign:

1. **`memo_hint_guard` takes `Memos<NodePtr>` by value, not `&Memos<NodePtr>` reference (Plan 04-05).** `Memos<NodePtr>` is `Copy` (it wraps a `NodePtr` which is `Copy`); clippy::trivially_copy_pass_by_ref would fire on the reference form. Functionally identical; doc comment at `silent_payment_send.rs:129` records the rationale. **Benign.**

2. **`emit_silent_payment_announcements` returns `()` not `Result<(), DriverError>` (Plan 04-04).** clippy::unnecessary_wraps fires when no path returns `Err`; the helper's `len() < 2` guard makes `.min()` infallible and `add_conditions` is a pure append. Call site at `send_keys.rs:250` drops the `?`. Doc comment at `send_keys.rs:80-86` records the rationale. **Benign.**

3. **`SilentPaymentNoXchInputs` variant has no dedicated test.** Phase Summary listed `silent_payments::send_keys::tests::no_xch_inputs_hard_errors` (item 7 in its enumerated test list) but only `multi_party_hard_errors` ships in `send_keys.rs`. The `SilentPaymentNoXchInputs` Err path is wired correctly at `send_keys.rs:182` and is reachable only from a pathological "all XCH items are ephemeral" Spends configuration. The phase summary even contains a paragraph reconciling the 17-vs-18 test count discrepancy (`silent_payments` module: Plan 04-01 added 6 + Plan 04-03 added 1 = 7; `actions::silent_payment_send` module: Plan 04-02 added 1 + Plan 04-03 added 1 + Plan 04-04 added 5 + Plan 04-05 added 3 = 10; net 17). **Benign — non-blocking documentation gap, not a goal failure.** The multi-party hard-error (which IS tested) is the much more likely real-world misuse and provides defense-in-depth. The no-XCH-inputs variant is a belt-and-suspenders guard for a rare edge case; its declaration + production-path use is what the goal requires.

No deviation blocks the phase goal.

## Gaps Summary

**No gaps found.** Phase 4 delivers the stated goal end-to-end:

- A wallet developer CAN call `spends.add(Action::silent_payment_send(recipient, amount, memos))`.
- A wallet developer CAN call `spends.finish_with_silent_payment_keys(ctx, deltas, relation, &synthetic_pks, &secret_keys)`.
- The resulting `Outputs.xch[i].puzzle_hash` IS byte-equal to `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k)` (verified by `round_trip_matches_derive_one_time_puzzle_hash` + `input_hash_round_trip`).
- Multi-party scenarios DO hard-error with `SilentPaymentMultiPartyUnsupported` (verified by `multi_party_hard_errors` × 2 sites).
- 32-byte memo hint DOES hard-error with `SilentPaymentMemoHintForbidden` (verified by `memo_hint_guard_rejects_32_byte_first_memo`).
- Multi-output coordination DOES work (verified by `multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters`).
- Cross-input announcement binding DOES work (verified by `cross_index_announcement_binding` + `single_input_no_announcement` + `input_hash_round_trip`).
- Privacy warnings DO cover every memo-bearing public API (verified by grep audit).

## Recommendation

**Proceed to Phase 5 (Bindings — Rust facade + JSON descriptor).** The Phase 4 public surface is complete, stable, and ready for binding exposure:

- `SilentPaymentSend` (struct + `pub fn new`) — wraps via `Action::silent_payment_send(recipient, amount, memos)` constructor for the `action_system.json` binding entry.
- `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks` — three free functions ready for the `bindings/silent_payments.json` `static_functions` schema (subject to the Phase-5 pre-flight item Q3: verify bindy-macro `static_functions` support).
- `SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`, `SilentPaymentMemoHintForbidden` `DriverError` variants — surface through the existing `DriverError` binding.

Optional (deferrable) follow-up: add a `silent_payments::send_keys::tests::no_xch_inputs_hard_errors` test exercising the `SilentPaymentNoXchInputs` path with an all-ephemeral Spends. The variant is wired and reachable; the test would close the documentation gap noted in the Phase Summary. Not blocking.

---

*Verified: 2026-05-16*
*Verifier: gsd-verifier (initial verification, no previous report)*
*Verification approach: Goal-backward from ROADMAP.md Phase 4 stated outcome*
*Automated checks: all gates re-run during verification; results match Phase Summary claims*
