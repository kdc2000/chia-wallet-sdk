---
phase: 09-real-block-tweakdata-bridge-python-relation-binding
plan: 06
subsystem: bindings
tags: [chip-0057, silent-payments, bindings, napi, pyo3, wasm, multi-input, bridge-06, e2e, example]

# Dependency graph
requires:
  - phase: 09-real-block-tweakdata-bridge-python-relation-binding
    provides: "BRIDGE-01 tweak_data_from_block_spends helper (Plan 09-01), BRIDGE-03 Relation opaque-handle binding (Plan 09-03), BRIDGE-04 Spends.prepare(deltas, Option<Relation>) signature extension (Plan 09-04), BRIDGE-05 SilentPayments.tweakDataFromBlockSpends binding (Plan 09-05) — all four entry points exercised end-to-end through three binding targets by this plan's tests."
provides:
  - "Simulator.block_spends(height) + Simulator.block_outputs(height) facade methods on the binding-side Simulator class, callable from napi/pyo3/wasm (matched descriptor entries in bindings/simulator.json)"
  - "Three multi-input cross-binding round-trip tests (one per target) exercising the full Phase 9 surface: napi/__test__/silent_payments_multi_input.spec.ts, wasm/__test__/silent_payments_multi_input.spec.ts, pyo3/tests/test_silent_payments.py::test_multi_input_e2e"
  - "Multi-input section appended to examples/silent_payment.rs (Stages 6-9) demonstrating the canonical Rust-side multi-input flow with tweak_data_from_block_spends + Relation::AssertConcurrent"
affects:
  - "Sage and other downstream wallet consumers — gain a canonical Rust example + cross-binding test fixtures for assembling multi-coin SP sends"
  - "Future CHIP-0058 transport-client implementations — can build TweakData from their wire messages using the now-runtime-tested SilentPayments.tweakDataFromBlockSpends entry point"

# Tech tracking
tech-stack:
  added: []  # zero new workspace deps; zero new crate deps
  patterns:
    - "Cross-binding multi-input round-trip test parity: each AVA / pytest test mirrors the same flow shape (farm 2+ XCH coins → single Action.send → prepare(deltas, Relation.assertConcurrent()) → farm → tweakDataFromBlockSpends(blockSpends, blockOutputs) → scanFromTweaks → assert 1 detection). Establishes the BIND-03-style test scaffolding for any future multi-input cross-binding work."
    - "Sender puzzle-hash equality via Uint8Array bytewise compare (napi + wasm) / `==` (pyo3): chose this over Buffer.compare to keep the test scaffolding identical between napi and wasm. The helper function `bytesEqual` is local-scoped to each spec file (no shared util needed for one test)."
    - "Example demonstrates real-block code path via simulator accessors: examples/silent_payment.rs Stages 7-9 use tweak_data_from_block_spends(sim.block_spends(h), sim.block_outputs(h)) — the same code path real-block callers use after generator decompression. The SDK is simulator-agnostic at this layer."

key-files:
  created:
    - "napi/__test__/silent_payments_multi_input.spec.ts (134 lines, 1 AVA test — BRIDGE-06 napi multi-input round-trip)"
    - "wasm/__test__/silent_payments_multi_input.spec.ts (139 lines, 1 AVA test — BRIDGE-06 wasm multi-input round-trip)"
  modified:
    - "crates/chia-sdk-bindings/src/simulator.rs (+22 lines: block_spends + block_outputs facade methods after tweak_data_from_block, before spend_coins, mirroring the existing facade convention)"
    - "bindings/simulator.json (+12 lines: block_spends + block_outputs descriptor entries appended to Simulator.methods)"
    - "napi/index.d.ts (+2 lines, regenerated: blockSpends(height) + blockOutputs(height) surface on Simulator class)"
    - "pyo3/tests/test_silent_payments.py (+100 lines: test_multi_input_e2e APPENDED — existing test_unlabeled_e2e untouched, diff shows zero deletions)"
    - "examples/silent_payment.rs (+82 lines: Stages 6-9 multi-input section appended after the existing Stage 5 spend loop; module-level rustdoc updated to mention Stages 6-9)"

key-decisions:
  - "Import Relation inside test_multi_input_e2e (`from chia_wallet_sdk import Relation` at function top) rather than modifying the file-level import block. Preserves the hard regression bar from CONTEXT.md acceptance: existing test_unlabeled_e2e source is byte-for-byte unchanged."
  - "Use sender_a / sender_b naming in the example (not sender1 / sender2) to avoid similar_names friction with the existing `sender` variable on Stage 1. The cross-binding tests use sender1 / sender2 since they don't share a scope with another sender variable."
  - "Example uses recipient.scan(...) trait method (not free-fn scan_from_tweaks) — matches the existing Stage 4 invocation style. The trait method is in scope via the umbrella prelude's chip-0057 driver re-export block (Plan 03-05)."
  - "wasm test uses `t.is(detections[0].label, undefined, ...)` not `null` per the Phase 5/6 wasm convention (wasm-bindgen emits Option<u32> as `number | undefined`); napi and pyo3 use `null` / `None`."

patterns-established:
  - "Pattern: Multi-input cross-binding test scaffolding (BIND-04-style). Each AVA / pytest test follows: farm N XCH coins → single Action.send to one destination → spends.prepare(deltas, Relation.assertConcurrent()) → loop pending spends matching each by puzzle hash to its key → farm → tweakDataFromBlockSpends → scanFromTweaks → 1 detection. Future multi-input cross-binding work (CAT2 SP, etc.) can copy this scaffolding."
  - "Pattern: append-only test addition to BIND-03 single-input file (pyo3 pattern). When extending a single-input test file with a multi-input sibling, append the new test function AND import any new dependencies (Relation) inside the new function — preserves the existing test's diff-clean baseline as a regression oracle."

requirements-completed: [BRIDGE-06]

# Metrics
duration: 19min
completed: 2026-05-29
---

# Phase 9 Plan 6: BRIDGE-06 cross-binding multi-input round-trip tests + example multi-input section Summary

**Three cross-binding multi-input round-trip tests (napi/pyo3/wasm) plus a Rust-side multi-input section in examples/silent_payment.rs prove the full Phase 9 surface (BRIDGE-01/03/04/05) works end-to-end from all three binding targets; the Simulator binding gains block_spends + block_outputs facade methods to construct inputs to SilentPayments.tweakDataFromBlockSpends.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-05-29T17:11:08Z
- **Completed:** 2026-05-29T17:30:11Z
- **Tasks:** 3 (3 atomic commits)
- **Files modified:** 5 source files + 1 regenerated `.d.ts` artifact + 2 new test files

## Accomplishments

- **BRIDGE-06 closed.** The full Phase 9 binding surface now has runtime coverage from all three binding targets via three multi-input round-trip tests + a runnable Rust example.
- **`Simulator.blockSpends(height)` + `Simulator.blockOutputs(height)` facade methods** added to `crates/chia-sdk-bindings/src/simulator.rs` with matching descriptor entries in `bindings/simulator.json`. Two ~5-line methods mirroring the existing `tweak_data_from_block` precedent; thin delegating wrappers around `chia_sdk_test::Simulator::block_spends/block_outputs`. Regenerated `napi/index.d.ts` surfaces both methods on the Simulator class.
- **Three new multi-input tests pass on first run:** napi 53/53 (52 existing + new), wasm 9/9 (8 existing + new), pyo3 2/2 (existing test_unlabeled_e2e + new test_multi_input_e2e). Each test drives 2 XCH coins → single `Action.send` to one SP address → `Spends.prepare(deltas, Relation.assertConcurrent())` → farm → `SilentPayments.tweakDataFromBlockSpends(sim.blockSpends(h), sim.blockOutputs(h))` → scan → asserts exactly 1 detection with amount=700.
- **Hard regression bar PASSES:** existing `pyo3/tests/test_silent_payments.py::test_unlabeled_e2e` passes unchanged. `git diff` on the file shows zero deletions — the new test is purely appended. The Relation import lives inside `test_multi_input_e2e` to preserve the existing test's file-level imports.
- **Phase 6 e2e oracle still byte-identical:** `cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e` → 3/3 passed (test_simulator_e2e_unlabeled, test_simulator_e2e_labeled, test_simulator_e2e_m0_self_change). The simulator helper's logic remains canonical post-BRIDGE-02 refactor.
- **examples/silent_payment.rs extended** with a 4-stage multi-input section (Stages 6-9) demonstrating `tweak_data_from_block_spends` + `Relation::AssertConcurrent`. Example builds + runs end-to-end under `--all-features`, printing all 9 stages.
- **Descriptor↔facade drift script clean:** `scripts/sp_descriptor_facade_drift.sh` reports 23 methods on both sides (unchanged — drift script only audits SP-namespace methods; Simulator additions live in `bindings/simulator.json`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Simulator.block_spends + block_outputs binding facade additions + descriptor entries** — `3dfe4e58` (feat)
2. **Task 2: Three multi-input cross-binding round-trip tests (napi + wasm + pyo3)** — `1f445a3a` (test)
3. **Task 3: Multi-input section appended to examples/silent_payment.rs (Stages 6-9)** — `43b71c51` (feat)

**Plan metadata:** _appended after SUMMARY landing via the standard `docs({phase}-{plan})` commit._

## Files Created/Modified

- `crates/chia-sdk-bindings/src/simulator.rs` — appended `block_spends(&self, height: u32) -> Result<Vec<CoinSpend>>` and `block_outputs(&self, height: u32) -> Result<Vec<Coin>>` methods on the `impl Simulator` block (placed between `tweak_data_from_block` and `spend_coins` per the plan's locked insertion point).
- `bindings/simulator.json` — appended `Simulator.methods.block_spends` and `Simulator.methods.block_outputs` entries with `args: { height: "u32" }` and matching `Vec<CoinSpend>` / `Vec<Coin>` returns.
- `napi/index.d.ts` — regenerated by `pnpm build`; surfaces `blockSpends(height: number): Array<CoinSpend>` and `blockOutputs(height: number): Array<Coin>` on the `Simulator` class definition (lines 2849-2850).
- `napi/__test__/silent_payments_multi_input.spec.ts` — NEW. 1 AVA test mirroring `silent_payments_e2e.spec.ts` scaffold; uses bytewise Uint8Array compare to match pending coins to their secret keys.
- `wasm/__test__/silent_payments_multi_input.spec.ts` — NEW. 1 AVA test mirroring the napi shape with imports from `../pkg` and `setPanicHook()` at module load.
- `pyo3/tests/test_silent_payments.py` — APPENDED `test_multi_input_e2e`; existing `test_unlabeled_e2e` is byte-for-byte unchanged. Import of `Relation` lives inside the new test function (no edits to the file-level import block).
- `examples/silent_payment.rs` — appended Stages 6-9 (~82 lines) demonstrating the multi-input flow: 2 sender BLS pairs → single Action.send for 700 mojos → `spends_multi.finish_with_keys(ctx, &deltas_multi, Relation::AssertConcurrent, &multi_pks)` → `tweak_data_from_block_spends(sim.block_spends(h), sim.block_outputs(h))` → scan → spend. Module-level rustdoc updated to mention Stages 6-9.

## Test Runtime (Pitfall 3 oracle)

- napi multi-input test: ~150ms (no explicit `(Xms)` annotation reported by AVA, meaning < 100ms typical AVA threshold for visibility — well within expected range; whole suite 53 tests in ~3s)
- wasm multi-input test: ~150ms (same AVA behavior; whole suite 9 tests in ~2.5s)
- pyo3 multi-input test: ~30ms (whole pytest run 0.06s for 2 tests)

All multi-input tests run faster than the BIND-03 single-input tests' baseline — no Pitfall 3 timeout pressure.

## Diff-Clean Regression Oracle (pyo3 hard bar)

```
$ git diff -- pyo3/tests/test_silent_payments.py | grep -c '^-[^-]'
0
```

Zero deletions = the existing `test_unlabeled_e2e` source is byte-for-byte unchanged. The new `test_multi_input_e2e` is pure addition. This satisfies the CONTEXT.md hard regression bar: "Existing Python E2E stays green with no signature changes beyond the optional `Relation` parameter."

## Decisions Made

- **Import `Relation` inside the new pyo3 test function** rather than modifying the file-level import block. The plan's skeleton anticipates this pattern (`from chia_wallet_sdk import Relation  # BRIDGE-03 — new binding`) and it preserves the existing test's diff-clean baseline.
- **Use `bytesEqual(a, b)` local helper for puzzle-hash comparison** in napi + wasm tests rather than `Buffer.compare(...) === 0`. The helper is local-scoped to each spec file (no shared util needed). Same shape in both napi and wasm scaffolding to keep them visually parallel.
- **wasm `t.is(detections[0].label, undefined, ...)` not `null`** — wasm-bindgen emits `Option<u32>` as `number | undefined` (per Phase 5/6 convention captured in the existing wasm single-input test).
- **Example uses `sender_a / sender_b` naming** (not `sender1 / sender2`) to avoid clippy::similar_names friction with the existing Stage 1 `sender` variable; the cross-binding tests use `sender1 / sender2` since they don't share scope with another `sender`.

## Deviations from Plan

None — plan executed exactly as written. Zero inline auto-fixes; zero blockers; zero clippy fights on the new code.

The plan-anticipated `, undefined` / `, None` insertions in existing BIND-03 single-input tests were NOT needed — Plan 09-04's BRIDGE-04 bindy-macro patch already ensured that `spends.prepare(deltas)` 1-arg calls remained ergonomic on all three targets without any test-source patches.

## Issues Encountered

None during this plan.

The single pre-existing chia-sdk-daemon clippy warning (match_wildcard_for_single_variants on client.rs:427) surfaces on `cargo clippy --workspace --all-features --all-targets -- -D warnings` but is verified pre-existing on main (same disposition as Phase 1/6/7 — documented in deferred-items.md, excluded from CI's clippy step which doesn't use `-D warnings`). Touched-crate-scoped clippy (`-p chia-sdk-bindings -p chia-sdk-driver -p chia-sdk-test --all-targets -- -D warnings`) is clean.

## User Setup Required

None — no external service configuration required. All work is binding-layer code, test scaffolding, and example code.

## Next Phase Readiness

- **BRIDGE-06 closed.** All 6 BRIDGE-* requirements (BRIDGE-01..06) now complete. Phase 9 surface fully ships.
- **Phase 9 verification-ready.** All four success criteria from the plan satisfied:
  - napi suite passes (53 existing + 1 new multi-input)
  - wasm suite passes (8 existing + 1 new multi-input)
  - pyo3 suite passes (existing test_unlabeled_e2e + new test_multi_input_e2e)
  - examples/silent_payment.rs builds AND runs successfully under `--all-features` printing all 9 stages
- **Drift script + machete clean.** Descriptor↔facade audit reports 23 methods on both sides; cargo-machete finds no unused deps.
- **CLEANUP-01 honored.** Zero planning-artifact refs in source/test/example code (verified by grep across all 5 modified files + 2 new files).

## Self-Check: PASSED

All claimed files exist on disk:
- `crates/chia-sdk-bindings/src/simulator.rs` — FOUND (`grep -q 'pub fn block_spends' && grep -q 'pub fn block_outputs'` both pass)
- `bindings/simulator.json` — FOUND (`grep -q '"block_spends":' && grep -q '"block_outputs":'` both pass; `python3 -m json.tool` passes)
- `napi/index.d.ts` — FOUND (`grep -q 'blockSpends' && grep -q 'blockOutputs'` both pass)
- `napi/__test__/silent_payments_multi_input.spec.ts` — FOUND
- `wasm/__test__/silent_payments_multi_input.spec.ts` — FOUND
- `pyo3/tests/test_silent_payments.py` — FOUND (`grep -q 'def test_multi_input_e2e'` passes)
- `examples/silent_payment.rs` — FOUND (`grep -q 'Stage 6/9'` + `grep -q 'Relation::AssertConcurrent'` + `grep -q 'tweak_data_from_block_spends'` all pass; `grep -c 'sim.bls('` returns 3)
- `.planning/phases/09-real-block-tweakdata-bridge-python-relation-binding/09-06-SUMMARY.md` — FOUND (this file)

Commits verified in `git log --oneline`:
- `3dfe4e58` (feat(09-06): expose Simulator.block_spends + block_outputs in binding facade) — FOUND
- `1f445a3a` (test(09-06): land BRIDGE-06 multi-input cross-binding tests) — FOUND
- `43b71c51` (feat(09-06): add multi-input section to examples/silent_payment.rs) — FOUND

CLEANUP-01 acceptance grep across new test files + modified example: zero planning-artifact references.

Drift script: `bash scripts/sp_descriptor_facade_drift.sh` → "No drift detected (23 methods on both sides)."

Hard regression bar: `git diff -- pyo3/tests/test_silent_payments.py | grep -c '^-[^-]'` → 0 (zero deletions).

---
*Phase: 09-real-block-tweakdata-bridge-python-relation-binding*
*Completed: 2026-05-29*
