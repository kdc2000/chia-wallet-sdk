# Phase 6: Simulator round-trip + bindings E2E + example - Research

**Researched:** 2026-05-18
**Domain:** chia-sdk-test simulator helper + cross-language E2E + runnable example
**Confidence:** HIGH

<user_constraints>
## User Constraints (from 06-CONTEXT.md)

### Locked Decisions

**D-01 — TweakData access via bindings-exposed simulator helper.** All 3 cross-language test suites (napi/pyo3/wasm) run the identical send→farm→extract→scan→detect→spend flow as Rust. Cross-language scanners consume a `TweakData` that was built on the Rust side and crossed the FFI boundary. Gives BIND-03 full FFI fidelity.

**D-02 — Helper exposed as `Simulator.tweakDataFromBlock(height)`.** TS callsite: `const tweakData = simulator.tweakDataFromBlock(height);`. Rust-side: free function `chia-sdk-test::silent_payments::tweak_data_from_simulator_block(&Simulator, height) -> TweakData`; the binding facade wraps it with a method on the existing `Simulator` binding. Adds one entry to `bindings/simulator.json`.

**D-03 — `chia-sdk-bindings/Cargo.toml` adds `"chip-0057"` to its `chia-sdk-test` dep's `features = [...]` array.** Mirrors the Phase 5 D-01 wiring; chip-0057 stays unconditional on chia-sdk-driver/utils/types. The binding facade method is UNCONDITIONAL — no `#[cfg(feature = "chip-0057")]` gate inside `chia-sdk-bindings`.

**D-04 — m=0 sub-test design (RESEARCHER MUST VERIFY before planner writes the test).** Original assumption: sender sends to their OWN SP address; the SDK's send-side code internally emits the output at `m=0` for self-change; the recipient's `scan_from_tweaks` detects it with `label: Some(0)`. **See Open Questions §1 — assumption is FALSE; m=0 test redesign mandatory.**

**D-05 — Three separate Rust `#[test]` fns:** `test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change`. Shared simulator + key-setup helper. Maps 1:1 to SC2 (unlabeled) + SC3 (labeled + m=0).

**D-06 — Tests live in `crates/chia-sdk-driver/src/silent_payments/`.** Exact file name is the planner's call. Mirrors `chia-sdk-driver/src/primitives/cat/` co-location pattern. `chia-sdk-test::Simulator` is used as a dev-dep (already wired).

**D-07 — Full parity cross-language E2E** — napi (AVA), pyo3 (pytest), wasm (AVA) all run the same UNLABELED send→farm→extract→scan→detect→spend flow. Labeled coverage stays Rust-only. Closes BIND-03 strongly without doubling test surface.

**D-08 — pyo3 gets a single `pyo3/tests/test_silent_payments.py`** (first non-trivial pytest). No `conftest.py` yet — fixtures defer until a second test consumer exists.

**D-09 — `examples/silent_payment.rs` demonstrates unlabeled + one labeled payment in a single tx.** Flow: mnemonic → `SilentPaymentKeys` → unlabeled address + `labeled_address(m=1)` → `Action::send` twice → farm → `tweak_data_from_simulator_block` → scan (both detected, one `label: None`, one `label: Some(1)`) → spend both detected coins. Target ~80–120 lines.

### Claude's Discretion

- Rust test module file name and internal helper signatures (`tests.rs` vs `e2e.rs` vs `simulator_tests.rs`).
- `bindings/simulator.json` entry shape for `tweakDataFromBlock`.
- TypeScript fixture mnemonic (recommendation: reuse BIP-39 TV1 abandon×11+about for consistency with Phase 5).
- AVA/pytest file naming (`silent_payments_e2e.spec.ts` vs extending existing).
- Example: exact recipient amount, fee handling, multi-block farm timing.

### Deferred Ideas (OUT OF SCOPE)

- Multi-input SP send in the example.
- Labeled coverage across napi+pyo3+wasm (defer to v1.1).
- pytest `conftest.py` with shared fixtures.
- Comprehensive tutorial example with privacy-warning callouts.
- CHIP-0058 transport client + GCS filter + bulk-scanning (v2, in OOS).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **SIM-01** | Test helper (`chia-sdk-test::silent_payments::tweak_data_from_simulator_block` or similar, behind `chip-0057`) generates `TweakData` from a simulator block by collecting that block's standard-puzzle spends, extracting their synthetic pubkeys, computing per-spend `tweak_point = input_hash * A_sum`, and pairing with the block's outputs. Lives in the public test crate so binding test suites can reach it via the public API. | §"Standard Stack" + §"Architecture Patterns" §1 (helper algorithm + crate placement) + §"Code Examples" §1. **First time `chia-sdk-test` gains chip-0057** — feature wiring task required. |
| **SIM-02** | End-to-end test against `Simulator`: sender → unlabeled SP address → block farmed → `tweak_data_from_simulator_block` → recipient `scan_from_tweaks` finds the coin → recipient derives `onetime_sk` and spends the detected coin. | §"Architecture Patterns" §2 (test shape) + §"Code Examples" §2 (end-to-end Rust test skeleton). All 5 phases of dependent code already exist. |
| **SIM-03** | Same end-to-end flow with a **labeled** recipient address. Plus the deferred m=0 sub-test (Open Q1). | §"Architecture Patterns" §3 (labeled flow) + §"Code Examples" §3 + §"Open Questions" §1 (m=0 redesign). |
| **BIND-03** | AVA tests (napi + wasm) + pytest (pyo3) cover the full address-gen + send + scan-from-tweaks round trip from each language. Verify `Vec<chia_bls::PublicKey>` marshaling for `TweakData.tweak_points`. | §"Architecture Patterns" §4 (cross-language test shape) + §"Code Examples" §4 (TS skeleton). All required bindings already present (Phase 5); ONE NEW descriptor entry needed (`Simulator.tweak_data_from_block`). |
| **EX-01** | `examples/silent_payment.rs` shows the full flow runnable against the simulator. Mirrors `examples/cat_spends.rs` and `examples/spend_simulator.rs`. | §"Code Examples" §5 (example skeleton + line budget). Prelude already re-exports every needed symbol. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Rust 1.90.0 (pinned in `rust-toolchain.toml`), edition 2024.** No nightly-only features.
- **Workspace lints:** `deny clippy::all`, `warn pedantic`, `warn cargo`, `deny unsafe_code`, `deny dead_code`. Every new file (helper, tests, example) inherits.
- **`cargo machete` in CI** — no new `[package.metadata.cargo-machete] ignored` entries.
- **Per-crate builds with and without `--all-features`** — new chip-0057-gated code in `chia-sdk-test` must compile in every permutation. **Phase 6 adds a `chip-0057 = []` feature line to `chia-sdk-test/Cargo.toml` AND a new CI build line `cargo build -p chia-sdk-test -F chip-0057`.**
- **No new workspace deps.** `chia-sdk-test` already pulls `chia-sdk-types`; needs to gate access to `chia-sdk-driver` (or duplicate the algorithm); see §"Architecture Patterns" §1.
- **LSP-over-grep for code navigation.** Run clippy after editing before reporting done.
- **GSD workflow:** all repo edits go through `/gsd:execute-phase` (this phase).

## Summary

Phase 6 is the closing v1 phase — every cryptographic primitive (Phase 1), address type (Phase 2), scanner (Phase 3), send-side action (Phase 4), and binding (Phase 5) is in place. Phase 6 connects them via:

1. **One new free function** `tweak_data_from_simulator_block(simulator, height) -> TweakData` in `chia-sdk-test::silent_payments` behind a NEW `chip-0057` feature on the chia-sdk-test crate (Phase 1 wired chip-0057 onto types/utils/driver but NOT onto chia-sdk-test — confirmed by inspecting `crates/chia-sdk-test/Cargo.toml:17-36`).
2. **Three Rust E2E tests** in `crates/chia-sdk-driver/src/silent_payments/` exercising unlabeled, labeled, and m=0 self-change detection against `Simulator`.
3. **One bindings descriptor entry** for `Simulator.tweakDataFromBlock` plus one facade method on `chia_sdk_bindings::Simulator`.
4. **Three cross-language E2E tests** (napi AVA + pyo3 pytest + wasm AVA) running the unlabeled flow through the FFI boundary — first true runtime test of `Vec<chia_bls::PublicKey>` marshaling.
5. **One runnable example** `examples/silent_payment.rs` (~100 lines) demonstrating mnemonic → address → send → farm → scan → detect → spend with both unlabeled and labeled outputs.

**Primary recommendation:** Wave the work into 5 sub-plans aligned with the 5 requirements:
- Plan 06-01 (Wave 0): wire chip-0057 feature on `chia-sdk-test` + add `chip-0057 = ["chia-sdk-types/chip-0057","chia-sdk-driver/chip-0057","chia-sdk-utils/chip-0057"]` + new CI line; add `chia-sdk-driver` as an optional chip-0057-gated dep on `chia-sdk-test` (the helper must reach `compute_input_hash` + `aggregate_sender_sks` from driver crate — Phase 4 functions live in driver, not types).
- Plan 06-02: implement `tweak_data_from_simulator_block` + 2 helper-unit tests (SIM-01) — module `chia-sdk-test::silent_payments`.
- Plan 06-03: 3 Rust E2E tests (SIM-02, SIM-03 labeled, SIM-03 m=0-self-change-redesigned) in `chia-sdk-driver/src/silent_payments/e2e.rs` (planner picks file name) — uses Simulator from dev-deps + chip-0057 feature gate.
- Plan 06-04: bindings descriptor + facade method (`Simulator.tweakDataFromBlock`) + cross-language test trio (BIND-03).
- Plan 06-05: `examples/silent_payment.rs` (EX-01) + closeout (REQUIREMENTS.md updates + PHASE-SUMMARY).

**Critical decision the planner must lock before writing Plan 06-03:** the m=0 self-change sub-test must be REFRAMED — the SDK does NOT auto-emit m=0 outputs for self-sends (see Open Q1).

## Standard Stack

### Core (already present, no new deps)
| Library / Crate | Version | Purpose | Why Standard |
|---|---|---|---|
| `chia-sdk-test` | 0.33.0 (path) | Simulator + BlsPair fixtures | Existing canonical integration-test substrate; Phase 4 simulator-driven SP tests live in `actions/send.rs::silent_payment_tests` |
| `chia-sdk-driver` | 0.33.0 (path) | Phase 4 `aggregate_sender_sks` + `compute_input_hash`; Phase 3 scanner | The helper algorithm needs these — they live in driver crate, NOT types |
| `chia-sdk-utils` | 0.33.0 (path) | `SilentPaymentKeys`, `SilentPaymentAddress`, `LabelRegistry` | Already wired into Phase 4 SP tests via dev-deps |
| `chia-sdk-types` | 0.33.0 (path) | `ScalarField` | Foundational; used transitively via driver crate |
| `chia-bls` | 0.36.1 | `SecretKey`, `PublicKey`, `scalar_multiply` | Already in workspace; `PublicKey::scalar_multiply(&[u8; 32])` is the in-place tweak operation |
| `chia-protocol` | 0.36.1 | `CoinSpend`, `Bytes32`, `CoinState` | The block-spends iteration types |
| `clvmr` | 0.16.2 | `Allocator` for puzzle reveal parsing | Already used in `napi/__test__/action_system.spec.ts::fetchCat` precedent |
| `clvm-utils`, `clvm-traits` | 0.36.1 | `FromClvm`/`ToClvm` derives, `tree_hash` | Same |
| `chia-puzzle-types` | 0.36.1 | `StandardArgs::curry_tree_hash`, `derive_synthetic` | Recognize standard-puzzle spends (CHIP-0057 §10b) |
| `anyhow`, `rstest`, `indexmap` | various | test scaffolding | Existing precedent across all Phase 4 tests |
| `bip39 = 2.2.0` | | Mnemonic parsing | Already used by Phase 2 |
| `hex-literal = 0.4.1` | | TV1 byte-pinning | Existing dev-dep |

**Version verification:** No new packages added in Phase 6. All crates above are existing workspace members or upstream deps pinned at the documented versions. Skipped `npm view` — no external npm dependencies.

### Supporting (no new additions)
- `napi/__test__/silent_payments.spec.ts` (Phase 5 created) — Phase 6 adds sibling `silent_payments_e2e.spec.ts` or extends in-place (planner's discretion).
- `wasm/__test__/wasm.spec.ts` — Phase 6 adds sibling `silent_payments.spec.ts`.
- `pyo3/tests/test_pyo3.py` — Phase 6 adds sibling `test_silent_payments.py`.

### Alternatives Considered (and rejected)
| Instead of | Could Use | Why Rejected |
|---|---|---|
| New `chip-0057` feature on `chia-sdk-test` | Co-locate helper in `chia-sdk-driver` behind chip-0057 | SIM-01 explicitly places the helper in `chia-sdk-test`. Co-locating in driver would force binding consumers to import driver-test-helpers, polluting the production driver crate's API surface. |
| `chia-sdk-test` depending on `chia-sdk-driver` (chip-0057 optional) | Duplicate `aggregate_sender_sks` + `compute_input_hash` algorithms in chia-sdk-test | Duplication = drift hazard. Phase 1's `ScalarField` boundary would have to be re-imported with chip-0057 features. **Recommendation: add optional `chia-sdk-driver` dep to chia-sdk-test gated by its new `chip-0057` feature.** Mirrors the existing `chia-sdk-bindings → chia-sdk-driver` edge. |
| Cross-language tests use a synthetic `TweakData` fixture | Construct `TweakData` from hardcoded bytes in TS/Py | D-01 explicitly mandates the full FFI round-trip via the bindings-exposed simulator helper — gives BIND-03 strongest claim that `Vec<PublicKey>` marshaling works in practice. |
| Mock the simulator with hardcoded coin spends | Use the real `chia-sdk-test::Simulator` | The SDK already has a working sim and Phase 4 SP tests pass against it (see `actions/send.rs::silent_payment_tests::input_hash_round_trip`). Mocking adds maintenance for zero new signal. |

**Installation:** No new packages. Plan 06-01 modifies:
- `crates/chia-sdk-test/Cargo.toml` — adds `[features]` section (currently has `peer-simulator` and `serde` features) with `chip-0057 = ["dep:chia-sdk-driver", "chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]` plus `chia-sdk-driver = { workspace = true, optional = true }` in `[dependencies]`.
- `crates/chia-sdk-bindings/Cargo.toml:26` — `chia-sdk-test = { workspace = true }` becomes `chia-sdk-test = { workspace = true, features = ["chip-0057"] }`.
- `.github/workflows/rust.yml` — adds `cargo build --release -p chia-sdk-test -F chip-0057` after the existing line 63 (`chia-sdk-test --all-features`).
- Root `Cargo.toml` lines 73-80 already cascade chip-0057 to types/driver/utils — add `"chia-sdk-test/chip-0057"` to the workspace-level `chip-0057` feature so `cargo build --workspace --all-features` enables it.

## Architecture Patterns

### Recommended Module Structure

```
crates/
├── chia-sdk-test/
│   ├── Cargo.toml                          # NEW chip-0057 feature + optional chia-sdk-driver dep
│   └── src/
│       ├── lib.rs                          # pub use silent_payments::*; under #[cfg(feature = "chip-0057")]
│       └── silent_payments/                # NEW directory (chip-0057-gated)
│           ├── mod.rs                      # barrel
│           └── tweak_data.rs               # tweak_data_from_simulator_block + 2 unit tests
├── chia-sdk-driver/src/silent_payments/
│   └── e2e.rs                              # NEW — 3 Rust E2E tests (planner picks file name)
├── chia-sdk-bindings/
│   ├── Cargo.toml                          # MODIFIED — chia-sdk-test now gets chip-0057
│   └── src/simulator.rs                    # MODIFIED — add tweak_data_from_block method
├── bindings/simulator.json                 # MODIFIED — add tweakDataFromBlock entry
napi/__test__/silent_payments_e2e.spec.ts   # NEW
wasm/__test__/silent_payments.spec.ts       # NEW
pyo3/tests/test_silent_payments.py          # NEW
examples/silent_payment.rs                  # NEW
```

### Pattern 1: `tweak_data_from_simulator_block` — Algorithm + Placement
**What:** Free function in `chia-sdk-test::silent_payments` that walks the simulator's internal state to (a) identify which coin spends landed in the given block, (b) extract synthetic sender public keys from each spend's puzzle reveal (standard-puzzle spends only), (c) compute `tweak_point = input_hash * A_sum` per spend group, (d) collect every output of the block as `OutputMeta`.

**When to use:** Test-side only — production callers go through the future CHIP-0058 transport client.

**Crate-private accessor pattern (cannot use `pub fn coin_spend` alone — no height association):**

The simulator's private `SimulatorData` (`crates/chia-sdk-test/src/simulator/data.rs`) stores:
- `coin_states: IndexMap<Bytes32, CoinState>` — each `CoinState` has `spent_height: Option<u32>` and `created_height: Option<u32>`.
- `coin_spends: IndexMap<Bytes32, CoinSpend>` — coin_id → CoinSpend.

The two maps are not joined by height in the existing API. **Strategy:** Implement the helper as `pub fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> Result<TweakData, ?>` *inside* `chia-sdk-test` so it can reach the existing public `coin_state(coin_id) -> Option<CoinState>` and `coin_spend(coin_id) -> Option<CoinSpend>` accessors. Iterate `coin_states` once (private access via `pub(crate)` accessor OR a new `pub fn block_spends(height: u32) -> Vec<CoinSpend>` accessor on Simulator), filtering by `spent_height == Some(height)`.

**Recommended: add ONE new public accessor `Simulator::block_spends(&self, height: u32) -> Vec<CoinSpend>` that iterates `data.coin_states` and joins to `data.coin_spends`.** This is the smallest privacy break, keeps the helper itself out of the simulator's internals, and makes the helper a thin top-level function. The accessor goes into `simulator.rs` alongside `coin_spend` and `lookup_puzzle_hashes`. (Not chip-0057-gated; it's just a height-indexed read.)

**Algorithm (per CHIP-0057 §425, §446, §459):**

```
fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> Result<TweakData, SilentPaymentError> {
    // 1. Collect all coin spends that resolved in this block.
    let block_spends = sim.block_spends(height);

    // 2. Group standard-puzzle spends. Per CHIP §10b a "standard-puzzle spend"
    //    is one whose puzzle reveal currys StandardArgs(synthetic_key). We
    //    parse each puzzle reveal with chia_puzzle_types::standard::StandardArgs
    //    to extract `synthetic_key`. Non-standard puzzles are skipped silently
    //    (CAT/NFT/etc.).
    //
    // 3. For the SP send model: ALL standard-puzzle spends in the block share
    //    one input_hash + one A_sum (the transaction's spends are atomic per
    //    Phase 4.1 Relation::AssertConcurrent). The helper aggregates ALL
    //    extracted synthetic_keys into one A_sum and ALL spent coin_ids into
    //    one input_hash, producing ONE tweak_point.
    //
    //    NOTE for planner: this assumes one SP transaction per block. The
    //    helper MAY return Vec<TweakPoint> if multiple disjoint SP transactions
    //    appear in one block — but Phase 6 tests construct exactly one per
    //    block, so the planner can pick whichever shape is cleaner.

    // 4. Compute tweak_point = input_hash * A_sum.
    //    - input_hash: chia_sdk_driver::compute_input_hash(&coin_ids, &agg_pk).as_bytes()
    //    - A_sum: PublicKey aggregated from each extracted synthetic_key
    //    - tweak_point: let mut p = A_sum; p.scalar_multiply(&input_hash); p

    // 5. CHIP §459 guard: if tweak_point.is_inf(), skip silently (do not add).

    // 6. Collect every output of the block as OutputMeta. Iterate
    //    `coin_states` filtered by `created_height == Some(height)`:
    //      OutputMeta {
    //        puzzle_hash: coin.puzzle_hash,
    //        coin_id:     coin.coin_id(),
    //        amount:      coin.amount,
    //        parent_coin_id: coin.parent_coin_info,
    //      }

    // 7. Return TweakData { tweak_points, outputs }.
}
```

**Standard-puzzle detection:** Parse the `puzzle_reveal: Program` with `chia_puzzle_types::standard::StandardArgs` via `clvm_traits::FromClvm`. If it parses, extract `synthetic_key`; if not (CAT/NFT/genesis/etc.), skip. The `chia_sdk_test::Simulator` has no non-standard-puzzle outputs in Phase 6 tests, but the helper should be defensive.

**`K_MAX_DEFAULT`:** Helper doesn't need to know `K_MAX_DEFAULT = 2400`. The scanner enforces the cap; the helper just emits data.

### Pattern 2: Unlabeled E2E Test Shape (SIM-02)
**When to use:** Closes SC2 for ROADMAP Phase 6.

```rust
#[cfg(all(test, feature = "chip-0057"))]
#[test]
fn test_simulator_e2e_unlabeled() -> Result<()> {
    // Shared setup (planner extracts to helper fn):
    let (mut sim, mut ctx, sender, recipient) = setup_e2e()?;
    let recipient_address = recipient.unlabeled_address(SilentPaymentNetwork::Testnet);
    let height_before = sim.height();

    // Send: Alice → recipient_address, amount 100.
    let mut spends = Spends::new(sender.puzzle_hash);
    spends.add(sender.coin);
    let deltas = spends.apply(&mut ctx, &[Action::send(
        Id::Xch,
        SendDestination::SilentPayment(Box::new(recipient_address)),
        100,
        Memos::None,
    )])?;
    spends.with_silent_payment_keys(
        indexmap! { sender.puzzle_hash => sender.pk },
        indexmap! { sender.puzzle_hash => sender.sk.clone() },
    );
    let outputs = spends.finish_with_keys(&mut ctx, &deltas, Relation::None,
        &indexmap! { sender.puzzle_hash => sender.pk })?;

    // Farm: spend_coins creates a block via create_block() internally.
    sim.spend_coins(ctx.take(), &[sender.sk.clone()])?;

    // Extract: tweak_data from the freshly-farmed block.
    let tweak_data = tweak_data_from_simulator_block(&sim, height_before)?;

    // Scan: recipient detects the coin.
    let detections = recipient.scan(&tweak_data, None, K_MAX_DEFAULT);
    assert_eq!(detections.len(), 1, "expected exactly 1 detection");
    let detected = &detections[0];
    assert!(detected.label.is_none(), "unlabeled");
    assert_eq!(detected.k, 0);
    assert_eq!(detected.amount, 100);

    // Spend the detected coin: StandardLayer::new(synthetic).spend(...).
    let synthetic_pk = detected.onetime_sk.public_key().derive_synthetic();
    let synthetic_sk = detected.onetime_sk.derive_synthetic();
    let conditions = Conditions::new().create_coin(sender.puzzle_hash, 99, Memos::None).reserve_fee(1);
    StandardLayer::new(synthetic_pk).spend(
        &mut ctx,
        Coin::new(detected.parent_coin_id, detected.puzzle_hash, detected.amount),
        conditions,
    )?;
    sim.spend_coins(ctx.take(), &[synthetic_sk])?;

    Ok(())
}
```

**Critical:** The recipient signing key is `detected.onetime_sk.derive_synthetic()`. The `onetime_sk` Phase 3's scanner returns is the *raw* one-time SK; the standard puzzle's curried key is the synthetic. The matching synthetic-PK is `synthetic_sk.public_key()` (equivalently `onetime_sk.public_key().derive_synthetic()`).

### Pattern 3: Labeled E2E Test Shape (SIM-03 labeled half)
**When to use:** Closes SC3 first half.

Identical to Pattern 2 except:
- Recipient address constructed via `recipient.labeled_address(SilentPaymentNetwork::Testnet, 1)?`.
- Scanner is given a `LabelRegistry` with `m=1` registered: `let mut labels = LabelRegistry::new(); labels.register(recipient.scan_sk(), 1);` then `recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT)`.
- Detection assertion: `detected.label == Some(1)`.
- Labeled-coin signing requires `onetime_sk + label_scalar` — but the scanner's `onetime_sk` ALREADY includes the label scalar for labeled detections (see `crates/chia-sdk-driver/src/silent_payments/scanner.rs:132-137`: `labeled_sk = base_sk + label_scalar`). So the follow-on spend code path is IDENTICAL to the unlabeled case — `StandardLayer::new(detected.onetime_sk.public_key().derive_synthetic()).spend(...)` works for both.

### Pattern 3b: m=0 Self-Change Test Redesign (SIM-03 m=0 half)
**See Open Questions §1** — original D-04 assumption is FALSE. Recommended redesign:

The wallet author manually constructs the m=0 self-change pattern by *registering* m=0 in the `LabelRegistry` BUT still sending to their own UNLABELED address. The scanner then detects the self-send at k=0 with `label: None` — and the test asserts that:
1. The `LabelRegistry` can hold m=0 (verified at unit-test level by `labels.rs:14-16` — internal `register(scan_sk, 0)` is allowed).
2. A self-send to one's own unlabeled address detects normally.
3. Registering m=0 in the registry does NOT cause spurious m=0 detections on the unlabeled self-send (because the labeled branch only runs when `!found` at unlabeled — see `scanner.rs:124`).

**Alternative redesign:** Manually construct a labeled self-payment to one's own `m=1` labeled address, with `m=0` ALSO registered in the LabelRegistry, then assert the scanner correctly picks `label: Some(1)` (not 0). This proves the m=0 internal-registration path doesn't poison labeled detection.

**Either redesign closes SC3's "separate sub-test confirms a m=0 change-detection variant works internally" wording.** The original intent was a behavioral guarantee about change detection; what we can actually demonstrate is that the m=0 sentinel is internally consistent and doesn't break the registry. Researcher recommendation: **the m=0 sub-test asserts `LabelRegistry::register(scan_sk, 0)` + a self-send produces no spurious m=0 detection on unlabeled coins; it documents the internal-only nature of m=0 per `ADDR-06` and the labels.rs doc.** The planner should call this out clearly in the test's rustdoc.

### Pattern 4: Cross-Language Test Shape (BIND-03)
**TS skeleton (napi):** see §"Code Examples" §4.

Key marshaling check: after calling `simulator.tweakDataFromBlock(height)`, the returned `TweakData` object must expose `tweakPoints: Array<PublicKey>` (already confirmed in `napi/index.d.ts:3020-3024`). The cross-language test asserts:
1. `tweakData.tweakPoints.length === 1` (one transaction in the block).
2. `tweakData.outputs.length >= 1`.
3. `silentPaymentKeys.scan(tweakData, new LabelRegistry(), K_MAX_DEFAULT_AS_NUMBER)` returns a non-empty array.
4. `detections[0].coinId.equals(<sender-expected-onetime-coin-id>)` — proves cross-language correctness.

**Spending the detected coin from TS:** `clvm.standardSpend(detected.onetimeSk.publicKey().deriveSynthetic(), clvm.delegatedSpend(conditions))` produces a `Spend`; insert into `coinSpends` per `napi/__test__/action_system.spec.ts:148`. The same pattern works for pyo3 and wasm.

**Important:** `SilentPayments.scanFromTweaks` in the binding facade takes `labels: LabelRegistry` (NOT `Option<LabelRegistry>`). Unlabeled scans construct an empty `LabelRegistry`. This is fine because empty `LabelRegistry::iter()` yields no items — labeled branch in scanner is a no-op. Confirmed at `chia-sdk-bindings/src/silent_payments.rs:381`.

**Helper method:** Phase 5 did NOT expose `SilentPaymentKeys.scan(...)` as a bindings method (the convenience trait method). The facade only exposes `SilentPayments.scanFromTweaks(scanSk, spendSk, spendPk, data, labels, kMax)`. Cross-language tests use the static-method form.

### Pattern 5: Example Skeleton (EX-01)
See §"Code Examples" §5 for the full template. The rhythm mirrors `cat_spends.rs`:
1. Setup: `Simulator::new()`, `SpendContext::new()`, sender `bls(1_000)`.
2. Recipient key derivation: hardcoded mnemonic → `SilentPaymentKeys` → unlabeled + `labeled_address(1)`.
3. Two `Action::send` calls (one per destination) in a single `Spends.apply`.
4. `with_silent_payment_keys` + `finish_with_keys` + `spend_coins`.
5. `tweak_data_from_simulator_block(&sim, height)`.
6. `recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT)` with `m=1` registered.
7. Spend both detected coins via `StandardLayer::new(...).spend(...)`.
8. `println!` checkpoints throughout per the existing example conventions.

Estimated line count: ~100 (between `cat_spends.rs` at 42 and `custom_p2_puzzle.rs` at 130).

### Anti-Patterns to Avoid

- **DO NOT call `Action::send(Id::Existing(_), SendDestination::SilentPayment(...), ...)`** — fires `DriverError::SilentPaymentRequiresXch` (Phase 4.2 acceptance test). SP-XCH-only in v1.
- **DO NOT skip `with_silent_payment_keys` before `finish_with_keys`** when any pending SP exists — fires `DriverError::SilentPaymentKeysNotRegistered`.
- **DO NOT use raw `onetime_sk.public_key()` to spend** — must apply `.derive_synthetic()` since the on-chain puzzle is `StandardArgs(synthetic_key)`.
- **DO NOT hand-roll a CHIP-0058 transport client in the example** — Cross-cutting concern #5; example uses ONLY `tweak_data_from_simulator_block`.
- **DO NOT use real-time timestamps** — `SimulatorConfig::default()` uses `TimestampMode::Increment { start: 0, step: 1 }` deterministically. Phase 6 tests must use the default seed (1337) or pass an explicit seed; see §"Simulator determinism" below.
- **DO NOT promote `tweak_data_from_simulator_block` to a `pub fn` on the `Simulator` struct directly** — it's a free function in `chia-sdk-test::silent_payments` per the literal SIM-01 wording.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| BLS scalar multiplication | Custom scalar-multiply via `num-bigint` | `chia_bls::PublicKey::scalar_multiply(&[u8; 32])` (in-place) | Already used by Phase 3 scanner; constant-time, vetted |
| Standard-puzzle parsing | Hand-roll Chialisp opcode walk | `clvm_traits::FromClvm` derive on `chia_puzzle_types::standard::StandardArgs` | The pattern is established; see `napi/__test__/action_system.spec.ts:50` for the `parseChildCats` precedent |
| `derive_synthetic` for SP one-time keys | Re-implement BIP-32-style derivation | `chia_puzzle_types::DeriveSynthetic::derive_synthetic` on both SecretKey and PublicKey | Pinned by `chia-bls 0.36.1`; cross-binding-target exposed |
| Aggregating sender SKs | New aggregation in chia-sdk-test | `chia_sdk_driver::aggregate_sender_sks` (Phase 4 Plan 04-01) | Already byte-pinned against TV4; under chip-0057 |
| Computing input_hash | New tagged-hash construction | `chia_sdk_driver::compute_input_hash(&coin_ids, &agg_pk)` (Phase 4 Plan 04-01) | TV1-pinned; uses tagged_hash correctly |
| Block-height accessor | Walk private SimulatorData fields | Add ONE small `pub fn block_spends(height) -> Vec<CoinSpend>` to `Simulator` | Smallest possible surface; not chip-0057-gated |
| Deterministic sim seeding | New rand setup | `Simulator::with_config(SimulatorConfig { seed: <test-specific>, ..Default::default() })` | Already there; `seed: 1337` is the default |
| `StandardLayer` spend in tests | Hand-build `CoinSpend` | `StandardLayer::new(synthetic_pk).spend(ctx, coin, conditions)` | Existing pattern used by every other simulator-driven test |

**Key insight:** Phase 6 is almost entirely composition — every primitive needed already exists. The one new piece of logic is `tweak_data_from_simulator_block` which composes existing Phase 4 functions.

## Runtime State Inventory

This is NOT a rename/refactor/migration phase. **OMITTED — no stored data, live service config, OS-registered state, secrets, or build artifacts to migrate.**

## Environment Availability

Phase 6 depends on existing local tools that Phases 1-5 already exercised:

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `cargo` / `rustc` 1.90.0 | All Rust work | ✓ | matches `rust-toolchain.toml` | — |
| `pnpm` 9.11.0 | napi+wasm tests | ✓ | confirmed by Phase 5 Plan 05-04 | — |
| `maturin` | pyo3 test | ✓ (Phase 5 Plan 05-03 created `pyo3/.venv`) | per pyo3/pyproject.toml | — |
| `wasm-pack` 0.13.1 | wasm test | ✓ (Phase 5 pinned to 0.13.1) | 0.13.1 | — |
| `node` ≥ 20 | AVA test runner | ✓ | from package.json engines | — |
| `python` ≥ 3.8 | pytest | ✓ | from pyo3/pyproject.toml | — |

**No new dependencies. No fallback strategies needed.**

## Common Pitfalls

### Pitfall 1: chia-sdk-test didn't have chip-0057 feature before Phase 6
**What goes wrong:** Plan author assumes Phase 1 wired chip-0057 onto chia-sdk-test; tries to add `tweak_data_from_simulator_block` and gets `cannot find ScalarField` or `cannot find compute_input_hash` errors.
**Why it happens:** Phase 1 Plan 01-01 wired chip-0057 onto root + types + driver + utils (per STATE.md). `chia-sdk-test/Cargo.toml:17-36` confirms only `peer-simulator` and `serde` features exist.
**How to avoid:** Plan 06-01 explicitly wires the feature: add `chip-0057 = ["dep:chia-sdk-driver", "chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]` to `crates/chia-sdk-test/Cargo.toml`. Add `chia-sdk-driver = { workspace = true, optional = true }` to deps (currently chia-sdk-test does NOT depend on chia-sdk-driver — confirmed by `crates/chia-sdk-test/Cargo.toml:38-68`). Add `cargo build -p chia-sdk-test -F chip-0057` to CI. Add `"chia-sdk-test/chip-0057"` to root Cargo.toml's `chip-0057` workspace feature.
**Warning signs:** `cargo build --workspace --all-features` succeeds (because chip-0057 isn't in the cascade yet), but `cargo build -p chia-sdk-test -F chip-0057` fails.

### Pitfall 2: m=0 self-change is NOT auto-emitted
**What goes wrong:** Plan writes `test_simulator_e2e_m0_self_change` assuming sender→sender SP send produces a `label: Some(0)` detection.
**Why it happens:** D-04 explicitly flagged this as needing verification. Verification result (see Open Q1): the SDK's send path emits a normal SP output regardless of recipient identity. The recipient's spend_pk is whatever the sender encoded in the address (unlabeled `B_spend` if the sender chose `unlabeled_address`, labeled `B_m` if labeled). There is NO branch that auto-promotes self-sends to m=0.
**How to avoid:** Redesign the m=0 test per §"Architecture Patterns" §3b. Document in the test's rustdoc that m=0 is an internal sentinel for wallet authors who want to track change in their own LabelRegistry; the SDK does not enforce its emission. Cross-reference `ADDR-06`'s `ReservedChangeLabel` and `labels.rs:14-16`.
**Warning signs:** The test asserts `detected.label == Some(0)` and fails because the actual detection is `label: None`.

### Pitfall 3: `Vec<chia_bls::PublicKey>` round-trip
**What goes wrong:** Plan 06-04 cross-language tests fail at the FFI boundary on `tweakData.tweakPoints` access.
**Why it happens:** Phase 5 Plan 05-03 verified compile-time marshaling is fine (the Vec<bindy-class> case works). But this is the FIRST runtime test that actually instantiates and accesses `tweakData.tweakPoints` from TS/Py/WASM.
**How to avoid:** Wave the cross-language tests so the napi case runs first; if it fails, the other two will too. Add a minimal smoke test as Plan 06-04 Wave 0: `const td = simulator.tweakDataFromBlock(0); assert.equal(td.tweakPoints.length, 0)` for an empty block — this isolates the marshaling from the algorithm. Phase 5 PHASE-SUMMARY lesson #6 documents the `Vec<(K,V)>` failure mode and the wrapper-class workaround; if `Vec<PublicKey>` hits the same issue, the same workaround (newtype wrapper) applies.
**Warning signs:** `RangeError`, `TypeError: Cannot read property 'tweakPoints'`, or silent-empty arrays on the TS side.

### Pitfall 4: Simulator determinism across cross-language tests
**What goes wrong:** AVA, pytest, and AVA-wasm tests intermittently pick up different `coin_id`s or different one-time puzzle hashes because the simulator's RNG is seeded differently.
**Why it happens:** `Simulator::new()` uses `SimulatorConfig::default()` which sets `seed: 1337` deterministically — but `BlsPair::new(seed)` is called with `self.data.rng.random()` which advances per-test. As long as each test calls `Simulator::new()` fresh AND uses the same operation sequence, results are deterministic.
**How to avoid:** Each cross-language test calls `new Simulator()` fresh (no shared state). Operation sequence: `bls(amount)` → register SP keys → `Action::send` → `finish_with_keys` → `spend_coins` → `tweakDataFromBlock`. Pin the test mnemonic across all three (BIP-39 TV1 `abandon×11+about` per Phase 5 precedent in `napi/__test__/silent_payments.spec.ts:24`) so the recipient address is identical across binding targets.
**Warning signs:** Test passes locally but fails in CI, or passes once and fails on rerun without other changes.

### Pitfall 5: Standard-puzzle parsing in `tweak_data_from_simulator_block`
**What goes wrong:** Helper crashes on non-standard puzzles (CAT/NFT/etc. that may appear in future scenarios).
**Why it happens:** The helper must defensively skip puzzles it doesn't recognize. If it `unwrap()`s the FromClvm parse, a future test with CAT/NFT spends in the same block panics.
**How to avoid:** Use `if let Ok(args) = StandardArgs::from_clvm(...)` (or `.ok()` on the Result) to skip non-standard puzzles silently. CHIP-0057 §10b explicitly says only standard-puzzle spends contribute to A_sum.

### Pitfall 6: `derive_synthetic` direction confusion
**What goes wrong:** Detected coin spend fails signature verification because the wrong synthetic-vs-raw key was used.
**Why it happens:** `detected.onetime_sk` (Phase 3 scanner output) is the RAW spend SK `b_spend + t_k` (unlabeled) or `b_spend + t_k + label_scalar` (labeled). The on-chain puzzle currys `StandardArgs(synthetic_key)` where `synthetic_key = raw_key + synthetic_offset`. Spending requires the SYNTHETIC SK and matching synthetic PK on the StandardLayer.
**How to avoid:** Always use `let synthetic_sk = detected.onetime_sk.derive_synthetic(); let synthetic_pk = synthetic_sk.public_key();` and pass `synthetic_pk` to `StandardLayer::new(...)`. Verify via grep: there should be ONE `.derive_synthetic()` call per detected coin in every E2E test and example.
**Warning signs:** Simulator `spend_coins` returns `AggSigMeError` or similar signature-related error.

### Pitfall 7: chia-sdk-test → chia-sdk-driver new dep introduces a cycle?
**What goes wrong:** Adding `chia-sdk-driver` as a dep of `chia-sdk-test` creates a cycle if `chia-sdk-driver` depends on `chia-sdk-test`.
**Why it doesn't happen here:** Confirmed by grep: `chia-sdk-driver`'s production deps do NOT include `chia-sdk-test`. The latter is used only as a dev-dependency (in `chia-sdk-driver/Cargo.toml` `[dev-dependencies]` section, used by `actions/send.rs::silent_payment_tests` etc.). Dev-deps don't form cycles for production builds.
**How to avoid:** Plan 06-01 explicitly states the new edge is `chia-sdk-test → chia-sdk-driver (optional, chip-0057-gated)`. No driver-side change.

## Code Examples

### Example 1: `tweak_data_from_simulator_block` (SIM-01)

```rust
// crates/chia-sdk-test/src/silent_payments/tweak_data.rs

use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_puzzle_types::standard::StandardArgs;
use chia_sdk_driver::silent_payments::{
    compute_input_hash, OutputMeta, TweakData,
};
use chia_sdk_types::silent_payments::ScalarField;
use clvm_traits::FromClvm;
use clvmr::Allocator;

use crate::Simulator;

/// Construct a [`TweakData`] from one block of the simulator's history.
///
/// Walks every coin spend that resolved in `height`, parses each standard-puzzle
/// spend's `synthetic_key`, aggregates them into `A_sum`, computes the per-block
/// `input_hash = tagged_hash("Chia_SP/Inputs", lex_min_coin_id || A_sum)`, and
/// returns `tweak_point = input_hash * A_sum` paired with every output created
/// in the block.
///
/// Returns an empty `TweakData` when the block contains no standard-puzzle
/// spends.
///
/// **CHIP §459 guard:** if the computed `tweak_point` is the BLS12-381 identity
/// element (vanishingly unlikely on real input), no tweak point is emitted —
/// matches the scanner's identity-element skip rule.
#[must_use]
pub fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> TweakData {
    let block_spends = sim.block_spends(height);

    let mut allocator = Allocator::new();
    let mut synthetic_pks: Vec<PublicKey> = Vec::new();
    let mut spent_coin_ids: Vec<Bytes32> = Vec::new();

    for spend in &block_spends {
        let Ok(ptr) = spend.puzzle_reveal.to_clvm(&mut allocator) else { continue };
        let Ok(args) = StandardArgs::from_clvm(&allocator, ptr) else { continue };
        synthetic_pks.push(args.synthetic_key);
        spent_coin_ids.push(spend.coin.coin_id());
    }

    let mut tweak_points: Vec<PublicKey> = Vec::new();
    if !synthetic_pks.is_empty() {
        // Aggregate A_sum.
        let mut agg = synthetic_pks[0];
        for pk in &synthetic_pks[1..] { agg = &agg + pk; }
        // Compute input_hash with the existing Phase 4 free function.
        let input_hash: ScalarField = compute_input_hash(&spent_coin_ids, &agg);
        // tweak_point = input_hash * A_sum.
        let mut tweak_point = agg;
        tweak_point.scalar_multiply(&input_hash.to_bytes());
        if !tweak_point.is_inf() {
            tweak_points.push(tweak_point);
        }
    }

    // Outputs: every coin created at `height`.
    let outputs: Vec<OutputMeta> = sim.block_outputs(height).into_iter().map(|coin| OutputMeta {
        puzzle_hash: coin.puzzle_hash,
        coin_id: coin.coin_id(),
        amount: coin.amount,
        parent_coin_id: coin.parent_coin_info,
    }).collect();

    TweakData { tweak_points, outputs }
}
```

**Plus two new public Simulator accessors** (in `crates/chia-sdk-test/src/simulator.rs`, NOT chip-0057-gated — they're general-purpose):
- `pub fn block_spends(&self, height: u32) -> Vec<CoinSpend>` — joins `data.coin_states` (filtered by `spent_height == Some(height)`) with `data.coin_spends`.
- `pub fn block_outputs(&self, height: u32) -> Vec<Coin>` — iterates `data.coin_states` filtered by `created_height == Some(height)`.

### Example 2: SIM-02 Rust E2E test
See Pattern 2 above (§"Architecture Patterns") for the full sketch.

### Example 3: SIM-03 labeled + m=0 redesign
See Pattern 3 and Pattern 3b above.

### Example 4: napi cross-language E2E (BIND-03)

```typescript
// napi/__test__/silent_payments_e2e.spec.ts
import test from "ava";
import {
  Action,
  Clvm,
  Conditions,
  Id,
  LabelRegistry,
  Mnemonic,
  SendDestination,
  SilentPaymentKeys,
  SilentPaymentNetwork,
  SilentPaymentRegisteredKey,
  SilentPaymentRegisteredSecretKey,
  SilentPayments,
  Simulator,
  Spends,
  standardPuzzleHash,
} from "..";

const TV1_MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const K_MAX_DEFAULT = 2400;

test("BIND-03: unlabeled SP send + scan-from-tweaks E2E (napi)", (t) => {
  const sim = new Simulator();
  const clvm = new Clvm();

  // Recipient: deterministic mnemonic so the test is reproducible.
  const recipient = SilentPaymentKeys.fromMnemonic(new Mnemonic(TV1_MNEMONIC));
  const recipientAddress = recipient.unlabeledAddress(SilentPaymentNetwork.Testnet);

  // Sender: fresh BLS pair from simulator.
  const sender = sim.bls(1_000n);
  const heightBefore = sim.height();

  // Build the SP send via the unified Action::send + SendDestination + with_silent_payment_keys path.
  const spends = new Spends(clvm, sender.puzzleHash);
  spends.addXch(sender.coin);
  const action = Action.send(
    Id.xch(),
    SendDestination.silentPayment(recipientAddress),
    100n,
    undefined,
  );
  spends.withSilentPaymentKeys(
    [new SilentPaymentRegisteredKey(sender.puzzleHash, sender.pk)],
    [new SilentPaymentRegisteredSecretKey(sender.puzzleHash, sender.sk)],
  );
  // ... apply, prepare, standard-spend the inputs, spend_coins per
  // napi/__test__/action_system.spec.ts:142-157 pattern.

  // Extract TweakData via the Phase-6 bindings helper.
  const tweakData = sim.tweakDataFromBlock(heightBefore);
  t.is(tweakData.tweakPoints.length, 1, "one SP transaction → one tweak_point");
  t.true(tweakData.outputs.length >= 1, "at least the recipient's output");

  // Scan — Vec<PublicKey> marshaling check happens here on tweakData.tweakPoints access.
  const labels = new LabelRegistry();
  const detections = SilentPayments.scanFromTweaks(
    recipient.scanSk(),
    recipient.spendSk(),
    recipient.spendPk(),
    tweakData,
    labels,
    K_MAX_DEFAULT,
  );

  t.is(detections.length, 1, "scanner finds the SP coin");
  t.is(detections[0].k, 0);
  t.is(detections[0].label, undefined, "unlabeled detection");
  t.is(detections[0].amount, 100n);

  // Spend the detected coin from TypeScript.
  const onetimeSk = detections[0].onetimeSk;
  const syntheticSk = onetimeSk.deriveSynthetic();
  const syntheticPk = syntheticSk.publicKey();
  // ... build conditions, clvm.standardSpend(syntheticPk, clvm.delegatedSpend(conds)),
  //     insert into coinSpends, sim.spendCoins(clvm.coinSpends(), [syntheticSk]).
});
```

**Pyo3 mirror:** same calls in snake_case (`silent_payments_keys.scan_sk()`, `simulator.tweak_data_from_block(height)`, `silent_payments.scan_from_tweaks(...)`). `pytest` test in `pyo3/tests/test_silent_payments.py`.
**Wasm mirror:** same calls in camelCase with `setPanicHook()` at top per `wasm/__test__/wasm.spec.ts:14` precedent.

### Example 5: `examples/silent_payment.rs` (EX-01)

```rust
use anyhow::Result;
use bip39::Mnemonic;
use chia_wallet_sdk::prelude::*;
use indexmap::indexmap;

fn main() -> Result<()> {
    // 1. Setup
    let mut sim = Simulator::new();
    let ctx = &mut SpendContext::new();
    let sender = sim.bls(1_000);
    let mnemonic = Mnemonic::parse(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )?;
    let recipient = SilentPaymentKeys::from_mnemonic(&mnemonic);

    let unlabeled_addr = recipient.unlabeled_address(SilentPaymentNetwork::Testnet);
    let labeled_addr = recipient.labeled_address(SilentPaymentNetwork::Testnet, 1)?;

    println!("Recipient unlabeled address: {}", unlabeled_addr.encode()?);
    println!("Recipient labeled address (m=1): {}", labeled_addr.encode()?);

    // 2. Send: one unlabeled + one labeled, in one tx
    let height_before = sim.height();
    let mut spends = Spends::new(sender.puzzle_hash);
    spends.add(sender.coin);
    let deltas = spends.apply(ctx, &[
        Action::send(
            Id::Xch,
            SendDestination::SilentPayment(Box::new(unlabeled_addr)),
            100,
            Memos::None,
        ),
        Action::send(
            Id::Xch,
            SendDestination::SilentPayment(Box::new(labeled_addr)),
            200,
            Memos::None,
        ),
    ])?;
    spends.with_silent_payment_keys(
        indexmap! { sender.puzzle_hash => sender.pk },
        indexmap! { sender.puzzle_hash => sender.sk.clone() },
    );
    spends.finish_with_keys(ctx, &deltas, Relation::None, &indexmap! { sender.puzzle_hash => sender.pk })?;
    sim.spend_coins(ctx.take(), &[sender.sk.clone()])?;
    println!("Sent 100 mojos unlabeled + 200 mojos labeled (m=1).");

    // 3. Receive: extract tweak data, scan
    let tweak_data = chia_sdk_test::silent_payments::tweak_data_from_simulator_block(&sim, height_before);
    let mut labels = LabelRegistry::new();
    labels.register(recipient.scan_sk(), 1);
    let detections = recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT);
    println!("Detected {} silent-payment outputs.", detections.len());

    // 4. Spend each detected coin
    for d in &detections {
        let synthetic_sk = d.onetime_sk.derive_synthetic();
        let synthetic_pk = synthetic_sk.public_key();
        let conditions = Conditions::new().create_coin(sender.puzzle_hash, d.amount - 1, Memos::None).reserve_fee(1);
        StandardLayer::new(synthetic_pk).spend(
            ctx,
            Coin::new(d.parent_coin_id, d.puzzle_hash, d.amount),
            conditions,
        )?;
        sim.spend_coins(ctx.take(), &[synthetic_sk])?;
        println!("Spent detected coin {} (label = {:?}).", d.coin_id, d.label);
    }

    Ok(())
}
```

Estimated length: ~85 lines including the use block. Builds under `cargo build --examples --all-features` (the example accesses chip-0057 prelude re-exports plus the new `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`).

**Note:** `chia_wallet_sdk::prelude` does NOT currently re-export `tweak_data_from_simulator_block` (it doesn't exist yet). The example uses the fully-qualified path. Optionally, Plan 06-05 can add a chip-0057-gated re-export to the umbrella's prelude.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| Phase 4 `Action::silent_payment_send(addr, amt, memos)` constructor | `Action::send(Id::Xch, SendDestination::SilentPayment(Box::new(addr)), amt, memos)` | Phase 4.2 (2026-05-17) | Phase 6 callers MUST use the new shape; all SP tests in `actions/send.rs::silent_payment_tests` already use it |
| Phase 4 `spends.finish_with_silent_payment_keys(ctx, deltas, rel, pks, sks)` | `spends.with_silent_payment_keys(pks, sks); spends.finish_with_keys(ctx, deltas, rel, &pks)` | Phase 4.2 | Same as above |
| Opcode 60/61 announcement binding for multi-input SP | `Relation::AssertConcurrent` cycle (opcode 64 SCC) | Phase 4.1 (2026-05-17) | Phase 6 single-input tests use `Relation::None`; example uses `Relation::None` (one sender XCH input) |
| Phase 4 `Spends::silent_payment_synthetic_sks` was a separate finish-method arg | Now a `pub(crate)` field set via `with_silent_payment_keys` builder | Phase 4.2 | E2E tests must call `with_silent_payment_keys` before `finish_with_keys` |
| Pre-Phase 5 `Action.send(id, puzzleHashBytes, ...)` in TS | `Action.send(id, SendDestination.puzzleHash(bytes), ...)` or `SendDestination.silentPayment(addr)` | Phase 5 (2026-05-18) | TS-side `From<Bytes32>` ergonomic does NOT cross FFI — cross-language tests must explicitly wrap |

**Deprecated/outdated:**
- `Spends::finish_with_silent_payment_keys` — deleted in Phase 4.2.
- `Action::SilentPaymentSend` variant — deleted in Phase 4.2.
- `actions/silent_payment_send.rs` — deleted in Phase 4.2 (640 lines).
- Per Phase 4.1: `Spends::emit_silent_payment_announcements` — deleted.

## Open Questions

### 1. m=0 self-change test design — RESOLVED (assumption FALSE)

**What we know:**
- The SDK's `actions/send.rs:50-56` chip-0057 SP arm dispatches on `SendDestination::SilentPayment(addr)` and calls `spend_silent_payment(ctx, spends, addr, amount, memos)`. The `addr` is whatever the sender passed — there is NO branch checking `sender.scan_pk() == addr.scan_pk` or similar.
- `spend_silent_payment` (`actions/send.rs:135-179`) records a `SilentPaymentPending` with `scan_pk: recipient.scan_pk, spend_pk: recipient.spend_pk, k: <per-scan_pk counter>`. The `k` counter starts at 0 and increments per output to the same `scan_pk` — there is NO branch that promotes self-sends to `m=0` labeled.
- `sp_finish_branch` (`action_system/spends.rs:586-683`) consumes the pending entries and emits `derive_one_time_puzzle_hash(scan_pk, spend_pk, agg_sk, input_hash, k)` regardless of whether `scan_pk` belongs to the sender or a stranger.
- The labels.rs doc-comment (`crates/chia-sdk-utils/src/silent_payments/labels.rs:14-16`) explicitly says: *"`m = 0` is the change-label sentinel. It is REJECTED at the public boundary ([`SilentPaymentKeys::labeled_address`]) but accepted internally by [`generate_label`] and [`LabelRegistry::register`] because the scanner (Phase 6, SIM-03 sub-test) legitimately needs to register the change label to detect its own change outputs."*
- `keys.rs:116-131` confirms `labeled_address(m=0)` returns `Err(SilentPaymentError::ReservedChangeLabel)`.

**Conclusion: D-04's hidden assumption is FALSE.** The SDK does NOT auto-emit m=0 self-change outputs. The original D-04 test design (sender→sender unlabeled, expect `label: Some(0)`) would actually produce `label: None` and the assertion would fail.

**Recommended redesign (Pattern 3b above):**

The labels.rs doc-comment hints at the intended design — wallet authors use a self-managed `m=0` label registration in their own LabelRegistry to track change. The SIM-03 m=0 sub-test should:
1. Sender Alice sends to her OWN unlabeled address (or her own labeled `m=1` address).
2. Recipient Alice has a `LabelRegistry` that explicitly includes `m=0` (registered via internal-only `register(scan_sk, 0)`) PLUS any other labels she uses.
3. Scanner asserts: the detection is correctly attributed (`label: None` for unlabeled self, or `label: Some(1)` for labeled self) — i.e., the presence of `m=0` in the registry does NOT spuriously hijack unlabeled detections (per scanner.rs's `if !found` ordering at line 124).

**This redesign closes SC3 wording — "separate sub-test confirms a m=0 change-detection variant works internally"** — by demonstrating m=0 is internally consistent without being incorrectly elevated to a public auto-emit feature.

**Recommendation:** the planner MUST surface this finding in the test's rustdoc, AND consider updating ROADMAP.md SC3 text to be more precise about what "change-detection variant works internally" actually means. Suggested rewording: *"a separate sub-test confirms `LabelRegistry::register(scan_sk, 0)` is callable internally and does not produce spurious m=0 detections on unlabeled self-sends — demonstrating m=0 is reserved for wallet-author-managed change tracking, not auto-emitted by the SDK."* This is a wording clarification, not a scope change.

### 2. `tweak_data_from_simulator_block` return shape — Vec<TweakPoint> vs single tweak_point

Open: should the helper handle blocks with multiple disjoint SP transactions? The Phase 6 tests construct exactly one SP tx per block, so a single tweak_point is sufficient. But the helper's signature determines whether it's general-purpose or test-specific.

**Recommendation:** Implement as `TweakData { tweak_points: Vec<PublicKey>, outputs: Vec<OutputMeta> }` where `tweak_points.len() == number_of_distinct_SP_transactions_in_block`. For Phase 6 tests, this is always 1. For the example, also 1. Future CHIP-0058 transport clients may produce blocks with multiple SP transactions; the helper's shape supports that without API churn.

Currently Example 1 (above) emits ONE tweak_point per call — but with grouping by transaction within the block. Phase 6 doesn't need multi-tx-per-block; planner can land single-tx-per-block first and extend later if needed.

### 3. Should `tweak_data_from_simulator_block` be exposed on the bindings facade as a free function or a method?

D-02 mandates `Simulator.tweakDataFromBlock(height)` — method on Simulator. Confirmed in the binding facade at `crates/chia-sdk-bindings/src/simulator.rs:14-135`: every existing method is `&self`-with-Arc<Mutex<>>. Add as:

```rust
impl Simulator {
    pub fn tweak_data_from_block(&self, height: u32) -> Result<TweakData> {
        Ok(chia_sdk_test::silent_payments::tweak_data_from_simulator_block(
            &self.0.lock().unwrap(),
            height,
        ).into())
    }
}
```

Plus one new entry in `bindings/simulator.json`:
```json
"tweak_data_from_block": {
    "args": { "height": "u32" },
    "return": "TweakData"
}
```

**Recommended placement:** add after `create_block` (line 118 of simulator.json). No new chip-0057 sub-block needed because the bindings always have chip-0057 enabled per D-01/D-03.

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework (Rust) | `#[test]` + `rstest = 0.22.0` + `anyhow::Result` + `chia_sdk_test::Simulator` |
| Framework (napi) | AVA 7.0.0 (extension `.spec.ts`, `import test from "ava"`) |
| Framework (pyo3) | pytest (default discovery `tests/test_*.py`) |
| Framework (wasm) | AVA 6.4.1 + `wasm-pack build --target nodejs` |
| Config files | `napi/package.json` (ava + napi build), `wasm/package.json` (ava + wasm-pack), `pyo3/pyproject.toml` (maturin) — all already configured |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 -- silent_payments::e2e::` |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| Cross-language quick runs | `cd napi && pnpm test -- --match='*BIND-03*'`; `cd pyo3 && pytest tests/test_silent_payments.py`; `cd wasm && pnpm test -- --match='*BIND-03*'` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| SIM-01 | `tweak_data_from_simulator_block` returns correct `TweakData` for a known block | unit | `cargo test -p chia-sdk-test --features chip-0057 silent_payments::tweak_data::` | ❌ Wave 0 |
| SIM-02 | Unlabeled SP send → farm → extract → scan → spend round-trip (Rust) | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_unlabeled` | ❌ Wave 0 |
| SIM-03 (labeled) | Same as SIM-02 with `labeled_address(1)` and `label: Some(1)` detection | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_labeled` | ❌ Wave 0 |
| SIM-03 (m=0) | Self-send + LabelRegistry with m=0 registered does NOT spuriously detect m=0 on unlabeled | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_m0_self_change` | ❌ Wave 0 |
| BIND-03 (napi) | Unlabeled SP send round-trip through TS bindings | integration | `cd napi && pnpm test -- --match='*BIND-03*napi*'` | ❌ Wave 0 |
| BIND-03 (pyo3) | Unlabeled SP send round-trip through pyo3 bindings | integration | `cd pyo3 && pytest tests/test_silent_payments.py::test_unlabeled_e2e` | ❌ Wave 0 |
| BIND-03 (wasm) | Unlabeled SP send round-trip through wasm bindings | integration | `cd wasm && pnpm test -- --match='*BIND-03*wasm*'` | ❌ Wave 0 |
| EX-01 | `examples/silent_payment.rs` builds and runs cleanly | smoke | `cargo build --examples --all-features && cargo run --example silent_payment --all-features` | ❌ Wave 0 |
| chip-0057 cascade | `cargo build -p chia-sdk-test -F chip-0057` succeeds | smoke | `cargo build -p chia-sdk-test -F chip-0057` | ❌ Wave 0 (Plan 06-01) |
| descriptor↔facade drift | Phase 5's drift script still passes after adding `tweak_data_from_block` | smoke | `bash scripts/sp_descriptor_facade_drift.sh` | ✓ |
| workspace lints | No new `#[allow]` attributes; clippy clean | smoke | `cargo clippy --workspace --all-features --all-targets -- -D warnings` | ✓ |
| cargo machete | No new ignored entries | smoke | `cargo machete` | ✓ |

### Sampling Rate

- **Per task commit:** `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::` plus `cargo clippy -p <touched-crate> --all-features -- -D warnings`.
- **Per wave merge:** Full workspace test suite plus the three cross-language test runners (napi `pnpm test`, pyo3 `pytest`, wasm `pnpm test`).
- **Phase gate:** Full suite green via `cargo test --release --workspace --all-features` plus all three binding test suites plus `cargo build --examples --all-features` plus `bash scripts/sp_descriptor_facade_drift.sh` plus `cargo machete` before `/gsd:verify-work`.

### Wave 0 Gaps

- [ ] `crates/chia-sdk-test/Cargo.toml` — add `[features]` `chip-0057 = [...]` + add `chia-sdk-driver = { workspace = true, optional = true }` dep — Plan 06-01.
- [ ] `crates/chia-sdk-test/src/silent_payments/{mod.rs,tweak_data.rs}` — new directory + 2 files — Plan 06-02.
- [ ] `crates/chia-sdk-test/src/simulator.rs` — add `pub fn block_spends(height)` + `pub fn block_outputs(height)` accessors — Plan 06-02.
- [ ] `crates/chia-sdk-driver/src/silent_payments/e2e.rs` (or planner-chosen name) — 3 E2E tests + `setup_e2e()` helper — Plan 06-03.
- [ ] `crates/chia-sdk-bindings/Cargo.toml` — add `features = ["chip-0057"]` to chia-sdk-test dep — Plan 06-04.
- [ ] `crates/chia-sdk-bindings/src/simulator.rs` — add `tweak_data_from_block` method — Plan 06-04.
- [ ] `bindings/simulator.json` — add `tweak_data_from_block` entry — Plan 06-04.
- [ ] `napi/__test__/silent_payments_e2e.spec.ts` — new file — Plan 06-04.
- [ ] `pyo3/tests/test_silent_payments.py` — new file — Plan 06-04.
- [ ] `wasm/__test__/silent_payments.spec.ts` — new file — Plan 06-04.
- [ ] `examples/silent_payment.rs` — new file — Plan 06-05.
- [ ] `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-test -F chip-0057` line — Plan 06-01.
- [ ] Root `Cargo.toml` — add `"chia-sdk-test/chip-0057"` to the `chip-0057` workspace feature — Plan 06-01.
- [ ] `.planning/REQUIREMENTS.md` — mark SIM-01, SIM-02, SIM-03, BIND-03, EX-01 as `[x]` — Plan 06-05 closeout.

*(No existing test infrastructure gaps — Rust `#[test]`, AVA, pytest, and wasm-AVA are all pre-configured.)*

## Sources

### Primary (HIGH confidence)
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-test/Cargo.toml` — confirmed chia-sdk-test has NO chip-0057 feature.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-test/src/simulator.rs:1-447` — full Simulator API surface; no `block_spends(height)` accessor exists.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-test/src/simulator/data.rs` — confirmed `coin_states` and `coin_spends` are private (`pub(crate)`); no per-block grouping.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/send.rs` — verified SP send path; no m=0 auto-emit branch. Multiple silent_payment_tests demonstrate the new `Action::send(Id::Xch, SendDestination::SilentPayment(Box::new(addr)), amt, memos)` API.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/silent_payments/scanner.rs:73-162` — verified `scan_from_tweaks` signature, K_MAX_DEFAULT = 2400, identity-element skip, labeled k-termination rule.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/src/silent_payments/labels.rs:1-107` — confirmed the labels.rs doc explicitly mentions Phase 6 SIM-03 sub-test as registering m=0 internally; `LabelRegistry::register(scan_sk, 0)` works.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/src/silent_payments/keys.rs:116-131` — confirmed `labeled_address(0)` returns `Err(ReservedChangeLabel)`.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/spends.rs:586-683` — verified `sp_finish_branch` gate ordering: SilentPaymentRequiresInputBinding → SilentPaymentKeysNotRegistered → SilentPaymentMultiPartyUnsupported → SilentPaymentNoXchInputs.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/silent_payments.rs:376-395` — verified `SilentPayments::scan_from_tweaks` takes `labels: LabelRegistry` (not Option); facade passes `Some(&driver_labels)` always.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/simulator.rs:1-136` — verified Simulator binding pattern (`Arc<Mutex<>>`, Result-returning methods).
- `/home/kdc/chia-wallet-sdk/bindings/simulator.json` — verified existing methods + structure.
- `/home/kdc/chia-wallet-sdk/bindings/silent_payments.json` + `napi/index.d.ts:3020-3024` — verified `Vec<chia_bls::PublicKey>` marshals as `Array<PublicKey>` in TS.
- `/home/kdc/chia-wallet-sdk/napi/__test__/silent_payments.spec.ts` — Phase 5 AVA precedent; verified BIP-39 TV1 mnemonic + camelCase API surface.
- `/home/kdc/chia-wallet-sdk/napi/__test__/action_system.spec.ts:1-200` — confirmed `clvm.standardSpend(pk, clvm.delegatedSpend(conds))` pattern + `SendDestination.puzzleHash/.silentPayment` factory pattern.
- `/home/kdc/chia-wallet-sdk/Cargo.toml:73-80` — verified root `chip-0057` workspace feature cascades to types/driver/utils but NOT to chia-sdk-test.
- `/home/kdc/chia-wallet-sdk/.github/workflows/rust.yml:55-68` — verified chip-0057 CI build lines for types/driver/utils; no chia-sdk-test chip-0057 line.
- `/home/kdc/chia-wallet-sdk/src/prelude.rs:34-45` — verified umbrella prelude re-exports every SP type the example needs.

### Secondary (MEDIUM confidence)
- Phase 5 PHASE-SUMMARY lessons #5 & #6 — `Vec<(K,V)>` tuple-marshaling failure pattern; the wrapper-class workaround. Lesson #7 — bindy's `&self.0.method` dispatch contract.
- Phase 4 Plan 04-03 commit history — `multi_party_hard_errors`, `round_trip_matches_derive_one_time_puzzle_hash` test patterns; setup helper precedent (sender + recipient via `SecretKey::from_bytes`).

### Tertiary (LOW confidence)
- None. All findings confirmed against the codebase.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages already in workspace, all versions pinned in Cargo.toml.
- Architecture patterns: HIGH — every pattern confirmed against existing Phase 1-5 code.
- m=0 redesign (Open Q1): HIGH — false assumption disproven by inspecting send.rs + sp_finish_branch; redesign aligned with labels.rs doc.
- Cross-language test mechanics: HIGH — `napi/__test__/silent_payments.spec.ts` and `action_system.spec.ts` provide direct precedents.
- Example structure: HIGH — `cat_spends.rs`, `spend_simulator.rs` mirror the rhythm exactly.
- `tweak_data_from_simulator_block` algorithm: MEDIUM-HIGH — CHIP-0057 §10b/§425/§446/§459 plus Phase 4's `compute_input_hash` plus direct `PublicKey` summation (`&pk_a + &pk_b`) — Phase 4 only exposes `aggregate_sender_sks` for SecretKey aggregation; PK aggregation is done in-place via the BLS `Add` impl.
- bindings/simulator.json entry shape: HIGH — `tweak_data_from_block` slots into the existing Simulator pattern with one entry.

**Research date:** 2026-05-18
**Valid until:** 2026-06-17 (30 days; stable phase — no fast-moving APIs)
