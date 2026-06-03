---
quick_id: 260603-ilc
type: quick
status: complete
---

# Quick Task 260603-ilc Summary

**Cleared both Phase-09.3 deferred binding-test failures — both were stale test expectations, no production code change; napi (53) / wasm (10) / pyo3 (4) all green.**

## What changed (test-only)

| Fix | Files | Change |
|-----|-------|--------|
| #1 GUARD-03 matcher | `napi/__test__/silent_payments_e2e.spec.ts`, `wasm/__test__/silent_payments.spec.ts`, `pyo3/tests/test_silent_payments.py` | matcher → `key not synthetic` (matches `DriverError::SilentPaymentKeyNotSynthetic` → `#[error("silent payment key not synthetic")]`) |
| #2 multi-input count | `napi/__test__/silent_payments_multi_input.spec.ts`, `wasm/__test__/silent_payments_multi_input.spec.ts`, `pyo3/tests/test_silent_payments.py` | `tweak_points` assertion 1 → 3 + explanatory comment; `detections == 1` unchanged |

## Diagnosis

Both deferred items were mislabeled as possible code bugs. #2's "suspected helper grouping bug" was wrong: the additive `tweak_data_from_block_spends` model (adopted in quick tasks 260602-ejk/mhw) intentionally emits 3 candidate tweak_points for a 2-input concurrent SP send (2 Pass-1 singletons + 1 Pass-2 SCC aggregate). Only the SCC-aggregate point matches the sender-derived `input_hash`, so the scanner detects exactly one output. The Rust inline test `block_tweak_data.rs::same_ph_multi_input_round_trip_via_concurrent_spend` already pins this at 3; the binding tests were written against the *simulator* helper's whole-block-coalescing count of 1.

## Verification

- `cd napi && pnpm build && pnpm test` → **53 passed** (BIND-03 + GUARD-03 + BRIDGE-06 multi-input)
- `cd wasm && pnpm test` → **10 passed**
- `pyo3/.venv` `maturin develop && pytest` → **4 passed** (test_unlabeled_e2e, test_raw_key_not_synthetic_errors, test_multi_input_e2e, + labeled)
- No generated-artifact churn (rebuilds byte-identical — no Rust behavior changed)

## Notes

`deferred-items.md` updated with a RESOLVED banner. Both binding test suites now pass end-to-end, closing the last items flagged during Phase 09.3.
