# Phase 1: Crypto primitives & workspace integration — Research

**Researched:** 2026-05-15
**Domain:** Cryptographic foundation for CHIP-0057 silent payments in `chia-sdk-types` + workspace feature integration across `chia-sdk-types`/`chia-sdk-utils`/`chia-sdk-driver`
**Confidence:** HIGH

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CRYPTO-01 | `ScalarField` newtype performs unsigned mod-r reduction over BLS12-381 subgroup order using `num-bigint 0.4.6`. Distinct from existing signed `mod_by_group_order`; no public constructor that accepts "signed" bytes. | Standard Stack table + reference prototype `~/silent-payments/crates/sp-common/src/scalar.rs`; verified `chia-puzzle-types-0.36.1/src/derive_synthetic.rs:39` is the existing signed function we are deliberately *not* using. |
| CRYPTO-02 | `tagged_hash(tag, data) -> [u8; 32]` implements BIP-340-style `SHA256(SHA256(tag) ‖ SHA256(tag) ‖ data)` using `chia-sha2`; expose `CHIA_SP_INPUTS`/`CHIA_SP_SHARED_SECRET`/`CHIA_SP_LABEL` as `pub const &'static str`. | Reference impl `tagged_hash.rs` confirms the construction; chia-sha2 API verified (uses `new/update/finalize`, NOT `::digest()`). |
| WS-01 | `chip-0057` workspace feature in root `Cargo.toml`, cascading to `chia-sdk-types/chip-0057`, `chia-sdk-driver/chip-0057`, `chia-sdk-utils/chip-0057`. | Mirrors `chip-0035` (line 73 root Cargo.toml) and `chip-0037` (line 74 + bbc7f57f precedent commit). |
| WS-02 | CI per-crate `-F chip-0057` build lines in `.github/workflows/rust.yml`. | Verified existing CI matrix; current file has `cargo build --release -p chia-sdk-types --all-features` already covering cumulative case. Per-crate `-F chip-0057` lines must be added for `chia-sdk-types`, and the `chip-0057` cascade is exercised by `--all-features`. |
| WS-03 | All chip-0057 code passes `deny clippy::all`, `warn pedantic`, `deny unsafe_code`, `deny dead_code`, and `cargo machete`. | Confirmed workspace lint policy in root `Cargo.toml` lines 32–67. `cargo machete` runs in CI. No new workspace deps required — all imports already live in `[workspace.dependencies]` and `chia-sdk-types/Cargo.toml`. |

---

## Summary

Phase 1 is the cryptographic seed crystal: `ScalarField` (unsigned mod-r reduction newtype), `tagged_hash` + three pinned domain-tag constants, and CHIP-0057 workspace feature cascade. The crypto is small (~150 LOC across `scalar.rs`, `tagged_hash.rs`, `mod.rs`), but it is load-bearing: every later phase (address derivation, scanner, send action) flows through these types. The signed-vs-unsigned scalar boundary is the #1 critical-path correctness pitfall for the entire project, and `ScalarField` is the type-system mechanism that prevents it. There is zero algorithmic ambiguity — the reference implementation at `~/silent-payments/crates/sp-common/src/{scalar,tagged_hash}.rs` is a working Rust port already validated against the CHIP test vectors; Phase 1 ports it into the SDK conventions (chia-sha2 instead of `sha2`, workspace deps, feature gating).

The workspace integration follows the `chip-0037` precedent (commit `bbc7f57f`) literally: a new feature in three `Cargo.toml` files plus a CI line. No new workspace dependencies. No new crates.

**Primary recommendation:** Port `scalar.rs` and `tagged_hash.rs` from `~/silent-payments/crates/sp-common/src/` verbatim into `crates/chia-sdk-types/src/silent_payments/`, gated behind `#[cfg(feature = "chip-0057")]`, with two SDK-conformance substitutions: (a) `sha2::Sha256` → `chia_sha2::Sha256` (using the `new/update/finalize` API, NOT a `::digest()` static method which chia-sha2 doesn't expose), and (b) hash-tag constants exposed as `pub const &'static str`. Add the `chip-0057` feature with empty deps list (`chip-0057 = []` — just like `chip-0035`) to `chia-sdk-types/Cargo.toml`, cascade through root + driver + utils, and add a single CI line. Phase 1 deliberately does **not** introduce `SilentPaymentKeys` derivation helpers — derivation paths are constants only (`pub const SCAN_PATH: &[u32]` / `SPEND_PATH`), the derivation function lives in `chia-sdk-utils` and is Phase 2's responsibility per ARCHITECTURE.md's module layout.

---

## Project Constraints (from CLAUDE.md)

These are authoritative — research conclusions and Standard Stack choices comply:

- **Rust 1.90.0, edition 2024.** No nightly features. Pinned in `rust-toolchain.toml`.
- **Workspace lints are deny-level.** `unsafe_code = "deny"`, `dead_code = "deny"`, `deny clippy::all`, `warn clippy::pedantic`, `warn clippy::cargo`. Phase 1 code must pass clippy with `-D warnings` cleanly.
- **New crates must declare `[lints] workspace = true`.** Phase 1 adds no new crates, so this is automatic.
- **Dependency versions live in `[workspace.dependencies]`.** Member crates reference via `{ workspace = true }`. Do NOT pin a fresh version in a member Cargo.toml.
- **Chia upstream deps are pinned at `0.36.1` (and `chia-puzzles = 0.20.3`, `clvmr = 0.16.2`).** No bumps in this phase or any phase of this project.
- **Each crate built individually in CI with and without `--all-features`.** `chip-0057` code MUST compile in every permutation: (a) `cargo build -p chia-sdk-types` (no features), (b) `cargo build -p chia-sdk-types --all-features`, (c) `cargo build -p chia-sdk-types -F chip-0057`, (d) `cargo build --workspace --all-features`, (e) `cargo build --workspace` (no features).
- **`cargo machete` runs in CI.** New `[dependencies]` entries must be referenced in code, not transitively-only. Use `[package.metadata.cargo-machete] ignored = [...]` ONLY for genuinely unused-but-required deps; the phase success criteria forbid new entries here.
- **No `unsafe` blocks anywhere in CHIP-0057 code.** `num-bigint`, `chia-sha2`, `chia-bls` (FFI hidden) are all safe Rust at the call site.
- **GSD workflow enforcement is active.** Phase 1 implementation will go through `/gsd:execute-phase`; this RESEARCH.md will feed the planner.
- **LSP preferred over Grep for code navigation.** Diagnostics must be clean after every edit. After writing Rust, run clippy before reporting done.

---

## Standard Stack

### Core (already in workspace — zero new deps)

| Library | Workspace Version | Purpose | Why Standard |
|---------|-------------------|---------|--------------|
| `num-bigint` | 0.4.6 | `BigUint::from_bytes_be(&bytes) % BigUint::from_bytes_be(&GROUP_ORDER)` inside `ScalarField::from_bytes_unsigned`. | Already in `[workspace.dependencies]` and pulled by `chia-sdk-driver`. **Critical**: use `BigUint::from_bytes_be` (unsigned) — explicitly NOT `BigInt::from_signed_bytes_be` which is what `mod_by_group_order` uses for synthetic offsets. Pure-Rust, no FFI, no `unsafe` → satisfies `unsafe_code = "deny"`. |
| `chia-sha2` | 0.36.1 | All SHA-256 operations — `tagged_hash` inner+outer hashes, and the tag-pin unit test. | SDK convention. Every existing module hashing in `chia-sdk-types` uses `chia_sha2::Sha256` (verified: `merkle_tree.rs:5`, `constants.rs:5`, `condition/announcements.rs:2`). Mixing `sha2` + `chia-sha2` in a new module would trip code review. API: `let mut h = Sha256::new(); h.update(buf); let out: [u8; 32] = h.finalize();`. **There is no `::digest()` static method** — see Pitfalls. |
| `hex-literal` | 0.4.1 | Compile-time `[u8; 32]` constants for `GROUP_ORDER` and pinned tag-hash test expectations. | Pattern used throughout `chia-puzzle-types::derive_synthetic` (`hex!("73eda753299d7d4833...")`) and existing puzzles. Compile-time failure on typos beats runtime. |
| `chia-bls` | 0.36.1 | NOT actively used in Phase 1, but stays in chia-sdk-types `[dependencies]` (already present). Phase 2 needs `SecretKey::from_seed`, `derive_unhardened`, etc. | Already declared; nothing to add. |
| `thiserror` | 2.0.17 | Reserved for later phases (error types). Not used in Phase 1. | Already in `chia-sdk-types` deps. |

### Test-only (dev-dependencies, already present)

| Library | Workspace Version | Purpose | When to Use |
|---------|-------------------|---------|-------------|
| `hex` | 0.4.3 | Optional — runtime hex-string assertions in test diagnostics. | Already in `chia-sdk-types/[dev-dependencies]`. |
| `rstest` | 0.22.0 | Optional — parametric tests if multiple `ScalarField` adversarial cases want a table format. | Already a dev-dep; pattern in `merkle_tree.rs:152`. Phase 1 has few enough tests that plain `#[test]` is fine. |
| `anyhow`, `rand`, `rand_chacha` | (workspace) | Not needed in Phase 1. | Reserved for later phases. |

### Installation

**No installations.** All dependencies already present.

**Version verification** (run before planning to confirm):

```bash
# Already-confirmed via on-disk crate source at ~/.cargo/registry/src/
# - num-bigint 0.4.6 → /home/kdc/.cargo/registry/.../num-bigint-0.4.6/
# - chia-sha2 0.36.1 → confirmed Sha256 { new/update/finalize } API, NO ::digest()
# - chia-bls 0.36.1 → DerivableKey trait, scalar_multiply, is_inf, from_seed all present
# - chia-puzzle-types 0.36.1 → mod_by_group_order at derive_synthetic.rs:39 (the
#   SIGNED helper we are deliberately NOT reusing)
```

Run if any doubt: `cargo tree -p chia-sdk-types --depth 1` and confirm `num-bigint 0.4.6`, `chia-sha2 0.36.1`, `hex-literal 0.4.1` are pulled.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `num-bigint` `BigUint % r` | `blst`/`bls12_381`/`ark-bls12-381` proper `Fr` type with constant-time arithmetic | **Long-term yes, this phase no.** `blst` adds an FFI/`unsafe` boundary; `ark-bls12-381` adds a 3rd BLS library to the workspace. The signed `mod_by_group_order` in `chia-puzzle-types::derive_synthetic` already uses `num-bigint` — keeping parity with that idiom is more important for review-clarity at the type boundary. Future-optimization: hot-swap the inner impl of `ScalarField` without touching call sites. |
| `chia-sha2` | `sha2 0.10.9` (bare crate) | The reference impl uses bare `sha2`. SDK convention is `chia-sha2`. Switching is one substitution. No correctness difference. |
| `chip-0057 = []` empty feature list in chia-sdk-types | `chip-0057 = ["dep:num-bigint"]` (gated dep) | Empty list matches the existing `chip-0035 = []` and `chip-0037 = []` patterns exactly. `num-bigint` and `chia-sha2` are already unconditional deps in `chia-sdk-types`. Adding `optional = true` to existing deps would be a needless complication. |
| Newtype `ScalarField([u8; 32])` | Newtype around `chia_bls::SecretKey` | A `SecretKey` rejects values `>= r` AND zero (returns `Result`). `ScalarField` represents an arbitrary mod-r residue, including zero (which is legal: the spec covers zero-aggregate cases as `is_inf()` checks at the *point* layer, not the scalar layer). `[u8; 32]` matches the prototype. |
| Place crypto in `chia-sdk-utils` | Keep in `chia-sdk-types` (recommended) | `chia-sdk-types` already declares all the deps we need (`chia-bls`, `chia-puzzle-types`, `chia-sha2`, `hex-literal`); `chia-sdk-utils` does not. ARCHITECTURE.md's planned layout puts crypto in types, and CHIP-0037 placed `p2_eip712_message`/`p2_controller_puzzle` types in `chia-sdk-types`. Match the precedent. |

---

## Architecture Patterns

### Recommended File Structure (Phase 1 only)

```
crates/chia-sdk-types/src/
├── lib.rs                          # ADD: pub mod silent_payments; gated by feature
└── silent_payments/                # NEW directory, all gated by #[cfg(feature = "chip-0057")]
    ├── mod.rs                      # Barrel — re-exports public surface
    ├── scalar.rs                   # ScalarField newtype + GROUP_ORDER const
    ├── tagged_hash.rs              # tagged_hash() + CHIA_SP_* tag string consts
    └── paths.rs                    # SCAN_PATH / SPEND_PATH derivation-path constants
```

**Why a directory not flat files:** Mirrors `crates/chia-sdk-types/src/puzzles/datalayer/`, `mips/`, `action_layer/` — established pattern when more than one file relates. The barrel `mod.rs` follows the same shape as `puzzles.rs` lines 23-43 (`#[cfg(feature = "chip-XXXX")] mod x; #[cfg(feature = "chip-XXXX")] pub use x::*;`).

### Workspace Feature Cascade Pattern

Phase 1 introduces the cascade — Phases 2/3/4 inherit it.

**Root `Cargo.toml` — add to `[features]` block (currently lines 72-79):**

```toml
chip-0057 = [
    "chia-sdk-driver/chip-0057",
    "chia-sdk-types/chip-0057",
    "chia-sdk-utils/chip-0057",
]
```

Insert immediately after `chip-0037 = [...]` line 74. The umbrella crate's lib.rs re-exports rely on these.

**`crates/chia-sdk-types/Cargo.toml` — add to `[features]` block (currently lines 17-20):**

```toml
chip-0057 = []
```

Append after `action-layer = []`. Empty list mirrors `chip-0035 = []` and `chip-0037 = []`. **All deps `silent_payments` needs (`num-bigint`, `chia-sha2`, `hex-literal`, `chia-bls`) are already in this crate's unconditional `[dependencies]`.**

**`crates/chia-sdk-driver/Cargo.toml` — add to `[features]` block (currently lines 20-24):**

```toml
chip-0057 = ["chia-sdk-types/chip-0057"]
```

Insert after `chip-0037 = [...]`. Phase 1 itself doesn't add code to the driver, but the feature must cascade so Phase 3/4 work (driver will need to see `silent_payments::ScalarField` etc. through `chia-sdk-types`).

**`crates/chia-sdk-utils/Cargo.toml` — add NEW `[features]` block:**

```toml
[features]
chip-0057 = []
```

`chia-sdk-utils` currently has no `[features]` table at all (verified by reading the file end-to-end). Add the block. Phase 1 itself adds no code here, but cascade is required for Phase 2 (`SilentPaymentKeys`, `SilentPaymentAddress`).

**Optional dep edge** (deferred to Phase 2 entry — flagged in STATE.md "Phase 2 pre-flight: Audit downstream consumers of `chia-sdk-utils` for the new optional `chia-sdk-types -> chia-sdk-utils` dep edge (Q8)"): Phase 2 will add `chia-sdk-types = { workspace = true, optional = true }` and change to `chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057"]`. **Not Phase 1's problem.** Phase 1 only needs the empty feature.

### Module Re-Export Pattern

**`crates/chia-sdk-types/src/lib.rs` — add gated `pub mod` (after current line 8 `mod run_puzzle;`):**

```rust
#[cfg(feature = "chip-0057")]
pub mod silent_payments;
```

The module is a `pub mod` (not `mod` + barrel `pub use`) because the silent-payments surface has its own namespace (`chia_sdk_types::silent_payments::ScalarField`) — matching `pub mod puzzles;` at line 1. The current crate keeps `mod condition; pub use condition::*;` for flat-namespace types but uses `pub mod puzzles` for nested ones. Silent payments is the latter.

### `silent_payments/mod.rs` Barrel

```rust
//! CHIP-0057 silent-payments cryptographic primitives.
//!
//! All scalar values that participate in the silent-payments ECDH protocol
//! (`input_hash`, `t_k`, `label_scalar`) are reduced mod r via [`ScalarField`]
//! using UNSIGNED interpretation. Do not mix with `chia_puzzle_types::derive_synthetic::mod_by_group_order`
//! which uses SIGNED interpretation for synthetic-key offsets — the two are not
//! interchangeable and silently disagree on ~50% of digest inputs.
//!
//! See CHIP-0057 "Synthetic Key Computation Note" for the full rationale.

mod paths;
mod scalar;
mod tagged_hash;

pub use paths::{SCAN_PATH, SPEND_PATH};
pub use scalar::{GROUP_ORDER, ScalarField};
pub use tagged_hash::{CHIA_SP_INPUTS, CHIA_SP_LABEL, CHIA_SP_SHARED_SECRET, tagged_hash};
```

### `silent_payments/scalar.rs` — Public API Surface (Phase 1 scope only)

Port from `~/silent-payments/crates/sp-common/src/scalar.rs` verbatim with two changes:
1. Module-level rustdoc tightened to call out the `mod_by_group_order` asymmetry explicitly.
2. Derive `Copy` (the prototype has only `Clone`; for a `[u8; 32]` newtype `Copy` is more ergonomic and the workspace `missing_copy_implementations = "warn"` lint already nudges this).

```rust
//! `ScalarField`: 32-byte big-endian scalar with UNSIGNED mod-r reduction.

use num_bigint::BigUint;

/// BLS12-381 subgroup order r (big-endian 32 bytes).
///
/// Identical to the constant inside `chia_puzzle_types::derive_synthetic`
/// (`GROUP_ORDER_BYTES`) but reproduced here so this module has no
/// reverse dependency on derive_synthetic.
pub const GROUP_ORDER: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48,
    0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe,
    0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarField([u8; 32]);

impl ScalarField {
    /// Reduce 32 big-endian bytes to `[0, r)` using UNSIGNED interpretation.
    ///
    /// This is the correct reduction for CHIP-0057 protocol scalars
    /// (`input_hash`, `t_k`, `label_scalar`). It is NOT interchangeable with
    /// `chia_puzzle_types::derive_synthetic::mod_by_group_order`, which uses
    /// SIGNED interpretation for synthetic-key offsets and disagrees with this
    /// function whenever bit 255 of the input is set.
    #[must_use]
    pub fn from_bytes_unsigned(bytes: [u8; 32]) -> Self { /* BigUint::from_bytes_be % r */ }

    /// Wrap bytes without reduction. Use only when the caller can prove the
    /// value is already `< r` (e.g., `chia_bls::SecretKey::to_bytes()`).
    #[must_use]
    pub fn from_bytes_raw(bytes: [u8; 32]) -> Self { /* Self(bytes) */ }

    /// `(self + other) mod r`.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self { /* BigUint add */ }

    /// `(self * other) mod r`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self { /* BigUint mul */ }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }

    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] { self.0 }

    #[must_use]
    pub fn is_zero(&self) -> bool { self.0 == [0u8; 32] }
}
```

**Explicitly NOT included in Phase 1 (deferred to consumer phases):**
- `From<[u8; 32]> for ScalarField` — would allow silent unsigned-vs-signed mixing; type-system says no.
- `From<&SecretKey> for ScalarField` — Phase 4 may want this for `aggregate_sender_sks`; defer until the caller exists.
- `mul_pk`/`scalar_mul_g1` integration with `chia_bls::PublicKey::scalar_multiply` — Phase 3 (ECDH) needs this; Phase 1 only exposes raw bytes via `as_bytes()`.
- Subtraction, negation, inverse — not used by any Phase 1 caller. YAGNI.

### `silent_payments/tagged_hash.rs` — Public API Surface

```rust
//! BIP-340-style tagged-hash: `SHA256(SHA256(tag) || SHA256(tag) || data)`.
//!
//! Domain tags for CHIP-0057 are pinned strings — any typo silently produces
//! a different hash that disagrees with every other implementation.

use chia_sha2::Sha256;

/// Tag for the `input_hash` scalar (BIP-352 §"Inputs hash" analogue).
pub const CHIA_SP_INPUTS: &str = "Chia_SP/Inputs";

/// Tag for the `t_k` output-tweak scalar (BIP-352 §"Output tweak" analogue).
pub const CHIA_SP_SHARED_SECRET: &str = "Chia_SP/SharedSecret";

/// Tag for the `label_scalar` (BIP-352 §"Labels" analogue).
pub const CHIA_SP_LABEL: &str = "Chia_SP/Label";

/// Compute `SHA256(SHA256(tag) || SHA256(tag) || data)`.
#[must_use]
pub fn tagged_hash(tag: &str, data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag.as_bytes());
    let tag_hash: [u8; 32] = h.finalize();

    let mut h = Sha256::new();
    h.update(tag_hash);
    h.update(tag_hash);
    h.update(data);
    h.finalize()
}
```

**Critical note for the planner:** the success-criteria phrasing `chia_sha2::Sha256::digest("Chia_SP/Inputs")` is shorthand. **`chia-sha2` does not expose a `::digest()` static method.** The tag-pin test must use `new/update/finalize`:

```rust
let mut h = chia_sha2::Sha256::new();
h.update(b"Chia_SP/Inputs");
let actual: [u8; 32] = h.finalize();
assert_eq!(actual, hex_literal::hex!("...pinned 32 bytes..."));
```

The pinned-hash bytes for the three tags will be precomputed during plan/execute by running `Sha256::new(); update(tag.as_bytes()); finalize()` and pasting the `hex!(...)` result into the test. (Pre-computing here would risk transcription error; the planner should generate them at task time using `cargo test` capture or a tiny `println!` test.)

### `silent_payments/paths.rs` — Constants

```rust
//! BIP-32-style derivation paths for silent-payment scan / spend keys.
//!
//! `m/12381/8444/12/0` — scan secret key
//! `m/12381/8444/13/0` — spend secret key
//!
//! Indices `12` (scan) and `13` (spend) are the CHIP-0057 reserved values,
//! distinct from index `2` used by the standard Chia wallet.

pub const SCAN_PATH: &[u32] = &[12381, 8444, 12, 0];
pub const SPEND_PATH: &[u32] = &[12381, 8444, 13, 0];
```

Constants only. The derivation FUNCTION (`SilentPaymentKeys::from_mnemonic`) lives in `chia-sdk-utils::silent_payments::keys` per ARCHITECTURE.md and is Phase 2's responsibility. Phase 1 just pins the path values so Phase 2 can reference them.

### Anti-Patterns to Avoid

- **`pub use chia_puzzle_types::derive_synthetic::mod_by_group_order` inside the `silent_payments` module** — defeats the type boundary. The whole point of `ScalarField` is to make `mod_by_group_order` invisible to silent-payments code.
- **`use sha2::Sha256` instead of `chia_sha2::Sha256`** — works, but breaks SDK convention and triggers clippy/review smell. Both compile cleanly; pick the convention.
- **`From<[u8; 32]> for ScalarField`** — silently swallows the unsigned-vs-signed-bytes choice. Force callers to invoke `from_bytes_unsigned` (reduces) or `from_bytes_raw` (asserts in-range) explicitly.
- **Hex-string literals for tag constants instead of `&'static str`** — would defeat the test that pins `SHA256(tag_bytes)`. The constant must BE the string, not a hex-encoded representation of bytes.
- **A `pub fn` that takes a tag as `&str` parameter from outside the module** — any caller passing a string literal could typo. The three tag constants are the only legal first arguments; the compiler doesn't enforce this but the doc-comment + code-review pattern does. Optional belt-and-suspenders: a `TaggedHasher::inputs() / shared_secret() / label()` builder pattern — defer unless the planner finds it ergonomically valuable.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Generic SHA-256 hashing | A new `sha256(data)` helper, or `use sha2::Sha256` | `chia_sha2::Sha256` (already in workspace) | SDK-wide convention. Mixing `sha2` and `chia-sha2` triggers review/clippy smell. |
| 256-bit mod-r reduction | Manual byte-array long division, hand-rolled subtraction loop | `num_bigint::BigUint::from_bytes_be(&bytes) % BigUint::from_bytes_be(&GROUP_ORDER)` | Already pulled in workspace and already used by `chia-puzzle-types::derive_synthetic` for the signed variant. Pure-Rust, no FFI. Side-channel concerns (non-constant-time) are documented in PITFALLS.md but are acceptable for this protocol layer because the secret inputs are wallet-derived, not user-attacker-controlled. |
| BIP-340 tagged hash | A custom SHA-256-of-concatenation routine | The three-line `tagged_hash(tag, data)` function pattern from the reference impl | The construction is small enough to inline, but the *pattern* (SHA256(tag) precomputed and concatenated twice) is BIP-340-standard. Don't deviate. |
| Workspace feature cascade | Ad-hoc per-crate features | Mirror `chip-0035` (root Cargo.toml:73) and `chip-0037` (root Cargo.toml:74) literally | Precedent commit `bbc7f57f` (Add CHIP-0037 puzzle scaffolding) shows the exact 5-line diff pattern. Reuse it. |
| BLS subgroup-order constant | Computing it from a library function | `pub const GROUP_ORDER: [u8; 32] = [0x73, 0xed, ...];` literal (already in reference impl) | Reproduces `chia_puzzle_types::derive_synthetic::GROUP_ORDER_BYTES`. Pinning as a `pub const` enables fast compile-time `hex!(...)` substitution in tests. |
| BIP-32 derivation chaining | A `derive_path(&path[..])` helper | Chained `.derive_unhardened(12381).derive_unhardened(8444)...` calls (Phase 2 concern, not Phase 1) | `chia-bls 0.36.1`'s `derive_path_unhardened` helper is private (verified at `derive_keys.rs:8`). For Phase 2, mirror `master_to_wallet_unhardened_intermediate` (lines 24-26) — they chain `.derive_unhardened()` four times. Phase 1 only pins the path constants. |

**Key insight:** Phase 1's "don't hand-roll" list is short because the cryptographic surface is small and the SDK already provides the building blocks. The risk is reaching for the **wrong** SDK building block (signed `mod_by_group_order`, bare `sha2`). The `ScalarField` newtype is the structural enforcement mechanism.

---

## Common Pitfalls

The full pitfall catalog is in `.planning/research/PITFALLS.md`. The ones that apply to **Phase 1 specifically** are below — the rest land in Phases 2-4. Phase 1 must establish the structural protections; Phase 1 cannot validate them end-to-end (that's CRYPTO-03 / SEND / RECV in later phases).

### Pitfall 1: Reaching for `mod_by_group_order` instead of `ScalarField`

**What goes wrong:** A developer (or LLM) familiar with the SDK sees `chia_puzzle_types::derive_synthetic::mod_by_group_order` and uses it for `input_hash`. It uses **signed** interpretation (`BigInt::from_signed_bytes_be`). For SHA-256 digests with bit 255 set (~50% of inputs), the reduced scalar differs from the unsigned reduction. The receiver — using `ScalarField::from_bytes_unsigned` — derives a different one-time PK. Payment is undetectable forever, no error raised.

**Why it happens:**
1. `mod_by_group_order` is already in the SDK; it looks like the "obvious" helper.
2. The CHIP spec §"Synthetic Key Computation Note" mentions signed reduction *first* (for synthetic offsets), so developers reading top-to-bottom anchor on it.
3. TV1/TV3/TV4 from the CHIP all happen to produce input_hash digests with bit 255 clear, so a signed-vs-unsigned bug passes the canonical vectors.

**How to avoid:**
- `ScalarField` is the ONLY mod-r reduction path inside `#[cfg(feature = "chip-0057")]` code (success criterion 4).
- The newtype has no public `From<[u8; 32]>`. Callers MUST go through `from_bytes_unsigned` (which reduces) or `from_bytes_raw` (which docs say "you must prove `< r`").
- Module doc-comment in `silent_payments/mod.rs` calls out `mod_by_group_order` by name as a "do not use here."
- Phase 1 includes the adversarial unit test: `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes() == (r - 1).to_be_bytes()`. This fails on signed reduction (which would map `[0xff; 32]` to a negative value with different bytes after the mod-r adjustment).

**Warning signs:**
- A PR adds `use chia_puzzle_types::derive_synthetic::mod_by_group_order` to a `silent_payments/` file.
- A grep for `mod_by_group_order` in `crates/chia-sdk-types/src/silent_payments/` returns hits (success criterion 4 — must be zero).
- `from_signed_bytes_be` appears in silent-payments code.

### Pitfall 2: Tag-constant typo

**What goes wrong:** Phase 1 pins three tag strings. A typo (`Chia-SP/Inputs`, `chia_sp/inputs`, `Chia_SP/Input`) propagates to every later phase. Internal tests (sender + scanner both build the wrong tag) pass; cross-implementation interop with the Python reference / CHIP-0058 server fails.

**Why it happens:**
1. Magic strings; no compiler check on content.
2. Internal-only tests build the same wrong tag on both sides.
3. Refactor risk: someone renames `Chia_SP` to `ChiaSP` "for consistency."

**How to avoid (Phase 1 mechanism):**
- Three `pub const &'static str` constants, named after the tag suffix (`CHIA_SP_INPUTS`, `CHIA_SP_SHARED_SECRET`, `CHIA_SP_LABEL`) so a typo in the constant *name* doesn't masquerade as a typo in the *value*.
- The tag-pin unit test (success criterion 3) hard-codes `SHA256(b"Chia_SP/Inputs")` etc. as `hex!(...)` `[u8; 32]` constants. If anyone changes the tag value, the test fails before any protocol code runs.
- Cross-reference: TV1/TV3/TV4 (Phase 3 / CRYPTO-03) implicitly pin the tags because `input_hash`/`t_k`/`label_scalar` depend on them. But Phase 1 cannot run TVs, so the SHA256-pin test is its substitute.

**Warning signs:** Multiple string literals with subtle differences (`"Chia_SP/Inputs"` vs `"Chia_SP/inputs"`); tag passed as a function parameter from non-constant source; the pinned-SHA256 test removed in a refactor.

### Pitfall 3: chia-sha2 API mismatch (`::digest()` doesn't exist)

**What goes wrong:** Success criterion 3 reads `chia_sha2::Sha256::digest("Chia_SP/Inputs")`. **The `chia-sha2` crate does not export a `::digest()` static method** — only `new()` / `update()` / `finalize()`. A developer who literally writes `Sha256::digest(...)` gets a compile error and, if confused, switches to `sha2::Sha256::digest(...)` — which compiles but breaks the SDK convention (and would slip through if the dev forgets the substitution).

**Why it happens:**
- The success-criteria language reads like the `sha2::Digest` trait API. Confusion is natural.
- The bare `sha2` crate IS in workspace deps (line 178 of root Cargo.toml) so `use sha2::{Sha256, Digest}; Sha256::digest(...)` will compile in silent-payments code.

**How to avoid:**
- Document explicitly (in this RESEARCH.md and in the plan) that the tag-pin test MUST use `chia_sha2::Sha256` with `new/update/finalize`.
- Plan task should include a clippy check that nothing in `silent_payments/` imports from `sha2` directly.

**Warning signs:** `use sha2::` in `crates/chia-sdk-types/src/silent_payments/`; `Sha256::digest(...)` syntax.

### Pitfall 4: Feature-gate hole — code compiles `--all-features` but breaks no-features

**What goes wrong:** A developer adds `silent_payments` code that uses something only available under `chip-0057`, but forgets to gate a `use` import or a downstream `pub use`. Cumulative `--all-features` build is green; the per-crate `cargo build -p chia-sdk-types` (no features) breaks.

**Why it happens:**
- CI builds happen in series, not in parallel mental model: a developer running `cargo build --all-features` locally won't see the no-features failure.
- Re-exports in `lib.rs` are easy to leave un-gated.

**How to avoid:**
- The phase success criterion 1 mandates all five CI permutations build green:
  - `cargo build -p chia-sdk-types` (no features)
  - `cargo build -p chia-sdk-types --all-features`
  - `cargo build -p chia-sdk-types -F chip-0057`
  - `cargo build --workspace` (no features)
  - `cargo build --workspace --all-features`
- The plan should include a verify task that runs all five locally before merging.

**Warning signs:** A `pub use silent_payments::*;` in `lib.rs` not gated by `#[cfg(feature = "chip-0057")]`. Compilation errors of the form "cannot find type `ScalarField`" when running without features.

### Pitfall 5: `cargo machete` flagging the new feature scaffolding

**What goes wrong:** Success criterion 5: no new `[package.metadata.cargo-machete] ignored` entries. If Phase 1 adds an unused dep to support later phases, `cargo machete` rejects.

**Why it happens:** Anticipatory dep adds — "I'll add `bip39` now since Phase 2 needs it" — fail because Phase 1 code doesn't reference `bip39`.

**How to avoid:**
- Add only deps that Phase 1 code references. Verified: `num-bigint`, `chia-sha2`, `hex-literal` are all referenced (the first two in non-test code, the third in tests). `chia-bls` stays unconditional (used by other modules in `chia-sdk-types` already).
- Defer Phase 2 deps (`bip39` is already a workspace dep — but `chia-sdk-utils` will need to *add* it; that's Phase 2's Cargo.toml change, not Phase 1's).

**Warning signs:** `cargo machete` output showing `chia-sdk-types: bip39` or any new dep.

---

## Code Examples

Patterns verified against existing SDK code and the reference impl.

### Example 1: Feature-gated module exposure (mirror chip-0037 pattern from puzzles.rs:29-37)

```rust
// crates/chia-sdk-types/src/lib.rs (after current line 8)
#[cfg(feature = "chip-0057")]
pub mod silent_payments;
```

### Example 2: Tagged-hash construction with chia-sha2

```rust
// crates/chia-sdk-types/src/silent_payments/tagged_hash.rs
use chia_sha2::Sha256;

pub const CHIA_SP_INPUTS: &str = "Chia_SP/Inputs";
pub const CHIA_SP_SHARED_SECRET: &str = "Chia_SP/SharedSecret";
pub const CHIA_SP_LABEL: &str = "Chia_SP/Label";

#[must_use]
pub fn tagged_hash(tag: &str, data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag.as_bytes());
    let tag_hash: [u8; 32] = h.finalize();

    let mut h = Sha256::new();
    h.update(tag_hash);
    h.update(tag_hash);
    h.update(data);
    h.finalize()
}
```

### Example 3: ScalarField unsigned reduction (port of sp-common/scalar.rs lines 28-38)

```rust
use num_bigint::BigUint;

pub const GROUP_ORDER: [u8; 32] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48,
    0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe,
    0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarField([u8; 32]);

impl ScalarField {
    #[must_use]
    pub fn from_bytes_unsigned(bytes: [u8; 32]) -> Self {
        let n = BigUint::from_bytes_be(&bytes);
        let r = BigUint::from_bytes_be(&GROUP_ORDER);
        let reduced = n % &r;
        let be = reduced.to_bytes_be();
        let mut out = [0u8; 32];
        // Left-pad with zeros if BigUint stripped leading zeros.
        if !be.is_empty() {
            out[32 - be.len()..].copy_from_slice(&be);
        }
        Self(out)
    }
}
```

### Example 4: Adversarial unsigned-reduction test (success criterion 2)

```rust
#[test]
fn from_bytes_unsigned_max_input_reduces_to_r_minus_one() {
    let s = ScalarField::from_bytes_unsigned([0xff; 32]);

    // r - 1 in big-endian: same as GROUP_ORDER but last byte is 0 (since GROUP_ORDER ends in 0x01).
    let mut r_minus_one = GROUP_ORDER;
    r_minus_one[31] = 0x00;

    assert_eq!(s.to_bytes(), r_minus_one);
    assert_ne!(s.to_bytes(), [0xff; 32]); // must NOT pass-through unchanged
}
```

### Example 5: Tag-pin SHA-256 test (success criterion 3)

```rust
use chia_sha2::Sha256;
use hex_literal::hex;

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize()
}

#[test]
fn tag_inputs_hash_pinned() {
    // SHA256("Chia_SP/Inputs") — value generated once via Sha256(b"Chia_SP/Inputs")
    // and pinned here; any typo in the tag constant flips this assertion.
    const EXPECTED: [u8; 32] = hex!("<32-byte hex computed at task time>");
    assert_eq!(sha256(CHIA_SP_INPUTS.as_bytes()), EXPECTED);
}

#[test]
fn tag_shared_secret_hash_pinned() {
    const EXPECTED: [u8; 32] = hex!("<32-byte hex computed at task time>");
    assert_eq!(sha256(CHIA_SP_SHARED_SECRET.as_bytes()), EXPECTED);
}

#[test]
fn tag_label_hash_pinned() {
    const EXPECTED: [u8; 32] = hex!("<32-byte hex computed at task time>");
    assert_eq!(sha256(CHIA_SP_LABEL.as_bytes()), EXPECTED);
}
```

The planner / executor will replace `<32-byte hex computed at task time>` with the actual values by running a one-shot computation (e.g., a `cargo test -- --nocapture` with a `println!` of the hex-encoded result, then pinning). Doing it this way avoids transcription error from this RESEARCH.md.

### Example 6: Workspace feature cascade (mirror bbc7f57f diff)

```toml
# /home/kdc/chia-wallet-sdk/Cargo.toml (insert after line 74)
chip-0057 = [
    "chia-sdk-driver/chip-0057",
    "chia-sdk-types/chip-0057",
    "chia-sdk-utils/chip-0057",
]
```

```toml
# /home/kdc/chia-wallet-sdk/crates/chia-sdk-types/Cargo.toml (append to [features])
chip-0057 = []
```

```toml
# /home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/Cargo.toml (insert after chip-0037 line)
chip-0057 = ["chia-sdk-types/chip-0057"]
```

```toml
# /home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/Cargo.toml (NEW [features] block before [dependencies])
[features]
chip-0057 = []
```

### Example 7: CI workflow addition (WS-02)

Add to `.github/workflows/rust.yml` "Build individual crates" step (around line 50-70). Insert a `chip-0057` line for `chia-sdk-types` immediately after the existing `--all-features` line:

```yaml
# Existing line for context:
# cargo build --release -p chia-sdk-types --all-features
# Add:
cargo build --release -p chia-sdk-types -F chip-0057
```

The `--all-features` line already covers the cumulative case (it activates `chip-0035` + `chip-0037` + `chip-0057` + `action-layer`). The dedicated `-F chip-0057` line catches feature-isolation bugs that `--all-features` would hide (e.g., a `use` statement that should be `#[cfg(feature = "chip-0057")]` but compiles fine when chip-0035 is also on).

For `chia-sdk-driver` and `chia-sdk-utils`: NOT needed in Phase 1's CI delta because Phase 1 adds no code to those crates. The cascading feature build is exercised through the workspace `--all-features` line at the top of the build step. Phase 2 / 3 / 4 add per-crate `-F chip-0057` lines as code lands.

---

## Runtime State Inventory

**Not applicable.** Phase 1 introduces purely new files and Cargo.toml entries. No renames, no migrations, no runtime-state changes. No stored data, no live service config, no OS-registered state, no secrets, no build artifacts to invalidate. (The phase deliberately does not delete or move any existing file.)

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Use signed `mod_by_group_order` for any 256-bit-digest-to-scalar reduction | Use unsigned `ScalarField::from_bytes_unsigned` for protocol scalars; keep signed `mod_by_group_order` ONLY for the standard puzzle's synthetic offset (its original purpose) | CHIP-0057 draft (2026, this project) | The two are NOT interchangeable. Mixing them silently breaks payment detection. Type boundary in Phase 1 is the prevention mechanism. |
| Hand-rolled BIP-340 tagged-hash inline | Single `tagged_hash(tag, data)` helper with three pinned `pub const` tag strings | Spread across the silent-payments ecosystem; this project codifies the SDK form | Eliminates tag-typo class of bugs; centralizes the tag pinning. |
| `chia-sha2` vs `sha2` mixed | Always `chia-sha2` for new code in this codebase | SDK convention established since `chia-sha2 0.36.1` ship (pre-2026) | Style + audit trail; no behavioral difference. |

**Deprecated / outdated for this phase:**
- None. Phase 1 is greenfield in the silent-payments module.

---

## Open Questions

1. **Should `ScalarField` derive `Hash`?**
   - What we know: Phase 1 callers don't hash scalars. Phase 4's `aggregate_sender_sks` produces a `ScalarField`; the action system may or may not key on it.
   - What's unclear: whether `HashMap<ScalarField, _>` will ever be needed.
   - Recommendation: omit `Hash` in Phase 1. Trivial to add later. Avoids speculative API surface.

2. **Should `ScalarField::from_bytes_raw` be `pub(crate)` instead of `pub`?**
   - What we know: `from_bytes_raw` skips reduction; misuse → wrong scalars.
   - What's unclear: whether Phase 2's `SilentPaymentKeys` will reach for it (to wrap a `SecretKey::to_bytes()` known `< r`).
   - Recommendation: keep `pub` in Phase 1 with a strong doc-comment ("you must prove the input is in [0, r)"). If Phase 2 doesn't use it, demote to `pub(crate)` in Phase 2's plan.

3. **`tagged_hash(tag: &str, ...)` vs `tagged_hash(tag: &[u8], ...)`?**
   - What we know: tags are UTF-8 strings in the CHIP. The function only hashes `tag.as_bytes()`.
   - What's unclear: ergonomics. `&str` matches the prototype and CHIP convention; `&[u8]` is more general.
   - Recommendation: `&str`. Reference impl uses `&str`. CHIP spec uses string tags. No reason to widen.

4. **Where do `GROUP_ORDER` and `SCAN_PATH` / `SPEND_PATH` live — `silent_payments/scalar.rs` / `paths.rs` (proposed) or a shared `silent_payments/constants.rs`?**
   - What we know: ARCHITECTURE.md proposes `scalar.rs` and `paths.rs` separately. The cluster is tiny (3 constants).
   - What's unclear: aesthetic.
   - Recommendation: `scalar.rs` exports `GROUP_ORDER` (because it's a scalar concept). `paths.rs` exports `SCAN_PATH` / `SPEND_PATH`. Slightly more locality than a flat constants file. Easy to refactor.

5. **Pinned tag-hash bytes — should Phase 1 commit them, or compute at test time?**
   - What we know: Success criterion 3 wants them pinned (typo prevention).
   - What's unclear: whether to embed the 96 bytes (3 × 32) into the test source verbatim, or compute once in a helper and assert equality.
   - Recommendation: embed verbatim with `hex_literal::hex!(...)`. That's the standard pattern in `chia-puzzle-types::derive_synthetic::tests` and similar. The hex values get computed during execute-phase via a one-shot test that prints them.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | All builds and tests | ✓ | 1.90.0 (rust-toolchain.toml-pinned) | — |
| `rustc` | All builds | ✓ | 1.90.0 | — |
| `cargo-machete` | CI lint (success criterion 5) | ✓ via `cargo binstall` in CI | latest | locally: `cargo install cargo-machete` or skip; CI is authoritative |
| `rustfmt` | `cargo fmt --check` | ✓ (toolchain component) | 1.90.0 | — |
| `clippy` | `cargo clippy` (success criterion 1) | ✓ (toolchain component) | 1.90.0 | — |
| `num-bigint 0.4.6` | `ScalarField` reduction | ✓ workspace dep | 0.4.6 | — |
| `chia-sha2 0.36.1` | `tagged_hash` | ✓ workspace dep | 0.36.1 | — |
| `hex-literal 0.4.1` | Compile-time hex constants in tests | ✓ workspace dep | 0.4.1 | — |
| `chia-bls 0.36.1` | (not used by Phase 1 directly; stays declared) | ✓ workspace dep | 0.36.1 | — |
| Reference impl at `~/silent-payments/crates/sp-common/src/{scalar,tagged_hash}.rs` | Port source of truth | ✓ on local disk | (uncommitted local prototype) | If file moves, the CHIP-0057 spec at `~/silent-payments/chip-silent-payments.md` is the secondary authority. |
| GitHub Actions runner | CI verification of WS-02 | ✓ (configured) | — | — |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None.

---

## Validation Architecture

Workflow `nyquist_validation` is `true` (verified in `.planning/config.json`). This phase requires the full validation map.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust test harness) + `rstest 0.22.0` (dev-dep, optional for parametric tables) |
| Config file | None required — `chia-sdk-types/Cargo.toml [dev-dependencies]` already has `hex`, `rstest`, `anyhow`, `rand`, `rand_chacha` |
| Quick run command | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` (runs only the `silent_payments` module tests under the gate) |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` (matches CI, line 70 of rust.yml) |

**Gate commands** (success criterion 1) — all five must pass:

```bash
cargo build --release -p chia-sdk-types                              # no features
cargo build --release -p chia-sdk-types --all-features
cargo build --release -p chia-sdk-types -F chip-0057
cargo build --release --workspace                                    # no features
cargo build --release --workspace --all-features
cargo clippy --workspace --all-features --all-targets                # clean
cargo fmt --all -- --files-with-diff --check                         # clean
cargo machete                                                        # no new ignored entries
```

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| CRYPTO-01 | `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes()` equals `r - 1` big-endian, NOT `[0xff; 32]` | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_unsigned_max_input_reduces_to_r_minus_one -- --exact` | ❌ Wave 0 — `crates/chia-sdk-types/src/silent_payments/scalar.rs` |
| CRYPTO-01 | Identity-reduction: small input passes through unchanged | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_unsigned_identity -- --exact` | ❌ Wave 0 |
| CRYPTO-01 | `mul`/`add` produce correct mod-r values for known small operands | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::mul_mod_r -- --exact` and `silent_payments::scalar::tests::add_wraps_at_r` | ❌ Wave 0 |
| CRYPTO-01 | `from_bytes_raw` does NOT reduce (regression — would defeat the type boundary if it did) | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_raw_does_not_reduce -- --exact` | ❌ Wave 0 |
| CRYPTO-02 | `tagged_hash` produces the BIP-340 construction (verify with a known cross-check vector: `tagged_hash("BIP0340/challenge", b"")` equals `c216d352f5818b7b4beacd4ae0a26fe888080823d2a598856661bcd54f1b3713`) | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tagged_hash_matches_bip340_challenge_vector -- --exact` | ❌ Wave 0 — `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` |
| CRYPTO-02 | `SHA256("Chia_SP/Inputs")` equals pinned bytes (typo guard) | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_inputs_hash_pinned -- --exact` | ❌ Wave 0 |
| CRYPTO-02 | `SHA256("Chia_SP/SharedSecret")` equals pinned bytes | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_shared_secret_hash_pinned -- --exact` | ❌ Wave 0 |
| CRYPTO-02 | `SHA256("Chia_SP/Label")` equals pinned bytes | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_label_hash_pinned -- --exact` | ❌ Wave 0 |
| CRYPTO-02 | `tagged_hash(tag, data)` is deterministic and tag-sensitive (different tag → different hash) | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::different_tags_different_outputs -- --exact` | ❌ Wave 0 |
| WS-01 | Root `chip-0057` feature exists and cascades — every gated build succeeds | structural / build | `cargo build --release --workspace --all-features` (verifies cascade activates) and `cargo build --release -p chia-sdk-types -F chip-0057` (verifies isolated build) | ❌ Wave 0 — Cargo.toml edits |
| WS-01 | `chip-0057` feature present on `chia-sdk-driver` and `chia-sdk-utils` | structural | `cargo build --release -p chia-sdk-driver -F chip-0057` and `cargo build --release -p chia-sdk-utils -F chip-0057` (both currently no-op; verifies the feature exists in each crate's Cargo.toml) | ❌ Wave 0 |
| WS-02 | CI workflow has per-crate `chip-0057` build line | structural | `grep -E 'chia-sdk-types.*chip-0057\|chip-0057.*chia-sdk-types' .github/workflows/rust.yml` returns ≥ 1 line | ❌ Wave 0 — `.github/workflows/rust.yml` edit |
| WS-03 | All chip-0057-gated code passes clippy under workspace lint policy | structural / lint | `cargo clippy --workspace --all-features --all-targets -- -D warnings` exits 0 | ❌ Wave 0 |
| WS-03 | `cargo machete` clean (success criterion 5) | structural / lint | `cargo machete` exits 0; `git diff` shows no new `[package.metadata.cargo-machete] ignored` entries in any Cargo.toml | ❌ Wave 0 |
| WS-03 (success criterion 4) | Grep-based ban on `mod_by_group_order` in silent_payments module | structural / grep-lint | `! grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` (negated: returns nonzero exit iff any match) | ❌ Wave 0 — module is being created |
| WS-03 (defense-in-depth) | Grep-based ban on bare `sha2::` imports in silent_payments module | structural / grep-lint | `! grep -rE '^use sha2::' crates/chia-sdk-types/src/silent_payments/` | ❌ Wave 0 |
| WS-03 (formatting) | `cargo fmt --check` clean | structural | `cargo fmt --all -- --files-with-diff --check` exits 0 | ❌ Wave 0 |

**Test type legend:**
- **unit** — Rust `#[test]` function, fast (<1s), in `mod tests { ... }` inside the source file.
- **structural** — passes/fails based on file presence/content/build result, not Rust test runtime.
- **lint** — uses a tool (clippy, cargo-machete) to enforce a coding constraint.
- **grep-lint** — a CI-runnable `grep` invocation, executed as a bash step or as a test that shells out. Per the phase success criteria, success criterion 4 is explicitly grep-based; treat it as a CI step, not a Rust test.

### Sampling Rate

- **Per task commit:** `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` (only the new module's tests; runs in < 5 seconds).
- **Per wave merge:** the five gate `cargo build` commands above + `cargo test --release -p chia-sdk-types --all-features` + `cargo clippy --workspace --all-features --all-targets`.
- **Phase gate:** full suite green before `/gsd:verify-work`. Specifically: `cargo build --release --all-features` + `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` + `cargo clippy --workspace --all-features --all-targets` + `cargo fmt --all -- --files-with-diff --check` + `cargo machete` + the success-criterion-4 grep check.

### Wave 0 Gaps

All test files for Phase 1 are net-new. Wave 0 must create:

- [ ] `crates/chia-sdk-types/src/silent_payments/mod.rs` — barrel + module doc-comments
- [ ] `crates/chia-sdk-types/src/silent_payments/scalar.rs` — `ScalarField` + `GROUP_ORDER` + `#[cfg(test)] mod tests`
- [ ] `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` — `tagged_hash` + tag consts + `#[cfg(test)] mod tests`
- [ ] `crates/chia-sdk-types/src/silent_payments/paths.rs` — `SCAN_PATH` / `SPEND_PATH` (no tests needed — values match the CHIP literally and will be validated end-to-end at Phase 2 via known mnemonic → key derivation tests)
- [ ] Edit `crates/chia-sdk-types/src/lib.rs` — add `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- [ ] Edit `Cargo.toml` (root) — append `chip-0057 = [...]` to `[features]`
- [ ] Edit `crates/chia-sdk-types/Cargo.toml` — append `chip-0057 = []` to `[features]`
- [ ] Edit `crates/chia-sdk-driver/Cargo.toml` — append `chip-0057 = ["chia-sdk-types/chip-0057"]` to `[features]`
- [ ] Edit `crates/chia-sdk-utils/Cargo.toml` — add `[features]` block with `chip-0057 = []`
- [ ] Edit `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-types -F chip-0057` line in the "Build individual crates" step

**Framework install:** Not needed. `cargo test` is built into the Rust toolchain. `hex_literal`, `chia-sha2`, `num-bigint` are all already in `chia-sdk-types/[dependencies]` or `[dev-dependencies]`.

**Pinned tag-hash bytes computation:** Wave 0 must include a task to compute the three pinned `[u8; 32]` values (one per tag). Suggested approach: write the tag-pin tests with placeholder `hex!("00".repeat(32))`, run `cargo test ... -- --nocapture` with the actual `Sha256(tag)` printed, then commit the real values. Alternatively: a tiny throwaway Rust scratch binary; or a Python one-liner using the equivalent `hashlib.sha256(b"Chia_SP/Inputs").hexdigest()` (the values are SHA-256 of an ASCII string — fully reproducible cross-language).

---

## Sources

### Primary (HIGH confidence)

- `~/silent-payments/crates/sp-common/src/scalar.rs` — Reference Rust impl of `ScalarField` with tests, validated against CHIP test vectors.
- `~/silent-payments/crates/sp-common/src/tagged_hash.rs` — Reference Rust impl of `tagged_hash`, validated against the BIP-340 test vector cross-check.
- `~/silent-payments/chip-silent-payments.md` — CHIP-0057 draft, defines tag strings, GROUP_ORDER, derivation paths, signed-vs-unsigned distinction (§"Synthetic Key Computation Note").
- `/home/kdc/chia-wallet-sdk/Cargo.toml` — workspace dependencies, lint policy, existing `chip-0035` / `chip-0037` / `action-layer` feature patterns.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/Cargo.toml` — confirms `num-bigint`, `chia-sha2`, `hex-literal`, `chia-bls`, `chia-puzzle-types` all already declared; existing feature block to extend.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/src/lib.rs` — confirms `pub mod puzzles;` is the existing pattern for nested feature-gated modules.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/src/puzzles.rs` — verified at lines 23-43 the `#[cfg(feature = "chip-XXXX")] mod x; #[cfg(feature = "chip-XXXX")] pub use x::*;` pattern.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/Cargo.toml` — confirms cascading feature pattern (lines 20-24).
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/Cargo.toml` — confirms NO existing `[features]` block (Phase 1 must create one).
- `/home/kdc/chia-wallet-sdk/.github/workflows/rust.yml` — confirms CI structure: per-crate `cargo build` lines, `cargo clippy --workspace --all-features --all-targets`, `cargo machete`.
- Commit `bbc7f57f` (Add CHIP-0037 puzzle scaffolding) — precedent for cascading-feature commit shape: 10 files, ~732 LOC across Cargo.toml diffs + new modules. Phase 1 is structurally smaller (~3 new files + 4 Cargo.toml edits + 1 lib.rs edit + 1 CI edit).
- `/home/kdc/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chia-puzzle-types-0.36.1/src/derive_synthetic.rs` — verified at line 39 the existing `pub fn mod_by_group_order(bytes: [u8; 32]) -> [u8; 32]` (signed, using `BigInt::from_signed_bytes_be`); this is the helper Phase 1 must NOT reuse.
- `/home/kdc/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chia-sha2-0.36.1/src/lib.rs` — verified the `Sha256 { new(), update(buf), finalize() -> [u8; 32] }` API; **confirmed there is no `::digest()` static method** (success-criteria phrasing is shorthand, not literal).
- `/home/kdc/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chia-bls-0.36.1/src/derive_keys.rs` — confirms `master_to_wallet_unhardened_intermediate` chains `derive_unhardened` (relevant for Phase 2 derivation, not Phase 1); `derive_path_unhardened` is private.
- `.planning/PROJECT.md` — confirms "no new workspace deps" constraint and ScalarField key decision.
- `.planning/REQUIREMENTS.md` — CRYPTO-01, CRYPTO-02, WS-01, WS-02, WS-03 definitions.
- `.planning/ROADMAP.md` — Phase 1 success criteria + downstream phase dependencies.
- `.planning/STATE.md` — Phase 2 pre-flight Q8 deferral confirmed (chia-sdk-types -> chia-sdk-utils dep edge is NOT Phase 1's responsibility).
- `.planning/research/STACK.md` — confirms zero new workspace deps; confirms chia-bls 0.36.1 has every API the project needs.
- `.planning/research/ARCHITECTURE.md` — confirms `silent_payments/{scalar,tagged_hash,paths}.rs` module layout in `chia-sdk-types`.
- `.planning/research/PITFALLS.md` — confirms Pitfall 1 (signed-vs-unsigned), Pitfall 4 (tag typos), Pitfall 9 (chia-bls API) — all relevant to Phase 1 structural protections.

### Secondary (MEDIUM confidence)

- None for Phase 1 — the entire scope is verifiable against on-disk source.

### Tertiary (LOW confidence)

- None for Phase 1.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every dep verified against `/home/kdc/.cargo/registry/src/...` source on disk.
- Architecture: HIGH — module layout mirrors existing `chip-0035` / `chip-0037` precedent verbatim; ARCHITECTURE.md from the project's own research authored 2026-05-15.
- Pitfalls: HIGH — reference prototype already shows the failure modes; PITFALLS.md catalogs them in detail; Phase 1 prevention mechanisms (newtype + `pub const` tags + tag-pin tests + grep ban) are all structurally enforced rather than convention-based.
- Validation architecture: HIGH — all gate commands run today, test framework is built-in `cargo test`, no new tooling needed.

**Research date:** 2026-05-15
**Valid until:** 30 days (2026-06-14) — stable Rust workspace, no fast-moving deps. Re-validate if `chia-bls` / `chia-sha2` / `chia-puzzle-types` ship a 0.37+ release.

---

## RESEARCH COMPLETE
