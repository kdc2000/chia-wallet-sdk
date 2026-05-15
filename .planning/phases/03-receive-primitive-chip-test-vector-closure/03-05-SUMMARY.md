---
phase: 03-receive-primitive-chip-test-vector-closure
plan: 05
subsystem: crypto
tags: [chip-0057, silent-payments, dos-guard, k-max, silent-payment-scan-trait, ci-matrix, prelude, recv-05, phase-closure, chia-sdk-driver]

# Dependency graph
requires:
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 01
    provides: "TweakData/OutputMeta/DetectedSpCoin wire types — the DOS-guard test constructs a TweakData with 10,000 forged-match OutputMeta entries"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 02
    provides: "compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, puzzle_hash_for_pk — used in-test to forge 10,000 matching puzzle hashes"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 03
    provides: "scan_from_tweaks core (CHIP §416 K_max cap path) — DOS-guard test exercises the cap by passing k_max=32 against 10,000 forged matches"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 04
    provides: "scan_from_tweaks labeled-detection branch (algorithm now feature-complete) — this plan adds the bindings/exposure layer + DOS-guard + final gate"
  - phase: 02-address-key-types
    provides: "SilentPaymentKeys (the type the `SilentPaymentScan` trait is implemented for) + the 5 chip-0057 utils re-exports already in prelude"
  - phase: 01-crypto-primitives-workspace-integration
    plan: 05
    provides: "Precedent for per-crate `-F chip-0057` CI build line + workspace clippy carry-forwards (chia-sdk-daemon pedantic warnings out of scope)"
provides:
  - "crates/chia-sdk-driver/src/silent_payments/scanner.rs — `pub trait SilentPaymentScan` + `impl SilentPaymentScan for SilentPaymentKeys` (orphan-rule-compliant: trait lives in the consumer crate, impl over the foreign type)"
  - "crates/chia-sdk-driver/src/silent_payments/scanner.rs — `dos_guard_caps_at_k_max` test: 10,000 forged-match outputs + k_max=32 → asserts detections.len() <= 32 (closes RECV-05 + CHIP §416 cap proof)"
  - "crates/chia-sdk-driver/src/silent_payments/scanner.rs — `silent_payment_keys_scan_method_matches_free_fn_tv1` test: byte-equal output between `scan_from_tweaks(...)` and `SilentPaymentKeys::scan(...)` on TV1 inputs (proves the two surfaces stay in lock-step)"
  - "crates/chia-sdk-driver/src/lib.rs — `mod silent_payments` promoted to `pub mod silent_payments` (Rule 3 fix — the umbrella prelude needs the module to be reachable from outside the driver crate)"
  - ".github/workflows/rust.yml — new line `cargo build --release -p chia-sdk-driver -F chip-0057` at 10-space indent, immediately after the no-features+all-features chia-sdk-driver pair (WS-02 equivalent for Phase 3, mirrors Phase 2 Plan 02-05 precedent for chia-sdk-utils)"
  - "src/prelude.rs — SECOND `#[cfg(feature = \"chip-0057\")]` block re-exporting 11 driver silent-payment symbols: DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks"
  - "Workspace test count: 2397 → 2399 (+2). chia-sdk-driver silent_payments tests: 10 → 12."
  - "RECV-05 closed; full Phase 3 RECV-01..05 + CRYPTO-03 closed."
affects:
  - "Phase 04 (send-side action) — consumes the same protocol primitives now reachable through the umbrella prelude (derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) for `derive_one_time_puzzle_hash`. The DriverError::SilentPayment variant from Plan 03-01 also stays available."
  - "Phase 05 (bindings) — the 11 prelude-exposed driver symbols form the receive-side surface that bindings/silent_payments.json must mirror via bindy-macro. The SilentPaymentScan trait will need a bindy treatment (method on SilentPaymentKeys' bound class)."
  - "Phase 06 (simulator E2E + example) — `examples/silent_payment.rs` can `use chia_wallet_sdk::prelude::*;` and reach the full receive primitive without per-crate imports. SIM-02/SIM-03 tests in `chia-sdk-test` similarly reach scan_from_tweaks via the prelude."

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Orphan-rule-compliant cross-crate method addition: `pub trait SilentPaymentScan` defined in `chia-sdk-driver` + `impl SilentPaymentScan for SilentPaymentKeys` where `SilentPaymentKeys` lives in `chia-sdk-utils`. The Rust orphan rule forbids `impl SomeForeignTrait for SomeForeignType` from inside a third crate, but `impl LocalTrait for ForeignType` is permitted as long as the trait is local. This is the idiomatic pattern when the consumer crate needs a bundled method on a foreign type without modifying the upstream crate. Documented in RESEARCH § Open Question 3."
    - "Method-vs-free-fn parity test pattern: `silent_payment_keys_scan_method_matches_free_fn_tv1` asserts byte-equality on all DetectedSpCoin fields (coin_id, puzzle_hash, k, onetime_sk.to_bytes(), label) between the free function and the trait method on the same input. Compile-time guarantee from the one-line delegation + runtime test pin — both surfaces stay in lock-step through future refactors."
    - "DOS-guard test pattern (CHIP §416): construct a TweakData where every k in [0, N) finds a forged match — the scanner's `if !found { break; }` never fires from a miss, so only the `k_max` cap can stop iteration. N must be >> k_max (we use N=10_000 vs k_max=32) so the cap test is unambiguous. The test takes ~3-4s wall-clock (10k forward derivations in setup + 32 scanner iterations); CI-safe."
    - "Const-at-top-of-function pattern to honor clippy::items_after_statements: when introducing `const N: usize = ...` literals inside a test body, declare ALL constants at the top of the function before any `let` bindings. The lint fires on `const` placed after the first `let`. Inline-fixed for `N_FORGED` and `K_MAX_TEST` in `dos_guard_caps_at_k_max`."
    - "Module visibility upgrade pattern: when adding `pub use crate_x::module_y::*` from a downstream crate (here, the umbrella prelude), `module_y` must be `pub mod` in `crate_x`'s lib.rs, not bare `mod`. The umbrella's re-export forced this change in `chia-sdk-driver/src/lib.rs`. Found at compile time during workspace --all-features build (Rule 3 inline fix)."

key-files:
  created: []
  modified:
    - "crates/chia-sdk-driver/src/silent_payments/scanner.rs (+148 lines, -3 lines: added `SilentPaymentKeys` import, trait + impl, two new tests; inline-fixed clippy doc_markdown + items_after_statements)"
    - "crates/chia-sdk-driver/src/lib.rs (1 line changed: `mod silent_payments;` → `pub mod silent_payments;`)"
    - ".github/workflows/rust.yml (+1 line: `cargo build --release -p chia-sdk-driver -F chip-0057` at 10-space indent)"
    - "src/prelude.rs (+6 lines net: blank separator + 1 cfg attribute + 1 opening `pub use chia_sdk_driver::silent_payments::{` + 2 indented name lines + 1 closing `};`)"

key-decisions:
  - "Trait-based method add over inherent-impl forking: the orphan rule prevents adding inherent methods on `chia_sdk_utils::silent_payments::SilentPaymentKeys` from inside `chia-sdk-driver` (the type lives in a third crate from the impl crate's perspective). Defining `pub trait SilentPaymentScan` in `chia-sdk-driver` and implementing it for `SilentPaymentKeys` lets cross-crate consumers write `use chia_wallet_sdk::prelude::SilentPaymentScan; keys.scan(&data, None, K_MAX_DEFAULT)` without us forking the upstream type. RESEARCH § Open Question 3 documented this as the recommended path; this plan executes it."
  - "Free fn + method coexistence: both `scan_from_tweaks(scan_sk, spend_sk, spend_pk, ...)` and `SilentPaymentKeys::scan(...)` survive. Hardware-split signers (where `spend_sk` lives on a device and only `scan_sk` is online) cannot construct a `SilentPaymentKeys` bundle so they need the raw-args entry point; wallet-author ergonomics benefits from the bundled method. The 11-symbol prelude re-export exposes both forms."
  - "10,000 forged matches vs k_max=32: N_FORGED is chosen large enough to make the cap test unambiguous (N must be >> k_max so any off-by-one in the cap implementation is caught), small enough to run in CI time (~3-4s wall-clock). The test passes `k_max=32` as a TEST INPUT — `K_MAX_DEFAULT = 2400` is unchanged."
  - "CI line placement after `chia-sdk-driver --all-features` (line 54), not after the chip-0057-pair (`chia-sdk-utils` lines 66-67): local grouping convention. Each crate's `cargo build` lines are co-located (the driver group at lines 53-55 now reads: no-features → all-features → chip-0057). Mirrors how Phase 1 placed `chia-sdk-types -F chip-0057` adjacent to `chia-sdk-types` (no-features), and Phase 2 placed `chia-sdk-utils -F chip-0057` adjacent to `chia-sdk-utils` (no-features)."
  - "Prelude re-export ordering: ALL types/constants first alphabetically (DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData), THEN all functions alphabetically (compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks). rustfmt-confirmed the shape: 5 type/const names on line 1, 4 functions on line 2, 2 functions on line 3. Plan-spec ordering matches what landed after rustfmt."
  - "Module-visibility upgrade in `chia-sdk-driver/src/lib.rs` (`mod silent_payments` → `pub mod silent_payments`) was a Rule 3 deviation found at workspace --all-features build time. Phase 3 plans 01-04 worked through `pub use silent_payments::*;` from inside chia-sdk-driver itself, which works for crate-internal use but does NOT expose the path `chia_sdk_driver::silent_payments::*` to downstream consumers like the umbrella crate. The fix is one-line; preserves all existing internal `pub use` patterns at the bottom of `lib.rs`."

patterns-established:
  - "Per-crate-F-chip-N CI build line: every chip-0057-bearing crate (chia-sdk-types since Phase 1, chia-sdk-utils since Phase 2, chia-sdk-driver since this plan) gets one `cargo build --release -p <crate> -F chip-0057` line in the 'Build individual crates' step. Catches per-crate feature-isolation regressions (the kind that Plan 02-04 fixed via `chia-sdk-types/chip-0057` cascade). Phase 5 (bindings) inherits the pattern when chia-sdk-bindings becomes chip-0057-bearing."
  - "Umbrella prelude as the canonical cross-crate consumer surface: by Phase 3 close, `chia_wallet_sdk::prelude::*` re-exports the full Phase 2 + Phase 3 silent-payment surface (16 symbols total). Future Phase-4 send-side additions extend the chip-0057 block in src/prelude.rs alongside the existing two; future bindings see the same prelude as the input surface to mirror in bindings/silent_payments.json."
  - "Dual-API surface: trait method (bundled, ergonomic) + free fn (raw-args, hardware-split-friendly) shipped together with a method-vs-free-fn parity test pinning byte-equality. Future Phase-4 send-side action and any other place where the SDK ergonomic surface differs from the canonical-raw surface should follow the same shape (one-line delegation + parity test)."

requirements-completed: [RECV-05]

# Metrics
duration: 26min
completed: 2026-05-15
---

# Phase 03 Plan 05: DOS-guard test + SilentPaymentScan trait + CI matrix line + 11 prelude re-exports + final phase gate Summary

**Wave 5 closes Phase 3: ships the `SilentPaymentScan` trait + `impl ... for SilentPaymentKeys` (orphan-rule-compliant cross-crate method add), the `dos_guard_caps_at_k_max` test (10,000 forged matches at one tweak point + `k_max=32` → asserts the cap fires), the `silent_payment_keys_scan_method_matches_free_fn_tv1` parity test, the per-crate `chia-sdk-driver -F chip-0057` CI build line (WS-02 equivalent for Phase 3), 11 silent-payment symbols re-exported through the umbrella crate's prelude behind `chip-0057`, and a green 16-gate final phase sweep. RECV-05 closes; all six Phase 3 requirements (RECV-01..05 + CRYPTO-03) closed; ROADMAP Phase 3 success criteria 1-6 all PASS.**

## Performance

- **Duration:** ~26 min
- **Started:** 2026-05-15T22:53:02Z
- **Completed:** 2026-05-15T23:19:14Z
- **Tasks:** 3 (Task 1: trait + impl + 2 tests in scanner.rs; Task 2: CI line + prelude + module-visibility fix; Task 3: 16-gate sweep + SUMMARY + PHASE-SUMMARY)
- **Files modified:** 4 (`scanner.rs`, `lib.rs`, `rust.yml`, `prelude.rs`)

## Accomplishments

1. **`pub trait SilentPaymentScan`** defined in `crates/chia-sdk-driver/src/silent_payments/scanner.rs` with the exact signature from the plan:
   ```rust
   pub trait SilentPaymentScan {
       fn scan(
           &self,
           tweak_data: &TweakData,
           labels: Option<&LabelRegistry>,
           k_max: usize,
       ) -> Vec<DetectedSpCoin>;
   }
   ```

2. **`impl SilentPaymentScan for SilentPaymentKeys`** — one-line wrapper that delegates to `scan_from_tweaks(self.scan_sk(), self.spend_sk(), self.spend_pk(), tweak_data, labels, k_max)`. Orphan-rule-compliant: trait lives in `chia-sdk-driver`, the foreign-type impl is permitted because the trait is local. RESEARCH § Open Question 3 documented this as the recommended pattern; this plan executes it.

3. **`dos_guard_caps_at_k_max` test** — closes RECV-05 + CHIP §416 cap proof:
   - For each k ∈ [0, 10_000), forge a `puzzle_hash` that the scanner WILL derive at that k using TV1's keys + tweak_point. Stuff all 10,000 forged puzzle hashes into the `OutputMeta` list.
   - Every k finds a "match", so the scanner's `if !found { break; }` never fires from a miss — only the `k_max` cap can stop iteration.
   - Pass `k_max = 32`. Assert `detections.len() <= 32`.
   - Runs in ~3-4s wall-clock (10k forward derivations in setup; 32 scanner iterations).

4. **`silent_payment_keys_scan_method_matches_free_fn_tv1` test** — proves the trait method and the free function produce byte-equal results on TV1 inputs. Asserts equality on all `DetectedSpCoin` fields (coin_id, puzzle_hash, k, onetime_sk.to_bytes(), label) — pins the parity-by-construction guarantee at runtime.

5. **New CI build line** in `.github/workflows/rust.yml`:
   ```yaml
             cargo build --release -p chia-sdk-driver -F chip-0057
   ```
   - 10-space leading indent (matches surrounding `cargo build` lines).
   - Inserted immediately after the existing `cargo build --release -p chia-sdk-driver --all-features` line at line 54.
   - WS-02 equivalent for Phase 3; mirrors Phase 1's `chia-sdk-types -F chip-0057` and Phase 2's `chia-sdk-utils -F chip-0057` lines.

6. **11 silent-payment symbols re-exported** in `src/prelude.rs` behind a new second `#[cfg(feature = "chip-0057")]` block, alphabetical types-then-functions:
   ```rust
   #[cfg(feature = "chip-0057")]
   pub use chia_sdk_driver::silent_payments::{
       DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData,
       compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,
       puzzle_hash_for_pk, scan_from_tweaks,
   };
   ```

7. **Rule 3 inline fix:** `crates/chia-sdk-driver/src/lib.rs` promoted `mod silent_payments;` → `pub mod silent_payments;` so the umbrella prelude can reach `chia_sdk_driver::silent_payments::*` from outside the driver crate. Caught at workspace --all-features compile time; one-line fix; preserves all existing internal `pub use silent_payments::*;` patterns in the driver crate.

8. **Workspace test count: 2397 → 2399 (+2 new tests).** chia-sdk-driver silent_payments tests: 10 → 12 (1 types + 2 protocol + 9 scanner).

9. **All Phase 1 grep bans still hold** under both `silent_payments/` trees combined: zero hits for `mod_by_group_order`, `^use sha2::`, `Sha256::digest`.

10. **Only one `#[allow]` under all three silent_payments/ trees combined** — the function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03 (already documented in 03-03-SUMMARY). No new `#[allow]` introduced by this plan.

11. **Full 16-gate phase sweep passes** — see "Phase 3 Plan 05 Gate" section below.

## Task Commits

1. **Task 1:** `feat(03-05): SilentPaymentScan trait + DOS-guard test + method-parity test` — `4525226b`
   - `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (+148 lines, -3 lines: import + trait + impl + 2 tests)
   - 9 scanner tests pass (was 7 — added 2 new in this commit).
   - Strict clippy + fmt clean on `chia-sdk-driver`. 4 inline lint fixes applied (2x clippy::doc_markdown, 2x clippy::items_after_statements).

2. **Task 2:** `feat(03-05): chia-sdk-driver -F chip-0057 CI line + 11 prelude re-exports` — `efc0f019`
   - `.github/workflows/rust.yml` (+1 line)
   - `src/prelude.rs` (+6 lines net)
   - `crates/chia-sdk-driver/src/lib.rs` (1 line: `mod` → `pub mod`)
   - Workspace --all-features build green; strict clippy clean; workspace clippy clean (chia-sdk-daemon carry-forwards stay out of scope); fmt clean.

3. **Task 3:** Verification-only — 16-gate sweep + SUMMARY + PHASE-SUMMARY. No code commit; final-doc commit pending.

## Files Created/Modified

### Created
None.

### Modified
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — 148 lines added, 3 removed: SilentPaymentKeys import added to the use-clause; trait + impl appended after `scan_from_tweaks` and before the test module; 2 new tests appended to the tests module (dos_guard_caps_at_k_max + silent_payment_keys_scan_method_matches_free_fn_tv1).
- `crates/chia-sdk-driver/src/lib.rs` — 1 line changed (`mod silent_payments;` → `pub mod silent_payments;`).
- `.github/workflows/rust.yml` — 1 line added at 10-space indent, inserted at workflow line 55 (after `chia-sdk-driver --all-features`, before `chia-sdk-client`).
- `src/prelude.rs` — 6 net lines added: 1 blank separator, 1 `#[cfg(feature = "chip-0057")]` attribute, 4 lines of the `pub use chia_sdk_driver::silent_payments::{ ... };` block.

## Decisions Made

1. **Orphan-rule-compliant trait pattern over alternative cross-crate-method paths.** `chia_sdk_utils::silent_payments::SilentPaymentKeys` lives in a separate crate from `chia-sdk-driver`. Rust's orphan rule permits `impl LocalTrait for ForeignType` (this plan) but forbids `impl ForeignTrait for ForeignType` from a third crate. The plan's `<action>` block specified the trait pattern; this plan executes it. Future Phase 5 bindings consumers see both `SilentPaymentScan` (as a trait the bindings emit a class-method shim for) and `scan_from_tweaks` (as a free function) — no duplication of code, just two surfaces.

2. **Both API surfaces ship.** The plan's must_have explicitly locks "Both forms (free fn + method) coexist (RESEARCH Open Question 3)." Free fn `scan_from_tweaks(scan_sk, spend_sk, spend_pk, ...)` stays available for hardware-split signers where `spend_sk` lives on a device and a `SilentPaymentKeys` bundle cannot be constructed. Trait method `SilentPaymentKeys::scan(...)` provides the bundled ergonomic flow for the common case. The parity test pins byte-equality.

3. **N_FORGED = 10_000 (vs k_max = 32) is intentionally large.** A naive cap-test with N == k_max + 1 would still pass even if the cap were off-by-1 (the loop terminates by exhausting outputs, not by hitting the cap). N >> k_max forces the cap to fire — the test is unambiguous about which control flow stopped the loop. CI cost: ~3-4 seconds total wall-clock (10k forward derivations in setup, 32 scanner iterations) — well within the workspace test budget.

4. **`pub mod silent_payments` in `chia-sdk-driver/src/lib.rs` (Rule 3 inline fix).** Phase 3 Plans 01-04 worked through `pub use silent_payments::*;` from inside chia-sdk-driver's `lib.rs`, which exposed the *contents* of the module via the driver's flat namespace (`chia_sdk_driver::TweakData`, etc.). The umbrella prelude's `pub use chia_sdk_driver::silent_payments::{...}` needs the *path* `chia_sdk_driver::silent_payments` to be reachable — and that requires `pub mod`. Caught at compile time during the workspace --all-features build. Fix is one-line + idempotent: existing internal `pub use` patterns still work; the new prelude path now also works.

5. **CI line placement: after `chia-sdk-driver --all-features` (line 54), not after the chip-0057 group (lines 65-67).** Convention from Phases 1+2: each crate's `cargo build` lines stay co-located. The chia-sdk-driver group at lines 53-55 reads top-to-bottom: no-features → all-features → chip-0057. Phase 1 placed `chia-sdk-types -F chip-0057` after `chia-sdk-types --all-features`; Phase 2 placed `chia-sdk-utils -F chip-0057` after `chia-sdk-utils` (no-features — utils didn't have an `--all-features` line yet); this plan places `chia-sdk-driver -F chip-0057` after `chia-sdk-driver --all-features`. Pattern: insert immediately after the crate's last existing cargo build line.

6. **Const-at-top-of-test-body convention.** The two `const` declarations in `dos_guard_caps_at_k_max` (`N_FORGED` and `K_MAX_TEST`) were inline-moved to the top of the function before any `let` binding, to honor `clippy::items_after_statements`. The plan's `<action>` Step B sample placed them after let bindings; the inline fix preserves the test logic exactly while satisfying the lint without an `#[allow]`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] `clippy::doc_markdown` × 2 in `dos_guard_caps_at_k_max` doc-comment**
- **Found during:** Task 1 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** The plan's `<action>` Step B sample contains "a TweakData with many forged matches" and "tweak_point" without backticks. Clippy flagged both as `doc_markdown` violations (TweakData is a type name; tweak_point is a CHIP-spec identifier).
- **Fix:** Added backticks: `TweakData` and `tweak_point`.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/scanner.rs:643,648`
- **Verification:** Strict clippy exits 0 post-fix. Documented in Task 1 commit message.
- **Committed in:** `4525226b`

**2. [Rule 1 - Lint] `clippy::items_after_statements` × 2 in `dos_guard_caps_at_k_max` body**
- **Found during:** Task 1 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** The plan's `<action>` Step B sample places `const N_FORGED: u32 = 10_000;` after 6 `let` bindings, and `const K_MAX_TEST: usize = 32;` after a `let outputs` + `let data` chain. Clippy `pedantic::items_after_statements` flagged both — declaring a `const` (a Rust *item*) after a statement makes the order confusing because items have scope from the start of the block.
- **Fix:** Moved both `const` declarations to the top of the test body before any `let` binding. Test logic unchanged.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/scanner.rs:653-654`
- **Verification:** Strict clippy exits 0 post-fix. Test passes.
- **Committed in:** `4525226b`

**3. [Rule 1 - Fmt] Rustfmt restructured the prelude re-export block**
- **Found during:** Task 2 (`cargo fmt --all`)
- **Issue:** The plan's `<action>` Step B sample formats the 11-name re-export across 3 lines:
  ```rust
  pub use chia_sdk_driver::silent_payments::{
      DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData,
      compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk,
      derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks,
  };
  ```
  Rustfmt's default wrap shifted some names between lines to optimize the 100-char limit (final shape: 5 types/consts + 4 functions on lines 2-3 of the block, 2 functions on line 4). Functionally identical.
- **Fix:** Let rustfmt apply its default. Final shape verified by `cargo fmt --all -- --check` exit 0 and by `grep` confirming all 11 names present in the prelude.
- **Files modified:** `src/prelude.rs:41-46`
- **Verification:** `cargo fmt --check` exits 0; all 11 symbols importable.
- **Committed in:** `efc0f019`

**4. [Rule 3 - Compile error] `chia-sdk-driver::silent_payments` module was private**
- **Found during:** Task 2 (`cargo build --release --workspace --all-features`)
- **Issue:** The umbrella crate's new prelude block `pub use chia_sdk_driver::silent_payments::{...}` failed at compile time: `error[E0603]: module 'silent_payments' is private`. Phase 3 Plans 01-04 had the module declared as `#[cfg(feature = "chip-0057")] mod silent_payments;` in chia-sdk-driver/src/lib.rs, with internal `pub use silent_payments::*;` re-exposing its contents at the driver crate's root. That works for crate-internal use but does NOT expose the `chia_sdk_driver::silent_payments::*` path to downstream consumers.
- **Fix:** Promoted `mod silent_payments;` → `pub mod silent_payments;` (line 22 of `crates/chia-sdk-driver/src/lib.rs`). One-line change. All existing internal `pub use silent_payments::*;` patterns continue to work; the new prelude path now also works.
- **Files modified:** `crates/chia-sdk-driver/src/lib.rs:22`
- **Verification:** `cargo build --release --workspace --all-features` exits 0 post-fix; all per-crate driver builds (no-features, -F chip-0057, --all-features) still exit 0.
- **Committed in:** `efc0f019` (Task 2)

---

**Total deviations:** 4 auto-fixed (3 lint/fmt; 1 compile-time module-visibility). No `#[allow]` attributes introduced; Plan 03-03's function-scoped `#[allow(clippy::similar_names)]` remains the only one anywhere under `silent_payments/`.

**Impact on plan:** All four deviations are intrinsic to the workspace's strict pedantic policy + cross-crate visibility rules + rustfmt's default behavior. No semantic change to the trait, impl, or test logic; no signature changes; no test-name changes. The exact API surface specified in the plan is preserved (11 prelude symbols + the trait + the impl + the 2 tests).

## Issues Encountered

None substantial. The four deviations above are normal pedantic/visibility maintenance for cross-crate adds. No blockers, no architectural decisions deferred, no work skipped.

## Phase 3 Plan 05 Gate — Final Status

| ID  | Check                                                                                                        | Status |
|-----|--------------------------------------------------------------------------------------------------------------|--------|
| G1  | `cargo build --release -p chia-sdk-driver` (no-features) clean                                                | PASS   |
| G2  | `cargo build --release -p chia-sdk-driver -F chip-0057` clean                                                 | PASS   |
| G3  | `cargo build --release -p chia-sdk-driver --all-features` clean                                               | PASS   |
| G4  | `cargo build --release --workspace` (no-features) clean                                                       | PASS   |
| G5  | `cargo build --release --workspace --all-features` clean                                                      | PASS   |
| G6  | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean                     | PASS   |
| G6b | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean (no Phase 2 regression) | PASS   |
| G7  | `cargo clippy --workspace --all-features --all-targets` (CI form) clean                                       | PASS (chia-sdk-daemon carry-forward warnings stay non-blocking; CI exits 0) |
| G8  | `cargo fmt --all -- --files-with-diff --check` clean                                                          | PASS   |
| G9  | `cargo machete` clean, no new ignored entries                                                                 | PASS   |
| G10 | `! grep -r 'mod_by_group_order' crates/chia-{sdk-driver,sdk-utils,sdk-types}/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G11 | `! grep -rE '^use sha2::' crates/chia-{sdk-driver,sdk-utils,sdk-types}/src/silent_payments/` (Phase 1 ban)    | PASS (zero matches) |
| G12 | `! grep -rE 'Sha256::digest' crates/chia-{sdk-driver,sdk-utils,sdk-types}/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G13 | `cargo test ... silent_payments` on chia-sdk-driver: 12 tests pass (1 types + 2 protocol + 9 scanner)         | PASS (12 passed) |
| G14 | `cargo test ... silent_payments` on chia-sdk-utils: 27 tests still pass (no Phase 2 regression)               | PASS (27 passed) |
| G15 | Full workspace test suite passes (2397 → 2399, +2 new tests)                                                  | PASS (2399 passed, 0 failed, 0 ignored) |
| G16 | `grep -c '#\[allow' crates/chia-sdk-driver/src/silent_payments/` = 1 (only the Plan-03-03 documented one)     | PASS (1 hit, justified) |

### Additional structural checks

| ID  | Check                                                                                                        | Status |
|-----|--------------------------------------------------------------------------------------------------------------|--------|
| S1  | `grep -q 'pub trait SilentPaymentScan' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                  | PASS |
| S2  | `grep -q 'impl SilentPaymentScan for SilentPaymentKeys' crates/chia-sdk-driver/src/silent_payments/scanner.rs` | PASS |
| S3  | `grep -E '^          cargo build --release -p chia-sdk-driver -F chip-0057' .github/workflows/rust.yml`        | PASS (10-space indent matches) |
| S4  | All 11 driver re-exports present in `src/prelude.rs` (DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks) | PASS |
| S5  | `grep -q 'pub mod silent_payments;' crates/chia-sdk-driver/src/lib.rs`                                         | PASS |
| S6  | `cargo build --release -p chia-wallet-sdk --all-features` clean (umbrella crate sees the new prelude re-exports) | PASS |
| S7  | `cargo test ... silent_payments::scanner::tests::dos_guard_caps_at_k_max -- --exact` passes                    | PASS (1 passed) |
| S8  | `cargo test ... silent_payments::scanner::tests::silent_payment_keys_scan_method_matches_free_fn_tv1 -- --exact` passes | PASS (1 passed) |

## ROADMAP Phase 3 Success Criteria — Final Status

| # | Criterion | Plan(s) Closing | Status |
|---|-----------|-----------------|--------|
| 1 | CHIP test vectors (TV1, TV3, TV4) byte-exact | 03-03 (TV1, TV4) + 03-04 (TV3) | PASS |
| 2 | Bespoke `k=1` test (catches ser32 LE regressions) | 03-04 `bespoke_k1_detection` | PASS |
| 3 | Adversarial `[0xff;32]` scalar reduces unsigned | 03-02 `adversarial_ff32_scalar_reduces_unsigned` | PASS |
| 4 | Identity tweak point + malformed pubkey | 03-03 `identity_tweak_point_skipped` + 03-01 `malformed_pubkey_caught_at_deserialization` | PASS |
| 5 | DOS guard `K_max` cap fires | 03-05 `dos_guard_caps_at_k_max` (this plan) | PASS |
| 6 | Labeled k-termination | 03-04 `labeled_k_termination_rule` | PASS |

All 6 of 6 ROADMAP Phase 3 success criteria PASS.

## Requirements Closed (cumulative across all 5 Phase 3 plans)

| REQ-ID    | Closing Plan(s)      | Verified-by tests                                                        |
|-----------|----------------------|--------------------------------------------------------------------------|
| RECV-01   | 03-01                | malformed_pubkey_caught_at_deserialization                              |
| RECV-02   | 03-03 + 03-04        | tv1_scan_detects_unlabeled_k0, tv4_scan_detects_multi_input_aggregation, identity_tweak_point_skipped, tv3_scan_detects_labeled_k0, labeled_k_termination_rule, unlabeled_preferred_over_labeled_at_same_k |
| RECV-03   | 03-02                | tv1_shared_secret_matches                                                |
| RECV-04   | 03-04                | tv3_scan_detects_labeled_k0, labeled_k_termination_rule, unlabeled_preferred_over_labeled_at_same_k |
| RECV-05   | 03-05 (this plan)    | dos_guard_caps_at_k_max                                                  |
| CRYPTO-03 | 03-02 + 03-03 + 03-04 | tv1_shared_secret_matches, adversarial_ff32_scalar_reduces_unsigned, tv1_scan_detects_unlabeled_k0, tv4_scan_detects_multi_input_aggregation, tv3_scan_detects_labeled_k0, bespoke_k1_detection |

All 6 Phase 3 requirements closed (RECV-01..05 + CRYPTO-03).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Phase 4 (send-side action) is unblocked.** Phase 4's `derive_one_time_puzzle_hash` consumes the same `derive_output_tweak → derive_onetime_pk → puzzle_hash_for_pk` chain that Phase 3 now exposes through the umbrella prelude. The 11 driver re-exports give Phase 4 access to the full receive primitive without per-crate imports; the prelude is the canonical cross-phase consumer surface.

**Open Phase-4 entry questions (carried from RESEARCH consolidated open questions):**
- **Q1: Option A vs B for deferred ECDH in `SilentPaymentSend`** — still open; needs a Phase-4 proof-of-concept early in planning.
- **Q2: `SyntheticSecretKey` newtype vs documented `&[SecretKey]` parameter** for `aggregate_sender_sks` — still open.

**Phase 5 (bindings)** sees the 11 prelude symbols + the trait as the receive-side surface to mirror in `bindings/silent_payments.json`. The `SilentPaymentScan` trait needs a bindy treatment (the bindy-macro can emit a class-method shim that delegates to the trait's `scan` method on the underlying `SilentPaymentKeys`).

**Phase 6 (simulator E2E + example)** will `use chia_wallet_sdk::prelude::*;` in `examples/silent_payment.rs` and reach the full receive primitive without per-crate imports. SIM-02/SIM-03 tests in `chia-sdk-test` similarly reach `scan_from_tweaks` and the protocol primitives through the prelude.

**No blockers for Phase 4 or downstream phases.**

## Self-Check: PASSED

Verified all claims:

**Files modified:**
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (FOUND, 749 lines)
- `crates/chia-sdk-driver/src/lib.rs` (FOUND, `pub mod silent_payments;` present)
- `.github/workflows/rust.yml` (FOUND, new CI line at 10-space indent verified)
- `src/prelude.rs` (FOUND, second chip-0057 block with 11 driver re-exports present)

**Commits exist:**
- `4525226b` (Task 1: trait + impl + 2 tests): FOUND via `git log --oneline | grep 4525226b`
- `efc0f019` (Task 2: CI line + prelude + module visibility): FOUND

**Code claims verified via grep:**
- `pub trait SilentPaymentScan` in scanner.rs: FOUND
- `impl SilentPaymentScan for SilentPaymentKeys` in scanner.rs: FOUND
- `pub mod silent_payments;` in chia-sdk-driver/src/lib.rs: FOUND
- All 11 prelude symbols present in src/prelude.rs: FOUND (verified by individual greps)
- `^          cargo build --release -p chia-sdk-driver -F chip-0057` in rust.yml: FOUND (10-space indent confirmed)

**Phase 1 grep bans verified:**
- `mod_by_group_order` under all three silent_payments/ trees: zero hits
- `^use sha2::` under all three silent_payments/ trees: zero hits
- `Sha256::digest` under all three silent_payments/ trees: zero hits

**Allow count:**
- Only one `#[allow]` under all three silent_payments/ trees combined: the function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03. NO NEW `#[allow]` introduced.

**Tests passing under `-- --exact`:**
- `dos_guard_caps_at_k_max`: PASS (1 passed)
- `silent_payment_keys_scan_method_matches_free_fn_tv1`: PASS (1 passed)
- All 9 scanner tests pass under `cargo test ... silent_payments::scanner`
- All 12 silent_payments tests pass under `cargo test ... silent_payments` on chia-sdk-driver
- All 27 silent_payments tests on chia-sdk-utils still pass (no Phase 2 regression)

**Strict gates:**
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo clippy --workspace --all-features --all-targets` exits 0 (CI form; chia-sdk-daemon carry-forwards non-blocking)
- `cargo fmt --all -- --check` exits 0
- `cargo machete` clean

**Workspace test count:** 2397 → 2399 (+2) confirmed via sum-of-test-result lines.

**Umbrella crate verification:** `cargo build --release -p chia-wallet-sdk --all-features` exits 0, confirming the new prelude re-exports compile.

---
*Phase: 03-receive-primitive-chip-test-vector-closure*
*Plan: 05*
*Completed: 2026-05-15*
