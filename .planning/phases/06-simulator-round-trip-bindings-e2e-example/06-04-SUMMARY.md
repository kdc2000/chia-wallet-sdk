---
phase: 06-simulator-round-trip-bindings-e2e-example
plan: 04
subsystem: chia-sdk-bindings
tags: [chip-0057, silent-payments, bind-03, ffi, napi, pyo3, wasm, vec-publickey-marshaling, simulator]

# Dependency graph
requires:
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-01 chip-0057 feature wiring on chia-sdk-test (cascade to chia-sdk-driver + chia-sdk-utils + chia-sdk-types)"
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-02 tweak_data_from_simulator_block helper (chip-0057-gated free fn in chia_sdk_test::silent_payments) — the function the new facade wraps"
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-03 Rust E2E tests proving the unlabeled flow works end-to-end at SDK level — confirms the surface this plan bridges to TS/Py is functionally complete"
  - phase: 05-bindings-rust-facade-json-descriptor
    provides: "11-type SP bindings surface (Phase 5 Plan 05-02): SilentPaymentAddress + SilentPaymentKeys + LabelRegistry + SilentPayments static-methods + SendDestination + SilentPaymentRegistered{Key,SecretKey} wrappers + Spends.with_silent_payment_keys"
  - phase: 04.2-unify-sp-send-into-action-send-via-senddestination-enum
    provides: "Unified Action::send(id, destination, amount, memos) + SendDestination::SilentPayment(Box<...>) — the bindings-side Action.send takes SendDestination directly"
provides:
  - "Simulator.tweakDataFromBlock(height) -> TweakData reachable from TypeScript (napi + wasm) AND Python (pyo3) — D-02 closure"
  - "One descriptor entry in bindings/simulator.json (canonical sync-method shape: `{ args: { height: u32 }, return: TweakData }`)"
  - "One Arc<Mutex<>>-dispatch facade method in crates/chia-sdk-bindings/src/simulator.rs (unconditional, no cfg-gates per D-03)"
  - "chia-sdk-bindings transitively enables chip-0057 on chia-sdk-test (D-03; mirrors driver+utils+types wiring)"
  - "3 cross-language E2E tests: napi AVA (silent_payments_e2e.spec.ts), pyo3 pytest (test_silent_payments.py::test_unlabeled_e2e), wasm AVA (silent_payments.spec.ts) — all 3 PASS"
  - "First runtime proof that Vec<chia_bls::PublicKey> on TweakData.tweakPoints marshals correctly across napi + pyo3 + wasm FFI boundaries (Phase 5 Q1 was compile-time only; this is runtime)"
  - "chia_sdk_driver::Spends::finish_silent_payments public chip-0057-gated method — Rule 2 missing-critical fix that exposes sp_finish_branch composition so bindings layer's prepare() can run the SP branch (without which the binding-side SP send is silently broken)"
  - "chia_sdk_bindings::Spends::prepare wired to call finish_silent_payments before sdk::Spends::prepare, mirroring sdk::Spends::finish_with_keys ordering"
  - "BIND-03 closure — all three cross-language tests complete the full send → farm → extract → scan → SPEND round-trip from their respective host languages"
affects: [06-05, EX-01]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Simulator-facade method pattern: Arc<Mutex<>>-dispatched binding method that locks once, delegates to chia_sdk_test free-fn, and converts the returned driver-side TweakData to the binding-facade TweakData via the existing From impl in crates/chia-sdk-bindings/src/silent_payments.rs (no double-conversion, no allocator threading)."
    - "Cross-language E2E test rhythm: mnemonic → SilentPaymentKeys.fromMnemonic → unlabeled_address → Spends.addXch + Action.send + withSilentPaymentKeys → spends.apply → spends.prepare → standardSpend the input → sim.spendCoins → sim.tweakDataFromBlock → SilentPayments.scanFromTweaks → assert detections.length === 1 + label is None + amount round-trips → deriveSynthetic → follow-on standardSpend → assert detected coin spent. Identical logic across napi/pyo3/wasm; only language-specific syntax (camelCase vs snake_case, undefined vs None) differs."
    - "Binding `Spends.prepare` SP-aware composition: prepare() now calls `finish_silent_payments(ctx, Relation::None)` before `sdk::Spends::prepare`, then iterates `unspent()` for Settlement spends. The new method is a no-op when no SP send is pending, so non-SP flows are unaffected. This mirrors the ordering inside `sdk::Spends::finish_with_keys` (the SP branch must run BEFORE prepare so CreateCoin conditions land on parents' payment_assertions before emit_conditions)."
    - "Pyo3 test environment: pytest installed inside pyo3/.venv via `pip install pytest` (Phase 5 wired maturin/.venv but not pytest). `python -m pytest tests/test_silent_payments.py::test_unlabeled_e2e` runs from the venv. No conftest.py per D-08."

key-files:
  created:
    - "napi/__test__/silent_payments_e2e.spec.ts"
    - "pyo3/tests/test_silent_payments.py"
    - "wasm/__test__/silent_payments.spec.ts"
    - ".planning/phases/06-simulator-round-trip-bindings-e2e-example/06-04-SUMMARY.md"
  modified:
    - "crates/chia-sdk-bindings/Cargo.toml"
    - "crates/chia-sdk-bindings/src/simulator.rs"
    - "crates/chia-sdk-bindings/src/action_system.rs"
    - "crates/chia-sdk-driver/src/action_system/spends.rs"
    - "bindings/simulator.json"
    - "napi/index.d.ts"

key-decisions:
  - "D-01 honored: cross-language tests obtain TweakData via the bindings facade (`Simulator.tweakDataFromBlock`) — NOT a JSON fixture, NOT synthetic construction. The TweakData object is built on the Rust side and crosses the FFI boundary into the host language, giving BIND-03 the strongest FFI-fidelity claim."
  - "D-02 honored: method lives on the `Simulator` class as `simulator.tweakDataFromBlock(height)` (TS) / `simulator.tweak_data_from_block(height)` (Python). The free-fn `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` remains the canonical Rust entry; the binding facade wraps it with a method on the existing Simulator binding (one descriptor entry, one facade method)."
  - "D-03 honored: chia-sdk-bindings transitively enables chip-0057 on chia-sdk-test (mirrors Phase 5's wiring on driver+utils+types). NO `#[cfg(feature = \"chip-0057\")]` gates inside the facade — `grep -c 'cfg(feature = \"chip-0057\"' crates/chia-sdk-bindings/src/simulator.rs` returns 0."
  - "D-07 honored: only the unlabeled flow is exercised cross-language; labeled coverage stays Rust-only (Plan 06-03's test_simulator_e2e_labeled)."
  - "D-08 honored: ONE pytest file (pyo3/tests/test_silent_payments.py), NO conftest.py."
  - "[DEVIATION] Discovered + fixed Rule-2 missing-critical functionality: `chia_sdk_bindings::Spends::prepare` did NOT run the chip-0057 SP finish branch (sp_finish_branch in chia-sdk-driver is private; only `sdk::Spends::finish_with_keys` calls it). Without this, the binding-side SP send completes apply but never emits the recipient's CreateCoin onto the parent's payment_assertions — so the simulator never farms the SP coin and the scanner finds zero detections. Fix: added `pub fn finish_silent_payments(&mut self, ctx, relation) -> Result<()>` to `chia_sdk_driver::Spends` (chip-0057 gated; no-op when no SP pending) and wired binding's prepare to call it BEFORE sdk::Spends::prepare. Rust-side `finish_with_keys` is unchanged (still calls sp_finish_branch internally). This is a forward-looking value addition: any caller wanting `prepare` semantics (returning Spends<Finished> rather than Outputs) but with SP support now has a clean public hook."

patterns-established:
  - "Bindings-layer SP composition pattern: when an SDK function bundles SP processing into a finish-and-spend pipeline that the bindings need to split (because bindings return a Spends<Finished> for callers to insert standard puzzles, not Outputs), the SDK exposes a public hook (`finish_silent_payments`) for the SP branch and bindings invoke it at the appropriate point in their own pipeline. Avoids reimplementing sp_finish_branch in the bindings; keeps the canonical algorithm in one place."
  - "Cross-language test fixture consistency: BIP-39 TV1 mnemonic + Testnet network + 100-mojo amount + same scanner k_max (2400) across napi/pyo3/wasm tests. Same Simulator default seed (1337). Each test runs `new Simulator()` fresh — no shared state — and asserts the exact same outcomes (detections.length === 1, label === None, k === 0, amount === 100, follow-on spend succeeds). Pinning the fixture means a regression in any single binding target produces a localized failure, not a mass false-positive."

requirements-completed: [BIND-03]

# Metrics
duration: 29min
completed: 2026-05-19
---

# Phase 06 Plan 04: BIND-03 Cross-Language SP E2E Summary

**Three cross-language E2E tests (napi AVA, pyo3 pytest, wasm AVA) drive the full unlabeled silent-payment send → farm → extract → scan → SPEND round-trip via a one-method-and-one-descriptor-entry expansion of the existing `Simulator` binding (`Simulator.tweakDataFromBlock(height) -> TweakData`), closing BIND-03 with the strongest possible FFI-fidelity claim: every cross-language scanner consumes a `TweakData` whose `Vec<chia_bls::PublicKey>` `tweakPoints` crossed the FFI boundary from Rust — the first runtime test of this Phase 5-Q1-verified shape.**

## Performance

- **Duration:** 29 min
- **Started:** 2026-05-19T00:00:01Z
- **Completed:** 2026-05-19T00:29:23Z
- **Tasks:** 6 (5 commits across implementation + 1 verification sweep absorbed into commits; SUMMARY commit is the 6th)
- **Files modified:** 6 source + 3 new test files (+ 1 regenerated napi/index.d.ts)

## Accomplishments

- `Simulator::tweak_data_from_block(height) -> TweakData` lands on the bindings facade as a sync `Arc<Mutex<>>`-dispatched method that delegates to `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` and converts the driver-side `TweakData` to the bindings-facade `TweakData` via the existing `From` impl. Method is unconditional (D-03: no `#[cfg(feature = "chip-0057")]` inside the facade).
- `bindings/simulator.json` gains one descriptor entry (`tweak_data_from_block` → `{ args: { height: u32 }, return: TweakData }`); bindy-macro resolves the `TweakData` return type via the existing `bindings/silent_payments.json` entry.
- `crates/chia-sdk-bindings/Cargo.toml` adds `features = ["chip-0057"]` to the `chia-sdk-test` dep declaration (D-03). chip-0057 is now wired unconditionally onto chia-sdk-driver + chia-sdk-utils + chia-sdk-types + chia-sdk-test for chia-sdk-bindings.
- All three binding targets regenerated cleanly: `napi pnpm build` (3m) → `napi/index.d.ts` now declares `tweakDataFromBlock(height: number): TweakData`; `pyo3 maturin develop` (52s) → `chia_wallet_sdk.Simulator.tweak_data_from_block(height)` is reachable; `wasm-pack build --target nodejs` (2m18s) → `wasm/pkg/chia_wallet_sdk_wasm.d.ts` declares the same method.
- `napi/__test__/silent_payments_e2e.spec.ts` AVA test passes: full unlabeled flow with sender funding via `sim.bls(1_000n)`, `SendDestination.silentPayment(addr)`, `Spends.withSilentPaymentKeys` registration, `tweakDataFromBlock`, `SilentPayments.scanFromTweaks`, `deriveSynthetic`, and follow-on `standardSpend`. Final assertion: `sim.coinState(detectedCoinId).spentHeight != null` (the detected SP coin is spent).
- `pyo3/tests/test_silent_payments.py::test_unlabeled_e2e` pytest passes: same flow in snake_case; pytest installed into pyo3/.venv via `pip install pytest`. No conftest.py per D-08.
- `wasm/__test__/silent_payments.spec.ts` AVA test passes: mirrors the napi test with imports from `../pkg` and `setPanicHook()` at module load. wasm-bindgen emits `Option<u32>` as `number | undefined` (vs napi's `number | null`); assertion adjusted accordingly.
- `Vec<chia_bls::PublicKey>` runtime marshaling proof fires explicitly in all 3 tests: a loop over `tweakData.tweakPoints` calls `tp.toBytes()` and asserts `length === 48` per element, BEFORE the scanner consumes the vec. First runtime test of this shape across any binding target (Phase 5 Q1 was compile-time only).
- Cross-cutting concern #5 honored: zero CHIP-0058 / websocket / sp_service / sp_client references anywhere in the 3 new test files (`grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' ...` returns 0 for all three).
- `bash scripts/sp_descriptor_facade_drift.sh` exits 0 (22 methods on both sides — descriptor↔facade drift audit unaffected by simulator.json changes, as expected).
- Full workspace test suite green: all 52 napi tests pass (51 pre-existing + 1 new BIND-03), all 8 wasm tests pass (7 pre-existing + 1 new BIND-03), all 2 pyo3 tests pass (1 pre-existing + 1 new BIND-03).

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire chip-0057 + facade method + descriptor entry** — `38f873d1` (feat)
2. **Task 2: Build all 3 binding targets** — `850ce27a` (chore — regenerated napi/index.d.ts)
3. **Task 3: napi AVA E2E test + SP-aware Spends.prepare** — `44145a00` (feat; includes Rule-2 deviation)
4. **Task 4: pyo3 pytest E2E test** — `bb47c594` (feat)
5. **Task 5: wasm AVA E2E test** — `8a4219b7` (feat)
6. **Task 6: Plan-gate verification** — sweep absorbed into per-task commits (no separate commit; all gates green on completion of Tasks 1-5)

**Plan metadata commit:** [to be created with this SUMMARY.md].

## Files Created/Modified

- `crates/chia-sdk-bindings/Cargo.toml` — **modified**. Added `features = ["chip-0057"]` to `chia-sdk-test` dep.
- `crates/chia-sdk-bindings/src/simulator.rs` — **modified**. Added `use crate::{BlsPairWithCoin, TweakData}` (broadened import) + 10-line `tweak_data_from_block` method between `coin_spend` and `spend_coins`.
- `crates/chia-sdk-bindings/src/action_system.rs` — **modified**. Updated `Spends::prepare` to call `spends.finish_silent_payments(&mut ctx, Relation::None)` before `sdk::Spends::prepare` (3 lines net + 5-line doc comment).
- `crates/chia-sdk-driver/src/action_system/spends.rs` — **modified**. Added `pub fn finish_silent_payments(&mut self, ctx, relation) -> Result<(), DriverError>` (chip-0057 gated) after `with_silent_payment_keys` — exposes private `sp_finish_branch` composition so binding-layer callers can run the SP branch out-of-band from `finish_with_keys`. ~40 lines including rustdoc.
- `bindings/simulator.json` — **modified**. Added `tweak_data_from_block` method descriptor entry (canonical sync-method shape) after `create_block`.
- `napi/index.d.ts` — **modified** (generated by `napi build`). `Simulator` class now declares `tweakDataFromBlock(height: number): TweakData`.
- `napi/__test__/silent_payments_e2e.spec.ts` — **created**. ~155 lines: AVA test for BIND-03 napi unlabeled E2E.
- `pyo3/tests/test_silent_payments.py` — **created**. ~145 lines: pytest function `test_unlabeled_e2e` mirroring the napi test in snake_case.
- `wasm/__test__/silent_payments.spec.ts` — **created**. ~165 lines: AVA test mirroring the napi test with wasm-pack-specific adjustments (imports from `../pkg`, `setPanicHook()` at module load, `undefined` vs `null` for `Option<u32>`).
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-04-SUMMARY.md` — **created** (this file).

## Decisions Made

- **D-01 closure path: TweakData crosses the FFI boundary.** All 3 cross-language tests construct the `TweakData` object on the Rust side (via `sim.tweakDataFromBlock(height)`) and consume it from the host language — no JSON fixture, no synthetic construction. This gives the strongest FFI-fidelity claim for BIND-03: not only does the type "compile" across the boundary (Phase 5 Q1), it round-trips a real-world payload with a non-empty `Vec<chia_bls::PublicKey>` and the host-language scanner correctly identifies the SP output.
- **D-02 method placement: on the existing `Simulator` class.** Following the existing pattern of every other `Simulator` method (`height`, `coinState`, `coinSpend`, etc.). The free-fn `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` remains the canonical Rust entry; the binding wraps it once. One descriptor entry, one facade method.
- **D-03 wiring: chip-0057 unconditional on chia-sdk-test.** Mirrors Phase 5's wiring (chip-0057 unconditional on chia-sdk-driver + chia-sdk-utils + chia-sdk-types from chia-sdk-bindings). No cargo feature on chia-sdk-bindings itself; no `#[cfg(feature = "chip-0057")]` gates inside the facade. `grep -c 'cfg(feature = "chip-0057"' crates/chia-sdk-bindings/src/simulator.rs` returns 0.
- **D-07: unlabeled only cross-language.** Labeled coverage stays Rust-only (Plan 06-03 `test_simulator_e2e_labeled`). The crypto for labeled is identical to unlabeled at the FFI boundary — only the scanner's labeled branch differs, and that's fully covered by Phase 3 unit tests + Plan 06-03 Rust E2E.
- **D-08: no conftest.py.** Single pytest file. A second pyo3 test consumer would justify shared fixtures later (e.g., a deterministic-mnemonic fixture, a Simulator-with-bls fixture).
- **[DEVIATION RATIONALE] Added `Spends::finish_silent_payments` to chia-sdk-driver public API.** The binding-layer `Spends::prepare` (in `crates/chia-sdk-bindings/src/action_system.rs:177`) calls `sdk::Spends::prepare(ctx, deltas, Relation::None)` directly. It does NOT call `sdk::Spends::finish_with_keys`, which is the only function in the SDK that calls `sp_finish_branch` (the private free fn that derives one-time puzzle hashes + emits `CreateCoin` conditions onto parents' `payment_assertions`). Without that branch running, the simulator never farms the SP coin → scanner finds zero detections → BIND-03 cannot pass. The minimal correct fix is to expose a public hook (`finish_silent_payments`) that runs `sp_finish_branch` when `silent_payments_pending` is non-empty. Rust-side `finish_with_keys` is unchanged (still calls `sp_finish_branch` internally); the new method is purely additive. Binding's prepare now mirrors `finish_with_keys` ordering: SP branch first, then `prepare`. This is genuine forward-looking value — any caller (Sage, third-party indexers) that wants `prepare` semantics with SP support now has a clean public hook.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `Spends::finish_silent_payments` public method + wired binding prepare to call it**

- **Found during:** Task 3 (napi AVA test execution)
- **Issue:** The plan's `<action>` block assumed `spends.prepare(deltas)` would handle SP sends correctly. First test run produced `detections.length === 0` (expected: 1) even though `tweakData.tweakPoints.length === 1` (the simulator DID see a standard puzzle spend and computed a tweak point). Investigation revealed: `chia_sdk_bindings::Spends::prepare` calls `sdk::Spends::prepare`, NOT `sdk::Spends::finish_with_keys`. The chip-0057 SP branch (`sp_finish_branch`) is invoked ONLY inside `finish_with_keys` (driver/src/action_system/spends.rs:534-537), and it's a private free function so the bindings cannot call it directly. Without the SP branch running, the recipient's one-time `CreateCoin` condition never lands on the sender's `payment_assertions`, so the simulator farms a "regular" sender-spend block with NO recipient coin — the scanner correctly finds nothing.
- **Fix:** Two-part fix landing in commit `44145a00`:
  1. Added `pub fn finish_silent_payments(&mut self, ctx, relation) -> Result<(), DriverError>` to `chia_sdk_driver::Spends` (chip-0057 gated). It's a no-op when `silent_payments_pending.is_empty()`; otherwise it calls the existing private `sp_finish_branch(ctx, self, relation)`. ~40 lines including rustdoc that documents the rationale (binding-layer pipelines that need `prepare` semantics rather than `finish_with_keys` semantics).
  2. Updated `chia_sdk_bindings::Spends::prepare` to call `spends.finish_silent_payments(&mut ctx, Relation::None)?` BEFORE `sdk::Spends::prepare`. Mirrors the ordering inside `sdk::Spends::finish_with_keys` (lines 534-539). Non-SP flows unaffected because the new method is a no-op when no SP send has been applied.
- **Files modified:** `crates/chia-sdk-driver/src/action_system/spends.rs`, `crates/chia-sdk-bindings/src/action_system.rs`
- **Verification:** Test reran and passed. `detections.length === 1`, `label === null` (napi) / `is None` (pyo3) / `=== undefined` (wasm), `amount === 100n`, follow-on `standardSpend` of the detected SP coin succeeds. Scoped clippy on chia-sdk-driver and chia-sdk-bindings both clean under `-D warnings`. No `#[allow]` attributes added.
- **Committed in:** `44145a00` (Task 3 commit)

**2. [Rule 1 - Bug] napi test used a separate `Clvm` allocator for the follow-on spend, causing `index out of bounds` in clvmr**

- **Found during:** Task 3 (napi AVA test execution, after the Rule-2 SP fix)
- **Issue:** First TS-test version constructed conditions using the outer `clvm` allocator (`clvm.createCoin(...)`, `clvm.reserveFee(...)`) but then passed them to `followClvm.delegatedSpend(conditions)` where `followClvm = new Clvm()`. Mixing NodePtrs from different allocators caused a panic in clvmr: `index out of bounds: the len is 127 but the index is 135`.
- **Fix:** Allocated the follow-on conditions in `followClvm` directly (`followClvm.createCoin(...)`, `followClvm.reserveFee(...)`).
- **Files modified:** `napi/__test__/silent_payments_e2e.spec.ts`
- **Verification:** Test passed cleanly.
- **Committed in:** `44145a00` (Task 3 commit; landed in the same commit as the Rule-2 fix because the bug manifested after the Rule-2 fix unblocked the test from reaching the follow-on spend)

---

**Total deviations:** 2 auto-fixed (1 Rule-2 missing-critical exposing private SP composition + wiring through bindings, 1 Rule-1 bug fixing a cross-allocator NodePtr mix-up in the napi test).

**Impact on plan:** The Rule-2 fix was essential for any cross-language SP send to work through the bindings — without it, BIND-03 cannot close. The fix is the minimum invasive change (one new pub fn on `Spends` + 3 lines in the binding's prepare); Rust-side `finish_with_keys` is unchanged and existing Rust E2E tests continue to pass. The fix is additive forward-looking value: third-party callers (Sage, indexers) wanting `prepare` semantics with SP support now have a clean public hook.

The plan's spirit (1 facade method + 1 descriptor entry + 3 cross-language tests = BIND-03 closed) realized exactly. All locked acceptance grep counts pass.

## Issues Encountered

- **Cargo cyclic-dev-dep concerns (preempted):** Plan 06-03's cyclic-dev-dep workaround does NOT apply at the bindings layer. The dependency chain is `chia-sdk-bindings → chia-sdk-test → chia-sdk-driver` (one-way); there's no `chia-sdk-driver → chia-sdk-bindings` edge to close the cycle. So calling `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` from `chia-sdk-bindings::Simulator::tweak_data_from_block` works directly without any inlining workaround.
- **pyo3 venv missing pytest:** Plan 06-03's pyo3 venv (`pyo3/.venv` from Phase 5 Plan 05-03) had `maturin` but not `pytest`. Resolved by `pip install pytest` inside the venv (one-time setup; pytest 9.0.3 installed). Documented in the pyo3 test docstring for future reference.
- **Pre-existing chia-sdk-daemon clippy warnings under `-D warnings`:** Already documented in `deferred-items.md` (Plan 06-01 baseline; reaffirmed by Plans 06-02 and 06-03). CI invocation (no `-D warnings`) exits 0 across the workspace; scoped clippy on chia-sdk-bindings + chia-sdk-driver + chia-sdk-test under `-D warnings` all exit 0. Per the GSD scope-boundary rule, pre-existing warnings in unrelated files are out of scope. No action taken in this plan.

## User Setup Required

None — no external service configuration required. (Note: one-time `pip install pytest` inside `pyo3/.venv` was needed; this is a developer-environment setup, not a deployment concern.)

## Next Phase Readiness

- **Plan 06-05 (example + closeout) is unblocked.** BIND-03 closed end-to-end across all 3 binding targets; the example (EX-01) can confidently demonstrate `tweak_data_from_simulator_block` from Rust knowing the same call works cross-language.
- **Zero new workspace deps.** Phase 6's "no new workspace deps" constraint upheld.
- **Zero new `#[allow]` attributes.** Workspace lint policy intact across the 3 touched Rust files (the only `#[allow]` anywhere in `silent_payments/` remains the one in `scanner.rs` from Plan 03-03).
- **Zero `unsafe` code.** No `unsafe_code = "deny"` violations.
- **Workspace test counts:**
  - napi: 51 → 52 (+1 new BIND-03)
  - pyo3: 1 → 2 (+1 new BIND-03)
  - wasm: 7 → 8 (+1 new BIND-03)
- **BIND-03 closed at runtime across all 3 FFI surfaces.**

## Verification Summary (Task 6 sweep)

| Gate | Command | Result |
|------|---------|--------|
| 1 | `cd napi && pnpm test -- --match='*BIND-03 napi*'` | PASS (1/1) |
| 2 | `cd pyo3 && python -m pytest tests/test_silent_payments.py::test_unlabeled_e2e -v` | PASS (1/1) |
| 3 | `cd wasm && pnpm test -- --match='*BIND-03 wasm*'` | PASS (1/1) |
| 4 | `cd napi && pnpm test` (full suite) | PASS (52/52) |
| 5 | `cd pyo3 && python -m pytest tests/` (full suite) | PASS (2/2) |
| 6 | `cd wasm && pnpm test` (full suite) | PASS (8/8) |
| 7 | `cargo build -p chia-sdk-bindings --all-features` | PASS |
| 8 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` (scoped) | PASS |
| 9 | `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` (scoped) | PASS |
| 10 | `cargo clippy -p chia-sdk-test --features chip-0057 --all-targets -- -D warnings` (scoped) | PASS |
| 11 | `cargo clippy --workspace --all-features --all-targets` (CI invocation, no -D warnings) | PASS (warnings only on pre-existing chia-sdk-daemon — documented) |
| 12 | `cargo fmt --all --check` | PASS |
| 13 | `cargo machete` | PASS (zero unused deps) |
| 14 | `bash scripts/sp_descriptor_facade_drift.sh` | PASS (22 methods on both sides) |

## Plan Acceptance Criteria

All success criteria from the plan met:

- [x] D-03 wiring: `chia-sdk-bindings/Cargo.toml` has `chia-sdk-test = { workspace = true, features = ["chip-0057"] }`
- [x] `crates/chia-sdk-bindings/src/simulator.rs` contains `pub fn tweak_data_from_block` (count: 1)
- [x] `crates/chia-sdk-bindings/src/simulator.rs` references `tweak_data_from_simulator_block` (count: 2 — use of fully-qualified path in fn body)
- [x] No `#[cfg(feature = "chip-0057")]` gates inside `crates/chia-sdk-bindings/src/simulator.rs` (count: 0)
- [x] `bindings/simulator.json` has `tweak_data_from_block` entry with `"return": "TweakData"` (Python json-load assertion verified)
- [x] `napi/index.d.ts` declares `tweakDataFromBlock(height: number): TweakData` (signature regex match)
- [x] `wasm/pkg/*.d.ts` declares `tweakDataFromBlock` (count: 2 — main + bg files)
- [x] napi BIND-03 test contains `BIND-03 napi`, `tweakDataFromBlock`, `tweakData.tweakPoints`, `SilentPayments.scanFromTweaks`, `deriveSynthetic`; no `@ts-ignore`
- [x] pyo3 BIND-03 test has `def test_unlabeled_e2e`, `tweak_data_from_block`, `tweak_data.tweak_points`, `SilentPayments.scan_from_tweaks`; NO conftest.py
- [x] wasm BIND-03 test has `BIND-03 wasm`, `tweakDataFromBlock`, `tweakData.tweakPoints`, `SilentPayments.scanFromTweaks`, `setPanicHook`, `from "../pkg"`
- [x] All 3 cross-language tests exit 0
- [x] `bash scripts/sp_descriptor_facade_drift.sh` exits 0
- [x] `cargo clippy` scoped on chia-sdk-bindings/chia-sdk-driver/chia-sdk-test under `-D warnings` all exit 0
- [x] `cargo fmt --all --check` exits 0
- [x] `cargo machete` exits 0
- [x] No new `#[allow]` attributes in any modified file
- [x] Zero CHIP-0058 / WS / sp_service / sp_client references in the 3 new test files (grep count = 0)
- [x] BIND-03 closure: full unlabeled send → farm → extract → scan → SPEND round-trip passes from napi (TypeScript), pyo3 (Python), and wasm (TypeScript-on-WASM)

## Self-Check: PASSED

All claimed files exist on disk:
- `crates/chia-sdk-bindings/Cargo.toml` (modified)
- `crates/chia-sdk-bindings/src/simulator.rs` (modified)
- `crates/chia-sdk-bindings/src/action_system.rs` (modified)
- `crates/chia-sdk-driver/src/action_system/spends.rs` (modified)
- `bindings/simulator.json` (modified)
- `napi/index.d.ts` (regenerated)
- `napi/__test__/silent_payments_e2e.spec.ts` (created)
- `pyo3/tests/test_silent_payments.py` (created)
- `wasm/__test__/silent_payments.spec.ts` (created)
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-04-SUMMARY.md` (this file)

All claimed commits exist in git history:
- `38f873d1` (Task 1: facade + descriptor + Cargo.toml chip-0057 wiring)
- `850ce27a` (Task 2: regenerated napi/index.d.ts)
- `44145a00` (Task 3: napi BIND-03 E2E + Rule-2 SP-aware Spends.prepare deviation)
- `bb47c594` (Task 4: pyo3 BIND-03 E2E)
- `8a4219b7` (Task 5: wasm BIND-03 E2E)

---
*Phase: 06-simulator-round-trip-bindings-e2e-example*
*Completed: 2026-05-19*
