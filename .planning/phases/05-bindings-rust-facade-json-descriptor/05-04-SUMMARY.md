---
phase: 05-bindings-rust-facade-json-descriptor
plan: 04
subsystem: bindings
tags: [chip-0057, bindy, napi, ava-test, drift-audit, phase-close-out, wave-3]

# Dependency graph
requires:
  - phase: 05-bindings-rust-facade-json-descriptor (Plan 05-03)
    provides: Regenerated napi/index.d.ts + index.js with 10 SP types + 4 SilentPayments statics + Action.send(destination: SendDestination) — the TS surface this plan's AVA tests exercise
provides:
  - 4 AVA tests in napi/__test__/silent_payments.spec.ts (2 closing SC2 address round-trip + 2 closing SC3 SendDestination smoke) — all pass under pnpm test
  - scripts/sp_descriptor_facade_drift.sh — descriptor↔facade drift audit (jq + awk + comm, runs <5s, zero drift)
  - REQUIREMENTS.md BIND-01 + BIND-02 descriptions updated with post-04.2 wording (was referencing the deleted SilentPaymentSend)
  - REQUIREMENTS.md Traceability table rows for BIND-01 + BIND-02 updated
  - STATE.md: progress.completed_phases 6 → 7; completed_plans 34 → 35; percent 50 → 88; Current Position flipped from Phase 5 EXECUTING to Phase 6 Ready; Phase 05-04 + Phase 5 COMPLETE entries added to Accumulated Context
  - ROADMAP.md: Phase 5 line flipped to [x] complete (2026-05-18); progress table row 5 set to 4/4 Complete
  - .planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md — phase close-out with 14-gate matrix, plans-completed table, requirements-closed table, 7 lessons learned, file manifest
  - napi/__test__/action_system.spec.ts: 10 pre-existing Action.send call sites wrapped in SendDestination.puzzleHash(...) per the post-04.2 binding-side signature change
affects:
  - Phase 6 (BIND-03 cross-language E2E + SIM-01..03 simulator round-trip + EX-01 example) — the AVA test pattern + drift audit script + cross-target build infrastructure unblock the Phase 6 work

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "AVA byte-equality assertions on public-key bytes (NOT bech32m string) — survives encoder library churn per RESEARCH Anti-Pattern 5"
    - "Wrapper-struct fallback proven necessary for TS callers: the Rust `From<Bytes32> for SendDestination` ergonomic does NOT translate through bindy; TS callers MUST explicitly wrap raw puzzle-hash bytes via `SendDestination.puzzleHash(bytes)` (this bit Plan 05-04 itself — fixed 10 sites in napi/__test__/action_system.spec.ts)"
    - "Descriptor↔facade drift audit pattern: jq extracts JSON method names per class; awk extracts `pub fn` names per `impl` block; `comm -23` and `comm -13` diff the two sorted lists; non-zero exit on either-direction drift"
    - "AVA --match filter requires `pnpm test -- --match 'pattern'` (pass `--` to forward to the AVA binary) — `pnpm test --match` would be interpreted as a pnpm option"

key-files:
  created:
    - napi/__test__/silent_payments.spec.ts (replaced the Wave-0 stub with 4 production AVA tests; ~110 lines)
    - scripts/sp_descriptor_facade_drift.sh (new; ~75 lines; jq + awk + comm)
    - .planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md (new; phase close-out)
    - .planning/phases/05-bindings-rust-facade-json-descriptor/05-04-SUMMARY.md (this file)
  modified:
    - napi/__test__/action_system.spec.ts (added SendDestination import; wrapped 10 Action.send call sites)
    - .planning/REQUIREMENTS.md (BIND-01 + BIND-02 descriptions + traceability rows)
    - .planning/STATE.md (frontmatter progress + Current Position + Performance Metrics + Accumulated Context + Session Continuity)
    - .planning/ROADMAP.md (Phase 5 [x] complete + progress table row)
    - .planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md (3 Wave-3 rows flipped to ✅ green)

key-decisions:
  - "Wrapped 10 pre-existing Action.send call sites in napi/__test__/action_system.spec.ts in SendDestination.puzzleHash(...) — [Rule 3 - Blocking] inline fix. The post-04.2 binding-side Action.send signature requires SendDestination, not raw Uint8Array. The Rust `From<Bytes32> for SendDestination` does NOT translate through bindy; TS callers must wrap explicitly. Without this, the full pnpm test exits 1 with TS compilation errors and Phase 5 close-out fails."
  - "Drift audit script uses awk to track current `impl ClassName {` block and emit `ClassName::method` for each `pub fn name(` inside — handles all 22 facade methods correctly without per-type special-casing. jq path uses `select(.value.type == \"class\")` to skip enum entries (which have no methods)."
  - "AVA --match flag requires the `pnpm test -- --match 'pattern'` form (passing through `--` to AVA). The plan's verify command (`pnpm test --match 'silent-payment*'`) would be interpreted by pnpm as an unknown option; documented for future replicators."

patterns-established:
  - "Phase-close AVA test pattern: byte-equality on PublicKey.toBytes() for cross-FFI assertions; the only place the AVA test touches the bech32m output is a string-startsWith for the HRP discriminator. Future binding tests for new chip-* surfaces should follow this pattern."
  - "Generated-artifact-driven test rewriting: when a binding signature changes (Action.send: Bytes32 → SendDestination), test files that pass the old shape will fail TS compilation. The fix is mechanical (wrap raw bytes in the factory) but must be done in the same plan that lands the binding change OR explicitly in the next plan; otherwise CI green breaks."

requirements-completed: [BIND-01, BIND-02]
# BIND-01 + BIND-02 were marked [x] by Plan 05-03 in REQUIREMENTS.md (the
# cross-target build verification mechanically satisfies the requirement
# text). This plan REFRESHES the description wording to reflect the
# post-04.2 reality (was referencing the deleted SilentPaymentSend type)
# and adds the AVA functional verification on top. Re-listing here as the
# definitive closing plan.

# Metrics
duration: 7min
completed: 2026-05-18
---

# Phase 05 Plan 04: Wave 3 — AVA tests + drift audit + phase close-out Summary

**4 AVA tests pass (51 total in `pnpm test`), descriptor↔facade drift audit reports zero drift (22 methods on both sides), REQUIREMENTS/STATE/ROADMAP/PHASE-SUMMARY all reflect Phase 5 COMPLETE; phase awaiting user-verification checkpoint.**

## Performance

- **Duration:** 7 min (AVA test run ~22s; drift script <1s; everything else doc writing)
- **Started:** 2026-05-18T01:56:35Z
- **Completed:** 2026-05-18T02:03:13Z
- **Tasks:** 3 of 4 complete (3 autonomous; Task 4 is the user-verify checkpoint — not yet approved)
- **Files modified/created:** 9

## Accomplishments

- Replaced the Wave-0 `test.skip` stub in `napi/__test__/silent_payments.spec.ts` with 4 production AVA tests: 2 closing SC2 (TV1 mainnet + testnet address round-trips via byte-equality on `scanPk.toBytes()` / `spendPk.toBytes()` — NOT bech32m string) + 2 closing SC3 (`SendDestination.silentPayment` factory + `Action.send` composition smoke; `SendDestination.puzzleHash` discriminator round-trip).
- Full `pnpm test` exits 0 with 51 tests passed — 4 new SP tests + 47 pre-existing tests across `action_system.spec.ts`, `cats.spec.ts`, `nfts.spec.ts`, `vaults.spec.ts`, `bulletins.spec.ts`, `clawbacks.spec.ts`, `mips_memos.spec.ts`, `napi.spec.ts`, `options.spec.ts`, `index.spec.ts`.
- Created `scripts/sp_descriptor_facade_drift.sh` (75 lines, jq + awk + comm). Initial run reports zero drift: 22 methods on both sides. The script exits non-zero on either-direction drift (JSON method without facade `pub fn` OR facade `pub fn` without JSON entry).
- Updated `REQUIREMENTS.md` BIND-01 + BIND-02 descriptions with the post-04.2 reality (was referencing the deleted `SilentPaymentSend` type); updated the Traceability table rows to reflect the actual closure path.
- Updated `STATE.md` frontmatter (progress.completed_phases 6 → 7, completed_plans 34 → 35, percent 50 → 88); flipped Current Position from Phase 5 EXECUTING to Phase 6 Ready; added the Phase 05 P04 row to Performance Metrics; appended Phase 05-04 + Phase 5 COMPLETE entries to Accumulated Context > Decisions; updated Session Continuity.
- Updated `ROADMAP.md` Phase 5 line to [x] complete (2026-05-18) + progress table row to 4/4 Complete.
- Created `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md` — phase close-out with 14-gate phase matrix (all PASS), plans-completed table, requirements-closed table, 7 lessons learned, file manifest.
- Flipped the 3 remaining Wave-3 rows in `05-VALIDATION.md` per-task verification map to ✅ green (AVA address round-trip, SendDestination TS construction smoke, descriptor↔facade drift check).

## Task Commits

Each task was committed atomically:

1. **Task 1: 4 AVA tests + action_system.spec.ts wrapping fix** — `1df079ed` (test)
2. **Task 2: scripts/sp_descriptor_facade_drift.sh** — `4af07f1f` (feat)
3. **Task 3: REQUIREMENTS + STATE + ROADMAP + PHASE-SUMMARY close-out** — `c05e1cb6` (docs)

**Task 4: human-verify checkpoint** — pending; agent has paused and returned a structured checkpoint to the orchestrator. NOT auto-approved.

**Plan metadata:** to be added by final_commit step alongside this SUMMARY.

## Files Created/Modified

- `napi/__test__/silent_payments.spec.ts` — replaced Wave-0 `test.skip` stub with 4 production AVA tests
- `napi/__test__/action_system.spec.ts` — added `SendDestination` to imports; wrapped 10 `Action.send(id, wallet.puzzleHash, ...)` call sites in `SendDestination.puzzleHash(...)`
- `scripts/sp_descriptor_facade_drift.sh` — NEW; descriptor↔facade drift audit (jq + awk + comm)
- `.planning/REQUIREMENTS.md` — BIND-01 + BIND-02 descriptions refreshed; traceability rows updated
- `.planning/STATE.md` — frontmatter progress + Current Position + Performance Metrics + Accumulated Context + Session Continuity
- `.planning/ROADMAP.md` — Phase 5 [x] complete + progress table row 5 to 4/4 Complete
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md` — NEW; phase close-out
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md` — 3 Wave-3 rows flipped to ✅ green
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-04-SUMMARY.md` — this file

## Decisions Made

- **Wrap action_system.spec.ts call sites inline ([Rule 3 - Blocking] deviation).** The post-04.2 binding-side `Action.send` signature requires `SendDestination`, not raw `Uint8Array`. The Rust `From<Bytes32> for SendDestination` does NOT translate through bindy — TS callers must wrap explicitly via `SendDestination.puzzleHash(bytes)`. Pre-existing `action_system.spec.ts` had 10 such call sites that failed TS compilation; fixed inline. Without this fix, the full `pnpm test` exits 1 and Phase 5 close-out fails. Documented as breaking-change lesson in PHASE-SUMMARY.md.
- **Drift audit script uses awk to track `impl ClassName {` blocks.** A simpler "grep for `pub fn`" approach would over-match (e.g., bare functions in the module, From-impl methods). Tracking the most recent `impl X {` declaration via awk pairs each `pub fn` with its owning class, correctly handling all 22 facade methods without per-type special-casing.
- **AVA --match flag requires the `pnpm test -- --match 'pattern'` form.** The plan's verify command (`pnpm test --match 'silent-payment*'`) was wrong — pnpm intercepts `--match` as an unknown option. Documented for future replicators; the actual command form is `pnpm test -- --match 'silent-payment*'` (note the `--`).
- **PHASE-SUMMARY.md unicode handling.** Avoided ✅/❌/⚪ Unicode markers in favor of plain "PASS" / "ZERO" labels in the gate matrix table to keep the file LSP-safe and grep-friendly across all platforms. (The plan's template suggested ✅ but the existing repo-pattern PHASE-SUMMARYs vary.)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Wrap 10 Action.send call sites in napi/__test__/action_system.spec.ts**
- **Found during:** Task 1 (initial `pnpm test` after writing silent_payments.spec.ts)
- **Issue:** `action_system.spec.ts` had 10 pre-existing `Action.send(id, wallet.puzzleHash, ...)` call sites passing raw `Uint8Array` as the second arg. Post-04.2 binding-side `Action.send` signature requires `SendDestination`, not Bytes32. TS compilation failed with 10 diagnostic code 2345 errors ("Argument of type 'Uint8Array' is not assignable to parameter of type 'SendDestination'"). The Rust `From<Bytes32> for SendDestination` impl ergonomic does NOT translate through bindy.
- **Fix:** Added `SendDestination` to the imports list; wrapped each of the 10 call sites in `SendDestination.puzzleHash(...)`. Two locations (the XCH-spend test ~line 173 and the CAT-spend test ~line 206), 5 call sites each.
- **Files modified:** `napi/__test__/action_system.spec.ts`
- **Verification:** `cd napi && pnpm test` exits 0 with 51 tests passed (4 new SP + 47 pre-existing including the 4 fixed action_system tests).
- **Committed in:** `1df079ed` (Task 1 commit)

**2. [Rule 1 - Bug] AVA --match flag invocation**
- **Found during:** Task 1 first test-run attempt
- **Issue:** The plan's verify command `cd napi && pnpm test --match 'silent-payment*'` failed with `ERROR Unknown option: 'match'`. pnpm intercepts `--match` as a pnpm option rather than forwarding to AVA.
- **Fix:** Use `pnpm test -- --match 'silent-payment*'` (pass `--` to forward to AVA). Documented in this SUMMARY's patterns-established section so future replicators don't replicate the typo.
- **Files modified:** None (operational fix; the plan's automated verify command was wrong but the test itself is correct).
- **Verification:** `cd napi && pnpm test -- --match 'silent-payment*' --match 'SendDestination*'` exits 0 with 4 SP tests passed.
- **Committed in:** N/A (no source change)

**3. [Rule 1 - Bug] PUZZLE_HASH Buffer<->Uint8Array comparison in SendDestination.puzzleHash test**
- **Found during:** Task 1 (writing the AVA test)
- **Issue:** `dest.asPuzzleHash()` returns napi's `Buffer | null` (per index.d.ts line 2728), but the test was creating a `Uint8Array` for comparison. `t.deepEqual(recovered, PUZZLE_HASH)` would treat Buffer and Uint8Array as different types in some AVA versions.
- **Fix:** Wrap the recovered Buffer in `new Uint8Array(recovered)` for the deepEqual comparison. The plan's template assumed they were structurally compatible; in practice the explicit wrap is safer.
- **Files modified:** `napi/__test__/silent_payments.spec.ts` (during initial write)
- **Verification:** Test passes; the `SendDestination.puzzleHash` round-trip preserves the 32 0x42-bytes correctly.
- **Committed in:** `1df079ed` (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (1 [Rule 3] blocking + 2 [Rule 1] bugs).
**Impact on plan:** Deviation #1 was the only material one — it required ~30 LOC of mechanical wrapping work in `action_system.spec.ts` and adds a lesson to PHASE-SUMMARY.md about binding signature changes being TS-breaking. The other two were verification-command and test-fixture polish. The plan's success criteria are mechanically satisfied; no plan-text changes needed.

## Issues Encountered

- The post-04.2 binding signature change for `Action.send` is a breaking change for TS callers. Pre-existing `action_system.spec.ts` had 10 such call sites that failed TS compilation when run against the Plan 05-03-regenerated `napi/index.d.ts`. Fixed inline (deviation #1). The Rust `From<Bytes32> for SendDestination` ergonomic that preserves 28 existing Rust callers does NOT translate through bindy; TS callers must wrap raw puzzle-hash bytes in `SendDestination.puzzleHash(...)` explicitly. Documented as breaking-change lesson #5 in PHASE-SUMMARY.md so the eventual Sage migration knows to apply the same wrapping.

## User Setup Required

None — no external service configuration required. All work was internal Rust/JSON/TS edits + the new bash script.

## Next Phase Readiness

- **Phase 6 ready to start** after the user approves Task 4's human-verify checkpoint. Phase 6 (Simulator round-trip + bindings E2E + example) scopes:
  - SIM-01: `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` helper exposed through bindings
  - SIM-02 + SIM-03: unlabeled + labeled simulator round-trip tests
  - BIND-03: AVA + pytest + wasm AVA cross-language E2E (full address-gen + send + scan-from-tweaks)
  - EX-01: `examples/silent_payment.rs` runnable against the simulator
- **One thing to remember in Phase 6:** the wasm-pack pin to 0.13.1 must be maintained as long as the workspace stays on rustc 1.90.0. The pyo3 venv lives at `pyo3/.venv/` (gitignored) — either source it (`source pyo3/.venv/bin/activate`) before running maturin or create a fresh one.
- **One thing to remember in Phase 6:** when the cross-language E2E tests in Phase 6 hit the same `Action.send` signature change in Python or WASM test files, the wrapping pattern is the same: explicit `SendDestination.puzzleHash(bytes)` / `SendDestination.silentPayment(addr)` factory call at the TS/Py/WASM caller. The Rust-side `From<Bytes32>` ergonomic does not cross the FFI boundary.
- **One thing to remember always:** any future binding-signature change should run `bash scripts/sp_descriptor_facade_drift.sh` as a pre-commit gate. The script catches descriptor↔facade drift in <1s and would have caught the original Action.send signature mismatch had it existed at the time.

## Self-Check: PASSED

All 9 created/modified files exist on disk with the expected content:

- `napi/__test__/silent_payments.spec.ts` — FOUND (4 named tests; imports include Action, Id, Mnemonic, SendDestination, SilentPaymentAddress, SilentPaymentKeys, SilentPaymentNetwork; asserts via `.toBytes()` byte-equality)
- `napi/__test__/action_system.spec.ts` — FOUND (added SendDestination to imports; 10 call sites wrapped in SendDestination.puzzleHash(...))
- `scripts/sp_descriptor_facade_drift.sh` — FOUND (executable; runs cleanly; reports zero drift, 22 methods on both sides)
- `.planning/REQUIREMENTS.md` — FOUND (BIND-01 + BIND-02 marked [x] with post-04.2 wording; Traceability rows updated)
- `.planning/STATE.md` — FOUND (progress.completed_phases: 7; Current Position: Phase 6; Performance Metrics has P04 row; Accumulated Context has Phase 05 P04 + Phase 5 COMPLETE entries)
- `.planning/ROADMAP.md` — FOUND (Phase 5 [x] complete; progress table row 5: 4/4 Complete)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md` — FOUND (14-gate matrix all PASS; plans-completed table; requirements-closed table; 7 lessons)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md` — FOUND (3 Wave-3 rows flipped to ✅ green)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-04-SUMMARY.md` — FOUND (this file)

All 3 task commits present in git log:

- `1df079ed` (Task 1 — test(05-04): add 4 AVA tests for SP address round-trip + SendDestination smoke) — FOUND
- `4af07f1f` (Task 2 — feat(05-04): add descriptor<->facade drift audit script) — FOUND
- `c05e1cb6` (Task 3 — docs(05-04): close phase 5 in REQUIREMENTS + STATE + ROADMAP + PHASE-SUMMARY) — FOUND

All success criteria from the plan's `<verification>` block pass:

1. `cd napi && pnpm test` — 51 tests passed (4 SP + 47 pre-existing) — PASS
2. `bash scripts/sp_descriptor_facade_drift.sh` — exit 0, "No drift detected (22 methods on both sides)." — PASS
3. `grep -qE '^- \[x\] \*\*BIND-01\*\*' .planning/REQUIREMENTS.md && grep -qE '^- \[x\] \*\*BIND-02\*\*' .planning/REQUIREMENTS.md` — PASS
4. `test -f .planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md` — PASS
5. `grep -q 'Phase: 6' .planning/STATE.md` — PASS

Task 4 (human-verify checkpoint) is the only outstanding item; per plan it requires user approval before phase close — agent will return a structured checkpoint to the orchestrator.

No stubs that would prevent the plan goal — all 4 AVA tests are production tests, the drift script is production tooling, and the docs reflect the actual final state of Phase 5.

---
*Phase: 05-bindings-rust-facade-json-descriptor*
*Completed: 2026-05-18*
