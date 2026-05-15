---
phase: 3
slug: receive-primitive-chip-test-vector-closure
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-15
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

> **Source of truth:** the `## Validation Architecture` section of `03-RESEARCH.md` enumerates the full per-test mapping (13 tests across RECV-01..05 + CRYPTO-03 + two defensive). This file is the execution-time gate: it lists Wave 0 deliverables, sampling rate, and sign-off. The Per-Task Verification Map is populated by `gsd-planner` from PLAN.md `<acceptance_criteria>` during planning.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in libtest); `rstest 0.22.0` available as a dev-dep for parametric coverage |
| **Config file** | None — `cargo test --release -p chia-sdk-driver --features chip-0057` is the canonical invocation |
| **Quick run command** | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Estimated runtime** | ~15 seconds (quick); ~3 minutes (full) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments` (~15 s)
- **After every plan wave:** Run `cargo test --release -p chia-sdk-driver --features chip-0057` (~30 s) to catch regressions in unrelated chip-0035/chip-0037 paths
- **Before `/gsd:verify-work`:** Full workspace suite green PLUS `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` PLUS `cargo fmt --check` PLUS `cargo machete` with zero new ignored entries on `chia-sdk-driver` PLUS the three Phase 1+2 grep bans (`! grep -r 'mod_by_group_order' silent_payments/`, `! grep -rE '^use sha2::' silent_payments/`, `! grep -rE 'Sha256::digest' silent_payments/`)
- **Max feedback latency:** 30 seconds (per-task)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| _populated from PLAN.md `<acceptance_criteria>` by gsd-planner_ | | | | | | | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

The full requirement → test map is in `03-RESEARCH.md` § Validation Architecture → Phase Requirements → Test Map. The 13 rows there are the source the planner converts into Task IDs.

---

## Wave 0 Requirements

Files that MUST exist (created during the first wave of Phase 3) before any verification command can run:

- [ ] `crates/chia-sdk-driver/src/silent_payments/mod.rs` — module barrel + `pub use` re-exports
- [ ] `crates/chia-sdk-driver/src/silent_payments/types.rs` — `TweakData`, `OutputMeta`, `DetectedSpCoin`, `SilentPaymentError`
- [ ] `crates/chia-sdk-driver/src/silent_payments/protocol.rs` — `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`, `compute_shared_secret_from_tweak`
- [ ] `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — `scan_from_tweaks`, `K_MAX_DEFAULT: usize = 2400`, all unit tests
- [ ] `crates/chia-sdk-driver/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- [ ] `crates/chia-sdk-driver/src/driver_error.rs` — `#[cfg(feature = "chip-0057")] SilentPayment(#[from] SilentPaymentError)` variant
- [ ] `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `generate_label` reach-through (Option A: `pub(super)` → `pub(crate)` plus a public reach-through in `mod.rs`, OR Option B: duplicate the 10-line helper in driver)
- [ ] `src/prelude.rs` — extend the `#[cfg(feature = "chip-0057")]` block with driver re-exports
- [ ] `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-driver -F chip-0057` CI build line

*Test framework: cargo + libtest are already present. No install command required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|

*None — all phase behaviors have automated verification via CHIP test vectors and adversarial-input unit tests.*

---

## Validation Sign-Off

- [ ] All tasks have `<acceptance_criteria>` with a `cargo test ... -- --exact` command or Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers all 8 MISSING file references above
- [ ] No watch-mode flags
- [ ] Feedback latency < 30 s
- [ ] `nyquist_compliant: true` set in frontmatter when planning completes

**Approval:** pending
