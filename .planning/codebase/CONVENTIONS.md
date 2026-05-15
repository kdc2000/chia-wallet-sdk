# Coding Conventions

**Analysis Date:** 2026-05-15

## Workspace Lint Policy

The workspace enforces strict linting via `[workspace.lints]` in `/home/kdc/chia-wallet-sdk/Cargo.toml`. All member crates inherit these via `[lints] workspace = true` (verified in `Cargo.toml` files across `crates/chia-sdk-driver/`, `crates/chia-sdk-types/`, `crates/chia-sdk-signer/`, and others).

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

**Allowed exceptions:**
- `missing_crate_level_docs` — crate-level `#![doc]` not required

### Clippy Lints (Deny)
- `all` — **all clippy lint categories denied** (priority = -1)

**Clippy Lints (Warn):**
- `cargo` — manifest metadata and dependency checks (priority = -1)
- `pedantic` — strictness beyond default (priority = -1)

**Clippy Allowed Exceptions:**
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

**Rustfmt Config:** `.rustfmt.toml`
```
edition = "2024"
```

**Edition:** Rust 2024 across all workspace members (declared in each crate's `Cargo.toml` `edition = "2024"`).

## Workspace Dependency Declaration Pattern

Pinned dependencies live in `[workspace.dependencies]` section of root `/home/kdc/chia-wallet-sdk/Cargo.toml`. Examples:
- `chia-protocol = "0.36.1"` — pinned Chia protocol version (external)
- `chia-sdk-driver = { version = "0.33.0", path = "./crates/chia-sdk-driver" }` — internal workspace crates
- `thiserror = "2.0.17"`, `tokio = "1.47.1"`, `serde = "1.0.228"` — pinned external deps

**Member crate dependency usage:**
```toml
[dependencies]
chia-sdk-driver = { workspace = true }
thiserror = { workspace = true }
```

Bumping a shared dependency (especially `chia-*` versions) is a **repo-wide action** affecting all members.

## Naming Conventions

### Type Hierarchy: Layer / Primitive / Info / AssetInfo

The codebase uses a **three-part naming pattern** for puzzle/coin abstractions:

**Layer types** (puzzle constructors in `crates/chia-sdk-driver/src/layers/`):
- `CatLayer` — constructs CAT puzzle; implements `Layer` trait
- `P2SingletonLayer`, `DidLayer`, `BulletinLayer` — other layer implementations
- Located: `src/layers/*.rs` and organized in `src/layers/` subdirectory

**Primitive types** (coin representations in `crates/chia-sdk-driver/src/primitives/`):
- `Cat` — represents a CAT coin with required fields; uses `CatLayer` and `CatInfo`
- `Nft`, `Vault`, `Singleton`, `Clawback` — other primitive types
- Located: `src/primitives/<name>.rs` with submodules in `src/primitives/<name>/`
- Example: `Cat` structure in `src/primitives/cat.rs` uses `CatInfo` and `CatLayer`

**Info types** (configuration/metadata for primitives):
- `CatInfo` — metadata for CAT (asset ID, revocation state, p2 puzzle hash)
- `VaultInfo` — singleton launcher metadata
- Located: `src/primitives/<name>/<name>_info.rs`
- Used to construct the outer puzzle and validate spending

**AssetInfo types** (for offers/trades):
- `CatAssetInfo`, `NftAssetInfo`, `OptionAssetInfo` — asset representations in offers
- Located: `src/offers/asset_info.rs`

### File and Function Naming

**Files:**
- snake_case for module files: `cat.rs`, `cat_layer.rs`, `clawback_v2.rs`
- Module subdirectories match parent: `primitives/cat/` contains `cat_info.rs`, `cat_spend.rs`

**Structs & Enums:**
- PascalCase: `Cat`, `CatInfo`, `CatLayer`, `DriverError`

**Functions:**
- snake_case: `mint_vault()`, `fetch_cat_coins()`, `parse_children()`

**Module Re-exports (Barrel Pattern):**
Each concept gets a barrel file re-exporting its submodules:

```rust
// src/primitives/cat.rs (barrel)
mod cat_info;
mod cat_spend;
mod single_cat_spend;

pub use cat_info::*;
pub use cat_spend::*;
pub use single_cat_spend::*;
```

Then in `lib.rs`:
```rust
pub use primitives::*;  // Re-exports all Cat types via barrel
```

## Feature-Gated Code

CHIP and optional features use `#[cfg(feature = "...")]` guards throughout.

**CHIP Integration Pattern:**

1. **Types side** (`crates/chia-sdk-types/src/`):
```rust
#[cfg(feature = "chip-0035")]
mod datalayer;

#[cfg(feature = "chip-0035")]
pub use datalayer::*;
```

2. **Driver side** (`crates/chia-sdk-driver/src/`):
```rust
#[cfg(feature = "chip-0035")]
mod datalayer;

#[cfg(feature = "chip-0035")]
pub use datalayer::*;
```

3. **Workspace features** (`Cargo.toml`):
```toml
[features]
chip-0035 = ["chia-sdk-driver/chip-0035", "chia-sdk-types/chip-0035"]
chip-0037 = ["chia-sdk-driver/chip-0037", "chia-sdk-types/chip-0037"]
```

**Examples in codebase:**
- `crates/chia-sdk-driver/src/primitives.rs` gates datalayer behind `chip-0035`
- `crates/chia-sdk-driver/src/layers.rs` gates p2_eip712_message and p2_controller_puzzle behind `chip-0037`

Other optional features:
- `offer-compression` — CAT offer DEFLATE compression
- `action-layer` — action system for complex conditions
- `native-tls`, `rustls` — TLS backend selection in client crates
- `peer-simulator` — in-process peer simulation in `chia-sdk-test`

## Module Structure Pattern

Each major concept has both a **file and a directory**:

```
src/
├── primitives.rs          (barrel)
├── primitives/
│   ├── cat.rs             (barrel)
│   ├── cat/
│   │   ├── cat_info.rs
│   │   ├── cat_spend.rs
│   │   └── single_cat_spend.rs
│   ├── nft.rs
│   ├── nft/
│   │   ├── nft_info.rs
│   │   ├── nft_spend.rs
│   │   └── ...
│   └── ...
├── layers.rs              (barrel)
├── layers/
│   ├── cat_layer.rs
│   ├── p2_singleton_layer.rs
│   └── ...
```

**Pattern:** Each `<concept>.rs` serves as a barrel re-exporting submodules from `<concept>/` directory. Sub-files focus on specific aspects (info, spend, metadata).

## Error Handling

Errors use `thiserror` crate with `#[derive(Error)]`:

**Example:** `crates/chia-sdk-driver/src/driver_error.rs`
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriverError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("try from int error")]
    TryFromInt(#[from] TryFromIntError),

    #[error("try from slice error: {0}")]
    TryFromSlice(#[from] TryFromSliceError),

    #[error("failed to serialize clvm value: {0}")]
    ToClvm(#[from] ToClvmError),

    #[error("failed to deserialize clvm value: {0}")]
    FromClvm(#[from] FromClvmError),

    #[error("clvm eval error: {0}")]
    Eval(#[from] EvalErr),

    #[error("invalid mod hash")]
    InvalidModHash,
    // ... more variants
}
```

**Pattern:**
- Use `#[from]` for error conversions from other types
- Include context in error message: `#[error("context: {0}")]`
- No panicking in library code — return errors via `Result<T, E>`

## Import Organization

Imports are organized in groups (blank line separated):

1. **External crates** (chia-*, clvm-*, crypto libraries)
2. **Chia protocol types** (`chia_protocol`, `chia_puzzle_types`)
3. **Workspace crates** (`chia_sdk_*`)
4. **Standard library** (`std::...`)
5. **Relative imports** (`crate::...`)

**Example from `crates/chia-sdk-driver/src/primitives/cat.rs`:**
```rust
use chia_bls::PublicKey;
use chia_protocol::{Bytes32, Coin};
use chia_puzzle_types::{
    CoinProof, LineageProof, Memos,
    cat::{CatSolution, EverythingWithSignatureTailArgs, GenesisByCoinIdTailArgs},
};
use chia_sdk_types::{
    Condition, Conditions,
    conditions::{CreateCoin, RunCatTail},
    puzzles::RevocationSolution,
    run_puzzle,
};
use clvm_traits::FromClvm;
use clvm_utils::ToTreeHash;
use clvmr::{Allocator, NodePtr};

use crate::{CatLayer, DriverError, Layer, Puzzle, RevocationLayer, Spend, SpendContext};
```

## Documentation

**Doc comments** are enforced by clippy but not all functions require them:

- **Allowed without doc:** internal helper functions, simple getters
- **Required:** public APIs, types, trait implementations, especially if behavior is non-obvious
- **Format:** Standard Rust doc comments with `///` for items and `//!` for module-level docs
- **Doc tests:** Must pass clippy rustdoc lints (compile and run correctly)

**Example:** From `src/primitives/cat.rs`:
```rust
/// Contains all information needed to spend the outer puzzles of CAT coins.
/// The [`CatInfo`] is used to construct the puzzle, but the [`LineageProof`] is needed for the solution.
///
/// The only thing missing to create a valid coin spend is the inner puzzle and solution.
/// However, this is handled separately to provide as much flexibility as possible.
///
/// This type should contain all of the information you need to store in a database for later.
/// As long as you can figure out what puzzle the p2 puzzle hash corresponds to and spend it,
/// you have enough information to spend the CAT coin.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cat {
    pub coin: Coin,
    pub lineage_proof: Option<LineageProof>,
    pub cat_info: CatInfo,
}
```

## Attributes & Derives

Common struct derives:
- `Debug` — required for all public types (clippy: missing_debug_implementations)
- `Clone, Copy, PartialEq, Eq` — for value types and immutable data
- `Serialize, Deserialize` — when crate features enable serde (explicit feature gates)
- `Error` — for error enums (with thiserror crate)

**Common attributes:**
- `#[must_use]` — on functions/types whose result should not be ignored
- `#[cfg(feature = "...")]` — feature-gated code
- `#[cfg(test)]` — test-only modules and imports
- `#[derive(...)]` — bulk derivation of standard traits

---

*Convention analysis: 2026-05-15*
