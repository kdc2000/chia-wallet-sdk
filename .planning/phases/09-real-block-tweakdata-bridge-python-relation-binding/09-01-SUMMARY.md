---
phase: 09-real-block-tweakdata-bridge-python-relation-binding
plan: 01
subsystem: silent-payments-driver
tags: [chip-0057, silent-payments, tarjan-scc, assert-concurrent-spend, real-block]

# Dependency graph
requires:
  - phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
    provides: silent_payments/protocol.rs (compute_input_hash, ScalarField, primitives)
provides:
  - tweak_data_from_block_spends helper in chia_sdk_driver::silent_payments
  - Iterative Tarjan SCC implementation over opcode-64 AssertConcurrentSpend graph
  - Pass 2a same-puzzle-hash bucketing + Pass 2b SCC grouping algorithm
  - Pollution-attack-resistant transaction-group emission
  - chia_wallet_sdk::prelude::tweak_data_from_block_spends re-export under chip-0057
affects: [09-02-bridge-simulator-refactor, 09-05-bridge-binding-facade, 09-06-bridge-cross-binding-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Iterative Tarjan SCC with explicit work stack (no recursion)
    - IndexMap-driven deterministic puzzle-hash bucketing
    - Pure-function block-shape helper (no chia-consensus dep, no Simulator dep)
    - Module-level rustdoc documenting load-bearing group emission ordering

key-files:
  created:
    - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
  modified:
    - crates/chia-sdk-driver/src/silent_payments/mod.rs
    - src/prelude.rs

key-decisions:
  - "Iterative Tarjan SCC over recursive: defends against adversarial deep AssertConcurrentSpend graphs that would blow the call stack."
  - "IndexMap (insertion-ordered) for Pass 2a buckets and graph node identity: emission order is byte-stable across runs, satisfying downstream byte-equality oracles."
  - "Group emission order pinned: Stage 2a buckets first, then Stage 2b SCCs in Tarjan finishing order, then standalone singletons in input order."
  - "Driver-side prelude does not need editing: silent_payments::* wildcard in lib.rs already re-exports the new symbol; only the umbrella src/prelude.rs (explicit list) needed updating."
  - "Identity-element test uses PublicKey::default() (BLS12-381 identity) as the synthetic key — simplest natural fixture that drives A_sum to identity and exercises the CHIP §459 guard end-to-end."

patterns-established:
  - "Block-shape helper signature: (coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData, DriverError> — matches what coinset RPC consumers already have post-decompression."
  - "build_standard_coin_spend test helper: construct a standard-puzzle CoinSpend by calling StandardLayer::new(pk).spend(&mut ctx, coin, conditions) and pulling the resulting CoinSpend from SpendContext::take()."

requirements-completed: [BRIDGE-01]

# Metrics
duration: 16min
completed: 2026-05-29
---

# Phase 9 Plan 01: BRIDGE-01 real-block TweakData helper Summary

**Canonical CHIP-0057 real-block TweakData builder with Pass 2a same-puzzle-hash bucketing, Pass 2b iterative Tarjan SCC over opcode-64 AssertConcurrentSpend edges, and pollution-attack-resistant transaction-group emission.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-05-29T15:59:34Z
- **Completed:** 2026-05-29T16:15Z (approximate)
- **Tasks:** 2
- **Files modified:** 3 (1 created, 2 edited)
- **Lines added:** 527 (helper + tests) + ~3 (mod.rs + prelude)
- **Tests added:** 6 (all green on first run after import fix-up)

## Accomplishments

- Landed `chia_sdk_driver::silent_payments::tweak_data_from_block_spends(coin_spends, additions) -> Result<TweakData, DriverError>` — the canonical block-shape entry point that downstream Python / TS / wasm consumers can call without depending on the simulator or `chia-consensus`.
- Implemented iterative Tarjan SCC (~65 lines, explicit work stack) over opcode-64 `AssertConcurrentSpend` directed-edge graphs — closes Pitfall 1 (Pass 2b pollution attack) which is the load-bearing correctness oracle for BRIDGE-01.
- Pinned group emission ordering (Stage 2a buckets in puzzle-hash insertion order, Stage 2b SCCs in Tarjan finishing order, standalone singletons in input order) so byte-equality tests against the simulator helper stay stable.
- All 6 BRIDGE-01 inline tests pass: `test_empty_block`, `test_non_standard_puzzle_skip`, `test_identity_element_guard`, `test_pass_2a_round_trip_matches_simulator_helper`, `test_multi_input_round_trip`, `test_pass_2b_pollution_resistance`.
- Helper reachable via `chia_wallet_sdk::prelude::tweak_data_from_block_spends` under chip-0057.

## Task Commits

Each task was committed atomically:

1. **Task 1: scaffold helper + iterative Tarjan SCC** — `bfee440a` (feat)
2. **Task 2: 6 inline tests + umbrella prelude wiring** — `d608fd61` (test)

_Note: TDD-marked tasks shipped as combined feat/test commits per plan structure (Task 1 = algorithm scaffold, Task 2 = full test suite + prelude wiring)._

## Files Created/Modified

- `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` (created, 527 lines) — helper + iterative Tarjan + 6 inline tests + module-level rustdoc with grouping algorithm + emission-order contract.
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` (modified) — `mod block_tweak_data;` + `pub use block_tweak_data::tweak_data_from_block_spends;` inserted between `protocol` and `scanner` blocks.
- `src/prelude.rs` (modified) — extended chip-0057 driver re-export block with `tweak_data_from_block_spends`.

## Decisions Made

- **Iterative Tarjan implementation choice:** translated the Python reference at `~/silent-payments/scanner.py:34-95` into Rust as an explicit `Vec<(usize, usize)>` work stack with three companion vectors (`indices`, `lowlinks`, `on_stack`). No new graph crate dependency (CLAUDE.md ban on new workspace deps); `IndexMap` (already in workspace) provides deterministic iteration order for the graph node identity map. The frame structure carries `(node, next_neighbor_index)` so re-entering a frame after a recursive call resumes neighbor iteration where it left off.
- **`PublicKey::default()` for the identity test:** the simplest natural fixture that drives `A_sum` to the BLS12-381 identity element. The standard puzzle curries fine with the identity public key (`StandardArgs::from_clvm` does not reject identity), so the helper's Stage 1 parse succeeds and Stage 3 emits a tweak point that is the identity, triggering the CHIP §459 guard. No need for the more complex `pk + (-pk)` arithmetic that would require `r - sk` byte computation against `GROUP_ORDER`.
- **`build_standard_coin_spend` test helper:** thin wrapper around `StandardLayer::new(pk).spend(&mut ctx, coin, conditions)` + `ctx.take().pop()` — exercises the same serialization path the production action surface uses, so the constructed `CoinSpend` is byte-identical in shape to what `Spends::prepare` emits.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] Three clippy errors in initial helper draft**

- **Found during:** Task 1 (algorithm scaffold)
- **Issue:** `clippy::collapsible_if` on two nested `if let` chains, `clippy::needless_range_loop` on the standalone-singletons enumeration over `grouped`.
- **Fix:** Collapsed `if let Condition::AssertConcurrentSpend(a) = cond { if let Some(...) = ... }` into Rust 1.88-style `&&`-chained `if let`. Re-wrote the standalone scan as `for (i, &is_grouped) in grouped.iter().enumerate()` and gated on the boolean.
- **Files modified:** crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
- **Verification:** `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0.
- **Committed in:** bfee440a (Task 1 commit)

**2. [Rule 3 - Blocking] Wrong import path for tweak_data_from_simulator_block**

- **Found during:** Task 2 (tests)
- **Issue:** Initial test draft imported `crate::silent_payments::tweak_data_from_simulator_block` (driver-side path), but the helper lives in `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` (test-side crate).
- **Fix:** Swapped to the test-side path; the `chia-sdk-test` dev-dep is already wired with `chip-0057` feature cascade so the import resolves.
- **Files modified:** crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
- **Verification:** All 6 tests compile and pass.
- **Committed in:** d608fd61 (Task 2 commit)

**3. [Rule 1 - Lint] Four `clippy::similar_names` + one `clippy::doc_markdown` in test fixtures**

- **Found during:** Task 2 (tests)
- **Issue:** `alice_sk` / `alice_pk` pair (sk/pk one-byte diff), and `pk_a` / `ph_a` (k/h one-byte diff) for the pollution test's coin-id pre-compute. Also a bare `tweak_points` in a doc comment.
- **Fix:** Renamed to `alice` / `alice_public` (no shared prefix collision), `puzzle_hash_a` / `puzzle_hash_b` / `puzzle_hash_polluter` (full word distinguishable from `pk_a` / `pk_b` / `pk_polluter`). Added backticks around `tweak_points` in rustdoc. No `#[allow]` added.
- **Files modified:** crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
- **Verification:** `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0.
- **Committed in:** d608fd61 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 lint, 1 blocking import path)
**Impact on plan:** All auto-fixes were lint-driven or single-line import corrections. Zero new `#[allow]` attributes; zero new workspace deps; algorithm shape and test coverage exactly as planned.

## Issues Encountered

None requiring escalation. The pollution-resistance test (the flagship correctness oracle for BRIDGE-01) passed on the first try after the import fix, validating the iterative Tarjan SCC implementation.

## User Setup Required

None.

## Next Phase Readiness

- **BRIDGE-02 (Plan 09-02 simulator refactor)** is unblocked — `tweak_data_from_simulator_block` can now delegate to `tweak_data_from_block_spends(&sim.block_spends(h), &sim.block_outputs(h))` and the existing Phase 6 e2e tests will validate byte-equality.
- **BRIDGE-05 (Plan 09-05 binding facade)** is unblocked — the helper has a clean `(Vec<CoinSpend>, Vec<Coin>) -> Result<TweakData>` signature that maps directly to a `SilentPayments.tweakDataFromBlockSpends` static method on the napi/pyo3/wasm surfaces.
- **BRIDGE-06 (Plan 09-06 cross-binding tests + example)** is unblocked — multi-input round-trip tests in each binding target can call the helper through the binding once BRIDGE-05 lands.

## Self-Check: PASSED

- File exists: `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` ✓
- Helper present: `pub fn tweak_data_from_block_spends` ✓
- Tarjan present: `fn iterative_tarjan_scc` ✓
- Module re-export: `pub use block_tweak_data::tweak_data_from_block_spends;` in mod.rs ✓
- Umbrella prelude: `tweak_data_from_block_spends` in src/prelude.rs ✓
- All 6 tests pass: `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests` reports 6 passed ✓
- Scoped clippy -D warnings: clean ✓
- Workspace --all-features build: clean ✓
- cargo fmt --check: clean ✓
- Zero new `#[allow]` attributes ✓
- Zero GSD planning-artifact references in source ✓
- Commits bfee440a and d608fd61 both present in `git log` ✓

---

*Phase: 09-real-block-tweakdata-bridge-python-relation-binding*
*Completed: 2026-05-29*
