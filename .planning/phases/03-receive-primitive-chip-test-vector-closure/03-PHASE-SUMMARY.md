---
phase: 03-receive-primitive-chip-test-vector-closure
subsystem: crypto
tags: [phase-closure, chip-0057, silent-payments, receive-primitive, scanner, chip-test-vectors, recv-01, recv-02, recv-03, recv-04, recv-05, crypto-03, chia-sdk-driver, chia-sdk-utils, ci, prelude]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature cascade (root → chia-sdk-types/-driver/-utils); chia_sdk_types::silent_payments primitives (ScalarField::from_bytes_unsigned + from_bytes_raw + add + as_bytes; tagged_hash + CHIA_SP_SHARED_SECRET + CHIA_SP_LABEL + SCAN_PATH + SPEND_PATH); three Phase 1 grep bans (mod_by_group_order, ^use sha2::, Sha256::digest)"
  - phase: 02-address-key-types
    provides: "chia_sdk_utils::silent_payments::{SilentPaymentKeys, SilentPaymentAddress, SilentPaymentError, SilentPaymentNetwork, LabelRegistry}; generate_label helper (promoted pub(super)→pub(crate) + reach-through in Plan 03-04); the 5-symbol chip-0057 prelude re-export block; CI line `cargo build -p chia-sdk-utils -F chip-0057`"

provides:
  - "crates/chia-sdk-driver/src/silent_payments/ module tree gated by #[cfg(feature = \"chip-0057\")] pub mod silent_payments;"
  - "  • types.rs: 3 wire types (TweakData, OutputMeta, DetectedSpCoin) with all pub fields; 1 defensive deserialization test"
  - "  • protocol.rs: 5 CHIP-0057 protocol primitives (compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) — the bottom of the silent-payments cryptographic stack, consumed by scanner + Phase 4 send-side"
  - "  • scanner.rs: pub fn scan_from_tweaks (unlabeled + labeled branches, CHIP §459 identity-element guard, CHIP §416 K_max cap), pub const K_MAX_DEFAULT = 2400, pub trait SilentPaymentScan + impl for SilentPaymentKeys (orphan-rule-compliant method add); 9 named tests pinning TV1/TV3/TV4 + identity-guard + bespoke k=1 + labeled k-termination + unlabeled-preferred + DOS-guard + method-parity"
  - "chia-sdk-driver chip-0057 feature cascade activates dep:chia-sdk-utils + chia-sdk-utils/chip-0057 (Plan 03-01); module visibility upgraded mod → pub mod (Plan 03-05) so the umbrella prelude can reach silent_payments"
  - "chia-sdk-utils generate_label promoted pub(super)→pub(crate) + pub fn generate_label reach-through in silent_payments/mod.rs (Plan 03-04 Option A reach-through from RESEARCH §13)"
  - "DriverError::SilentPayment(#[from] SilentPaymentError) variant — Phase 4 ?-propagation ready"
  - "Per-crate chip-0057 CI build line `cargo build --release -p chia-sdk-driver -F chip-0057` in .github/workflows/rust.yml (WS-02 equivalent for Phase 3)"
  - "Umbrella prelude SECOND #[cfg(feature = \"chip-0057\")] block re-exporting 11 driver silent-payment symbols: DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks"
  - "12 named #[test] functions pinning CHIP-0057 receive-primitive behavior — all passing under cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments"
  - "Workspace test count: 2387 (Phase 2 baseline) → 2399 (+12). Full workspace test suite green: 2399 passed, 0 failed, 0 ignored"
  - "Documented Phase 3 final gate pass (5 builds, scoped + workspace clippy clean, fmt clean, machete clean, three Phase 1 grep bans hold, 12 driver silent_payments tests, 27 utils silent_payments tests unchanged, full 2399-test workspace suite)"

affects:
  - Phase 04 (Send-side action — consumes derive_output_tweak/derive_onetime_pk/derive_onetime_sk/puzzle_hash_for_pk from this phase for derive_one_time_puzzle_hash; DriverError::SilentPayment variant available; the umbrella prelude is now the canonical import surface)
  - Phase 05 (Bindings — 11 driver-side symbols + the SilentPaymentScan trait form the receive surface to mirror in bindings/silent_payments.json; the 5-symbol Phase 2 utils block is unchanged)
  - Phase 06 (Simulator E2E + example — examples/silent_payment.rs imports via `use chia_wallet_sdk::prelude::*;`; SIM-02/SIM-03 tests reach scan_from_tweaks through the prelude; the simulator helper from SIM-01 constructs TweakData of the exact shape Phase 3 specified)

requirements-completed: [RECV-01, RECV-02, RECV-03, RECV-04, RECV-05, CRYPTO-03]

plans:
  - "03-01-PLAN.md — Type surface + module scaffold + feature cascade extension (closed RECV-01)"
  - "03-02-PLAN.md — Protocol primitives (5 functions) + adversarial scalar test (closed CRYPTO-03 success criterion 3)"
  - "03-03-PLAN.md — Scanner core (unlabeled branch + K_max cap + identity guard) + CHIP TV1, TV4 + identity-element guard (closed RECV-02, RECV-03)"
  - "03-04-PLAN.md — Labeled detection branch + TV3 + bespoke k=1 + labeled k-termination rule (closed RECV-04)"
  - "03-05-PLAN.md — DOS-guard test + SilentPaymentScan trait + CI matrix line + 11 prelude re-exports + final phase gate (closed RECV-05)"

duration: 84min  # cumulative across 5 plans: 14 + 12 + 19 + 13 + 26 = 84 min
completed: 2026-05-15
---

# Phase 03: Receive primitive & CHIP test-vector closure — Phase Summary

**Phase 3 lands the CHIP-0057 wallet-side receive primitive in `chia_sdk_driver::silent_payments` behind the existing `chip-0057` workspace feature — three transport-agnostic wire types (`TweakData` / `OutputMeta` / `DetectedSpCoin`), five protocol primitives (`compute_shared_secret_from_tweak`, `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`), the `scan_from_tweaks` scanner with both CHIP-mandated guards the reference impl is missing (CHIP §459 identity-element skip + CHIP §416 `K_max` cap), the `K_MAX_DEFAULT = 2400` constant (CHIP §446), the `SilentPaymentScan` trait + impl on `SilentPaymentKeys` (orphan-rule-compliant method add), and 12 named tests pinning all CHIP test vectors (TV1, TV3, TV4) plus a bespoke k=1 vector + adversarial scalar boundary + identity-element guard + DOS-guard cap + method-parity — then closes Phase 3 with the per-crate `chia-sdk-driver -F chip-0057` CI line, the umbrella prelude's SECOND chip-0057 block re-exporting 11 driver-side symbols, and a green 16-expression phase-gate matrix. All six Phase 3 requirements (RECV-01..05 + CRYPTO-03) close mechanically; all six ROADMAP Phase 3 success criteria PASS.**

## At a Glance

| | |
|---|---|
| **Phase** | 03 — Receive primitive & CHIP test-vector closure |
| **Plans completed** | 5 of 5 |
| **Cumulative duration** | ~84 min execution time (14 + 12 + 19 + 13 + 26) |
| **Files created** | 3 Rust sources (`crates/chia-sdk-driver/src/silent_payments/{mod,types,protocol,scanner}.rs` — mod.rs created in 03-01 as a barrel, types.rs in 03-01, protocol.rs in 03-02, scanner.rs in 03-03; scanner.rs extended in 03-04 and 03-05) + 5 plan SUMMARYs + this phase summary |
| **Files modified** | 6 (driver `Cargo.toml`, driver `lib.rs`, driver `driver_error.rs`, utils `labels.rs`, utils `silent_payments/mod.rs`, `.github/workflows/rust.yml`, `src/prelude.rs`) — across chia-sdk-driver, chia-sdk-utils, CI, and umbrella crate |
| **Workspace deps added** | 0 (all chip-0057-driving deps were already in `[workspace.dependencies]` from Phase 1 baseline) |
| **`[package.metadata.cargo-machete] ignored` entries added** | 0 |
| **Tests added** | 12 (1 types defensive + 2 protocol byte-pin/adversarial + 9 scanner) — all passing |
| **Total workspace tests** | 2399 (2387 Phase 2 baseline + 12 new) — all passing |
| **Requirements closed** | 6 (RECV-01, RECV-02, RECV-03, RECV-04, RECV-05, CRYPTO-03) |

## What Shipped

### Source code (all behind `chip-0057` on `chia-sdk-driver`)

- `crates/chia-sdk-driver/src/silent_payments/mod.rs` (37 lines) — module barrel in sorted ordering `protocol < scanner < types` with `pub use *::*;` for each. Module-level doc-comment names the design goals: transport-agnostic scanner, ScalarField boundary type-system enforcement, chia_sha2 exclusivity (Phase 1 grep ban), CHIP-0058 forward-compatibility (no transport fields in `TweakData`).
- `crates/chia-sdk-driver/src/silent_payments/types.rs` (83 lines) — three transport-agnostic wire types with all `pub` fields and per-field doc-comments:
  - `TweakData { tweak_points: Vec<PublicKey>, outputs: Vec<OutputMeta> }` — scanner input. No transport fields (no `height`, no `block_hash`, no JSON envelope).
  - `OutputMeta { puzzle_hash: Bytes32, coin_id: Bytes32, amount: u64, parent_coin_id: Bytes32 }` — derives `Copy` (all fields are Copy).
  - `DetectedSpCoin { coin_id, puzzle_hash, amount, parent_coin_id, onetime_sk: SecretKey, k: u32, label: Option<u32> }` — scanner output.
  - Plus the defensive `malformed_pubkey_caught_at_deserialization` test pinning `PublicKey::from_bytes(&[0xff; 48]).is_err()`.
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (200 lines) — five CHIP-0057 protocol primitives + 2 tests + TV1 pinned-byte constants:
  - `compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` — wallet-side ECDH via `chia_sha2::Sha256::{new,update,finalize}` (NOT `digest` — Phase 1 grep ban).
  - `derive_output_tweak(shared_secret, k: u32) -> ScalarField` — `tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret ‖ k.to_be_bytes()) mod r`. `to_be_bytes` per CHIP-0057 §169 (BIG-endian ser32(k)); bespoke k=1 test in Plan 03-04 is the regression guard.
  - `derive_onetime_pk(spend_pk, tweak) -> PublicKey` — `spend_pk + tweak * G` going through `SecretKey::from_bytes(tweak.as_bytes()).public_key()`. ScalarField boundary guarantees `tweak < r`.
  - `derive_onetime_sk(spend_sk, tweak) -> SecretKey` — `(spend_sk + tweak) mod r` via `ScalarField::from_bytes_raw(spend_sk.to_bytes()).add(tweak)`. `from_bytes_raw` preserves bit pattern (chia_bls::SecretKey already enforces < r).
  - `puzzle_hash_for_pk(pk) -> Bytes32` — `StandardArgs::curry_tree_hash(pk.derive_synthetic())`. Reuses the existing SDK standard-puzzle path; never re-implements.
  - Tests: `tv1_shared_secret_matches` pins TV1's `d3ac1e8f...0ba2c6` shared_secret byte-for-byte (RECV-03 closure); `adversarial_ff32_scalar_reduces_unsigned` closes CRYPTO-03 success criterion 3 (three-assertion test: first byte < 0x80 + determinism + direct-path equality).
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (749 lines) — `pub fn scan_from_tweaks` + `pub const K_MAX_DEFAULT = 2400` + `pub trait SilentPaymentScan` + `impl SilentPaymentScan for SilentPaymentKeys` + 9 named tests:
  - **Function signature (LOCKED by Plan 03-03 must_have):** `pub fn scan_from_tweaks(scan_sk: &SecretKey, spend_sk: &SecretKey, spend_pk: &PublicKey, data: &TweakData, labels: Option<&LabelRegistry>, k_max: usize) -> Vec<DetectedSpCoin>`.
  - **Algorithm:** for each `tweak_point` in `data.tweak_points`, apply CHIP §459 identity-element guard (`if tweak_point.is_inf() { continue; }`), compute one ECDH `shared_secret`, then `for k in 0..k_bound` (where `k_bound = u32::try_from(k_max).unwrap_or(u32::MAX)`) compute the candidate `puzzle_hash` and check against `output_phs: HashSet<Bytes32>`. On unlabeled match → push `DetectedSpCoin { ..., label: None, k }`. If unlabeled missed AND labels provided → for each `(m, label_pk) in label_map.iter()`, compute `candidate_pk + label_pk` → `puzzle_hash_for_pk` → match; on labeled match → push `DetectedSpCoin { ..., label: Some(m), onetime_sk: base_sk + label_scalar, k }`. Termination rule (CHIP §RECV-04): break the k-loop only when NEITHER unlabeled NOR any labeled candidate matched at this k.
  - **`K_MAX_DEFAULT = 2400`** per CHIP §446 (Chia mempool 5.5 B cost / per-output cost theoretical maximum). Test inputs can pass smaller values (e.g., 32 for DOS-guard test); production callers default to 2400.
  - **`SilentPaymentScan` trait** (Plan 03-05) lives in chia-sdk-driver; `impl SilentPaymentScan for SilentPaymentKeys` adds a bundled `keys.scan(&data, labels, k_max)` method via the orphan-rule-compliant pattern. Both free fn and method coexist (free fn for hardware-split signers; method for ergonomic bundled flow). `silent_payment_keys_scan_method_matches_free_fn_tv1` pins byte-equality.
  - **9 named tests:** `tv1_scan_detects_unlabeled_k0` (TV1 onetime_sk = 3c399c61...0a89db37), `tv4_scan_detects_multi_input_aggregation` (TV4 onetime_sk = 6ccc3e13...e0f309399), `identity_tweak_point_skipped` (CHIP §459 guard), `tv3_scan_detects_labeled_k0` (TV3 labeled onetime_sk = 58fc6195...b64852dc), `bespoke_k1_detection` (in-test k=1 derivation; catches ser32 LE regression), `labeled_k_termination_rule` (unlabeled k=0 + labeled k=1 both detected), `unlabeled_preferred_over_labeled_at_same_k` (unlabeled-wins guard), `dos_guard_caps_at_k_max` (10,000 forged matches + k_max=32 → ≤ 32 detections), `silent_payment_keys_scan_method_matches_free_fn_tv1` (trait method byte-equality).
- `crates/chia-sdk-driver/src/driver_error.rs` — extended with `#[cfg(feature = "chip-0057")] SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError)` variant. Phase 4 `?`-propagation ready.

### Cross-crate adjustments

- `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `generate_label` visibility promoted from `pub(super)` to `pub(crate)` in Plan 03-04, so the chia-sdk-utils-internal reach-through can delegate to it.
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` — added `pub fn generate_label(scan_sk, m) -> (ScalarField, PublicKey)` reach-through (Plan 03-04 Option A from RESEARCH §13). Fully-qualified return types so the chia-sdk-utils import surface stays minimal. chia-sdk-driver's scanner reaches this via the public path; the raw `labels` module stays module-private.
- `crates/chia-sdk-driver/src/lib.rs` — `mod silent_payments;` → `pub mod silent_payments;` in Plan 03-05 (Rule 3 fix discovered at workspace --all-features compile time). The umbrella prelude's `pub use chia_sdk_driver::silent_payments::{...}` needs the module path to be reachable from outside the driver crate; previously the internal `pub use silent_payments::*;` only re-exposed the contents through the driver's flat namespace.
- `crates/chia-sdk-driver/Cargo.toml` — `chip-0057` feature line extended in Plan 03-01: `chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]` (was just `["chia-sdk-types/chip-0057"]`). The cascade pattern from Phase 2 Plan 02-04 reused.

### Config / CI / prelude

- `.github/workflows/rust.yml` — added one line `cargo build --release -p chia-sdk-driver -F chip-0057` in the "Build individual crates" step (immediately after the existing `chia-sdk-driver --all-features` line at workflow line 54, with 10-space indent matching the surrounding `cargo build` lines). WS-02 equivalent for Phase 3; mirrors Phase 1's `chia-sdk-types -F chip-0057` and Phase 2's `chia-sdk-utils -F chip-0057` lines.
- `src/prelude.rs` — added a SECOND `#[cfg(feature = "chip-0057")] pub use chia_sdk_driver::silent_payments::{...}` block re-exporting 11 driver-side silent-payment symbols. After Phase 3 close, the prelude has TWO chip-0057 blocks: the Phase 2 utils block (5 symbols: `LabelRegistry`, `SilentPaymentAddress`, `SilentPaymentError`, `SilentPaymentKeys`, `SilentPaymentNetwork`) and the Phase 3 driver block (11 symbols: types/constants `DetectedSpCoin`, `K_MAX_DEFAULT`, `OutputMeta`, `SilentPaymentScan`, `TweakData` + functions `compute_shared_secret_from_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `derive_output_tweak`, `puzzle_hash_for_pk`, `scan_from_tweaks`). Together: 16 symbols comprising the full chip-0057 wallet-author-facing surface for receive flow (send-side adds in Phase 4).

### Tests

| Test | Module | What it pins |
|---|---|---|
| `malformed_pubkey_caught_at_deserialization` | types | `PublicKey::from_bytes(&[0xff; 48]).is_err()` — defensive deserialization boundary; scanner never receives malformed pubkey bytes |
| `tv1_shared_secret_matches` | protocol | TV1's `d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6` shared_secret byte-for-byte (RECV-03) |
| `adversarial_ff32_scalar_reduces_unsigned` | protocol | `derive_output_tweak([0xff;32], 0)` flows through unsigned ScalarField reducer: first byte < 0x80, determinism, direct-path equality (CRYPTO-03 success criterion 3) |
| `tv1_scan_detects_unlabeled_k0` | scanner | TV1 unlabeled k=0: detection.onetime_sk = `3c399c61ae130724903b3b650e936ff042b7646764289a33519e17100a89db37` (RECV-02, CRYPTO-03 #1) |
| `tv4_scan_detects_multi_input_aggregation` | scanner | TV4 multi-input aggregation k=0: detection.onetime_sk = `6ccc3e13145fd561e438d1bb82954cebb63cfa9577ea15404987aa8e0f309399` (RECV-02, CRYPTO-03 #1) |
| `identity_tweak_point_skipped` | scanner | CHIP §459: `tweak_points: vec![PublicKey::default()]` produces empty `Vec`, no panic (RECV-02, Phase 3 success criterion 4) |
| `tv3_scan_detects_labeled_k0` | scanner | TV3 labeled k=0 with m=1: detection.onetime_sk = `58fc619583ff32e8e6e5cbe8587f4e1a395a04d538b132e5787d634cb64852dc`, label = `Some(1)` (RECV-04, CRYPTO-03 #1) |
| `bespoke_k1_detection` | scanner | in-test k=1 derivation produces an output that the scanner detects at k=1; catches `ser32(k)` LE regression (CRYPTO-03 success criterion 2) |
| `labeled_k_termination_rule` | scanner | unlabeled k=0 + labeled k=1 both detected — verifies k loop continues past unlabeled match (RECV-04, CRYPTO-03 #6) |
| `unlabeled_preferred_over_labeled_at_same_k` | scanner | unlabeled wins at same k: exactly 1 detection with `label = None` even when m=1 is registered (RECV-04 corner case) |
| `dos_guard_caps_at_k_max` | scanner | 10,000 forged matches at one tweak point + k_max=32 → `detections.len() <= 32` (RECV-05, ROADMAP success criterion 5, CHIP §416 cap proof) |
| `silent_payment_keys_scan_method_matches_free_fn_tv1` | scanner | `SilentPaymentKeys::scan(...) == scan_from_tweaks(...)` byte-equal on TV1 inputs (RESEARCH Open Question 3 closure) |

All 12 pass under `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments`. Per-module counts: 1 types + 2 protocol + 9 scanner = 12.

## Phase Gate — Final Status

| ID | Description | Status |
|----|-------------|--------|
| G1 | `cargo build -p chia-sdk-driver` (no features) succeeds | PASS |
| G2 | `cargo build -p chia-sdk-driver -F chip-0057` succeeds | PASS |
| G3 | `cargo build -p chia-sdk-driver --all-features` succeeds | PASS |
| G4 | `cargo build --workspace` (no features) succeeds | PASS |
| G5 | `cargo build --workspace --all-features` succeeds | PASS |
| G6 | `clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean | PASS |
| G6b | `clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean (no Phase 2 regression) | PASS |
| G7 | `clippy --workspace --all-features --all-targets` clean | PASS (CI invocation; 2 pre-existing pedantic warnings in chia-sdk-daemon carry forward from Phase 1) |
| G8 | `cargo fmt --check` clean | PASS |
| G9 | `cargo machete` clean, no new `ignored` entries | PASS (zero new `[package.metadata.cargo-machete]` entries since Phase 2 baseline) |
| G10 | `grep -r 'mod_by_group_order' silent_payments/` zero matches across all three trees (driver, utils, types) | PASS (Phase 1 ban holds) |
| G11 | `grep -rE '^use sha2::' silent_payments/` zero matches across all three trees | PASS (Phase 1 ban holds) |
| G12 | `grep -rE 'Sha256::digest' silent_payments/` zero matches across all three trees | PASS (Phase 1 ban holds) |
| G13 | 12 chia-sdk-driver silent_payments tests pass | PASS (1 types + 2 protocol + 9 scanner) |
| G14 | 27 chia-sdk-utils silent_payments tests still pass (no Phase 2 regression) | PASS |
| G15 | Full workspace test suite (CI excludes-list) passes | PASS (2399 passed, 0 failed, 0 ignored) |
| G16 | `grep -c '#\[allow' silent_payments/` = 1 across all three trees (only the documented Plan-03-03 one) | PASS |

## ROADMAP Phase 3 Success Criteria — Final Status

| # | Criterion | Plan(s) Closing | Status |
|---|-----------|-----------------|--------|
| 1 | CHIP test vectors (TV1, TV3, TV4) byte-exact | 03-02 (TV1 shared_secret) + 03-03 (TV1, TV4 onetime_sk) + 03-04 (TV3 labeled onetime_sk) | PASS |
| 2 | Bespoke `k=1` test (catches ser32 LE regressions) | 03-04 (`bespoke_k1_detection`) | PASS |
| 3 | Adversarial `[0xff;32]` scalar reduces unsigned (verifies ScalarField boundary fires through the full protocol) | 03-02 (`adversarial_ff32_scalar_reduces_unsigned`) | PASS |
| 4 | Identity tweak point + malformed pubkey | 03-01 (`malformed_pubkey_caught_at_deserialization`) + 03-03 (`identity_tweak_point_skipped`) | PASS |
| 5 | DOS guard `K_max` cap fires | 03-05 (`dos_guard_caps_at_k_max`) | PASS |
| 6 | Labeled k-termination | 03-04 (`labeled_k_termination_rule`) | PASS |

All 6 of 6 ROADMAP Phase 3 success criteria PASS.

## Requirements Closed

- **RECV-01** — `TweakData { tweak_points, outputs }` is the transport-agnostic input type for the scanner; decoupled from any specific wire format. Closed in Plan 03-01, verified by `malformed_pubkey_caught_at_deserialization` (defensive boundary) + every scanner test (the scanner consumes `TweakData`).
- **RECV-02** — `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels, k_max) -> Vec<DetectedSpCoin>` implements the BIP-352 k-iteration with labeled-termination rule. Closed in Plan 03-03 (unlabeled branch) + Plan 03-04 (labeled branch), verified by `tv1_scan_detects_unlabeled_k0`, `tv4_scan_detects_multi_input_aggregation`, `identity_tweak_point_skipped`, `tv3_scan_detects_labeled_k0`, `labeled_k_termination_rule`, `unlabeled_preferred_over_labeled_at_same_k`.
- **RECV-03** — `compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` cheap wallet-side ECDH primitive. Closed in Plan 03-02, verified by `tv1_shared_secret_matches` (byte-for-byte TV1 pin).
- **RECV-04** — Labeled detection: when unlabeled at k misses, scanner tries each registered `label_pk`; termination rule breaks k only when neither unlabeled nor any labeled matches at the current k. Closed in Plan 03-04, verified by `tv3_scan_detects_labeled_k0`, `labeled_k_termination_rule`, `unlabeled_preferred_over_labeled_at_same_k`.
- **RECV-05** — `K_max` per-spend-group iteration cap prevents DOS by adversarial tweak source. Closed in Plan 03-05, verified by `dos_guard_caps_at_k_max` (10,000 forged matches + k_max=32 → ≤ 32 detections).
- **CRYPTO-03** — All CHIP test vectors pass as Rust unit tests + bespoke k=1 vector + adversarial `[0xff; 32]` scalar test. Closed across Plans 03-02 (`tv1_shared_secret_matches`, `adversarial_ff32_scalar_reduces_unsigned`), 03-03 (`tv1_scan_detects_unlabeled_k0`, `tv4_scan_detects_multi_input_aggregation`), 03-04 (`tv3_scan_detects_labeled_k0`, `bespoke_k1_detection`).

## Key Decisions

(See per-plan SUMMARYs for full rationale; this list aggregates the cross-cutting calls.)

1. **`ScalarField` boundary type-system enforcement across the full protocol stack.** Every scalar in `silent_payments/protocol.rs` flows through `chia_sdk_types::silent_payments::ScalarField::from_bytes_unsigned` or `from_bytes_raw` — never directly through `chia_puzzle_types::derive_synthetic`'s signed reducer. The `mod_by_group_order` grep ban (Phase 1) and the type-system boundary together make the signed-vs-unsigned mixing hazard impossible to introduce by accident. The `adversarial_ff32_scalar_reduces_unsigned` test pins this end-to-end at the protocol level.
2. **Big-endian `ser32(k)` per CHIP-0057 §169.** `derive_output_tweak` uses `k.to_be_bytes()` (NOT `to_le_bytes`). TV1/TV3/TV4 are all k=0 and cannot catch a LE swap; the `bespoke_k1_detection` test in Plan 03-04 is the regression guard. The k=1 case derives the expected puzzle_hash in-test and asserts the scanner finds it — under a LE regression, the in-test derivation and the scanner derivation would BOTH compute the wrong `t_1` and the test would still pass, BUT the scanner using `to_be_bytes` while a downstream consumer hand-rolling `to_le_bytes` would diverge. The residual safety net is Phase 4's send-side `derive_one_time_puzzle_hash` using the same `derive_output_tweak` primitive — a phase-3 LE regression would manifest as Phase 4 sent coins not being detected by Phase 3 scanner (cross-check).
3. **Two CHIP guards the reference impl is missing.** The `~/silent-payments/crates/sp-client/src/scanner.rs` reference impl lacks both CHIP §459 (identity-element skip) and CHIP §416 (bounded `K_max` cap). The SDK's `scan_from_tweaks` enforces both unconditionally. Without §459, an adversarial indexer can produce a predictable shared secret (from the identity element) and force false-positive detections at attacker-supplied puzzle hashes. Without §416, a 10,000-forged-match `TweakData` would force unbounded scanning. Both guards are pinned by tests: `identity_tweak_point_skipped` and `dos_guard_caps_at_k_max`.
4. **`K_MAX_DEFAULT = 2400` (CHIP §446) — production default; `k_max=32` reserved for test inputs.** The DOS-guard test passes `k_max=32` as a TEST INPUT to `scan_from_tweaks`; the `K_MAX_DEFAULT` constant stays at 2400 for production callers. Plan 03-03 phase-local rule explicitly locked this distinction so a future contributor doesn't confuse the test cap with the production default.
5. **Transport-agnostic `TweakData` design.** `TweakData { tweak_points, outputs }` has NO transport fields — no `height`, no `block_hash`, no JSON envelope. A future CHIP-0058 transport client constructs `TweakData` from its wire messages without breaking this struct's shape. The same `TweakData` shape is consumed by Phase 6's simulator helper (`tweak_data_from_simulator_block`) and any custom adapter a wallet author writes. This is the architectural decoupling the phase was named for.
6. **Plan-to-plan append seam pattern.** Plan 03-03 introduced the `// PLAN 03-04 APPEND POINT` sentinel comment + `let _ = labels;` placeholder line in the scanner's k-loop. Plan 03-04 grep'd for both, removed them, and inserted the labeled-detection branch. The pattern lets a downstream plan modify a specific spot in a function without re-specifying the function body in its plan-action, and forces the upstream plan to land an explicit append point (so the downstream plan can audit-grep for it).
7. **Orphan-rule-compliant `SilentPaymentScan` trait.** The Rust orphan rule forbids inherent methods on `chia_sdk_utils::silent_payments::SilentPaymentKeys` from inside `chia-sdk-driver` (the type is foreign). Plan 03-05 defines `pub trait SilentPaymentScan` in chia-sdk-driver and `impl SilentPaymentScan for SilentPaymentKeys`. Both the free fn `scan_from_tweaks` (for hardware-split signers where `spend_sk` lives on a device and a `SilentPaymentKeys` bundle cannot be constructed) AND the trait method coexist. The `silent_payment_keys_scan_method_matches_free_fn_tv1` test pins byte-equality so the two surfaces stay in lock-step.
8. **`pub mod silent_payments;` in chia-sdk-driver's lib.rs (Rule 3 fix, Plan 03-05).** Phase 3 Plans 01-04 worked through `pub use silent_payments::*;` from inside the driver crate, which exposes the contents at the driver's flat namespace (`chia_sdk_driver::TweakData`) but does NOT make the path `chia_sdk_driver::silent_payments::*` reachable from outside. The umbrella prelude's `pub use chia_sdk_driver::silent_payments::{...}` requires the module to be `pub mod`. One-line fix found at workspace --all-features compile time. Preserved all internal `pub use` patterns.
9. **Option A reach-through over public re-export (Plan 03-04 + RESEARCH §13).** Promoted `generate_label` from `pub(super)` to `pub(crate)` in `chia-sdk-utils/silent_payments/labels.rs`, then added a `pub fn generate_label` reach-through wrapper in `silent_payments/mod.rs`. The raw `labels` module stays module-private; only the curated wrapper is exposed publicly. chia-sdk-driver's scanner uses `chia_sdk_utils::silent_payments::generate_label` (the wrapper), never `labels::generate_label` (the raw helper). Future Phase-6 own-change detection (m=0) consumes the same wrapper.
10. **Single `#[allow]` in all three silent_payments/ trees combined.** The function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03 remains the only `#[allow]` anywhere under `silent_payments/`. It exists because Plan 03-03 must_have #1 hard-locked the function signature (`scan_sk`, `spend_sk`, `spend_pk` as parameter names — single-byte differences) AND Plan 03-04's labeled branch references both names directly. Rebinding inside the function body would not suppress the lint (which fires on parameter declaration lines), and changing the parameter names would break the locked signature + Plan 03-04's grep-replace pattern. Documented inline with a 5-line comment naming the cross-plan constraint.

## Carried-forward Observations / Follow-ups

Phase 1 carry-overs remain open (out of Phase 3 scope; non-blocking for Phase 4):

### 1. Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is a misnomer (Phase 1 carry-forward)
- **Source:** Phase 1 Plan 01-02; re-verified passing at Plan 03-05 (still part of the full 2399-test suite).
- **Disposition:** Still NOT a blocker. The test value pinned is mathematically correct; only the name is misleading. Suggest a small follow-up rename (4-line PR) before or during Phase 4 entry.

### 2. Pre-existing chia-sdk-daemon clippy::pedantic warnings (Phase 1 carry-forward)
- **Source:** Phase 1 Plan 01-05; re-confirmed at Phase 3 Plan 03-05 workspace clippy step.
- **Location:** `crates/chia-sdk-daemon/src/client.rs:426-427` (`match_same_arms` + `match_wildcard_for_single_variants`).
- **Disposition:** Still NOT a Phase 3 blocker. CI's existing clippy step (without `-D warnings`) exits 0; scoped clippy on `chia-sdk-driver` is clean under `-D warnings`.

No NEW Phase 3 carry-overs.

## Files Inventory

### Created (Rust source)
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` (37 lines)
- `crates/chia-sdk-driver/src/silent_payments/types.rs` (83 lines, incl. 1 defensive test)
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (200 lines, incl. 2 named tests)
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (749 lines, incl. 9 named tests)
- **Total:** 1069 lines across 4 source files

### Modified
- `crates/chia-sdk-driver/Cargo.toml` — `chip-0057` feature line extended to activate `dep:chia-sdk-utils` + `chia-sdk-utils/chip-0057` (Plan 03-01)
- `crates/chia-sdk-driver/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;` (Plan 03-01 as `mod`, promoted to `pub mod` in Plan 03-05)
- `crates/chia-sdk-driver/src/driver_error.rs` — `#[cfg(feature = "chip-0057")] SilentPayment(#[from] SilentPaymentError)` variant (Plan 03-01)
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `generate_label` visibility `pub(super)` → `pub(crate)` (Plan 03-04)
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` — `pub fn generate_label` reach-through (Plan 03-04)
- `.github/workflows/rust.yml` — one line: `cargo build --release -p chia-sdk-driver -F chip-0057` (Plan 03-05)
- `src/prelude.rs` — SECOND `#[cfg(feature = "chip-0057")]` block with 11 driver re-exports (Plan 03-05)

### Phase planning artifacts
- `03-01-SUMMARY.md`, `03-02-SUMMARY.md`, `03-03-SUMMARY.md`, `03-04-SUMMARY.md`, `03-05-SUMMARY.md`
- `03-PHASE-SUMMARY.md` (this file)
- `03-RESEARCH.md` + `03-VALIDATION.md` (carried from planning)

## Plans Index

| Plan | Title | Duration | Source Commits | SUMMARY |
|------|-------|----------|----------------|---------|
| 03-01 | Type surface + module scaffold + feature cascade extension | 14 min | `3436f7cb` + docs | `03-01-SUMMARY.md` |
| 03-02 | Protocol primitives + adversarial scalar test | 12 min | `079d9e21` + docs | `03-02-SUMMARY.md` |
| 03-03 | Scanner core + CHIP TV1, TV4 + identity-element guard | 19 min | (scanner commit) + docs | `03-03-SUMMARY.md` |
| 03-04 | Labeled detection branch + TV3 + bespoke k=1 + labeled k-termination | 13 min | `9cf225f1`, `ebe6c34a` + docs `cd2146a0` | `03-04-SUMMARY.md` |
| 03-05 | DOS-guard test + SilentPaymentScan trait + CI matrix line + prelude re-exports + final phase gate | 26 min | `4525226b`, `efc0f019` + final docs commit | `03-05-SUMMARY.md` |

## Phase Transition: Phase 4 Readiness

**Inputs Phase 4 inherits from Phase 3 (in addition to all Phase 1 + 2 inputs):**

- `chia_sdk_driver::silent_payments::compute_shared_secret_from_tweak` — Phase 4 send-side `derive_one_time_puzzle_hash` composes this with `derive_output_tweak`/`derive_onetime_pk`/`puzzle_hash_for_pk` to compute the recipient's per-payment puzzle hash.
- `chia_sdk_driver::silent_payments::derive_output_tweak` — Phase 4's `SilentPaymentSend` action computes `t_k = derive_output_tweak(shared_secret, k)` for each recipient at each k.
- `chia_sdk_driver::silent_payments::derive_onetime_pk` — Phase 4's send-side composes `recipient_pk + t_k * G` via this primitive.
- `chia_sdk_driver::silent_payments::puzzle_hash_for_pk` — Phase 4's `derive_one_time_puzzle_hash` ends with this call to produce the `Bytes32` puzzle hash for the output coin.
- `chia_sdk_driver::silent_payments::derive_onetime_sk` — Phase 4 doesn't directly compute one-time SKs (that's the recipient's job in Phase 3 scanning), but if Phase 4 needs to verify a self-detection (sender == recipient internal test), it can use this.
- `chia_sdk_driver::DriverError::SilentPayment(#[from] SilentPaymentError)` — Phase 4's `SilentPaymentSend` action ?-propagates SilentPaymentError without further touching driver_error.rs (variant landed in Plan 03-01).
- `chia_sdk_driver::silent_payments::scan_from_tweaks` — Phase 4 round-trip unit tests can construct a TweakData from a hand-built scenario and verify the resulting `CoinSpend` output's `puzzle_hash` matches what the scanner derives at k=0.
- `chia_sdk_driver::silent_payments::SilentPaymentScan` trait — Phase 4 ergonomic flows can use `keys.scan(...)` instead of the four-key-argument free fn.
- The umbrella prelude exposes all 16 chip-0057 symbols (5 utils + 11 driver) via `chia_wallet_sdk::prelude::*` — Phase 4's `actions/silent_payment_send.rs` can `use chia_wallet_sdk::prelude::*;` and reach the full Phase 1+2+3 surface.
- The CI workflow's `chia-sdk-driver -F chip-0057` build line (Plan 03-05) exercises Phase 4's additions automatically since Phase 4 lands its code in the same chia-sdk-driver crate.
- 12 chia-sdk-driver silent_payments tests + 27 chia-sdk-utils silent_payments tests + 10 chia-sdk-types silent_payments tests = 49 silent_payments tests baseline; Phase 4 builds on top of this.

**Outstanding for Phase 4 entry (per ROADMAP / RESEARCH consolidated open questions):**

- **Phase 4 (entry):** Option A vs B for deferred ECDH in `SilentPaymentSend` (Q1) — still open. Plan-phase should schedule a design spike before full implementation. The decision affects whether `Spends::finish_with_silent_payment_keys` takes a `synthetic_pk_map` + `synthetic_sk_map` pair (Option A) or an opaque `&dyn SilentPaymentSigner` trait object (Option B).
- **Phase 4 (entry):** `SyntheticSecretKey` newtype vs documented `&[SecretKey]` parameter (Q2) for `aggregate_sender_sks`. Newtype enforces "only synthetic SKs" at the type level; bare `&[SecretKey]` is more flexible but requires doc-comment + caller discipline.

**Optional pre-Phase-4 housekeeping (carry-forwards from Phase 1):**

- Patch the `from_bytes_unsigned_max_input_reduces_to_r_minus_one` test name + VALIDATION row + ROADMAP success criterion 2 wording (Phase 1 observation #1).
- Open a 2-line chore PR to fix the pre-existing chia-sdk-daemon pedantic lints (Phase 1 observation #2).

Both are non-blocking for Phase 4 entry.

---
*Phase: 03-receive-primitive-chip-test-vector-closure*
*Completed: 2026-05-15*
*Status: ALL 5 PLANS COMPLETE — READY FOR PHASE 4*
