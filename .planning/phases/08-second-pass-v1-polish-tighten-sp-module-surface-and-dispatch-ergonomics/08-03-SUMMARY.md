---
phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
plan: 03
subsystem: refactoring

tags: [chip-0057, silent-payments, dispatch, match-exhaustiveness, code-cleanup]

# Dependency graph
requires:
  - phase: 04.2-action-api-unification
    provides: SendDestination enum with chip-0057-gated SilentPayment variant; Action::send dispatches puzzle-hash vs silent-payment paths through this enum
  - phase: 07-code-review-cleanup
    provides: actions/silent_payment_send.rs module with pub(crate) handle_silent_payment_send entry point that the dispatch arm calls
provides:
  - Single exhaustive match for SendDestination dispatch in SendAction::spend (chip-0057 SP arm returns from inside; PuzzleHash arm continues to Cat/Did/Nft/Option dispatch)
  - Zero unreachable!() invocations in actions/send.rs
  - Cleaner top-to-bottom reading flow at the action dispatch site
affects: [phase-08-04-fold-protocol]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cfg-gated match arm with inner return (preferred over if-let early-return + post-match unreachable!) when one arm short-circuits to a helper and the other arm falls through to shared code"

key-files:
  created: []
  modified:
    - "crates/chia-sdk-driver/src/actions/send.rs"

key-decisions:
  - "Inner return from cfg-gated match arm replaces if-let + unreachable!() pair; behavior unchanged"
  - "Both explanatory comment blocks (3-line chip-0057 SP rationale + 3-line PuzzleHash exhaustive-extraction rationale) deleted — the new shape is self-explanatory (SP arm visibly returns; PuzzleHash arm visibly produces puzzle_hash)"
  - "rustfmt re-wraps the handle_silent_payment_send call's 6 positional args one-per-line per workspace >100-col canonical form (same precedent as Plan 08-01's rustfmt re-wrap of pub use blocks)"

patterns-established:
  - "When a cfg-gated enum variant needs different control flow than the un-gated variant, use a cfg-gated match arm with inner return rather than an outer if-let-then-match-with-unreachable pair"

requirements-completed: [POLISH-04]

# Metrics
duration: 7min
completed: 2026-05-20
---

# Phase 8 Plan 03: POLISH-04 Single-Match Dispatch Restructure Summary

**Replaced the chip-0057 dispatch in SendAction::spend with a single exhaustive match (SP arm returns from inside), eliminating both `unreachable!()` hits and two explanatory comment blocks while preserving byte-identical behavior.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-05-20T20:02:07Z
- **Completed:** 2026-05-20T20:08:54Z
- **Tasks:** 2 (1 verification-only + 1 source edit)
- **Files modified:** 1

## Accomplishments
- Task 1 (precondition) confirmed both `unreachable!` hits were at exactly lines 58 (comment) and 62 (executable arm), both inside the POLISH-04-scoped dispatch block — Risk R4 from 08-RESEARCH.md resolved; Open Q1 hypothesis confirmed
- Task 2 collapsed 23 lines (lines 41-63 pre-edit: comment + if-let + comment + 4-arm match) into the locked D-04 9-line single-match block (rustfmt-expanded to ~14 lines)
- All 9 named tests from success criteria passed: `test_action_send_xch`, `test_action_send_xch_with_change`, `test_action_send_xch_split`, `test_action_send_cat` (case_1_normal + case_2_revocable), `test_action_send_cat_with_change` (×2), `test_action_send_cat_split` (×2)
- All 5 dispatch-touching regression tests passed: `action_state_machine`, `silent_payment_destination_requires_xch_id`, `silent_payment_keys_not_registered_errors_at_finish`, `round_trip_matches_derive_one_time_puzzle_hash`, `multi_party_hard_errors`
- All builds clean: no-features driver, chip-0057 driver, workspace --all-features
- Scoped clippy `-D warnings` under chip-0057 clean
- `cargo fmt --all --check` clean (after rustfmt re-wrap)
- CLEANUP-01 grep ban still holds on the touched file (0 hits)

## Task Commits

Each task was committed atomically:

1. **Task 1: Precondition — locate both `unreachable!` hits** — verification-only; no commit (zero source edits per plan)
2. **Task 2: Replace the if-let + unreachable!() dispatch with a single exhaustive match per D-04** — `54176195` (refactor)

## Files Created/Modified

- `crates/chia-sdk-driver/src/actions/send.rs` — Dispatch block restructured at lines 41-63 of pre-edit (now lines 41-54). The match handles `SendDestination::PuzzleHash(ph) => *ph` and `#[cfg(feature = "chip-0057")] SendDestination::SilentPayment(addr) => return handle_silent_payment_send(...)`. The `let puzzle_hash = match ...` block now produces `puzzle_hash` for the unchanged Cat/Did/Nft/Option dispatch below.

### Pre-edit block (23 lines, lines 41-63)

```rust
        // chip-0057 SP arm: delegate to the dedicated module so this dispatch
        // stays generic. The helper fires SilentPaymentRequiresXch (Id check)
        // BEFORE the memo-hint guard BEFORE parent reservation.
        #[cfg(feature = "chip-0057")]
        if let SendDestination::SilentPayment(addr) = &self.destination {
            return crate::actions::silent_payment_send::handle_silent_payment_send(
                ctx,
                spends,
                &self.id,
                addr,
                self.amount,
                self.memos,
            );
        }

        // PuzzleHash destination — exhaustive extraction. Under chip-0057 the
        // SilentPayment(_) arm is statically handled above; the early return
        // makes the post-handled match exhaustiveness arm unreachable!().
        let puzzle_hash = match &self.destination {
            SendDestination::PuzzleHash(ph) => *ph,
            #[cfg(feature = "chip-0057")]
            SendDestination::SilentPayment(_) => unreachable!("handled above"),
        };
```

### Post-edit block (14 lines after rustfmt expansion of the 6-arg call)

```rust
        let puzzle_hash = match &self.destination {
            SendDestination::PuzzleHash(ph) => *ph,
            #[cfg(feature = "chip-0057")]
            SendDestination::SilentPayment(addr) => {
                return crate::actions::silent_payment_send::handle_silent_payment_send(
                    ctx,
                    spends,
                    &self.id,
                    addr,
                    self.amount,
                    self.memos,
                );
            }
        };
```

### Pre/post grep oracles

| Grep | Pre-edit | Post-edit |
| ---- | -------- | --------- |
| `grep -c 'unreachable!' send.rs` | 2 | **0** |
| `grep -c 'handle_silent_payment_send' send.rs` | 1 | **1** |
| `grep -cE '<CLEANUP-01 pattern>' send.rs` | 0 | **0** |
| `wc -l send.rs` | 399 | **390** (-9 net after rustfmt) |

## Decisions Made

- **rustfmt expansion accepted as workspace canonical**: the plan's locked D-04 block shows the call args compact on one line; rustfmt re-wrapped them to one-per-line because the line exceeded 100 chars. Same precedent as Plan 08-01 (rustfmt re-wraps multi-line `pub use protocol::{...}`). The grep oracles (the primary POLISH-04 acceptance) both pass; line count is informational only.
- **Two explanatory comment blocks deleted, no replacement comment added**: the new shape's intent is visible from the code itself (SP arm visibly returns; PuzzleHash arm visibly produces `puzzle_hash`). Adding a one-line replacement comment would be redundant.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rustfmt re-wraps the handle_silent_payment_send call to one-arg-per-line**
- **Found during:** Task 2 (post-edit `cargo fmt --all --check`)
- **Issue:** The plan locked the D-04 replacement block with the 6 positional args compact on a single line (`ctx, spends, &self.id, addr, self.amount, self.memos`). After applying the edit, `cargo fmt --all --check` failed — rustfmt insists on expanding the call to one arg per line because the compact form exceeds the workspace 100-col width limit when nested inside an `impl ... { fn ... { let ... = match ... { => { return crate::actions::... }}}}` structure.
- **Fix:** Ran `cargo fmt --all` to accept rustfmt's preferred multi-line shape. The 6 args expand to one per line (6 lines instead of 1), yielding a 14-line post-edit block instead of the plan's anticipated 9-line block. Net line delta: -9 instead of -14.
- **Files modified:** crates/chia-sdk-driver/src/actions/send.rs
- **Verification:** `cargo fmt --all --check` exits 0; grep oracles still pass (`unreachable!`=0, `handle_silent_payment_send`=1); all 14 named tests still pass.
- **Committed in:** 54176195 (part of Task 2 commit)
- **Precedent:** Same disposition as Plan 08-01's pub use rustfmt re-wrap recorded in STATE.md.

---

**Total deviations:** 1 auto-fixed (1 blocking).
**Impact on plan:** rustfmt-canonical shape supersedes the plan's locked verbatim format; behavior and POLISH-04 primary acceptance unaffected. No scope creep.

## Issues Encountered

None beyond the rustfmt re-wrap noted above.

## Risk R4 Resolution

08-RESEARCH.md flagged Risk R4: there are TWO `unreachable!` hits in send.rs — one in a comment (~line 58 referencing the now-deleted `unreachable!()` arm) and one in the match arm (~line 62). Task 1's precondition grep confirmed both hits were at the expected positions inside the POLISH-04-scoped dispatch block; D-04's match restructure deletes BOTH in one atomic edit. No scope expansion was needed.

## Behavior preservation

The Cat/Did/Nft/Option dispatch block past pre-edit line 65 (now post-edit line 56) is byte-identical to pre-edit — the only changes are within the 23-line dispatch block that collapsed to 14 lines:

- Pre-edit `let output = Output::new` at line 65 → post-edit at line 56
- Pre-edit `if matches!(self.id, Id::Xch)` at line 68 → post-edit at line 59

Both shifts are exactly the -9-line net delta. No collateral code changes anywhere else in the file.

## Self-Check: PASSED

Verifications performed after writing this SUMMARY:

- `crates/chia-sdk-driver/src/actions/send.rs` exists (FOUND)
- Commit `54176195` exists on `main` (FOUND via `git log --oneline | grep 54176195`)
- POLISH-04 primary grep `grep -c 'unreachable!' send.rs` = 0 (PASS)
- POLISH-04 second grep `grep -c 'handle_silent_payment_send' send.rs` = 1 (PASS)
- CLEANUP-01 grep ban hits = 0 (PASS)
- All 5 dispatch-touching tests pass (PASS)
- All 9 named send tests from success criteria pass (PASS)
- `cargo fmt --all --check` exits 0 (PASS)
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0 (PASS)
- No-features + chip-0057 + workspace --all-features builds all exit 0 (PASS)

## User Setup Required

None — pure source refactor; no external configuration.

## Next Phase Readiness

- Plan 08-04 (POLISH-02: fold `aggregate.rs` + `input_hash.rs` + `one_time.rs` into `protocol.rs`) is unblocked. POLISH-04 is independent of POLISH-02 per CONTEXT.md's wave assignment guidance.
- Phase 8 is now 3/4 plans complete. Plan 08-04 remains to close the phase.

---
*Phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics*
*Completed: 2026-05-20*
