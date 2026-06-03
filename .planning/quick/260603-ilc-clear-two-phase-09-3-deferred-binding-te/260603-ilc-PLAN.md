---
quick_id: 260603-ilc
type: quick
status: complete
---

# Quick Task 260603-ilc — Clear two Phase-09.3 deferred binding-test failures

## Objective

Clear the two pre-existing binding-test failures logged in Phase 09.3's
`deferred-items.md`. Both are stale test EXPECTATIONS — no production Rust change.

## Tasks

1. **GUARD-03 matcher** — driver emits `#[error("silent payment key not synthetic")]`
   (`driver_error.rs:179`); update the negative-test matchers from
   `/not the synthetic key|KeyNotSynthetic/i` / `"not the synthetic key"` to
   `key not synthetic` in napi/wasm/pyo3.
2. **Multi-input tweak-point count** — binding multi-input tests use the additive
   `tweak_data_from_block_spends` helper (3 candidate tweak_points: 2 Pass-1
   singletons + 1 Pass-2 SCC aggregate), but assert `== 1`. Change to `== 3`
   (keep `detections == 1`). Matches Rust inline test
   `block_tweak_data.rs::same_ph_multi_input_round_trip_via_concurrent_spend`.

## Verify

Rebuild + run all three binding suites green (napi `pnpm build && pnpm test`,
wasm `pnpm test`, pyo3 `maturin develop && pytest`).
