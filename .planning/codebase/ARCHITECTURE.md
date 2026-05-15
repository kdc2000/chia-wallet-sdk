# Architecture

**Analysis Date:** 2026-05-15

## Pattern Overview

**Overall:** Layered, composable puzzle-based SDK for building Chia blockchain applications.

**Key Characteristics:**
- **Puzzle-layer abstraction**: Coins are solved by puzzles; puzzles compose from multiple layers (e.g. CAT + P2)
- **Primitive wrapping**: Each blockchain construct (Cat, Did, Nft, Singleton) is a `Primitive` that composes layers for parsing and spending
- **SpendContext + action system**: High-level builder for batching spends; prefer over manual assembly
- **Workspace layering**: Umbrella crate re-exports 10 member crates under short names; bindings (napi, pyo3, wasm) share one Rust core
- **Feature-gated additions**: `chip-0035`, `chip-0037`, `action-layer` gate new puzzle types and layers

## Workspace Layering

### Top Level
- `chia-wallet-sdk` (umbrella): Re-exports workspace crates as short names (`driver`, `types`, `utils`, etc.); curates public surface via `src/prelude.rs`

### Core Types & Constants
- `chia-sdk-types` (`crates/chia-sdk-types/src/`): Chia condition types, puzzle mod definitions (including gated `chip-0035` and `chip-0037` puzzles), merkle tree, mainnet/testnet constants, action-layer definitions

### Driver (Core)
- `chia-sdk-driver` (`crates/chia-sdk-driver/src/`): **The heart of the SDK**
  - Layers for standard puzzles (CAT, DID, NFT, Singleton, Vault, MIPS, Datalayer, Clawback, Streaming)
  - Primitives (parsed coin info + spend API)
  - `SpendContext` (allocator cache + batch coin-spend collection)
  - Action system (`Spends`, `Action`, `Spend`, `SpendWithConditions`, `Outputs`, `Deltas`, `Relation`)
  - Offers system

### Utilities & Helpers
- `chia-sdk-utils` (`crates/chia-sdk-utils/src/`): Bech32 `Address`, coin selection, hex parsing
- `chia-sdk-signer` (`crates/chia-sdk-signer/src/`): Required signature computation (BLS, SECP)
- `chia-sdk-test` (`crates/chia-sdk-test/src/`): `Simulator`, key-pair fixtures, in-process peer (via `peer-simulator` feature)

### Network Peers (Transport)
- `chia-sdk-client` (`crates/chia-sdk-client/src/`): Light-wallet protocol peer (TLS via `native-tls` or `rustls` feature)
- `chia-sdk-coinset` (`crates/chia-sdk-coinset/src/`): REST client for full-node coinset endpoints (cursor-based pagination)
- `chia-sdk-daemon` (`crates/chia-sdk-daemon/src/`): WebSocket client for Chia daemon (recent addition)

### Bindings
- `chia-sdk-bindings` (`crates/chia-sdk-bindings/src/`): Rust facades for cross-language exposure
- `bindy`, `bindy-macro` (`crates/chia-sdk-bindings/{bindy,bindy-macro}/`): Code generation from `bindings/*.json`
- `napi/`, `pyo3/`, `wasm/`: Per-platform binding crates with generated API surfaces

### Other
- `chia-sdk-cli` (`crates/chia-sdk-cli/src/`): Small CLI built on the SDK

## Layer + Primitive + SpendContext Model (Driver)

### Layer Trait
**Location:** `crates/chia-sdk-driver/src/layer.rs`

Defines how to parse and construct a single layer of a puzzle:

```
pub trait Layer {
    type Solution;
    fn parse_puzzle(allocator: &Allocator, puzzle: Puzzle) -> Result<Option<Self>>;
    fn parse_solution(allocator: &Allocator, solution: NodePtr) -> Result<Self::Solution>;
    fn construct_puzzle(&self, ctx: &mut SpendContext) -> Result<NodePtr>;
    fn construct_solution(&self, ctx: &mut SpendContext, solution: Self::Solution) -> Result<NodePtr>;
    fn construct_spend(&self, ctx: &mut SpendContext, solution: Self::Solution) -> Result<Spend>;
}
```

### Layer Implementations
**Location:** `crates/chia-sdk-driver/src/layers/`

- `StandardLayer` (`standard_layer.rs`): P2 ownership control
- `CatLayer` (`cat_layer.rs`): CAT token restrictions (asset_id + inner_puzzle)
- `DidLayer` (`did_layer.rs`): DID identity layer
- `NftOwnershipLayer`, `NftStateLayer` (`nft_ownership_layer.rs`, `nft_state_layer.rs`): NFT layers
- `SingletonLayer` (`singleton_layer.rs`): Singleton state tracking
- `P2SingletonLayer` (`p2_singleton_layer.rs`): P2 with singleton enforcement
- `P2DelegatedConditionsLayer` (`p2_delegated_conditions_layer.rs`): Delegated puzzle variant
- `SettlementLayer` (`settlement_layer.rs`): Offers settlement
- `RevocationLayer` (`revocation_layer.rs`): Revocation support
- `RoyaltyTransferLayer` (`royalty_transfer_layer.rs`): Royalty enforcement
- `BulletinLayer` (`bulletin_layer.rs`): Bulletin board messages
- `OptionContractLayer` (`option_contract_layer.rs`): Options contracts
- `StreamingLayer` (`streaming_layer.rs`): Streaming assets
- `DatalayerLayer`, `DatalayerWriterLayer`, `DatalayerOracleLayer`, `DatalayerDelegationLayer` (`datalayer/`): Data layer access (chip-0035 gated)
- `P2Eip712MessageLayer` (`p2_eip712_message_layer.rs`): EIP-712 signatures (chip-0037 gated)
- `P2ControllerPuzzleLayer` (`p2_controller_puzzle_layer.rs`): Controller puzzle (chip-0037 gated)
- `ActionLayer*` (`action_layer/`): Action-layer metacontracts (action-layer feature gated)

### Primitive Struct
**Location:** `crates/chia-sdk-driver/src/primitives/`

Composites one or more layers to parse full coin info and provide spend API.

**Naming convention:** `XInfo` struct holds the parsed puzzle state; `X` struct is the complete coin context.

- `Cat` + `CatInfo` (`cat/`): CAT token spend API
  - `CatSpend`, `SingleCatSpend`: Spend types
- `Did` + `DidInfo` (`did/`): DID identity
- `Nft` + `NftInfo` (`nft/`): NFT with metadata + mint
  - `NftMint`, `MetadataUpdate`
- `Singleton` + `SingletonInfo` (`singleton.rs`): Singleton state machine
- `Vault` + `VaultInfo` (`vault/`): MIPS vault (chip-0035 gated)
- `Launcher`, `IntermediateLauncher` (`launcher.rs`, `intermediate_launcher.rs`): Coin creation
- `OptionContract` + `OptionInfo` (`option/`): Options contract with metadata
  - `OptionLauncher`, `OptionUnderlying`, `OptionMetadata`
- `Datalayer` + `DatastoreInfo` (`datalayer/`): Data layer (chip-0035 gated)
  - `DatastoreLauncher`
- `StreamedAsset` (`streamed_asset.rs`): Streaming asset
- `Bulletin` (`bulletin.rs`): Bulletin board message
- `Clawback`, `ClawbackV2` (`clawback.rs`, `clawback_v2.rs`): Clawback patterns
- `P2ParentCoin` (`p2_parent_coin.rs`): Parent coin tracking (action-layer gated)

### SpendContext
**Location:** `crates/chia-sdk-driver/src/spend_context.rs`

Wrapper around `Allocator` that caches puzzles and collects `CoinSpend`s:

```
pub struct SpendContext {
    allocator: Allocator,
    puzzles: HashMap<TreeHash, NodePtr>,
    coin_spends: Vec<CoinSpend>,
}
```

**Key methods:**
- `new()`: Create context
- `iter()`, `take()`: Access collected spends
- `insert()`: Add a manual `CoinSpend`
- `spend()`: Serialize a `Spend` and add as `CoinSpend`
- `alloc()`: Allocate a CLVM value
- `serialize()`: Serialize a pointer to bytes

### Action System
**Location:** `crates/chia-sdk-driver/src/action_system/`

High-level builder for composing spends. Avoid manual `CoinSpend` assembly when possible.

**Key types:**
- `Spends` (`spends.rs`): Builder collecting `SpendAction`s, produces `SpendBundle` via `finish()`
- `Action` (`action.rs`): High-level action (e.g., "send XCH to address")
- `Spend` (`spend.rs`): Low-level spend tuple (puzzle + solution)
- `SpendWithConditions` (`spend_with_conditions.rs`): Spend + conditions asserted
- `Outputs` (`output.rs`): Output coins for an action
- `Deltas` (`deltas.rs`): State changes (fees, asset flows)
- `Relation` (`relation.rs`): Links between actions (ordering, conditions)
- `SpendableAsset`, `FungibleSpends`, `SingletonSpends` (`spendable_asset.rs`, `fungible_spends.rs`, `singleton_spends.rs`): Asset-specific spend strategies

**Macro pattern:** Actions in `crates/chia-sdk-driver/src/actions/` define specific operations (e.g., `create_did.rs`, `mint_nft.rs`, `issue_cat.rs`)

### Spend
**Location:** `crates/chia-sdk-driver/src/spend.rs`

Minimal struct pairing puzzle and solution pointers:

```
pub struct Spend {
    pub puzzle: NodePtr,
    pub solution: NodePtr,
}
```

## Offers System
**Location:** `crates/chia-sdk-driver/src/offers/`

Composable offer construction and settlement:

- `Offer` (`offer.rs`): Core offer builder
- `RequestedPayments` (`requested_payments.rs`): Requested coin outputs
- `OfferCoins`, `OfferAmounts` (`offer_coins.rs`, `offer_amounts.rs`): Coin/amount tracking
- `RoyaltyInfo` (`royalty.rs`): Royalty enforcement
- `SettlementLayer` (in `layers/settlement_layer.rs`): Layer for settling offers
- `compress.rs`: Flate2-based compression (gated by `offer-compression` feature)

## Feature Flags (Workspace Level)

**Location:** `Cargo.toml` line 72-79

- `chip-0035`: CHIP-0035 puzzles (vault/MIPS, datalayer)
  - Gates `chia-sdk-types/src/puzzles/datalayer.rs`
  - Gates `chia-sdk-driver/src/primitives/datalayer/`, `layers/datalayer/`
- `chip-0037`: CHIP-0037 puzzles (EIP-712 / controller-puzzle)
  - Gates `chia-sdk-types/src/puzzles/{p2_eip712_message.rs, p2_controller_puzzle.rs}`
  - Gates `chia-sdk-driver/src/layers/{p2_eip712_message_layer.rs, p2_controller_puzzle_layer.rs}`
- `action-layer`: Higher-level action layer driver code
  - Gates `chia-sdk-types/src/puzzles/action_layer/`, `chia-sdk-driver/src/layers/action_layer/`
  - Enables metacontracts (catalog registry, reward distributor, xchandles, etc.)
- `offer-compression`: Flate2-backed compressed offers
  - Gates `chia-sdk-driver/src/offers/compress.rs`
- `native-tls` / `rustls`: TLS backend selection for `chia-sdk-client`, `chia-sdk-coinset`, `chia-sdk-daemon`
- `peer-simulator`: In-process peer for testing (gates `chia-sdk-test/peer-simulator`)

Each crate is built individually with and without `--all-features` in CI.

## Bindings Pipeline

**Entry points:**
- `crates/chia-sdk-bindings/src/` — Pure Rust facades over the SDK (one file per concept)
- `bindings.json` — Top-level type-group mappings (e.g., `{bytes} → Uint8Array` in napi, `bytes` in python)
- `bindings/*.json` — Per-concept type mappings (20+ JSON files for clvm, puzzles, rpc, simulator, etc.)
- `bindy-macro` — Code generator (runs at compile-time via `bindy_napi!()`, `bindy_pyo3!()`, `bindy_wasm!()` invocations)
- `bindy` — Runtime trait implementations (`FromRust`, `IntoRust`, per-target contexts)

**Per-binding crate (`napi/src/lib.rs`, `pyo3/src/lib.rs`, `wasm/src/lib.rs`):**

```rust
// Example from napi/src/lib.rs
bindy_macro::bindy_napi!("bindings.json");

// Hand-written extras (e.g., Clvm::alloc) for things the macro can't express
#[napi]
impl Clvm {
    pub fn alloc(&self, env: Env, value: Value<'_>) -> Result<Program> { ... }
}
```

**How to add a new binding:**
1. Implement pure Rust function/type in `crates/chia-sdk-bindings/src/` (e.g., `src/foo.rs`)
2. Add entry to appropriate `bindings/*.json` file(s) with per-target type mappings
3. (Optional) Hand-write shim in binding crate if macro can't express it
4. Re-run `napi build`, `maturin develop`, or `wasm-pack build` to test

**Existing binding JSON files:**
- `action_system.json`: Spends, Outputs, Deltas, Relation
- `address.json`: Address, Bech32
- `bls.json`: BLS key pairs and signatures
- `clear_signing.json`: Clear message signing
- `clvm.json`: Allocator, Program, TreeHash, solver
- `clvm_types.json`: Pair, Atom, NodePtr
- `coin.json`: Coin, CoinSpend, CoinState
- `conditions.json`: Condition types (AggSig*, CreateCoin, etc.)
- `constants.json`: MAINNET/TESTNET constants
- `mips.json`: MIPS vault types and memo parsing
- `mnemonic.json`: Bip39 mnemonic generation
- `offer.json`: Offer, RequestedPayments, OfferAmounts
- `peer.json`: Peer connection, PeerOptions
- `program.json`: Program serialization, currying
- `puzzles.json`: All puzzle types and info structs
- `rpc.json`: RPC client and response types
- `reward_distributor.json`: Action-layer reward distributor
- `secp.json`: SECP key pairs and signatures
- `simulator.json`: Simulator, SimulatorConfig
- `utils.json`: Address, coin selection

## Data Flow Example: Spending a CAT Coin

1. **On-chain lookup**: Fetch parent `CoinSpend` via peer/coinset endpoint
2. **Construct `CatInfo`**: Parse parent with `CatLayer` to extract asset_id, lineage proof, inner puzzle info
3. **Create `Cat` primitive**: Wrap `CatInfo` + coin + lineage proof
4. **Set up spend**: Call `Cat::spend()` or similar, which composes layers and returns `Spend` (puzzle + solution pointers)
5. **Use `SpendContext`**: Pass `SpendContext` to primitives; they serialize puzzle/solution and collect in context
6. **Build spend bundle**: Gather collected `CoinSpend`s and sign with required signatures (from `chia-sdk-signer`)
7. **Broadcast**: Push spend bundle to peer/node

## Error Handling

**Strategy:** Result-based, no panics in library code.

**Key error types:**
- `DriverError` (`crates/chia-sdk-driver/src/driver_error.rs`): Enumeration of puzzle/layer/spend errors
- `Bech32Error` (`crates/chia-sdk-utils/src/bech32.rs`): Address decode/encode errors
- Type-specific errors from upstream crates (e.g., `clvmr::serde::DecodeError`)

## Cross-Cutting Concerns

**Logging:** Implicit via `tracing` (optional, no hard dependency in library code)

**Validation:** Embedded in layer parsing (return `None` if puzzle doesn't match, `Err` if should have matched but parsing failed)

**Authentication:** External; `chia-sdk-signer` computes required signatures, caller is responsible for key management

**Feature composition:** Workspace-level features in `Cargo.toml` gate entire puzzle families (chip-0035, chip-0037). CI verifies both feature and no-feature builds for each crate.

---

*Architecture analysis: 2026-05-15*
