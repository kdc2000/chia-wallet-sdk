---
phase: 09-real-block-tweakdata-bridge-python-relation-binding
plan: 02
subsystem: silent-payments-test-helper
tags: [chip-0057, silent-payments, simulator, refactor, byte-equality-oracle]

# Dependency graph
requires:
  - phase: 09-real-block-tweakdata-bridge-python-relation-binding
    plan: 01
    provides: chia_sdk_driver::silent_payments::tweak_data_from_block_spends canonical helper
provides:
  - One canonical implementation of CHIP-0057 Pass 2a/2b grouping in the workspace
  - Drift-impossible adapter from simulator height to TweakData
  - Byte-identical Phase 6 e2e oracle output across refactor boundary
affects: [09-06-bridge-cross-binding-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Thin-adapter delegate over canonical real-block helper
    - .expect() guard at boundary where Err arm is structurally unreachable for simulator-shaped inputs

key-files:
  created: []
  modified:
    - crates/chia-sdk-test/src/silent_payments/tweak_data.rs

key-decisions:
  - "Used .expect() to unwrap Result<TweakData, DriverError> from tweak_data_from_block_spends rather than propagating via ? + changing return type to Result. Simulator-shaped inputs are well-formed by construction (Simulator::spend_coins only accepts validated coin spends); the Err arm in the canonical helper is reserved for future protocol-level validation. .expect() with a descriptive message preserves the existing public signature exactly (must_have #4: byte-for-byte unchanged) and is documented in the panic message."
  - "Deleted all 7 imports that powered the inline algorithm (chia_bls::PublicKey, chia_protocol::Bytes32, OutputMeta, compute_input_hash, Layer, Puzzle, StandardLayer, ScalarField, ToClvm, Allocator). Adapter needs only TweakData + tweak_data_from_block_spends from the driver, plus the local Simulator."
  - "Kept both inline tests (tweak_data_empty_block_returns_empty_tweak_data, tweak_data_genesis_height_is_safe) verbatim per plan must_have #3. They exercise the adapter end-to-end; the empty-block case also exercises the canonical helper's empty-input branch (which returns Ok(TweakData{tweak_points:[], outputs:[]}))."
  - "Removed two short doc-comment phrases (`SIM-01 defensive guard:` and the `CHIP §459`-prefixed paragraph in the rustdoc) by absorbing them into a single 'See the driver-side module-level docs ...' pointer. CLEANUP-01 grep would not have flagged them; the abbreviation is purely a readability improvement that keeps documentation co-located with the algorithm it describes."

patterns-established:
  - "Thin-adapter convention: when a test-side helper duplicates logic that has a canonical driver-side implementation, the adapter body collapses to fetch-inputs + delegate-call + .expect/? — see this file as the precedent for future BRIDGE-* style refactors."

requirements-completed: [BRIDGE-02]

# Metrics
duration: 4min
completed: 2026-05-29
---

# Phase 9 Plan 02: BRIDGE-02 simulator helper collapse to adapter Summary

**Collapsed `chia-sdk-test::tweak_data_from_simulator_block` to a 5-line delegate over `chia_sdk_driver::silent_payments::tweak_data_from_block_spends`; canonical Pass 2a/2b grouping algorithm now lives in exactly one place in the workspace.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-29T16:25:37Z
- **Completed:** 2026-05-29T16:29:52Z
- **Tasks:** 1
- **Files modified:** 1
- **Lines deleted:** 56 net (-121 + 65)

## Accomplishments

- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` shrunk from 121 lines (full Stage 1/3/4 algorithm + 2 tests) to 65 lines (delegate + 2 tests).
- Public API of `chia_sdk_test::silent_payments::tweak_data_from_simulator_block(sim, height) -> TweakData` byte-for-byte unchanged (signature, return type, `#[must_use]` attribute all preserved per must_have #4).
- All 7 algorithm-specific imports deleted: `chia_bls::PublicKey`, `chia_protocol::Bytes32`, `OutputMeta`, `compute_input_hash`, `Layer`, `Puzzle`, `StandardLayer`, `ScalarField`, `ToClvm`, `Allocator`. Adapter retains only `TweakData` + `tweak_data_from_block_spends` from the driver and the local `Simulator`.
- Drift between simulator-only and real-block grouping is now structurally impossible — both code paths share the canonical helper.

## Byte-equality oracle results

The 3 Phase 6 simulator e2e tests are the load-bearing regression bar for this plan. All three passed unchanged with the new adapter:

- `test_simulator_e2e_unlabeled` — PASS
- `test_simulator_e2e_labeled` — PASS
- `test_simulator_e2e_m0_self_change` — PASS

This confirms that the canonical helper's standalone single-spend branch reproduces the simulator helper's "aggregate-everything" semantics for simulator-style 1-tx-per-block blocks (per CONTEXT D-03 expectation; Pitfall 4 resolved by construction — each simulator block contains exactly one coin spend, which falls into the standalone-singleton branch and aggregates trivially).

The 2 inline simulator-helper tests also passed:

- `tweak_data_empty_block_returns_empty_tweak_data` — PASS
- `tweak_data_genesis_height_is_safe` — PASS

## Task Commits

1. **Task 1: collapse helper body to delegate** — `89c5cb91` (refactor)

## Files Created/Modified

- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` (modified, 121 → 65 lines) — body collapsed to 5-line delegate; rustdoc shortened to point at driver-side module docs; both inline defensive tests preserved verbatim.

## Decisions Made

- **`.expect()` vs `?` error handling:** Chose `.expect()` with a descriptive message ("never errors on simulator-shaped inputs (all spends are well-formed by construction)"). The canonical helper's `Result<TweakData, DriverError>` return reserves space for future protocol-level validation, but the simulator only constructs well-formed `CoinSpend`s via `Simulator::spend_coins`, so the error path is structurally unreachable today. Using `?` would have forced the adapter's return type to `Result<TweakData, DriverError>`, breaking must_have #4 (public signature byte-for-byte unchanged). The panic message documents the invariant for any future maintainer who hits the unexpected case.
- **Rustdoc rewrite vs verbatim preservation:** The original rustdoc explained the algorithm (Pass 2a/2b grouping, identity-element guard, non-standard-puzzle skip) inline. The new rustdoc deletes the algorithm explanation and points readers at the driver-side module-level docs (which are now the single source of truth). Keeping the algorithm description in two places would re-introduce the drift problem the plan eliminates at the code level.

## Deviations from Plan

None. Zero auto-fixed issues; zero blocking issues; zero authentication gates. The plan-skeleton file shape (rustdoc + delegate + tests) compiled cleanly on first write, passed clippy on first run, and matched the byte-equality oracle on first execution.

The plan's `<action>` block anticipated a possible deviation if BRIDGE-01 added an error path that simulator inputs CAN trip — this did not materialize (the canonical helper's only error path is reserved for future protocol-level validation, not anything simulator inputs would trigger).

## Acceptance Criteria Status

- File size ≤ 80 lines: PASS (65 lines)
- Delegate call present: PASS (`grep -q 'tweak_data_from_block_spends'`)
- Old algorithm bits gone: PASS (`compute_input_hash`, `StandardLayer::parse_puzzle`, `synthetic_pks`, `scalar_multiply` all absent)
- Public signature unchanged: PASS (`pub fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> TweakData`)
- Both inline tests present: PASS
- Both inline tests pass: PASS (2 passed, 0 failed)
- 3 Phase 6 oracle tests pass byte-identically: PASS (3 passed, 0 failed)
- Scoped clippy `-D warnings` clean: PASS
- No GSD planning-artifact refs: PASS (grep returns 0 hits)
- rustfmt clean: PASS

## Issues Encountered

None requiring escalation or attention.

## User Setup Required

None.

## Next Phase Readiness

- **BRIDGE-06 (Plan 09-06 cross-binding multi-input tests):** The simulator helper is now a thin adapter; cross-binding tests that exercise it through the binding surface will inherit the canonical implementation. Combined with BRIDGE-05 (direct binding of `tweak_data_from_block_spends`), both binding paths now route through the same Rust function — no drift possible across napi/pyo3/wasm targets either.
- **Phase 9 remaining plans (04, 05, 06):** All Wave 2 plans depending on Wave 1 are unblocked by this completion; BRIDGE-02 was the last Wave 2 plan blocked specifically on BRIDGE-01.

## Self-Check: PASSED

- File modified: `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` (65 lines) — FOUND
- Commit `89c5cb91` present in `git log` — FOUND
- 2 inline tests pass — VERIFIED
- 3 Phase 6 oracle tests pass byte-identically — VERIFIED
- Scoped clippy `-D warnings` clean — VERIFIED
- Workspace `--all-features` build (excluding bindings) clean — VERIFIED
- `cargo machete` clean — VERIFIED
- `cargo fmt --check` clean — VERIFIED

---

*Phase: 09-real-block-tweakdata-bridge-python-relation-binding*
*Completed: 2026-05-29*
