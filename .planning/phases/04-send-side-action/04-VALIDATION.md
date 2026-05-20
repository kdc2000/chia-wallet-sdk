---
phase: 4
slug: send-side-action
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-15
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Materialized from `04-RESEARCH.md §9 Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in libtest); `rstest 0.22.0` available as a dev-dep for parametric coverage |
| **Config file** | None — `cargo test --release -p chia-sdk-driver --features chip-0057` is the canonical invocation |
| **Quick run command** | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_send` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Estimated runtime** | ~45 seconds (driver-only feature build); ~3–5 minutes (full workspace gate) |

---

## Sampling Rate

- **After every task commit:** `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_send` (~20s)
- **After every plan wave:** `cargo test --release -p chia-sdk-driver --features chip-0057` (~45s) — also catches regressions in Phase 1/2/3 scanner code
- **Before `/gsd:verify-work`:** Full workspace suite must be green; plus `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`; plus `cargo fmt --check`; plus `cargo machete`; plus the four grep bans (`mod_by_group_order`, `use sha2::`, `Sha256::digest`, `'Privacy warning'` coverage)
- **Max feedback latency:** 45 seconds (per-wave gate)

---

## Per-Task Verification Map

> Task IDs are filled in by the planner. Each entry is sourced from `04-RESEARCH.md §9 Phase Requirements → Test Map`.

| Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01 | 1 | SEND-01 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::one_time::tests::tv1_derive_one_time_puzzle_hash_matches -- --exact` | ❌ W0 | ⬜ pending |
| 04-01 | 1 | SEND-01 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::one_time::tests::derive_one_time_puzzle_hash_k1_round_trip -- --exact` | ❌ W0 | ⬜ pending |
| 04-01 | 1 | SEND-02 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::tv1_compute_input_hash_matches -- --exact` | ❌ W0 | ⬜ pending |
| 04-01 | 1 | SEND-02 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::input_hash_uses_lex_min_coin_id -- --exact` | ❌ W0 | ⬜ pending |
| 04-01 | 1 | SEND-02 | unit (property) | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::input_hash_order_independent -- --exact` | ❌ W0 | ⬜ pending |
| 04-01 | 1 | SEND-03 (free-fn) | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::aggregate::tests::tv4_aggregate_sender_sks_matches -- --exact` | ❌ W0 | ⬜ pending |
| 04-02 | 2 | SEND-04 (apply) | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::action_state_machine -- --exact` | ❌ W0 | ⬜ pending |
| 04-03 | 3 | SEND-03 (Spends-level) | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::send_keys::tests::multi_party_hard_errors -- --exact` | ❌ W0 | ⬜ pending |
| 04-03 | 3 | SEND-04 (finish) | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::round_trip_matches_derive_one_time_puzzle_hash -- --exact` | ❌ W0 | ⬜ pending |
| 04-04 | 4 | SEND-05 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k -- --exact` | ❌ W0 | ⬜ pending |
| 04-04 | 4 | SEND-05 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_distinct_scan_pks_independent_counters -- --exact` | ❌ W0 | ⬜ pending |
| 04-04 | 4 | SEND-06 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::cross_index_announcement_binding -- --exact` | ❌ W0 | ⬜ pending |
| 04-04 | 4 | SEND-06 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::single_input_no_announcement -- --exact` | ❌ W0 | ⬜ pending |
| 04-04 | 4 | SEND-06 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::input_hash_round_trip -- --exact` | ❌ W0 | ⬜ pending |
| 04-05 | 5 | SEND-07 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_rejects_32_byte_first_memo -- --exact` | ❌ W0 | ⬜ pending |
| 04-05 | 5 | SEND-07 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_allows_sentinel_prefixed -- --exact` | ❌ W0 | ⬜ pending |
| 04-05 | 5 | SEND-07 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_allows_none -- --exact` | ❌ W0 | ⬜ pending |
| 04-05 | 5 | SEND-08 | doc / grep | `! grep -L 'Privacy warning' crates/chia-sdk-driver/src/actions/silent_payment_send.rs crates/chia-sdk-driver/src/silent_payments/{one_time,aggregate,input_hash,send_keys}.rs` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Files that must exist before tests can pass (created during plan execution):

- [ ] `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — SEND-04, SEND-05, SEND-06, SEND-07
- [ ] `crates/chia-sdk-driver/src/silent_payments/one_time.rs` — SEND-01
- [ ] `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` — SEND-02
- [ ] `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` — SEND-03 (free function)
- [ ] `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` — SEND-03 (Spends-level hard-error) + `SilentPaymentPending` + `Spends::finish_with_silent_payment_keys`
- [ ] `crates/chia-sdk-driver/src/silent_payments/memo_guard.rs` (or inlined in `silent_payment_send.rs`) — SEND-07 helper
- [ ] `crates/chia-sdk-driver/src/silent_payments/mod.rs` — extend barrel with 4 new modules + re-exports
- [ ] `crates/chia-sdk-driver/src/actions.rs` — add `#[cfg(feature = "chip-0057")] mod silent_payment_send;` + re-export
- [ ] `crates/chia-sdk-driver/src/action_system/action.rs` — add `Action::SilentPaymentSend` variant + constructor + match arms (3 sites)
- [ ] `crates/chia-sdk-driver/src/action_system/spends.rs` — add `silent_payment_counters` + `silent_payments_pending` fields (chip-0057-gated)
- [ ] `crates/chia-sdk-driver/src/driver_error.rs` — add 3 new variants (`SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`, `SilentPaymentMemoHintForbidden`)
- [ ] `src/prelude.rs` — extend `#[cfg(feature = "chip-0057")]` block with `SilentPaymentSend`, `derive_one_time_puzzle_hash`, `compute_input_hash`

*Test framework: cargo + libtest are already present. No install command required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Privacy-warning rustdoc text reads naturally to a human consumer | SEND-08 | Grep proves presence, not quality | After Plan 04-05, run `cargo doc -p chia-sdk-driver --features chip-0057 --no-deps --open` and skim the public memo-bearing items in `silent_payments` and `actions::silent_payment_send`; confirm wording matches the canonical phrasing in `04-RESEARCH.md §8` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 45s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
