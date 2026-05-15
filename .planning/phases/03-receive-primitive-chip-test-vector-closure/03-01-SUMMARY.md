---
phase: 03-receive-primitive-chip-test-vector-closure
plan: 01
subsystem: infra
tags: [chip-0057, silent-payments, scaffold, feature-cascade, chia-sdk-driver, wire-types, driver-error, recv-01]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature cascade (root → chia-sdk-types/-driver/-utils); chia_sdk_types::silent_payments primitives; per-crate chip-0057 build hygiene"
  - phase: 02-address-key-types
    provides: "chia_sdk_utils::silent_payments::{SilentPaymentError, SilentPaymentKeys, SilentPaymentAddress, LabelRegistry, SilentPaymentNetwork}; chip-0057 feature cascade precedent on chia-sdk-utils (dep:chia-sdk-types + chia-sdk-types/chip-0057)"
provides:
  - "Extended chip-0057 feature cascade on chia-sdk-driver: chip-0057 now activates dep:chia-sdk-utils + chia-sdk-utils/chip-0057 so use chia_sdk_utils::silent_payments::* resolves under per-crate -F chip-0057 builds"
  - "crates/chia-sdk-driver/src/silent_payments/ module tree, gated at module level by #[cfg(feature = \"chip-0057\")] in lib.rs (first cfg-gated module in the driver crate; chip-0035/chip-0037 gate at type level)"
  - "silent_payments/mod.rs barrel — module-level doc-comment + pub mod types + pub use types::*"
  - "silent_payments/types.rs — three transport-agnostic wire types: TweakData { tweak_points: Vec<PublicKey>, outputs: Vec<OutputMeta> }, OutputMeta { puzzle_hash, coin_id, amount, parent_coin_id } (Copy), DetectedSpCoin { coin_id, puzzle_hash, amount, parent_coin_id, onetime_sk, k, label: Option<u32> }; plus the defensive test malformed_pubkey_caught_at_deserialization"
  - "DriverError gains #[cfg(feature = \"chip-0057\")] SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError) variant — Phase 4's SilentPaymentSend can ?-propagate silent-payment errors without further touching driver_error.rs"
  - "+1 silent_payments test on chia-sdk-driver (1 test now; 12 more land in 03-02..05)"
affects:
  - Plan 03-02 (protocol primitives — appends mod protocol; pub use protocol::*; to silent_payments/mod.rs and consumes TweakData/OutputMeta/DetectedSpCoin via super)
  - Plan 03-03 (scanner core — appends mod scanner; uses TweakData and DetectedSpCoin)
  - Plan 03-04 (labeled detection branch — consumes LabelRegistry via the now-resolvable chia_sdk_utils path)
  - Plan 03-05 (DOS guard + prelude re-export + final phase gate)
  - Phase 04 (send-side action — DriverError::SilentPayment variant ready for SilentPaymentSend ?-propagation)
  - Phase 05 (bindings — TweakData/OutputMeta/DetectedSpCoin shapes are bindings-clean by construction: Bytes32 + u64 + Option<u32> + Vec<PublicKey>)
  - Phase 06 (simulator E2E — TweakData has no transport fields so a simulator helper can construct it without breaking the API)

# Tech tracking
tech-stack:
  added: []  # No new workspace deps; only chip-0057 feature line extension
  patterns:
    - "First module-level cfg-gated module in chia-sdk-driver (#[cfg(feature = \"chip-0057\")] mod silent_payments;). Phase 1's chia-sdk-types established the same pattern; chip-0035/chip-0037 in the driver gate at type level inside primitives.rs/layers.rs instead."
    - "chip-0057 feature cascade activates BOTH the optional dep AND its chip-0057 feature (dep:chia-sdk-utils, chia-sdk-utils/chip-0057). Identical to Phase 2 Plan 02-04 precedent on chia-sdk-utils."
    - "Wire-type discipline: all fields pub, derive(Clone, Debug); derive Copy as soon as all fields are Copy (workspace missing_copy_implementations is a warn-lint and clippy -D warnings would otherwise reject)."
    - "Defensive-test-at-boundary pattern: malformed_pubkey_caught_at_deserialization documents that PublicKey::from_bytes rejects garbage so the scanner (which takes PublicKey, not bytes) never panics. This is the test-as-documentation pattern Phase 2 used for identity-pubkey rejection."

key-files:
  created:
    - "crates/chia-sdk-driver/src/silent_payments/mod.rs (33 lines — module barrel with doc-comment)"
    - "crates/chia-sdk-driver/src/silent_payments/types.rs (79 lines — three wire types + 1 defensive test)"
  modified:
    - "crates/chia-sdk-driver/Cargo.toml (chip-0057 feature line extended)"
    - "crates/chia-sdk-driver/src/lib.rs (+5 lines: cfg-gated mod silent_payments; + pub use silent_payments::*)"
    - "crates/chia-sdk-driver/src/driver_error.rs (+4 lines: cfg-gated SilentPayment variant)"

key-decisions:
  - "OutputMeta derives Copy. All fields are Copy (Bytes32, u64) and the workspace missing_copy_implementations warn-lint would trip clippy -D warnings. TweakData and DetectedSpCoin are NOT Copy-eligible (Vec / SecretKey)."
  - "Module gated at MODULE level via #[cfg(feature = \"chip-0057\")] mod silent_payments; in lib.rs — first such pattern in chia-sdk-driver. chip-0035/chip-0037 gate at type level inside primitives.rs/layers.rs but those touch many existing files; silent_payments is an isolated subtree so a top-level cfg gate is cleaner."
  - "Task 1 + Task 2 shipped as ONE atomic commit per the plan's done directive ('commit only after Task 2 is also done'). The chip-0057 build would have failed between tasks if split: Cargo.toml + lib.rs reference silent_payments/{mod.rs, types.rs} which don't exist until Task 2."
  - "DriverError::SilentPayment variant landed in Plan 03-01 even though Phase 3's scanner returns Vec<DetectedSpCoin> (no error path per 03-RESEARCH.md §3). Phase 4's SilentPaymentSend will be the primary consumer. Land now to avoid touching driver_error.rs twice."
  - "All three wire types pub-fielded. No new-style accessor methods because the bindings descriptor (Phase 5) needs to see fields directly via bindy-macro; struct-with-pub-fields is the SDK convention (mirrors chia_protocol::Coin)."

patterns-established:
  - "chia-sdk-driver chip-0057-gated module pattern: #[cfg(feature = \"chip-0057\")] mod silent_payments; + #[cfg(feature = \"chip-0057\")] pub use silent_payments::*; in lib.rs (alongside the other module declarations). Followed by chia-sdk-types' lib.rs from Phase 1."
  - "Feature cascade for optional dep + its own feature: chip-0057 = [\"chia-sdk-types/chip-0057\", \"dep:chia-sdk-utils\", \"chia-sdk-utils/chip-0057\"]. The dep:<name> activator AND the <dep>/<feature> activator must BOTH appear — activating only the dep without its feature fails E0432 on the dep's silent_payments::* exports."
  - "Wire-type binding granularity: Bytes32 + u64 + Option<u32> + Vec<PublicKey> shape. Bindy-macro already maps all four shapes — no new bindings.json type-group entries will be needed in Phase 5."

requirements-completed: [RECV-01]

# Metrics
duration: 14min
completed: 2026-05-15
---

# Phase 3 Plan 01: Type surface + module scaffold + feature cascade extension Summary

**Wave 0 scaffold for Phase 3's receive primitive — extends the chia-sdk-driver chip-0057 feature cascade to pull chia-sdk-utils, lands a module-level cfg-gated silent_payments tree (first such pattern in the driver crate), defines the three transport-agnostic wire types (TweakData / OutputMeta / DetectedSpCoin) with all pub fields, and adds the DriverError::SilentPayment variant once so Phase 4 doesn't have to touch driver_error.rs again.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-05-15T21:32:17Z (orchestrator marked status=executing in STATE.md)
- **Completed:** 2026-05-15T21:46:18Z
- **Tasks:** 2 (executed as one paired atomic commit per plan's done directive)
- **Files modified:** 3 (Cargo.toml, lib.rs, driver_error.rs); 2 created (silent_payments/{mod.rs, types.rs})

## Accomplishments

1. **chip-0057 feature cascade extension on chia-sdk-driver.** `chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]`. Per-crate `cargo build -p chia-sdk-driver -F chip-0057` now activates the optional `chia-sdk-utils` dep AND cascades its `chip-0057` feature so future imports of `chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentKeys, SilentPaymentError}` (Plans 03-03/03-04/03-05) resolve.
2. **Module-level cfg gating.** `crates/chia-sdk-driver/src/lib.rs` adds `#[cfg(feature = "chip-0057")] mod silent_payments;` + `#[cfg(feature = "chip-0057")] pub use silent_payments::*;` — the first cfg-gated module in the driver crate.
3. **silent_payments/mod.rs barrel** (33 lines) — module-level doc-comment naming the design goals (transport-agnostic scanner, ScalarField boundary, chia_sha2 only, CHIP-0058 forward-compat). Declares `pub mod types; pub use types::*;`. Plans 03-02 and 03-03 will append `mod protocol;` and `mod scanner;` lines respectively.
4. **silent_payments/types.rs** (79 lines) — three transport-agnostic structs with all `pub` fields and module-level + per-field doc-comments. `TweakData` (Vec<PublicKey> + Vec<OutputMeta>); `OutputMeta` (Bytes32 puzzle_hash + Bytes32 coin_id + u64 amount + Bytes32 parent_coin_id, Copy); `DetectedSpCoin` (coin_id + puzzle_hash + amount + parent_coin_id + onetime_sk: SecretKey + k: u32 + label: Option<u32>). Plus one defensive test pinning the deserialization boundary.
5. **DriverError extension.** `#[cfg(feature = "chip-0057")] SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError)` variant added once. Phase 4's `SilentPaymentSend` will `?`-propagate; Phase 3's scanner returns `Vec<DetectedSpCoin>` directly.
6. **+1 silent_payments test on chia-sdk-driver.** `silent_payments::types::tests::malformed_pubkey_caught_at_deserialization` asserts `PublicKey::from_bytes(&[0xff; 48])` returns `Err(_)`. Documents that malformed bytes are rejected at the type boundary (`PublicKey::from_bytes`) and never reach the scanner internals.

## Task Commits

Two tasks shipped as ONE atomic commit per the plan's `<done>` directive (Cargo.toml + lib.rs reference files that Task 2 creates; splitting would have produced an intermediate broken state):

1. **Task 1 + Task 2 (paired):** `feat(03-01): silent_payments module scaffold + wire types + DriverError variant` — `3436f7cb`

## Files Created/Modified

### Created
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` (33 lines) — Module barrel + doc-comment. `pub mod types; pub use types::*;`. Plans 03-02 and 03-03 will append `mod protocol;` and `mod scanner;` lines (sorted: protocol < scanner < types).
- `crates/chia-sdk-driver/src/silent_payments/types.rs` (79 lines) — The three wire-protocol types + 1 defensive deserialization test.

### Modified
- `crates/chia-sdk-driver/Cargo.toml` — line 23: `chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]` (was `["chia-sdk-types/chip-0057"]`). No new `[dependencies]` line — `chia-sdk-utils = { workspace = true, optional = true }` was already declared (line 45) for `offer-compression`.
- `crates/chia-sdk-driver/src/lib.rs` — +5 lines: `#[cfg(feature = "chip-0057")] mod silent_payments;` after `mod spend_with_conditions;` (line 21-22); `#[cfg(feature = "chip-0057")] pub use silent_payments::*;` after `pub use spend_with_conditions::*;` (line 37-38).
- `crates/chia-sdk-driver/src/driver_error.rs` — +4 lines: `#[cfg(feature = "chip-0057")] #[error("silent payment error: {0}")] SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError),` after the `MissingVaultCoinSpend` variant.

## Decisions Made

1. **OutputMeta derives Copy in addition to Clone+Debug.** The plan's `must_haves` says "All `#[derive(Clone, Debug)]`" but the workspace `missing_copy_implementations` warn-lint fires when a struct with all-Copy fields lacks the impl, and `cargo clippy -- -D warnings` turns that into an error. All four fields of `OutputMeta` are Copy (`Bytes32` is `BytesImpl<32>` which is Copy; `u64` is Copy), so adding `Copy` is the minimal-surprise fix. `TweakData` (`Vec<_>` fields) and `DetectedSpCoin` (`SecretKey` field) are not Copy-eligible. Logged as Rule 1 auto-fix (correctness — clippy gate failure).
2. **Task 1 + Task 2 ship as ONE atomic commit.** The plan explicitly notes "Tasks 1 + 2 ship as one atomic commit" in Task 1's acceptance-criteria block: Cargo.toml's feature line and lib.rs's `mod silent_payments;` reference files that Task 2 creates. Splitting would have produced an intermediate broken state where `cargo build -p chia-sdk-driver -F chip-0057` fails between commits.
3. **DriverError::SilentPayment variant landed now, even though Phase 3's scanner returns `Vec<DetectedSpCoin>`.** Phase 4's `SilentPaymentSend` will be the primary consumer. The plan called this out as "land it now to avoid touching driver_error.rs twice" and 03-RESEARCH.md §1 reinforces the rationale.
4. **`chia-sdk-utils` was already declared `optional = true` on the driver** (for `offer-compression`). The chip-0057 feature line activates the same optional dep — no new dep declaration needed. The dep:<name> + <dep>/<feature> cascade pattern is identical to Phase 2 Plan 02-04 on chia-sdk-utils.
5. **Module-level cfg gating in the driver crate is a new pattern.** chip-0035 and chip-0037 gate at type level inside `primitives.rs`/`layers.rs` (those features touch many existing files). silent_payments is an isolated subtree so a top-level `#[cfg(feature = "chip-0057")] mod silent_payments;` is cleaner and exactly mirrors Phase 1's pattern on `chia-sdk-types/src/lib.rs:3-4`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Correctness/Lint] OutputMeta gained Copy derive in addition to Clone+Debug**
- **Found during:** Task 2 (`cargo build --release -p chia-sdk-driver -F chip-0057`)
- **Issue:** The plan's `must_haves` truth #7 specifies `OutputMeta` with `#[derive(Clone, Debug)]`. After landing the struct, the build emitted `warning: type could implement Copy; consider adding impl Copy` due to the workspace `missing_copy_implementations` warn-lint. Workspace clippy with `-D warnings` (the local strict gate in the plan's acceptance criteria) would turn this warning into an error.
- **Fix:** Added `Copy` to the derive: `#[derive(Clone, Copy, Debug)]`. All fields are `Copy` (`chia_protocol::Bytes32 = BytesImpl<32>` is Copy; `u64` is Copy). Both `TweakData` (Vec fields) and `DetectedSpCoin` (`SecretKey`) remain not-Copy.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/types.rs:42`
- **Verification:** Rebuilt `cargo build --release -p chia-sdk-driver -F chip-0057` — clean. Rebuilt with `--all-features` — clean. `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0.
- **Committed in:** `3436f7cb` (paired Task 1+2 commit)

---

**Total deviations:** 1 auto-fixed (1 correctness/lint)
**Impact on plan:** Single-bit derive widening with no API or semantic impact. `OutputMeta` is still constructed by all callers as before; gaining Copy only makes the struct cheaper to pass by value (which is a small ergonomic win for Phase 5's bindings). Logged here for traceability.

## Issues Encountered

None during the planned work. The plan's design — paired Task 1+2 commit, exact insertion points named — landed first time. The `missing_copy_implementations` warning was the only friction and was a one-line fix.

## Phase 3 Plan 01 Gate — Final Status

| ID | Check | Status |
|----|-------|--------|
| G1 | `cargo build --release -p chia-sdk-driver` (no features) clean | PASS |
| G2 | `cargo build --release -p chia-sdk-driver -F chip-0057` clean | PASS |
| G3 | `cargo build --release -p chia-sdk-driver --all-features` clean | PASS |
| G4 | `cargo build --release --workspace --all-features` clean | PASS |
| G5 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean | PASS |
| G6 | `cargo fmt --all -- --files-with-diff --check` clean | PASS |
| G7 | `cargo machete` clean, no new ignored entries | PASS |
| G8 | `! grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G9 | `! grep -rE '^use sha2::' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G10 | `! grep -rE 'Sha256::digest' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G11 | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::types::tests::malformed_pubkey_caught_at_deserialization -- --exact` passes | PASS (1 passed) |
| G12 | Full CI test suite (workspace --all-features minus binding crates) passes | PASS (2388 passed, 0 failed, 0 ignored — was 2387 baseline, +1 = new test) |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Plan 03-02 (protocol primitives) unblocked.** The scaffold is in place:
- `silent_payments/mod.rs` is ready to gain `pub mod protocol; pub use protocol::*;` (insertion point: before `pub mod types;` per final sorted ordering `protocol < scanner < types`).
- `use chia_sdk_types::silent_payments::{CHIA_SP_SHARED_SECRET, ScalarField, tagged_hash}` will resolve under the cascade.
- `compute_shared_secret_from_tweak`, `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk` can land as `pub fn`s in `protocol.rs`.

**Plan 03-03 (scanner core) unblocked.** Same scaffold; needs `mod scanner;` line in mod.rs (between protocol and types in sorted order).

**Plan 03-04 (labeled detection) unblocked.** `use chia_sdk_utils::silent_payments::LabelRegistry` resolves through the cascade extension.

**Plan 03-05 (DOS guard + CI matrix + final gate).** Will add the `cargo build --release -p chia-sdk-driver -F chip-0057` line to `.github/workflows/rust.yml` and extend `src/prelude.rs:34-38` with the new driver re-exports.

**Phase 4 inheritance.** `DriverError::SilentPayment(#[from] SilentPaymentError)` is in place; `SilentPaymentSend` can `?`-propagate silent-payment errors without further driver_error.rs touches.

**Phase 5 inheritance.** The wire-type shapes (Bytes32 + u64 + Option<u32> + Vec<PublicKey>) are all expressible via existing bindings.json type-group mappings — no new top-level descriptor entries will be required.

**No blockers for Plans 03-02..05.**

## Self-Check: PASSED

Verified all claims:
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` exists (FOUND)
- `crates/chia-sdk-driver/src/silent_payments/types.rs` exists (FOUND)
- `git log --oneline --all | grep -q '3436f7cb'` (FOUND)
- `grep -E '^chip-0057 = ' crates/chia-sdk-driver/Cargo.toml` matches `chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]` (FOUND)
- `pub struct TweakData`, `pub struct OutputMeta`, `pub struct DetectedSpCoin` all present in `types.rs` (FOUND)
- `SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError)` present in `driver_error.rs` (FOUND)

---
*Phase: 03-receive-primitive-chip-test-vector-closure*
*Plan: 01*
*Completed: 2026-05-15*
