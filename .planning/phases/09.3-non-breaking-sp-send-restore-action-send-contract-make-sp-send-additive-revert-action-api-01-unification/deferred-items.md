# Deferred Items — Phase 09.3

Pre-existing failures discovered during plan 09.3-02 execution. Confirmed
present at the pre-09.3 baseline commit `cfaaf275` (built + ran in a throwaway
worktree), so they are NOT caused by the 09.3 SP-send migration. Out of scope
per the executor SCOPE BOUNDARY rule (only auto-fix issues directly caused by
the current task's changes). The 09.3-02 migration preserved the exact same
runtime behavior — the single-input SP-send E2E (the detection oracle) passes
across napi/wasm/pyo3.

## 1. GUARD-03 negative-test error-message regex mismatch (napi + wasm + pyo3)

- **Files:**
  - `napi/__test__/silent_payments_e2e.spec.ts` (GUARD-03 test, `t.throws` matcher)
  - `wasm/__test__/silent_payments.spec.ts` (GUARD-03 test)
  - `pyo3/tests/test_silent_payments.py` (GUARD-03 negative test, if asserting on message text)
- **Symptom:** the assertion expects `/not the synthetic key|KeyNotSynthetic/i`
  but the driver error renders as `"silent payment key not synthetic"`
  (`DriverError::SilentPaymentKeyNotSynthetic` → `#[error("silent payment key not synthetic")]`
  at `crates/chia-sdk-driver/src/driver_error.rs:179`). Neither alternation
  branch is a substring of the actual message, so the matcher never matches.
- **Root cause:** test-side regex written against a never-shipped error string
  (introduced Phase 09.2-03, commit `20b040bd`); the driver error text never
  contained "the synthetic key" nor "KeyNotSynthetic".
- **Fix (when picked up):** change the matcher to `/key not synthetic/i` (or the
  exact rendered string) in all three binding GUARD-03 tests. Pure test-side
  one-liner; no Rust change.

## 2. Multi-input `tweak_data_from_block_spends` SCC grouping yields 3 tweak points instead of 1 (napi + wasm)

- **Files:**
  - `napi/__test__/silent_payments_multi_input.spec.ts` (BRIDGE-06)
  - `wasm/__test__/silent_payments_multi_input.spec.ts` (BRIDGE-06)
- **Symptom:** `tweakData.tweakPoints.length` is `3`, the test asserts `1`
  ("one SP transaction group -> one tweak_point").
- **Scope note:** the failing path is the `SilentPayments.tweakDataFromBlockSpends`
  helper (BRIDGE-05) consuming `sim.blockSpends(h)` + `sim.blockOutputs(h)`
  (BRIDGE-06 facade) — a Phase 09-06 binding-side grouping helper, NOT the
  SP-send action. The Rust-level multi-input E2E
  (`crates/chia-sdk-driver/tests/silent_payments_e2e.rs::test_simulator_e2e_multi_input`),
  which uses `tweak_data_from_simulator_block`, PASSES — so the Rust core SP send
  + `AssertConcurrent` cycle binding + scan all work. The bug is isolated to the
  block-spends grouping helper's tweak-point coalescing (Pass-2b SCC over
  opcode-64 AssertConcurrentSpend edges) as exercised through the bindings.
- **Root cause (suspected):** `tweak_data_from_block_spends` does not coalesce
  the two concurrent SP-input spends into one tweak_point group the way
  `tweak_data_from_simulator_block` does; needs investigation in the Phase-09-06
  BRIDGE-05 helper. Predates 09.3.
- **Fix (when picked up):** debug the SCC grouping in the block-spends tweak-data
  helper (chia-sdk-bindings simulator/silent_payments facade); add a Rust-level
  test for `tweak_data_from_block_spends` multi-input grouping to mirror the
  passing `tweak_data_from_simulator_block` coverage.

Both items were green-or-failing identically at `cfaaf275` (verified by building
that commit's napi and running both specs). They block neither the
ACTION-API-01 revert nor the SEND-04 additive SP-send surface this phase
delivers.
