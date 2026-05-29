# Phase 9: Real-block TweakData bridge + Python Relation binding - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Two surgical additions to the existing v1 CHIP-0057 surface that unblock downstream Python consumers from driving multi-input silent-payment sends and scanning real (non-simulator) blocks. Pure additive — no API breakage to the v1 surface or to the existing single-input pyo3 E2E. Same shape as Phase 8: small, mechanically verifiable, no behavior change to existing code paths.

**Gap 1 (real-block TweakData):** Extract the pure logic of `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` into a new `tweak_data_from_block_spends(coin_spends, additions)` helper in `chia-sdk-driver`. Refactor the simulator helper to call the new core, then expose the new helper via bindings.

**Gap 2 (Python Relation):** Bind `chia_sdk_driver::Relation` to all three binding targets (pyo3 + napi + wasm) via the opaque-handle + factories pattern, and extend `Spends::prepare` to accept an optional `Relation` parameter so multi-input SP sends (≥2 non-ephemeral XCH inputs) can pass `Relation::AssertConcurrent` from the binding layer.

**In scope:** ~5 task groups across both gaps (new helper, simulator refactor, Relation binding + descriptor, Spends.prepare signature change, multi-input tests in all three bindings, example update).

**Out of scope:**
- Generator-output decompression (separate concern; chia-consensus / chia_rs already provide it).
- New SP protocol features (CHIP-0057 spec is closed for v1).
- Labels-per-spend on the new helper (the simulator helper doesn't expose them either; not asked for).
- CAT2 / NFT silent payments (PROJECT.md out-of-scope).
- CHIP-0058 transport client (deferred to a follow-up that lands when CHIP-0058 is defined).

</domain>

<decisions>
## Implementation Decisions

### Gap 1 — Block input shape

- **D-01:** **`fn tweak_data_from_block_spends(coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData, DriverError>`** in `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` (new sibling file). The two-slice shape mirrors the simulator helper's logical inputs (`sim.block_spends(height)` + block outputs); Python callers already build `Vec<CoinSpend>` + `Vec<Coin>` directly from coinset RPC responses post-decompression. Zero new types. New sibling file (NOT `protocol.rs` append, NOT `scanner.rs` append) because (a) it mirrors `scanner.rs`'s receive-consumer placement on the producer side, (b) keeps `protocol.rs` focused on the primitives + send composers it absorbed in Phase 8, (c) avoids inverting scanner.rs's producer/consumer split.

- **D-02:** **Generator decompression is out of scope.** Callers hand the helper decompressed `Vec<CoinSpend>` + `Vec<Coin>`. Reason: keeps `chia-sdk-driver` free of the `chia-consensus` generator-runner dep; keeps the helper a pure function; matches how downstream Python consumers already work (they have chia_rs wheels with generator decompression baked in).

### Gap 1 — Logic extraction strategy

- **D-03:** **Extract pure core into the new helper; refactor the simulator helper to call it.** After this phase, `chia-sdk-test::silent_payments::tweak_data_from_simulator_block(sim, height)` becomes a thin adapter that fetches `sim.block_spends(height)` + block outputs and delegates to `chia_sdk_driver::silent_payments::tweak_data_from_block_spends(spends, outputs)`. Single canonical implementation of the parse → group → aggregate → input_hash → tweak_point logic. Drift risk eliminated.

- **D-04:** **Pass 2a (same-puzzle-hash) and Pass 2b (AssertConcurrent SCC over opcode-64 edges) grouping lives inside the new helper.** Callers pass the full block's spends; the helper emits one `tweak_point` per "transaction" group. Matches the simulator helper's current grouping shape (currently one block = one group, which the simulator helper materializes via a single `synthetic_pks` accumulator; the new helper must generalize this to multiple groups when a block contains multiple SP-using transactions). NO separate `group_spends_into_tx_groups` public API — keep the grouping internal to avoid surface-area growth.

- **D-05:** **Non-standard puzzles (CAT, NFT, etc.) silently skipped via `StandardLayer::parse_puzzle` defensive parse.** Identical to the simulator helper's current behavior; the doc comment at `tweak_data.rs:41-43` already establishes this convention. Mixed blocks (standard + CAT spends in the same block) just have the CAT spends contribute nothing to A_sum / coin_ids.

- **D-06:** **CHIP §459 identity-element guard preserved.** Computed `tweak_point` that equals the BLS12-381 identity element is suppressed (no tweak_point emitted for that group). Matches the simulator helper. Implementation comes through naturally from extracting the existing logic.

### Gap 2 — Python `Relation` binding

- **D-07:** **Opaque-handle + factories** for `Relation` — matches `Id` precedent at `crates/chia-sdk-bindings/src/action_system.rs:452` and `SendDestination` at `:505`:

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

  Reason: descriptor-driven; consistent with the rest of the action_system binding surface; ships to all three targets without per-target hand-written shims; mirrors the Id precedent reviewers already know.

- **D-08:** **Descriptor entry in `bindings/action_system.json`.** Relation parameterizes `Spends.prepare` and conceptually belongs in the action_system module. Adds ~15 lines next to `Id` and `SendDestination`. NOT a top-level type-group mapping (over-broad — only action_system needs Relation today).

### Gap 2 — `Spends.prepare` signature

- **D-09:** **`fn prepare(&self, deltas: Deltas, relation: Option<Relation>) -> Result<FinishedSpends>`** — when `relation` is `None`, the binding passes `sdk::Relation::None` to the Rust `prepare` (preserving the current hardcoded behavior at `crates/chia-sdk-bindings/src/action_system.rs:185`). When `relation` is `Some(r)`, pass `r.0` through. Python ergonomics:

  ```python
  finished = spends.prepare(deltas)                              # current single-input callers — unchanged
  finished = spends.prepare(deltas, Relation.assert_concurrent())  # new multi-input SP path
  ```

  Existing `pyo3/tests/test_silent_payments.py::test_unlabeled_e2e` keeps working with no signature changes — meets the acceptance bullet "Existing Python E2E stays green with no signature changes beyond the optional `Relation` parameter."

### Cross-binding parity scope

- **D-10:** **All three targets (pyo3 + napi + wasm) ship the extended surface in this phase.** The bindy-macro descriptor-driven flow means adding `Relation` to `bindings/action_system.json` and the `Spends.prepare(relation)` signature change auto-propagates. Marginal extra cost is per-target tests; binding-glue code is zero per target. Matches Phase 5/6 zero-drift convention; avoids a v1.1 follow-up to close drift.

- **D-11:** **Multi-input round-trip test in each binding target.** One test per target (`napi/__test__/silent_payments_multi_input.spec.ts`, `pyo3/tests/test_silent_payments.py::test_multi_input_e2e`, `wasm/__test__/silent_payments_multi_input.spec.ts`) demonstrating: farm 2+ XCH coins → single `Action::send` to one SP address → `Spends.prepare(deltas, Relation.assert_concurrent())` → farm bundle → `tweak_data_from_block_spends(spends, additions)` → scan → exactly one DetectedSpCoin. Mirrors Phase 6 BIND-03 single-input round-trip pattern.

### Example update

- **D-12:** **`examples/silent_payment.rs` gains a multi-input section.** Add ~30-50 lines after the existing unlabeled + labeled flows demonstrating: farm 2+ XCH coins → single `Action::send` to one SP address → `Spends::prepare(deltas, Relation::AssertConcurrent)` → farm → scan + spend the detected coin. Uses the existing in-process simulator path (the example doesn't need to demonstrate the non-simulator path explicitly — that's the binding tests' job). Acceptance bullet "updated if the new API materially changes the canonical usage pattern" — multi-input + `Relation` is a new canonical pattern that wallets will hit; the example should show it.

### Rust-side tests

- **D-13:** **Inline `#[cfg(test)] mod tests {}` at the bottom of `block_tweak_data.rs`.** Matches the post-Phase-8 convention in `protocol.rs`, the convention in `scanner.rs`, and every primitive in `crates/chia-sdk-driver/src/primitives/`. No new integration target. Tests cover: single SP send (matches simulator helper output byte-for-byte against a hand-built block), multi-input SP send (Pass 2b SCC over opcode-64 edges), Pass 2b pollution attack (third-party AssertConcurrent assertion pointing at a victim's coin must NOT merge groups), non-standard-puzzle skip, identity-element CHIP §459 guard, empty-block edge case.

### Bindings expose `tweak_data_from_block_spends`

- **D-14:** **Expose `tweak_data_from_block_spends` as a static method on the `SilentPayments` namespace class** in `chia-sdk-bindings::silent_payments` + `bindings/silent_payments.json`. Matches the BIND-02 pattern (Phase 5 D-02) — `SilentPayments.scanFromTweaks`, `SilentPayments.deriveOneTimePuzzleHash`, etc. are the existing static-method precedents. Method signature: `SilentPayments.tweakDataFromBlockSpends(coinSpends: CoinSpend[], additions: Coin[]) -> TweakData` (camelCase per binding convention).

### Claude's Discretion

- **Inline vs sub-mod test organization in `block_tweak_data.rs`.** Phase 8 D-02 left this open per file; planner picks a single flat `mod tests {}` (recommended) vs. nested sub-mods (e.g., `mod tests { mod pass_2a; mod pass_2b; mod edge_cases; }`) based on what reads cleanest after the tests are written. The flat option is preferred unless ≥8 tests land with clear logical groupings.
- **Whether to add a property-test (proptest) for Pass 2b grouping correctness.** Optional; not asked for. Planner decides whether the targeted unit tests above are enough or whether a property test buys additional confidence cheaply.
- **`Relation` binding doc comments.** Planner copies/adapts the rustdoc from `crates/chia-sdk-driver/src/action_system/relation.rs` (which already documents the "load-bearing for CHIP-0057" intent at the variant level). The pyo3/napi/wasm-side doc surfacing follows whatever convention the existing `Id` and `SendDestination` bindings use.
- **Where the binding-side `tweak_data_from_block_spends` lives.** Likely `crates/chia-sdk-bindings/src/silent_payments.rs` (where the existing SP facade lives, per Phase 5 D-01). Planner confirms when the actual file size lands.
- **Exact test names.** Planner picks readable names; suggested seed: `test_multi_input_round_trip`, `test_pass_2b_pollution_resistance`, `test_non_standard_puzzle_skip`, `test_identity_element_guard`, `test_empty_block`.

### Folded Todos

None — no pending todos matched Phase 9 scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase boundary + scope
- `.planning/ROADMAP.md` — Phase 9 entry (added 2026-05-29; goal + depends-on + plans skeleton)
- `sdk-gaps-prompt.md` (repo root) — the source-of-truth gap description from the downstream Python consumer; defines Gap 1 + Gap 2 + acceptance bullets

### Code to extract from / refactor
- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` — the simulator helper whose pure core gets extracted (D-03); current implementation at `:42+`. Doc comment at `:41-43` establishes the non-standard-puzzle skip convention (D-05).

### Code to extend
- `crates/chia-sdk-bindings/src/action_system.rs:177-191` — current `Spends::prepare(deltas)` binding (hardcodes `Relation::None` at `:185`). Extend per D-09.
- `crates/chia-sdk-bindings/src/action_system.rs:452-490` — `Id` opaque-handle binding pattern. Mirror for the new `Relation` binding (D-07).
- `crates/chia-sdk-bindings/src/action_system.rs:488-545` — `SendDestination` opaque-handle binding pattern (with `is_*`/`as_*` introspectors). Second precedent for D-07.
- `bindings/action_system.json` — descriptor for action_system bindings. New `Relation` entry per D-08.
- `bindings/silent_payments.json` — descriptor for silent_payments bindings. New `SilentPayments.tweakDataFromBlockSpends` static method per D-14.
- `crates/chia-sdk-bindings/src/silent_payments.rs` — SP facade module (Phase 5 D-01). Hosts the `tweak_data_from_block_spends` binding glue.

### Rust enum being bound
- `crates/chia-sdk-driver/src/action_system/relation.rs:10-35` — `Relation` enum definition (2 variants: `None`, `AssertConcurrent`). Rustdoc at variant level explains the "load-bearing for CHIP-0057" intent — preserve in binding doc surfacing.

### Rust-side gate being satisfied
- `crates/chia-sdk-driver/src/action_system/spends.rs:600-608` — the `non_ephemeral_xch_count >= 2 && !AssertConcurrent` gate that returns `Err(DriverError::SilentPaymentRequiresInputBinding)`. Phase 9 makes the BINDING layer able to satisfy this gate; the gate itself doesn't change.
- `crates/chia-sdk-driver/src/action_system/spends.rs:374` — the actual `Relation::AssertConcurrent` emission path (the cyclic ASSERT_CONCURRENT_SPEND condition builder). Pinned by `:834+` test.

### Tests to extend
- `pyo3/tests/test_silent_payments.py` — existing single-input E2E. Phase 9 ADDS a new `test_multi_input_e2e` (D-11); existing test stays untouched.
- `napi/__test__/silent_payments_e2e.spec.ts` — Phase 6 BIND-03 napi E2E. Phase 9 adds a new sibling `silent_payments_multi_input.spec.ts`.
- `wasm/__test__/silent_payments.spec.ts` — Phase 6 BIND-03 wasm E2E. Phase 9 adds a new sibling `silent_payments_multi_input.spec.ts`.

### Example to extend
- `examples/silent_payment.rs` — 119/120-line existing example. Phase 9 adds a multi-input section per D-12.

### Prior phase decisions that carry forward
- `.planning/phases/04.1-sage-style-binding-refactor/04.1-CONTEXT.md` — `Relation::AssertConcurrent` is THE multi-input SP atomicity mechanism (replaces opcode 60/61 announcements). The Rust gate `spends.rs:604` is the runtime enforcement. Phase 9 makes the binding layer able to feed the gate.
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-CONTEXT.md` — `Action::send(id, destination, amount, memos)` is the unified send action. Phase 9 doesn't change this; multi-input SP sends use the same `Action::send` constructor (just with multiple non-ephemeral XCH coins in the spend bundle).
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-CONTEXT.md` D-02 — `SilentPayments` namespace class hosts static methods (e.g., `scanFromTweaks`, `deriveOneTimePuzzleHash`). Phase 9 adds `tweakDataFromBlockSpends` per the same pattern (D-14). D-04 established the `SendDestination` opaque-handle precedent that informs D-07.
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-CONTEXT.md` — BIND-03 cross-language E2E pattern (one test per binding target). Phase 9's D-11 mirrors this for the multi-input case.
- `.planning/phases/07-code-review-cleanup/07-CONTEXT.md` CLEANUP-03 — `Spends::finish_silent_payments` was removed; SP finish branch fires inside `Spends::prepare`. Means Phase 9's `prepare(deltas, relation)` extension is the right plug-in point (no other public surface to extend).
- `.planning/phases/08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics/08-CONTEXT.md` D-02 — protocol primitives now live in `silent_payments/protocol.rs`. Phase 9's new sibling `block_tweak_data.rs` follows the post-fold sibling-file convention.

### Project-level
- `CLAUDE.md` — Rust 1.90.0, edition 2024, workspace clippy `deny clippy::all + warn pedantic + warn cargo`, `unsafe_code = deny`, no new workspace deps, per-crate `cargo build` with and without `--all-features`.
- `.planning/PROJECT.md` § "Out of Scope" — CHIP-0058 transport client deferred; CAT2/NFT SP deferred. Phase 9 stays inside v1's XCH-SP boundary.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`chia-sdk-test::tweak_data_from_simulator_block` (the helper to extract from):** Already does Pass 2a same-puzzle-hash group, Pass 2b SCC grouping (implicit via per-tx walking), aggregation, `compute_input_hash`, `tweak_point` computation, identity-element guard, output pairing. Phase 9's new `tweak_data_from_block_spends` absorbs this logic; the simulator helper becomes a thin adapter.
- **`chia_sdk_driver::StandardLayer::parse_puzzle`:** Defensive standard-puzzle parse — returns `Ok(None)` for any puzzle whose mod hash isn't the standard p2 puzzle's. Used as the non-standard-puzzle skip mechanism (D-05).
- **`chia_sdk_driver::silent_payments::{compute_input_hash, ScalarField}`:** Post-Phase-8 these live in `silent_payments/protocol.rs`. The new helper composes them; no new primitives needed.
- **`chia_sdk_driver::Relation::{None, AssertConcurrent}`:** The Rust enum being bound; 2 variants only.
- **`Id` and `SendDestination` opaque-handle bindings:** Precedents at `chia-sdk-bindings/src/action_system.rs:452+` and `:505+`. Mirror exactly for the new `Relation` binding (D-07).
- **BIND-03 cross-language E2E pattern (Phase 6):** napi `silent_payments_e2e.spec.ts`, pyo3 `test_silent_payments.py`, wasm `silent_payments.spec.ts`. Phase 9 adds parallel multi-input siblings using the same scaffolding.

### Established Patterns

- **Descriptor + facade + zero drift:** `bindings/*.json` ↔ `crates/chia-sdk-bindings/src/*.rs`. Phase 9 follows: descriptor entries in `action_system.json` (Relation) + `silent_payments.json` (tweakDataFromBlockSpends); facade impls in the matching Rust files.
- **Opaque-handle for non-trivial Rust types:** `Id`, `SendDestination`, now `Relation`. Factory methods + `is_*`/`as_*` introspectors + `equals`. No pyo3-specific `#[pyclass]` enum.
- **`chip-0057` feature gate:** The new `block_tweak_data.rs` lives behind `chip-0057` (the entire `silent_payments/` module is gated). The new `Relation` binding is NOT chip-0057-gated (Relation is part of the general action system, not SP-specific).
- **Inline `#[cfg(test)] mod tests {}`:** Workspace-wide convention; D-13 follows.
- **No backwards-compat shims (CLAUDE.md):** Phase 9 changes `Spends.prepare(deltas)` to `Spends.prepare(deltas, Option<Relation>)` directly. Old callers keep working because `None` defaults to `Relation::None`. No `prepare_legacy` or `#[deprecated]` shim.

### Integration Points

- **`Spends::prepare` (binding):** Single insertion point for the new `Relation` parameter (D-09); the underlying Rust `Spends::prepare(ctx, deltas, relation)` already takes a `Relation` argument since Phase 4.1.
- **`SilentPayments` namespace class (binding):** New static method `tweakDataFromBlockSpends` (D-14) sits next to `scanFromTweaks`, `deriveOneTimePuzzleHash`, etc.
- **`bindings/action_system.json` + `bindings/silent_payments.json` descriptors:** Two file edits.
- **3 binding test directories:** `napi/__test__/`, `pyo3/tests/`, `wasm/__test__/` — one new test file each.
- **`examples/silent_payment.rs`:** Append-only extension per D-12.

</code_context>

<specifics>
## Specific Ideas

- **New helper signature (exact):**
  ```rust
  // crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
  #[must_use]
  pub fn tweak_data_from_block_spends(
      coin_spends: &[CoinSpend],
      additions: &[Coin],
  ) -> Result<TweakData, DriverError>;
  ```
  Returns `Err` only on protocol-level corruption (e.g., a standard-puzzle reveal that fails to extract a synthetic_key); non-standard puzzles are skipped silently, not errored.

- **New Spends.prepare binding signature (exact):**
  ```rust
  // crates/chia-sdk-bindings/src/action_system.rs (replacing :177)
  pub fn prepare(&self, deltas: Deltas, relation: Option<Relation>) -> Result<FinishedSpends> {
      let relation = relation.map(|r| r.0).unwrap_or(sdk::Relation::None);
      // ... (rest of the body uses `relation` instead of the hardcoded Relation::None at :185)
  }
  ```

- **Test plan for `tweak_data_from_block_spends` (Pass 2b correctness — Risk):**
  The Pass 2b "pollution attack" test is the most important: construct a block containing (a) a legit multi-input SP send forming a closed cycle of `AssertConcurrentSpend` conditions over its 2 coins, AND (b) a third-party coin that emits a single `AssertConcurrentSpend` pointing at coin (a)'s first coin. The legit send's SCC must be {a1, a2}; the polluter must sit in its own trivial SCC {polluter}. If grouping inadvertently merges the polluter into the legit SCC, A_sum gets polluted and the receiver fails to detect. This is the test that proves correctness.

- **Test parity oracle:** After D-03 refactor, the simulator helper's output must be **byte-identical** to pre-refactor for any input block. The existing Phase 6 simulator tests (`test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change` in `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`) provide the regression oracle — they must continue to pass with no changes.

- **Phase requirement IDs (suggested for REQUIREMENTS.md addition during planning):**
  - **BRIDGE-01** — `tweak_data_from_block_spends` helper in `chia-sdk-driver` with the exact signature in D-01 + inline tests covering Pass 2b SCC pollution, non-standard-puzzle skip, identity-element guard, empty-block.
  - **BRIDGE-02** — `chia-sdk-test::tweak_data_from_simulator_block` refactored to delegate to BRIDGE-01's helper. All Phase 6 simulator-test outputs byte-identical pre/post.
  - **BRIDGE-03** — `Relation` opaque-handle binding in `chia-sdk-bindings::action_system` + descriptor entry in `bindings/action_system.json`. Surfaces on all 3 binding targets.
  - **BRIDGE-04** — `Spends.prepare(deltas, relation: Option<Relation>)` extended signature. Default behavior unchanged; existing pyo3 E2E passes without changes.
  - **BRIDGE-05** — `tweak_data_from_block_spends` bound on the `SilentPayments` namespace class; descriptor entry in `bindings/silent_payments.json`.
  - **BRIDGE-06** — Multi-input round-trip test in each of pyo3, napi, wasm; new `silent_payment` multi-input section in `examples/silent_payment.rs`.

- **Phase ordering / wave hint for the planner:**
  - **Wave 1 (parallel-safe):** BRIDGE-01 (new helper + inline tests) + BRIDGE-03 (Relation binding) + BRIDGE-04 (Spends.prepare signature change). These touch disjoint files.
  - **Wave 2:** BRIDGE-02 (simulator refactor, depends on BRIDGE-01) + BRIDGE-05 (binding for the new helper, depends on BRIDGE-01) + BRIDGE-06 (cross-binding tests + example, depends on BRIDGE-01, BRIDGE-03, BRIDGE-04, BRIDGE-05).
  - Total guess: 2 plans across 2 waves (one plan per wave), OR 6 plans (one per BRIDGE-*). Planner picks.

</specifics>

<deferred>
## Deferred Ideas

- **Generator decompression integrated into the SDK.** Discussed and explicitly out of scope per D-02. If a future caller asks for a generator-bytes input shape, that's a separate phase.
- **Label support on the per-spend output of `tweak_data_from_block_spends`.** Not requested; the simulator helper doesn't expose labels either (labels are a scan-side concern attached to detection outputs, not tweak-source data). If a future use case emerges, that's a separate concern.
- **`Relation` exposed as a fully-introspectable enum (e.g., `Relation.variants()`, pattern matching).** The opaque-handle pattern (D-07) covers all current use cases via factories + `is_*`/`as_*` introspectors. If a future caller needs richer enum introspection, that's a v2 concern.
- **Async / streaming variant for very large blocks.** Not asked for. The current helper walks a `&[CoinSpend]` synchronously; blocks are bounded by Chia consensus rules and fit in memory.
- **A separate `chia-sdk-rpc-bridge` crate.** Not needed for Phase 9's scope. The new helper is pure logic on existing types (`CoinSpend`, `Coin`, `TweakData`) — no RPC client involvement.
- **Property-based testing (proptest) for Pass 2b grouping correctness.** Left to Claude's Discretion. Targeted unit tests (D-13 + the Pass 2b pollution attack specifically called out in `<specifics>`) are likely sufficient.
- **CHIP-0058 transport client.** Carried forward from PROJECT.md "Out of Scope". The new helper is the bridge that lets wallets implement transport clients now and swap to CHIP-0058 later without API churn.

### Reviewed Todos (not folded)

None — no pending todos matched Phase 9 scope.

</deferred>

---

*Phase: 09-real-block-tweakdata-bridge-python-relation-binding*
*Context gathered: 2026-05-29*
