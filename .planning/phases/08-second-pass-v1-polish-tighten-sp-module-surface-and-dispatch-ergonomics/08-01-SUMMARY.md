---
phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
plan: 01
subsystem: refactoring

tags: [silent-payments, chip-0057, re-exports, module-surface, refactor]

# Dependency graph
requires:
  - phase: 07-code-review-cleanup
    provides: "Stable post-cleanup silent_payments/mod.rs body to refactor"
provides:
  - "Named pub-use re-exports for aggregate/input_hash/one_time/protocol/scanner/types public modules in silent_payments/mod.rs"
  - "Intermediate-shape mod.rs body (4 mod declarations + 4 named pub-use blocks) ready for POLISH-02 (plan 08-04) to fold into single protocol mod"
  - "Named-list authority Plan 08-04 (Wave 2 POLISH-02 fold) will consult to collapse the 4 named-export blocks into a single `pub use protocol::{...}` block after the 3 file deletions"
affects: [08-04, polish-02, sp-module-surface]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Named pub-use re-exports over wildcards on public modules (matches src/prelude.rs:35-45 + crates/chia-sdk-utils/src/silent_payments/mod.rs workspace style)"
    - "Crate-private wildcard `pub(crate) use foo::*;` permitted when module contains only `pub(crate)` items"

key-files:
  created: []
  modified:
    - crates/chia-sdk-driver/src/silent_payments/mod.rs

key-decisions:
  - "Intermediate shape (4 separate mod declarations + 4 named pub-use blocks) ships in this plan; POLISH-02 (08-04) folds aggregate/input_hash/one_time into protocol.rs and collapses to single `pub use protocol::{...}`"
  - "Module rustdoc (lines 1-30) left untouched per CONTEXT.md Claude's Discretion item 3 — current rustdoc references primitives by symbol, not by sub-file, so it remains correct post-edit"
  - "Symbol ordering within `pub use protocol::{...}` left as rustfmt-canonical (rustfmt re-wraps the multi-line block to a single line when symbols fit under the 100-col width limit)"

patterns-established:
  - "Pure-refactor wave-1 plan: single-file edit + per-crate build + workspace --all-features build + scoped clippy -D warnings + cargo fmt --check as the surgical-edit gate matrix"
  - "Acceptance-grep oracle pattern: `grep -cE '^pub use [a-z_]+::\\*;' file` returns 0 captures the named-only contract while permitting `pub(crate)` wildcards"

requirements-completed: [POLISH-01]

# Metrics
duration: 5min
completed: 2026-05-20
---

# Phase 8 Plan 01: Tighten silent_payments/mod.rs re-exports (POLISH-01)

**Replaced 6 `pub use foo::*;` wildcards in silent_payments/mod.rs with explicit named re-exports for 14 symbols across 6 sub-modules; `pub(crate) use send_keys::*;` preserved.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-05-20T19:46:19Z
- **Completed:** 2026-05-20T19:51:29Z
- **Tasks:** 2 (Task 1 verification-only, Task 2 surgical edit + commit)
- **Files modified:** 1

## Accomplishments

- Replaced 5 public-module wildcards (`pub use {aggregate, input_hash, one_time, protocol, scanner, types}::*;`) with explicit named re-exports for 14 symbols (1 + 1 + 1 + 5 + 3 + 3 across the modules); `pub(crate) use send_keys::*;` preserved verbatim per D-01 (crate-private wildcard explicitly permitted).
- POLISH-01 primary acceptance grep `^pub use [a-z_]+::\*;` returns 0 hits on `crates/chia-sdk-driver/src/silent_payments/mod.rs`.
- All 13 prelude SP names at `src/prelude.rs:41-45` (`DetectedSpCoin`, `K_MAX_DEFAULT`, `OutputMeta`, `SilentPaymentScan`, `TweakData`, `compute_input_hash`, `compute_shared_secret_from_tweak`, `derive_one_time_puzzle_hash`, `derive_onetime_pk`, `derive_onetime_sk`, `derive_output_tweak`, `puzzle_hash_for_pk`, `scan_from_tweaks`) remain reachable through the new named lists.
- `aggregate_sender_sks` (consumed by binding facade at `crates/chia-sdk-bindings/src/silent_payments.rs:432` via the documented top-level `chia_sdk_driver::aggregate_sender_sks` path) remains reachable through `silent_payments/mod.rs` and propagates through `chia-sdk-driver/src/lib.rs:40`'s `pub use silent_payments::*;` to driver top level.
- Workspace `--all-features` build clean (includes napi, pyo3, wasm); scoped clippy under `chip-0057` with `-D warnings` clean.

## Task Commits

Each task was committed atomically:

1. **Task 1: Verify baseline and current consumer-callsite paths before editing** — *no commit* (verification-only precondition task; no source code edits)
2. **Task 2: Replace 5 public-module wildcards in silent_payments/mod.rs with explicit named re-exports** — `75d56116` (refactor)

## Files Created/Modified

- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — Replaced lines 31-44 (the wildcard block) with the intermediate-shape named-export body. Module rustdoc on lines 1-30 untouched. Final file body (lines 31-55):

```rust
mod aggregate;
pub use aggregate::aggregate_sender_sks;

mod input_hash;
pub use input_hash::compute_input_hash;

mod one_time;
pub use one_time::derive_one_time_puzzle_hash;

mod protocol;
pub use protocol::{
    compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,
    puzzle_hash_for_pk,
};

mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};

mod send_keys;
pub(crate) use send_keys::*;

mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
```

## Decisions Made

- **Intermediate shape kept for Wave 1.** The plan explicitly directs landing the intermediate shape (4 separate `mod` declarations + 4 named pub-use blocks for aggregate/input_hash/one_time/protocol) rather than collapsing to the POLISH-02 final shape. Plan 08-04 (Wave 2) will delete aggregate.rs / input_hash.rs / one_time.rs and collapse the 4 named-export blocks into a single `pub use protocol::{aggregate_sender_sks, compute_input_hash, compute_shared_secret_from_tweak, derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk};` block. This sequencing preserves Wave 1 parallelizability and keeps POLISH-02 as a single atomic commit per Risk R8.
- **Module rustdoc unchanged.** Per CONTEXT.md Claude's Discretion item 3: current rustdoc (lines 1-30) references primitives by symbol (e.g., `[compute_shared_secret_from_tweak]`, `[scan_from_tweaks]`), not by sub-file location. The doc-comment intra-doc links still resolve post-edit. No edit required.
- **Symbol ordering within `pub use protocol::{...}` left as rustfmt-canonical.** Rustfmt re-wraps the multi-line block to a single line (`compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk,`) when symbols fit under the 100-col limit; this matches the workspace style. Symbol ordering is alphabetical within the block.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rustfmt re-wrapped the multi-line `pub use protocol::{...}` block**
- **Found during:** Task 2 (post-edit fmt-check)
- **Issue:** The plan's verbatim post-edit body specified one symbol per line for the 5-name protocol re-export; `cargo fmt --all --check` rejected this format because rustfmt's default behavior re-wraps multi-name imports onto a single line if they fit within the column limit (100 chars).
- **Fix:** Ran `cargo fmt --all` to apply the rustfmt-canonical formatting. The 5 protocol symbols now sit on two lines (`compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,` + `puzzle_hash_for_pk,`) inside the `pub use protocol::{...}` block. Semantically identical; matches rest-of-workspace formatting.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/mod.rs`
- **Verification:** `cargo fmt --all --check` passes; all 5 grep acceptance gates (`^pub use [a-z_]+::\*;` = 0, `^pub(crate) use send_keys::\*;` = 1, `^pub use protocol::` >= 1, `^pub use scanner::\{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks\};` = 1, `^pub use types::\{DetectedSpCoin, OutputMeta, TweakData\};` = 1) all pass post-fmt.
- **Committed in:** `75d56116` (Task 2 commit)

**2. [Plan precondition correction — informational, not a code change] `chia-sdk-driver/src/lib.rs` DOES contain `pub use silent_payments::*;`**
- **Found during:** Task 1 baseline grep
- **Issue:** Task 1's verification step 3 expected `grep -c 'pub use silent_payments' crates/chia-sdk-driver/src/lib.rs` to return 0 ("no top-level driver re-export of the silent_payments module's contents"). Actual count is 1 — `lib.rs:40` has `#[cfg(feature = "chip-0057")] pub use silent_payments::*;` matching the workspace pattern for all 14 other top-level driver modules (action_system, actions, clear_signing, driver_error, hashed_ptr, layer, layers, offers, primitives, puzzle, spend, spend_bundle_cost, spend_context, spend_with_conditions all use the same wildcard).
- **Why this is correct intended behavior, not a bug:** The bindings facade at `crates/chia-sdk-bindings/src/silent_payments.rs:432` calls `chia_sdk_driver::aggregate_sender_sks` (top-level driver path, not `chia_sdk_driver::silent_payments::aggregate_sender_sks`). Removing the top-level re-export would break this consumer. The plan's expectation was incorrect; named-list re-exports in `silent_payments/mod.rs` propagate cleanly through this wildcard to driver top level. Workspace `--all-features` build (including bindings, napi, pyo3, wasm) is clean, confirming the propagation works.
- **Fix:** None required — proceeded with Task 2 as planned. Documented for future plans.
- **Files modified:** none
- **Verification:** `cargo build --release --workspace --all-features` clean.

---

**Total deviations:** 1 auto-fixed (1 blocking — rustfmt re-wrap) + 1 informational precondition correction (no code change)
**Impact on plan:** rustfmt re-wrap is cosmetic; semantically the named-export body is identical to D-01's locked verbatim shape. The precondition mismatch on `lib.rs:40` does not change Task 2's edits; the top-level re-export is the documented binding-consumer path and survives Phase 8.

## Issues Encountered

None. All verification gates pass:
- `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returns 0
- `grep -c '^pub(crate) use send_keys::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returns 1
- `cargo build --release -p chia-sdk-driver --features chip-0057` exits 0
- `cargo build --release --workspace --all-features` exits 0
- `cargo build --release -p chia-wallet-sdk --all-features` exits 0
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo fmt --all --check` exits 0

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 08-04 (Wave 2, POLISH-02) inherits the named-list authority recorded here. The post-fold mod.rs body will collapse the 4 named-export blocks (`pub use aggregate::aggregate_sender_sks;` + `pub use input_hash::compute_input_hash;` + `pub use one_time::derive_one_time_puzzle_hash;` + `pub use protocol::{compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk};`) into a single `pub use protocol::{aggregate_sender_sks, compute_input_hash, compute_shared_secret_from_tweak, derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk};` block after deleting aggregate.rs / input_hash.rs / one_time.rs.
- Plans 08-02 (POLISH-03 doc dedup) and 08-03 (POLISH-04 dispatch restructure) are independent of this plan and can proceed in Wave 1 without depending on this artifact.

---
*Phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics*
*Completed: 2026-05-20*

## Self-Check: PASSED

- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — FOUND
- Commit `75d56116` — FOUND (refactor(08-01): replace mod.rs wildcards with named re-exports (POLISH-01))
- POLISH-01 acceptance grep returns 0 — VERIFIED
- All 5 build/clippy/fmt gates green — VERIFIED
