# Phase 9: Real-block TweakData bridge + Python Relation binding - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in 09-CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 09-real-block-tweakdata-bridge-python-relation-binding
**Areas discussed:** Gap 1 — Block input shape, Gap 1 — Logic extraction strategy, Gap 2 — Relation binding + Spends.prepare shape, Cross-binding parity scope

---

## Gray Areas Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Gap 1 — Block input shape | Exact type signature for the new RPC-side helper. Vec<CoinSpend> + additions list vs single bundle struct vs builder pattern. Includes naming and module/file location. | ✓ |
| Gap 1 — Logic extraction strategy | Simulator helper already has parse → group → aggregate → hash logic. Extract pure shared function (DRY) vs build standalone (avoid touching working code). | ✓ |
| Gap 2 — Relation binding + Spends.prepare shape | Relation isn't bound to Python yet. Follow Id/SendDestination opaque-handle precedent vs native pyo3 enum. Also: prepare signature shape. | ✓ |
| Cross-binding parity scope | pyo3 only (smallest blast radius) vs full parity across pyo3+napi+wasm (consistent, more work). | ✓ |

**User's choice:** All 4 selected.

---

## Gap 1 — Block input shape

### Q: What input signature for the new RPC-side helper?

| Option | Description | Selected |
|--------|-------------|----------|
| Two slices | `fn tweak_data_from_block_spends(coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData>` — mirrors simulator helper's logical inputs; zero new types. | ✓ |
| New BlockSummary struct | Bundle spends + additions + (optionally) height in a new struct. Cleaner call site but introduces new wire-protocol-adjacent type that needs binding. | |
| Builder pattern | `TweakDataBuilder::new().with_coin_spends(...).with_additions(...).build()`. Heavier surface for a pure function. | |

**User's choice:** Two slices (Recommended).

### Q: Helper function name?

| Option | Description | Selected |
|--------|-------------|----------|
| tweak_data_from_block_spends | Prompt's suggestion. Mirrors `tweak_data_from_simulator_block`. | ✓ |
| tweak_data_from_coin_spends | More technically accurate; loses block-context framing. | |
| compute_tweak_data | Terse; parallels `compute_input_hash`; loses block-context signal. | |

**User's choice:** tweak_data_from_block_spends (Recommended).

### Q: Where does the helper live?

| Option | Description | Selected |
|--------|-------------|----------|
| New sibling file | `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` — dedicated module mirroring `scanner.rs` and `protocol.rs`. | ✓ |
| Append to protocol.rs | Continues Phase 8's fold philosophy; ~150-200 lines added; under 700-line ceiling. | |
| Inside scanner.rs | Inverts producer/consumer split; scanner.rs already 735 lines. | |

**User's choice:** New sibling file (Recommended).

### Q: Generator decompression — in scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Out of scope | Caller hands decompressed CoinSpends + additions. Keeps chia-sdk-driver free of chia-consensus dep. | ✓ |
| Accept generator bytes + refs, decompress inside | Single-input call site but pulls generator-runner dep into chia-sdk-driver. | |
| Both — ship two helpers | Maximal flexibility at the cost of double API surface and the generator dep. | |

**User's choice:** Out of scope (Recommended).

---

## Gap 1 — Logic extraction strategy

### Q: How to share logic between the simulator helper and the new RPC-side helper?

| Option | Description | Selected |
|--------|-------------|----------|
| Extract pure core, refactor simulator helper | Pure core in chia-sdk-driver; simulator helper becomes thin adapter. DRY; no drift risk. | ✓ |
| Standalone new helper, leave simulator alone | Duplicate logic. Lower risk; drift risk for future protocol changes. | |
| Move ALL logic to chia-sdk-driver; thin simulator adapter | Largest diff; touches both crates more heavily. | |

**User's choice:** Extract pure core, refactor simulator helper (Recommended).

### Q: Where does Pass 2a + Pass 2b grouping live?

| Option | Description | Selected |
|--------|-------------|----------|
| In the new helper itself | Grouping internal; one tweak_point per group emitted; callers don't need separate API. | ✓ |
| Separate `group_spends_into_tx_groups` helper | Independent grouping API + composability; adds 2-call ceremony for the common case. | |

**User's choice:** In the new helper itself (Recommended).

### Q: How should the helper handle non-standard puzzles?

| Option | Description | Selected |
|--------|-------------|----------|
| Silently skip | Matches simulator helper exactly. `StandardLayer::parse_puzzle` returns `Ok(None)` for non-standard; those spends contribute nothing. | ✓ |
| Error on non-standard | Forces callers to filter upfront; breaks natural 'hand me a block' usage. | |
| Configurable via builder/flag | More flexibility but inconsistent with simulator helper. | |

**User's choice:** Silently skip (Recommended).

### Q: Pass 2b SCC scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Strongly connected component over opcode-64 edges | Per CHIP-0057 + Phase 4.1. Matches simulator helper + Relation::AssertConcurrent Rust gate. | ✓ |
| Same as Pass 2a only | Wrong — silently breaks receive-side detection for multi-input sends. | |

**User's choice:** Strongly connected component over opcode-64 edges (Recommended).

---

## Gap 2 — Relation binding + Spends.prepare shape

### Q: Python `Relation` binding shape?

| Option | Description | Selected |
|--------|-------------|----------|
| Opaque-handle + factories | `Relation::none()`, `Relation::assert_concurrent()` + `is_*` introspectors. Matches Id and SendDestination precedent. | ✓ |
| Native pyo3 enum class | `#[pyclass] enum Relation`. Cleaner Python ergonomics; inconsistent with rest of binding surface. | |
| Module-level constants | Lightest binding; no introspectors; inconsistent. | |

**User's choice:** Opaque-handle + factories (Recommended).

### Q: Spends.prepare(deltas, relation) parameter shape?

| Option | Description | Selected |
|--------|-------------|----------|
| Optional positional, default Relation::None | `prepare(deltas, relation: Option<Relation>)`. Existing E2E unchanged; multi-input new path. | ✓ |
| Required positional, no default | Forces explicit choice; breaks existing pyo3 E2E (acceptance forbids this). | |
| Two methods (overload) | Keep prepare(deltas); add prepare_with_relation(deltas, relation). Cluttered. | |

**User's choice:** Optional positional, default Relation::None (Recommended).

### Q: Where does the Relation type live in bindings/*.json descriptors?

| Option | Description | Selected |
|--------|-------------|----------|
| bindings/action_system.json | Next to Id, SendDestination, Spends, Deltas. ~15 lines. | ✓ |
| Top-level bindings.json type-group map | Over-broad; only action_system needs Relation today. | |

**User's choice:** bindings/action_system.json (Recommended).

---

## Cross-binding parity scope

### Q: Which binding targets does Phase 9 extend?

| Option | Description | Selected |
|--------|-------------|----------|
| All three (pyo3 + napi + wasm) | Descriptor-driven means automatic propagation. Matches Phase 5/6 zero-drift convention. | ✓ |
| pyo3 only | Matches stated demand. Creates drift across bindings; needs v1.1 follow-up. | |
| pyo3 + napi (skip wasm) | Partial drift. | |

**User's choice:** All three (Recommended).

### Q: Multi-input round-trip test — which targets?

| Option | Description | Selected |
|--------|-------------|----------|
| All three bindings | One test per target; mirrors Phase 6 BIND-03. | ✓ |
| pyo3 only (acceptance minimum) | Strict reading of acceptance bullet. Untested code paths in napi/wasm. | |

**User's choice:** All three bindings (Recommended).

### Q: Should examples/silent_payment.rs be updated to demo multi-input?

| Option | Description | Selected |
|--------|-------------|----------|
| Add a multi-input section | Extend existing example with ~30-50 line multi-input demo. | ✓ |
| Leave example as-is | Acceptance is conditional; argument for skipping. | |
| Add a SECOND example file | Two examples to maintain. | |

**User's choice:** Add a multi-input section (Recommended).

### Q: Rust-side test for the new helper — where does it live?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline in block_tweak_data.rs | Matches post-Phase-8 convention; primitives/* convention. | ✓ |
| New integration test under tests/ | Tests public API from outside crate boundary; more setup. | |
| Both | Most thorough; ~2x test code; arguably overkill. | |

**User's choice:** Inline in block_tweak_data.rs (Recommended).

---

## Final Confirmation

### Q: Anything else to discuss before I write CONTEXT.md?

| Option | Description | Selected |
|--------|-------------|----------|
| I'm ready for context | Write CONTEXT.md with the 4 areas above locked. | ✓ |
| Explore more gray areas | Pick more (error handling, label support, Cargo features, performance). | |
| Revisit a previous area | Reopen one of the four discussed areas. | |

**User's choice:** I'm ready for context (Recommended).

---

## Claude's Discretion

Captured in CONTEXT.md `<decisions>` § "Claude's Discretion":
- Inline vs sub-mod test organization in `block_tweak_data.rs`
- Whether to add proptest for Pass 2b grouping correctness
- `Relation` binding doc comment exact wording
- Where the binding-side `tweak_data_from_block_spends` lives (likely `crates/chia-sdk-bindings/src/silent_payments.rs`)
- Exact test names

## Deferred Ideas

Captured in CONTEXT.md `<deferred>` section:
- Generator decompression integrated into the SDK
- Label support on per-spend output of the new helper
- Relation as fully-introspectable enum (variants(), pattern matching)
- Async/streaming variant for very large blocks
- Separate `chia-sdk-rpc-bridge` crate
- Property-based testing (left to Claude's Discretion)
- CHIP-0058 transport client (carried from PROJECT.md "Out of Scope")
