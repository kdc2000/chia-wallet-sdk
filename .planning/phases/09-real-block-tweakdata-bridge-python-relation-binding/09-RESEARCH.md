# Phase 9: Real-block TweakData bridge + Python Relation binding — Research

**Researched:** 2026-05-29
**Domain:** Chia silent-payments scanner (CHIP-0057 Pass 2a/2b grouping) + bindy-macro cross-language type plumbing
**Confidence:** HIGH (every locked decision in CONTEXT.md verified against the current source tree; precedents inspected file-by-file; bindy `Option<Class>` parameter pattern verified in `bindings/action_system.json` for `Option<Id>` and `Option<TransferNftById>`).

## Summary

Phase 9 makes two surgical additions to v1's CHIP-0057 surface so downstream Python (and TS/wasm) consumers can drive multi-input SP sends and scan real (non-simulator) blocks. The CONTEXT.md is exceptionally well-scoped (14 locked decisions, exact signatures and file paths), so research scope reduces to: (a) drift-checking the cited precedents against current source, (b) surfacing implementation traps the decisions don't already address, and (c) producing the Validation Architecture mapping required for the Nyquist VALIDATION.md.

All precedents in CONTEXT.md verified in the current tree: `chia-sdk-bindings/src/action_system.rs:185` does hardcode `Relation::None` inside `prepare`; the `Id` opaque-handle pattern is at `:449-486` and `SendDestination` at `:504-540`; `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` exists at the cited file with the structure CONTEXT.md describes; the driver-side `sp_finish_branch` runs inside `Spends::prepare` (Phase 7 CLEANUP-03 closed the previous `finish_silent_payments` public leak, so the binding's `prepare` already drives the SP branch via the driver's `prepare` — meaning Phase 9 only needs to thread `Relation` through, not re-plumb the SP composition); the `Relation` enum lives at `crates/chia-sdk-driver/src/action_system/relation.rs:11-35` (2 variants, well-documented). The `Spends::prepare(coin_ids... Relation::AssertConcurrent)` cycle-pinning test at `spends.rs:843+` confirms the cycle shape downstream scanners depend on.

**The single highest-risk technical finding:** the current `tweak_data_from_simulator_block` does NOT do Pass 2a/2b grouping. It aggregates *every* standard-puzzle spend in the block into one `synthetic_pks` accumulator and emits one `tweak_point`. This works for the simulator (one tx per block, by convention) but is wrong for real blocks where multiple SP-using transactions can land in the same block. D-04 acknowledges this generalization is required; the planner must NOT treat the extraction as a pure refactor. The reference Python implementation at `~/silent-payments/scanner.py:339-421` shows the correct algorithm (Tarjan SCC over opcode-64 edges from solution execution), and the SDK has the building blocks (`chia_sdk_types::run_puzzle` runs puzzle+solution to extract conditions; `Condition::AssertConcurrentSpend` parses opcode 64).

**Primary recommendation:** Treat BRIDGE-01 (the new `tweak_data_from_block_spends` helper) as new logic, not a refactor of the simulator helper. The simulator helper becomes a thin adapter (D-03), but the helper itself must implement the full Pass 2a + Pass 2b grouping algorithm from scratch using `chia_sdk_types::run_puzzle` + the existing `Condition::AssertConcurrentSpend` parser. Plan an explicit Tarjan-SCC implementation step. Everything else (Relation binding, Spends.prepare signature change, exposing the new helper through bindings, multi-input tests) is mechanically simple.

## <user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Gap 1 — Block input shape:**

- **D-01:** `fn tweak_data_from_block_spends(coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData, DriverError>` in `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` (new sibling file). Two-slice shape mirrors the simulator helper's logical inputs. Zero new types. New sibling file (NOT appended to `protocol.rs` or `scanner.rs`).
- **D-02:** Generator decompression is OUT of scope. Callers hand the helper decompressed `Vec<CoinSpend>` + `Vec<Coin>`.

**Gap 1 — Logic extraction strategy:**

- **D-03:** Extract pure core into the new helper; refactor `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` to delegate. Single canonical implementation.
- **D-04:** Pass 2a (same-puzzle-hash) AND Pass 2b (`Relation::AssertConcurrent` SCC over opcode-64 edges) grouping lives INSIDE the new helper. Callers pass full block; helper emits one `tweak_point` per transaction group. NO separate `group_spends_into_tx_groups` public API.
- **D-05:** Non-standard puzzles (CAT/NFT/etc.) silently skipped via `StandardLayer::parse_puzzle` defensive parse.
- **D-06:** CHIP §459 identity-element guard preserved (no tweak_point emitted when result is BLS12-381 identity element).

**Gap 2 — Python `Relation` binding:**

- **D-07:** Opaque-handle + factories for `Relation`, matching `Id` precedent at `crates/chia-sdk-bindings/src/action_system.rs:449-486` and `SendDestination` at `:504-540`:
  ```rust
  #[derive(Clone, Debug)]
  pub struct Relation(pub(crate) sdk::Relation);

  impl Relation {
      pub fn none() -> Result<Self> { Ok(Self(sdk::Relation::None)) }
      pub fn assert_concurrent() -> Result<Self> { Ok(Self(sdk::Relation::AssertConcurrent)) }
      pub fn is_none(&self) -> Result<bool> { Ok(matches!(self.0, sdk::Relation::None)) }
      pub fn is_assert_concurrent(&self) -> Result<bool> { Ok(matches!(self.0, sdk::Relation::AssertConcurrent)) }
      pub fn equals(&self, other: Relation) -> Result<bool> { Ok(self.0 == other.0) }
  }
  ```
- **D-08:** Descriptor entry in `bindings/action_system.json` (next to `Id` and `SendDestination`). NOT a top-level type-group mapping.

**Gap 2 — `Spends.prepare` signature:**

- **D-09:** `fn prepare(&self, deltas: Deltas, relation: Option<Relation>) -> Result<FinishedSpends>` — `None` defaults to `sdk::Relation::None` (preserves current behavior at `chia-sdk-bindings/src/action_system.rs:185`). Existing single-input pyo3 E2E (`test_unlabeled_e2e`) keeps working with no signature change at call sites.

**Cross-binding parity scope:**

- **D-10:** All three targets (pyo3 + napi + wasm) ship the extended surface in this phase. Descriptor change auto-propagates via bindy-macro.
- **D-11:** Multi-input round-trip test in each binding target (3 new tests).

**Example update:**

- **D-12:** `examples/silent_payment.rs` gains a multi-input section (~30-50 lines).

**Rust-side tests:**

- **D-13:** Inline `#[cfg(test)] mod tests {}` at the bottom of `block_tweak_data.rs`. Coverage: single SP send byte-equality with simulator helper output, multi-input SP send (Pass 2b SCC), **Pass 2b pollution attack** (third-party AssertConcurrentSpend pointing at victim's coin must NOT merge groups), non-standard-puzzle skip, CHIP §459 identity-element guard, empty-block.

**Bindings expose `tweak_data_from_block_spends`:**

- **D-14:** Expose as static method on `SilentPayments` namespace class in `chia-sdk-bindings::silent_payments` + `bindings/silent_payments.json`. Signature: `SilentPayments.tweakDataFromBlockSpends(coinSpends: CoinSpend[], additions: Coin[]) -> TweakData` (camelCase per binding convention).

### Claude's Discretion

- Inline flat `mod tests {}` vs nested sub-mods in `block_tweak_data.rs` (flat preferred unless ≥8 tests).
- Property-based (proptest) test for Pass 2b grouping correctness — optional; targeted unit tests probably sufficient.
- `Relation` binding doc comments — adapt from `relation.rs` rustdoc (already documents "load-bearing for CHIP-0057" at variant level).
- Exact location of binding-side `tweak_data_from_block_spends` (likely `crates/chia-sdk-bindings/src/silent_payments.rs`).
- Exact test names; suggested seeds in CONTEXT.md `<decisions>` section.

### Deferred Ideas (OUT OF SCOPE)

- Generator decompression integrated into the SDK (D-02 forbids — separate phase if asked for).
- Label support on per-spend output of `tweak_data_from_block_spends`.
- `Relation` exposed as a fully-introspectable enum (opaque-handle covers all current needs).
- Async / streaming variant for very large blocks.
- A separate `chia-sdk-rpc-bridge` crate.
- Property-based testing (left to Claude's Discretion).
- CHIP-0058 transport client.
</user_constraints>

## Project Constraints (from CLAUDE.md)

| Constraint | How Plan Must Honor |
|------------|---------------------|
| Rust 1.90.0 pinned (`rust-toolchain.toml`); edition 2024 | No nightly-only features, no edition 2024 features that haven't stabilized by 1.90 |
| Workspace clippy: `deny clippy::all`, `warn pedantic`, `warn cargo`; `deny unsafe_code`; `deny dead_code` | New code must pass scoped `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` AND `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings`. No `unsafe` blocks. No `#[allow]` attributes (only one pre-existing `#[allow]` in `scanner.rs::scan_from_tweaks` per Plan 03-03; phase 9 must not add a second). |
| `cargo machete` clean (no new ignored entries) | If any new dep is needed (none planned), document. The phase planner should NOT add new workspace deps — D-02 stays. |
| CI per-crate build with and without `--all-features` | Code behind `chip-0057` must compile when feature is OFF for any crate that already has the feature gate. `block_tweak_data.rs` lives under `silent_payments/` which is `#[cfg(feature = "chip-0057")]`-gated at the mod declaration in `chia-sdk-driver/src/lib.rs:40` — automatically inherits the gate. |
| Prefer LSP over Grep/Read | Use LSP for symbol lookup during plan execution. Verified during research: `workspaceSymbol` would find `tweak_data_from_simulator_block`, `Spends::prepare`, `Relation`, etc. faster than grep. |
| After writing code, check LSP diagnostics and fix errors | Per-plan acceptance includes `cargo check` clean before commit. |
| GSD workflow enforcement | All file changes must go through `/gsd:execute-phase` per CLAUDE.md project rule. |

**Authority:** CLAUDE.md directives have the same force as locked decisions. Research should not recommend approaches that contradict them — e.g., no `Vec<u8>` workarounds via `unsafe`, no `#[allow]` to silence pedantic lints, no new workspace deps without folding them through a workspace-level decision.

## <phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BRIDGE-01 | New helper `chia_sdk_driver::silent_payments::tweak_data_from_block_spends(coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData, DriverError>` in new sibling file `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs`. Implements Pass 2a + Pass 2b grouping. Inline `#[cfg(test)] mod tests {}` covers single-input byte-equality with simulator helper, multi-input SCC grouping, Pass 2b pollution attack, non-standard-puzzle skip, CHIP §459 identity-element guard, empty-block. | Standard Stack §"Pass 2 grouping algorithm" (uses `chia_sdk_types::run_puzzle`, `Condition::AssertConcurrentSpend`, custom Tarjan); Architecture §"Block-level grouping pipeline"; Pitfall 1 (Pass 2b pollution); Pitfall 2 (allocator reuse); Pitfall 3 (puzzle execution cost). |
| BRIDGE-02 | Refactor `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` to delegate to `tweak_data_from_block_spends`. Existing Phase 6 simulator tests (`test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change` in `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`) and the two existing inline tests in `chia-sdk-test/src/silent_payments/tweak_data.rs` (`tweak_data_empty_block_returns_empty_tweak_data`, `tweak_data_genesis_height_is_safe`) all continue to pass byte-identically. | Architecture §"Simulator helper post-refactor"; Pitfall 4 (existing Simulator helper aggregates entire block as one group — refactor must preserve simulator-conventional 1-tx-per-block behavior as a special case of the general algorithm). |
| BRIDGE-03 | Opaque-handle `Relation` binding in `chia-sdk-bindings::action_system` + 5-method descriptor entry in `bindings/action_system.json`. Surfaces on napi/pyo3/wasm. Mirrors `Id`/`SendDestination` precedent. | Standard Stack §"bindy-macro patterns"; Architecture §"Opaque-handle binding"; Code Examples §"Relation binding". |
| BRIDGE-04 | `Spends.prepare(deltas, relation: Option<Relation>)` extended signature in `chia-sdk-bindings::action_system`. `None` → `sdk::Relation::None` (preserves current behavior). Existing pyo3 E2E (`test_unlabeled_e2e`) passes without source changes. Descriptor entry in `bindings/action_system.json::Spends.methods.prepare` updated with `"relation": "Option<Relation>"`. | Architecture §"Spends.prepare binding signature"; Code Examples §"Spends.prepare extension"; Pitfall 5 (default-arg semantics across napi/pyo3/wasm differ — `null`/`undefined`/`None`). |
| BRIDGE-05 | `tweak_data_from_block_spends` bound on `SilentPayments` namespace class. Descriptor entry in `bindings/silent_payments.json::SilentPayments.methods.tweak_data_from_block_spends`. camelCase auto-derived. | Architecture §"SilentPayments namespace static methods"; Code Examples §"Binding facade for block helper". |
| BRIDGE-06 | Three new cross-binding multi-input round-trip tests: `napi/__test__/silent_payments_multi_input.spec.ts`, `pyo3/tests/test_silent_payments.py::test_multi_input_e2e`, `wasm/__test__/silent_payments_multi_input.spec.ts`. Each: farm 2+ XCH coins → single `Action::send` to one SP address → `Spends.prepare(deltas, Relation.assert_concurrent())` → farm bundle → `SilentPayments.tweak_data_from_block_spends(spends, additions)` → scan → exactly one DetectedSpCoin. Plus example update per D-12. | Architecture §"Cross-binding multi-input test rhythm"; Code Examples §"Multi-input round-trip skeleton"; Pitfall 6 (binding-side `sim.block_spends`/`sim.block_outputs` need to be reachable from binding tests — verify); Pitfall 7 (cross-allocator NodePtr mix-up — pattern documented in Phase 6 04-SUMMARY). |
</phase_requirements>

## Standard Stack

### Core (already in workspace; verified `chia_sdk_*` re-exports from umbrella `chia-wallet-sdk` at `src/lib.rs`)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `chia-protocol` | 0.36.1 (workspace pinned) | `CoinSpend`, `Coin`, `Bytes32` — the public input types of `tweak_data_from_block_spends` | Pre-existing pinned dep; no version bump permitted (CLAUDE.md). |
| `chia-bls` | 0.36.1 | `PublicKey`, scalar multiplication, `is_inf` identity check | Already used by every SP code path. |
| `chia-sdk-types` | path dep | `Condition::AssertConcurrentSpend` (parses opcode 64); `run_puzzle` (runs `puzzle.run(solution)` to extract conditions); `ScalarField` for `compute_input_hash` return type | The two SDK building blocks that make Pass 2b cheaply expressible in Rust. `run_puzzle` is at `crates/chia-sdk-types/src/run_puzzle.rs:9-22` (already used by the EIP-712 layer at `p2_eip712_message_layer.rs:274`). |
| `chia_sdk_driver::Layer`, `StandardLayer`, `Puzzle` | path dep | `StandardLayer::parse_puzzle` is the defensive standard-p2 parse that returns `Ok(None)` for non-standard puzzles. Already used by `tweak_data_from_simulator_block` (verified at `tweak_data.rs:55`). | Pre-existing pattern; no change. |
| `chia_sdk_driver::silent_payments::compute_input_hash` | re-exported | The pure scalar primitive that the new helper composes per-group. Verified at `protocol.rs:182`. | Pre-existing; reuse. |
| `chia-sdk-types::silent_payments::ScalarField` | path dep | Output type of `compute_input_hash`; not exposed in helper public API but used internally. | Pre-existing. |
| `clvm-traits` | 0.36.1 | `ToClvm` for converting `puzzle_reveal`/`solution` bytes → `NodePtr` before `run_puzzle` / `Puzzle::parse`. | Already imported in `tweak_data.rs:13`. |
| `clvmr` | 0.16.2 | `Allocator`, `NodePtr` | Already used; one `Allocator::new()` per helper invocation is correct (Pitfall 2). |
| bindy + bindy-macro | path (in repo) | Cross-language code generation. `Option<Relation>` parameter pattern verified — `Option<Id>` and `Option<TransferNftById>` already in `bindings/action_system.json:173,181` as args. | Pre-existing; no change. |

### Supporting (verified pre-existing)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chia-sdk-test::Simulator` (with chip-0057) | path | Provides `block_spends(height) -> Vec<CoinSpend>` (at `simulator.rs:183`) and `block_outputs(height) -> Vec<Coin>` (at `simulator.rs:201`). | Both bindings tests (multi-input round-trip) and the post-refactor `tweak_data_from_simulator_block` use these. |
| `chia-sdk-driver::Action`, `Id`, `SendDestination`, `Spends`, `SpendContext` | path | The send-side action surface for building the multi-input test scenarios. | All multi-input tests (BRIDGE-06) and Rust-side `block_tweak_data.rs::tests` use these. |

### Alternatives Considered (and rejected by CONTEXT.md decisions)

| Instead of | Could Use | Why Rejected |
|------------|-----------|--------------|
| `tweak_data_from_block_spends(coin_spends, additions)` | `tweak_data_from_block(generator_bytes, ...)` taking compressed generator + decompressing internally | D-02 — pulls `chia-consensus` dep, breaks pure-function shape, doesn't match how Python callers (post-decompress in chia_rs) already work. |
| Inline grouping logic in `chia-sdk-test`'s adapter | Centralized algorithm in driver | D-03 — single canonical implementation eliminates drift between simulator helper and real-block helper. |
| Separate public `group_spends_into_tx_groups` API | Single grouping fn called inside helper | D-04 — keeps grouping internal, no surface-area growth. |
| Native pyo3 `Relation` enum (`#[pyclass(eq)]`) instead of opaque-handle | Opaque-handle + factories | D-07 — opaque-handle is descriptor-driven (works for napi/pyo3/wasm uniformly); mirrors `Id` and `SendDestination` precedents reviewers already know; ZERO bindy-macro work. |
| Top-level Relation type-group entry in `bindings.json` | Single descriptor entry in `bindings/action_system.json` | D-08 — Relation is only used by action_system today (`Spends.prepare`); over-broad to put at top-level. |
| Two `prepare` overloads (`prepare(deltas)` and `prepare_with_relation(deltas, relation)`) | Single `prepare(deltas, relation: Option<Relation>)` | D-09 — Option default preserves single-input call sites; no API split. |

**Installation:** Zero new deps. Verify with `cargo machete` post-phase.

**Version verification:**

```bash
# All deps are pinned in [workspace.dependencies]; verify no drift.
grep -E '^(chia-protocol|chia-bls|chia-puzzle-types|chia-sdk-types|chia-sdk-driver|chia-sdk-test|clvm-traits|clvmr) =' /home/kdc/chia-wallet-sdk/Cargo.toml
```

Phase 9 must NOT bump any of these; all are pinned at workspace-canonical values (Rust 1.90.0; chia-* at 0.36.1; chia-puzzles 0.20.3; clvmr 0.16.2 per CLAUDE.md).

## Architecture Patterns

### Recommended File Structure (delta from current tree)

```
crates/chia-sdk-driver/src/silent_payments/
├── mod.rs                    # MODIFIED — append `mod block_tweak_data; pub use block_tweak_data::tweak_data_from_block_spends;`
├── protocol.rs               # UNCHANGED
├── scanner.rs                # UNCHANGED
├── send_keys.rs              # UNCHANGED
├── types.rs                  # UNCHANGED
└── block_tweak_data.rs       # NEW (Pass 2a + Pass 2b grouping + per-group aggregation)

crates/chia-sdk-test/src/silent_payments/
└── tweak_data.rs             # MODIFIED — body collapses to: let spends = sim.block_spends(height); let outputs = sim.block_outputs(height); chia_sdk_driver::silent_payments::tweak_data_from_block_spends(&spends, &outputs).expect("...")
                              # The existing 2 inline tests stay; their semantics are preserved.

crates/chia-sdk-bindings/src/
├── action_system.rs          # MODIFIED — add Relation opaque-handle (~25 lines after SendDestination); change Spends::prepare signature
└── silent_payments.rs        # MODIFIED — add tweak_data_from_block_spends static method to SilentPayments impl

bindings/
├── action_system.json        # MODIFIED — add Relation descriptor (~15 lines after SendDestination); add relation arg to Spends.prepare
└── silent_payments.json      # MODIFIED — add tweak_data_from_block_spends static method to SilentPayments

crates/chia-sdk-driver/src/prelude.rs   # MODIFIED — add tweak_data_from_block_spends to chip-0057 driver re-export block

napi/__test__/silent_payments_multi_input.spec.ts    # NEW
pyo3/tests/test_silent_payments.py                    # MODIFIED — append test_multi_input_e2e
wasm/__test__/silent_payments_multi_input.spec.ts    # NEW

examples/silent_payment.rs    # MODIFIED — append multi-input section per D-12
```

### Pattern 1: Block-level grouping pipeline

**What:** The new `tweak_data_from_block_spends` is a 4-stage pipeline:
1. **Stage 1 — Standard-puzzle filter.** Walk `coin_spends`; for each, parse `puzzle_reveal` via `StandardLayer::parse_puzzle`; skip non-standard puzzles (`Ok(None)`); for standard puzzles, record `(coin_id, synthetic_pk, puzzle_hash)`.
2. **Stage 2a — Same-puzzle-hash grouping.** Group standard-puzzle spends by `puzzle_hash`. Groups of size ≥ 2 are Pass 2a candidates. (Single-spend "groups" go straight to Stage 3 as single-input groups.)
3. **Stage 2b — Concurrent-spend SCC grouping.** For every standard-puzzle spend not already in a Pass 2a group, execute `puzzle.run(solution)` via `chia_sdk_types::run_puzzle` to extract conditions; parse `AssertConcurrentSpend` (opcode 64) targets; build directed graph (coin_id → target_coin_id); compute Tarjan SCCs; SCCs of size ≥ 2 are Pass 2b groups.
4. **Stage 3 — Per-group aggregation + tweak_point emission.** For each group: `A_sum = Σ synthetic_pks`; `input_hash = compute_input_hash(coin_ids, A_sum)`; `tweak_point = A_sum.scalar_multiply(input_hash.to_bytes())`; skip identity element (CHIP §459); push to `tweak_points`.
5. **Stage 4 — Pair with outputs.** Map `additions: &[Coin]` to `Vec<OutputMeta>` (already done identically in simulator helper at `tweak_data.rs:83-92`).

**When to use:** This is the entire helper body. No additional layers.

**Example (skeleton — full impl is BRIDGE-01's task):**
```rust
// crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
// Source pattern: chia-sdk-test/src/silent_payments/tweak_data.rs (Stages 1, 3, 4)
//                 + chia-sdk-types/src/run_puzzle.rs (Stage 2b condition extraction)
//                 + ~/silent-payments/scanner.py:339-421 (Stages 2a + 2b reference algorithm)

pub fn tweak_data_from_block_spends(
    coin_spends: &[CoinSpend],
    additions: &[Coin],
) -> Result<TweakData, DriverError> {
    let mut allocator = Allocator::new();

    // Stage 1: parse standard-puzzle spends.
    struct StandardSpend { coin_id: Bytes32, synthetic_pk: PublicKey, puzzle_hash: Bytes32, puzzle: NodePtr, solution: NodePtr }
    let mut standard_spends: Vec<StandardSpend> = Vec::new();
    for spend in coin_spends {
        let Ok(puzzle_ptr) = spend.puzzle_reveal.to_clvm(&mut allocator) else { continue; };
        let parsed = Puzzle::parse(&allocator, puzzle_ptr);
        let Ok(Some(layer)) = StandardLayer::parse_puzzle(&allocator, parsed) else { continue; };
        let Ok(solution_ptr) = spend.solution.to_clvm(&mut allocator) else { continue; };
        standard_spends.push(StandardSpend {
            coin_id: spend.coin.coin_id(),
            synthetic_pk: layer.synthetic_key,
            puzzle_hash: spend.coin.puzzle_hash,
            puzzle: puzzle_ptr,
            solution: solution_ptr,
        });
    }

    // Stage 2a: group by puzzle_hash.
    let mut ph_groups: IndexMap<Bytes32, Vec<usize>> = IndexMap::new();
    for (i, ss) in standard_spends.iter().enumerate() {
        ph_groups.entry(ss.puzzle_hash).or_default().push(i);
    }
    let mut grouped_indices: HashSet<usize> = HashSet::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (_, indices) in ph_groups {
        if indices.len() >= 2 {
            for &i in &indices { grouped_indices.insert(i); }
            groups.push(indices);
        }
    }

    // Stage 2b: SCC over opcode-64 edges, for spends not in a Pass 2a group.
    // Build graph from coin_id → target_coin_ids (parse conditions via run_puzzle).
    // Run Tarjan; SCCs of size ≥ 2 are Pass 2b groups.
    // (See ~/silent-payments/scanner.py:34-95 for an iterative Tarjan in Python — Rust impl is ~50-80 lines.)
    let id_to_idx: HashMap<Bytes32, usize> = standard_spends.iter().enumerate()
        .filter(|(i, _)| !grouped_indices.contains(i))
        .map(|(i, ss)| (ss.coin_id, i))
        .collect();
    let mut graph: HashMap<usize, Vec<usize>> = id_to_idx.values().map(|&i| (i, Vec::new())).collect();
    for (&coin_id, &i) in &id_to_idx {
        let Ok(output) = chia_sdk_types::run_puzzle(&mut allocator, standard_spends[i].puzzle, standard_spends[i].solution) else { continue; };
        // Walk output as a list of conditions; for each Condition::AssertConcurrentSpend, record edge.
        let Ok(conditions) = Vec::<Condition>::from_clvm(&allocator, output) else { continue; };
        for cond in conditions {
            if let Condition::AssertConcurrentSpend(a) = cond {
                if let Some(&target_i) = id_to_idx.get(&a.coin_id) {
                    graph.get_mut(&i).unwrap().push(target_i);
                }
            }
        }
    }
    let sccs = tarjan_scc(&graph);  // private helper in this file
    for scc in sccs {
        if scc.len() >= 2 {
            groups.push(scc);
        }
    }

    // Stage 3: aggregate per-group + emit tweak_points.
    let mut tweak_points: Vec<PublicKey> = Vec::new();
    for group in groups {
        let coin_ids: Vec<Bytes32> = group.iter().map(|&i| standard_spends[i].coin_id).collect();
        let mut a_sum = standard_spends[group[0]].synthetic_pk;
        for &i in &group[1..] { a_sum += &standard_spends[i].synthetic_pk; }
        let input_hash = compute_input_hash(&coin_ids, &a_sum);
        let mut tweak_point = a_sum;
        tweak_point.scalar_multiply(&input_hash.to_bytes());
        if !tweak_point.is_inf() {  // CHIP §459
            tweak_points.push(tweak_point);
        }
    }

    // Also emit a tweak_point for any single-spend "group" (single-input SP send).
    // (See Pitfall 4 — the simulator helper aggregates ALL spends as one group; the new
    // algorithm treats single-input SP sends as 1-element "groups" but must still emit
    // a tweak_point for them.)
    let standalone: Vec<usize> = (0..standard_spends.len())
        .filter(|i| !grouped_indices.contains(i) && !groups.iter().any(|g| g.contains(i)))
        .collect();
    for i in standalone {
        let coin_ids = vec![standard_spends[i].coin_id];
        let a_sum = standard_spends[i].synthetic_pk;
        let input_hash = compute_input_hash(&coin_ids, &a_sum);
        let mut tweak_point = a_sum;
        tweak_point.scalar_multiply(&input_hash.to_bytes());
        if !tweak_point.is_inf() { tweak_points.push(tweak_point); }
    }

    // Stage 4: pair with additions.
    let outputs: Vec<OutputMeta> = additions.iter().map(|coin| OutputMeta {
        puzzle_hash: coin.puzzle_hash,
        coin_id: coin.coin_id(),
        amount: coin.amount,
        parent_coin_id: coin.parent_coin_info,
    }).collect();

    Ok(TweakData { tweak_points, outputs })
}
```

⚠️ Above is a research skeleton, not a plan-ready implementation. The planner refines: deterministic group ordering (so output is reproducible for byte-equality tests), Tarjan SCC implementation choice (iterative vs recursive — iterative is safer for adversarial deep graphs), `Condition::from_clvm` slice walking (the chia-sdk-types path may use a different iterator pattern).

### Pattern 2: Opaque-handle binding for Rust enum (D-07)

**What:** A 5-method Rust struct that wraps the enum opaquely; descriptor entry declares factories + introspectors. No native pyo3/wasm/napi enum surface.

**When to use:** When binding a simple enum to all three targets and the enum's surface is "construct + check variant + extract value (if any)". Used for `Id`, `SendDestination`, and now `Relation`.

**Example (verbatim from CONTEXT.md D-07 + locked precedent):**
```rust
// crates/chia-sdk-bindings/src/action_system.rs (append after SendDestination, ~line 541)
/// Cross-binding handle for `chia_sdk_driver::Relation`.
///
/// Multi-input SP sends require `Relation::AssertConcurrent` so the scanner's
/// Pass 2b SCC detection can group the bundle's coins (`Spends::prepare`
/// returns `Err(DriverError::SilentPaymentRequiresInputBinding)` otherwise).
/// Pass `Relation.assert_concurrent()` as the second arg to `Spends.prepare`
/// for any ≥2-coin SP bundle; single-input sends accept `None` / omit.
///
/// Mirrors the `Id` and `SendDestination` opaque-handle pattern in this file.
#[derive(Clone, Debug)]
pub struct Relation(pub(crate) sdk::Relation);

impl Relation {
    pub fn none() -> Result<Self> { Ok(Self(sdk::Relation::None)) }
    pub fn assert_concurrent() -> Result<Self> { Ok(Self(sdk::Relation::AssertConcurrent)) }
    pub fn is_none(&self) -> Result<bool> { Ok(matches!(self.0, sdk::Relation::None)) }
    pub fn is_assert_concurrent(&self) -> Result<bool> { Ok(matches!(self.0, sdk::Relation::AssertConcurrent)) }
    pub fn equals(&self, other: Relation) -> Result<bool> { Ok(self.0 == other.0) }
}
```

```json
// bindings/action_system.json — append after SendDestination block (~line 290)
"Relation": {
  "type": "class",
  "methods": {
    "none": { "type": "factory" },
    "assert_concurrent": { "type": "factory" },
    "is_none": { "return": "bool" },
    "is_assert_concurrent": { "return": "bool" },
    "equals": {
      "args": { "other": "Relation" },
      "return": "bool"
    }
  }
}
```

### Pattern 3: `Spends.prepare(deltas, relation)` signature extension (D-09)

**What:** Add `relation: Option<Relation>` as a second arg; map `None` → `sdk::Relation::None`; pass through `Some(r) → r.0`.

**Example:**
```rust
// crates/chia-sdk-bindings/src/action_system.rs:177 (replacing current body lines 177-208)
pub fn prepare(&self, deltas: Deltas, relation: Option<Relation>) -> Result<FinishedSpends> {
    let mut spends = self.spends.lock().unwrap();

    let change_puzzle_hash = spends.change_puzzle_hash;
    let spends = std::mem::replace(&mut *spends, sdk::Spends::new(change_puzzle_hash));

    let mut ctx = self.clvm.lock().unwrap();

    let relation = relation.map_or(Relation::None, |r| r.0);  // NEW (was hardcoded)

    let spends = spends.prepare(&mut ctx, &deltas.0, relation)?;

    // ... rest of body unchanged
}
```

```json
// bindings/action_system.json::Spends.methods.prepare (update existing entry at line 62)
"prepare": {
  "args": {
    "deltas": "Deltas",
    "relation": "Option<Relation>"
  },
  "return": "FinishedSpends"
}
```

### Pattern 4: SilentPayments namespace static method for block helper (D-14)

**What:** Append a 4th static method to the `impl SilentPayments` block in `chia-sdk-bindings::silent_payments`.

**Example:**
```rust
// crates/chia-sdk-bindings/src/silent_payments.rs (inside impl SilentPayments {}, append after aggregate_sender_sks at line 432)
pub fn tweak_data_from_block_spends(
    coin_spends: Vec<chia_protocol::CoinSpend>,
    additions: Vec<chia_protocol::Coin>,
) -> Result<TweakData> {
    let driver_td = chia_sdk_driver::silent_payments::tweak_data_from_block_spends(
        &coin_spends, &additions,
    )?;
    Ok(driver_td.into())  // From<chia_sdk_driver::TweakData> for TweakData already exists at line 251.
}
```

```json
// bindings/silent_payments.json::SilentPayments.methods — append after aggregate_sender_sks
"tweak_data_from_block_spends": {
  "type": "static",
  "args": {
    "coin_spends": "Vec<CoinSpend>",
    "additions": "Vec<Coin>"
  },
  "return": "TweakData"
}
```

### Pattern 5: Cross-binding multi-input test rhythm (BRIDGE-06)

**What:** Each binding test follows the BIND-03 napi/pyo3/wasm scaffold from Phase 6, with: (a) `sim.bls(N)` × 2 to farm two non-ephemeral XCH coins; (b) `spends.add_xch(coin1)` + `spends.add_xch(coin2)`; (c) one `Action.send(Id.xch(), SendDestination.silent_payment(addr), amount, None)`; (d) `spends.with_silent_payment_keys([k1, k2], [s1, s2])`; (e) `spends.prepare(deltas, Relation.assert_concurrent())` — NEW; (f) the rest follows BIND-03.

**Critical verification step:** `Spends.prepare` MUST receive `Relation.assert_concurrent()` — without it, the binding-side `prepare` will pass `Relation::None`, hit the driver-side `non_ephemeral_xch_count >= 2 && !AssertConcurrent` gate at `spends.rs:604`, and return `Err(SilentPaymentRequiresInputBinding)`. The test fails with a clear error if either D-09 mapping or descriptor wiring is wrong.

### Anti-Patterns to Avoid

- **DON'T add a public `tarjan_scc` helper outside the new module.** D-04 keeps grouping internal. A future SP service might benefit from a public Tarjan, but Phase 9's scope is one helper; keep all grouping internals private to `block_tweak_data.rs`.
- **DON'T add `Relation` to top-level `bindings.json::type_groups`.** D-08 — descriptor entry lives in `action_system.json` only.
- **DON'T preserve the simulator-helper's single-group simplification.** D-04 requires the new helper to handle multiple SP-using transactions per block. The simulator-helper-style "aggregate all standard-puzzle spends as one group" is wrong for real blocks; the new algorithm computes groups properly and the simulator helper inherits the correct behavior automatically (single-tx-per-block in simulator → exactly one group in output, byte-equal to pre-refactor for SIM-01..03 tests).
- **DON'T use a recursive Tarjan.** Real Chia blocks can carry adversarial spends; iterative Tarjan (per the reference Python at `scanner.py:34-95`) prevents stack-overflow DOS.
- **DON'T introduce `#[allow]` attributes.** Workspace policy + Phase 1..8 invariant. The one existing `#[allow]` on `scanner.rs::scan_from_tweaks` (Plan 03-03) is the project's permanent ceiling.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Running a puzzle to extract its conditions | A custom `clvmr::run_program` invocation | `chia_sdk_types::run_puzzle(&mut allocator, puzzle, solution)` at `crates/chia-sdk-types/src/run_puzzle.rs:9` | Already configured with `ENABLE_KECCAK_OPS_OUTSIDE_GUARD` + `MAINNET_CONSTANTS.max_block_cost_clvm`; matches what the rest of the SDK uses (e.g., `p2_eip712_message_layer.rs:274`). |
| Parsing standard-puzzle reveals | Manual `StandardArgs::from_clvm` + `CurriedProgram` decoding | `StandardLayer::parse_puzzle(&allocator, puzzle)` — returns `Ok(None)` for non-standard. | Already used by `tweak_data_from_simulator_block`; one canonical defensive parse path. |
| Parsing `AssertConcurrentSpend` (opcode 64) | Manual atom/pair walk + integer decode | `Vec::<Condition>::from_clvm(&allocator, output)` then `match cond { Condition::AssertConcurrentSpend(a) => ... }` | `chia-sdk-types` already has the `Condition` enum with all standard-puzzle condition variants typed; opcode 64 lives there. |
| Computing per-group `input_hash` | Manual `tagged_hash` invocation | `chia_sdk_driver::silent_payments::compute_input_hash(&coin_ids, &a_sum)` at `protocol.rs:182` | Already pinned to TVs; preserves lex-min coin_id semantics; emits `ScalarField` (unsigned mod-r). |
| Pairing additions with `OutputMeta` | Custom struct construction | The exact 6-line block at `chia-sdk-test/src/silent_payments/tweak_data.rs:83-92` | Copy verbatim; the simulator helper got this right. |
| BLS scalar multiplication | Manual scalar mult via raw chia-bls | `PublicKey::scalar_multiply(&[u8; 32])` (in-place; already used at `tweak_data.rs:75`) | One canonical scalar-multiply path; preserves type invariants. |
| Identity-element check (CHIP §459) | Custom byte comparison | `PublicKey::is_inf()` (used at `tweak_data.rs:78`) | Pre-existing `chia-bls` method; semantics correct. |
| Cross-language enum binding | Native `#[pyclass(eq)]` + duplicate napi/wasm-bindgen impls | Opaque-handle + factories (D-07 — mirror `Id` precedent) | Descriptor-driven; no per-target hand-code; consistent reviewer experience. |
| Tarjan SCC implementation | Adapt a generic graph crate | A focused ~80-line iterative Tarjan implementation in `block_tweak_data.rs` (private) | No new workspace dep (CLAUDE.md ban); the algorithm is small + well-understood; the reference Python at `scanner.py:34-95` is the reading model. |

**Key insight:** Phase 9 adds zero new conceptual building blocks. Every primitive needed already exists in the workspace. The work is composition (and iterative Tarjan, which is the only "new" code that isn't pure assembly of existing parts).

## Runtime State Inventory

> Include this section for rename/refactor/migration phases only. Omit entirely for greenfield phases.

Phase 9 is **additive** (new helper, new binding, signature extension that preserves old call sites via `Option<>` default), not a rename/refactor/migration. The single state-affecting change is BRIDGE-04's `Spends.prepare` signature, and that is descriptor-driven (every napi/pyo3/wasm caller regenerates the typed surface from `bindings/action_system.json` at build time). No databases, no live services, no OS-registered state, no secrets, no installed packages outside the standard `npm install` / `maturin develop` / `wasm-pack build` cycles.

**Nothing found in any category — verified by:** (a) reading `bindings/action_system.json` (no DB-backed state), (b) confirming the change is API-level only (no on-chain state, no `chia-sdk-test` Simulator persistence layer), (c) confirming chip-0057 feature wiring is already in place from Phases 5-7 (no `Cargo.toml` feature gates change).

## Common Pitfalls

### Pitfall 1: Pass 2b pollution attack (HIGHEST RISK)

**What goes wrong:** Third-party spend in the same block emits a single `AssertConcurrentSpend` pointing at a legitimate SP send's coin. If the helper uses **undirected** connected components (or any weaker grouping than strongly-connected), the polluter gets merged into the legit group, A_sum gets polluted with the polluter's synthetic_pk, the input_hash diverges, and the receiver's scanner misses the payment entirely.

**Why it happens:** The cycle that the SDK emits for `Relation::AssertConcurrent` is a **closed cycle** (coin 0 → coin N-1, coin i → coin i-1) — it forms a strongly connected component. A polluter's single forward edge to a victim coin is NOT a cycle; with SCC, the polluter sits in its own trivial SCC. Naive undirected CC merges everything.

**How to avoid:** Use Tarjan (or any SCC algorithm); never undirected CC. Verified in the reference Python at `~/silent-payments/scanner.py:34-95,349-355` (uses iterative Tarjan, explicitly documents the choice).

**Warning signs:** The Pass 2b pollution attack test (D-13) is the canonical regression — IF that test passes, this pitfall is closed. The planner MUST land that test in BRIDGE-01.

### Pitfall 2: Allocator reuse across many spends

**What goes wrong:** Re-using a single `Allocator` for every spend's puzzle+solution parse leaks memory across coin spends in a real block (which may have hundreds of standard-puzzle spends).

**Why it happens:** `clvmr::Allocator` is per-program. Used correctly in the simulator helper at `tweak_data.rs:44-60` (one allocator per call). But Phase 9's helper additionally runs `run_puzzle` for Stage 2b, which appends to the same allocator — for a 100-spend block, ~100 puzzle runs land in one allocator before it's dropped.

**How to avoid:** ONE `Allocator::new()` per helper call is correct for v1 (matches simulator helper). Real-block correctness: an allocator can hold the entire block's parse + run state simultaneously (the allocator caps are designed for full-block consensus runs, well above what a block's spends can produce). Don't allocator-per-spend — the cost is conversion of `PublicKey`/`Bytes32` references back, and they don't live in the allocator. Don't allocator-per-stage — same reason.

**Warning signs:** OOM or absurd allocator growth in BRIDGE-01 multi-input tests with synthetic large blocks. Unlikely at v1 sizes.

### Pitfall 3: Puzzle execution cost (Stage 2b is the expensive stage)

**What goes wrong:** Running `puzzle.run(solution)` per non-Pass-2a spend is non-trivial cost — a standard-puzzle run is typically a few million CLVM cost units. On a 100-standard-puzzle-spend block where 99 are non-Pass-2a, that's ~100 runs.

**Why it happens:** Pass 2b requires actually executing the puzzle to extract conditions. There's no shortcut via static analysis.

**How to avoid:** Don't double-process. Once a spend lands in a Pass 2a group, skip it from Stage 2b's input set (the planner skeleton above uses `grouped_indices: HashSet<usize>`). Document the cost characteristic in the helper's rustdoc.

**Warning signs:** Wall-clock test time of multi-input tests should stay under 1-2 seconds. If it spikes, profile run_puzzle invocations.

### Pitfall 4: Simulator helper currently aggregates ENTIRE block as one group (algorithm divergence at refactor boundary)

**What goes wrong:** The existing `tweak_data_from_simulator_block` does NOT do Pass 2a/2b at all — it builds a SINGLE `synthetic_pks` Vec from every standard-puzzle spend in the block (file `tweak_data.rs:45-60`) and emits ONE tweak_point. After D-03 refactors it to delegate to `tweak_data_from_block_spends`, the new helper's grouping behavior differs from the old helper's "everything is one group" assumption.

**Why it happens:** The simulator convention is one transaction per block. So in practice, the existing helper's "aggregate everything" matches "aggregate the one transaction's spends" — i.e., it accidentally produces the right answer for simulator-only inputs. The new helper does proper grouping.

**How to avoid:** For SINGLE-transaction simulator blocks (where every standard-puzzle spend belongs to the same Pass 2a or Pass 2b group), the new helper MUST produce the same `tweak_points` as the old helper. The 3 Phase 6 simulator tests (`test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change`) are the regression oracles — they must pass byte-identically post-refactor.

**Subtle interaction:** In the single-input case (one XCH coin → one SP recipient), the spend has NO `AssertConcurrentSpend` (Phase 4.1 verified at `spends.rs:380` — the cycle is only emitted for N≥2 inputs). So Stage 2b emits no edges, and the spend sits in its own trivial SCC. The "standalone" branch at the bottom of the helper skeleton above is what handles this case — emit a tweak_point for each ungrouped standard-puzzle spend.

**Warning signs:** Phase 6 simulator tests fail byte-equality post-D-03 refactor. If this happens, the new helper's "standalone single-spend" branch is missing or buggy.

### Pitfall 5: napi vs pyo3 vs wasm null/None/undefined semantics for `Option<Relation>` (BRIDGE-04)

**What goes wrong:** Each binding target marshals `Option<T>` differently:
- **napi:** `None` ↔ `null` (verified in pyo3 test patterns — `assert detection.label is None`); also accepts `undefined` for optional args.
- **pyo3:** `None` ↔ Python `None`.
- **wasm-bindgen:** `None` ↔ `undefined` (Phase 6 04-SUMMARY documented this — "wasm-bindgen emits `Option<u32>` as `number | undefined` (vs napi's `number | null`)").

**Why it happens:** Each FFI surface has its own optional-arg convention. bindy-macro abstracts this but downstream test authors must use the right convention per target.

**How to avoid:** Test scaffolding in BRIDGE-06 mirrors Phase 6's BIND-03 tests:
- Python: `spends.prepare(deltas)` (no relation arg — Python supports trailing-arg omission if bindy generates with default) OR `spends.prepare(deltas, None)` OR `spends.prepare(deltas, Relation.assert_concurrent())`.
- TypeScript (napi): `spends.prepare(deltas)` may not work if bindy generates a strict 2-arg signature; safer to pass `undefined` or `null` explicitly: `spends.prepare(deltas, undefined)` / `spends.prepare(deltas, Relation.assertConcurrent())`.
- TypeScript (wasm): `spends.prepare(deltas, undefined)` / `spends.prepare(deltas, Relation.assertConcurrent())`.

**Open verification step:** During BRIDGE-04 plan execution, BUILD all three targets and inspect generated `.d.ts` / `.pyi` to confirm the actual signature shape. If bindy generates `prepare(deltas, relation: Relation | null)` for napi vs `prepare(deltas, relation?: Relation)` for wasm, the multi-input tests need to use the matching shorthand. Past precedent: bindy-macro is consistent across targets but each target's "default arg" syntax may differ.

**Warning signs:** Existing single-input pyo3 test (`test_unlabeled_e2e` at line 77: `finished = spends.prepare(deltas)`) breaks during BRIDGE-04 because it doesn't pass the new `relation` arg. If this fires, the binding signature change requires explicit `relation=None` Python keyword arg OR bindy generates trailing-arg-defaultable, OR the test must be touched (last resort — CONTEXT.md acceptance forbids "signature changes beyond the optional `Relation` parameter," so the planner must ensure trailing-arg defaulting works in pyo3).

### Pitfall 6: Binding tests need `Simulator.tweakDataFromBlock` AND `SilentPayments.tweakDataFromBlockSpends` reachable

**What goes wrong:** Test authors might assume BRIDGE-06 tests use the simulator helper bound at Phase 6 (`sim.tweakDataFromBlock(height)`), but the test's whole point is to exercise the NEW helper (`SilentPayments.tweakDataFromBlockSpends(spends, additions)`).

**Why it happens:** The simulator binding already works end-to-end (Phase 6); using it would silently bypass BRIDGE-05's new entry point.

**How to avoid:** BRIDGE-06 tests MUST call `SilentPayments.tweakDataFromBlockSpends(sim.block_spends(...), sim.block_outputs(...))` via the binding, not `sim.tweakDataFromBlock(...)`. To make `sim.block_spends` and `sim.block_outputs` reachable from bindings, those `Simulator` methods need binding facade entries. **Verification:** `bindings/simulator.json` currently does NOT expose `block_spends` or `block_outputs`; they're public on `chia-sdk-test::Simulator` (verified at `simulator.rs:183,201`) but not on the binding facade `chia-sdk-bindings::Simulator`. Phase 9 may need to bind them in BRIDGE-06's scope, OR tests could construct synthetic `Vec<CoinSpend>` + `Vec<Coin>` fixtures (less faithful).

**Recommended approach:** Add `Simulator::block_spends(height)` and `Simulator::block_outputs(height)` to the binding facade + descriptor as part of BRIDGE-06 (small additive change, mirrors existing `Simulator` methods like `coin_spend` and `coin_state`). This makes the multi-input test self-contained and runs the new helper through its full intended path.

**Warning signs:** Multi-input test authors reach for `sim.tweak_data_from_block(height)` (the OLD path) and BRIDGE-05's binding entry never gets runtime coverage.

### Pitfall 7: Cross-allocator NodePtr mix-up in TypeScript tests (Phase 6 lesson)

**What goes wrong:** Constructing conditions in one `Clvm` allocator and consuming them in a different `Clvm` allocator panics in clvmr with "index out of bounds".

**Why it happens:** Each `Clvm` instance is its own `Allocator`. NodePtrs are allocator-local.

**How to avoid:** Pattern documented in Phase 6 06-04 SUMMARY (deviation #2): always build conditions in the same `Clvm` allocator that will execute the spend. Phase 6's BIND-03 tests fixed this; BRIDGE-06's multi-input tests can copy-paste those test scaffolds verbatim.

**Warning signs:** napi/wasm multi-input test panics with `"index out of bounds: the len is 127 but the index is 135"` (the exact error from Phase 6 deviation).

### Pitfall 8: `coin_spends.is_empty()` and `additions.is_empty()` edge cases

**What goes wrong:** `tweak_data_from_block_spends(&[], &[])` should return `Ok(TweakData { tweak_points: vec![], outputs: vec![] })` — not error.

**Why it happens:** Some downstream callers may pass in empty blocks (testnet blocks with no SP-using transactions).

**How to avoid:** `compute_input_hash` panics on empty `coin_ids` (verified at `protocol.rs:183`). The helper must skip the per-group aggregation entirely when no standard-puzzle spends exist (Stage 1 produces empty list → Stages 2a/2b/3 skip naturally → Stage 4 emits empty outputs if `additions` is empty). The simulator helper's empty-block test (`tweak_data_empty_block_returns_empty_tweak_data` at line 105-111) is the regression oracle.

**Warning signs:** Empty-block test in `block_tweak_data.rs::tests` (D-13) panics. If it does, the per-group loop is unguarded.

### Pitfall 9: Determinism of group iteration order matters for byte-equality tests

**What goes wrong:** Iterating `HashMap<Bytes32, Vec<usize>>` for Pass 2a produces nondeterministic order; SCCs from Tarjan have a deterministic order BUT only if the graph is built deterministically.

**Why it happens:** Rust's `HashMap` uses a randomly-seeded hash. Iteration order changes between runs.

**How to avoid:** Use `IndexMap<Bytes32, Vec<usize>>` for Pass 2a grouping (insertion-order); build the Stage 2b graph in `coin_spends` input-order (which is the simulator/coinset-API order); emit tweak_points in the order: Pass 2a groups (in puzzle-hash insertion order) → Pass 2b SCCs (in Tarjan finishing order) → standalone singletons (in input order). The BRIDGE-01 byte-equality test against the simulator helper depends on this; document the ordering contract in the helper rustdoc.

**Warning signs:** BRIDGE-01 byte-equality regression test passes locally but flakes in CI. Cause: HashMap iteration order.

## Code Examples

Verified patterns from current source. Each code block cites exact file:line.

### Pattern: Defensive standard-puzzle parse (Stage 1)
```rust
// Source: crates/chia-sdk-test/src/silent_payments/tweak_data.rs:51-60 (verified 2026-05-29)
let Ok(ptr) = spend.puzzle_reveal.to_clvm(&mut allocator) else {
    continue;
};
let puzzle = Puzzle::parse(&allocator, ptr);
let Ok(Some(layer)) = StandardLayer::parse_puzzle(&allocator, puzzle) else {
    continue;
};
synthetic_pks.push(layer.synthetic_key);
spent_coin_ids.push(spend.coin.coin_id());
```

### Pattern: Per-group aggregation + input_hash + tweak_point (Stage 3)
```rust
// Source: crates/chia-sdk-test/src/silent_payments/tweak_data.rs:63-80 (verified 2026-05-29)
let mut agg = synthetic_pks[0];
for pk in &synthetic_pks[1..] {
    agg += pk;
}
let input_hash: ScalarField = compute_input_hash(&spent_coin_ids, &agg);
let mut tweak_point = agg;
tweak_point.scalar_multiply(&input_hash.to_bytes());
if !tweak_point.is_inf() {  // CHIP §459
    tweak_points.push(tweak_point);
}
```

### Pattern: Puzzle+solution → conditions extraction (Stage 2b)
```rust
// Source: crates/chia-sdk-types/src/run_puzzle.rs:9-22 (verified 2026-05-29)
// Combined with crates/chia-sdk-types/src/conditions.rs Condition enum:
let Ok(output) = chia_sdk_types::run_puzzle(&mut allocator, puzzle_ptr, solution_ptr) else {
    continue;
};
let Ok(conditions) = Vec::<Condition>::from_clvm(&allocator, output) else {
    continue;
};
for cond in &conditions {
    if let Condition::AssertConcurrentSpend(a) = cond {
        // a.coin_id is the asserted (target) coin id
        // record edge: this_spend.coin_id → a.coin_id
    }
}
```

### Pattern: Opaque-handle Rust binding (matches Id at action_system.rs:449-486)
See `## Architecture Patterns → Pattern 2` above. Source verified at `crates/chia-sdk-bindings/src/action_system.rs:449-486` for `Id` and `:504-540` for `SendDestination`.

### Pattern: Spends.prepare signature extension (D-09)
See `## Architecture Patterns → Pattern 3` above. Source for current state: `crates/chia-sdk-bindings/src/action_system.rs:177-208`.

### Pattern: Multi-input round-trip test skeleton (BRIDGE-06 — adapted from Phase 6 BIND-03 napi)
```typescript
// napi/__test__/silent_payments_multi_input.spec.ts (NEW per BRIDGE-06)
// Adapted from napi/__test__/silent_payments_e2e.spec.ts (verified 2026-05-29).
// Key delta vs single-input BIND-03: 2 sender BLS pairs, 2 add_xch calls, 2
// register_key entries, prepare(deltas, Relation.assertConcurrent()).
import test from "ava";
import {
  Action, Clvm, Id, LabelRegistry, Mnemonic,
  Relation,                            // NEW (BRIDGE-03)
  SendDestination, SilentPaymentKeys, SilentPaymentNetwork,
  SilentPaymentRegisteredKey, SilentPaymentRegisteredSecretKey,
  SilentPayments, Simulator, Spends,
} from "..";

const TV1_MNEMONIC = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const K_MAX_DEFAULT = 2400;

test("BRIDGE-06 napi: multi-input SP send → tweak_data_from_block_spends → scan", (t) => {
  const sim = new Simulator();
  const clvm = new Clvm();

  const recipient = SilentPaymentKeys.fromMnemonic(new Mnemonic(TV1_MNEMONIC));
  const recipientAddress = recipient.unlabeledAddress(SilentPaymentNetwork.Testnet);

  // TWO sender coins (different BLS pairs OK — Pass 2b will glue them via SCC).
  const sender1 = sim.bls(500n);
  const sender2 = sim.bls(500n);
  const heightBefore = sim.height();

  const spends = new Spends(clvm, sender1.puzzleHash);
  spends.addXch(sender1.coin);
  spends.addXch(sender2.coin);

  const actions = [
    Action.send(Id.xch(), SendDestination.silentPayment(recipientAddress), 700n, undefined),
  ];

  spends.withSilentPaymentKeys(
    [
      new SilentPaymentRegisteredKey(sender1.puzzleHash, sender1.pk),
      new SilentPaymentRegisteredKey(sender2.puzzleHash, sender2.pk),
    ],
    [
      new SilentPaymentRegisteredSecretKey(sender1.puzzleHash, sender1.sk),
      new SilentPaymentRegisteredSecretKey(sender2.puzzleHash, sender2.sk),
    ],
  );

  const deltas = spends.apply(actions);

  // NEW (BRIDGE-04): Relation.assert_concurrent() unlocks multi-input SP send.
  // Without this, Spends.prepare returns SilentPaymentRequiresInputBinding.
  const finished = spends.prepare(deltas, Relation.assertConcurrent());

  for (const pending of finished.pendingSpends()) {
    // Use the secret key matching this coin's puzzle hash.
    const sk = (pending.coin().puzzleHash === sender1.puzzleHash) ? sender1.sk : sender2.sk;
    const pk = (pending.coin().puzzleHash === sender1.puzzleHash) ? sender1.pk : sender2.pk;
    finished.insert(
      pending.coin().coinId(),
      clvm.standardSpend(pk, clvm.delegatedSpend(pending.conditions())),
    );
  }
  finished.spend();

  sim.spendCoins(clvm.coinSpends(), [sender1.sk, sender2.sk]);

  // NEW (BRIDGE-05): call the NEW helper, not sim.tweakDataFromBlock.
  // Requires Simulator::block_spends / block_outputs binding — see Pitfall 6.
  const blockSpends = sim.blockSpends(heightBefore);
  const blockAdditions = sim.blockOutputs(heightBefore);
  const tweakData = SilentPayments.tweakDataFromBlockSpends(blockSpends, blockAdditions);
  t.is(tweakData.tweakPoints.length, 1, "one SP transaction → one tweak_point");

  const labels = new LabelRegistry();
  const detections = SilentPayments.scanFromTweaks(
    recipient.scanSk(), recipient.spendSk(), recipient.spendPk(),
    tweakData, labels, K_MAX_DEFAULT,
  );

  t.is(detections.length, 1, "scanner finds exactly one SP output");
  t.is(detections[0].k, 0);
  t.is(detections[0].label, null);
  t.is(detections[0].amount, 700n, "multi-input SP output amount round-trips");
});
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-group block aggregation (every standard-puzzle spend → one tweak_point) | Pass 2a + Pass 2b grouping (multiple tweak_points per block) | Phase 9 (this work) | Real-block correctness — multi-tx blocks. Simulator behavior unchanged (1 tx/block → 1 tweak_point still). |
| Hardcoded `Relation::None` in binding `Spends.prepare` | `Option<Relation>` parameter | Phase 9 | Multi-input SP sends become possible from any binding target. |
| Simulator-only block helper (chia-sdk-test) | Driver-side helper (chia-sdk-driver) + simulator thin adapter | Phase 9 D-03 | Production-shape callers no longer pull `chia-sdk-test` (Simulator dep). |
| `Spends::finish_silent_payments` public method (Phase 6) | `sp_finish_branch` fires inside `Spends::prepare` (Phase 7 CLEANUP-03) | Phase 7 (already shipped) | Binding-side `prepare` already drives the SP branch via the driver's `prepare` — Phase 9 only needs to thread `Relation`, no SP-composition re-plumbing. |
| Opcode 60/61 announcement binding (Phase 4 single-shot) | `Relation::AssertConcurrent` cycle binding (Phase 4.1) | Phase 4.1 (already shipped) | The cycle pattern Phase 9's Pass 2b scanner exploits — the SDK already emits it; Phase 9 makes the binding-side surface able to request it. |

**Deprecated/outdated:** None. Phase 9 is purely additive; no removals.

## Open Questions

1. **Bindy default-arg generation per target — does `prepare(deltas)` keep working in pyo3 with the new `Option<Relation>` arg?**
   - **What we know:** Phase 6 added `Option<u32>` returns and they marshal correctly; `Option<Bytes32>`/`Option<Id>`/`Option<TransferNftById>` parameters are well-precedented in `bindings/action_system.json`. CONTEXT.md acceptance bullet says "existing pyo3 E2E stays green with no signature changes beyond the optional `Relation` parameter" — implies bindy DOES generate trailing-arg-optional in pyo3.
   - **What's unclear:** Whether `spends.prepare(deltas)` (positional, no kwarg) Just Works in pyo3 with the new signature, OR whether the test needs `spends.prepare(deltas, relation=None)`. Same question for `spends.prepare(deltas)` in napi/wasm — bindy may generate `prepare(deltas, relation?: Relation)` (TS optional) vs strict `prepare(deltas, relation: Relation | null)` (TS nullable required).
   - **Recommendation:** Plan a quick pre-flight in BRIDGE-04: change the signature, run `pnpm build` (napi) / `maturin develop` (pyo3) / `wasm-pack build` (wasm), inspect generated `.d.ts` / `.pyi`. If trailing-optional works on all targets, no test edits needed. If napi/wasm require explicit `undefined`, update tests minimally (single-input napi test in `silent_payments.spec.ts` and BIND-03 napi/wasm tests in `silent_payments_e2e.spec.ts` may need a 1-character `, undefined` insertion). The pyo3 test almost certainly works unchanged.

2. **Should `Simulator::block_spends` / `block_outputs` get binding entries in Phase 9 scope?**
   - **What we know:** They're public on `chia-sdk-test::Simulator` but not on the binding facade. BRIDGE-06 multi-input tests need them to construct the input to `SilentPayments.tweakDataFromBlockSpends`.
   - **What's unclear:** Whether the planner should fold these in (1 facade method + 1 descriptor entry × 2 methods = 4 small additions) or use a different test approach (synthetic fixtures, or bridge via `sim.tweakDataFromBlock` for the OLD path while using the NEW helper for a SYNTHETIC second test).
   - **Recommendation:** Fold them in. Mirror `Simulator::tweak_data_from_block` precedent at `chia-sdk-bindings/src/simulator.rs:93`. Two ~5-line facade methods + two descriptor entries. Keeps BRIDGE-06 tests honest (run the full new helper through bindings against real simulator state).

3. **Group ordering contract for byte-equality test (Pitfall 9).**
   - **What we know:** The simulator helper produces a single tweak_point in a deterministic order (because it aggregates everything into one group). The new helper produces multiple tweak_points; their order must be defined.
   - **What's unclear:** What ordering the byte-equality test against the simulator helper expects. If the simulator helper always produces exactly one tweak_point (because simulator convention is 1-tx-per-block → 1 group), the byte-equality test only ever sees a 1-element Vec, so ordering is moot for simulator-helper-parity.
   - **Recommendation:** Document the helper's ordering contract in rustdoc: "tweak_points are emitted in the order: Pass 2a groups in puzzle-hash insertion order; Pass 2b SCCs in Tarjan finishing order; standalone single-spends in `coin_spends` input order." The simulator parity tests (Pitfall 4 oracles) inherit this trivially because they only have 1 group.

4. **Pass 2b SCC algorithm choice — iterative Tarjan vs Kosaraju vs path-based?**
   - **What we know:** Reference Python uses iterative Tarjan (verified at `scanner.py:34-95`). Iterative is safer than recursive for adversarial deep graphs.
   - **What's unclear:** Whether to copy the Python impl verbatim (translate to Rust) or write a clean Rust impl from scratch. Both are ~80 lines.
   - **Recommendation:** Translate the Python impl verbatim, with comments citing `~/silent-payments/scanner.py:34-95` as the reference algorithm. Don't reinvent. NOTE: don't cite the Python file in PR comments (CLEANUP-01 grep ban on planning artifacts) — cite "iterative Tarjan SCC" or CHIP §"Scanner Grouping Strategies" instead.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain 1.90.0 | All Rust crates | ✓ | rustc 1.90.0 (verified) | — |
| cargo 1.90 | Builds | ✓ | cargo 1.90.0 (verified) | — |
| Node + pnpm (≥9) | napi binding build + tests | Assumed ✓ (Phase 5-8 used it) | — | — |
| Python 3.8+ + maturin + pytest | pyo3 binding build + tests | Assumed ✓ (`pyo3/.venv` populated in Phase 5) | — | — |
| wasm-pack (cargo-installed, 0.13.1 per Phase 5 pin) | wasm binding build + tests | Assumed ✓ (Phase 5 noted version pin) | 0.13.1 | — |
| `chia_sdk_test::Simulator` (with chip-0057 feature) | BRIDGE-06 multi-input tests | ✓ (verified — `chia-sdk-bindings/Cargo.toml` enables chip-0057 on chia-sdk-test transitively) | — | — |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework (Rust) | `cargo test` (rstest available for parametric; not needed for Phase 9) |
| Framework (napi) | AVA via `pnpm test` in `napi/` |
| Framework (pyo3) | pytest via `python -m pytest` in `pyo3/` (venv at `pyo3/.venv`) |
| Framework (wasm) | AVA via `pnpm test` in `wasm/` |
| Config file (Rust) | `Cargo.toml` workspace (no per-crate test config) |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data` |
| Full suite (Rust) | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` (mirrors CI per CLAUDE.md) |
| Full suite (napi) | `cd napi && pnpm test` |
| Full suite (pyo3) | `cd pyo3 && python -m pytest tests/` |
| Full suite (wasm) | `cd wasm && pnpm test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BRIDGE-01 | `tweak_data_from_block_spends` correctly groups spends by puzzle_hash (Pass 2a) | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_pass_2a_same_puzzle_hash_grouping` | ❌ Wave 0 (new test in new file) |
| BRIDGE-01 | `tweak_data_from_block_spends` correctly groups via SCC over opcode-64 edges (Pass 2b multi-input) | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_multi_input_round_trip` | ❌ Wave 0 |
| BRIDGE-01 | Pass 2b pollution attack resistance (third-party AssertConcurrentSpend does NOT merge into legit SCC) | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_pass_2b_pollution_resistance` | ❌ Wave 0 (CRITICAL — Pitfall 1 oracle) |
| BRIDGE-01 | Non-standard puzzles (CAT/NFT) silently skipped | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_non_standard_puzzle_skip` | ❌ Wave 0 |
| BRIDGE-01 | CHIP §459 identity-element guard | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_identity_element_guard` | ❌ Wave 0 |
| BRIDGE-01 | Empty-block returns empty TweakData | unit (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests::test_empty_block` | ❌ Wave 0 |
| BRIDGE-02 | Simulator helper output byte-identical pre/post refactor (3 oracle tests) | integration (Rust) | `cargo test -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` | ✅ exists at `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`; expected to PASS unchanged |
| BRIDGE-02 | Simulator helper's own inline tests still pass | unit (Rust) | `cargo test -p chia-sdk-test --features peer-simulator,chip-0057 silent_payments::tweak_data::tests` | ✅ exists at `crates/chia-sdk-test/src/silent_payments/tweak_data.rs:100-121` |
| BRIDGE-03 | `Relation` opaque-handle compiles + descriptor matches | build verification | `cargo build -p chia-sdk-bindings --all-features && bash scripts/sp_descriptor_facade_drift.sh` | ✅ scripts/sp_descriptor_facade_drift.sh exists per Plan 05-04 |
| BRIDGE-03 | `Relation` reachable in napi/pyo3/wasm at build | build verification | `cd napi && pnpm build` AND `cd pyo3 && maturin develop` AND `cd wasm && wasm-pack build --target nodejs` | ✅ all three commands work today; need to confirm `Relation` in generated `.d.ts`/`.pyi` post-build |
| BRIDGE-04 | `Spends.prepare(deltas)` (1-arg) still works from pyo3 | integration (pyo3) | `cd pyo3 && python -m pytest tests/test_silent_payments.py::test_unlabeled_e2e -v` | ✅ exists; must PASS unchanged (or with minimal `relation=None`) |
| BRIDGE-04 | `Spends.prepare(deltas)` (1-arg) still works from napi/wasm | integration | `cd napi && pnpm test --match='*BIND-03*'` AND `cd wasm && pnpm test --match='*BIND-03*'` | ✅ Phase 6 BIND-03 tests exist; must PASS (possibly with `, undefined` insertion) |
| BRIDGE-05 | `SilentPayments.tweakDataFromBlockSpends` reachable + correct types | build verification + drift audit | `bash scripts/sp_descriptor_facade_drift.sh` | ✅ drift audit script exists |
| BRIDGE-06 | Multi-input round-trip end-to-end from pyo3 | integration (pyo3) | `cd pyo3 && python -m pytest tests/test_silent_payments.py::test_multi_input_e2e -v` | ❌ Wave 0 (new test in existing file) |
| BRIDGE-06 | Multi-input round-trip end-to-end from napi | integration (AVA) | `cd napi && pnpm test --match='*BRIDGE-06 napi*'` | ❌ Wave 0 (new spec file `silent_payments_multi_input.spec.ts`) |
| BRIDGE-06 | Multi-input round-trip end-to-end from wasm | integration (AVA) | `cd wasm && pnpm test --match='*BRIDGE-06 wasm*'` | ❌ Wave 0 (new spec file `silent_payments_multi_input.spec.ts`) |
| BRIDGE-06 | Example `silent_payment.rs` multi-input section builds + runs | example check | `cargo build --example silent_payment --all-features && cargo run --example silent_payment --all-features --release` | ✅ example exists; multi-input section is NEW (append-only) |
| BRIDGE-01..06 | Workspace lint policy across all changes | static analysis | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings && cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings && cargo fmt --all --check && cargo machete` | ✅ all clippy targets work today |
| BRIDGE-01..06 | No new `#[allow]` attributes anywhere under `silent_payments/` or binding facades | grep | `! grep -rE '#\[allow\(' crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs crates/chia-sdk-bindings/src/action_system.rs crates/chia-sdk-bindings/src/silent_payments.rs` (excluding pre-existing `scan_from_tweaks` allow in scanner.rs) | ❌ Wave 0 — grep audit must land in plan acceptance criteria |
| BRIDGE-01..06 | CLEANUP-01 grep ban: no GSD planning-artifact references in source comments | grep | `! grep -rE 'CONTEXT\.md\|RESEARCH(\.md)?\|Plan 0[1-9]-\|\bD-0[1-9]\b\|Pitfall [0-9]\|Pattern [0-9]' crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs crates/chia-sdk-bindings/src/{action_system,silent_payments}.rs napi/__test__/silent_payments_multi_input.spec.ts pyo3/tests/test_silent_payments.py wasm/__test__/silent_payments_multi_input.spec.ts examples/silent_payment.rs` | ❌ Wave 0 grep gate (CLEANUP-01 inherited invariant) |

### Sampling Rate
- **Per task commit:** `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data` (~3-5 sec) for BRIDGE-01 work; `cargo build -p chia-sdk-bindings --all-features` (~30 sec) for BRIDGE-03/04/05 work; per-binding `pnpm test --match='...'` or `pytest -v` (~5-30 sec each) for BRIDGE-06 incremental work.
- **Per wave merge:** Wave 1: scoped clippy + full driver suite under chip-0057 (`cargo test -p chia-sdk-driver --features chip-0057`). Wave 2: cross-binding build sweep (`cd napi && pnpm build; cd pyo3 && maturin develop; cd wasm && wasm-pack build --target nodejs`) + full per-binding tests.
- **Phase gate:** Full workspace test command from CLAUDE.md (`cargo test --release --workspace --all-features --exclude ...binding crates...`) + all 3 binding suites green + scoped clippy clean under `-D warnings` + `cargo fmt --check` + `cargo machete` + `bash scripts/sp_descriptor_facade_drift.sh` exits 0 + all grep gates from BRIDGE-01..06 invariants pass.

### Wave 0 Gaps
- [ ] `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` — covers BRIDGE-01 (new file)
- [ ] `napi/__test__/silent_payments_multi_input.spec.ts` — covers BRIDGE-06 napi (new file)
- [ ] `wasm/__test__/silent_payments_multi_input.spec.ts` — covers BRIDGE-06 wasm (new file)
- [ ] `pyo3/tests/test_silent_payments.py::test_multi_input_e2e` — covers BRIDGE-06 pyo3 (new test in existing file, append-only)
- [ ] `examples/silent_payment.rs` (multi-input section) — covers BRIDGE-06 example demonstration (append-only)
- [ ] Grep audit script for `#[allow]` ban — already implicit in CLAUDE.md policy; the planner should include it explicitly in the phase-gate

*(Framework install: no new install needed — pytest is at `pyo3/.venv` from Phase 6, AVA at `napi/node_modules` and `wasm/node_modules` from Phase 5-6.)*

## Sources

### Primary (HIGH confidence — direct source inspection)
- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` — full file read; confirms simulator helper structure (one-group aggregation, identity-element guard, defensive parse, output pairing).
- `crates/chia-sdk-bindings/src/action_system.rs` — full file read; confirms `Spends::prepare` at line 177 hardcoding `Relation::None` at line 185; `Id` opaque-handle at 449-486; `SendDestination` at 504-540.
- `crates/chia-sdk-driver/src/action_system/relation.rs` — full file read; confirms 2-variant enum with "load-bearing for CHIP-0057" rustdoc.
- `crates/chia-sdk-driver/src/action_system/spends.rs` (lines 350-680, 820-920) — confirms `sp_finish_branch` invoked from `prepare` at line 488-491; SilentPaymentRequiresInputBinding gate at 604; `emit_relation` cycle emission at 371-397; pinning test at 843-916.
- `crates/chia-sdk-bindings/src/simulator.rs` — full file read; confirms `tweak_data_from_block` at line 93 + binding facade pattern.
- `crates/chia-sdk-bindings/src/silent_payments.rs` — full file read; confirms `SilentPayments` namespace class pattern + 4 existing static methods.
- `bindings/action_system.json` — full file read; confirms `Option<Id>`/`Option<TransferNftById>` parameter precedents.
- `bindings/silent_payments.json` — full file read; confirms `SilentPayments` static-functions schema.
- `crates/chia-sdk-types/src/run_puzzle.rs` — full file read; confirms `run_puzzle(allocator, puzzle, solution) -> Result<NodePtr, EvalErr>` for Stage 2b condition extraction.
- `crates/chia-sdk-driver/src/silent_payments/types.rs`, `protocol.rs:130-200`, `mod.rs` — full + targeted reads; confirms TweakData/OutputMeta wire types and protocol primitives.
- `crates/chia-sdk-test/src/simulator.rs:175-209` — confirms `block_spends` / `block_outputs` public methods.
- `pyo3/tests/test_silent_payments.py` — full file read; confirms exact call site pattern for `spends.prepare(deltas)` (single-arg) that BRIDGE-04 must preserve.
- `napi/__test__/silent_payments_e2e.spec.ts` and `wasm/__test__/silent_payments.spec.ts` — full reads; confirms BIND-03 test scaffolding pattern to mirror in BRIDGE-06.
- `~/silent-payments/scanner.py:27-95,320-423` — reference Python impl of Tarjan SCC + Pass 2a/2b algorithm; confirms algorithmic choice (iterative Tarjan over directed graph).
- `~/silent-payments/sdk-gaps-prompt.md` (also at repo root) — source-of-truth gap description; confirms Python-consumer rewrite is blocked on these two gaps.
- `.planning/REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md`, `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-04-SUMMARY.md` — confirms phase ordering and Phase 6 / Phase 7 invariants.
- `CLAUDE.md` (project root) — confirms Rust 1.90.0 pin, workspace lint policy, GSD workflow enforcement.
- `.planning/config.json` — confirms `nyquist_validation: true` (Validation Architecture section required).

### Secondary (MEDIUM confidence — pattern inference from precedent)
- bindy-macro `Option<Class>` parameter handling — inferred from existing `Option<Id>` and `Option<TransferNftById>` descriptor entries that successfully generate cross-target bindings. No bindy-macro source code read for Phase 9; risk: pre-flight build in BRIDGE-04 to confirm.
- Tarjan SCC algorithm in Rust — translated from reference Python (iterative variant). Not a SDK-internal precedent; rely on the algorithm correctness and the test oracles (BRIDGE-01 inline tests) to validate.

### Tertiary (LOW confidence — none required for Phase 9)
- (No web-fetched sources used. All findings sourced directly from the repo.)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every recommended library is already pinned in `[workspace.dependencies]` and used by existing SP code.
- Architecture (file structure, opaque-handle pattern, signature extension): HIGH — every change mirrors an existing precedent verified in current source.
- Pass 2a/2b grouping algorithm: HIGH (algorithm choice) / MEDIUM (Rust implementation) — algorithm is well-specified by CHIP and reference Python; Rust implementation is novel-to-this-repo but mechanically simple (~80 lines).
- Pitfalls: HIGH — Pitfalls 1, 4, 6, 7 came from current-source inspection or Phase 6/7 summaries (documented deviations); Pitfalls 2, 3, 5, 8, 9 are domain knowledge with high-confidence mitigation strategies.
- Validation architecture: HIGH — every test command verified against existing infrastructure; only the new test files don't exist yet (correctly flagged in Wave 0 Gaps).

**Research date:** 2026-05-29
**Valid until:** 2026-06-12 (14 days — short half-life because main branch is active; recheck `chia-sdk-bindings/src/action_system.rs:185` and `chia-sdk-driver/src/action_system/spends.rs:604` before planning if any commit lands between now and plan generation).
