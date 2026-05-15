# Architecture Research: CHIP-0057 Silent Payments Integration

**Domain:** Wallet-side cryptographic primitives + send-side action + transport-agnostic receive primitive integrated into the existing chia-wallet-sdk layered driver architecture.
**Researched:** 2026-05-15
**Confidence:** HIGH — derived from direct reads of the SDK's existing CHIP-0035/CHIP-0037 integration points, the action system source, and the `sp-common` reference implementation that this work folds in.

## System Overview

Silent payments add three layers of new code on top of three existing SDK crates. No new crate, no new transport.

```
┌──────────────────────────────────────────────────────────────────────────┐
│            chia-sdk-bindings (always-on facade)                           │
│  silent_payments.rs:  SilentPaymentKeys, SilentPaymentAddress, TweakData, │
│                       DetectedSpCoin, scan_from_tweaks, derive_one_time_*│
│         (re-exports types from driver+utils+types behind chip-0057)       │
└──────────────────────────────────────────────────────────────────────────┘
                                    ▲
                                    │ #[cfg(feature="chip-0057")] re-exports
                                    │
┌──────────────────────────────────────────────────────────────────────────┐
│        chia-sdk-driver (chip-0057 gated)                                  │
│                                                                            │
│   src/silent_payments/                                                    │
│     mod.rs                ── barrel ─────────────────────┐                │
│     ecdh.rs               compute_shared_secret_*,       │                │
│                            compute_input_hash             │                │
│     protocol.rs           derive_output_tweak,           │  consumed by   │
│                            derive_onetime_{pk,sk},        │  SilentPayment │
│                            create_silent_payment_outputs, │  Send action   │
│                            scan_from_tweaks (RECV-02)     │                │
│     aggregate.rs          aggregate_sender_sks (SEND-03) │                │
│     labels.rs             generate_label, LabelRegistry  │                │
│     tweak_data.rs         TweakData, DetectedSpCoin,     │                │
│                            OutputMeta                     │                │
│   src/actions/silent_payment_send.rs                                      │
│       SilentPaymentSend ── impl SpendAction ─────────────┘                │
│   src/action_system/action.rs                                             │
│       Action::SilentPaymentSend(...) variant (chip-0057 gated)            │
└──────────────────────────────────────────────────────────────────────────┘
                                    ▲                ▲
                                    │                │
┌────────────────────────────────────┐  ┌────────────────────────────────────┐
│  chia-sdk-utils (chip-0057 gated)  │  │  chia-sdk-types (chip-0057 gated) │
│                                    │  │                                    │
│   src/silent_payments/             │  │   src/silent_payments/             │
│     mod.rs                         │  │     mod.rs                         │
│     keys.rs                        │  │     scalar.rs   ScalarField        │
│       SilentPaymentKeys            │  │     tagged_hash.rs                 │
│         from_mnemonic              │  │       tagged_hash() + constants:   │
│         scan_sk(), spend_sk()      │  │       CHIA_SP_INPUTS,              │
│         unlabeled_address()        │  │       CHIA_SP_SHARED_SECRET,       │
│         labeled_address(m)         │  │       CHIA_SP_LABEL                │
│     address.rs                     │  │     paths.rs                       │
│       SilentPaymentAddress         │  │       SCAN_PATH = [12381,8444,12,0]│
│         encode()/decode()          │  │       SPEND_PATH = [12381,8444,13,0]│
│         HRP: spxch / tspxch        │  │                                    │
│                                    │  │                                    │
└────────────────────────────────────┘  └────────────────────────────────────┘
            uses ScalarField,                  zero internal deps;
            tagged_hash, paths                 only chia-bls, chia-sha2,
                                               num-bigint
```

Arrows are `depends-on`. The graph is acyclic — utils and driver both depend on types; driver depends on utils for `SilentPaymentAddress` reconstruction in receive-side test helpers and binding facade composition. No back-edge.

### Component Responsibilities

| Component | Responsibility | Crate | Feature |
|-----------|----------------|-------|---------|
| `ScalarField` | Unsigned mod-r reduction (BLS12-381 subgroup order) distinct from existing signed `mod_by_group_order`. Compiler-enforced separation from synthetic-key math. | `chia-sdk-types::silent_payments::scalar` | `chip-0057` |
| `tagged_hash` + tag constants | BIP-340-style tagged hash with `Chia_SP/Inputs`, `Chia_SP/SharedSecret`, `Chia_SP/Label`. Pure function. | `chia-sdk-types::silent_payments::tagged_hash` | `chip-0057` |
| `SCAN_PATH` / `SPEND_PATH` constants | Derivation path constants (12381/8444/12/0 and 12381/8444/13/0). No code, just `&'static [u32]`. | `chia-sdk-types::silent_payments::paths` | `chip-0057` |
| `SilentPaymentKeys` | Mnemonic → `(scan_sk, spend_sk)` derivation; `unlabeled_address()` and `labeled_address(m)` constructors. Holds the secret material wallets need to keep online for scanning. | `chia-sdk-utils::silent_payments::keys` | `chip-0057` |
| `SilentPaymentAddress` | 96-byte `scan_pk \|\| spend_pk` payload + HRP (`spxch` / `tspxch`). Bech32m encode/decode. Mirrors existing `Address` in the same crate. | `chia-sdk-utils::silent_payments::address` | `chip-0057` |
| `ecdh` module | `compute_shared_secret_sender`, `compute_shared_secret_scanner`, `compute_input_hash` (uses lex-min coin_id + aggregated sender_pk). | `chia-sdk-driver::silent_payments::ecdh` | `chip-0057` |
| `protocol` module | `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `create_silent_payment_outputs`, `scan_from_tweaks` (RECV-02). | `chia-sdk-driver::silent_payments::protocol` | `chip-0057` |
| `aggregate` module | `aggregate_sender_sks` for single-party multi-input aggregation. Returns `ScalarField`. Documented as single-party only (multi-party must aggregate at sign time). | `chia-sdk-driver::silent_payments::aggregate` | `chip-0057` |
| `labels` module | `generate_label(scan_sk, m) -> (ScalarField, PublicKey)` and `LabelRegistry`: wallet-side `m -> label_pk` plus reverse `label_pk -> m` lookup. | `chia-sdk-driver::silent_payments::labels` | `chip-0057` |
| `TweakData`, `DetectedSpCoin`, `OutputMeta` | Transport-agnostic input/output types for `scan_from_tweaks`. Designed to be the stable surface that a future CHIP-0058 client adapts to. | `chia-sdk-driver::silent_payments::tweak_data` | `chip-0057` |
| `SilentPaymentSend` action | `impl SpendAction` (`calculate_delta` + `spend`). Coordinates with `Spends.xch` to allocate XCH inputs, compute aggregated sender SK + input_hash, derive the one-time puzzle hash, and synthesize a regular `CreateCoin` condition on the XCH source. Variant added to the `Action` enum. | `chia-sdk-driver::actions::silent_payment_send` | `chip-0057` |
| `silent_payments` binding facade | One file in `chia-sdk-bindings`. Always compiled; uses `#[cfg(feature = "chip-0057")]` to gate types when the underlying feature is off. Re-exposes types via `bindings/silent_payments.json`. | `chia-sdk-bindings::silent_payments` | always-on, gated re-exports |

## Recommended Module Layout

```
crates/
├── chia-sdk-types/src/silent_payments/         # chip-0057
│   ├── mod.rs                                  # barrel, re-exports
│   ├── scalar.rs                               # ScalarField newtype
│   ├── tagged_hash.rs                          # tagged_hash() + tag &'static str consts
│   └── paths.rs                                # SCAN_PATH / SPEND_PATH arrays
│
├── chia-sdk-utils/src/silent_payments/         # chip-0057
│   ├── mod.rs                                  # barrel
│   ├── keys.rs                                 # SilentPaymentKeys
│   └── address.rs                              # SilentPaymentAddress
│
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

### Structure Rationale

- **Why a `silent_payments/` directory at each crate root, not split into individual files?** Mirrors `chia-sdk-driver/src/primitives/cat/`, `nft/`, etc. — when more than one or two files relate, a directory with a `mod.rs` barrel is the established pattern (verified in `STRUCTURE.md` "Module Structure Pattern"). Datalayer and action-layer follow this. Silent payments has 5 driver files, 2 utils files, and 3 types files — directory is correct.
- **Why types in `chia-sdk-types`, not `chia-sdk-driver`?** `chia-sdk-types` owns "pure value types and constants with no driver dependencies." `ScalarField`, `tagged_hash`, and derivation paths are pure (`num-bigint`, `sha2`, `chia-bls` — already workspace deps). The fact that CHIP-0037 already places similar primitive crypto (`p2_eip712_message.rs`, `p2_controller_puzzle.rs` types) in `chia-sdk-types` confirms this placement.
- **Why `SilentPaymentAddress` in `chia-sdk-utils`, not `chia-sdk-driver`?** `chia-sdk-utils/src/bech32.rs` already houses the standard `Address` and `Bech32` types. The silent-payment address is the same family (bech32m + HRP + payload, different prefix and payload length). Co-located by category, not by feature.
- **Why `SilentPaymentKeys` in `chia-sdk-utils`, not `chia-sdk-types`?** `SilentPaymentKeys` is a wallet-facing convenience type that derives `SecretKey`s and produces `SilentPaymentAddress` strings. It uses `bip39::Mnemonic` + `chia_bls::DerivableKey`. Putting it in `utils` keeps the bech32-related code together (the address constructors are right there) and matches where `chia-sdk-bindings::mnemonic.rs` already sits in spirit. Putting it in `types` would force `chia-sdk-types` to pull `bip39` as a new dep, which is a heavier coupling than needed for what should be a leaf crate.
- **Why `silent_payment_send.rs` under `actions/`, not under `silent_payments/`?** `actions/` is the established location for `SpendAction` impls (`send.rs`, `create_did.rs`, `mint_nft.rs`, etc.). The crypto + transport-agnostic primitives live in `silent_payments/`; the integration with the spend builder lives in `actions/`. Same split CHIP-0035 uses for vault: types/layers under `primitives/vault/`, but the vault-related actions are inline with other actions.
- **Why no new top-level crate?** Locked by PROJECT.md (constraint). Aligned with the precedent set by `chip-0035` (datalayer + vault) and `chip-0037` (EIP-712 + controller puzzle) — both ship as modules behind a feature flag in `chia-sdk-types` + `chia-sdk-driver`. A standalone `chia-sdk-silent-payments` crate may emerge later if/when a CHIP-0058 transport client lands.

## Dependency Direction

```
chia-sdk-types::silent_payments        (leaf, no internal SDK deps)
        ▲
        │
        ├─── chia-sdk-utils::silent_payments       (uses ScalarField, tagged_hash, paths)
        │           ▲
        │           │
        └─── chia-sdk-driver::silent_payments      (uses ScalarField, tagged_hash,
                    ▲                               and SilentPaymentAddress for tests)
                    │
                    └── chia-sdk-bindings::silent_payments
                          (re-exports from driver/utils/types under chip-0057 cfg)
```

**Verification of acyclicity:**
- `chia-sdk-types` already depends on nothing internal (confirmed by reading its `Cargo.toml`: only `chia-protocol`, `chia-bls`, `clvm-*` deps). Adding `silent_payments` keeps it a leaf.
- `chia-sdk-utils` currently does not depend on `chia-sdk-types` (it only uses `chia-protocol` + `bech32`). The silent-payments addition introduces a new edge `utils → types`. This is a one-directional addition; `chia-sdk-types` has no reason to ever depend on `utils`.
- `chia-sdk-driver` already depends on both `chia-sdk-types` and `chia-sdk-utils` (via workspace `[dependencies]`). No new edges.

**One follow-on action:** add `chia-sdk-types = { workspace = true, optional = true }` to `chia-sdk-utils/Cargo.toml`, with the new dependency activated only by `chip-0057 = ["dep:chia-sdk-types/chip-0057"]`. This keeps the no-feature build of `chia-sdk-utils` dependency-free, matching how `chia-sdk-driver/Cargo.toml` gates `dep:sha3` under `chip-0037`.

### Feature-Flag Cascade

```toml
# Root Cargo.toml [features]
chip-0057 = [
    "chia-sdk-types/chip-0057",
    "chia-sdk-utils/chip-0057",
    "chia-sdk-driver/chip-0057",
]

# crates/chia-sdk-types/Cargo.toml [features]
chip-0057 = []                                   # pure-types, no extra deps

# crates/chia-sdk-utils/Cargo.toml [features]
chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057"]

# crates/chia-sdk-driver/Cargo.toml [features]
chip-0057 = ["chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]
```

This exactly mirrors the `chip-0035` / `chip-0037` patterns shown in the workspace root `Cargo.toml` lines 73–74. Bindings are always-on but re-export via `#[cfg(feature = "chip-0057")]` like other gated re-exports in `chia-sdk-bindings/src/lib.rs`.

## Action System Integration (SEND-04)

This is the load-bearing integration point. The `Spends` builder owns the entire XCH input set, computes change, and emits `CoinSpend`s through `StandardLayer`. `SilentPaymentSend` must hook into that flow without forking it.

### Trait Shape

```rust
// crates/chia-sdk-driver/src/actions/silent_payment_send.rs
use chia_protocol::Bytes32;
use chia_puzzle_types::Memos;

use crate::{
    Asset, Deltas, DriverError, Id, Output, SilentPaymentAddress,
    SpendAction, SpendContext, Spends,
};

#[derive(Debug, Clone)]
pub struct SilentPaymentSend {
    pub recipient: SilentPaymentAddress,    // already-parsed; binding facade decodes string
    pub amount: u64,
    pub memos: Memos,
}

impl SilentPaymentSend {
    pub fn new(recipient: SilentPaymentAddress, amount: u64, memos: Memos) -> Self {
        Self { recipient, amount, memos }
    }
}

impl SpendAction for SilentPaymentSend {
    fn calculate_delta(&self, deltas: &mut Deltas, _index: usize) {
        // Same shape as SendAction — XCH outflow.
        deltas.update(Id::Xch).output += self.amount;
        deltas.set_needed(Id::Xch);
    }

    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        _index: usize,
    ) -> Result<(), DriverError> {
        // 1. Resolve XCH source for this output.
        // 2. Compute aggregated_sender_sk from this transaction's wallet-controlled
        //    XCH inputs (via Spends::xch.items).
        // 3. Compute input_hash = tagged_hash("Chia_SP/Inputs",
        //                                    lex_min(coin_ids) || aggregated_sender_pk)
        // 4. Compute one-time puzzle hash = derive_one_time_puzzle_hash(
        //        scan_pk, spend_pk, aggregated_sender_sk, input_hash, k=0)
        //    For multiple SilentPaymentSends in the same Spends batch to the SAME
        //    scan_pk, k must increment — track this in a Spends-local helper. See
        //    "Multi-output coordination" below.
        // 5. Emit a CreateCoin condition on the source XCH coin with that puzzle hash.
        //    From here forward, this is just a regular send — Spends will collect
        //    a standard XCH CoinSpend on the source via StandardLayer.

        let one_time_puzzle_hash: Bytes32 = /* derive_one_time_puzzle_hash(...) */;

        // Then reuse the same primitive that SendAction uses: place a CreateCoin
        // on the appropriate XCH source via spends.xch.output_source(...).
        let output = Output::new(one_time_puzzle_hash, self.amount);
        let source = spends.xch.output_source(ctx, &output)?;
        let parent = &mut spends.xch.items[source];
        let parent_puzzle_hash = parent.asset.full_puzzle_hash();
        let create_coin = chia_sdk_types::conditions::CreateCoin::new(
            one_time_puzzle_hash, self.amount, self.memos,
        );
        parent.kind.create_coin_with_assertion(
            ctx, parent_puzzle_hash, &mut spends.xch.payment_assertions, create_coin,
        );

        spends.outputs.xch.push(chia_protocol::Coin::new(
            parent.asset.coin_id(), one_time_puzzle_hash, self.amount,
        ));
        Ok(())
    }
}
```

### When does `input_hash` get computed?

**During `SpendAction::spend()` (inside `Spends::apply`), not in `finish()`.** Justification:

1. The existing `SendAction::spend` already mutates `spends.xch.items` to add a `CreateCoin` against a specific XCH source. By the time `apply()` runs, all XCH inputs have already been added via `Spends::add(coin)`. The input set is stable.
2. `Spends::finish_with_keys` only collects spends from already-mutated state and signs — it does not give per-action hooks. Deferring to `finish()` would require either a new hook or a parallel "pending crypto operations" buffer, both of which fork the existing pattern.
3. The cost is one BLS scalar mul + SHA256 per `SilentPaymentSend` — negligible inside `apply()`.

The aggregated `sender_pk` used in `input_hash` must include **all** XCH inputs the wallet controls in this transaction, not just the one that funds the one specific silent-payment output. So the action queries `spends.xch.items` (all of them, filtered to non-ephemeral), aggregates their synthetic SKs/PKs, and uses the aggregate. **The aggregation requires synthetic SKs at action time, but `Spends` only has the synthetic *public* keys until `finish_with_keys` is called.** Resolution: the aggregation needs to happen one of two ways:

- **Option A (preferred):** Aggregate **public** keys at action time (using the synthetic PKs already curried into each XCH item's puzzle hash via `StandardArgs`). The `input_hash` only depends on the aggregated **public** key, so this works for `input_hash`. The aggregated **secret** key for the ECDH `shared_secret` then needs to be computed differently — either:
  - At signing time, by the wallet, before calling `Spends::finish_with_keys`. The wallet caller provides the aggregated synthetic SK as a `Spends`-level parameter (new `finish_with_keys` overload: `finish_with_silent_payment_keys`), OR
  - The wallet pre-aggregates the synthetic SK and the action takes it as a constructor parameter (`SilentPaymentSend::new_with_aggregate(addr, amount, memos, aggregated_sender_sk)`).

  **Recommendation:** Add a new `Spends::silent_payments` field — a `HashMap<SilentPaymentSendId, SilentPaymentDeferredCrypto>` — that the action populates with everything except the shared_secret derivation. Then `finish_with_keys` performs the ECDH at signing time using the aggregated synthetic SK constructed from the same `synthetic_keys: &IndexMap<Bytes32, PublicKey>` keyed lookup, plus a parallel `synthetic_secret_keys: &IndexMap<Bytes32, SecretKey>` parameter on a new `finish_with_silent_payment_keys` constructor variant.

- **Option B:** Make the action take a closure or pre-aggregated SK at construction time. Less idiomatic — caller has to know about the multi-input aggregation rule before calling `spends.add(...)`.

**Decision pending — flag in phase 4 ("Send-side integration") for design refinement after writing a first proof-of-concept test.** Both options are viable; option A keeps the action ergonomic (`spends.add(SilentPaymentSend::new(...))` with no extra args), at the cost of one extra method on `Spends`. This matches the precedent set by `finish_with_keys` already needing a key map.

### Multi-Output Coordination

When a single transaction contains multiple `SilentPaymentSend` actions targeting the **same** scan_pk, BIP-352 requires the per-output counter `k` to increment (0, 1, 2, ...). The action cannot independently know its `k` — it must consult shared state.

**Recommendation:** Add a `Spends::silent_payment_counters: HashMap<[u8; 48], u32>` (scan_pk → next k). `SilentPaymentSend::spend` reads-and-increments this map. Simple, no new types.

### Where does this hook into `Action`?

Extend the `Action` enum in `crates/chia-sdk-driver/src/action_system/action.rs` with a new variant:

```rust
#[derive(Debug, Clone)]
pub enum Action {
    Send(SendAction),
    Settle(SettleAction),
    // ... existing variants ...
    Fee(FeeAction),

    #[cfg(feature = "chip-0057")]
    SilentPaymentSend(SilentPaymentSend),
}

impl Action {
    #[cfg(feature = "chip-0057")]
    pub fn silent_payment_send(
        recipient: SilentPaymentAddress,
        amount: u64,
        memos: Memos,
    ) -> Self {
        Self::SilentPaymentSend(SilentPaymentSend::new(recipient, amount, memos))
    }
}

impl SpendAction for Action {
    fn calculate_delta(&self, deltas: &mut Deltas, index: usize) {
        match self {
            // ... existing arms ...
            #[cfg(feature = "chip-0057")]
            Action::SilentPaymentSend(a) => a.calculate_delta(deltas, index),
        }
    }
    fn spend(&self, ctx: &mut SpendContext, spends: &mut Spends, index: usize)
        -> Result<(), DriverError>
    {
        match self {
            // ... existing arms ...
            #[cfg(feature = "chip-0057")]
            Action::SilentPaymentSend(a) => a.spend(ctx, spends, index),
        }
    }
}
```

This is the only mutation of an existing enum. It's analogous to how `chip-0035` and `chip-0037` would extend `Layer` if they added new puzzle layers — gate by `#[cfg(feature = "chip-0057")]`. The enum is non-exhaustive in practice (no exterior matches outside the crate need updating, since callers go through `Action::send` / `Action::silent_payment_send` constructors).

## Receive-Side: `scan_from_tweaks` (RECV-01, RECV-02)

The receive primitive is purely a function — no `Spends` integration, no signer integration. It takes a `&TweakData` and returns `Vec<DetectedSpCoin>`. The caller takes those coins and feeds them into their own wallet's spend flow (using `StandardLayer::new(detected.onetime_sk.public_key())` to derive the puzzle for spending the detected coin).

### Data Shapes

```rust
// crates/chia-sdk-driver/src/silent_payments/tweak_data.rs

use chia_bls::PublicKey;
use chia_protocol::{Bytes32, Coin};

/// One block (or one tweak group) of indexer-provided data: tweak points and
/// the output coins to test against. Transport-agnostic.
#[derive(Debug, Clone)]
pub struct TweakData {
    pub tweak_points: Vec<PublicKey>,   // = input_hash_i * A_sum_i, one per tx group
    pub outputs: Vec<OutputMeta>,        // outputs in the block (or whatever scope)
}

/// Metadata about an on-chain output. Indexer-supplied; binding-friendly.
#[derive(Debug, Clone)]
pub struct OutputMeta {
    pub coin: Coin,                      // for downstream spending
    pub puzzle_hash: Bytes32,            // == coin.puzzle_hash, denormalized for hashing
    // (optional fields for hints/parents added in later phases)
}

/// A detected silent-payment output the recipient can spend.
#[derive(Debug, Clone)]
pub struct DetectedSpCoin {
    pub coin: Coin,
    pub onetime_sk: chia_bls::SecretKey,
    pub k: u32,
    pub label: Option<u32>,
}
```

### `scan_from_tweaks` Signature

```rust
pub fn scan_from_tweaks(
    scan_sk: &chia_bls::SecretKey,
    spend_sk: &chia_bls::SecretKey,
    spend_pk: &chia_bls::PublicKey,
    tweak_data: &TweakData,
    labels: Option<&LabelRegistry>,
) -> Vec<DetectedSpCoin>;
```

Each `tweak_point` already encodes `input_hash * A_sum` for its tx group — so the scanner only needs `scan_sk * tweak_point` per group, hashed to get the shared_secret, then iterates `k` deriving one-time puzzle hashes to match against `tweak_data.outputs`. Cheap.

### The `LabelRegistry` Type

This is the design decision flagged in topic 5. Recommendation: **a dedicated type, not a bare `HashMap`.** Rationale:

1. The label map needs **two directions**: when generating addresses, `m → (label_scalar, label_pk)`; when scanning, `label_pk → m`. A bare `HashMap<[u8; 48], u32>` exposes only one direction and forces callers to build their own forward map separately.
2. It's a state object with non-trivial invariants (label 0 reserved for "change"; m values are wallet-monotonic). Wrapping it lets us add validation and helpers (`register(scan_sk, m)`, `lookup(label_pk)`, `forward(m)`).
3. Binding generators (bindy-macro) handle named types with methods cleanly. Bare maps require per-target conversions.

```rust
// crates/chia-sdk-driver/src/silent_payments/labels.rs

#[derive(Debug, Default, Clone)]
pub struct LabelRegistry {
    forward: indexmap::IndexMap<u32, (ScalarField, chia_bls::PublicKey)>,
    reverse: std::collections::HashMap<[u8; 48], u32>,
}

impl LabelRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, scan_sk: &chia_bls::SecretKey, m: u32) -> chia_bls::PublicKey {
        let (label_scalar, label_pk) = generate_label(scan_sk, m);
        self.forward.insert(m, (label_scalar, label_pk));
        self.reverse.insert(label_pk.to_bytes(), m);
        label_pk
    }

    pub fn lookup(&self, label_pk: &chia_bls::PublicKey) -> Option<u32> {
        self.reverse.get(&label_pk.to_bytes()).copied()
    }

    pub fn iter_pks(&self) -> impl Iterator<Item = (u32, &chia_bls::PublicKey)> {
        self.forward.iter().map(|(m, (_, pk))| (*m, pk))
    }
}
```

`scan_from_tweaks` takes `Option<&LabelRegistry>`; `None` means unlabeled-only scanning.

## Bindings Descriptor Structure (BIND-01, BIND-02)

A new `bindings/silent_payments.json` file follows the patterns established by `bindings/coin.json` (class with fields + methods) and `bindings/address.json` (factory-decoded class).

### Sketch

```json
{
  "SilentPaymentAddress": {
    "type": "class",
    "new": true,
    "fields": {
      "scan_pk": "PublicKey",
      "spend_pk": "PublicKey",
      "prefix": "String"
    },
    "methods": {
      "encode": { "return": "String" },
      "decode": {
        "type": "factory",
        "args": { "address": "String" }
      }
    }
  },

  "SilentPaymentKeys": {
    "type": "class",
    "new": false,
    "methods": {
      "from_mnemonic": {
        "type": "factory",
        "args": { "mnemonic": "String" }
      },
      "scan_sk": { "return": "SecretKey" },
      "spend_sk": { "return": "SecretKey" },
      "scan_pk": { "return": "PublicKey" },
      "spend_pk": { "return": "PublicKey" },
      "unlabeled_address": {
        "args": { "prefix": "String" },
        "return": "SilentPaymentAddress"
      },
      "labeled_address": {
        "args": { "m": "u32", "prefix": "String" },
        "return": "SilentPaymentAddress"
      }
    }
  },

  "TweakData": {
    "type": "class",
    "new": true,
    "fields": {
      "tweak_points": "Vec<PublicKey>",
      "outputs": "Vec<OutputMeta>"
    }
  },

  "OutputMeta": {
    "type": "class",
    "new": true,
    "fields": {
      "coin": "Coin",
      "puzzle_hash": "Bytes32"
    }
  },

  "DetectedSpCoin": {
    "type": "class",
    "new": true,
    "fields": {
      "coin": "Coin",
      "onetime_sk": "SecretKey",
      "k": "u32",
      "label": "Option<u32>"
    }
  },

  "LabelRegistry": {
    "type": "class",
    "new": true,
    "methods": {
      "register": {
        "args": { "scan_sk": "SecretKey", "m": "u32" },
        "return": "PublicKey"
      },
      "lookup": {
        "args": { "label_pk": "PublicKey" },
        "return": "Option<u32>"
      }
    }
  },

  "SilentPayments": {
    "type": "static_functions",
    "functions": {
      "scan_from_tweaks": {
        "args": {
          "scan_sk": "SecretKey",
          "spend_sk": "SecretKey",
          "spend_pk": "PublicKey",
          "tweak_data": "TweakData",
          "labels": "Option<LabelRegistry>"
        },
        "return": "Vec<DetectedSpCoin>"
      },
      "derive_one_time_puzzle_hash": {
        "args": {
          "scan_pk": "PublicKey",
          "spend_pk": "PublicKey",
          "aggregated_sender_sk": "SecretKey",
          "input_hash": "Bytes32",
          "k": "u32"
        },
        "return": "Bytes32"
      },
      "compute_input_hash": {
        "args": {
          "coin_ids": "Vec<Bytes32>",
          "aggregated_sender_pk": "PublicKey"
        },
        "return": "Bytes32"
      },
      "aggregate_sender_sks": {
        "args": { "sks": "Vec<SecretKey>" },
        "return": "SecretKey"
      }
    }
  }
}
```

### How does `bindy-macro` handle method-bearing structs?

Verified against `bindings/coin.json` (which already has `Coin.coin_id()` and `SpendBundle.{to_bytes, from_bytes, hash}`): the macro supports `methods` on `type: "class"` declarations directly. `from_mnemonic` uses `"type": "factory"` (matching `SpendBundle::from_bytes` and `Address::decode`), which generates an associated-function constructor in each target language. Default-construction via `"new": false` means the type is not directly constructable from fields; only the named factory + methods are exposed. This matches `SpendBundle`'s pattern.

**The current schema doesn't have a `"type": "static_functions"` precedent.** Looking at how existing free-function APIs are exposed (e.g., `bindings/mnemonic.json`), they're attached as factories or methods on a related class. Recommendation: hang the scan/derive/aggregate free functions off a zero-field `SilentPayments` class with `"new": false` and `"methods"` only — same pattern as `Bech32`. If that pattern doesn't yet exist for static-only classes, the alternative is to attach each function to the most relevant class:
- `scan_from_tweaks` → on `TweakData` as a method
- `derive_one_time_puzzle_hash` → on `SilentPaymentAddress` as a method (takes sender SK + input_hash + k)
- `compute_input_hash` → standalone factory on a small helper class

**Action item for phase 5:** before implementing bindings, read `crates/chia-sdk-bindings/bindy-macro/src/` to confirm the exact schema for static-only types. If the macro doesn't natively support it, fall back to attaching methods to existing types.

`SilentPaymentSend` itself is not in the JSON descriptor — it's an `Action` variant, and the existing `bindings/action_system.json` already exposes the `Action` enum + constructors. Add a single `silent_payment_send` factory there, gated behind `chip-0057` in the binding facade.

### Bindings Facade in Rust

```rust
// crates/chia-sdk-bindings/src/silent_payments.rs

#[cfg(feature = "chip-0057")]
pub use chia_sdk_driver::silent_payments::{
    DetectedSpCoin, LabelRegistry, OutputMeta, TweakData,
    aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash,
    scan_from_tweaks,
};
#[cfg(feature = "chip-0057")]
pub use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentKeys};
```

And in `crates/chia-sdk-bindings/src/lib.rs`:

```rust
#[cfg(feature = "chip-0057")]
mod silent_payments;
#[cfg(feature = "chip-0057")]
pub use silent_payments::*;
```

The binding crates (`napi/`, `pyo3/`, `wasm/`) need their `Cargo.toml` `[features]` to enable `chip-0057` on `chia-sdk-bindings`. Per BIND-01, "Bindings always-on" — recommendation is the binding crates default-enable `chip-0057`. Verify with the maintainers whether bindings always ship the kitchen sink (`--all-features` build) or are feature-gated like the workspace.

## Data Flow: `Spends::apply([Action::silent_payment_send(...)])` Through `finalize()`

```
1. caller:
     spends = Spends::new(my_change_puzzle_hash);
     spends.add(my_xch_coin_1);
     spends.add(my_xch_coin_2);

2. caller:
     deltas = spends.apply(&mut ctx, &[
         Action::silent_payment_send(recipient_addr, 100, Memos::None),
     ])?;

3. inside Spends::apply():
     for action in actions { action.spend(ctx, self, index)?; }

4. inside SilentPaymentSend::spend():
     a. peek at spends.xch.items to collect all wallet-controlled XCH coin_ids
        and synthetic PKs.
     b. read/increment spends.silent_payment_counters[scan_pk] -> k.
     c. compute aggregated_sender_pk = sum(synthetic_pks of all xch items).
     d. compute input_hash = tagged_hash("Chia_SP/Inputs",
                            lex_min(coin_ids) || aggregated_sender_pk).
     e. defer ECDH: record DeferredSpCrypto {
            scan_pk, spend_pk, input_hash, k, amount, memos,
        } into spends.silent_payments_pending.
     f. pick a placeholder XCH source via spends.xch.output_source().
        We CAN compute the one-time puzzle hash here at construction time IF
        we have the aggregated sender SK — option A (defer to finish) leaves a
        placeholder. Alternative: option B requires SK at action construction.
        DECISION DEFERRED to phase 4 prototype.

5. caller:
     outputs = spends.finish_with_silent_payment_keys(
         &mut ctx, &deltas, Relation::None,
         &synthetic_pks, &synthetic_sks,
     )?;

6. inside finish_with_silent_payment_keys():
     a. resolve pending DeferredSpCrypto entries:
          aggregated_sender_sk = sum(synthetic_sks looked up by p2 puzzle hash)
          shared_secret = SHA256(aggregated_sender_sk * input_hash * scan_pk)
          tweak = tagged_hash("Chia_SP/SharedSecret", shared_secret || k) mod r
          one_time_pk = spend_pk + tweak * G
          one_time_puzzle_hash = StandardArgs::curry_tree_hash(one_time_pk.derive_synthetic())
     b. mutate the recorded CreateCoin condition's puzzle_hash field to the
        computed value (the placeholder is overwritten).
     c. run the standard finish_with_keys path:
          each XCH item → StandardLayer::new(synthetic_key).spend_with_conditions
                          → SpendContext::spend → CoinSpend collected.
     d. ctx.take() returns the CoinSpends; caller signs and broadcasts.

7. on chain:
     Output coin lands at one_time_puzzle_hash. To an observer,
     indistinguishable from a normal XCH send.
```

**Why defer the placeholder-fixup to `finish_with_silent_payment_keys` and not do it inline?** Because the recipient's puzzle hash depends on `aggregated_sender_sk`, which we don't have during `apply()`. The cleanest split: action-time records the deterministic bits (scan_pk, spend_pk, input_hash, k, amount, memos), finish-time computes the ECDH using the same key map that signing already needs. One pass through the pending list, no extra signer changes.

**Alternative considered (Option B):** Make the caller provide the aggregated synthetic SK upfront when constructing `SilentPaymentSend`. Rejected because it (a) exposes the multi-input aggregation rule to the caller, (b) requires the caller to know all XCH inputs *before* adding them to `Spends`, contradicting the builder's normal flow.

## Simulator Test Architecture (SIM-01, SIM-02, SIM-03)

The SIM-01 helper extracts `TweakData` from a simulated block by:
1. Iterating coin spends in the block.
2. Filtering to standard-puzzle spends (parse with `StandardLayer::parse_puzzle`).
3. Extracting the curried synthetic pubkey from each.
4. Grouping spends that share a coinbase (same transaction).
5. Per-group: computing `A_sum = Σ synthetic_pk_i`, `input_hash = tagged_hash(...)`, `tweak_point = input_hash * A_sum`.
6. Collecting `OutputMeta` for all created coins in the block.

### Placement: `chia-sdk-test` or `chia-sdk-driver`'s `#[cfg(test)]`?

**Recommendation: `chia-sdk-test`, behind a `chip-0057` feature on that crate.** Rationale:

1. The helper is reusable — both the driver crate's own integration tests AND the bindings test suites (`napi/__test__`, `pyo3/tests`, `wasm/__test__`) will want to construct `TweakData` from a simulator block to validate round-trip detection. Putting it in `#[cfg(test)]` inside `chia-sdk-driver` makes it inaccessible to the binding tests, which compile against the public API.
2. `chia-sdk-test` already houses `Simulator` plus `BlsPair`, `BlsPairWithCoin`, etc. — fixture-style helpers belong here. Adding `chia-sdk-test::silent_payments::extract_tweak_data(&Simulator, height)` (or similar) fits naturally.
3. The cost is a new `chip-0057` feature on `chia-sdk-test`. Negligible; consistent with how `peer-simulator` is already gated.

**Sketch:**

```rust
// crates/chia-sdk-test/src/silent_payments.rs   (chip-0057 gated)

use chia_protocol::{Bytes32, CoinSpend};
use chia_sdk_driver::{Layer, Puzzle, SpendContext, StandardLayer};
use chia_sdk_driver::silent_payments::{TweakData, OutputMeta, compute_input_hash};

use crate::Simulator;

/// Build a TweakData from every standard-puzzle spend at `height` in the simulator.
pub fn extract_tweak_data_from_block(
    sim: &Simulator,
    height: u32,
) -> Result<TweakData, anyhow::Error> {
    // 1. coin_spends_at(height) → Vec<CoinSpend>
    // 2. parse each puzzle as StandardLayer; collect synthetic_pks per tx group
    // 3. for each group: input_hash, A_sum, tweak_point = input_hash * A_sum
    // 4. collect OutputMeta for all created coins
    // ... implementation ...
    todo!()
}
```

Verified that `chia-sdk-test::Simulator` is reachable as a downstream dep — driver crate tests already import it via `use chia_sdk_test::Simulator;` in `cat.rs:530`, `did.rs:311`, `bulletin.rs:119`, etc.

## Recommended Build Order

```
Phase 1: Crypto primitives (types crate)
  ├── ScalarField with TV-vector unit tests (CRYPTO-01, partial CRYPTO-03)
  ├── tagged_hash + Chia_SP/* tag constants (CRYPTO-02)
  ├── SCAN_PATH / SPEND_PATH constants
  └── feature flag cascade plumbing (chip-0057 in types Cargo.toml)
  PROVES: math is right, no driver entanglement.

Phase 2: Address & key types (utils crate)
  ├── SilentPaymentKeys::from_mnemonic with TV-vector tests (ADDR-01)
  ├── SilentPaymentAddress encode/decode with bech32m TV (ADDR-02)
  ├── SilentPaymentKeys::labeled_address (ADDR-03)
  └── feature flag plumbing in utils
  DEPENDS ON: Phase 1 (ScalarField for label_scalar, tagged_hash for label tag).
  PROVES: address layer works, mnemonic path is correct.

Phase 3: Receive primitive (driver crate, no Spends touch yet)
  ├── ecdh.rs (sender + scanner shared secret, compute_input_hash) — TV1 tests
  ├── protocol.rs (derive_output_tweak, derive_onetime_pk, derive_onetime_sk) — TV1
  ├── aggregate.rs (aggregate_sender_sks) — TV4
  ├── labels.rs (generate_label, LabelRegistry) — TV3
  ├── tweak_data.rs (TweakData, DetectedSpCoin, OutputMeta — pure data)
  └── scan_from_tweaks (RECV-01, RECV-02, RECV-03, RECV-04)
  DEPENDS ON: Phase 1 (ScalarField, tagged_hash), Phase 2 (only for tests that
              materialize addresses).
  PROVES: end-to-end scan against synthesized TweakData works.

Phase 4: Send-side action (driver crate)
  ├── derive_one_time_puzzle_hash (SEND-01) — pure helper composing Phase 3
  ├── SilentPaymentSend struct + SpendAction impl (SEND-04)
  ├── Action::SilentPaymentSend enum variant (chip-0057 gated)
  ├── Spends-level state: silent_payment_counters + silent_payments_pending
  ├── Spends::finish_with_silent_payment_keys (or inline into existing finish_with_keys with optional SK map)
  └── unit tests using a hand-built scenario (no simulator yet)
  DEPENDS ON: Phase 3 (ECDH + protocol functions).
  PROVES: the action composes with Spends and produces a valid XCH CoinSpend with
          the correct one-time puzzle hash.

Phase 5: Bindings (chia-sdk-bindings + bindings/silent_payments.json + per-target shims)
  ├── chia-sdk-bindings/src/silent_payments.rs re-exports (BIND-01, BIND-02)
  ├── bindings/silent_payments.json descriptor
  ├── Action::silent_payment_send entry in bindings/action_system.json
  ├── per-target shims if static_functions pattern needs help
  └── verify napi/pyo3/wasm all compile
  DEPENDS ON: Phases 2 + 3 + 4 (everything that gets exposed).
  PROVES: cross-language wallets can call the full API.

Phase 6: Simulator round-trip (chia-sdk-test + driver integration tests)
  ├── chia-sdk-test::silent_payments::extract_tweak_data_from_block (SIM-01)
  ├── unlabeled E2E test: send → farm → extract → scan → spend (SIM-02)
  ├── labeled E2E test: same with labeled address (SIM-03)
  └── AVA/pytest tests in napi/pyo3 mirroring the Rust E2E (BIND-03)
  DEPENDS ON: Phases 4 + 5.
  PROVES: the full real-on-chain flow works end-to-end.

Phase 7: Example + docs
  ├── examples/silent_payment.rs (EX-01) — mirrors examples/cat_spends.rs
  └── crate-level docs in chia-sdk-driver/docs.md mentioning the chip-0057 module
  DEPENDS ON: Phase 6 (proven flow).
```

**Critical-path commentary:**

- **Phases 1–3 are parallelizable** with Phases 4 only after Phase 3 stabilizes. The bottleneck for shipping is Phase 4 (action integration), because the design decision deferred above (Option A vs B for sender-SK timing) might require one iteration after a prototype.
- **Why not do bindings first?** The bindings facade is mechanical re-export; it must follow the Rust API or it'll churn. Same reason `chip-0037` shipped Rust before the bindings descriptor.
- **Why simulator AFTER bindings?** SIM tests exercise the public API and the bindings tests need the JSON descriptor in place. Bindings can be a small ahead-of-simulator step since they don't require sim test pass.

## Re-Export Discipline (Prelude)

The current `src/prelude.rs` exposes the high-frequency types: `Action`, `Cat`, `Did`, `Nft`, `SpendContext`, `Spends`, `StandardLayer`, `Address`, `Bech32`, etc. The bar for inclusion is "a typical wallet author writes this type name in their first 50 lines of code."

**Recommended prelude additions (chip-0057 gated):**

```rust
// In src/prelude.rs, append:
#[cfg(feature = "chip-0057")]
pub use chia_sdk_driver::silent_payments::{
    DetectedSpCoin, LabelRegistry, OutputMeta, SilentPaymentSend, TweakData,
    derive_one_time_puzzle_hash, scan_from_tweaks,
};
#[cfg(feature = "chip-0057")]
pub use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentKeys};
```

**Deliberately omitted from the prelude:**
- `ScalarField`, `tagged_hash`, `CHIA_SP_*` tag constants — internal crypto, callers shouldn't reach for them.
- `generate_label`, `aggregate_sender_sks`, `compute_input_hash` — internal; `SilentPaymentSend` and `scan_from_tweaks` handle these.
- `SCAN_PATH` / `SPEND_PATH` — internal to `SilentPaymentKeys::from_mnemonic`.

This matches the existing prelude philosophy: `Cat` is in (callers say "I have a Cat"), but `CatLayer` is in too (callers compose), while `CatInfo` is in but raw puzzle types from `chia-puzzle-types` are not.

## Anti-Patterns

### Anti-Pattern 1: Mixing `ScalarField` with `mod_by_group_order`

**What people do:** Reuse Chia's existing signed `mod_by_group_order` helper (used for synthetic-key offsets) for `input_hash` or `output_tweak` reduction.
**Why it's wrong:** The existing helper treats the 32-byte input as signed (a negative high bit subtracts r); CHIP-0057 protocol scalars require unsigned reduction. Silent breakage: detection fails for ~50% of payments.
**Do this instead:** Always go through `ScalarField::from_bytes_unsigned`. The newtype makes the boundary explicit at compile time.

### Anti-Pattern 2: Free-standing `aggregate_sender_sks` call in user code

**What people do:** Re-export `aggregate_sender_sks` as a public utility and expect wallet authors to call it directly before constructing a `SilentPaymentSend`.
**Why it's wrong:** Forces the caller to know the multi-input aggregation rule, leaves room for silently using a single-input synthetic SK in a multi-input transaction (which produces undetectable payments — see PROJECT.md `aggregate_sender_sks` Out-of-Scope discussion).
**Do this instead:** Make `Spends::finish_with_silent_payment_keys` perform aggregation internally over the SK map it already gets. Hide `aggregate_sender_sks` behind a less prominent module path (`chia_sdk_driver::silent_payments::aggregate`) and don't put it in the prelude.

### Anti-Pattern 3: Computing `input_hash` per `SilentPaymentSend` action

**What people do:** Each action independently computes its own `input_hash` and shared_secret using only the inputs it touches.
**Why it's wrong:** `input_hash` is a *transaction-level* primitive — it depends on **all** inputs in the bundle, not just the ones funding this specific output. A 2-input/2-output silent-payment tx where each output sees only one input would produce undetectable payments.
**Do this instead:** `Spends` is the natural transaction boundary. Always derive `input_hash` from `spends.xch.items.all_coin_ids()` and `aggregate_sender_sk_for(spends.xch.items)`. The action's responsibility is recording the recipient + amount + memos; the aggregation belongs at the `Spends` level.

### Anti-Pattern 4: Asserting `aggregate_sender_sks` works for partial control (offers, multi-party)

**What people do:** Use `SilentPaymentSend` inside an offer settlement or PSBT-like multi-party flow without warning that aggregation requires full control of all inputs.
**Why it's wrong:** A counterparty's synthetic SK is unknown at action time; aggregation produces wrong `sender_sk`, and the recipient cannot detect the payment.
**Do this instead:** v1 explicitly rejects this. Phase-4 implementation should `Err(DriverError::SilentPaymentMultiPartyUnsupported)` when an offer's settlement spend coexists in the same `Spends` as a `SilentPaymentSend`. Multi-party support is deferred to v2 (`SilentPaymentTweakSource` async trait + sign-time aggregation).

### Anti-Pattern 5: Pre-deriving the on-chain coin id in `SilentPaymentSend::spend()`

**What people do:** Try to construct a fully-formed `Coin` (parent_coin_id, puzzle_hash, amount) during action-time, including the parent's eventual coin_id.
**Why it's wrong:** The parent XCH coin's identity is stable, but the *output* coin's puzzle hash isn't computable until `finish_with_silent_payment_keys` resolves the deferred ECDH. Premature `Coin` construction with a placeholder puzzle hash leaks into `spends.outputs.xch` and downstream callers see a coin id that doesn't match the on-chain reality.
**Do this instead:** Either (a) defer `spends.outputs.xch.push(...)` to the finish step alongside the puzzle-hash fixup, or (b) make `Outputs::xch` slot a `Coin` *after* finish completes. Recommendation: option (a), since `Outputs` is currently constructed during apply and consumed after finish.

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `silent_payments::scan_from_tweaks` ↔ external indexer (`sp-service` today, CHIP-0058 client later) | Caller constructs `TweakData` from any transport. No async traits, no I/O in the SDK. | The CHIP-0058 transport client will become a separate crate (`chia-sdk-sp-client` or similar). Until then, `TweakData` is the contract. Keep it minimal and forward-compatible. |
| `SilentPaymentSend` action ↔ `Spends` builder | Action mutates `spends.xch.items` and writes to `spends.silent_payments_pending` (new field). Finish-time resolves pending entries using the same key map signing uses. | One new field on `Spends`. Encapsulated in driver crate; no leak to callers. |
| `chia-sdk-bindings::silent_payments` ↔ Rust core | `pub use` re-exports under `#[cfg(feature = "chip-0057")]`. Static functions hung off zero-field facade class. | Verify bindy-macro static-functions pattern before commit; if not supported, distribute methods onto existing classes (TweakData, SilentPaymentAddress). |
| `chia-sdk-test::silent_payments::extract_tweak_data_from_block` ↔ `Simulator` | Test helper queries simulator block state + parses puzzles with `StandardLayer::parse_puzzle`. | Lives in chia-sdk-test under a `chip-0057` feature to keep it reachable from binding-side test code without bloating production builds. |

### External Services

None. All silent-payments code is in-process. The "external service" (a CHIP-0058 indexer) is explicitly out of scope; the SDK only consumes its eventual output via `TweakData`.

## Sources

- `/home/kdc/chia-wallet-sdk/.planning/PROJECT.md` — locked scope, constraints, key decisions
- `/home/kdc/chia-wallet-sdk/.planning/codebase/ARCHITECTURE.md` — existing layered SDK architecture
- `/home/kdc/chia-wallet-sdk/.planning/codebase/STRUCTURE.md` — directory layout, feature-gating pattern, module barrel pattern
- `/home/kdc/chia-wallet-sdk/.planning/codebase/CONVENTIONS.md` — feature-gate cascade, lint policy, type-naming
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/lib.rs` — driver module exports
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/spend_context.rs` — SpendContext shape and `spend()` flow
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/action.rs` — `Action` enum + `SpendAction` trait
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/spends.rs` — `Spends` builder, `apply`/`finish_with_keys` flow
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/actions/send.rs` — reference `SpendAction` impl pattern
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/layers/standard_layer.rs` — how `StandardLayer` is consumed in `finish_with_keys`
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/primitives/cat.rs` — primitive directory/file layout precedent
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/src/{lib,bech32}.rs` — existing `Address`/`Bech32` patterns and crate scope
- `/home/kdc/chia-wallet-sdk/bindings/{coin,address}.json` — bindy-macro JSON schema for classes with methods and factories
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/lib.rs` — facade re-export pattern
- `/home/kdc/chia-wallet-sdk/Cargo.toml` — workspace features `chip-0035` / `chip-0037` cascade
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/Cargo.toml` — per-crate feature cascade `chip-0037 = ["chia-sdk-types/chip-0037", "dep:sha3"]`
- `/home/kdc/silent-payments/crates/sp-common/src/{lib,protocol,ecdh,scalar,tagged_hash,keys,puzzle}.rs` — reference implementation source for the algorithms being folded in
- `/home/kdc/chia-wallet-sdk/src/prelude.rs` — existing prelude composition for inclusion criteria

---
*Architecture research for: CHIP-0057 silent payments integration into chia-wallet-sdk*
*Researched: 2026-05-15*
