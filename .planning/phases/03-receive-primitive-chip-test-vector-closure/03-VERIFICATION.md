---
phase: 03-receive-primitive-chip-test-vector-closure
verified: 2026-05-15T00:00:00Z
status: passed
score: 12/12 must-haves verified
re_verification: null
---

# Phase 3: Receive primitive & CHIP test-vector closure — Verification Report

**Phase Goal:** A wallet can take a `TweakData` blob from any indexer adapter and detect every payment to the wallet's scan/spend key pair (unlabeled and labeled), with bounded compute under adversarial input. All CHIP test vectors pass end-to-end.

**Verified:** 2026-05-15
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria from ROADMAP)

| # | Truth (SC) | Status | Evidence |
|---|------------|--------|----------|
| SC1 | CHIP test vectors pass as Rust unit tests: TV1 (unlabeled single-input), TV3 (labeled single-input), TV4 (multi-input aggregation) — intermediate values byte-for-byte match | ✓ VERIFIED | Tests `tv1_scan_detects_unlabeled_k0`, `tv3_scan_detects_labeled_k0`, `tv4_scan_detects_multi_input_aggregation` all pass with byte-exact pinned onetime_sk values (TV1=`3c399c61...0a89db37`, TV3=`58fc6195...b64852dc`, TV4=`6ccc3e13...e0f309399`). Pinned TV1 shared secret `d3ac1e8f...0ba2c6` verified in `tv1_shared_secret_matches`. |
| SC2 | Bespoke `k=1` test vector passes (catches `ser32(k)` LE bugs) | ✓ VERIFIED | `bespoke_k1_detection` passes — uses TV1's `tweak_point` + computes k=1 expected puzzle_hash in-test, scanner detects at k=1. |
| SC3 | Adversarial scalar test passes (first byte ≥ 0x80 reduces unsigned, NOT signed) | ✓ VERIFIED | `adversarial_ff32_scalar_reduces_unsigned` passes — three assertions: first byte < 0x80, determinism, direct-path equality through `ScalarField::from_bytes_unsigned`. |
| SC4 | Adversarial `TweakData` test (identity element returns `Vec::new()` + malformed pubkey rejected at deserialization) | ✓ VERIFIED | `identity_tweak_point_skipped` passes (CHIP §459 guard fires). `malformed_pubkey_caught_at_deserialization` passes (`PublicKey::from_bytes(&[0xff; 48]).is_err()`). |
| SC5 | DOS-guard test: 10,000 forged matches at one tweak point + `k_max=32` → ≤ 32 detections in bounded time | ✓ VERIFIED | `dos_guard_caps_at_k_max` passes — constructs `N_FORGED=10_000` outputs each crafted to match scanner's derived puzzle_hash; asserts `detections.len() <= 32`. |
| SC6 | Labeled k-termination test: scanner detects labeled output at k=1 when k=0 is unlabeled | ✓ VERIFIED | `labeled_k_termination_rule` passes — `TweakData` with TV1 unlabeled at k=0 + labeled-m=1 at k=1 returns BOTH detections (sorted: `[0].label=None`, `[1].label=Some(1)`). |

**Score:** 6/6 success criteria verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chia-sdk-driver/Cargo.toml` | `chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]` | ✓ VERIFIED | Line 23 exactly matches expected cascade. |
| `crates/chia-sdk-driver/src/lib.rs` | `#[cfg(feature = "chip-0057")] pub mod silent_payments;` + `pub use silent_payments::*;` | ✓ VERIFIED | Line 22: `pub mod silent_payments;` (promoted from `mod` to `pub mod` in Plan 03-05). Line 40: `pub use silent_payments::*;`. Both cfg-gated. |
| `crates/chia-sdk-driver/src/silent_payments/mod.rs` | Barrel with `mod {protocol, scanner, types}; pub use *;` in sorted order | ✓ VERIFIED | Lines 31-36 declare all three modules in sorted order (protocol < scanner < types) with `pub use *::*;` for each. |
| `crates/chia-sdk-driver/src/silent_payments/types.rs` | TweakData, OutputMeta, DetectedSpCoin with all pub fields, Clone+Debug derives | ✓ VERIFIED | All 3 types defined with all `pub` fields. `TweakData` and `DetectedSpCoin` derive `Clone, Debug`; `OutputMeta` derives `Clone, Copy, Debug` (Copy is bonus since all fields are Copy). Defensive test passes. |
| `crates/chia-sdk-driver/src/silent_payments/protocol.rs` | 5 protocol primitives + 2 tests | ✓ VERIFIED | All 5 functions present with correct signatures and `#[must_use]`. `compute_shared_secret_from_tweak` uses `chia_sha2::Sha256::{new,update,finalize}` (NOT `digest`). `derive_output_tweak` uses `k.to_be_bytes()`. `ScalarField::from_bytes_unsigned` for tweak; `from_bytes_raw` for spend_sk. `puzzle_hash_for_pk` uses `StandardArgs::curry_tree_hash`. |
| `crates/chia-sdk-driver/src/silent_payments/scanner.rs` | `scan_from_tweaks` + `K_MAX_DEFAULT=2400` + identity guard + bounded k loop + labeled branch + `SilentPaymentScan` trait + impl + 9 tests | ✓ VERIFIED | All present. `K_MAX_DEFAULT: usize = 2400` (line 40). `is_inf()` guard (line 88). `for k in 0..k_bound` (line 94) with `k_bound = u32::try_from(k_max).unwrap_or(u32::MAX)` (line 84). Labeled branch with `if !found && let Some(label_map) = labels` ordering. Trait + impl present. |
| `crates/chia-sdk-driver/src/driver_error.rs` | `#[cfg(feature = "chip-0057")] SilentPayment(#[from] SilentPaymentError)` variant | ✓ VERIFIED | Line 136 has the variant; cfg-gated on chip-0057. |
| `crates/chia-sdk-utils/src/silent_payments/labels.rs` | `generate_label` is `pub(crate)` | ✓ VERIFIED | Line 31: `pub(crate) fn generate_label(scan_sk: &SecretKey, m: u32) -> (ScalarField, PublicKey)`. |
| `crates/chia-sdk-utils/src/silent_payments/mod.rs` | `pub fn generate_label` public reach-through | ✓ VERIFIED | Line 53: `pub fn generate_label(...)`; delegates to `labels::generate_label(scan_sk, m)` on line 60. |
| `src/prelude.rs` | Second `#[cfg(feature = "chip-0057")]` block re-exporting 11 driver symbols | ✓ VERIFIED | Block present after Phase 2's utils block. All 11 symbols re-exported: DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks. |
| `.github/workflows/rust.yml` | `cargo build --release -p chia-sdk-driver -F chip-0057` CI line | ✓ VERIFIED | Line 55 present at 10-space indent, immediately after `chia-sdk-driver --all-features` line. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| Cargo.toml feature flag | chia-sdk-utils dep + feature | `chia-sdk-utils/chip-0057` + `dep:chia-sdk-utils` | ✓ WIRED | Build with `-F chip-0057` succeeds (verified). |
| `lib.rs` `pub mod silent_payments;` | `silent_payments/mod.rs` | cfg-gated module declaration | ✓ WIRED | `pub use chia_sdk_driver::silent_payments::*` in `src/prelude.rs` resolves under `--all-features` build. |
| Driver scanner | Driver protocol primitives | `use super::protocol::{...}` | ✓ WIRED | Tests pass; scanner uses all 5 protocol primitives. |
| Driver scanner | utils LabelRegistry + generate_label | `use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentKeys, generate_label};` | ✓ WIRED | Labeled tests pass (TV3, labeled k-termination). |
| `driver_error.rs::SilentPayment` | `chia_sdk_utils::silent_payments::SilentPaymentError` | `#[from]` propagation | ✓ WIRED | Build with `-F chip-0057` succeeds; variant resolved. |
| Prelude re-export | Driver silent_payments module | `pub use chia_sdk_driver::silent_payments::{...}` | ✓ WIRED | Workspace `--all-features` build succeeds. |
| CI build line | Driver `chip-0057` feature | Per-crate `-F chip-0057` invocation | ✓ WIRED | Same command `cargo build --release -p chia-sdk-driver -F chip-0057` succeeds locally. |
| Trait `SilentPaymentScan::scan` | `scan_from_tweaks` free fn | Delegation in impl block | ✓ WIRED | `silent_payment_keys_scan_method_matches_free_fn_tv1` passes (byte-equal). |

### Behavioral Spot-Checks (Build/Test Matrix)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Driver builds no-features | `cargo build --release -p chia-sdk-driver` | `Finished release profile` | ✓ PASS |
| Driver builds with chip-0057 | `cargo build --release -p chia-sdk-driver --features chip-0057` | `Finished release profile` | ✓ PASS |
| Driver builds all-features | `cargo build --release -p chia-sdk-driver --all-features` | `Finished release profile` | ✓ PASS |
| Workspace builds all-features | `cargo build --release --workspace --all-features` | `Finished release profile` | ✓ PASS |
| Phase 3 silent_payments tests | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments` | `test result: ok. 12 passed; 0 failed` | ✓ PASS |
| Phase 2 regression (utils) | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` | `test result: ok. 27 passed; 0 failed` | ✓ PASS |
| Full workspace test sweep (CI excludes) | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` | `2399 tests passed, 0 failed, 0 ignored` | ✓ PASS |
| Strict clippy on driver | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | clean | ✓ PASS |
| Strict clippy on utils | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` | clean | ✓ PASS |
| Workspace clippy (CI invocation) | `cargo clippy --workspace --all-features --all-targets` | clean except 2 pre-existing chia-sdk-daemon pedantic warnings (Phase 1 carry-forward; documented) | ✓ PASS |
| fmt | `cargo fmt --check` | clean (no output) | ✓ PASS |
| machete | `cargo machete` | "didn't find any unused dependencies" | ✓ PASS |
| Phase 1 grep ban: mod_by_group_order | `grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` | zero matches | ✓ PASS |
| Phase 1 grep ban: `^use sha2::` | `grep -rE '^use sha2::' crates/chia-sdk-driver/src/silent_payments/` | zero matches | ✓ PASS |
| Phase 1 grep ban: `Sha256::digest` | `grep -rE 'Sha256::digest' crates/chia-sdk-driver/src/silent_payments/` | zero matches | ✓ PASS |

### Test Inventory (12 named Phase 3 tests, all passing)

| Test | Module | Asserts |
|------|--------|---------|
| `malformed_pubkey_caught_at_deserialization` | types | `PublicKey::from_bytes(&[0xff; 48]).is_err()` |
| `tv1_shared_secret_matches` | protocol | TV1 shared secret = `d3ac1e8f...0ba2c6` (byte-exact) |
| `adversarial_ff32_scalar_reduces_unsigned` | protocol | first byte < 0x80, determinism, direct-path equality |
| `tv1_scan_detects_unlabeled_k0` | scanner | TV1 unlabeled k=0 onetime_sk = `3c399c61...0a89db37` |
| `tv4_scan_detects_multi_input_aggregation` | scanner | TV4 onetime_sk = `6ccc3e13...e0f309399` |
| `identity_tweak_point_skipped` | scanner | CHIP §459 guard: identity-element returns empty Vec |
| `tv3_scan_detects_labeled_k0` | scanner | TV3 labeled onetime_sk = `58fc6195...b64852dc`, label=Some(1) |
| `bespoke_k1_detection` | scanner | scanner finds k=1 detection for in-test-derived puzzle_hash |
| `labeled_k_termination_rule` | scanner | unlabeled-k=0 + labeled-k=1 both detected |
| `unlabeled_preferred_over_labeled_at_same_k` | scanner | exactly 1 detection (unlabeled wins) |
| `dos_guard_caps_at_k_max` | scanner | 10,000 forged + k_max=32 → `<= 32` detections |
| `silent_payment_keys_scan_method_matches_free_fn_tv1` | scanner | `SilentPaymentKeys::scan == scan_from_tweaks` byte-equal |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RECV-01 | 03-01 | `TweakData { tweak_points, outputs }` transport-agnostic input | ✓ SATISFIED | Type exists with no transport fields. Defensive test pins deserialization boundary. REQUIREMENTS.md marks `[x]`. |
| RECV-02 | 03-03 (+ 03-04) | `scan_from_tweaks` with BIP-352 k-iteration + labeled-termination rule | ✓ SATISFIED | Function exists with locked signature. Six scanner tests cover unlabeled, labeled, k-termination, identity guard. REQUIREMENTS.md marks `[x]`. |
| RECV-03 | 03-02 | `compute_shared_secret_from_tweak` ECDH primitive | ✓ SATISFIED | Function exists; `tv1_shared_secret_matches` pins byte-exact output. REQUIREMENTS.md marks `[x]`. |
| RECV-04 | 03-04 | Labeled detection branch + k-termination rule | ✓ SATISFIED | Labeled branch implemented in scanner.rs (lines 124-151). Tests `tv3_scan_detects_labeled_k0`, `labeled_k_termination_rule`, `unlabeled_preferred_over_labeled_at_same_k` pass. REQUIREMENTS.md marks `[x]`. |
| RECV-05 | 03-05 | `K_max` per-spend-group cap (DOS guard) | ✓ SATISFIED | `K_MAX_DEFAULT: usize = 2400` pub const; bounded `for k in 0..k_bound` loop. `dos_guard_caps_at_k_max` passes. REQUIREMENTS.md marks `[x]`. |
| CRYPTO-03 | 03-02 (+ 03-03 + 03-04) | All CHIP test vectors + bespoke k=1 + adversarial `[0xff;32]` | ✓ SATISFIED | TV1/TV3/TV4 scanner tests pass byte-exact; `bespoke_k1_detection` covers k=1; `adversarial_ff32_scalar_reduces_unsigned` covers signed-vs-unsigned reduction. REQUIREMENTS.md marks `[x]`. |

All 6 Phase 3 requirement IDs are marked `[x]` (complete) in REQUIREMENTS.md and each has corresponding implementation evidence verified above. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/chia-sdk-driver/src/silent_payments/scanner.rs` | 71 | `#[allow(clippy::similar_names)]` (function-scoped) | ℹ️ Info | Documented in PHASE-SUMMARY Key Decision #10 and in-file comment (lines 62-70). Function signature was hard-locked across plans 03-03 / 03-04 to use parameter names `scan_sk`/`spend_sk`/`spend_pk` (single-byte differences); clippy::similar_names fires below its similarity threshold but cannot be suppressed via rebinding without breaking the locked signature and Plan 03-04's append-point grep pattern. Function-scope only (NOT module-scope). Only `#[allow]` in any silent_payments tree. Counted by G16 in PHASE-SUMMARY phase gate as expected. |

No other `#[allow]` attributes found anywhere in driver, utils, or types silent_payments trees. No TODO/FIXME/placeholder comments. No stub returns. No hardcoded empty-data props. All grep bans (`mod_by_group_order`, `^use sha2::`, `Sha256::digest`) return zero matches in all three silent_payments trees.

### Human Verification Required

None — Phase 3 is library-internal Rust code with cryptographic primitive correctness verified via pinned byte-exact test vectors against the CHIP-0057 specification and the `~/silent-payments` reference implementation. Every observable behavior is exercised by an automated test.

### Gaps Summary

No gaps found. All 6 ROADMAP success criteria verified by passing named tests with byte-exact assertions against pinned test-vector bytes. All 6 phase requirements (RECV-01..05 + CRYPTO-03) closed and marked `[x]` in REQUIREMENTS.md. All 4 build permutations (no-features, `-F chip-0057`, `--all-features` per-crate, workspace `--all-features`) succeed. All 12 silent_payments tests pass under `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments`. Phase 2 regression check passes (27 Phase 2 silent_payments tests still green). Full workspace test sweep (2399 tests) passes. Strict scoped clippy `-D warnings` on driver + utils both clean. Workspace clippy clean except 2 pre-existing chia-sdk-daemon pedantic warnings (Phase 1 carry-forward, documented). fmt clean, machete clean. All three Phase 1 grep bans hold across the silent_payments tree. The single `#[allow(clippy::similar_names)]` on `scan_from_tweaks` is function-scoped, documented inline + in PHASE-SUMMARY Key Decision #10, and represents a known approved cross-plan signature-locking constraint.

The PHASE-SUMMARY's claims are corroborated by the codebase, not just stated. Phase 3 achieves its goal: a wallet can take a `TweakData` blob from any indexer adapter and detect every payment to the wallet's scan/spend key pair (unlabeled and labeled), with bounded compute under adversarial input, and all CHIP test vectors pass end-to-end with byte-exact pinned values.

---

*Verified: 2026-05-15*
*Verifier: Claude (gsd-verifier)*
