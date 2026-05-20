---
phase: 07-code-review-cleanup
plan: 03
subsystem: action-system
tags: [chip-0057, silent-payments, action-system, spends, prepare, bindings, refactor]

# Dependency graph
requires:
  - phase: 04-send-side-action
    provides: chip-0057 SP send pipeline through Spends::finish_with_keys + sp_finish_branch helper
  - phase: 04.2-action-api-unification
    provides: SendDestination::SilentPayment + Spends::with_silent_payment_keys + Spends prep/finish surface
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: chia-sdk-bindings::Spends::prepare wrapper (Phase 6 Plan 04 added explicit finish_silent_payments call as stopgap)
provides:
  - "Spends::prepare runs the chip-0057 SP branch internally — no caller has to remember to call a separate finish method"
  - "Spends::finish_silent_payments deleted from chia_sdk_driver::Spends public surface (one fewer load-bearing public method)"
  - "Binding-side Spends::prepare wrapper no longer makes the explicit finish_silent_payments call (4 lines deleted)"
  - "Spends::finish_with_keys signature simplified from mut self to self; cfg_attr(unused_mut) attribute removed"
affects: [phase-04 SP send callers, phase-05 bindings consumers, phase-06 cross-language E2E tests, future v2 SP work]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Internal-sequencing-invariant elimination: when a public method's documented usage pattern requires a strict ordering with another method (here: 'call finish_silent_payments before prepare'), the type system enforces correctness better than rustdoc — push the dependent call into the parent method body and delete the public surface that exposed the invariant."

key-files:
  created: []
  modified:
    - "crates/chia-sdk-driver/src/action_system/spends.rs (+15/-51 lines): finish_silent_payments removed; prepare gains SP branch at top; finish_with_keys loses inline SP branch + uses `self` not `mut self`"
    - "crates/chia-sdk-bindings/src/action_system.rs (+3/-11 lines): binding-side Spends::prepare wrapper no longer calls finish_silent_payments; rustdoc on with_silent_payment_keys updated to reflect new internal location; spurious `let mut spends` rebinding dropped"

key-decisions:
  - "Per D-02 (CLEANUP-03): finish_silent_payments was a Phase 6 stopgap exposing an internal sequencing invariant — pushing the branch into prepare itself eliminates the invariant entirely. The type system now enforces correctness."
  - "Spends::prepare runs the chip-0057 SP branch BEFORE create_change/emit_conditions so CreateCoin conditions emitted by sp_finish_branch land on parents' payment_assertions before emit_conditions consumes them (ordering preserved from finish_with_keys' previous inline placement)."
  - "Rule 1 inline fix in chia-sdk-bindings/src/action_system.rs: dropped now-unused `mut` on line 181's local `spends` binding — was needed only when finish_silent_payments(&mut spends, ...) borrowed mutably; with that call removed, spends.prepare(...) consumes by value and the `mut` triggered #[warn(unused_mut)]."
  - "Workspace --all-features build was transiently broken between Task 1 and Task 2 commits (Task 1 deleted finish_silent_payments while chia-sdk-bindings still called it); per-crate chia-sdk-driver build + driver test suite (1088 tests) verified green before Task 1 commit; Task 2 commit re-greens the full workspace build. Same precedent as Plan 04.1-01 ('transient test-build break' deviation logged in STATE.md)."

patterns-established:
  - "Internal-sequencing-invariant elimination via parent-method absorption: when method B must always be called before method A on the same receiver, pushing B's body into A's prologue (with a guard for B's no-op case) removes the documentation burden and the maintainer footgun."

requirements-completed: [CLEANUP-03]

# Metrics
duration: 21min
completed: 2026-05-20
---

# Phase 7 Plan 03: CLEANUP-03 (push chip-0057 SP branch into Spends::prepare) Summary

**Pushed the chip-0057 SP finish branch into `Spends::prepare` itself; deleted the leaky `Spends::finish_silent_payments` public method that Phase 6 Plan 04 added as a binding-side stopgap.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-05-20T15:57:00Z
- **Completed:** 2026-05-20T16:18:38Z
- **Tasks:** 2/2
- **Files modified:** 2

## Accomplishments

- `Spends::prepare` (chia_sdk_driver) now runs the chip-0057 SP branch automatically when `silent_payments_pending` is non-empty — eliminating the "must call X before Y" invariant that Phase 6's `finish_silent_payments` stopgap exposed.
- `Spends::finish_silent_payments` deleted entirely (method + 23-line rustdoc). Public method surface on `chia_sdk_driver::Spends` shrinks by one.
- `Spends::finish_with_keys` no longer carries the duplicated inline SP-branch block; signature simplified from `mut self` to `self`; `cfg_attr(unused_mut)` attribute dropped.
- Binding-side `Spends::prepare` wrapper (in `chia-sdk-bindings/src/action_system.rs`) no longer makes the explicit `finish_silent_payments(&mut ctx, Relation::None)?` call — the SP branch fires implicitly through `sdk::Spends::prepare`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Refactor `Spends::prepare` to run sp_finish_branch internally; delete `finish_silent_payments`; clean `finish_with_keys`** — `386222d4` (refactor)
2. **Task 2: Remove the binding-side `finish_silent_payments` call and re-verify cross-language tests** — `2934e69b` (refactor)

**Plan metadata commit:** pending (will land with the SUMMARY.md/STATE.md/ROADMAP.md docs commit at end of executor flow).

## Files Created/Modified

- `crates/chia-sdk-driver/src/action_system/spends.rs` — Net **-36 lines** (+15/-51). Removed: `finish_silent_payments` (10-line method + 28-line rustdoc), inline SP branch in `finish_with_keys` (7 lines including comment header + #[cfg] + if-block), `cfg_attr(not(feature = "chip-0057"), allow(unused_mut))` (1 line), `mut` on `self` parameter (1 char). Added: 8-line SP branch (3-line comment + #[cfg] + 4-line if-block) at top of `prepare`'s body; one-word doc tweak in `finish_with_keys`'s rustdoc to point at `Spends::prepare` as the new SP-branch host. **`sp_finish_branch` private free fn UNCHANGED** — same signature, same body; only its callers changed.
- `crates/chia-sdk-bindings/src/action_system.rs` — Net **-8 lines** (+3/-11). Removed: 4-line call + comment block (`spends.finish_silent_payments(&mut ctx, Relation::None)?;` + 3 comment lines documenting why), rustdoc parenthetical referencing `Spends::finish_with_keys` (2 lines replaced with 1 line referencing `Spends::prepare`), `mut` modifier on local `spends` binding (1 char trigger for unused_mut warning). `Relation` import preserved — still used by `Relation::None` argument at surviving `spends.prepare(...)` call.

## Decisions Made

Followed the plan precisely. Two minor implementation-level details captured:

- **Doc-tweak in `finish_with_keys` rustdoc** (Task 1, not explicitly called out in plan Step 3 but flagged by the pre-edit rustdoc-coherence audit): the existing rustdoc said "a chip-0057 SP branch runs BEFORE `prepare()` so the derived `CreateCoin` conditions feed into the parents' `payment_assertions` before `emit_conditions`". That parenthetical "BEFORE `prepare()`" became stale once the branch moved into `prepare`'s body. Rewrote to "the chip-0057 SP branch runs inside [`Spends::prepare`] (called below)". Privacy-warning content otherwise preserved verbatim.
- **Doc-tweak on `with_silent_payment_keys` in chia-sdk-bindings/src/action_system.rs** (Task 2 Step 2): the pre-edit rustdoc said "the chip-0057 branch of `Spends::prepare` (in `chia_sdk_driver::Spends::finish_with_keys`) can derive each pending one-time puzzle hash". The parenthetical was already misleading post-Plan-07-03 because the branch now runs inside `chia_sdk_driver::Spends::prepare`, not `finish_with_keys`. Rewrote to "the chip-0057 branch that runs inside `chia_sdk_driver::Spends::prepare` can derive each pending one-time puzzle hash."

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Drop now-unused `mut` on local `spends` binding in binding's Spends::prepare wrapper**
- **Found during:** Task 2 (Step 4: chia-sdk-bindings build verification)
- **Issue:** After deleting the `spends.finish_silent_payments(&mut ctx, Relation::None)?;` call, the local binding `let mut spends = std::mem::replace(&mut *spends, sdk::Spends::new(change_puzzle_hash));` at line 181 no longer needed `mut` — the only remaining usage `spends.prepare(...)` consumes by value. Compile-warning `#[warn(unused_mut)]` fired (workspace lint policy: `unused_mut` warns; with `-D warnings` it would deny).
- **Fix:** Changed `let mut spends = std::mem::replace(...)` to `let spends = std::mem::replace(...)`. The outer `let mut spends = self.spends.lock().unwrap();` at line 178 still needs `mut` because `mem::replace(&mut *spends, ...)` requires the mutable borrow.
- **Files modified:** crates/chia-sdk-bindings/src/action_system.rs (line 181)
- **Verification:** `cargo build -p chia-sdk-bindings --all-features` exits 0 with zero warnings post-fix; clippy -D warnings clean.
- **Committed in:** `2934e69b` (Task 2 commit).

**2. [Rule 3 - Blocking] Transient workspace --all-features build break between Task 1 and Task 2**
- **Found during:** Task 1 verify step (`cargo build --release --workspace --all-features` listed in plan's automated verify command).
- **Issue:** Task 1 deletes `Spends::finish_silent_payments` from chia-sdk-driver. Until Task 2 lands, chia-sdk-bindings at line 191 still calls `spends.finish_silent_payments(&mut ctx, Relation::None)?` which now fails to compile (E0599). This is a known cross-task atomic-deletion pattern (see STATE.md "Plan 04.1-01 deviation Rule 3" precedent: "Committed Task 1 with non-test library build verified + grep gates green; documented in commit").
- **Fix:** Committed Task 1 with the per-crate chia-sdk-driver build + driver test suite (1088 tests) verified green; documented the transient state in the Task 1 commit message; immediately landed Task 2 to re-green the workspace build. Verified post-Task-2: `cargo build --release --workspace --all-features` exits 0; full CI-parity workspace test suite passes (2335 driver + 7 + 26 + 24 + 32 + 2 + 2 doc tests).
- **Files modified:** None additional (the resolution IS Task 2's planned edit; this deviation is the bookkeeping note about the transient state, not a code change).
- **Verification:** Both grep oracles green post-Task-2; workspace build green; all 3 binding test suites green (napi 52/52, wasm 8/8, pyo3 2/2).
- **Committed in:** N/A — flagged in Task 1 commit message body; resolved by Task 2 commit `2934e69b`.

---

**Total deviations:** 2 auto-fixed (1 Rule-1 bug, 1 Rule-3 blocking — both anticipated by the plan's Pitfall section and Plan-04.1-01 precedent).
**Impact on plan:** Both deviations are bookkeeping/cleanup that flowed mechanically from the plan's planned edits. No scope creep; no semantic change. All 4 `Spends::prepare` callsites (per RESEARCH §1.3 line 392 audit) verified working.

## Issues Encountered

- **Pre-existing chia-sdk-daemon clippy warnings re-surfaced under `cargo clippy --workspace --all-features --all-targets -- -D warnings`** (`match_wildcard_for_single_variants` and `match_same_arms` in client.rs:426-427). Verified pre-existing on `main` (pre-Task-1) via `git stash` + clippy invocation. Per STATE.md Plan 01-05 decision: "chia-sdk-daemon clippy::pedantic lints (client.rs:426-427) are pre-existing upstream warnings — out of Phase 1 scope, logged to deferred-items.md, NOT auto-fixed. Scoped clippy on chia-sdk-types is clean with -D warnings; CI's existing clippy step (no -D warnings) exits 0." Same disposition applied here — out of plan scope, no fix attempted.
- **Pre-existing chia-sdk-driver no-features `missing_copy_implementations` warning on `SendDestination`**: when `chip-0057` is OFF, the enum cfg-strips down to a single variant `PuzzleHash(Bytes32)` (Copy-eligible), tripping the workspace lint. Verified pre-existing on `main`. Same disposition: out of plan scope (would require an `#[allow(missing_copy_implementations)]` attribute or restructuring of `SendDestination`, neither of which CLEANUP-03 mandates).

## Verification Matrix

| Check | Result |
|-------|--------|
| `grep -c 'pub fn finish_silent_payments\b' crates/chia-sdk-driver/src/action_system/spends.rs` | 0 (PASS) |
| `grep -c 'finish_silent_payments' crates/chia-sdk-bindings/src/action_system.rs` | 0 (PASS) |
| Exactly one `if !self.silent_payments_pending.is_empty()` block in spends.rs (inside `prepare`) | PASS |
| `Spends::finish_with_keys` signature uses `self,` not `mut self,` | PASS |
| `cargo build --release -p chia-sdk-driver --features chip-0057` | exit 0 |
| `cargo build --release -p chia-sdk-driver` (no features) | exit 0 (1 pre-existing warning) |
| `cargo build --release --workspace --all-features` | exit 0 (post-Task-2) |
| `cargo build -p chia-sdk-bindings --all-features` | exit 0 (no warnings post-Rule-1-fix) |
| `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` | exit 0 |
| `cargo test --release -p chia-sdk-driver --features chip-0057 round_trip_matches_derive_one_time_puzzle_hash` | 1/1 PASS (SEND-04 round-trip) |
| `cargo test --release -p chia-sdk-driver --features chip-0057 assert_concurrent_relation_emits_cycle` | 3/3 PASS (callsite #2 in audit, non-SP prepare exercise) |
| `cargo test --release -p chia-sdk-driver --features chip-0057` (full driver suite) | 1088/1088 PASS |
| `cargo test --release --workspace --all-features` (CI-parity exclusions) | 2335 driver + 7 client + 26 daemon + 24 + 32 + 2 + 2 doc tests = all PASS |
| `cargo fmt --all --check` | clean (no diff) |
| `cargo machete` | clean (zero new ignored entries) |
| `cd napi && pnpm test` | 52/52 PASS (incl. 4 silent_payments + 1 silent_payments_e2e BIND-03) |
| `cd wasm && pnpm test` | 8/8 PASS (incl. silent_payments BIND-03 wasm E2E) |
| `cd pyo3 && pytest` | 2/2 PASS (incl. test_silent_payments.py) |
| All 4 `Spends::prepare` callsites verified (RESEARCH §1.3 line 392 audit) | PASS (callsites 1+4 deleted/rewritten; 2+3 still pass tests) |
| `Relation` import in chia-sdk-bindings/src/action_system.rs preserved | PASS (still used by `Relation::None` at surviving `prepare(...)` call) |

## Next Phase Readiness

- **Plan 07-04** (next in queue per `incomplete_plans` from init context) unblocked — CLEANUP-03 carries no forward-impacting state changes that would block subsequent CLEANUP-NN plans.
- **Cross-language SP send/scan flow** continues to work end-to-end across napi/pyo3/wasm — first-pass behavioral parity confirmed via the 3 cross-language E2E tests originally introduced in Phase 6 Plan 04 (BIND-03).

## Self-Check: PASSED

- FOUND: `.planning/phases/07-code-review-cleanup/07-03-SUMMARY.md`
- FOUND: commit `386222d4` (Task 1: refactor spends.rs)
- FOUND: commit `2934e69b` (Task 2: drop binding-side call)

---
*Phase: 07-code-review-cleanup*
*Completed: 2026-05-20*
