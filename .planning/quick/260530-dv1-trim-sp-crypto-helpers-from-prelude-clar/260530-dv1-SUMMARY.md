---
phase: quick
plan: 260530-dv1
subsystem: api
tags: [chip-0057, silent-payments, prelude, cargo-features, surface-cleanup]

# Dependency graph
requires:
  - phase: 09.1
    provides: post-v1.0-audit chip-0057 silent-payments surface
provides:
  - Trimmed chip-0057 driver re-export block in prelude (7 user-facing names only)
  - Documented load-bearing chia-sdk-test/chip-0057 dev-dep feature line
affects: [silent-payments, bindings, future chip-0057 surface work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Curated prelude exposes primitive + info types only (Cat/Nft/Did precedent), not internal derivation fns"

key-files:
  created: []
  modified:
    - src/prelude.rs
    - crates/chia-sdk-driver/Cargo.toml

key-decisions:
  - "Trim 7 internal SP derivation helpers from prelude::* (still reachable via chia_sdk_driver::silent_payments::), aligning SP surface with Cat/Nft/Did"
  - "Keep chia-sdk-test/chip-0057 feature line (load-bearing for non-all-features e2e test) and only annotate it with a clarifying comment"

patterns-established:
  - "SP prelude surface mirrors Cat/Nft/Did: only user-facing primitive/info/scan symbols, no internal crypto helpers"

requirements-completed: []

# Metrics
duration: 6min
completed: 2026-05-30
---

# Quick Task 260530-dv1: Trim SP Crypto Helpers from Prelude + Clarify Dev-Dep Summary

**Removed 7 internal silent-payment derivation helpers from the curated `prelude::*` (Cat/Nft/Did surface parity) and annotated the load-bearing `chia-sdk-test/chip-0057` dev-dep feature line so it no longer reads as dead weight — zero behavior change, zero new deps.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-05-30T16:01:04Z
- **Completed:** 2026-05-30T16:06:30Z
- **Tasks:** 2
- **Files modified:** 2 (file-disjoint)

## Accomplishments

- `prelude::*` chip-0057 driver block trimmed from 14 names to the 7 user-facing symbols (`DetectedSpCoin`, `K_MAX_DEFAULT`, `OutputMeta`, `SilentPaymentScan`, `TweakData`, `scan_from_tweaks`, `tweak_data_from_block_spends`). The 7 internal helpers (`compute_input_hash`, `compute_shared_secret_from_tweak`, `derive_one_time_puzzle_hash`, `derive_onetime_pk`, `derive_onetime_sk`, `derive_output_tweak`, `puzzle_hash_for_pk`) stay fully reachable via `chia_sdk_driver::silent_payments::` — definitions in `protocol.rs` untouched.
- `chia-sdk-test/chip-0057` entry in the `chip-0057` feature array now carries an inline comment documenting that it enables `chia_sdk_test::silent_payments` for the non-all-features e2e integration test. Line preserved verbatim (NOT removed); only a comment added.
- Verified the critical load-bearing path: `cargo test -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` (NO `--all-features`) still compiles and passes all 3 e2e tests.

## Task Commits

Each task was committed atomically (normal pre-commit hooks, no `--no-verify`):

1. **Task 1: Trim 7 internal SP helpers from the curated prelude** - `c8582f38` (refactor)
2. **Task 2: Document the load-bearing chia-sdk-test dev-dep feature line** - `220a00e0` (docs)

## Files Created/Modified

- `src/prelude.rs` - Removed 7 internal SP derivation helpers from the chip-0057 `chia_sdk_driver::silent_payments::` re-export block; kept the 7 user-facing names. `chia_sdk_utils::silent_payments` block untouched.
- `crates/chia-sdk-driver/Cargo.toml` - Added an inline `#` comment to the `chia-sdk-test/chip-0057` entry in the `chip-0057` feature array explaining its load-bearing purpose for the non-all-features e2e test.

## Decisions Made

None beyond plan - executed exactly as specified. Both decisions were pre-validated in the plan (Cat/Nft/Did surface parity; keep-the-line + annotate).

## Deviations from Plan

None - plan executed exactly as written.

The plan's Task 1 KEPT-OK acceptance check (`grep -c ... | grep -qx 7`) was formulated assuming rustfmt would place each kept name on its own line, but rustfmt canonically packs the 7 names onto 2 lines, so the line-count grep returns 2 rather than 7. This is a check-formulation artifact, not a substantive failure: per-name occurrence counting confirms all 7 user-facing names are present exactly once each, none duplicated or missing, and the resulting block is byte-identical to the plan's prescribed output (`src/prelude.rs:41-44`). The must_have ("still re-exports the 7 user-facing SP driver symbols incl. K_MAX_DEFAULT") is fully satisfied. No code change was made to accommodate the check.

## Issues Encountered

None. The two pre-existing `chia-sdk-daemon` clippy::pedantic warnings (`client.rs:426-427`, `match_wildcard_for_single_variants` and `match_like_matches_macro`) appeared in the workspace clippy run — these are documented pre-existing, out-of-scope warnings (STATE.md Plan 01-05 decision, logged to deferred-items.md) explicitly allowed by the plan's regression gate. They do not touch either edited file.

## Verification Results

Per-task acceptance:
- `REMOVED-OK` — 7 internal helper names gone from `src/prelude.rs` (grep returns 0).
- 7 user-facing names present (each exactly once); resulting block matches plan spec verbatim.
- `DEFS-UNTOUCHED` — `pub fn compute_input_hash` still present in `protocol.rs`.
- `cargo build -p chia-wallet-sdk --all-features` — succeeds.
- `cargo build --examples --features chip-0057` — succeeds (example resolves `K_MAX_DEFAULT` via prelude glob at lines 92 and 182).
- `LINE-PRESENT` + `COMMENT-PRESENT` — `chia-sdk-test/chip-0057` line still present, now carries `#` comment.
- `cargo test -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` (NO `--all-features`) — **3 passed; 0 failed** (`test_simulator_e2e_m0_self_change`, `test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`). Critical correctness guard satisfied.

Whole-task regression gate:
- `cargo build --release --workspace --all-features` — succeeds (3m 32s).
- `cargo build --examples` (no features) — succeeds (gated example skipped).
- `cargo clippy --workspace --all-features --all-targets` — clean except the 2 pre-existing `chia-sdk-daemon` warnings (allowed by plan).
- `cargo fmt --all -- --check` — clean.
- `cargo machete` — clean (no unused deps, zero new ignored entries).
- Zero new workspace deps; zero new `#[allow]` attributes; zero behavior change (confirmed via full diff inspection).

## Next Phase Readiness

Quick task — no phase dependency. SP prelude surface is now consistent with Cat/Nft/Did conventions, and the Cargo manifest footgun is documented. Both audit nits closed.

---
*Quick task: 260530-dv1*
*Completed: 2026-05-30*

## Self-Check: PASSED

- FOUND: src/prelude.rs
- FOUND: crates/chia-sdk-driver/Cargo.toml
- FOUND: .planning/quick/260530-dv1-trim-sp-crypto-helpers-from-prelude-clar/260530-dv1-SUMMARY.md
- FOUND: commit c8582f38 (Task 1)
- FOUND: commit 220a00e0 (Task 2)
