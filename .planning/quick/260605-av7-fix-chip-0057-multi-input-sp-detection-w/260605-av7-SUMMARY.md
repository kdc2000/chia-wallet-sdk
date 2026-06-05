---
phase: quick-260605-av7
plan: 01
subsystem: payments
tags: [chip-0057, silent-payments, assert-concurrent-spend, scc, action-system, regression-test]

# Dependency graph
requires:
  - phase: 04.1
    provides: AssertConcurrent input-binding cycle (emit_relation) + SilentPaymentRequiresInputBinding gate
  - phase: 04.2 / 09.3
    provides: gated Action::silent_payment_send + sp_finish_branch finish path
provides:
  - SP-specific closed XCH-only AssertConcurrentSpend sub-cycle in sp_finish_branch
  - mixed-asset multi-input SP detection regression guard
affects: [silent-payments, send-side, receive-side, scanner, PR-prep]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SP send emits an additive, SP-specific closed assert_concurrent_spend cycle over exactly the compute_input_hash XCH input set, alongside (not replacing) the general emit_relation cycle"

key-files:
  created: []
  modified:
    - crates/chia-sdk-driver/src/action_system/spends.rs
    - crates/chia-sdk-driver/tests/silent_payments_e2e.rs

key-decisions:
  - "Fix is additive: emit_relation's general cycle is left untouched; the SP sub-cycle is layered on top inside the chip-0057-gated sp_finish_branch. Redundant assert_concurrent_spend conditions on the XCH coins (from both cycles) are harmless."
  - "Sub-cycle is collected over non-ephemeral AND conditions-kind XCH items, matching the compute_input_hash predicate restricted to conditions-kind (settlement-kind XCH inputs cannot carry assert_concurrent_spend and are not part of SP sends)."
  - "Regression test uses the CAT-issuance approach (Action::single_issue_cat co-bundled with Action::silent_payment_send) rather than spending an existing CAT — reachable via the action API already used by issue_cat tests."

patterns-established:
  - "Cycle shape mirrors emit_relation exactly (entry 0 asserts last entry's coin_id; entry i>=1 asserts entry i-1's) so the receiver's SCC reconstruction is identical."

requirements-completed: [SEND-06, FINGERPRINT-01]

# Metrics
duration: ~20min
completed: 2026-06-05
---

# Quick Task 260605-av7: CHIP-0057 Multi-Input SP Detection Under Co-Bundled Assets Summary

**Fixed the multi-input silent-payment detection false-negative that occurred when an SP send was co-bundled with a non-XCH asset (CAT/DID/NFT), by emitting a closed XCH-only AssertConcurrentSpend sub-cycle over exactly the compute_input_hash input set inside sp_finish_branch, plus a verified-regressing mixed-asset round-trip test.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `sp_finish_branch` now emits a closed `assert_concurrent_spend` cycle over exactly the non-ephemeral, conditions-kind XCH input set (set `S`, identical to the set fed to `compute_input_hash`) whenever that set has >= 2 members. This sub-cycle is self-contained among standard-puzzle coins, so it survives the receiver's Stage 1 standard-only filter intact and the `{X1, X2, ...}` strongly-connected component (hence `input_hash`) is reproduced even when a non-XCH asset is co-spent.
- The general `emit_relation` AssertConcurrent cycle is unchanged — the fix is purely additive and entirely under `#[cfg(feature = "chip-0057")]`.
- Added `test_simulator_e2e_multi_input_mixed_asset`, a full send -> farm -> extract -> scan -> detect regression guard that co-bundles a CAT issuance with a 2-XCH-input SP send and asserts the SP output is still detected.
- The stale GATE 1 "open design question" comment (referencing quick-260601-e6z) was removed/replaced with a note describing the now-implemented sub-cycle.

## Task Commits

1. **Task 1: SP-specific closed XCH-only AssertConcurrentSpend sub-cycle + GATE 1 comment cleanup** - `64428865` (fix)
2. **Task 2: Mixed-asset multi-input SP detection regression guard** - `15f6b59d` (test)

## Files Created/Modified

- `crates/chia-sdk-driver/src/action_system/spends.rs` - Added the closed XCH-only sub-cycle at the end of `sp_finish_branch` (after the per-pending derivation loop, before `Ok(())`); collects `(index, coin_id)` of every non-ephemeral conditions-kind XCH item in an immutable pass, then in a second mutable pass emits `Conditions::new().assert_concurrent_spend(predecessor)` matching `emit_relation`'s shape. Replaced the stale GATE 1 design-question comment.
- `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` - Added `test_simulator_e2e_multi_input_mixed_asset` mirroring `test_simulator_e2e_multi_input` with a co-bundled `Action::single_issue_cat(None, 1)`, asserting `detections.len() == 1`, `amount == 1000`, `label.is_none()`, `k == 0`.

## Decisions Made

- **Additive, not a rewrite of emit_relation.** Honored the hard constraint that the general cycle (load-bearing for non-SP multi-asset bundles) must not change. The SP sub-cycle is layered on inside the already-`#[cfg(feature = "chip-0057")]` `sp_finish_branch`. Both cycles touch the XCH coins; the duplicate `assert_concurrent_spend` conditions are harmless.
- **Predicate: non-ephemeral AND conditions-kind.** This equals the `compute_input_hash` input set in practice (settlement-kind XCH inputs are not part of SP sends and cannot carry an `assert_concurrent_spend`). Collecting over conditions-kind is exact and lets the second pass call `add_conditions` via `SpendKind::Conditions`.
- **CAT-issuance test approach (not existing-CAT spend).** Reachable via the same action API the `issue_cat` tests use; the issued CAT is a non-standard co-spend in the same bundle whose parent XCH spend is part of the SP input cycle, and the CAT coin is dropped at the receiver's Stage 1 — exactly the false-negative scenario.

## Regression-on-Revert Verification (load-bearing)

The new test is a genuine regression guard, not a tautology. Verified directly: with Task 1's sub-cycle temporarily disabled (replaced by a no-op `let _ = &sp_cycle_inputs;`), `cargo test ... test_simulator_e2e_multi_input_mixed_asset` **FAILS** with:

```
assertion `left == right` failed: mixed-asset multi-input send must still produce exactly one detection ...
  left: 0
 right: 1
```

With the fix in place the test **PASSES** (`detections.len() == 1`). The fix was restored byte-for-byte after the revert check (confirmed `sp_cycle_inputs.len() >= 2` present; full driver suite + clippy re-run clean afterward). The XCH-only `test_simulator_e2e_multi_input` continues to pass both with and without the fix, confirming the fix does not perturb the XCH-only path (`C == S` there).

## Deviations from Plan

None - plan executed exactly as written. `Action::silent_payment_send` and `Action::single_issue_cat` were both present and used as the plan specified (per Phase 09.3, `Action::silent_payment_send` is the current gated SP send constructor).

## Issues Encountered

- The plan's `@`-referenced file paths resolve to the shared checkout, but this agent is isolated in a git worktree (`.claude/worktrees/agent-a96cdcaee6752f912`). All edits, builds, and commits were performed against the worktree copy on branch `worktree-agent-a96cdcaee6752f912`. No remote or `chip-0057-silent-payments` branch was touched (per constraints).

## Verification Gate Results (actual output)

1. **`cargo build --release -p chia-sdk-driver -F chip-0057`** — clean. `Finished release profile ... in 4.60s`.
2. **`cargo clippy -p chia-sdk-driver --all-features --all-targets -- -D warnings`** — clean. `Finished dev profile ... in 15.44s`; zero warnings, zero new `#[allow]`.
3. **`cargo test --release -p chia-sdk-driver --all-features`** — `test result: ok. 2343 passed; 0 failed` (lib) + `5 passed; 0 failed` (silent_payments_e2e integration target). Named no-regression guards all pass: `assert_concurrent_relation_emits_cycle_for_n_coins_2/_3/_4`, `same_ph_multi_input_round_trip_via_concurrent_spend`, `test_concurrent_spend_pollution_resistance`, `test_bug1_mixed_ph_multi_input_full_cycle_detected`, `test_bug2_single_input_sharing_ph_detected_via_singleton`, `test_simulator_e2e_multi_input` (XCH-only). New `test_simulator_e2e_multi_input_mixed_asset` passes.
4. **`cargo fmt --all -- --check`** — clean (exit 0).
5. **`cargo build --release -p chia-sdk-driver`** (no chip-0057) — clean. `Finished release profile ... in 7.12s`. Confirms nothing leaked out of the `chip-0057` gate.

## FLAG FOR ORCHESTRATOR

`.planning/PR-BODY.md`'s **"Open design question"** bullet (about whether to filter the `AssertConcurrent` cycle to exactly the SP XCH-input set vs. document the constraint) **must be DROPPED** — this fix resolves it by emitting the SP-specific closed XCH-only sub-cycle. The executor does not own `.planning/PR-BODY.md` and has not modified it.

Test-construction approach used: **CAT issuance** (`Action::single_issue_cat(None, 1)` co-bundled with `Action::silent_payment_send`). The **regress-on-revert property holds** (verified empirically above: detections 0 -> 1).

## Next Steps / Readiness

- Ready for orchestrator-side branch propagation and PR prep (PR-BODY.md "Open design question" bullet drop).
- No blockers. No ROADMAP update (quick task, per constraints).

## Self-Check: PASSED

- FOUND: crates/chia-sdk-driver/src/action_system/spends.rs (sub-cycle present)
- FOUND: crates/chia-sdk-driver/tests/silent_payments_e2e.rs (test fn present)
- FOUND: .planning/quick/260605-av7-.../260605-av7-SUMMARY.md
- FOUND: commit 64428865 (Task 1 fix)
- FOUND: commit 15f6b59d (Task 2 test)

---
*Quick task: 260605-av7*
*Completed: 2026-06-05*
