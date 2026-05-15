# Technology Stack

**Analysis Date:** 2026-05-15

## Languages

**Primary:**
- **Rust** 1.90.0 (pinned in `rust-toolchain.toml`) - Core SDK implementation
- Edition 2024
- Targets: x86_64-apple-darwin, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-apple-darwin, aarch64-unknown-linux-gnu

**Secondary (Bindings):**
- **TypeScript** 4.x - napi and wasm binding tests via AVA
- **Python** 3.8+ - pyo3/maturin binding tests via pytest

## Runtime

**Rust Async Runtime:**
- tokio 1.47.1 with features: sync, time, rt

**Node.js (napi/wasm):**
- Node >= 14 (engine constraint in `napi/package.json`)
- Tested against Node 20, 22, 24
- Package manager: pnpm 9.11.0

**Python (pyo3):**
- Python 3.x (detected via maturin)
- abi3-py38 for stable ABI compatibility

## Frameworks

**Core SDK:**
- tokio-tungstenite 0.24.0 - WebSocket client (TLS-aware)
- reqwest 0.12.23 - HTTP client for REST APIs (Coinset)
- tungstenite 0.24.0 - WebSocket protocol handling

**Cryptography:**
- chia-bls 0.36.1 - BLS signing (Chia reference)
- chia-secp 0.36.1 - SECP256k1 operations
- chia-sha2 0.36.1 - SHA2 hashing
- sha2 0.10.9, sha3 0.10.8 - Additional hash support
- k256 0.13.4, p256 0.13.2, signature 2.2.0 - ECDSA/ECDH

**Chia Protocol:**
- chia-protocol 0.36.1 - Wire protocol messages (peer protocol)
- chia-consensus 0.36.1 - Consensus rules
- chia-traits 0.36.1 - Serialization traits
- chia-puzzle-types 0.36.1 - Standard puzzle definitions
- chia-puzzles 0.20.3 - Puzzle implementations
- clvmr 0.16.2 - Chialisp VM
- clvm-traits 0.36.1, clvm-utils 0.36.1 - CLVM utilities
- chialisp 0.4.1 - Chialisp compiler

**Bindings:**
- napi 3.3.0, napi-derive 3.2.5 - Node.js/napi-rs
- pyo3 0.23.5, pyo3-async-runtimes 0.23 - Python bindings
- wasm-bindgen 0.2.100, wasm-bindgen-futures 0.4.50 - WebAssembly

**Testing:**
- rstest 0.22.0 - Parametric tests
- tokio (full features) in dev - Async testing

**Serialization:**
- serde 1.0.228, serde_json 1.0.145 - JSON/serialization
- bincode 2.0.1 - Binary encoding
- bech32 0.9.1 - Address encoding

**Build/Code Generation:**
- syn 2.0.106, quote 1.0.41, proc-macro2 1.0.101 - Procedural macros
- convert_case 0.8.0 - Case conversion utilities
- napi-build 3.0.0-beta.0 - napi build support

**Utilities:**
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

**TLS Options:**
- native-tls 0.2.14 - Platform-native TLS (OpenSSL on Linux, Secure Transport on macOS, SChannel on Windows)
- rustls 0.23.32 - Pure-Rust TLS implementation
- rustls-pemfile 2.2.0 - PEM certificate parsing
- aws-lc-rs 1.x - AWS crypto library (bindgen support for rustls)

**Offer Compression (conditional):**
- flate2 1.1.4 with zlib-ng-compat - DEFLATE compression for offer wire format

**Compiler/Code Gen:**
- rue-compiler 0.6.0, rue-options 0.6.0, rue-lir 0.6.0 - Rue language compilation

**JavaScript/TypeScript dependencies (napi only):**
- @napi-rs/cli 3.0.0-alpha.91 - napi build tool
- @types/node 22.13.1
- ava 7.0.0 - Test runner
- ts-node 10.9.2 - TypeScript execution

**WebAssembly (wasm only):**
- wasm-pack (installed via cargo-install) - WASM build tooling
- getrandom 0.3.4 - Random number generation in WASM

## Configuration

**Workspace Configuration:**
- `[workspace.dependencies]` in `Cargo.toml` at root specifies all shared versions
- All member crates use `{ workspace = true }` for dependency references
- Strict linting configuration applied via `[workspace.lints]`

**Linting (Workspace-enforced):**
```
[workspace.lints.rust]
- unsafe_code = "deny" (no unsafe blocks)
- dead_code = "deny" (all code must be used)
- Rust 2018/2021 idioms as "deny"
- nonstandard_style as "deny"

[workspace.lints.clippy]
- all = "deny" (deny all clippy warnings)
- pedantic = "warn" (warn on style suggestions)
- cargo = "warn" (warn on Cargo.toml practices)
```

**Build Profiles:**
- `[profile.release]` with LTO enabled and symbol stripping for published binaries

**Environment Variables:**
- None detected as critical; bindings discovery is compile-time

## Platform Requirements

**Development:**
- Rust 1.90.0 (via rustup with `rust-toolchain.toml`)
- cargo-binstall for rapid tool installation in CI
- cargo-workspaces for multi-crate operations
- cargo-machete for dependency verification
- For napi: Node.js 20+ and pnpm 9
- For wasm: wasm-pack (cargo-installed)
- For pyo3: Python 3.8+ and maturin

**Production:**
- Deployment as Rust library (vendored in SDK consumer projects)
- Or as published npm packages: `chia-wallet-sdk` (napi) and `chia-wallet-sdk-wasm` (wasm)
- Or as published Python package: `chia_wallet_sdk` (pyo3/maturin wheels)

## Feature Flags (Workspace-level)

**Puzzle Layers:**
- `chip-0035` - CHIP-0035 puzzles (vault/MIPS family). Gates `chia-sdk-types/chip-0035` and `chia-sdk-driver/chip-0035`
- `chip-0037` - CHIP-0037 puzzles (EIP-712/controller-puzzle scaffolding). Gates types + driver features + optional sha3 dependency

**Action System:**
- `action-layer` - Higher-level action layer driver code (SpendAction builder). Gates `chia-sdk-types/action-layer` and `chia-sdk-driver/action-layer`

**Compression:**
- `offer-compression` - Flate2-backed compressed offers wire format. Gates `chia-sdk-driver/offer-compression` + optional flate2 and chia-sdk-utils

**TLS (choose one):**
- `native-tls` - Platform-native TLS for `chia-sdk-client`, `chia-sdk-coinset`, `chia-sdk-daemon`. Gates tokio-tungstenite/native-tls + reqwest/native-tls
- `rustls` - Pure-Rust TLS for the same. Gates tokio-tungstenite/rustls-tls-webpki-roots + rustls + rustls-pemfile + aws-lc-rs + reqwest/rustls-tls

**Test Utilities:**
- `peer-simulator` - In-process peer for `chia-sdk-test`. Exposes Simulator for integration tests

**Binding features (in member crate manifests):**
- `napi` in `chia-sdk-bindings/Cargo.toml` - Enables napi-specific binding generation
- `wasm` in `chia-sdk-bindings/Cargo.toml` - Enables wasm-bindgen binding generation
- `pyo3` in `chia-sdk-bindings/Cargo.toml` - Enables pyo3 binding generation

## Build & Test Commands (as used by CI)

**Rust Core (`.github/workflows/rust.yml`):**
```bash
# Lint
cargo fmt --all -- --files-with-diff --check
cargo clippy --workspace --all-features --all-targets
cargo machete                      # Detect unused dependencies

# Build (per-crate for feature coverage)
cargo build --release
cargo build --release --all-features
cargo build --release -p chia-sdk-driver [--all-features]
cargo build --release -p chia-sdk-client [--all-features]
cargo build --release -p chia-sdk-coinset [--all-features]
cargo build --release -p chia-sdk-daemon [--all-features]
cargo build --release -p chia-sdk-types [--all-features]
cargo build --release -p chia-sdk-test [--all-features]
cargo build --release -p chia-sdk-signer
cargo build --release -p chia-sdk-utils
cargo build --release -p chia-sdk-cli
cargo build --release -p chia-sdk-bindings -F napi
cargo build --release -p chia-sdk-bindings -F wasm
cargo build --release -p chia-sdk-bindings -F pyo3

# Tests (excludes binding crates)
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

**Node.js (napi) (`.github/workflows/napi.yml`):**
```bash
cd napi
pnpm install
pnpm build:macos-x64             # or other platform targets
pnpm test                         # AVA test runner
```

**WebAssembly (`.github/workflows/wasm.yml`):**
```bash
cd wasm
pnpm install
pnpm test                         # wasm-pack build --target nodejs + AVA
wasm-pack build                   # Create pkg/
wasm-pack pack                    # Create npm .tgz
```

**Python (pyo3) (`.github/workflows/pyo3.yml`):**
```bash
cd pyo3
maturin develop                   # Build and install in-place
pytest                            # Run Python tests

# Platform-specific wheel builds (cross-platform via maturin-action)
maturin build --release --out dist --find-interpreter
  [--manylinux manylinux_2_28 | musllinux_1_2]  # Linux variants
  [--target x86_64 | aarch64]
```

## Workspace Member Crates

**Core Implementation:**
- `crates/chia-sdk-driver` - High-level puzzle layer composition and spend building
- `crates/chia-sdk-types` - Type definitions, conditions, puzzle info
- `crates/chia-sdk-client` - Light-wallet peer protocol client (WebSocket + TLS)
- `crates/chia-sdk-coinset` - Coinset REST API client
- `crates/chia-sdk-daemon` - Chia daemon websocket RPC client
- `crates/chia-sdk-signer` - BLS/SECP signature computation
- `crates/chia-sdk-test` - Test utilities and in-process simulator
- `crates/chia-sdk-utils` - Bech32 addresses, coin selection, hex parsing
- `crates/chia-sdk-cli` - Command-line utility

**Bindings Infrastructure:**
- `crates/chia-sdk-bindings` - Pure-Rust facades for cross-language API
- `crates/chia-sdk-bindings/bindy` - Runtime type conversion traits
- `crates/chia-sdk-bindings/bindy-macro` - Procedural macro for generating bindings
- `crates/chia-sdk-types/derive` - Custom derive macros for types
- `napi/` - Node.js native addon (generates .node files per platform)
- `wasm/` - WebAssembly module (wasm-bindgen, generates npm package)
- `pyo3/` - Python C extension (maturin, generates .so/.pyd wheels)
- `pyo3/stub-generator` - Auto-generates .pyi type stubs for Python

**Entry Point:**
- `src/` - Umbrella crate `chia-wallet-sdk`, re-exports internals under short names
  - `lib.rs` re-exports `chia_sdk_*` crates
  - `prelude.rs` curated public surface for consumers

---

*Stack analysis: 2026-05-15*
