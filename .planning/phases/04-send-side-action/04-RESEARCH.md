# Phase 4: Send-side Action — Research

**Researched:** 2026-05-15
**Domain:** Wallet-side silent-payment sender — `SilentPaymentSend` `Action` variant + `Spends` integration + opcode 60/61 cross-input announcement binding + multi-output `k` counter + memo-position hint guard
**Confidence:** HIGH on every recommendation below; each is anchored to a direct read of the SDK source (action system, `SendAction`, `SettleAction`, `FungibleSpends`, `Spends::finish_with_keys`, `StandardLayer`, `condition.rs` opcodes 60/61) and to the `~/silent-payments` reference (`sp-common/src/{protocol,ecdh,puzzle}.rs` + `send_payment.py` for the announcement-binding shape). Open uncertainties are flagged inline and treated as MEDIUM/LOW, never elevated.

## Summary

Phase 4 turns the Phase 3 protocol primitives (`derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`, `compute_shared_secret_from_tweak`) into a wallet-author-callable `Action::SilentPaymentSend(SilentPaymentSend)` variant, adds three new free functions (`derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`), wires per-batch state into `Spends` (a `k`-counter map + a deferred-ECDH pending list), introduces a new `Spends::finish_with_silent_payment_keys(..., synthetic_pks, synthetic_sks)` overload, and emits CHIP-mandated coin-announcement bindings (opcode 60 on the lex-smallest coin id, opcode 61 on every other input) when there are ≥ 2 XCH inputs.

The driver-side surface lands in `crates/chia-sdk-driver/src/silent_payments/` (new files: `aggregate.rs`, `input_hash.rs`, `one_time.rs`, `send_keys.rs`) plus `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (the action) plus a one-variant extension of `Action` in `action_system/action.rs` plus three new `DriverError` variants (`SilentPaymentMultiPartyUnsupported`, `SilentPaymentMemoHintForbidden`, `SilentPaymentNoXchInputs`). All gated under `chip-0057`. Total estimated surface: ~700 lines of source + ~600 lines of tests; one new CI line is **not** required (the Phase 3 line `cargo build -p chia-sdk-driver -F chip-0057` already covers Phase 4 additions).

**Primary recommendation:** Land `SilentPaymentSend` as an `Action` enum variant (precedent: `SendAction`, `SettleAction`, `FeeAction`), implement `SpendAction` directly on it (no closures, no builder), defer the ECDH math to `Spends::finish_with_silent_payment_keys(...)` (Option A from the open architectural questions — Pitfalls research §2 + cross-cutting concern #2), pin per-batch state on `Spends` (two new fields: `silent_payment_counters: HashMap<[u8; 48], u32>` and `silent_payments_pending: Vec<SilentPaymentPending>`), bind cross-input groups via opcode-60 from the lex-min coin and opcode-61 from the rest with `message = b""` per the Python reference (`~/silent-payments/send_payment.py:317-323`), and reject 32-byte first memos with the new error variant (Option A from §7 below — explicit hard-error). All eight SEND-* requirements are addressable in 4–5 sequential plans (see §10 plan slicing).

<user_constraints>
## User Constraints

**There is no CONTEXT.md for Phase 4.** No `/gsd:discuss-phase` has been run yet for this phase. Constraints therefore come from `./CLAUDE.md`, `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md`, and the three cross-phase research docs (`.planning/research/{ARCHITECTURE,PITFALLS,FEATURES,STACK,SUMMARY}.md`). If a `/gsd:discuss-phase` is run before this planner consumes the research, copy the result into this section verbatim and treat anything below as superseded by locked decisions.

### Locked Decisions (from PROJECT.md, REQUIREMENTS.md, ROADMAP.md, STATE.md)

- **Feature flag:** `chip-0057` is the only umbrella. No `silent-payments`, no name-aliases, no sub-features.
- **No new workspace dependencies.** Phase 4 uses what's already declared: `chia-bls = 0.36.1`, `chia-protocol = 0.36.1`, `chia-puzzle-types = 0.36.1`, `chia-sdk-types/conditions`, `chia-sha2`, `num-bigint`, `hex`, `hex-literal`, `thiserror`, `indexmap`. (Bumping `chia-protocol` or `chia-puzzles` is permanently out of scope.)
- **No new top-level crate.** Phase 4 code lives in `chia-sdk-driver/` behind `chip-0057`, mirroring how `chip-0035` (vault/datalayer), `chip-0037` (EIP-712/controller), `action-layer`, and Phases 1–3 already slot into the driver crate.
- **`ScalarField` is the only mod-r reducer.** Phase 1 grep ban (`! grep -r 'mod_by_group_order' silent_payments/`) extends to every file Phase 4 ships. Aggregation, input-hash, and tweak math all flow through `chia_sdk_types::silent_payments::ScalarField::from_bytes_unsigned` (for hashed outputs) or `::from_bytes_raw` (for in-range SK bytes).
- **`chia-sha2`, never bare `sha2`.** Phase 1 defense-in-depth grep ban (`! grep -rE '^use sha2::' silent_payments/`) holds. The reference impl at `~/silent-payments/crates/sp-common/src/ecdh.rs:4` uses `sha2::{Sha256, Digest}` — this is the line that must change when porting.
- **Workspace lint policy (WS-03):** every chip-0057-gated file passes `deny clippy::all`, `warn pedantic`, `warn cargo`, `deny unsafe_code`, `deny dead_code`, plus `cargo machete` with zero new ignored entries.
- **`SilentPaymentSend` composes through the action system, not as a free helper.** Per PITFALLS §12 (Pitfall 12 — CAT2 footgun): a bare `derive_one_time_puzzle_hash` function would let wallets accidentally wrap a one-time puzzle hash in a `CatLayer`, producing on-chain coins the recipient cannot detect. The mitigation is to ship the math behind the `SilentPaymentSend` action, not as a freely-composable helper. `derive_one_time_puzzle_hash` IS exposed (SEND-01 requires it) — but it carries the Privacy-warning docstring and the action is the primary path.
- **Multi-party hard-error in v1.** Per PITFALLS §3 (Pitfall 3 — multi-party silent fallback): when the local wallet does not hold every input's synthetic SK, `finish_with_silent_payment_keys` MUST hard-error with a typed variant. Silent single-input aggregation is the failure mode that breaks offers and PSBT-style flows. v2 will add a multi-party aggregator; v1 refuses.
- **Memo-position hint guard.** Per PITFALLS §"Memo-position guard" + ROADMAP success criterion #5: a 32-byte first memo gets promoted to a `puzzle_hash` hint by the standard Chia wallet, which exposes the one-time puzzle hash to every indexer and defeats the silent-payment privacy gain. The SDK MUST either reject or rewrite this memo shape. See §7 for the decision.
- **Privacy doc-comments on every memo-bearing API.** SEND-08 is doc-only; the canonical wording is in §8.
- **Synthetic-vs-raw key boundary.** Per PITFALLS §2 (Pitfall 2): `aggregate_sender_sks` consumes synthetic SKs (the ones whose PKs are curried into `StandardArgs`). The function signature lives in `chia-sdk-driver`; the doc-comment makes the synthetic requirement explicit. Open Q2 (`SyntheticSecretKey` newtype vs. documented `&[SecretKey]`) is resolved at this research's recommendation (§4) toward Option B (documented `&[SecretKey]`) because the chia-bls `SecretKey` type is shared with all other SDK signing flows and a parallel newtype would create a viral API change; Option A is still acceptable if the planner wants compile-time safety.
- **Opcode 60/61 announcement binding is mandatory for ≥ 2 input scenarios.** Per ROADMAP success criterion #4 + REQUIREMENTS SEND-06 + CHIP-0057 §319-327 (Pass 2b scanner grouping): without this, the recipient's scanner can only detect single-input or same-derivation-index sends. Cross-index sends are undetectable without announcement linkage. The Python reference at `~/silent-payments/send_payment.py:307-323` is canonical.
- **Multi-output `Vec<Recipient>` shape with per-scan_pk `k` counter on `Spends`.** Per REQUIREMENTS SEND-05 + FEATURES.md "Multi-output sends to the same recipient": `SilentPaymentSend` accepts a single `(recipient, amount, memos)` triple per action (composable in `Vec<Action>` for multi-recipient batches), and `Spends` owns the per-scan_pk `k` counter so multiple sends in one batch increment correctly. See §6 for the shared-state location.
- **No `derive_synthetic` inside silent-payments output construction.** Per PROJECT.md "Out of Scope (permanently)": "Replacing the existing `StandardArgs::curry_tree_hash` + `derive_synthetic` flow with a hand-rolled `puzzle_hash_for_pk` — re-use what the SDK already has." Phase 3 already lands `puzzle_hash_for_pk(pk) = StandardArgs::curry_tree_hash(pk.derive_synthetic())` in `chia-sdk-driver/src/silent_payments/protocol.rs`. Phase 4's `derive_one_time_puzzle_hash` composes this — no new puzzle-hash math.

### Claude's Discretion (planner can choose)

- **`SilentPaymentSend` field shape.** Recommendation in §3: `{ recipient: SilentPaymentAddress, amount: u64, memos: Memos<NodePtr> }`. Alternatives: replace `recipient` with a `Recipient { scan_pk, spend_pk, label: Option<u32> }` struct, or add a `label: Option<u32>` field (for labeled sends — the recipient's labeled spend_pk is already in the address payload so this is informational only). The recommended shape mirrors `SendAction`'s 4-field shape (`id, puzzle_hash, amount, memos`) — the cleanest mental mapping for wallet authors.
- **`derive_one_time_puzzle_hash` exposure level.** Phase 5 will re-export it in `bindings/silent_payments.json`. Phase 4 ships it as `pub fn` in `chia-sdk-driver/src/silent_payments/one_time.rs`; the prelude in `src/prelude.rs` already lists it (Phase 3 PLAN 03-05 added the re-export). Planner can decide whether to also expose `compute_input_hash` and `aggregate_sender_sks` in the prelude — recommendation: yes for `compute_input_hash`, no for `aggregate_sender_sks` (the latter is too easy to misuse per PITFALLS §3).
- **`SilentPaymentPending` field layout.** §3 below recommends `{ scan_pk, spend_pk, parent_xch_index: usize, k: u32, amount: u64, memos: Memos<NodePtr> }`. The planner may collapse this with `SilentPaymentSend` if it stays under 200 lines combined.
- **Whether `finish_with_silent_payment_keys` is a separate method or an enriched `finish_with_keys`.** Recommendation: separate method that adds two parameters (`synthetic_pks: &IndexMap<Bytes32, PublicKey>`, `synthetic_sks: &IndexMap<Bytes32, SecretKey>`) on top of `finish_with_keys`'s existing `synthetic_keys`. The `synthetic_pks` parameter is kept distinct from `synthetic_keys` even though they will usually be the same map — see §4 for the data-flow rationale.
- **Test-fixture mnemonic.** Phase 3 uses `TV1_MNEMONIC = "abandon abandon ... about"` (BIP-39 zero entropy). Phase 4 can reuse this directly (`SilentPaymentKeys::from_mnemonic(&TV1_MNEMONIC)`) or use `BlsPair::new(seed)` for the sender wallet. Recommendation: use `BlsPair::new(0..N)` for sender XCH coins (consistent with `actions/send.rs:114`) and `SilentPaymentKeys` for the recipient (whose scan/spend keys must be CHIP-derived for round-trip correctness against Phase 3's `scan_from_tweaks`).
- **Whether to gate `DriverError::SilentPaymentMultiPartyUnsupported` etc. behind `#[cfg(feature = "chip-0057")]`.** Recommendation: yes, mirror the existing `DriverError::SilentPayment(...)` variant from Phase 3 (`crates/chia-sdk-driver/src/driver_error.rs:134-136`). Keeps the no-feature build's `DriverError` enum minimal.

### Deferred Ideas (OUT OF SCOPE for Phase 4)

- **`bindings/silent_payments.json` descriptor.** Phase 5 (BIND-01, BIND-02). Phase 4 designs `derive_one_time_puzzle_hash` / `compute_input_hash` / `aggregate_sender_sks` with the binding constraint in mind ("Bytes32 + Vec<PublicKey> + Vec<SecretKey> + u32") but does not author the JSON.
- **napi/pyo3/wasm builds.** Phase 5 (BIND-02). Phase 4 only verifies `cargo build -p chia-sdk-driver -F chip-0057` and the workspace `--all-features` build.
- **Simulator round-trip test.** Phase 6 (SIM-02). Phase 4's tests are unit-level: hand-built `Spends` scenarios that compose actions, finish, inspect the `CoinSpend` outputs, and assert byte-for-byte equality between the action's emitted puzzle hash and `derive_one_time_puzzle_hash`. The Simulator-farms-a-block flow is Phase 6.
- **Multi-party silent-payment send.** Permanently deferred to v2 per REQUIREMENTS "v2 Requirements (deferred)" + PITFALLS §3. v1 hard-errors via `DriverError::SilentPaymentMultiPartyUnsupported`.
- **CAT2 silent-payment send.** Permanently deferred to v2 per PROJECT.md Key Decisions + PITFALLS §12. v1 only handles XCH inputs and XCH outputs.
- **Offer-settlement composition with silent-payment.** Permanently deferred to v2 per PROJECT.md + PITFALLS §"Anti-Pattern 4". v1 explicitly rejects an offer-settlement spend coexisting in the same `Spends` as a `SilentPaymentSend` (the multi-party hard-error subsumes this — a settlement input is by definition a counterparty input).
- **Sage / wallet UI integration.** Out of scope for the SDK.
- **CHIP-0058 transport client.** v2 deferred.

## Project Constraints (from CLAUDE.md)

Actionable directives the planner must honor:

1. **Rust 1.90.0, edition 2024** (`rust-toolchain.toml`). No nightly features.
2. **`unsafe_code = "deny"`.** No `unsafe` blocks anywhere in Phase 4 source.
3. **Workspace clippy:** `deny clippy::all` + `warn pedantic` + `warn cargo`. Local strict gate: `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`. Inline fixes only — no `#[allow(...)]` attributes anywhere in `silent_payments/` or `actions/silent_payment_send.rs` (Phase 3 had ONE function-scoped `#[allow(clippy::similar_names)]` for the `spend_sk`/`spend_pk` parameter pair — that exception is precedent but should not be replicated unless equally unavoidable).
4. **`cargo machete` clean.** Zero new `[package.metadata.cargo-machete] ignored` entries on `chia-sdk-driver`.
5. **`cargo fmt --check`** clean.
6. **`[lints] workspace = true`** in `chia-sdk-driver/Cargo.toml` (line 18 — already set). Do not add a per-crate override.
7. **Every dep is `{ workspace = true }`**, never a literal version. All required deps already listed in `chia-sdk-driver/Cargo.toml`.
8. **`chia-sha2`, never bare `sha2`.** Phase 1 grep ban. The reference impl violates this — these are the lines that change when porting.
9. **No `From<[u8;32]> for ScalarField`.** Already enforced by Phase 1.
10. **No `mod_by_group_order` literal** anywhere in `chip-0057`-gated code. Phase 1 grep ban.
11. **GSD workflow enforcement** — all edits go through a GSD command. Phase 4 is invoked via `/gsd:plan-phase` → spawns gsd-planner (which consumes this research).
12. **LSP-first navigation.** Prefer `workspaceSymbol` / `findReferences` over `grep` when locating SDK types and call sites during planning.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **SEND-01** | `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) -> Bytes32` computes the recipient's per-payment puzzle hash; produces a standard p2 puzzle hash on the tweaked one-time public key. | §3 (signature), §6 (data flow — composes Phase 3's `compute_shared_secret_from_tweak` + `derive_output_tweak` + `derive_onetime_pk` + `puzzle_hash_for_pk`). Pure function; no `Spends` touch. |
| **SEND-02** | `compute_input_hash(coin_ids, sender_pk_aggregated) -> ScalarField` uses the lexicographically smallest spent coin ID and aggregated synthetic sender PK, per `tagged_hash("Chia_SP/Inputs", coin_id_min || serialize(A_sum))`. | §5 (input-hash binding section); algorithm verbatim from `~/silent-payments/crates/sp-common/src/ecdh.rs:37-45`. TV1 pinned value `38a1c837...cc9411`. |
| **SEND-03** | `aggregate_sender_sks(sks)` aggregates synthetic secret keys across all wallet-controlled inputs of a single transaction. Returns `ScalarField`. Hard-errors (no silent fallback) when partial control is detected. | §4 (sender-keys section). The hard-error path falls out of `Spends::finish_with_silent_payment_keys` invariants — if `synthetic_sks` does not cover every XCH input's `p2_puzzle_hash`, the finish returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)`. The free function `aggregate_sender_sks(&[SecretKey])` itself never errors — it just sums; the error fires at the `Spends` boundary where partial control is detectable. |
| **SEND-04** | `SilentPaymentSend` action composes with `Spends`. `spends.add(SilentPaymentSend { recipient, amount, memos })` records the deterministic pieces during `apply()` and defers ECDH to `Spends::finish_with_silent_payment_keys(...)`. | §1 (action anatomy), §3 (struct shape), §4 (Option A architecture). Defers ECDH per ARCHITECTURE.md Open Q1 Option A — confirmed by reading `Spends::finish_with_keys` (`crates/chia-sdk-driver/src/action_system/spends.rs:460-495`). |
| **SEND-05** | `SilentPaymentSend` participates in multi-output sends: a single transaction can emit multiple `SilentPaymentSend` actions sharing one `input_hash` with a per-recipient `k` counter. | §6 (multi-output coordination). The counter lives on `Spends` (new field `silent_payment_counters: HashMap<[u8; 48], u32>`); each action reads-and-increments. |
| **SEND-06** | Multi-input sends across different wallet key indices emit opcode 60 (`CREATE_COIN_ANNOUNCEMENT`) on one input and opcode 61 (`ASSERT_COIN_ANNOUNCEMENT`) on the others, so the recipient's Pass 2b scanner can reconstruct the group. | §5 (announcement binding section). Algorithm: opcode-60 with `message = b""` on the lex-min `coin_id` input; opcode-61 with `announcement_id = SHA256(coin_id_min || "")` on every other XCH input. Mirrors `~/silent-payments/send_payment.py:307-323` exactly. |
| **SEND-07** | Memo-position hint guard: 32-byte first memos are rejected or rewritten. The hazard is hard to hit by accident. | §7 (memo-hint guard section). Recommendation: Option A (explicit hard-error `DriverError::SilentPaymentMemoHintForbidden`). Test plan: pass a `Memos::Some([0xff; 32])` to `SilentPaymentSend::new` and assert the error fires at `apply()` time. |
| **SEND-08** | Doc-comment privacy warnings on every public memo-bearing API. | §8 (privacy doc-comments section). Canonical wording pinned verbatim; every `pub fn`, `pub struct`, `pub enum` variant in `silent_payments/` or `actions/silent_payment_send.rs` whose surface includes a `Memos` parameter or field carries it. |
</phase_requirements>

## 1. Action system anatomy

The `SpendAction` trait is the entry point for everything in `Action`. Phase 4 adds one new variant to that enum.

### 1a. The trait surface (canonical)

From `crates/chia-sdk-driver/src/action_system/action.rs:227-236` (read end-to-end):

```rust
pub trait SpendAction {
    fn calculate_delta(&self, deltas: &mut Deltas, index: usize);

    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        index: usize,
    ) -> Result<(), DriverError>;
}
```

Both methods are non-async, non-fallible-by-design (`calculate_delta` returns `()`; `spend` returns `Result<(), DriverError>`). The `Action` enum implements `SpendAction` itself (lines 238-275) by match-dispatching to each variant. Every existing variant (`Send`, `Settle`, `CreateDid`, `UpdateDid`, `MintNft`, `UpdateNft`, `IssueCat`, `RunTail`, `MintOption`, `MeltSingleton`, `Fee`) is a struct that implements `SpendAction`.

### 1b. The action lifecycle in detail

From `crates/chia-sdk-driver/src/action_system/spends.rs:82-92` (`Spends::apply`):

```rust
pub fn apply(
    &mut self,
    ctx: &mut SpendContext,
    actions: &[Action],
) -> Result<Deltas, DriverError> {
    let deltas = Deltas::from_actions(actions);  // calls calculate_delta on each
    for (index, action) in actions.iter().enumerate() {
        action.spend(ctx, self, index)?;          // then calls spend on each
    }
    Ok(deltas)
}
```

`Deltas::from_actions` (`deltas.rs:19-25`) iterates first to compute net XCH/CAT/DID/NFT/Option deltas (input vs output), then `apply()` iterates again to call each `spend()` which mutates `Spends`. The deltas drive `Spends::create_change` later (`spends.rs:94-142`); `spend` is where each action writes its `CreateCoin` (or its conditions, or its payment) into the appropriate XCH/CAT/singleton spend slot.

`Spends::finish_with_keys` (`spends.rs:460-495`) then:
1. Calls `self.prepare(ctx, deltas, relation)` to materialize change + emit collected conditions + emit `Relation` (concurrent-spend) links.
2. Iterates `spends.unspent()` (`spends.rs:499-531`) and, for each `SpendableAsset + SpendKind`:
   - **Conditions kind:** look up `synthetic_keys[asset.p2_puzzle_hash()]` → `StandardLayer::new(synthetic_key).spend_with_conditions(ctx, spend.finish())` → record the `Spend` in `coin_spends: HashMap<Bytes32, Spend>`.
   - **Settlement kind:** `SettlementLayer.construct_spend(ctx, SettlementPaymentsSolution::new(spend.finish()))`.
3. Calls `spends.spend(ctx, coin_spends)` (line 494) which iterates every XCH/CAT/singleton item and calls `ctx.spend(coin, spend)` → producing the final `CoinSpend` list inside `SpendContext::coin_spends`.
4. Returns `Outputs`.

**Critical observation:** `finish_with_keys` consumes `self` (the `Spends`) and returns `Outputs`. The action-time mutation (during `apply`) writes `CreateCoin` conditions into `spends.xch.items[i].kind` (a `SpendKind::Conditions(ConditionsSpend)` — see `spends/spend_kind.rs:43-71` and `conditions_spend.rs:18-26`). At finish time, those conditions get unwrapped and signed via `StandardLayer`.

### 1c. How `SendAction::spend` shapes the precedent

From `crates/chia-sdk-driver/src/actions/send.rs:28-94` (read end-to-end):

```rust
impl SpendAction for SendAction {
    fn calculate_delta(&self, deltas: &mut Deltas, _index: usize) {
        deltas.update(self.id).output += self.amount;
        deltas.set_needed(self.id);
    }

    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        _index: usize,
    ) -> Result<(), DriverError> {
        let output = Output::new(self.puzzle_hash, self.amount);
        let create_coin = CreateCoin::new(self.puzzle_hash, self.amount, self.memos);

        if matches!(self.id, Id::Xch) {
            let source = spends.xch.output_source(ctx, &output)?;
            let parent = &mut spends.xch.items[source];
            let parent_puzzle_hash = parent.asset.full_puzzle_hash();

            parent.kind.create_coin_with_assertion(
                ctx,
                parent_puzzle_hash,
                &mut spends.xch.payment_assertions,
                create_coin,
            );

            let coin = Coin::new(
                parent.asset.coin_id(),
                create_coin.puzzle_hash,
                create_coin.amount,
            );

            spends.outputs.xch.push(coin);
        } /* else for cats/dids/nfts/options/burn */
        Ok(())
    }
}
```

**Three things to copy in `SilentPaymentSend`:**
1. `Output::new(puzzle_hash, amount)` + `spends.xch.output_source(ctx, &output)?` to pick a parent XCH coin.
2. `parent.kind.create_coin_with_assertion(...)` to record the `CreateCoin` condition on that parent.
3. `spends.outputs.xch.push(Coin::new(parent_coin_id, ph, amount))` to record the resulting output for the caller.

**One thing to change:** the `create_coin` puzzle hash is NOT known at `apply()` time — it requires `aggregated_sender_sk` which arrives only at finish time. The puzzle hash is computed lazily; see §4.

### 1d. How `FeeAction` shapes the no-output precedent

From `crates/chia-sdk-driver/src/actions/fee.rs:18-37` (read end-to-end): `FeeAction` doesn't touch any `xch.items[i].kind` at all — it only writes `spends.outputs.fee` and `spends.outputs.reserved_fee`. The `reserve_fee` condition is emitted later by `Spends::emit_conditions` (`spends.rs:244-246`) onto whichever XCH item gets selected as the intermediate-conditions source. This is the precedent for **emitting conditions at finish time, not apply time** — useful for the opcode-60/61 announcement binding (§5).

### 1e. The two-step state machine

| Phase | Method | What happens | What's known | What's missing |
|-------|--------|-------------|---------------|----------------|
| Apply | `SpendAction::spend(ctx, &mut spends, index)` | Choose parent XCH item; emit deterministic `CreateCoin` and record output coin. | `scan_pk`, `spend_pk`, recipient amount, sender XCH inputs (just-added coins), `k` (from Spends counter), `memos` | `aggregated_sender_sk` (needed for shared_secret → puzzle_hash) |
| Finish | `Spends::finish_with_silent_payment_keys(ctx, deltas, relation, synthetic_pks, synthetic_sks)` | Compute aggregated_sender_sk = Σ synthetic_sks_for_xch_inputs; compute input_hash; for each pending entry, derive shared_secret, t_k, one_time_pk, one_time_puzzle_hash; **mutate** the placeholder `CreateCoin` puzzle hash; emit opcode 60/61 announcement bindings; then call the standard finish path. | Everything | — |

This split is the architectural decision deferred to Phase 4 (ARCHITECTURE.md Open Q1). Option A wins because the alternative (Option B — accept aggregated_sender_sk at action construction) forces the caller to know all XCH inputs upfront, contradicting the `Spends` builder pattern.

## 2. Where `SilentPaymentSend` lives

**Decision: `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (NOT `silent_payments/actions/`).**

Rationale:

- **Precedent 1: `SendAction` is in `actions/send.rs`** (verified by `ls /home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/` → `create_did.rs`, `fee.rs`, `issue_cat.rs`, `melt_singleton.rs`, `mint_nft.rs`, `mint_option.rs`, `run_tail.rs`, `send.rs`, `settle.rs`, `update_did.rs`, `update_nft.rs`). Every `SpendAction` impl lives flat under `actions/`, one file per action.
- **Precedent 2: ARCHITECTURE.md §"Recommended Module Layout"** (`.planning/research/ARCHITECTURE.md:107-111`):
  ```
  └── chia-sdk-driver/src/
      ├── silent_payments/                        # chip-0057 (NEW module)
      │   ├── mod.rs                              # barrel
      │   ├── ecdh.rs                             # ECDH + input_hash
      │   ├── protocol.rs                         # output_tweak, onetime PK/SK, scan_from_tweaks
      │   ├── aggregate.rs                        # aggregate_sender_sks
      │   ├── labels.rs                           # generate_label, LabelRegistry
      │   └── tweak_data.rs                       # TweakData, DetectedSpCoin, OutputMeta
      ├── actions/
      │   └── silent_payment_send.rs              # SilentPaymentSend action (chip-0057)
      └── action_system/
          └── action.rs                           # Action::SilentPaymentSend variant (chip-0057)
  ```
  Architecture research already settled this. The split is "crypto + transport-agnostic primitives in `silent_payments/`; Spends-integration action in `actions/`."
- **Companion files in `silent_payments/`:** Phase 4 adds four new files to `crates/chia-sdk-driver/src/silent_payments/`:
  - `input_hash.rs` — `compute_input_hash(coin_ids: &[Bytes32], aggregated_sender_pk: &PublicKey) -> ScalarField` (SEND-02)
  - `aggregate.rs` — `aggregate_sender_sks(sks: &[SecretKey]) -> ScalarField` (SEND-03)
  - `one_time.rs` — `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) -> Bytes32` (SEND-01)
  - `send_keys.rs` — `Spends::finish_with_silent_payment_keys` extension method + `SilentPaymentPending` struct (SEND-04, SEND-05, SEND-06)

The `actions/silent_payment_send.rs` file holds the action struct and its `SpendAction` impl; everything else lives in `silent_payments/`.

**Module-system gate:** `crates/chia-sdk-driver/src/actions.rs` (the barrel for `actions/`) needs a `#[cfg(feature = "chip-0057")] mod silent_payment_send;` and matching `pub use`. The `action_system::action::Action` enum gets one new variant; the `silent_payments` barrel grows by 4 modules.

## 3. API shape options for `SilentPaymentSend`

Three options considered:

| Option | Surface | Ergonomics — single recipient | Ergonomics — multi-recipient | Ergonomics — mixed batches | Bindings impact | Verdict |
|--------|---------|-------------------------------|------------------------------|----------------------------|-----------------|---------|
| **A: struct + builder + `spends.add(...)`** | `SilentPaymentSend::new(addr, amount, memos)` becomes an `Action` variant; caller writes `Action::silent_payment_send(addr, amount, memos)` and passes it to `spends.apply(&[...])` | Clean — one line | Clean — `vec![Action::silent_payment_send(a, 1, m), Action::silent_payment_send(b, 2, m)]`. Spends-level counter handles k-increment | Clean — mixes with `Action::send`, `Action::fee`, etc. trivially | Direct `Action::silent_payment_send` constructor in `bindings/action_system.json` — one factory entry | **RECOMMENDED** |
| **B: method on Spends (`spends.send_silent_payment(...)`)** | `spends.send_silent_payment(addr, amount, memos)` directly | Clean | Clean | **Bad** — caller has to mix `spends.add_action(...)` and `spends.send_silent_payment(...)`, two different APIs for the same logical operation | New method-on-class binding; doesn't fit the existing `bindings/action_system.json` Action enum | Rejected |
| **C: method on SilentPayment context object** | `SilentPaymentBatch::new().send(a, 1, m).send(b, 2, m).into_action()` | Verbose for single | Cleaner for multi (chained) | Bad — caller has to construct the batch separately before adding to Spends | Yet another carrier type in bindings | Rejected |

**Recommended struct shape:**

```rust
// crates/chia-sdk-driver/src/actions/silent_payment_send.rs

use chia_puzzle_types::Memos;
use clvmr::NodePtr;

use chia_sdk_utils::silent_payments::SilentPaymentAddress;

#[derive(Debug, Clone)]
pub struct SilentPaymentSend {
    /// The recipient's silent-payment address (already-parsed `scan_pk` +
    /// `spend_pk` + network). Use [`SilentPaymentAddress::decode`] at the
    /// caller's edge to parse a bech32m `spxch1...` / `tspxch1...` string.
    pub recipient: SilentPaymentAddress,
    pub amount: u64,
    /// Privacy warning: memos are stored on-chain in plaintext and are visible
    /// to anyone holding the recipient's scan key. Do not include sensitive
    /// data. The first memo position is also a `puzzle_hash` hint slot — passing
    /// a 32-byte first memo will be rejected by `SilentPaymentSend::spend`
    /// (returns `DriverError::SilentPaymentMemoHintForbidden`).
    pub memos: Memos<NodePtr>,
}

impl SilentPaymentSend {
    pub fn new(recipient: SilentPaymentAddress, amount: u64, memos: Memos<NodePtr>) -> Self {
        Self { recipient, amount, memos }
    }
}

impl SpendAction for SilentPaymentSend {
    fn calculate_delta(&self, deltas: &mut Deltas, _index: usize) {
        deltas.update(Id::Xch).output += self.amount;
        deltas.set_needed(Id::Xch);
    }

    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        _index: usize,
    ) -> Result<(), DriverError> {
        // 1. Memo-hint guard (§7).
        memo_hint_guard(ctx, &self.memos)?;

        // 2. Pick a placeholder puzzle hash (DEFAULT or BURN_PUZZLE_HASH) so
        //    `output_source` can find an XCH parent and reserve the amount.
        //    The placeholder is REPLACED at finish time.
        let placeholder = Bytes32::default();
        let output = Output::new(placeholder, self.amount);
        let source = spends.xch.output_source(ctx, &output)?;
        let parent_coin_id = spends.xch.items[source].asset.coin_id();
        let parent_puzzle_hash = spends.xch.items[source].asset.full_puzzle_hash();

        // 3. Increment k counter for this recipient (§6).
        let scan_pk_bytes: [u8; 48] = self.recipient.scan_pk.to_bytes();
        let k = *spends
            .silent_payment_counters
            .entry(scan_pk_bytes)
            .and_modify(|n| *n += 1)
            .or_insert(0);

        // 4. Record deterministic pieces; ECDH and the actual CreateCoin
        //    emission happen at finish time (§4).
        spends.silent_payments_pending.push(SilentPaymentPending {
            scan_pk: self.recipient.scan_pk,
            spend_pk: self.recipient.spend_pk,
            parent_xch_index: source,
            parent_coin_id,
            parent_puzzle_hash,
            k,
            amount: self.amount,
            memos: self.memos,
        });

        Ok(())
    }
}
```

Two important caveats baked into the above:

1. **Placeholder puzzle hash:** the action records the parent + amount + k + memos, but **does NOT emit a `CreateCoin` condition at apply time**. Reason: the puzzle hash depends on `aggregated_sender_sk`, which arrives only at finish. The `CreateCoin` is emitted in `finish_with_silent_payment_keys`. This is a divergence from `SendAction::spend` which emits the `CreateCoin` inline. (`spends.outputs.xch.push(...)` is also deferred — see ARCHITECTURE.md "Anti-Pattern 5" which warns against pre-deriving on-chain coin ids.)

2. **`output_source` still needs to be called at apply time** because we need to reserve an XCH parent's amount slot. The placeholder `Bytes32::default()` works because `Output::new(ph, amount)` only uses `amount` for the source-selection arithmetic (`FungibleSpends::output_source` at `fungible_spends.rs:37-51`). The `ph` field is used for `is_allowed` checks (`OutputSet::is_allowed`), which for standard `ConditionsSpend` is always permissive (`conditions_spend.rs:33-50` — output equality is checked only when finalizing). Verify this empirically in Plan 04-02; if the placeholder PH causes spurious conflicts, fall back to `BURN_PUZZLE_HASH` (which is `hex!("...dead")` in `action.rs:15-17`) which is known-safe in the action system.

**Action enum extension:**

```rust
// crates/chia-sdk-driver/src/action_system/action.rs (extension)

#[derive(Debug, Clone)]
pub enum Action {
    Send(SendAction),
    Settle(SettleAction),
    CreateDid(CreateDidAction),
    /* ... existing ... */
    Fee(FeeAction),

    #[cfg(feature = "chip-0057")]
    SilentPaymentSend(SilentPaymentSend),
}

impl Action {
    /* ... existing constructors ... */

    #[cfg(feature = "chip-0057")]
    pub fn silent_payment_send(
        recipient: SilentPaymentAddress,
        amount: u64,
        memos: Memos<NodePtr>,
    ) -> Self {
        Self::SilentPaymentSend(SilentPaymentSend::new(recipient, amount, memos))
    }
}

impl SpendAction for Action {
    fn calculate_delta(&self, deltas: &mut Deltas, index: usize) {
        match self {
            /* ... existing arms ... */
            #[cfg(feature = "chip-0057")]
            Action::SilentPaymentSend(a) => a.calculate_delta(deltas, index),
        }
    }
    fn spend(/* ... */) -> Result<(), DriverError> {
        match self {
            /* ... existing arms ... */
            #[cfg(feature = "chip-0057")]
            Action::SilentPaymentSend(a) => a.spend(ctx, spends, index),
        }
    }
}
```

The `#[cfg(feature = "chip-0057")]` gating mirrors how CHIP-0035/CHIP-0037 would extend `Action` if they had new variants. The match arms need `#[cfg]` in both `calculate_delta` and `spend`.

**`SilentPaymentPending` struct:**

```rust
// crates/chia-sdk-driver/src/silent_payments/send_keys.rs

#[derive(Debug, Clone)]
pub(crate) struct SilentPaymentPending {
    pub scan_pk: chia_bls::PublicKey,
    pub spend_pk: chia_bls::PublicKey,
    pub parent_xch_index: usize,
    pub parent_coin_id: chia_protocol::Bytes32,
    pub parent_puzzle_hash: chia_protocol::Bytes32,
    pub k: u32,
    pub amount: u64,
    pub memos: chia_puzzle_types::Memos<clvmr::NodePtr>,
}
```

`pub(crate)` because callers never construct it directly — they go through `SilentPaymentSend::spend`.

## 4. Sender keys: where do they come from?

This is the load-bearing decision for SEND-03 + SEND-04. Three sources considered:

| Source | Mechanism | Multi-party detection | Verdict |
|--------|-----------|------------------------|---------|
| **A1: Register synthetic SKs via `SpendContext`** (e.g., `ctx.register_silent_payment_sk(coin_id, sk)`) | Caller pushes synthetic SKs into the context before `finish`. Each `SilentPaymentSend::spend` records the per-coin SK lookup. | At finish, iterate `spends.xch.items` and for each non-ephemeral item, look up its `p2_puzzle_hash` → SK. If any missing, hard-error. | Rejected — adds state to `SpendContext`, which is currently a pure allocator + coin-spend collector. |
| **A2: Pass synthetic SK map to `Spends::finish_with_silent_payment_keys`** | New method on `Spends` taking `synthetic_sks: &IndexMap<Bytes32, SecretKey>` alongside the existing `synthetic_keys: &IndexMap<Bytes32, PublicKey>` (the latter renamed `synthetic_pks` for symmetry). | At finish, for each XCH item, look up `synthetic_sks[item.asset.p2_puzzle_hash()]`. If any missing, return `Err(DriverError::SilentPaymentMultiPartyUnsupported)`. | **RECOMMENDED** |
| **B: Take aggregated SK at `SilentPaymentSend::new`** | Caller pre-aggregates and passes a `ScalarField`. | Caller is responsible. | Rejected — exposes the multi-input aggregation rule to the caller, violates the `Spends` builder pattern, makes multi-output batches awkward (each action would carry the same aggregated SK). |
| **C: Infer from `chia-sdk-signer`** | The signer crate produces `RequiredBlsSignature`s; could extract synthetic PKs from those. | Would have to be at sign time, after the bundle is built — too late to rewrite puzzle hashes. | Rejected — wrong direction in the data flow. |

**Recommended path (A2):**

```rust
// crates/chia-sdk-driver/src/silent_payments/send_keys.rs

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;
use indexmap::IndexMap;

use crate::{DriverError, Outputs, Relation, SpendContext, Spends, action_system::Deltas};

impl Spends {
    /// Finish the spend with a synthetic-key map (PKs for signing, SKs for
    /// silent-payment ECDH at finish time).
    ///
    /// Hard-errors with `DriverError::SilentPaymentMultiPartyUnsupported` if
    /// the wallet does not hold a synthetic SK for every non-ephemeral XCH
    /// input. Multi-party silent payments are out of scope for v1; flows that
    /// mix wallet-controlled inputs with counterparty inputs MUST use
    /// `finish_with_keys` (which does not require SK material) and route the
    /// payment through a non-silent path.
    ///
    /// `synthetic_pks` is the same map `finish_with_keys` takes; `synthetic_sks`
    /// carries the secret material the silent-payment ECDH requires. The two
    /// maps are kept distinct so signers that split scan-online/spend-offline
    /// can pass an empty SK map when no `SilentPaymentSend` actions are in the
    /// batch (in which case `finish_with_silent_payment_keys` behaves
    /// identically to `finish_with_keys`).
    pub fn finish_with_silent_payment_keys(
        self,
        ctx: &mut SpendContext,
        deltas: &Deltas,
        relation: Relation,
        synthetic_pks: &IndexMap<Bytes32, PublicKey>,
        synthetic_sks: &IndexMap<Bytes32, SecretKey>,
    ) -> Result<Outputs, DriverError> {
        // Step 1: if no pending silent payments, delegate to finish_with_keys.
        if self.silent_payments_pending.is_empty() {
            return self.finish_with_keys(ctx, deltas, relation, synthetic_pks);
        }

        // Step 2: collect XCH input coin_ids + verify SK coverage.
        let xch_input_ids = self.collect_xch_input_ids(); // helper on Spends
        let mut sender_sks = Vec::with_capacity(self.xch.items.len());
        for item in self.xch.items.iter().filter(|i| !i.ephemeral) {
            let ph = item.asset.p2_puzzle_hash();
            let Some(sk) = synthetic_sks.get(&ph) else {
                return Err(DriverError::SilentPaymentMultiPartyUnsupported);
            };
            sender_sks.push(sk.clone());
        }
        if sender_sks.is_empty() {
            return Err(DriverError::SilentPaymentNoXchInputs);
        }

        // Step 3: aggregate sender SK + compute input_hash.
        let aggregated_sender_sk = aggregate_sender_sks(&sender_sks);
        let aggregated_sender_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes())
            .expect("ScalarField guarantees < r")
            .public_key();
        let input_hash = compute_input_hash(&xch_input_ids, &aggregated_sender_pk);

        // Step 4: for each pending silent-payment, derive the puzzle hash and
        //         emit a CreateCoin condition on the recorded parent.
        for pending in &self.silent_payments_pending {
            let ph = derive_one_time_puzzle_hash(
                &pending.scan_pk,
                &pending.spend_pk,
                &aggregated_sender_sk,
                &input_hash,
                pending.k,
            );

            let create_coin = CreateCoin::new(ph, pending.amount, pending.memos);
            let parent = &mut self.xch.items[pending.parent_xch_index];
            parent.kind.create_coin_with_assertion(
                ctx,
                pending.parent_puzzle_hash,
                &mut self.xch.payment_assertions,
                create_coin,
            );

            self.outputs.xch.push(Coin::new(pending.parent_coin_id, ph, pending.amount));
        }

        // Step 5: emit cross-input announcement binding (§5).
        emit_silent_payment_announcements(ctx, &mut self.xch, &xch_input_ids)?;

        // Step 6: delegate to standard finish path.
        self.finish_with_keys(ctx, deltas, relation, synthetic_pks)
    }
}

#[must_use]
pub fn aggregate_sender_sks(sks: &[SecretKey]) -> ScalarField {
    let mut sum = ScalarField::from_bytes_raw([0u8; 32]);
    for sk in sks {
        let sk_scalar = ScalarField::from_bytes_raw(sk.to_bytes());
        sum = sum.add(&sk_scalar);
    }
    sum
}
```

The pseudo-code above mutates `self` after the SK-coverage check passes, which conflicts with `self.finish_with_keys` taking `self` by value at the end. The planner needs to either (a) inline the bits of `finish_with_keys` that mutate `self.xch` before consuming `self`, or (b) restructure `finish_with_keys` to be `&mut self` for the mutating bits + a final consume step. Option (b) is cleaner and matches the precedent set by `Spends::prepare` (`spends.rs:436-458`) which already separates the mutating `prepare` step from the consuming `finish` step.

**On Option Q2 (`SyntheticSecretKey` newtype):** The pitfalls research recommends a newtype to compile-time-enforce synthetic vs raw keys. The SDK currently does not use such a newtype anywhere (`BlsPair.sk` is a raw `SecretKey`; `chia_bls::SecretKey::public_key()` does not derive synthetic — synthetic derivation happens via `DeriveSynthetic::derive_synthetic()` from `chia_puzzle_types`). Introducing a newtype just for silent payments creates a parallel API that wallets have to translate to/from. **Recommendation: stay with `&[SecretKey]` + a documented + debug-asserted parameter name (`synthetic_sks`). The synthetic guarantee is structural**: every key in `synthetic_sks` is the SK whose PK is the curried `synthetic_key` field in `StandardArgs` for the corresponding XCH input. The doc-comment plus the integration test ("send → scan → spend" round-trip on a simulator block in Phase 6) is the proof.

**Multi-party detection falls out naturally:** the `synthetic_sks` map is keyed by `p2_puzzle_hash`. If `Spends.xch.items` contains an input whose `p2_puzzle_hash` is not in the map, the wallet cannot have aggregated its SK — that's by definition a multi-party scenario. Return `Err(DriverError::SilentPaymentMultiPartyUnsupported)`.

**New `DriverError` variants:**

```rust
// crates/chia-sdk-driver/src/driver_error.rs (extension)

#[cfg(feature = "chip-0057")]
#[error("silent payment requires aggregating synthetic SKs for every input; multi-party flows are unsupported in v1")]
SilentPaymentMultiPartyUnsupported,

#[cfg(feature = "chip-0057")]
#[error("silent payment requires at least one wallet-controlled XCH input")]
SilentPaymentNoXchInputs,

#[cfg(feature = "chip-0057")]
#[error("a 32-byte first memo would be promoted to a puzzle_hash hint by the standard wallet, defeating silent-payment privacy")]
SilentPaymentMemoHintForbidden,
```

The existing `#[from] SilentPaymentError` variant (line 134-136) stays unchanged; these are three additional variants.

## 5. Input-hash binding via announcements (SEND-06, ROADMAP success criterion #4)

This is the second load-bearing decision. The CHIP defines the receive side (Pass 2b scanner grouping, lines 319-327 of `chip-silent-payments.md`); the SDK has to emit the matching send-side conditions.

### 5a. The CHIP semantics (canonical)

From `~/silent-payments/chip-silent-payments.md:319-329`:

> **Pass 2b:** Group by announcement linkage
> * For each *removal* in *removals*:
>   * Run the puzzle with its solution to extract output conditions
>   * If any condition has opcode 60 (`CREATE_COIN_ANNOUNCEMENT`): record the announcement ID = SHA256(*removal*.coin_id || *message*)
>   * If any condition has opcode 61 (`ASSERT_COIN_ANNOUNCEMENT`): record the asserted announcement ID
> * For each announcing *removal*, find all *removals* that assert its announcement ID. This forms a linked group.

The scanner's `announcement_id` is `SHA256(coin_id_of_creator || message)` per `chia_sdk_types::condition::announcements::announcement_id` (`crates/chia-sdk-types/src/condition/announcements.rs:6-11` — already in the SDK).

### 5b. The Python reference (canonical)

From `~/silent-payments/send_payment.py:303-323`:

```python
for i, ci in enumerate(sage_coins):
    if i == 0:
        # First coin carries the payment output and change
        conditions = [
            [51, onetime_puzzle_hash, payment_amount],  # CREATE_COIN
        ]
        # ... change, fee ...
        if len(sage_coins) > 1:
            conditions.append([60, b''])  # CREATE_COIN_ANNOUNCEMENT for binding
    else:
        # Additional coins assert the first coin's announcement
        import hashlib as _hl
        ann_id = _hl.sha256(sage_coins[0]["coin_id"] + b'').digest()
        conditions = [[61, ann_id]]  # ASSERT_COIN_ANNOUNCEMENT
```

**Critical observations:**
1. `i == 0` is the **first coin in the iteration order** (the order `sage_coins` is constructed in). The reference does NOT use the lex-min coin id — it uses iteration order. This is fine because Pass 2b only cares about announcement linkage, not which specific coin is the announcer.
2. The message is **empty** (`b''`). Announcement_id = `SHA256(coin_id_first || "")` = `SHA256(coin_id_first)` effectively.
3. Only the announcer emits opcode 60; every other coin emits opcode 61.

### 5c. The SDK's choice

| Choice | Pro | Con |
|--------|-----|-----|
| **Announcer = lex-min coin_id input** | Deterministic across spend orderings; receiver's Pass 2b doesn't care; matches CHIP §"Pass 2a" (which uses lex-min `coin_id_L` for input_hash) | Differs from Python reference |
| **Announcer = first input in `spends.xch.items` order** | Matches Python reference byte-for-byte; deterministic given insertion order | Different across spend orderings if `spends.add(coin)` order varies |

**Recommendation: lex-min coin_id input**, with `message = b""` per Python reference. Rationale:

1. The Spends builder pattern makes insertion order easy to vary (callers add coins as they come in from coin-selection). Lex-min is order-independent.
2. The `input_hash` computation already uses lex-min (`compute_input_hash` uses `coin_ids.iter().min()` — same precedent).
3. Pass 2b scanner doesn't care which input is the announcer — it just needs the announce-assert linkage to exist. Both choices produce identical detection.

### 5d. Implementation sketch

```rust
// crates/chia-sdk-driver/src/silent_payments/send_keys.rs

use chia_sdk_types::{
    Conditions,
    condition::{announcement_id, CreateCoinAnnouncement, AssertCoinAnnouncement},
};

fn emit_silent_payment_announcements(
    ctx: &mut SpendContext,
    xch: &mut FungibleSpends<Coin>,
    xch_input_ids: &[Bytes32],
) -> Result<(), DriverError> {
    if xch_input_ids.len() < 2 {
        return Ok(()); // single-input — no binding needed
    }

    let lex_min_coin_id = *xch_input_ids.iter().min().expect("non-empty");
    let ann_id = announcement_id(lex_min_coin_id, b"");

    for (idx, item) in xch.items.iter_mut().enumerate() {
        if item.ephemeral {
            continue;
        }
        let coin_id = item.asset.coin_id();
        let SpendKind::Conditions(spend) = &mut item.kind else {
            continue; // settlement spends don't carry conditions; skip
        };
        if coin_id == lex_min_coin_id {
            spend.add_conditions(Conditions::new().with(CreateCoinAnnouncement::new(b"".to_vec().into())));
        } else {
            spend.add_conditions(Conditions::new().with(AssertCoinAnnouncement::new(ann_id)));
        }
    }
    Ok(())
}
```

**Why this works:** the receiver's Pass 2b extracts `(message, coin_id)` from each opcode-60 condition, computes `SHA256(coin_id || message)`, then for each opcode-61 condition checks whether the asserted id matches any creator. With the SDK's emission rule above:
- Lex-min input emits opcode 60 with `message = b""`.
- Other inputs emit opcode 61 with `announcement_id = SHA256(lex_min_coin_id || "")`.
- Receiver's announcement_creators map gets `{ SHA256(lex_min_coin_id || "") -> lex_min_idx }`.
- Receiver's announcement_asserters list contains `(other_idx, SHA256(lex_min_coin_id || ""))` for each other input.
- Union-find groups them all. Pass 2b runs `ScanForSilentPayment` over the unioned group with `A_sum = Σ synthetic_pk_i` and `coin_ids = sorted([all coin_ids])` — which is exactly what the sender used to compute `input_hash`.

**Closing the round-trip:** Phase 3's `compute_input_hash` (which Phase 4 ships in `silent_payments/input_hash.rs`) takes `coin_ids: &[Bytes32]` and `aggregated_sender_pk: &PublicKey`. The receiver computes its own `compute_input_hash(coin_ids_sorted, A_sum)`. If both sides see the same lex-min coin_id + aggregated PK, the input_hash matches; ECDH closes; detection succeeds. Tested by ROADMAP success criterion #4 + Phase 4 integration test (§9, test count `cross_index_announcement_binding`).

### 5e. Conditions builder helpers

The `conditions!` macro at `crates/chia-sdk-types/src/condition.rs:17-100` auto-generates builder methods on `Conditions`. The macro hasn't generated a `.create_coin_announcement(...)` helper directly on `Conditions` — usages in the codebase like `mint_nft.create_coin_announcement(b"$".to_vec().into())` (`primitives/nft/nft_launcher.rs:124`) suggest the helper is generated. Worth verifying at plan time:

```bash
cargo doc -p chia-sdk-types --features chip-0057 --open
# search for `create_coin_announcement` and `assert_coin_announcement` on Conditions
```

If the auto-generated helpers don't exist, fall back to `Conditions::new().with(CreateCoinAnnouncement::new(message))` and `.with(AssertCoinAnnouncement::new(announcement_id))`. Both `CreateCoinAnnouncement::new` and `AssertCoinAnnouncement::new` are constructors generated by the macro (confirmed by reading `condition.rs:73-80`).

## 6. k-counter per recipient (SEND-05, ROADMAP success criterion #3)

The rule: same `scan_pk` across multiple `SilentPaymentSend` actions in one batch produces outputs at `k = 0, 1, 2, ...`.

### 6a. Where the counter lives

Three options:

| Location | Pro | Con |
|----------|-----|-----|
| **On `Spends`** | Single source of truth; trivial reset between batches (each new `Spends` starts fresh) | One more field on the already-large `Spends` struct |
| **On `SpendContext`** | More general (could be reused for non-Spends scenarios) | `SpendContext` is currently a pure allocator + coin-spend collector; adding silent-payment state pollutes its mental model |
| **On a `SilentPaymentBatch` helper** | Encapsulates the counter | Extra type for callers to know about; doesn't compose with the `Action` enum cleanly |

**Recommendation: `Spends`** — cheapest correct option, matches the precedent set by `Spends::silent_payments_pending` (also new, also batch-scoped).

```rust
// crates/chia-sdk-driver/src/action_system/spends.rs (extension)

#[derive(Debug, Clone)]
#[must_use]
pub struct Spends<S = Unfinished> {
    pub xch: FungibleSpends<Coin>,
    pub cats: IndexMap<Id, FungibleSpends<Cat>>,
    /* ... existing ... */

    #[cfg(feature = "chip-0057")]
    pub(crate) silent_payment_counters:
        std::collections::HashMap<[u8; 48], u32>,
    #[cfg(feature = "chip-0057")]
    pub(crate) silent_payments_pending:
        Vec<crate::silent_payments::SilentPaymentPending>,
    _state: S,
}
```

The `#[cfg(feature = "chip-0057")]` on the fields means the no-feature build's `Spends` is unchanged in memory layout — important for downstream consumers. Initializers in `Spends::new` / `with_separate_change_puzzle_hash` get matching `#[cfg]`-gated defaults.

`pub(crate)` because external callers never touch these directly — they go through `SilentPaymentSend::spend` (writes) and `Spends::finish_with_silent_payment_keys` (reads). External callers can read the resulting `Outputs.xch` after finish.

### 6b. Counter semantics

The counter is keyed by **`scan_pk` bytes** (48-byte compressed), NOT by recipient address or by `(scan_pk, spend_pk)` pair. CHIP-0057's per-spend-group `k` enumeration is per scan key — the recipient's scanner iterates `k = 0, 1, ...` for a single `shared_secret` (derived from `scan_sk * tweak_point`), and the same `shared_secret` produces all outputs that share that `scan_pk`. Different `spend_pk` values (i.e., labeled sub-addresses) share the same scan key and thus share the same `k` counter on the sender side.

**Edge case: labeled vs unlabeled addresses with the same scan key.** A labeled address has the same `scan_pk` as its unlabeled parent but a different `spend_pk` (`B_m = B_spend + label_pk`). If the sender sends to both `(scan_pk, B_spend)` and `(scan_pk, B_m)` in the same batch, the counter increments across both — k=0 for the first, k=1 for the second. This is correct: the receiver's scanner iterates k=0, finds the unlabeled match; iterates k=1, the unlabeled candidate misses but the labeled candidate (via `LabelRegistry`) hits. This matches Phase 3's `labeled_k_termination_rule` test (`scanner.rs:528-589`).

### 6c. Test coverage

Phase 4 must add at least three tests:

1. **`multi_output_same_scan_pk_increments_k`**: two `SilentPaymentSend` actions to the same recipient in one `Spends`. Inspect `spends.silent_payments_pending` (visible in the test crate via `pub(crate)` + `#[cfg(test)]` accessor) — assert `[0].k == 0` and `[1].k == 1`.
2. **`multi_output_distinct_scan_pks_independent_counters`**: two `SilentPaymentSend` actions to two different recipients in one `Spends`. Assert `[0].k == 0` and `[1].k == 0` (each counter starts fresh).
3. **`multi_output_post_finish_emits_correct_puzzle_hashes`**: after `finish_with_silent_payment_keys`, inspect `outputs.xch[0].puzzle_hash` and `outputs.xch[1].puzzle_hash`. Recompute the expected hashes using `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k=0)` and `k=1`. Assert byte-equality.

## 7. Memo-position hint guard (SEND-07, ROADMAP success criterion #5)

The hazard: the standard Chia wallet (Sage, mainnet wallet) interprets a 32-byte first memo as a `puzzle_hash` hint and indexes the output by that hash. For silent payments, the one-time `puzzle_hash` would be exposed publicly to any indexer — defeating the entire privacy gain.

### 7a. Three policy options

| Option | Behavior | Pro | Con |
|--------|----------|-----|-----|
| **A: Error on any 32-byte first memo** | `SilentPaymentSend::spend` returns `Err(DriverError::SilentPaymentMemoHintForbidden)` when the first memo is exactly 32 bytes | Loudest signal; easy to test (`assert!(matches!(result, Err(...)))`); zero risk of accidentally silent-failure | Wallet authors who legitimately want a 32-byte non-hint memo at position 0 have to wrap it with a 1-byte sentinel themselves |
| **B: Rewrite — insert a non-hint sentinel at position 0; error if remaining memos are 32 bytes** | The action transparently inserts a 1-byte sentinel (e.g., `0x00`) at position 0 if the user-supplied first memo is 32 bytes, OR errors if there are no memos and a 32-byte memo was intended as the first | Most user-friendly | The "transparent rewrite" surprises wallet authors who can't reproduce the on-chain memo structure from their input |
| **C: Error iff the first memo is exactly 32 bytes** | Same as A, narrower | Same as A | Same as A (this is effectively Option A) |

**Recommendation: Option A** — explicit hard-error with `DriverError::SilentPaymentMemoHintForbidden`.

Rationale:
1. **Loudest signal.** The privacy hazard is severe and silent (the payment goes through, but is publicly indexed). A hard error at apply time forces the wallet author to handle the case explicitly.
2. **Testable.** The test plan is "pass a 32-byte memo, assert the error variant fires" — bit-exact verification.
3. **Composable.** Wallet authors who legitimately want a 32-byte memo (e.g., a payment-purpose tag) can prepend their own 1-byte sentinel — the SDK doesn't mutate their input.
4. **Matches the SDK's existing hard-error philosophy.** `aggregate_sender_sks` hard-errors on multi-party; `labeled_address(0)` hard-errors on the reserved label. Memo-hint follows the same pattern.

### 7b. Implementation sketch

```rust
// crates/chia-sdk-driver/src/actions/silent_payment_send.rs

use chia_puzzle_types::Memos;
use clvmr::NodePtr;

use crate::{DriverError, SpendContext};

fn memo_hint_guard(ctx: &SpendContext, memos: &Memos<NodePtr>) -> Result<(), DriverError> {
    let Memos::Some(ptr) = memos else {
        return Ok(()); // no memos — safe
    };
    // Memos<NodePtr> is a CLVM list pointer. We need to extract the first
    // memo's byte length. The standard Chia memo list is `(memo1 memo2 ...)`,
    // a cons-cell chain where each memo is a CLVM atom.
    //
    // Use `clvmr::Allocator` accessors to walk the list:
    //   - if ptr is atom: malformed (memos must be a list); reject? — probably tolerate.
    //   - if ptr is pair: head = first memo; check head atom length.
    let allocator: &clvmr::Allocator = ctx.deref(); // SpendContext derefs to Allocator
    let Ok(first_atom) = clvm_extract_first_memo(allocator, *ptr) else {
        return Ok(()); // not a list, or no first element — safe
    };
    if first_atom.len() == 32 {
        return Err(DriverError::SilentPaymentMemoHintForbidden);
    }
    Ok(())
}

// Helper: extract the first element of a (cons ...) chain as a byte slice.
fn clvm_extract_first_memo(
    allocator: &clvmr::Allocator,
    ptr: NodePtr,
) -> Result<Vec<u8>, ()> {
    use clvmr::SExp;
    match allocator.sexp(ptr) {
        SExp::Pair(head, _tail) => match allocator.sexp(head) {
            SExp::Atom => Ok(allocator.atom(head).as_ref().to_vec()),
            SExp::Pair(_, _) => Err(()),
        },
        SExp::Atom => Err(()),
    }
}
```

The exact CLVM walk depends on `SpendContext`'s `Deref<Target = Allocator>` (verified at `spend_context.rs:20` — `SpendContext` holds a `pub(crate) allocator: Allocator`). The planner can inspect via `ctx.tree_hash(ptr)` or via direct `clvmr::Allocator::sexp` API. Plan 04-04 (memo guard) should produce a single helper in `silent_payments/memo_guard.rs` that's both `pub(crate)` and tested directly.

### 7c. Test plan (SEND-07 verification)

```rust
#[test]
fn memo_hint_guard_rejects_32_byte_first_memo() -> Result<()> {
    let mut sim = Simulator::new();
    let mut ctx = SpendContext::new();
    let alice = sim.bls(1);
    let recipient_keys = SilentPaymentKeys::from_mnemonic(/* TV1 */)?;
    let recipient_addr = recipient_keys.unlabeled_address(SilentPaymentNetwork::Mainnet);

    let bad_memos = ctx.memos(&[[0xff; 32]])?; // 32-byte memo at position 0

    let mut spends = Spends::new(alice.puzzle_hash);
    spends.add(alice.coin);

    let result = spends.apply(
        &mut ctx,
        &[Action::silent_payment_send(recipient_addr, 1, bad_memos)],
    );

    assert!(
        matches!(result, Err(DriverError::SilentPaymentMemoHintForbidden)),
        "expected SilentPaymentMemoHintForbidden, got {result:?}"
    );
    Ok(())
}

#[test]
fn memo_hint_guard_allows_non_32_byte_first_memo() -> Result<()> {
    let mut ctx = SpendContext::new();
    // 1-byte sentinel followed by 32-byte payload — the user's escape hatch.
    let safe_memos = ctx.alloc(&[vec![0x00u8], vec![0xff; 32]])?;
    let safe_memos = Memos::Some(safe_memos);

    let result = memo_hint_guard(&ctx, &safe_memos);
    assert!(result.is_ok());
    Ok(())
}
```

The post-emission verification (per ROADMAP success criterion #5: "verified by re-parsing the emitted `CreateCoin` condition") is satisfied at the same level by inspecting `spends.outputs.xch[0]` after finish — the test asserts that `outputs.xch[0].puzzle_hash` matches `derive_one_time_puzzle_hash(...)` AND `cargo expand` of the action shows the `CreateCoin` carries the user-supplied memos verbatim (no SDK-side mutation).

## 8. Privacy doc-comments (SEND-08)

Every public memo-bearing API in `silent_payments/` and `actions/silent_payment_send.rs` carries the same warning. The wording below is the canonical text the planner should use verbatim:

```rust
/// Privacy warning: memos are stored on-chain in plaintext and are visible to
/// anyone holding the recipient's scan key. Do not include sensitive data. A
/// 32-byte first memo would be promoted to a `puzzle_hash` hint by the standard
/// wallet, defeating silent-payment privacy entirely; `SilentPaymentSend`
/// rejects this shape with [`DriverError::SilentPaymentMemoHintForbidden`].
```

### 8a. Symbols requiring this comment

Existing (Phase 3, may need amending at Phase 4 if memos were not yet a concern):

| Symbol | Crate | File | Has it now? |
|--------|-------|------|-------------|
| `SilentPaymentAddress` | chia-sdk-utils | `silent_payments/address.rs` | No — address itself doesn't carry memos; not required |
| `SilentPaymentKeys` | chia-sdk-utils | `silent_payments/keys.rs` | Has a privacy note about scan-key compromise; restate the memo concern for completeness — not strictly required since `SilentPaymentKeys` doesn't carry memos |
| `TweakData`, `OutputMeta`, `DetectedSpCoin` | chia-sdk-driver | `silent_payments/types.rs` | No — receive-side types; no memos |

New (Phase 4):

| Symbol | File | Must carry the warning |
|--------|------|------------------------|
| `pub struct SilentPaymentSend` | `actions/silent_payment_send.rs` | **YES** — primary memo-bearing API |
| `pub struct SilentPaymentSend.memos` (field) | same | **YES** |
| `pub fn SilentPaymentSend::new(addr, amount, memos)` | same | **YES** |
| `pub fn Action::silent_payment_send(addr, amount, memos)` | `action_system/action.rs` | **YES** |
| `pub fn derive_one_time_puzzle_hash(...)` | `silent_payments/one_time.rs` | **YES** — outputs are silently inspectable; needs the warning to discourage CAT2 footgun composition |
| `pub fn aggregate_sender_sks(...)` | `silent_payments/aggregate.rs` | Recommended (no memo direct, but it's a send-side primitive) |
| `pub fn compute_input_hash(...)` | `silent_payments/input_hash.rs` | Recommended (same reasoning) |
| `pub fn Spends::finish_with_silent_payment_keys(...)` | `silent_payments/send_keys.rs` | **YES** — entry point for the whole flow |
| `DriverError::SilentPaymentMemoHintForbidden` | `driver_error.rs` | **YES** — the error's `#[error("...")]` text IS the documentation |

### 8b. Verification

```bash
# Doc-coverage check: every public symbol in silent_payments/ + the action
# carries 'Privacy warning' in its rustdoc.
grep -L 'Privacy warning' \
    crates/chia-sdk-driver/src/actions/silent_payment_send.rs \
    crates/chia-sdk-driver/src/silent_payments/{one_time,aggregate,input_hash,send_keys}.rs
# Expected: empty output (every file has at least one Privacy warning).
```

Plan 04-05 (final phase gate) should include this grep as a CI-equivalent verification.

## 9. Validation Architecture

> Phase 4 has `nyquist_validation` enabled per `.planning/config.json:19`. This section drives `04-VALIDATION.md` creation.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in libtest); `rstest 0.22.0` available as a dev-dep for parametric coverage |
| Config file | None — `cargo test --release -p chia-sdk-driver --features chip-0057` is the canonical invocation |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_send` |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| **SEND-01** | `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sk, input_hash, k=0)` matches TV1 puzzle_hash byte-for-byte | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::one_time::tests::tv1_derive_one_time_puzzle_hash_matches -- --exact` | ❌ Wave 0 |
| **SEND-01** | k=1 round-trip: `derive_one_time_puzzle_hash` produces a Phase-3-scannable hash for TV1 keys at k=1 | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::one_time::tests::derive_one_time_puzzle_hash_k1_round_trip -- --exact` | ❌ Wave 0 |
| **SEND-02** | `compute_input_hash([TV1_coin_id], TV1_sender_pk)` matches `38a1c837...cc9411` | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::tv1_compute_input_hash_matches -- --exact` | ❌ Wave 0 |
| **SEND-02** | `compute_input_hash` with two coin_ids picks the lex-min one | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::input_hash_uses_lex_min_coin_id -- --exact` | ❌ Wave 0 |
| **SEND-02** | `compute_input_hash` is order-independent (`(a,b)` == `(b,a)`) | unit (property-style) | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::input_hash::tests::input_hash_order_independent -- --exact` | ❌ Wave 0 |
| **SEND-03** | `aggregate_sender_sks(&[sk0, sk1])` for TV4 matches `5600d878...cbf95b89` | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::aggregate::tests::tv4_aggregate_sender_sks_matches -- --exact` | ❌ Wave 0 |
| **SEND-03** | Multi-party hard-error fires when `Spends` has 2 XCH inputs but only 1 in `synthetic_sks` | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::send_keys::tests::multi_party_hard_errors -- --exact` | ❌ Wave 0 |
| **SEND-04** | Round-trip: hand-built `Spends` + `Action::silent_payment_send` → finish → `outputs.xch[0].puzzle_hash` equals `derive_one_time_puzzle_hash(...)` byte-for-byte | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::round_trip_matches_derive_one_time_puzzle_hash -- --exact` | ❌ Wave 0 |
| **SEND-04** | Action's apply-time + finish-time split: `spends.silent_payments_pending` has one entry after `apply`; `outputs.xch` has the matching coin after `finish_with_silent_payment_keys` | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::action_state_machine -- --exact` | ❌ Wave 0 |
| **SEND-05** | Two `SilentPaymentSend` actions to the same `scan_pk` produce outputs at `k=0` and `k=1` | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k -- --exact` | ❌ Wave 0 |
| **SEND-05** | Two actions to different scan_pks produce independent counters (both at k=0) | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_distinct_scan_pks_independent_counters -- --exact` | ❌ Wave 0 |
| **SEND-06** | `Spends` with 2 XCH inputs at different indices emits opcode 60 on the lex-min coin and opcode 61 on the other; the asserted id matches `SHA256(coin_id_min \|\| "")` | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::cross_index_announcement_binding -- --exact` | ❌ Wave 0 |
| **SEND-06** | `Spends` with a single XCH input emits NO opcode 60/61 (single-input doesn't need binding) | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::single_input_no_announcement -- --exact` | ❌ Wave 0 |
| **SEND-06** | The receiver-side `compute_input_hash` (Phase 4's free function) computed over the on-chain coin_ids + extracted aggregated PK matches the sender's input_hash byte-for-byte | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::input_hash_round_trip -- --exact` | ❌ Wave 0 |
| **SEND-07** | 32-byte first memo errors with `DriverError::SilentPaymentMemoHintForbidden` at `apply()` time | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_rejects_32_byte_first_memo -- --exact` | ❌ Wave 0 |
| **SEND-07** | 1-byte sentinel + 32-byte payload memos pass the guard | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_allows_sentinel_prefixed -- --exact` | ❌ Wave 0 |
| **SEND-07** | `Memos::None` passes the guard | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::memo_hint_guard_allows_none -- --exact` | ❌ Wave 0 |
| **SEND-08** | Every public memo-bearing API has a `Privacy warning` rustdoc comment | doc / grep | `! grep -L 'Privacy warning' crates/chia-sdk-driver/src/actions/silent_payment_send.rs crates/chia-sdk-driver/src/silent_payments/{one_time,aggregate,input_hash,send_keys}.rs` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_send` (~20s)
- **Per wave merge:** `cargo test --release -p chia-sdk-driver --features chip-0057` (~45s) — catches regressions in Phase 1/2/3 scanner code that Phase 4 might break inadvertently
- **Phase gate:** Full workspace suite green before `/gsd:verify-work`; plus `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`; plus `cargo fmt --check`; plus `cargo machete`; plus the four grep bans (Phase 1's `mod_by_group_order` + `use sha2::` + Phase 3's `Sha256::digest` + Phase 4's `'Privacy warning'` coverage check).

### Wave 0 Gaps
- [ ] `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — covers SEND-04, SEND-05, SEND-06, SEND-07
- [ ] `crates/chia-sdk-driver/src/silent_payments/one_time.rs` — covers SEND-01
- [ ] `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` — covers SEND-02
- [ ] `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` — covers SEND-03 (free function)
- [ ] `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` — covers SEND-03 (Spends-level hard-error) + `SilentPaymentPending` + `Spends::finish_with_silent_payment_keys`
- [ ] `crates/chia-sdk-driver/src/silent_payments/memo_guard.rs` (or inlined in `silent_payment_send.rs`) — covers SEND-07 helper
- [ ] `crates/chia-sdk-driver/src/silent_payments/mod.rs` — extend barrel with 4 new modules + re-exports
- [ ] `crates/chia-sdk-driver/src/actions.rs` — add `#[cfg(feature = "chip-0057")] mod silent_payment_send; #[cfg(feature = "chip-0057")] pub use silent_payment_send::*;`
- [ ] `crates/chia-sdk-driver/src/action_system/action.rs` — add `Action::SilentPaymentSend` variant + constructor + match arms (3 sites)
- [ ] `crates/chia-sdk-driver/src/action_system/spends.rs` — add `silent_payment_counters` + `silent_payments_pending` fields (chip-0057-gated)
- [ ] `crates/chia-sdk-driver/src/driver_error.rs` — add 3 new variants (`SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`, `SilentPaymentMemoHintForbidden`)
- [ ] `src/prelude.rs` — extend `#[cfg(feature = "chip-0057")]` block with `SilentPaymentSend`, `derive_one_time_puzzle_hash`, `compute_input_hash` (recommend NOT including `aggregate_sender_sks` per PITFALLS §"Anti-Pattern 2")

*Test framework: cargo + libtest are already present. No install command required.*

## 10. Plan slicing

Recommended split into 5 plans. Each plan closes 1–2 SEND-* requirements and produces a green workspace test count.

| Plan | Title | SEND-* IDs closed | Wave 0 deliverables | Roughly |
|------|-------|--------------------|---------------------|---------|
| **04-01** | `derive_one_time_puzzle_hash` + `compute_input_hash` + `aggregate_sender_sks` free functions | SEND-01, SEND-02, SEND-03 (free-fn portion) | `silent_payments/{one_time,input_hash,aggregate}.rs` + extend `silent_payments/mod.rs` barrel + 6 unit tests pinning TV1/TV4 values | ~300 lines |
| **04-02** | `SilentPaymentSend` struct + `SpendAction` impl + `Action::SilentPaymentSend` variant | SEND-04 (action plumbing) | `actions/silent_payment_send.rs` + `actions.rs` barrel + `action_system/action.rs` 3-site extension + `action_system/spends.rs` two new fields + 2 integration tests (round-trip + state-machine) | ~250 lines |
| **04-03** | `Spends::finish_with_silent_payment_keys` + multi-party hard-error | SEND-03 (Spends-level hard-error), SEND-04 (finish-time) | `silent_payments/send_keys.rs` + `SilentPaymentPending` + 3 new `DriverError` variants + 2 integration tests (multi-party hard-error + finish round-trip) | ~250 lines |
| **04-04** | Multi-output `k` counter + cross-input announcement binding | SEND-05, SEND-06 | Extend `send_keys.rs` (`emit_silent_payment_announcements`) + extend action.rs (counter increment) + 5 integration tests (multi-output × 2, cross-index announcement × 3) | ~150 lines |
| **04-05** | Memo-hint guard + Privacy-warning doc-comments + final phase gate | SEND-07, SEND-08 | `silent_payments/memo_guard.rs` (or inlined in action) + 3 unit tests + audit pass over all new `pub` symbols + `! grep -L 'Privacy warning' ...` gate + full workspace sweep | ~100 lines |

| SEND-* | Plan |
|--------|------|
| SEND-01 | 04-01 |
| SEND-02 | 04-01 |
| SEND-03 | 04-01 (free function) + 04-03 (Spends-level hard-error) |
| SEND-04 | 04-02 (apply-time) + 04-03 (finish-time) |
| SEND-05 | 04-04 |
| SEND-06 | 04-04 |
| SEND-07 | 04-05 |
| SEND-08 | 04-05 |

## 11. Pitfalls

Non-obvious issues the planner could get wrong.

### Pitfall A: `aggregate_sender_sks` collapsed to a `ScalarField` loses the PK
The aggregated `ScalarField` is what `derive_one_time_puzzle_hash` needs internally (for the shared-secret ECDH); but `compute_input_hash` needs the aggregated **PK**, which requires a round-trip back through `chia_bls::SecretKey::from_bytes(...).public_key()`. The temptation: skip the PK round-trip and just hash the scalar bytes. **Wrong** — `input_hash = tagged_hash("Chia_SP/Inputs", coin_id_min || serialize(A_sum))` where `serialize(A_sum)` is the 48-byte compressed BLS PK, not the 32-byte SK. The pseudo-code in §4 has the right shape:
```rust
let aggregated_sender_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes())
    .expect("ScalarField guarantees < r")
    .public_key();
```

The `.expect` is sound because `ScalarField::from_bytes_raw` outputs are guaranteed `< r` (the `.add` operation reduces mod r). Verify the test passes TV1 byte-for-byte before relying on this.

### Pitfall B: `Spends::finish_with_silent_payment_keys` consumes `self` but emits conditions during the consume
`Spends::finish_with_keys` (line 460-495) takes `self` by value. Phase 4's extension wants to (a) emit `CreateCoin` conditions on existing `xch.items` AND (b) emit opcode-60/61 announcement bindings on existing `xch.items`, BEFORE the standard finish path runs. If the implementation writes "compute aggregated_sk, then mutate self.xch[i].kind, then call self.finish_with_keys(...)", the borrow checker complains because `self` is partially moved. The fix is to split: do all mutation in a `&mut self` helper method called from `finish_with_silent_payment_keys`, then call `self.finish_with_keys(...)` as the final consume step. Matches the existing `Spends::prepare` pattern (line 436-458).

### Pitfall C: The `output_source` placeholder puzzle hash conflicts
Calling `spends.xch.output_source(ctx, &Output::new(Bytes32::default(), amount))` in `SilentPaymentSend::spend` might accidentally match a placeholder output the SDK constructed elsewhere. The `Bytes32::default()` (all zeros) is an unusual puzzle hash but not impossible. Recommended alternative: use `BURN_PUZZLE_HASH` (`hex!("0000...0000dead")`, declared in `action.rs:15-17`) as the placeholder. Burn hash is by construction unspendable and shouldn't collide. The real puzzle hash is written at finish time.

### Pitfall D: `synthetic_sks` map keying by `p2_puzzle_hash` for the BlsPair test fixture
`BlsPair.puzzle_hash` is `StandardArgs::curry_tree_hash(pk).into()` (`key_pairs.rs:33`) — note: `pk` is `sk.public_key()`, NOT `sk.public_key().derive_synthetic()`. So in `BlsPair`, the "synthetic" key IS the raw key (because the test fixture doesn't apply the hidden-puzzle offset). When writing the Phase 4 integration tests, `synthetic_pks` and `synthetic_sks` are built as `indexmap! { alice.puzzle_hash => alice.pk }` / `indexmap! { alice.puzzle_hash => alice.sk }` — same key shape as `SendAction`'s existing tests. The "synthetic-ness" of the SK is a property of the wallet's key derivation pipeline, not of the SDK's enforced typing.

For Phase 6's simulator round-trip, the keys will be CHIP-derived (from `SilentPaymentKeys::from_mnemonic`) and the puzzle hash will be `StandardArgs::curry_tree_hash(spend_sk.public_key())` for the sender's wallet — same shape, just sourced differently. The Phase 4 integration tests can use `BlsPair` and the round-trip closes correctly because the math is consistent across both interpretations.

### Pitfall E: `Spends::finish` is not async (and never was)
ARCHITECTURE.md mentioned this might be an open question. **Confirmed sync** — `finish_with_keys` returns `Result<Outputs, DriverError>` directly (`spends.rs:460-495`), no `async`. Phase 4's `finish_with_silent_payment_keys` is also sync. The ECDH math is fast (one BLS scalar-mul per pending entry, one SHA256, one mod-r reduction).

### Pitfall F: Compile error when one of the chip-0057 actions extends a non-cfg-gated enum
`Action::SilentPaymentSend` is gated, but `Action`'s `Debug, Clone` derives must still work when the feature is off. Rust's derive macros handle `#[cfg]`-gated variants correctly. Verify by running `cargo check --workspace` (no features) after Plan 04-02. The pattern of cfg-gated enum variants is used elsewhere in the SDK — `chia-sdk-types`'s `Puzzle` enum has `#[cfg(feature = "chip-0035")]` variants. So this is established precedent.

### Pitfall G: `Conditions::with(CreateCoinAnnouncement::new(message))` vs `.create_coin_announcement(message)` — verify the macro generates the helper
The `conditions!` macro at `crates/chia-sdk-types/src/condition.rs:17` is a proc macro from `chia-sdk-derive`. Its codegen for builder methods isn't visible at grep time. Three call sites use `.create_coin_announcement(...)` or `.assert_coin_announcement(...)` (`bindings/conditions.rs:178`, `primitives/nft/nft_launcher.rs:124,130`, `primitives/launcher.rs:141`, `primitives/intermediate_launcher.rs:84`) — the helper IS generated. Recommended: use the builder method (`Conditions::new().create_coin_announcement(message)`) as primary; fall back to `.with(CreateCoinAnnouncement::new(message))` only if the helper has a name collision (it won't in the silent-payments file because no existing conditions code is in this scope).

### Pitfall H: BLS aggregated SK can hit zero (vanishing probability)
`aggregate_sender_sks(&[sk1, sk2])` returns `ScalarField::from_bytes_raw(sk1.bytes) + ScalarField::from_bytes_raw(sk2.bytes)` reduced mod r. There's a `1/r ≈ 2^-255` chance the sum is zero, in which case `SecretKey::from_bytes(zero)` returns `Err`. **In practice this never happens** (cosmic-ray-level probability), but the `.expect` in the planned code (`§4`) would panic. Acceptable v1 risk — same as the synthetic-key offset in `derive_synthetic` (also has the same vanishing-probability issue). Document but do not handle.

### Pitfall I: `Spends::silent_payments_pending` ordering matters for finalization
The `Vec<SilentPaymentPending>` preserves action-iteration order. The finalize step iterates this vec in order and assigns puzzle hashes. If two recipients share a scan_pk, the order of pending entries determines which gets k=0 vs k=1 (per §6). This is intentional and correct — matches the order the actions appeared in `Spends::apply(&[Action, Action, ...])`. Document; do not sort.

### Pitfall J: The `parent_xch_index: usize` in `SilentPaymentPending` becomes stale if `Spends.xch.items` grows between apply and finish
`SpendKind` operations during apply (intermediate-source creation, change creation) can append new items to `xch.items` via `FungibleSpends::intermediate_source` (line 112-141). If a later action's `spend()` triggers `intermediate_source`, the indices of EARLIER items don't shift (Vec append-only), but the indices of LATER items haven't been finalized yet. So storing the index at apply time and looking it up at finish time is safe **as long as no `xch.items` element is removed**. Verified: nothing in the existing `Spends` flow removes items from `xch.items` between apply and finish.

## Sources

### Primary (HIGH confidence) — read end-to-end
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/action.rs` — `Action` enum, `SpendAction` trait, dispatcher
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/spends.rs` — `Spends` builder, `apply`, `prepare`, `finish_with_keys`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/fungible_spends.rs` — `FungibleSpends`, `output_source`, `intermediate_source`, `create_change`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/spend_kind.rs` — `SpendKind`, `create_coin_with_assertion`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` — `ConditionsSpend::add_conditions`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/deltas.rs` — `Deltas`, `Delta`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/id.rs` — `Id::{Xch, Existing, New}`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/asset.rs` — `Asset` trait
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/send.rs` — `SendAction` reference
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/fee.rs` — `FeeAction` reference (no-output pattern)
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/settle.rs` — `SettleAction` reference (announcement-emitting pattern)
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/layers/standard_layer.rs` — `StandardLayer::new(synthetic_key)`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/driver_error.rs` — `DriverError` enum + `SilentPayment(SilentPaymentError)` variant from Phase 3
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/spend_context.rs` — `SpendContext`, `memos`, `hint`, `alloc`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/silent_payments/{mod,protocol,scanner,types}.rs` — Phase 3 primitives that Phase 4 composes
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/src/silent_payments/{mod,keys,address}.rs` — `SilentPaymentKeys`, `SilentPaymentAddress`, `generate_label` reach-through
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/src/condition.rs` — `CreateCoinAnnouncement` (opcode 60), `AssertCoinAnnouncement` (opcode 61), `CreateCoin` (opcode 51), `Conditions` builder
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/src/condition/announcements.rs` — `announcement_id(coin_id, message) = SHA256(coin_id || message)`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-test/src/key_pairs.rs` — `BlsPair` test fixture (synthetic vs raw key reality)
- `/home/kdc/chia-wallet-sdk/src/prelude.rs` — current chip-0057 prelude block (extend in Plan 04-05)
- `/home/kdc/silent-payments/crates/sp-common/src/protocol.rs` — reference `aggregate_sender_sks`, `create_silent_payment_outputs`
- `/home/kdc/silent-payments/crates/sp-common/src/ecdh.rs` — reference `compute_input_hash` (lex-min coin_id), `compute_shared_secret_sender`
- `/home/kdc/silent-payments/crates/sp-common/src/puzzle.rs` — reference `puzzle_hash_for_pk` (Phase 3 ported)
- `/home/kdc/silent-payments/send_payment.py:303-323` — canonical Python reference for opcode 60/61 announcement binding
- `/home/kdc/silent-payments/chip-silent-payments.md:285-329` — CHIP Pass 2b scanner grouping semantics
- `/home/kdc/silent-payments/.claude/worktrees/agent-a1a50c7d/crates/sp-service/src/grouping.rs` — receiver-side Pass 2b implementation (announcement union-find)

### Secondary (HIGH confidence) — read but not end-to-end
- `/home/kdc/chia-wallet-sdk/.planning/research/ARCHITECTURE.md` — already-validated architecture, especially §"Action System Integration (SEND-04)" lines 167-319
- `/home/kdc/chia-wallet-sdk/.planning/research/PITFALLS.md` §§ 1, 2, 3, 6, 7, 9, 12 — covers signed/unsigned, synthetic/raw, multi-party, endianness, k-termination, BLS API, CAT2 footgun
- `/home/kdc/chia-wallet-sdk/.planning/research/FEATURES.md` — `Vec<Recipient>` recommendation + memo-position guard derivation
- `/home/kdc/chia-wallet-sdk/.planning/research/SUMMARY.md` — Q1 (Option A vs B), Q2 (newtype vs documented), Q4 (`Vec<Recipient>`) resolution
- `/home/kdc/chia-wallet-sdk/.planning/PROJECT.md` — locked decisions, out-of-scope, REQUIREMENTS table
- `/home/kdc/chia-wallet-sdk/.planning/REQUIREMENTS.md` — SEND-01..08 + traceability
- `/home/kdc/chia-wallet-sdk/.planning/ROADMAP.md` — Phase 4 entry + 6 success criteria
- `/home/kdc/chia-wallet-sdk/.planning/STATE.md` — Phase 3 closure context
- `/home/kdc/chia-wallet-sdk/.planning/phases/03-receive-primitive-chip-test-vector-closure/03-RESEARCH.md` — Phase 3 type surface that Phase 4 inherits
- `/home/kdc/chia-wallet-sdk/CLAUDE.md` — workspace lint policy, feature-gate cascade rules

### Tertiary (no LOW-confidence findings) — none

## Metadata

**Confidence breakdown:**
- Action system anatomy (§1): HIGH — read all `Action`/`SpendAction`/`Spends` source end-to-end
- File location (§2): HIGH — every existing action is at `actions/<name>.rs`; ARCHITECTURE.md confirms
- API shape (§3): HIGH — three options compared against existing patterns; recommended option matches `SendAction` precedent byte-for-byte
- Sender keys (§4): HIGH — Option A vs B resolved by reading `finish_with_keys` data flow; Option A2 (parallel SK map) is the unique sound choice. Option Q2 (newtype) downgraded to "documented" based on SDK norms (chia-bls SecretKey is shared)
- Announcement binding (§5): HIGH — Python reference + CHIP §319-329 specify the exact rule; opcode 60 with `message = b""` on lex-min input is byte-compatible
- k-counter (§6): HIGH — per scan_pk keying matches Phase 3 scanner's per-spend-group k-iteration semantics
- Memo guard (§7): HIGH — Option A wins on the "loudest signal" criterion; CLVM walk is established by `SpendContext::memos` precedent
- Privacy docs (§8): HIGH — canonical wording derived from PITFALLS §13, §14, §11 and SEND-08 phrasing
- Validation architecture (§9): HIGH — 18 tests across 7 SEND-* + 1 doc gate; matches Phase 3's 13-test density
- Plan slicing (§10): HIGH — 5 plans aligns with Phase 3's 5-plan rhythm; SEND-* split is clean
- Pitfalls (§11): HIGH — 10 pitfalls, all anchored to specific source lines or behavior

**Research date:** 2026-05-15
**Valid until:** 2026-06-15 (30 days; the SDK is stable, no anticipated upstream API changes)
