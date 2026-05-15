# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository purpose

A Rust SDK for building applications that interact with coins on the Chia blockchain (wallets, dApps). The umbrella crate `chia-wallet-sdk` re-exports the workspace crates under shorter names (e.g. `chia_sdk_driver` → `driver`, `chia_sdk_types` → `types`); `src/prelude.rs` is the curated public surface. This is **not** a prebuilt wallet — Sage Wallet is the reference consumer.

The same Rust core also ships as Node.js (napi-rs), Python (pyo3/maturin), and WASM (wasm-bindgen) bindings.

## Commands

Rust toolchain is pinned to **1.90.0** (see `rust-toolchain.toml`); edition 2024.

```bash
# Build everything (mirrors CI)
cargo build --release --all-features

# Run the full test suite the way CI does
cargo test --release --workspace --all-features \
  --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive \
  --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm \
  --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro

# Single test in a single crate
cargo test --release -p chia-sdk-driver --all-features -- nft::tests::test_mint

# Lint gates that CI runs (all must pass)
cargo fmt --all -- --files-with-diff --check
cargo clippy --workspace --all-features --all-targets
cargo machete            # unused-dependency check

# Build a specific crate with its features (CI builds each individually)
cargo build --release -p chia-sdk-driver --all-features
cargo build --release -p chia-sdk-bindings -F napi    # or -F wasm / -F pyo3
```

Binding test runners (from each binding directory):

```bash
# napi/   — TypeScript tests via AVA
pnpm install && pnpm build && pnpm test

# wasm/   — wasm-pack build + AVA
pnpm install && pnpm test          # builds nodejs target then runs AVA

# pyo3/   — maturin develop + pytest
maturin develop && pytest
```

Workspace clippy is **deny clippy::all + warn clippy::pedantic + warn clippy::cargo** with `unsafe_code = "deny"` and `dead_code = "deny"`. After editing Rust, run clippy before reporting done — warnings on pedantic become errors with `-D warnings` locally even though CI only fails on `deny`-level. Prefer LSP diagnostics for tight feedback.

## Architecture

### Workspace layout

- `src/` — the top-level `chia-wallet-sdk` umbrella crate; `lib.rs` re-exports each `chia_sdk_*` as a short module name, `prelude.rs` is the curated public surface.
- `crates/chia-sdk-driver` — **the core of the SDK**. Implements layers and primitives for the standard Chia puzzles (CAT, DID, NFT, Singleton, Vault/MIPS, Option, Datalayer, Clawback, Streaming). Also exposes `SpendContext` and the action system used to compose transactions.
- `crates/chia-sdk-types` — Chia condition types, puzzle mod definitions, merkle tree, mainnet/testnet constants. Many primitives are gated behind `chip-0035`, `chip-0037`, `action-layer` features.
- `crates/chia-sdk-client` — light-wallet protocol peer client (TLS via `native-tls` or `rustls` feature).
- `crates/chia-sdk-coinset` — REST client for full-node coinset endpoints (recently gained cursor/paginated support).
- `crates/chia-sdk-daemon` — websocket client for the Chia daemon (newer, see commit `ec1a3517`).
- `crates/chia-sdk-signer` — computes required BLS / SECP signatures for coin spends.
- `crates/chia-sdk-test` — `Simulator`, key-pair fixtures, used for integration tests. The `peer-simulator` feature exposes an in-process peer.
- `crates/chia-sdk-utils` — Bech32 `Address`, coin selection, hex parsing.
- `crates/chia-sdk-cli` — small CLI built on top of the SDK.
- `crates/chia-sdk-bindings` — Rust glue + the `bindy` / `bindy-macro` machinery that generates the cross-language API.
- `napi/`, `pyo3/`, `wasm/` — thin per-platform crates that invoke the bindy macro and expose a few hand-written extras.
- `examples/` — runnable Rust examples (`spend_simulator.rs`, `cat_spends.rs`, `custom_p2_puzzle.rs`, …) — read these first when learning a primitive.

### Driver mental model

Read `crates/chia-sdk-driver/docs.md` (rendered as the crate-level doc) for the canonical version. Key vocabulary:

- **Puzzle** = a coin's lock; conditions are produced from solving it. Composed of stacked **layers**.
- **Layer** = one slice of smart-coin logic, also called an "inner puzzle." Each layer in `driver/src/layers/` can parse and construct its piece independently, so layers compose freely (with some restrictions — e.g. `CatLayer` cannot wrap another `CatLayer`).
- **P2 ("pay to") puzzle** = the base layer that controls ownership (`p2_conditions`, `p2_delegated_puzzle_or_hidden_puzzle` aka the "standard transaction", `p2_singleton`, `p2_eip712_message`, `p2_controller_puzzle`, …). New CHIP work usually lands as a new layer + a new primitive on top.
- **Primitive** (`driver/src/primitives/`) = a struct that composes one or more layers to (a) parse a coin's full info from its parent spend, and (b) provide a spend API. Examples: `Cat`, `Did`, `Nft`, `Singleton`, `Vault`, `OptionContract`, `Datalayer`, `Streaming`, `Bulletin`, `Clawback`.
- **`SpendContext` + `Spends` action system** — high-level builder that batches `SpendAction`s (`Action`, `Spend`, `SpendWithConditions`, `Outputs`, `Deltas`, `Relation`) into a coin spend bundle. Prefer this over assembling `CoinSpend`s by hand.
- **Offers** (`driver/src/offers/`) — `Offer`, `RequestedPayments`, `SettlementLayer`, `RoyaltyInfo`. The `offer-compression` feature pulls in flate2 for the wire format.

### Bindings architecture

The bindings are **generated**, not hand-written. The flow:

1. `crates/chia-sdk-bindings/src/*.rs` defines pure-Rust facades over the SDK (one file per concept: `clvm.rs`, `peer.rs`, `simulator.rs`, etc.).
2. `bindings/*.json` (note: top-level `bindings/`, not `bindings.json`) declares which Rust functions/types are exposed and how their types map per target. The root `bindings.json` declares type-group mappings (e.g. `Bytes32 → Uint8Array` for napi, `bytes` for python).
3. `bindy-macro` reads `bindings.json` at compile time via `bindy_macro::bindy_napi!`, `bindy_pyo3!`, `bindy_wasm!` invocations inside `napi/src/lib.rs`, `pyo3/src/lib.rs`, `wasm/src/lib.rs`.
4. `bindy` (the runtime crate) supplies the `FromRust` / `IntoRust` traits and per-target conversion contexts.

When adding a new API to the bindings, the change usually has three parts: Rust impl in `chia-sdk-bindings`, a JSON descriptor entry in `bindings/*.json`, and (occasionally) a hand-written shim in one of the three binding crates for things the macro can't express. Don't hand-edit `napi/index.d.ts` or `napi/index.js` — they are generated by `napi build`.

### Feature flags (workspace level)

- `chip-0035` — CHIP-0035 puzzles (vault / MIPS family).
- `chip-0037` — CHIP-0037 puzzles (EIP-712 / controller-puzzle scaffolding).
- `action-layer` — the higher-level action layer driver code.
- `offer-compression` — flate2-backed compressed offers.
- `native-tls` / `rustls` — pick one for `chia-sdk-client`, `chia-sdk-coinset`, `chia-sdk-daemon`.
- `peer-simulator` — adds an in-process peer to `chia-sdk-test`.

CI builds **each crate** individually with and without `--all-features` — if you add code behind a feature, verify both modes compile.

## Conventions worth knowing

- **Workspace lints in `Cargo.toml` are authoritative.** New crates must declare `[lints] workspace = true`.
- Dependency versions live in `[workspace.dependencies]`; reference them with `{ workspace = true }` in member `Cargo.toml`s rather than pinning a new version.
- `chia-*` upstream deps are pinned to `0.36.1` (and `chia-puzzles = 0.20.3`, `clvmr = 0.16.2`) — bumping these is a deliberate, repo-wide change.
- Examples are tested implicitly via `cargo build --all-features` in CI; they are also the friendliest entry point when learning a primitive.
- `cargo machete` runs in CI — keep dependency lists tight; use `[package.metadata.cargo-machete]` `ignored` arrays only for deps that are genuinely used but undetectable (e.g. proc-macro re-exports in the binding crates).

<!-- GSD:project-start source:PROJECT.md -->
## Project

**CHIP-0057 Silent Payments — chia-wallet-sdk Integration**

Wallet-facing support for [CHIP-0057 silent payments](https://github.com/Chia-Network/chips) inside `chia-wallet-sdk`. Silent payments let a recipient publish one static `spxch1...` address (and labeled sub-addresses) while every payment on chain lands at a fresh, unlinkable one-time puzzle hash derived via ECDH on BLS12-381. This work adds the cryptographic primitives, address types, send-side driver code, transport-agnostic receive primitive, and bindings exposure needed for wallets like Sage to display silent-payment addresses and send XCH to one.

Adapted from [BIP-352](https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki). Reference implementation lives at `~/silent-payments` (Python prototype + an `sp-common` / `sp-service` / `sp-client` Rust workspace) — only `sp-common`'s wallet-side primitives map into the SDK; `sp-service` and `sp-client` are external infrastructure that consume the SDK, not part of it.

**Core Value:** A wallet developer can derive a silent-payment address from a mnemonic, display it, send XCH to a silent-payment address, and (once a CHIP-0058 tweak-data source exists) detect incoming silent payments — without re-implementing any cryptography and through the same idiomatic Layer/Primitive/Spends/bindings surface the SDK already uses for everything else.

### Constraints

- **Tech stack**: Rust 1.90.0 (pinned in `rust-toolchain.toml`), edition 2024. Cannot use nightly-only features. Cryptographic implementations must compile under `unsafe_code = "deny"`.
- **Dependencies**: No new workspace deps. `bip39`, `num-bigint`, `hex`, `sha2`, `chia-bls`, `chia-puzzle-types`, `clvm-utils` are all already in `[workspace.dependencies]`. Bumping `chia-protocol` (0.36.1) or `chia-puzzles` (0.20.3) is out of scope for this work.
- **CI gates**: Workspace-level clippy (`deny clippy::all`, `warn pedantic`, `warn cargo`), `cargo machete` (unused-deps), `cargo fmt --check`, and per-crate builds with and without `--all-features`. New `chip-0057`-gated code must compile in every CI permutation.
- **Bindings format**: Wire-protocol surfaces (`TweakData`, `DetectedSpCoin`) must be expressible in `bindings/silent_payments.json` so napi/pyo3/wasm get them. Bytes32 + lists of (PublicKey, OutputMeta) is the granularity to design around.
- **Feature flag**: The `chip-0057` workspace feature is the only umbrella for this work. No feature within a feature, no name-aliases.
- **Forward compatibility with CHIP-0058**: The receive-side primitive accepts a transport-agnostic `TweakData` input. Any CHIP-0058 transport client built later must be able to construct `TweakData` from its wire messages without breaking the existing SDK API.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- **Rust** 1.90.0 (pinned in `rust-toolchain.toml`) - Core SDK implementation
- Edition 2024
- Targets: x86_64-apple-darwin, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-apple-darwin, aarch64-unknown-linux-gnu
- **TypeScript** 4.x - napi and wasm binding tests via AVA
- **Python** 3.8+ - pyo3/maturin binding tests via pytest
## Runtime
- tokio 1.47.1 with features: sync, time, rt
- Node >= 14 (engine constraint in `napi/package.json`)
- Tested against Node 20, 22, 24
- Package manager: pnpm 9.11.0
- Python 3.x (detected via maturin)
- abi3-py38 for stable ABI compatibility
## Frameworks
- tokio-tungstenite 0.24.0 - WebSocket client (TLS-aware)
- reqwest 0.12.23 - HTTP client for REST APIs (Coinset)
- tungstenite 0.24.0 - WebSocket protocol handling
- chia-bls 0.36.1 - BLS signing (Chia reference)
- chia-secp 0.36.1 - SECP256k1 operations
- chia-sha2 0.36.1 - SHA2 hashing
- sha2 0.10.9, sha3 0.10.8 - Additional hash support
- k256 0.13.4, p256 0.13.2, signature 2.2.0 - ECDSA/ECDH
- chia-protocol 0.36.1 - Wire protocol messages (peer protocol)
- chia-consensus 0.36.1 - Consensus rules
- chia-traits 0.36.1 - Serialization traits
- chia-puzzle-types 0.36.1 - Standard puzzle definitions
- chia-puzzles 0.20.3 - Puzzle implementations
- clvmr 0.16.2 - Chialisp VM
- clvm-traits 0.36.1, clvm-utils 0.36.1 - CLVM utilities
- chialisp 0.4.1 - Chialisp compiler
- napi 3.3.0, napi-derive 3.2.5 - Node.js/napi-rs
- pyo3 0.23.5, pyo3-async-runtimes 0.23 - Python bindings
- wasm-bindgen 0.2.100, wasm-bindgen-futures 0.4.50 - WebAssembly
- rstest 0.22.0 - Parametric tests
- tokio (full features) in dev - Async testing
- serde 1.0.228, serde_json 1.0.145 - JSON/serialization
- bincode 2.0.1 - Binary encoding
- bech32 0.9.1 - Address encoding
- syn 2.0.106, quote 1.0.41, proc-macro2 1.0.101 - Procedural macros
- convert_case 0.8.0 - Case conversion utilities
- napi-build 3.0.0-beta.0 - napi build support
- hex 0.4.3, hex-literal 0.4.1 - Hex encoding
- thiserror 2.0.17 - Error handling
- futures-util 0.3.30, futures-channel 0.3.30 - Async combinators
- tokio (sync, time, rt) - Async primitives
- itertools 0.13.0 - Iterator utilities
- indexmap 2.11.4 - Ordered maps
- bigdecimal 0.4.8, num-bigint 0.4.6 - Arbitrary precision
- bip39 2.2.0 - BIP-39 mnemonics
- fastrand 2.3.0 - Random number generation
- rand 0.9.2, rand_chacha 0.9.0 - PRNG
- parking_lot 0.12.5 - Synchronization
- tracing 0.1.41 - Logging/diagnostics
- indoc 2.0.6 - Inline documentation
- console_error_panic_hook 0.1.7 - WASM panic handling
- colored 3.1.1 - CLI color output
- clap 4.5.50 - CLI argument parsing
- prettytable-rs 0.10.0 - Table formatting
- native-tls 0.2.14 - Platform-native TLS (OpenSSL on Linux, Secure Transport on macOS, SChannel on Windows)
- rustls 0.23.32 - Pure-Rust TLS implementation
- rustls-pemfile 2.2.0 - PEM certificate parsing
- aws-lc-rs 1.x - AWS crypto library (bindgen support for rustls)
- flate2 1.1.4 with zlib-ng-compat - DEFLATE compression for offer wire format
- rue-compiler 0.6.0, rue-options 0.6.0, rue-lir 0.6.0 - Rue language compilation
- @napi-rs/cli 3.0.0-alpha.91 - napi build tool
- @types/node 22.13.1
- ava 7.0.0 - Test runner
- ts-node 10.9.2 - TypeScript execution
- wasm-pack (installed via cargo-install) - WASM build tooling
- getrandom 0.3.4 - Random number generation in WASM
## Configuration
- `[workspace.dependencies]` in `Cargo.toml` at root specifies all shared versions
- All member crates use `{ workspace = true }` for dependency references
- Strict linting configuration applied via `[workspace.lints]`
- unsafe_code = "deny" (no unsafe blocks)
- dead_code = "deny" (all code must be used)
- Rust 2018/2021 idioms as "deny"
- nonstandard_style as "deny"
- all = "deny" (deny all clippy warnings)
- pedantic = "warn" (warn on style suggestions)
- cargo = "warn" (warn on Cargo.toml practices)
- `[profile.release]` with LTO enabled and symbol stripping for published binaries
- None detected as critical; bindings discovery is compile-time
## Platform Requirements
- Rust 1.90.0 (via rustup with `rust-toolchain.toml`)
- cargo-binstall for rapid tool installation in CI
- cargo-workspaces for multi-crate operations
- cargo-machete for dependency verification
- For napi: Node.js 20+ and pnpm 9
- For wasm: wasm-pack (cargo-installed)
- For pyo3: Python 3.8+ and maturin
- Deployment as Rust library (vendored in SDK consumer projects)
- Or as published npm packages: `chia-wallet-sdk` (napi) and `chia-wallet-sdk-wasm` (wasm)
- Or as published Python package: `chia_wallet_sdk` (pyo3/maturin wheels)
## Feature Flags (Workspace-level)
- `chip-0035` - CHIP-0035 puzzles (vault/MIPS family). Gates `chia-sdk-types/chip-0035` and `chia-sdk-driver/chip-0035`
- `chip-0037` - CHIP-0037 puzzles (EIP-712/controller-puzzle scaffolding). Gates types + driver features + optional sha3 dependency
- `action-layer` - Higher-level action layer driver code (SpendAction builder). Gates `chia-sdk-types/action-layer` and `chia-sdk-driver/action-layer`
- `offer-compression` - Flate2-backed compressed offers wire format. Gates `chia-sdk-driver/offer-compression` + optional flate2 and chia-sdk-utils
- `native-tls` - Platform-native TLS for `chia-sdk-client`, `chia-sdk-coinset`, `chia-sdk-daemon`. Gates tokio-tungstenite/native-tls + reqwest/native-tls
- `rustls` - Pure-Rust TLS for the same. Gates tokio-tungstenite/rustls-tls-webpki-roots + rustls + rustls-pemfile + aws-lc-rs + reqwest/rustls-tls
- `peer-simulator` - In-process peer for `chia-sdk-test`. Exposes Simulator for integration tests
- `napi` in `chia-sdk-bindings/Cargo.toml` - Enables napi-specific binding generation
- `wasm` in `chia-sdk-bindings/Cargo.toml` - Enables wasm-bindgen binding generation
- `pyo3` in `chia-sdk-bindings/Cargo.toml` - Enables pyo3 binding generation
## Build & Test Commands (as used by CI)
# Lint
# Build (per-crate for feature coverage)
# Tests (excludes binding crates)
# Platform-specific wheel builds (cross-platform via maturin-action)
## Workspace Member Crates
- `crates/chia-sdk-driver` - High-level puzzle layer composition and spend building
- `crates/chia-sdk-types` - Type definitions, conditions, puzzle info
- `crates/chia-sdk-client` - Light-wallet peer protocol client (WebSocket + TLS)
- `crates/chia-sdk-coinset` - Coinset REST API client
- `crates/chia-sdk-daemon` - Chia daemon websocket RPC client
- `crates/chia-sdk-signer` - BLS/SECP signature computation
- `crates/chia-sdk-test` - Test utilities and in-process simulator
- `crates/chia-sdk-utils` - Bech32 addresses, coin selection, hex parsing
- `crates/chia-sdk-cli` - Command-line utility
- `crates/chia-sdk-bindings` - Pure-Rust facades for cross-language API
- `crates/chia-sdk-bindings/bindy` - Runtime type conversion traits
- `crates/chia-sdk-bindings/bindy-macro` - Procedural macro for generating bindings
- `crates/chia-sdk-types/derive` - Custom derive macros for types
- `napi/` - Node.js native addon (generates .node files per platform)
- `wasm/` - WebAssembly module (wasm-bindgen, generates npm package)
- `pyo3/` - Python C extension (maturin, generates .so/.pyd wheels)
- `pyo3/stub-generator` - Auto-generates .pyi type stubs for Python
- `src/` - Umbrella crate `chia-wallet-sdk`, re-exports internals under short names
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Workspace Lint Policy
### Rust Built-in Lints (Deny)
- `rust_2018_idioms` (priority = -1)
- `rust_2021_compatibility` (priority = -1)
- `future_incompatible` (priority = -1)
- `nonstandard_style` (priority = -1)
- `unsafe_code` — **no unsafe blocks allowed**
- `non_ascii_idents`
- `unused_extern_crates`
- `trivial_casts`
- `trivial_numeric_casts`
- `unreachable_patterns`
- `dead_code`
- `deprecated`
- `deprecated_in_future`
### Rust Built-in Lints (Warn)
- `missing_debug_implementations`
- `missing_copy_implementations`
- `unreachable_code`
### Rustdoc Lints (Deny)
- `all` — all doctests must pass and be well-formed (priority = -1)
- `missing_crate_level_docs` — crate-level `#![doc]` not required
### Clippy Lints (Deny)
- `all` — **all clippy lint categories denied** (priority = -1)
- `cargo` — manifest metadata and dependency checks (priority = -1)
- `pedantic` — strictness beyond default (priority = -1)
- `too_many_lines` — functions can exceed 200 lines
- `missing_errors_doc` — error handling doc comments optional
- `missing_panics_doc` — panic scenarios doc comments optional
- `module_name_repetitions` — e.g., `CatInfo`, `CatAssetInfo` allowed in same module
- `multiple_crate_versions` — workspace dependencies pinned globally; some transitive deps may have multiple versions
- `must_use_candidate` — not all return values must have `#[must_use]`
- `cargo_common_metadata` — relaxed manifest checks
- `result_large_err` — large error types in `Result<T, E>` allowed
- `format_push_string` — string concatenation patterns allowed
## Formatting & Edition
## Workspace Dependency Declaration Pattern
- `chia-protocol = "0.36.1"` — pinned Chia protocol version (external)
- `chia-sdk-driver = { version = "0.33.0", path = "./crates/chia-sdk-driver" }` — internal workspace crates
- `thiserror = "2.0.17"`, `tokio = "1.47.1"`, `serde = "1.0.228"` — pinned external deps
## Naming Conventions
### Type Hierarchy: Layer / Primitive / Info / AssetInfo
- `CatLayer` — constructs CAT puzzle; implements `Layer` trait
- `P2SingletonLayer`, `DidLayer`, `BulletinLayer` — other layer implementations
- Located: `src/layers/*.rs` and organized in `src/layers/` subdirectory
- `Cat` — represents a CAT coin with required fields; uses `CatLayer` and `CatInfo`
- `Nft`, `Vault`, `Singleton`, `Clawback` — other primitive types
- Located: `src/primitives/<name>.rs` with submodules in `src/primitives/<name>/`
- Example: `Cat` structure in `src/primitives/cat.rs` uses `CatInfo` and `CatLayer`
- `CatInfo` — metadata for CAT (asset ID, revocation state, p2 puzzle hash)
- `VaultInfo` — singleton launcher metadata
- Located: `src/primitives/<name>/<name>_info.rs`
- Used to construct the outer puzzle and validate spending
- `CatAssetInfo`, `NftAssetInfo`, `OptionAssetInfo` — asset representations in offers
- Located: `src/offers/asset_info.rs`
### File and Function Naming
- snake_case for module files: `cat.rs`, `cat_layer.rs`, `clawback_v2.rs`
- Module subdirectories match parent: `primitives/cat/` contains `cat_info.rs`, `cat_spend.rs`
- PascalCase: `Cat`, `CatInfo`, `CatLayer`, `DriverError`
- snake_case: `mint_vault()`, `fetch_cat_coins()`, `parse_children()`
## Feature-Gated Code
#[cfg(feature = "chip-0035")]
#[cfg(feature = "chip-0035")]
#[cfg(feature = "chip-0035")]
#[cfg(feature = "chip-0035")]
- `crates/chia-sdk-driver/src/primitives.rs` gates datalayer behind `chip-0035`
- `crates/chia-sdk-driver/src/layers.rs` gates p2_eip712_message and p2_controller_puzzle behind `chip-0037`
- `offer-compression` — CAT offer DEFLATE compression
- `action-layer` — action system for complex conditions
- `native-tls`, `rustls` — TLS backend selection in client crates
- `peer-simulator` — in-process peer simulation in `chia-sdk-test`
## Module Structure Pattern
## Error Handling
#[derive(Debug, Error)]
- Use `#[from]` for error conversions from other types
- Include context in error message: `#[error("context: {0}")]`
- No panicking in library code — return errors via `Result<T, E>`
## Import Organization
## Documentation
- **Allowed without doc:** internal helper functions, simple getters
- **Required:** public APIs, types, trait implementations, especially if behavior is non-obvious
- **Format:** Standard Rust doc comments with `///` for items and `//!` for module-level docs
- **Doc tests:** Must pass clippy rustdoc lints (compile and run correctly)
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
## Attributes & Derives
- `Debug` — required for all public types (clippy: missing_debug_implementations)
- `Clone, Copy, PartialEq, Eq` — for value types and immutable data
- `Serialize, Deserialize` — when crate features enable serde (explicit feature gates)
- `Error` — for error enums (with thiserror crate)
- `#[must_use]` — on functions/types whose result should not be ignored
- `#[cfg(feature = "...")]` — feature-gated code
- `#[cfg(test)]` — test-only modules and imports
- `#[derive(...)]` — bulk derivation of standard traits
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
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
```
```
### Layer Implementations
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
- `Cat` + `CatInfo` (`cat/`): CAT token spend API
- `Did` + `DidInfo` (`did/`): DID identity
- `Nft` + `NftInfo` (`nft/`): NFT with metadata + mint
- `Singleton` + `SingletonInfo` (`singleton.rs`): Singleton state machine
- `Vault` + `VaultInfo` (`vault/`): MIPS vault (chip-0035 gated)
- `Launcher`, `IntermediateLauncher` (`launcher.rs`, `intermediate_launcher.rs`): Coin creation
- `OptionContract` + `OptionInfo` (`option/`): Options contract with metadata
- `Datalayer` + `DatastoreInfo` (`datalayer/`): Data layer (chip-0035 gated)
- `StreamedAsset` (`streamed_asset.rs`): Streaming asset
- `Bulletin` (`bulletin.rs`): Bulletin board message
- `Clawback`, `ClawbackV2` (`clawback.rs`, `clawback_v2.rs`): Clawback patterns
- `P2ParentCoin` (`p2_parent_coin.rs`): Parent coin tracking (action-layer gated)
### SpendContext
```
```
- `new()`: Create context
- `iter()`, `take()`: Access collected spends
- `insert()`: Add a manual `CoinSpend`
- `spend()`: Serialize a `Spend` and add as `CoinSpend`
- `alloc()`: Allocate a CLVM value
- `serialize()`: Serialize a pointer to bytes
### Action System
- `Spends` (`spends.rs`): Builder collecting `SpendAction`s, produces `SpendBundle` via `finish()`
- `Action` (`action.rs`): High-level action (e.g., "send XCH to address")
- `Spend` (`spend.rs`): Low-level spend tuple (puzzle + solution)
- `SpendWithConditions` (`spend_with_conditions.rs`): Spend + conditions asserted
- `Outputs` (`output.rs`): Output coins for an action
- `Deltas` (`deltas.rs`): State changes (fees, asset flows)
- `Relation` (`relation.rs`): Links between actions (ordering, conditions)
- `SpendableAsset`, `FungibleSpends`, `SingletonSpends` (`spendable_asset.rs`, `fungible_spends.rs`, `singleton_spends.rs`): Asset-specific spend strategies
### Spend
```
```
## Offers System
- `Offer` (`offer.rs`): Core offer builder
- `RequestedPayments` (`requested_payments.rs`): Requested coin outputs
- `OfferCoins`, `OfferAmounts` (`offer_coins.rs`, `offer_amounts.rs`): Coin/amount tracking
- `RoyaltyInfo` (`royalty.rs`): Royalty enforcement
- `SettlementLayer` (in `layers/settlement_layer.rs`): Layer for settling offers
- `compress.rs`: Flate2-based compression (gated by `offer-compression` feature)
## Feature Flags (Workspace Level)
- `chip-0035`: CHIP-0035 puzzles (vault/MIPS, datalayer)
- `chip-0037`: CHIP-0037 puzzles (EIP-712 / controller-puzzle)
- `action-layer`: Higher-level action layer driver code
- `offer-compression`: Flate2-backed compressed offers
- `native-tls` / `rustls`: TLS backend selection for `chia-sdk-client`, `chia-sdk-coinset`, `chia-sdk-daemon`
- `peer-simulator`: In-process peer for testing (gates `chia-sdk-test/peer-simulator`)
## Bindings Pipeline
- `crates/chia-sdk-bindings/src/` — Pure Rust facades over the SDK (one file per concept)
- `bindings.json` — Top-level type-group mappings (e.g., `{bytes} → Uint8Array` in napi, `bytes` in python)
- `bindings/*.json` — Per-concept type mappings (20+ JSON files for clvm, puzzles, rpc, simulator, etc.)
- `bindy-macro` — Code generator (runs at compile-time via `bindy_napi!()`, `bindy_pyo3!()`, `bindy_wasm!()` invocations)
- `bindy` — Runtime trait implementations (`FromRust`, `IntoRust`, per-target contexts)
```rust
#[napi]
```
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
## Error Handling
- `DriverError` (`crates/chia-sdk-driver/src/driver_error.rs`): Enumeration of puzzle/layer/spend errors
- `Bech32Error` (`crates/chia-sdk-utils/src/bech32.rs`): Address decode/encode errors
- Type-specific errors from upstream crates (e.g., `clvmr::serde::DecodeError`)
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
