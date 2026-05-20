---
phase: 8
slug: second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-20
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust 1.90.0 built-in harness) + grep/wc/file-existence oracles for POLISH-01/02/03/04 acceptance |
| **Config file** | none (Cargo auto-discovers); `rust-toolchain.toml` pins toolchain to 1.90.0 |
| **Quick run command** | `cargo test --release -p chia-sdk-driver --features chip-0057 --lib silent_payments` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Integration target** | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` |
| **Estimated runtime** | ~30 seconds (driver-only quick run); ~6 minutes (full workspace) |

---

## Sampling Rate

- **After every task commit:** scoped grep oracle for the affected POLISH item + `cargo build --release -p chia-sdk-driver --features chip-0057` + `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`
- **After every plan wave:** `cargo test --release -p chia-sdk-driver --features chip-0057` + `cargo fmt --check` + workspace clippy at warn-level (Phase 7 precedent: pre-existing `chia-sdk-daemon` warnings remain)
- **Before `/gsd:verify-work`:** all 11 cross-cutting CI gates from RESEARCH.md §Validation Architecture + all 11 per-requirement acceptance oracles + full workspace test suite green
- **Max feedback latency:** ~30 seconds (driver-only quick run); ~5 seconds for any single grep oracle

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 8-*-* | TBD | 1 | POLISH-01 | grep oracle | `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returns 0 | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-01 | build oracle | `cargo build --release --workspace --all-features` clean | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-01 | prelude smoke | `cargo build --release -p chia-wallet-sdk --all-features` clean (prelude re-exports all 13 SP names) | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-03 | grep oracle | `grep -c 'Cannot derive .Copy.' crates/chia-sdk-driver/src/action_system/send_destination.rs` returns 0 | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-03 | grep oracle | `grep -c 'large_enum_variant' crates/chia-sdk-driver/src/action_system/send_destination.rs` returns exactly 1 | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-04 | precondition grep | `grep -n 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs` returns 2 hits pre-edit (Risk R4) | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-04 | grep oracle | `grep -c 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs` returns 0 | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-04 | grep oracle | `grep -c 'handle_silent_payment_send' crates/chia-sdk-driver/src/actions/send.rs` returns exactly 1 | ✅ | ⬜ pending |
| 8-*-* | TBD | 1 | POLISH-04 | regression | `cargo test --release -p chia-sdk-driver --features chip-0057 -- test_action_send_xch test_action_send_cat test_action_send_xch_with_change action_state_machine silent_payment_destination_requires_xch_id silent_payment_keys_not_registered_errors_at_finish` all pass | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | file-existence oracle | `test ! -f crates/chia-sdk-driver/src/silent_payments/aggregate.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/input_hash.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/one_time.rs` | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | wc oracle | `wc -l crates/chia-sdk-driver/src/silent_payments/protocol.rs` returns ≤ 700 (target ~640) | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | test-name parity | `cargo test --release -p chia-sdk-driver --features chip-0057 -- tv4_aggregate_sender_sks_matches tv1_compute_input_hash_matches input_hash_uses_lex_min_coin_id input_hash_order_independent tv1_derive_one_time_puzzle_hash_matches derive_one_time_puzzle_hash_k1_round_trip tv1_shared_secret_matches adversarial_ff32_scalar_reduces_unsigned` all 8 pass | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | test-count parity | `cargo test --release -p chia-sdk-driver --features chip-0057 2>&1 \| grep -E 'test result:' \| awk '{print $4}' \| paste -sd+ \| bc` equals pre-phase total | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | rustdoc oracle | `cargo doc -p chia-sdk-driver --features chip-0057 --no-deps` returns 0 errors (intra-doc links resolve post-fold; Risk R7) | ✅ | ⬜ pending |
| 8-*-* | TBD | 2 | POLISH-02 | callsite smoke | `cargo build --release --workspace --all-features` clean (verifies all 6 external callsites still resolve: chia-sdk-bindings, chia-sdk-test, silent_payments_e2e.rs, prelude, silent_payment_send.rs, action_system/spends.rs) | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Task IDs are placeholder (`8-*-*`); planner replaces with concrete `8-NN-MM` IDs and assigns plans.*

---

## Wave 0 Requirements

*None — Phase 8 introduces no new tests.* The 8 test functions across the 4 silent_payments protocol files merge in place during POLISH-02; test names and bodies stay byte-identical (only file location changes). The acceptance oracles are all native CLI: `grep`, `wc`, `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`, `cargo machete`, `cargo doc`, `test -f` / `! -f`.

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| POLISH-02 test merger reads cleanly | POLISH-02 | Whether to flat-merge tests vs. nest as sub-mods is a readability judgment (RESEARCH.md Open Q3 recommends flat); not grep-checkable | After fold, visually inspect `protocol.rs` `#[cfg(test)] mod tests {}` block. Confirm test grouping (test-vector-1, input-hash, derive-one-time, etc.) reads logically end-to-end. |
| POLISH-04 dispatch flow reads top-to-bottom | POLISH-04 | Whether the new exhaustive `match` block reads better than the if-let + post-match `unreachable!()` is judgment | Read `send.rs` lines around the dispatch site. Confirm both arms (`PuzzleHash`, `SilentPayment`) are visible in a single match statement, no orphaned comments referencing the deleted shape. |
| protocol.rs rustdoc header opportunistic update | POLISH-02 (Claude's Discretion / Open Q5) | Whether the rustdoc on `silent_payments/mod.rs` or `protocol.rs` should mention the absorbed compositions is judgment | Read post-fold rustdoc. Confirm it doesn't claim a module structure that no longer exists. Updating it is optional polish on top of POLISH-02. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (N/A — no Wave 0 needed)
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s for driver-only quick run
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
