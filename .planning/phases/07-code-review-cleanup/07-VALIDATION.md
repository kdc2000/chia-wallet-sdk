---
phase: 7
slug: code-review-cleanup
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-20
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace standard) + grep-based oracles for CLEANUP-01/02/03/04 acceptance + manual frontmatter inspection for CLEANUP-06 |
| **Config file** | none (Cargo auto-discovers); `rust-toolchain.toml` pins toolchain to 1.90.0 |
| **Quick run command** | `cargo test --release -p chia-sdk-driver --features chip-0057 -- --test-threads=1` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Estimated runtime** | ~90 seconds (driver-only quick run); ~6 minutes (full workspace) |

---

## Sampling Rate

- **After every task commit:** Run `cargo build --release -p chia-sdk-driver --features chip-0057` plus the CLEANUP-* acceptance oracle for that task
- **After every plan wave:** Run `cargo test --release -p chia-sdk-driver --features chip-0057` + `cargo clippy --workspace --all-features --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full workspace test suite (Rust) + bindings test suites (napi pnpm, pyo3 pytest, wasm pnpm) + all 5 acceptance greps green + `cargo fmt --check` + `cargo machete`
- **Max feedback latency:** ~90 seconds (driver-only quick run)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 7-01-* | 01 | 1 | CLEANUP-01 | grep oracle | `grep -rE 'CONTEXT\.md\|RESEARCH(\.md)?\|Plan 0[1-6]-\|\bD-0[1-9]\b\|Pitfall [0-9]\|Pattern [0-9]' crates/*/src/silent_payments/ crates/chia-sdk-bindings/src/silent_payments.rs crates/chia-sdk-driver/src/action_system/send_destination.rs examples/silent_payment.rs napi/__test__/silent_payments*.ts pyo3/tests/test_silent_payments.py wasm/__test__/silent_payments.spec.ts` returns 0 hits | ✅ | ⬜ pending |
| 7-01-* | 01 | 1 | CLEANUP-01 | regression | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` | ✅ | ⬜ pending |
| 7-02-* | 02 | 2 | CLEANUP-02 | grep/wc oracle | `test -f crates/chia-sdk-driver/src/actions/silent_payment_send.rs && [ $(wc -l < crates/chia-sdk-driver/src/actions/send.rs) -le 600 ]` | ✅ | ⬜ pending |
| 7-02-* | 02 | 2 | CLEANUP-02 | regression | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_tests` | ✅ | ⬜ pending |
| 7-02-* | 02 | 2 | CLEANUP-02 | smoke | `cargo build --release --workspace --all-features` and `cargo build --release --examples --all-features` | ✅ | ⬜ pending |
| 7-03-* | 03 | 3 | CLEANUP-03 | grep oracle | `grep -c 'pub fn finish_silent_payments\b' crates/chia-sdk-driver/src/action_system/spends.rs` returns 0 | ✅ | ⬜ pending |
| 7-03-* | 03 | 3 | CLEANUP-03 | grep oracle | `grep -c 'finish_silent_payments' crates/chia-sdk-bindings/src/action_system.rs` returns 0 | ✅ | ⬜ pending |
| 7-03-* | 03 | 3 | CLEANUP-03 | regression | `cd napi && pnpm test`; `cd pyo3 && pytest`; `cd wasm && pnpm test` | ✅ | ⬜ pending |
| 7-03-* | 03 | 3 | CLEANUP-03 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 round_trip_matches_derive_one_time_puzzle_hash` | ✅ | ⬜ pending |
| 7-04-* | 04 | 4 | CLEANUP-04 | grep/file oracle | `test ! -f crates/chia-sdk-driver/src/silent_payments/e2e.rs` | ✅ | ⬜ pending |
| 7-04-* | 04 | 4 | CLEANUP-04 | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` runs 3 tests, all pass | ✅ (target file created by CLEANUP-04) | ⬜ pending |
| 7-05-* | 05 | 5 | CLEANUP-06 | grep oracle | `grep -l 'nyquist_compliant: true' .planning/phases/*/*-VALIDATION.md \| wc -l` returns 8 | ✅ | ⬜ pending |
| 7-05-* | 05 | 5 | CLEANUP-06 | grep oracle | `grep -l 'wave_0_complete: true' .planning/phases/*/*-VALIDATION.md \| wc -l` returns 8 | ✅ | ⬜ pending |
| 7-05-* | 05 | 5 | CLEANUP-06 | code inspection | Visual review of `~/.claude/get-shit-done/bin/lib/phase.cjs` `cmdPhaseComplete` for the added VALIDATION flip block | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Task IDs are placeholder (`7-NN-*`); planner will replace with concrete `7-NN-MM` IDs.*

---

## Wave 0 Requirements

*None — the acceptance oracles are grep / wc / file-existence checks plus the existing workspace test suite. No new test framework, no new test files, no shared fixtures needed. Each CLEANUP-* requirement has a built-in oracle that's mechanical to check.*

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CLEANUP-01 Pass 2 comment-quality review | CLEANUP-01 | Surviving comment readability is a judgment call — not grep-checkable | Read each comment touched in Pass 1; confirm it reads as standalone technical content, cites CHIP/BIP, or describes the constraint directly. Delete any that no longer add value. |
| CLEANUP-06 Part B CLI behavior | CLEANUP-06 | gsd-tools has no test harness in this repo; user-home-dir patch | After patching `phase.cjs`, inspect the added flip block visually; optionally run `gsd-tools phase complete` on a test phase to confirm flag flip. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (N/A — no Wave 0 needed)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s for driver-only quick run
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
