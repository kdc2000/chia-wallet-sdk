---
phase: quick-260605-bm5
plan: 01
subsystem: chia-sdk-driver / silent-payments (chip-0057)
tags: [chip-0057, silent-payments, action-system, driver-error, invariant]
requires:
  - "chip-0057 SP send action + sp_finish_branch (Phase 04 / 04.1 / 04.2)"
provides:
  - "DriverError::SilentPaymentMixedAssetBundle (chip-0057 gated)"
  - "XCH-only invariant hard-error guard (GATE 0) in sp_finish_branch"
  - "test_simulator_e2e_multi_input_mixed_asset_rejected"
affects:
  - "Spends::finish_with_keys behavior when an SP payment is pending"
tech-stack:
  added: []
  patterns:
    - "Early uniform hard-error guard (GATE 0) before all other SP finish gates"
key-files:
  created: []
  modified:
    - crates/chia-sdk-driver/src/driver_error.rs
    - crates/chia-sdk-driver/src/action_system/spends.rs
    - crates/chia-sdk-driver/tests/silent_payments_e2e.rs
decisions:
  - "Mixed-asset SP bundles are REJECTED, not made detectable: a simpler, safer XCH-only contract."
  - "av7 sub-cycle: nothing to delete on this branch (pre-av7 base); CAVEAT verified safe empirically."
metrics:
  duration: ~15 min
  completed: 2026-06-05
---

# Quick 260605-bm5: Hard-error reject CHIP-0057 SP sends co-bundled with non-XCH assets — Summary

One-liner: Enforce the XCH-only invariant for silent-payment send bundles by adding `DriverError::SilentPaymentMixedAssetBundle` and an early uniform GATE-0 hard-error guard in `sp_finish_branch` that fires whenever any of `spends.cats/dids/nfts/options` is non-empty while an SP payment is pending; replace the would-be mixed-asset detection test with a rejection test that genuinely guards the variant.

## What changed

- **`driver_error.rs`**: Added `SilentPaymentMixedAssetBundle` (chip-0057 gated, doc comment + `#[error(...)]` matching the `SilentPayment*` style) after `SilentPaymentKeyNotSynthetic`.
- **`action_system/spends.rs`**: Added GATE 0 as the FIRST statement in `sp_finish_branch` (before GATE 1's `non_ephemeral_xch_count` block). It returns `Err(DriverError::SilentPaymentMixedAssetBundle)` when any of the four non-XCH `IndexMap`s is non-empty. Fires uniformly regardless of XCH input count. The GATE 1 comment was rewritten to note GATE 0 neutralizes the non-XCH chain (replacing a stale `quick-260601-e6z` deferred-follow-up note).
- **`tests/silent_payments_e2e.rs`**: Imported `DriverError`; added `test_simulator_e2e_multi_input_mixed_asset_rejected` (2 XCH inputs + co-bundled `Action::single_issue_cat(None, 1)`) asserting `matches!(result, Err(DriverError::SilentPaymentMixedAssetBundle))`.

## CAVEAT outcome (Task 2) — REQUIRED DISCLOSURE

**Decision: the av7 sub-cycle was NOT deleted, because it does not exist on this branch.**

This worktree (`worktree-agent-afdce163f5d9ec6c4`, based on `133cc80b`) is a **pre-av7 base**. The av7 "SP-specific closed XCH-only AssertConcurrentSpend sub-cycle" lives only in commit `df0bb366` on `main`, which this branch does NOT contain. Concretely, on this branch:
- `sp_finish_branch` ends with the per-pending `CreateCoin` loop followed directly by `Ok(())` — there is NO `sp_cycle_inputs` collection and NO `if sp_cycle_inputs.len() >= 2 { ... }` block.
- There is NO `test_simulator_e2e_multi_input_mixed_asset` detection test to replace (so the rejection test is ADDED, not swapped).
- The GATE 1 comment referenced a different deferred item (`quick-260601-e6z`), not the av7 sub-cycle.

The orchestrator forward-ports these commits onto the PR branch (`chip-0057-silent-payments`); whether the av7 sub-cycle exists there at merge time is the orchestrator's concern. On THIS branch there was no sub-cycle to remove.

### CAVEAT verification evidence (performed regardless)

Even though there was nothing to delete, I performed the full code-path trace and the empirical confirmation the plan demands, treating "no sub-cycle present" as the post-deletion state:

1. **Code-path trace (ephemeral standard-XCH coins in the AssertConcurrent cycle):** The general `emit_relation` cycle (`spends.rs:392-418`) iterates `iter_conditions_spends()` (`spends.rs:234-291`), which does NOT filter `ephemeral` on the `xch.items` chain and chains in CAT/DID/NFT/option conditions-spends.
   - **(b) the CAT/DID/NFT/option chain** is fully neutralized by GATE 0: for any SP finish that reaches the cycle, `cats/dids/nfts/options` are guaranteed empty (otherwise GATE 0 already errored).
   - **(a) ephemeral standard-XCH coins**: `intermediate_conditions_source` (`fungible_spends.rs:172-203`) CAN push an ephemeral conditions-kind child XCH coin into `xch.items` when leftover required/optional conditions need a carrier coin. In principle this is a residual risk that the av7 `!item.ephemeral`-filtered sub-cycle was designed to exclude. The dids/nfts/options `intermediate_fungible_xch_spend` pushes (`spends.rs:343-383`) cannot fire (those maps are empty under GATE 0).
2. **Empirical confirmation:** `test_simulator_e2e_multi_input` (XCH-only, multi-input, no sub-cycle present on this branch) detects **exactly 1**. Run output: `test test_simulator_e2e_multi_input ... ok`. This proves that for the canonical XCH-only multi-input scenario, no ephemeral standard-XCH coin pollutes the cycle into a superset of S — the general cycle binds exactly the set the receiver reconstructs, so detection holds without any sub-cycle.

**Conclusion:** The XCH-only multi-input detection path is correct on this branch without a sub-cycle (empirically verified), AND mixed-asset bundles now hard-error at GATE 0 before the cycle is ever built. No detection gap is introduced by this change. The residual theoretical (a)-risk for exotic ephemeral-XCH layouts is the same risk that exists today on this pre-av7 branch independent of this task; it is not introduced here and is out of scope (the existing `test_simulator_e2e_multi_input` exercises the production multi-input path and is green).

## Rejection-test guard validation — REQUIRED DISCLOSURE

The rejection test was validated as a genuine guard (not a tautology):
- It asserts the SPECIFIC variant: `matches!(result, Err(DriverError::SilentPaymentMixedAssetBundle))` (not "any Err").
- **Sanity-check performed:** I temporarily disabled GATE 0 (wrapped the condition in `if false && (...)`), re-ran the test, and it FAILED with `finish_with_keys` returning `Ok(Outputs { ... cats: {New(0): [Cat {...}]} ... })` — i.e. the mixed XCH+CAT bundle succeeded. I then restored the guard byte-for-byte (`git diff` on `spends.rs` is empty against the committed Task-1 state) and the test passes again. This proves the test depends on the guard.

## Deviations from Plan

The plan's framing assumed an av7 sub-cycle and an av7 detection test were present to reverse/replace. On this pre-av7 branch neither exists. This is not a behavioral deviation from the plan's GOAL (XCH-only invariant + hard-error rejection) — every must-have truth and artifact is satisfied — but the mechanics differ:
- Task 2 "delete the sub-cycle" → no-op (nothing to delete); CAVEAT verification still performed and reported above.
- Task 3 "REPLACE the mixed-asset detection test" → ADD the rejection test (no detection test existed to replace).

No auto-fixes (Rules 1-3) were required. No architectural changes (Rule 4). No new `#[allow]` attributes (verified via `git diff 133cc80b..HEAD`). No new dependencies.

## Verification gates (actual output)

| Gate | Command | Result |
| ---- | ------- | ------ |
| 1. chip-0057 build | `cargo build --release -p chia-sdk-driver -F chip-0057` | `Finished release ... in 4.67s` |
| 2. clippy all-features all-targets | `cargo clippy -p chia-sdk-driver --all-features --all-targets -- -D warnings` | `Finished` — clean under `-D warnings` |
| 3. all-features tests | `cargo test --release -p chia-sdk-driver --all-features` | `2343 passed; 0 failed` (lib) + `5 passed; 0 failed` (silent_payments_e2e) |
| 4. fmt | `cargo fmt --all -- --check` | clean (no diff) |
| 5. non-chip-0057 build | `cargo build -p chia-sdk-driver` | `Finished dev ... in 23.84s` |

SP e2e suite (5 tests, all green):
```
test test_simulator_e2e_multi_input_mixed_asset_rejected ... ok
test test_simulator_e2e_m0_self_change ... ok
test test_simulator_e2e_labeled ... ok
test test_simulator_e2e_unlabeled ... ok
test test_simulator_e2e_multi_input ... ok
```

## Commits

- `ca449f64` feat(quick-260605-bm5): reject mixed-asset SP send bundles (XCH-only invariant)
- `b7bf81db` test(quick-260605-bm5): add mixed-asset SP bundle rejection test

## Known Stubs

None.

## Branch note

Developed on the agent worktree branch only (`worktree-agent-afdce163f5d9ec6c4`, based on `133cc80b`). Did NOT touch the `chip-0057-silent-payments` branch, any remote, or `PR-BODY.md` — branch propagation is handled by the orchestrator. ROADMAP.md was not updated per constraints.

## Self-Check: PASSED

All 3 modified source files + SUMMARY.md exist; the `SilentPaymentMixedAssetBundle` token is present in driver_error.rs, spends.rs, and the test; both task commits (`ca449f64`, `b7bf81db`) are in the git log.
