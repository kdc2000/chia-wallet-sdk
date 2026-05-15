# Codebase Structure

**Analysis Date:** 2026-05-15

## Directory Layout

```
chia-wallet-sdk/
├── src/                           # Umbrella crate (chia-wallet-sdk)
│   ├── lib.rs                    # Re-exports workspace crates as short names
│   └── prelude.rs                # Curated public surface (types + utilities)
├── crates/                       # 10 member crates
│   ├── chia-sdk-driver/          # Core SDK (layers, primitives, SpendContext)
│   ├── chia-sdk-types/           # Types, conditions, puzzles, constants
│   ├── chia-sdk-utils/           # Address, coin selection, hex parsing
│   ├── chia-sdk-client/          # Light-wallet protocol peer
│   ├── chia-sdk-coinset/         # Full-node REST client
│   ├── chia-sdk-daemon/          # Chia daemon WebSocket client
│   ├── chia-sdk-signer/          # Signature computation
│   ├── chia-sdk-test/            # Simulator, fixtures, test peer
│   ├── chia-sdk-cli/             # CLI tool
│   ├── chia-sdk-bindings/        # Rust facades + bindy machinery
│   │   ├── bindy/                # Runtime conversion traits
│   │   └── bindy-macro/          # Code generation from bindings.json
│   └── chia-sdk-types/derive/    # Derive macros for types
├── napi/                         # Node.js binding crate (NAPI-RS)
│   ├── src/lib.rs               # Entry point (invokes bindy_napi!)
│   ├── __test__/                 # AVA test suite
│   ├── Cargo.toml               # napi-rs, napi-derive
│   ├── package.json             # pnpm
│   └── index.d.ts / index.js    # Generated TypeScript definitions / JS API
├── pyo3/                         # Python binding crate (PyO3)
│   ├── src/lib.rs               # Entry point (invokes bindy_pyo3!)
│   ├── tests/                    # pytest suite
│   ├── stub-generator/           # Generates .pyi stub file
│   ├── Cargo.toml               # pyo3, maturin
│   ├── pyproject.toml           # maturin config
│   └── chia_wallet_sdk.pyi      # Generated type stubs
├── wasm/                         # WebAssembly binding crate (wasm-bindgen)
│   ├── src/lib.rs               # Entry point (invokes bindy_wasm!)
│   ├── __test__/                 # AVA test suite
│   ├── Cargo.toml               # wasm-bindgen
│   ├── package.json             # pnpm
│   └── tsconfig.json
├── bindings/                      # Type mapping descriptors (per-concept JSONs)
│   ├── action_system.json
│   ├── address.json
│   ├── bls.json
│   ├── clear_signing.json
│   ├── clvm.json
│   ├── clvm_types.json
│   ├── coin.json
│   ├── conditions.json
│   ├── constants.json
│   ├── mips.json
│   ├── mnemonic.json
│   ├── offer.json
│   ├── peer.json
│   ├── program.json
│   ├── puzzles.json
│   ├── reward_distributor.json
│   ├── rpc.json
│   ├── secp.json
│   ├── simulator.json
│   └── utils.json
├── bindings.json                  # Top-level type-group mappings (napi, pyo3, wasm)
├── examples/                      # Runnable Rust examples
│   ├── address_conversion.rs
│   ├── cat_spends.rs
│   ├── custom_p2_puzzle.rs
│   └── spend_simulator.rs
├── .github/workflows/             # CI/CD
│   ├── rust.yml                  # Rust build, test, clippy, fmt
│   ├── napi.yml                  # NAPI-RS build, test
│   ├── pyo3.yml                  # PyO3 build, test
│   └── wasm.yml                  # WASM build, test
├── Cargo.toml                     # Workspace manifest (features, workspace deps)
├── Cargo.lock                     # Dependency lock
├── rust-toolchain.toml            # Pinned to 1.90.0
└── CLAUDE.md                      # Architecture guidance for Claude
```

## Crates Directory Purposes

### `chia-sdk-driver`
**Location:** `crates/chia-sdk-driver/`

The core SDK. Contains layers, primitives, SpendContext, action system, and offers.

- `src/lib.rs`: Module declarations and public re-exports
- `src/layer.rs`: `Layer` trait definition
- `src/layers/`: Individual layer implementations
- `src/primitives/`: Primitive coin wrappers (Cat, Did, Nft, Singleton, etc.)
- `src/action_system/`: High-level spend builder
- `src/actions/`: Action implementations (create_did, mint_nft, etc.)
- `src/offers/`: Offer construction and settlement
- `src/spend_context.rs`: Allocator cache + coin-spend batch collection
- `src/spend.rs`: Puzzle + solution pair
- `src/spend_with_conditions.rs`: Spend + condition enforcement
- `src/puzzle.rs`: Puzzle parsing helpers
- `src/driver_error.rs`: Error enumeration
- `src/clear_signing.rs`: Message signing utilities
- `src/hashed_ptr.rs`: Allocator pointer hashing
- `docs.md`: Rendered as crate-level doc

### `chia-sdk-types`
**Location:** `crates/chia-sdk-types/`

Type definitions, condition types, puzzle modules, constants.

- `src/lib.rs`: Module declarations
- `src/condition/`: Condition type definitions
- `src/puzzles/`: CLVM puzzle definitions (raw, imported from upstream)
  - `puzzles.rs`: Module root with feature-gated imports
  - `p2_singleton.rs`, `p2_delegated_conditions.rs`, etc.: Non-gated puzzles
  - `datalayer/`: CHIP-0035 puzzles (feature-gated)
  - `p2_eip712_message.rs`, `p2_controller_puzzle.rs`: CHIP-0037 puzzles (feature-gated)
  - `action_layer/`: Action-layer metacontracts (feature-gated)
- `src/constants.rs`: MAINNET/TESTNET constants
- `src/merkle.rs`: Merkle tree implementation
- `derive/`: Derive macro crate for streamable types

### `chia-sdk-utils`
**Location:** `crates/chia-sdk-utils/src/`

Utility functions and address types.

- `bech32.rs`: `Bech32`, `Address` types (decode/encode with tree-hash)
- `coin_selection.rs`: Coin selection algorithm
- `hex.rs`: Hex string parsing helpers
- `lib.rs`: Module root

### `chia-sdk-client`
**Location:** `crates/chia-sdk-client/src/`

Light-wallet peer protocol client.

- Peer connection, options, TLS (native-tls or rustls feature)

### `chia-sdk-coinset`
**Location:** `crates/chia-sdk-coinset/src/`

Full-node REST client (coinset endpoints).

- Cursor-based pagination for large result sets
- RPC types and responses

### `chia-sdk-daemon`
**Location:** `crates/chia-sdk-daemon/src/`

Chia daemon WebSocket client (newer addition).

### `chia-sdk-signer`
**Location:** `crates/chia-sdk-signer/src/`

Required signature computation (BLS, SECP256K1, SECP256R1).

- `RequiredBlsSignature`, `RequiredSecpSignature`, `RequiredSignature` types
- `AggSigConstants`

### `chia-sdk-test`
**Location:** `crates/chia-sdk-test/src/`

Testing utilities: `Simulator` (in-process coin spend validator), key-pair fixtures, optional in-process peer.

- `Simulator`: Full-node simulator for testing
- Key pairs: `BlsPair`, `BlsPairWithCoin`, `K1Pair`, `R1Pair`
- `peer-simulator` feature: In-process peer implementation

### `chia-sdk-cli`
**Location:** `crates/chia-sdk-cli/src/`

Command-line interface built on the SDK.

### `chia-sdk-bindings`
**Location:** `crates/chia-sdk-bindings/src/`

Rust facades (one file per concept) that the bindy macro exposes to all three language bindings.

- `lib.rs`: Module root, clippy allowances for generated code
- `action_layer.rs`: Action-layer types and operations
- `action_system.rs`: Spends, Outputs, Deltas, Relation builders
- `address.rs`: Address, Bech32 encoding/decoding
- `bls.rs`: BLS key pairs, signatures
- `clear_signing.rs`: Message signing
- `clvm.rs`: Allocator, Program, tree hashing
- `clvm_types.rs`: Pair, Atom, other CLVM primitives
- `coin.rs`: Coin, CoinSpend, CoinState
- `conditions.rs`: All condition types
- `constants.rs`: MAINNET/TESTNET constants
- `key_pairs.rs`: All key pair types
- `mips.rs`: MIPS vault memo parsing
- `mnemonic.rs`: BIP39 mnemonic generation
- `offer.rs`: Offer, RequestedPayments, OfferAmounts
- `peer.rs`: Peer (gated by napi/pyo3 features)
- `program.rs`: Program currying, serialization
- `puzzle.rs`: Puzzle types and parsing
- `rpc.rs`: RPC client and all response types
- `secp.rs`: SECP key pairs, signatures
- `simulator.rs`: Simulator, SimulatorConfig
- `utils.rs`: Address, coin selection

### `bindy` & `bindy-macro`
**Location:** `crates/chia-sdk-bindings/{bindy,bindy-macro}/`

- `bindy`: Runtime trait implementations (`FromRust`, `IntoRust`, `NapiParamContext`, `Pyo3Context`, `WasmContext`)
- `bindy-macro`: Procedural macro that reads `bindings.json` and generates foreign function bindings at compile-time

---

## Driver Source Structure in Detail

### `crates/chia-sdk-driver/src/layers/`

**Purpose:** Layer implementations (composable puzzle slices).

**Key files:**
- `standard_layer.rs` — P2 ownership (basic or delegated)
- `cat_layer.rs` — CAT token (asset_id + inner puzzle)
- `did_layer.rs` — DID identity layer
- `nft_ownership_layer.rs`, `nft_state_layer.rs` — NFT layers (ownership + metadata)
- `singleton_layer.rs` — Singleton state machine
- `p2_singleton_layer.rs` — P2 enforcing singleton
- `p2_delegated_conditions_layer.rs` — P2 with delegated inner puzzle
- `p2_eip712_message_layer.rs` — EIP-712 message signing (chip-0037)
- `p2_controller_puzzle_layer.rs` — Controller puzzle (chip-0037)
- `settlement_layer.rs` — Offer settlement
- `revocation_layer.rs` — Revocation support
- `royalty_transfer_layer.rs` — Royalty enforcement
- `bulletin_layer.rs` — Bulletin board messages
- `option_contract_layer.rs` — Options contract
- `streaming_layer.rs` — Streaming assets
- `clawback_v2_layer.rs` — Clawback state
- `augmented_condition_layer.rs` — Augmented condition handling
- `datalayer/` — Data layer sub-layers (chip-0035)
  - `delegation_layer.rs`, `oracle_layer.rs`, `writer_layer.rs`
- `action_layer/` — Action-layer metacontract layers (action-layer feature)
  - `action_layer.rs` — Main action layer trait
  - `conditions_layer.rs`, `m_of_n_layer.rs`, `precommit_layer.rs`, etc.
  - `actions/` — Specific action implementations (catalog, reward distributor, xchandles)
- `p2_curried_layer.rs` — Generic curried puzzle parsing
- `p2_one_of_many_layer.rs` — M-of-N puzzle selection
- `p2_parent.rs` — Parent coin reference
- `augmented_condition_layer.rs` — Augmented condition support

### `crates/chia-sdk-driver/src/primitives/`

**Purpose:** Primitive coin wrappers (parse coin info, provide spend API).

**Directory pattern:** Each primitive typically has:
- `X.rs` — Main module exporting struct + parsing logic
- `X/x_info.rs` — Info type (parsed puzzle state)
- `X/x_launcher.rs` — Launcher type (for creation)
- `X/x_spend.rs` — Spend implementation

**Primitives:**
- `cat/` — CAT token
  - `cat.rs`, `cat_info.rs`, `cat_spend.rs`, `single_cat_spend.rs`
- `did/` — DID identity
  - `did.rs`, `did_info.rs`, `did_launcher.rs`
- `nft/` — NFT with metadata
  - `nft.rs`, `nft_info.rs`, `nft_launcher.rs`, `nft_mint.rs`, `metadata_update.rs`
- `singleton.rs` — Singleton state machine
- `vault/` — MIPS vault (chip-0035)
  - `vault.rs`, `vault_info.rs`, `vault_launcher.rs`
- `mips/` — MIPS family (chip-0035)
  - `mips.rs`, `mips_spend.rs`, `mips_spend_kind.rs`, `m_of_n.rs`, `restriction.rs`
  - `memo/` — Memo parsing (parsed_member.rs, parsed_restriction.rs, parsed_wrapper.rs)
- `option/` — Options contract
  - `option.rs`, `option_info.rs`, `option_launcher.rs`, `option_contract.rs`
  - `option_metadata.rs`, `option_underlying.rs`
- `datalayer/` — Data layer (chip-0035)
  - `datalayer.rs`, `datastore_info.rs`, `datastore_launcher.rs`, `datastore.rs`
- `streamed_asset.rs` — Streaming asset
- `bulletin.rs` — Bulletin board message
- `clawback.rs`, `clawback_v2.rs` — Clawback patterns
- `launcher.rs`, `intermediate_launcher.rs` — Coin creation
- `p2_parent_coin.rs` — Parent coin reference (action-layer)
- `action_layer/` — Action-layer primitives (action-layer feature)
  - `action_layer.rs` — Main types
  - `catalog_registry.rs`, `reward_distributor.rs`, `state_scheduler.rs`, `verification.rs`
  - Subdirectories for each action type

### `crates/chia-sdk-driver/src/action_system/`

**Purpose:** High-level spend builder (prefer over manual assembly).

**Key files:**
- `action.rs` — `Action` trait and high-level operations
- `spends.rs` — `Spends` builder that collects actions and produces `SpendBundle`
- `spend_kind.rs` — Spend categorization (conditions vs. settlement)
- `asset.rs` — Asset identity and tracking
- `id.rs` — Unique ID generation
- `output.rs` — Output coins from action
- `deltas.rs` — State changes (fees, asset flows)
- `relation.rs` — Links between actions
- `spendable_asset.rs`, `fungible_spends.rs`, `singleton_spends.rs` — Asset-specific strategies

### `crates/chia-sdk-driver/src/actions/`

**Purpose:** Concrete action implementations.

**Files:**
- `create_did.rs` — Create DID coin
- `update_did.rs` — Update DID state
- `mint_nft.rs` — Mint NFT
- `update_nft.rs` — Update NFT metadata
- `issue_cat.rs` — Issue CAT token
- `run_tail.rs` — Run CAT TAIL program
- `send.rs` — Send XCH/tokens
- `settle.rs` — Settle offer
- `mint_option.rs` — Create option contract
- `fee.rs` — Fee handling

### `crates/chia-sdk-driver/src/offers/`

**Purpose:** Offer construction, validation, settlement.

**Files:**
- `offer.rs` — `Offer` builder and types
- `requested_payments.rs` — Requested output coins
- `offer_coins.rs`, `offer_amounts.rs` — Coin/amount tracking
- `royalty.rs` — Royalty enforcement and info
- `asset_info.rs` — Asset metadata in offers
- `compress.rs` — Flate2 compression (offer-compression feature)
- `test_data/` — Test fixtures

---

## Types Source Structure

### `crates/chia-sdk-types/src/puzzles/`

**Purpose:** CLVM puzzle definitions.

**Non-gated:**
- `mods.rs` — Puzzle mod declarations
- `p2_curried.rs` — Curried puzzle format
- `p2_singleton.rs` — Singleton wrapping
- `p2_delegated_conditions.rs` — Delegated puzzle variant
- `p2_parent.rs` — Parent coin reference
- `p2_one_of_many.rs` — M-of-N variant
- `revocation.rs` — Revocation support
- `augmented_condition.rs` — Augmented condition
- `option_contract/` — Options contract types
- `mips/` — MIPS family types (chip-0035 gated elsewhere)

**CHIP-0035 (feature-gated):**
- `datalayer/` — Data layer puzzles

**CHIP-0037 (feature-gated):**
- `p2_eip712_message.rs` — EIP-712 message signing puzzle
- `p2_controller_puzzle.rs` — Controller puzzle

**Action-layer (feature-gated):**
- `action_layer/` — Metacontract puzzles and types

---

## Naming Conventions

### Files
- `XyzLayer` type lives in `layers/xyz_layer.rs`
- `Xyz` primitive lives in `primitives/xyz.rs` or `primitives/xyz/xyz.rs`
- Modules re-export their main types via `pub use`

### Types
- **Layer:** `CamelCase` ending in `Layer` (e.g., `CatLayer`, `StandardLayer`, `P2SingletonLayer`)
- **Primitive:** `CamelCase` without suffix (e.g., `Cat`, `Did`, `Nft`, `Singleton`)
- **Info:** `CamelCase` + `Info` suffix (e.g., `CatInfo`, `DidInfo`, `NftInfo`, `VaultInfo`)
- **Launcher:** `CamelCase` + `Launcher` suffix (e.g., `DidLauncher`, `NftLauncher`, `VaultLauncher`)

### Functions/Methods
- `parse_*` — Parse a type from CLVM or bytes
- `construct_*` — Construct a CLVM puzzle or solution
- `spend` — Produce a spend (puzzle + solution)
- `create`, `mint`, `update` — High-level actions

---

## Where to Add New Code

### New Primitive
1. Create `crates/chia-sdk-driver/src/primitives/xyz/` directory
2. Define `XyzInfo` struct (parsed puzzle state) in `xyz_info.rs`
3. Define `Xyz` struct (full coin context) in `xyz.rs`
4. Implement layer parsing and spend methods
5. Export via `primitives.rs` module root
6. Add tests alongside implementation

### New Layer
1. Create `crates/chia-sdk-driver/src/layers/xyz_layer.rs`
2. Implement `Layer` trait
3. Export via `layers.rs` module root
4. Optionally create corresponding primitive wrapping the layer

### New Feature-Gated Types (e.g., CHIP-NNNN)
1. Define puzzle types in `crates/chia-sdk-types/src/puzzles/` behind feature gate
2. Define layer in `crates/chia-sdk-driver/src/layers/` behind feature gate
3. Add feature to workspace `Cargo.toml` line 72-79:
   ```toml
   chip-nnnn = ["chia-sdk-driver/chip-nnnn", "chia-sdk-types/chip-nnnn"]
   ```
4. Mark crate features as `#[cfg(feature = "chip-nnnn")]`
5. CI automatically tests both feature and no-feature builds

### New Transport Client
1. Create `crates/chia-sdk-XXX-client/` crate
2. Depend on shared types from `chia-sdk-types`
3. Export via umbrella `src/lib.rs`

### New Binding API
1. Implement pure Rust function/type in `crates/chia-sdk-bindings/src/new_concept.rs`
2. Add entry to `bindings/*.json` file(s) with per-target type mapping
3. (Optional) Hand-write shim in `napi/src/lib.rs`, `pyo3/src/lib.rs`, or `wasm/src/lib.rs` for macro-inexpressible types
4. Regenerate bindings: `napi build` / `maturin develop` / `wasm-pack build`

### Tests
- **Unit tests:** Alongside source file (e.g., `#[cfg(test)] mod tests { ... }`)
- **Integration tests:**
  - Rust: `crates/*/tests/` directory
  - Node.js: `napi/__test__/` directory (AVA)
  - Python: `pyo3/tests/` directory (pytest)
  - WASM: `wasm/__test__/` directory (AVA)
- **Examples:** `examples/*.rs` (automatically tested in CI via `cargo build`)

---

## Special Directories

### `.github/workflows/`
**CI/CD pipeline:**
- `rust.yml` — Rust build, test, fmt, clippy, machete
- `napi.yml` — NAPI-RS build, test (AVA)
- `pyo3.yml` — PyO3 build, test (pytest)
- `wasm.yml` — WASM build, test (AVA)

### `examples/`
**Runnable Rust examples (tested in CI):**
- `address_conversion.rs` — Bech32 address encoding/decoding
- `cat_spends.rs` — Constructing CAT spends
- `custom_p2_puzzle.rs` — Custom P2 puzzle definition
- `spend_simulator.rs` — In-process simulator usage

### `bindings/`
**Per-concept type mapping JSON files (20+ files):**

Each file declares which Rust types/functions are exposed and their per-target equivalents:
```json
{
  "ModuleName": {
    "functions": { "fn_name": { "return": "Type", "args": [...] } },
    "types": { "RustType": "ExternalType" }
  }
}
```

**Example:** `bindings/address.json` declares `Address` and `Bech32` types.

---

*Structure analysis: 2026-05-15*
