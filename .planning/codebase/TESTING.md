# Testing Patterns

**Analysis Date:** 2026-05-15

## Rust Testing

### Test Runner

**Framework:** Rust built-in test harness via `cargo test`

**Workspace Test Command:**
```bash
cargo test --release --workspace \
  --exclude chia-wallet-sdk-napi \
  --exclude chia-sdk-derive \
  --exclude chia-wallet-sdk-py \
  --exclude chia-wallet-sdk-wasm \
  --exclude chia-sdk-bindings \
  --exclude bindy \
  --exclude bindy-macro \
  --all-features
```

This is the command used in CI (`.github/workflows/rust.yml`). The excluded crates are binding generators and FFI crates tested separately.

**Development Tests:**
```bash
cargo test                    # Default (debug build)
cargo test --release         # Optimized (faster for integration tests)
cargo test --lib             # Unit tests only
cargo test --test '*'        # Integration tests only
cargo test <test_name>       # Run specific test by name
cargo test -- --nocapture    # Show println! output
```

### Test Organization

**Location:** Test modules live within source files using `#[cfg(test)] mod tests { ... }`

**Module co-location pattern:**
```rust
// In src/primitives/cat.rs
pub struct Cat { ... }

impl Cat {
    pub fn spend(...) -> Result<()> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cat_spending() -> Result<()> {
        // ...
    }
}
```

**Naming:** Test functions prefixed with `test_` (conventional); test module always named `tests`.

### Test Framework: chia-sdk-test

**Location:** `crates/chia-sdk-test/src/`

**Exports:**
- `Simulator` — in-process blockchain simulator for testing coin spends
- `BlsPair` — BLS keypair with seed-based generation
- `K1Pair` — secp256k1 keypair
- `R1Pair` — P256 keypair
- `BlsPairWithCoin` — BLS keypair + coin
- `SimulatorConfig` — configure simulator behavior (seed, constants)
- `Benchmark` — measure spend size/cost
- Helper functions: `sign_transaction()`, `validate_clvm_and_signature()`

**Files:**
- `src/simulator.rs` — main `Simulator` implementation
- `src/key_pairs.rs` — `BlsPair`, `K1Pair`, `R1Pair` with seed-based generation
- `src/peer_simulator.rs` — gated by `peer-simulator` feature, provides in-process peer
- `src/benchmark.rs` — spend cost/size measurement

### Test Pattern: Simulator-Based Integration Tests

**Example from `crates/chia-sdk-driver/src/offers/offer.rs`:**
```rust
#[cfg(test)]
mod tests {
    use chia_sdk_test::{Simulator, sign_transaction};
    use indexmap::indexmap;
    use crate::{...};

    #[test]
    fn test_offer_nft_for_nft() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        // Create keypairs
        let alice = sim.bls(2);
        let bob = sim.bls(0);

        // Set up coins via CAT/NFT creation
        let cat_info = CatInfo::new(asset_id, None, alice.puzzle_hash);
        
        // Issue CAT coin
        let eve = Cat::new(
            Coin::new(alice.coin.coin_id(), cat_info.puzzle_hash(), 1),
            None,
            cat_info,
        );

        // Spend the coin
        eve.spend(&mut ctx, alice.pk, standard_conditions)?;

        // Submit spend bundle to simulator
        sim.spend_coins(ctx.take(), &[alice.sk])?;

        // Verify result (coins created, balances correct, etc.)
        Ok(())
    }
}
```

**Key patterns:**
1. Create `Simulator` and `SpendContext`
2. Generate keypairs: `sim.bls(seed)` returns `BlsPair` with `sk`, `pk`, `puzzle_hash`, `coin`
3. Build puzzle structures (CAT, NFT, etc.) using `chia-sdk-driver` types
4. Construct spend conditions via `crate::Layer` implementations
5. Submit spend bundle: `sim.spend_coins(ctx.take(), &secret_keys)?`
6. Assert expected state changes

**Keypair generation:**
```rust
// BLS keypair from seed
let alice = sim.bls(123);          // Returns BlsPair
let alice_pk = alice.pk;            // PublicKey
let alice_sk = alice.sk;            // SecretKey
let alice_puzzle_hash = alice.puzzle_hash;  // Bytes32

// Generate multiple keypairs
let pairs: Vec<BlsPair> = BlsPair::range_vec_with_seed(0, 5);  // 5 keypairs, seed 0-4

// Secp256k1 (K1) and P256 (R1) pairs
let k1_pair = sim.k1(seed);  // Returns K1Pair with sk, pk
let r1_pair = sim.r1(seed);  // Returns R1Pair with sk, pk
```

### Parametrized Tests: rstest

The workspace includes `rstest = "0.22.0"` in `[workspace.dependencies]`.

**Usage examples:**

From `crates/chia-sdk-driver/src/primitives/streamed_asset.rs`:
```rust
#[cfg(test)]
mod tests {
    use rstest::rstest;
    use chia_sdk_test::{Simulator, Benchmark};

    #[rstest]
    fn test_streamed_asset(#[values(true, false)] xch_stream: bool) -> anyhow::Result<()> {
        let mut ctx = SpendContext::new();
        let mut sim = Simulator::new();
        let mut benchmark = Benchmark::new(format!(
            "Streamed {}",
            if xch_stream { "XCH" } else { "CAT" }
        ));

        // Test runs twice: once with xch_stream=true, once with false
        // ...
        Ok(())
    }
}
```

From `crates/chia-sdk-driver/src/primitives/clawback_v2.rs`:
```rust
#[rstest]
fn test_clawback_v2_early_claim() -> anyhow::Result<()> { ... }

#[rstest]
#[values(1000, 2000, 3000)]
fn test_various_delays(delay_seconds: u64) -> anyhow::Result<()> { 
    // Runs 3 times with different delay values
    // ...
}
```

**rstest features:**
- `#[values(...)]` — parameterize test over multiple values
- `#[case(...)]` — case-based parameterization (data-driven tests)
- Fixture support — reusable test setup (rarely used in this codebase)

### Test Utilities

**Benchmark tool:** `Benchmark::new(name)` in `src/benchmark.rs`
```rust
let mut benchmark = Benchmark::new("CAT Transfer".to_string());
// ... perform spends ...
benchmark.finalize(&spend_bundle)?;  // Prints cost/size metrics
```

**Result assertions:** Standard Rust assertions plus `anyhow::Result<()>` for error propagation
```rust
#[test]
fn my_test() -> anyhow::Result<()> {
    let result = some_operation()?;  // Propagate errors
    assert_eq!(result, expected);    // Standard assertions
    Ok(())
}
```

**Error propagation pattern:**
- Use `?` operator instead of `.unwrap()` in tests
- Return `anyhow::Result<()>` to surface failures
- Test framework prints full error chain on failure

## Binding Tests

### NAPI (Node.js) Tests

**Location:** `napi/__test__/`

**Framework:** AVA (`ava ^7.0.0`)

**Configuration:** `napi/package.json`
```json
{
  "packageManager": "pnpm@9.11.0",
  "scripts": {
    "test": "ava"
  },
  "ava": {
    "timeout": "3m",
    "cache": false,
    "verbose": true,
    "workerThreads": false,
    "extensions": ["ts"],
    "require": ["./register-ts-node.cjs"]
  }
}
```

**Run tests:**
```bash
cd napi
pnpm test
```

**Test files:** `*.spec.ts` — examples: `options.spec.ts`, `nfts.spec.ts`, `bulletins.spec.ts`, `mips_memos.spec.ts`

**Example test pattern from `napi/__test__/options.spec.ts`:**
```typescript
import test from "ava";
import {
  Cat,
  CatInfo,
  CatSpend,
  Clvm,
  Coin,
  OptionContract,
  OptionInfo,
  Simulator,
  Spend,
} from "../index.js";

test("mints and spends an option", (t) => {
  const sim = new Simulator();
  const clvm = new Clvm();

  const alice = sim.bls(2n);
  
  const tail = clvm.nil();
  const assetId = tail.treeHash();
  const catInfo = new CatInfo(assetId, null, alice.puzzleHash);

  // Create coin
  clvm.spendStandardCoin(
    alice.coin,
    alice.pk,
    clvm.delegatedSpend([clvm.createCoin(catInfo.puzzleHash(), 1n)])
  );

  // Build CAT
  const eve = new Cat(
    new Coin(alice.coin.coinId(), catInfo.puzzleHash(), 1n),
    null,
    catInfo,
  );

  // Assertions
  t.is(eve.assetId.toString(), assetId.toString());
});
```

**pnpm version:** Pinned to `9.11.0` in `package.json` `packageManager` field.

### WASM Tests

**Location:** `wasm/__test__/`

**Framework:** AVA with wasm-pack

**Configuration:** `wasm/package.json`
```json
{
  "scripts": {
    "test": "wasm-pack build --target nodejs && ava"
  },
  "ava": {
    "timeout": "3m",
    "extensions": ["ts"],
    "require": ["ts-node/register"]
  }
}
```

**Run tests:**
```bash
cd wasm
pnpm test  # Builds wasm with wasm-pack, then runs AVA
```

**Test file:** `wasm/__test__/wasm.spec.ts`

**Example pattern from `wasm/__test__/wasm.spec.ts`:**
```typescript
import test from "ava";
import {
  Clvm,
  CreateCoin,
  PublicKey,
  Signature,
  toHex,
  fromHex,
  setPanicHook,
} from "../pkg";

setPanicHook();  // Catch panics from Rust

test("functions return Uint8Array rather than Buffer", (t) => {
  const array = fromHex("00");
  t.assert(!(array instanceof Buffer));
  t.assert(array instanceof Uint8Array);
});

test("alloc", (t) => {
  const clvm = new Clvm();
  const program = clvm.alloc([
    clvm.nil(),
    PublicKey.infinity(),
    "Hello, world!",
    42n,
    true,
    new Uint8Array([1, 2, 3]),
  ]);
  
  t.is(
    toHex(program.serialize()),
    "ff80ffb0c00000000000000000000000000000000000000000000000000000000000000000..."
  );
});
```

**Key pattern:** wasm-pack builds the Rust code to WebAssembly, then TypeScript tests import from `../pkg/`.

### PyO3 (Python) Tests

**Location:** `pyo3/tests/`

**Framework:** pytest

**Setup:**
```bash
cd pyo3
maturin develop  # Build Python extension in dev mode
pytest tests/
```

**Test file:** `pyo3/tests/test_pyo3.py`

**Example pattern from `pyo3/tests/test_pyo3.py`:**
```python
from chia_wallet_sdk import Clvm, PublicKey, RunCatTail, to_hex

def test_alloc():
    clvm = Clvm()
    
    program = clvm.alloc([
        clvm.nil(),
        PublicKey.infinity(),
        "Hello, world!",
        42,
        100,
        True,
        bytes([1, 2, 3]),
        bytes.fromhex("00" * 32),
        None,
        None,
        RunCatTail(clvm.nil(), clvm.nil()),
    ])
    
    assert (
        to_hex(program.serialize())
        == "ff80ffb0c00000000000000000000000000000000000000000000000000000000000000000..."
    )
```

## Dependency Checking

**Tool:** `cargo machete` runs in CI to flag unused dependencies.

**Crates with exceptions:** Some binding crates declare metadata to ignore re-exported proc-macro dependencies:

From `crates/chia-sdk-test/Cargo.toml`:
```toml
[package.metadata.cargo-machete]
ignored = ["prettytable-rs"]
```

From `crates/chia-sdk-daemon/Cargo.toml`:
```toml
[package.metadata.cargo-machete]
ignored = ["chia-sdk-client"]
```

From `crates/chia-sdk-client/Cargo.toml`:
```toml
[package.metadata.cargo-machete]
ignored = ["aws-lc-rs"]
```

These dependencies may be used via trait implementations or re-exports that cargo-machete doesn't detect. The `ignored` list tells machete to skip the warning.

## Test Coverage

No enforced coverage target is detected in the codebase configuration. Tests focus on integration tests via `Simulator` rather than unit test percentages.

**Coverage analysis approach:**
- Unit tests co-located with implementations (verify single functions)
- Integration tests via `Simulator` (verify end-to-end spends)
- Parametrized tests with `rstest` (test multiple scenarios with same code)

## Feature Gating in Tests

Tests can be feature-gated:

**Example from `crates/chia-sdk-test/src/simulator.rs`:**
```rust
#[cfg(feature = "serde")]
pub fn serialize(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
    bincode::serde::encode_to_vec(&self.data, bincode::config::standard())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde")]
    #[test]
    fn test_serialization() -> anyhow::Result<()> {
        let sim = Simulator::new();
        let bytes = sim.serialize()?;
        // ...
    }
}
```

Run feature-gated tests:
```bash
cargo test --all-features     # Run all tests including feature-gated
cargo test --features chip-0035  # Run with specific feature
```

---

*Testing analysis: 2026-05-15*
