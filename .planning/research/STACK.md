# Stack Research — CHIP-0057 Silent Payments

**Domain:** Cryptographic protocol added to existing Rust SDK workspace
**Researched:** 2026-05-15
**Confidence:** HIGH (all primary deps verified against on-disk crate source at `~/.cargo/registry/src/...`)

## TL;DR

**Zero new workspace dependencies.** Every primitive CHIP-0057 needs is already pinned at the workspace root:
`chia-bls 0.36.1`, `chia-puzzle-types 0.36.1`, `chia-sha2 0.36.1`, `num-bigint 0.4.6`, `bip39 2.2.0`, `bech32 0.9.1`, `hex 0.4.3`, `hex-literal 0.4.1`. The prototype's choice of `chia-bls = "0.41"` and bare `sha2` is **not required** — chia-bls 0.36.1 exposes the same surface area we need (`scalar_multiply` on `PublicKey`, `derive_unhardened`/`derive_hardened` via `DerivableKey`, `Add`/`AddAssign`/`SubAssign` on PublicKey, `is_inf()` for identity-element checks), and the SDK already standardises on `chia-sha2` (not `sha2`) for hashing.

The only crate-level changes are: (a) gate code under `#[cfg(feature = "chip-0057")]`, (b) add the cascading feature in three Cargo.tomls (root, `chia-sdk-types`, `chia-sdk-driver`, `chia-sdk-utils`), and (c) add per-crate CI build lines.

## Recommended Stack

### Core Technologies (Already in Workspace — Reuse As-Is)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `chia-bls` | 0.36.1 | G1 group ops (PublicKey scalar_multiply, add, is_inf), BLS12-381 SecretKey arithmetic and unhardened derivation, BIP-39 seed→master_sk via `SecretKey::from_seed` | **Verified sufficient** for CHIP-0057. `public_key.rs:141` exposes `pub fn scalar_multiply(&mut self, int_bytes: &[u8])`; `public_key.rs:131` exposes `pub fn is_inf(&self)` (catches zero-sum aggregation, spec §"Identity Element"). `secret_key.rs:224` and `public_key.rs:282` both impl `DerivableKey::derive_unhardened`. `Add`/`AddAssign`/`SubAssign` on PublicKey (`public_key.rs:232-262`) and `Add`/`AddAssign` on SecretKey (`secret_key.rs:184-206`) cover scan/spend aggregation and `B_spend + label_pk`. **Do not bump.** |
| `chia-puzzle-types` | 0.36.1 | One-time puzzle-hash computation via `StandardArgs::curry_tree_hash(pk.derive_synthetic())` | Drops the prototype's standalone `puzzle_hash_for_pk` per PROJECT.md key decision. `puzzles/standard.rs:19` and `derive_synthetic.rs:11-37` provide exactly this combo; `DeriveSynthetic::derive_synthetic` is the canonical path. Also provides the **signed** `mod_by_group_order` (`derive_synthetic.rs:39`) — useful as a reference point when writing the new **unsigned** `ScalarField` to make the asymmetry visible. |
| `chia-sha2` | 0.36.1 | All SHA-256: `tagged_hash`, `shared_secret = SHA256(serialize(S))`, plus the inner `SHA256(tag)` for tag pre-hash | SDK convention — every existing crate (`chia-sdk-types/src/constants.rs:5`, `merkle_tree.rs:5`, `condition/announcements.rs:2`, all driver primitives) uses `chia_sha2::Sha256`, never bare `sha2`. API surface (`new()` / `update(buf)` / `finalize() -> [u8; 32]`) matches what tagged_hash needs and returns `[u8; 32]` directly (no `.into()` conversion dance). |
| `num-bigint` | 0.4.6 | `BigUint::from_bytes_be(&hash) % BigUint::from_bytes_be(&GROUP_ORDER)` inside `ScalarField::from_bytes_unsigned`; same for scalar add/mul mod r | Already in workspace and already used by `chia-puzzle-types::derive_synthetic` (as `BigInt::from_signed_bytes_be`). The unsigned variant uses `BigUint::from_bytes_be` — explicitly *not* `from_signed_bytes_be`. Tag the call site with a comment referencing `mod_by_group_order` to make the signed-vs-unsigned distinction obvious to future readers. Pure-Rust, no FFI, no `unsafe` — satisfies `unsafe_code = "deny"`. |
| `bip39` | 2.2.0 | `Mnemonic::parse` + `to_seed("")` → 64-byte seed → `SecretKey::from_seed` | Same path already used by `chia-sdk-bindings/src/mnemonic.rs` and `chia-sdk-test/src/key_pairs.rs`. Reference prototype uses the identical flow (`keys.rs:15-21`). |
| `bech32` | 0.9.1 | Encode/decode 96-byte `scan_pk \|\| spend_pk` payload with HRP `spxch` / `tspxch`, variant Bech32m | `chia-sdk-utils::Bech32` (a thin wrapper over `bech32::decode`/`encode` forcing `Variant::Bech32m`) already supports arbitrary-length payloads — the `Address` type happens to constrain to 32 bytes, but `Bech32 { data: Bytes, prefix: String }` is generic. Build a sibling `SilentPaymentAddress` in `chia-sdk-utils` that calls `Bech32::new(payload_96_bytes, "spxch".into()).encode()`. |
| `hex` / `hex-literal` | 0.4.3 / 0.4.1 | Test-vector ingestion (hex strings → `[u8; 32]` / `[u8; 48]`) | `hex_literal::hex!("...")` is the compile-time pattern used by every existing crate (`chia-puzzle-types::derive_synthetic`, `chia-sdk-types::merkle_tree`, etc.) for fixed-length test constants. Use `hex::decode` only for runtime-loaded vectors (which we don't need here). |
| `thiserror` | 2.0.17 | Error enum for address decode / aggregation / detection failures | SDK convention — every public error type uses it (`Bech32Error` in `chia-sdk-utils/src/bech32.rs:5-18` is the template). |

### Supporting Libraries (Already in Workspace, Dev-Only)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rstest` | 0.22.0 | Parametric tests for the four CHIP test vectors | Already used in `chia-sdk-types/src/merkle_tree.rs:152` for table-driven tests — one `#[rstest] #[case(...)]` block per test vector keeps CRYPTO-03 readable. Alternative: a single `#[test]` per vector — fine if rstest feels heavy. |
| `anyhow` | (workspace) | Test fixtures only | Dev-dep convention in every crate. Production code uses `thiserror` enums. |
| `hex` | 0.4.3 | Test assertion strings (hex-encode-and-compare for intermediate values) | The prototype's `assert_eq!(hex::encode(...), "...")` pattern is fine for diagnostic-friendly failures showing the bad value. |

### Development Tools (Workspace-Wide, No Changes Needed)

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo clippy --workspace --all-features --all-targets` | Enforce workspace lints | Run before every commit; CHIP-0057 code must pass `deny clippy::all` + `warn pedantic`. |
| `cargo machete` | Reject unused deps | If a new dep is added to a crate's `[dependencies]` it must be referenced in non-feature-gated code OR sit behind `optional = true` and be activated by the `chip-0057` feature. |
| `cargo fmt --check` | Format check | Pre-existing — no change. |
| `hex_literal::hex!` macro | Compile-time hex constants | Use for `GROUP_ORDER`, test-vector keys, expected outputs. Catches typos at build time, not at test time. |

## Installation

No installs. Every dep is already at the workspace root. **Per-crate Cargo.toml changes only:**

```toml
# /home/kdc/chia-wallet-sdk/Cargo.toml (root)
[features]
chip-0057 = [
    "chia-sdk-driver/chip-0057",
    "chia-sdk-types/chip-0057",
    "chia-sdk-utils/chip-0057",
]
```

```toml
# crates/chia-sdk-types/Cargo.toml
[features]
chip-0057 = []  # Pure-Rust types; no new deps required.
# (chia-bls, chia-puzzle-types, chia-sha2, hex-literal already in [dependencies])
```

```toml
# crates/chia-sdk-driver/Cargo.toml
[features]
chip-0057 = ["chia-sdk-types/chip-0057"]
# (chia-bls, chia-puzzle-types, chia-sha2, num-bigint, bip39, hex-literal already in [dependencies])
```

```toml
# crates/chia-sdk-utils/Cargo.toml
[features]
chip-0057 = []
# Need to ADD to [dependencies] (only here — bech32 lives in utils today, but no chia-bls):
chia-bls = { workspace = true }       # For SilentPaymentAddress encode/decode payload parsing
chia-puzzle-types = { workspace = true } # If SilentPaymentAddress lives here AND validates pks
# (bech32, hex, thiserror, chia-protocol already in [dependencies])
```

Caveat: `chia-sdk-utils` does not currently depend on `chia-bls`/`chia-puzzle-types`. Three options, in order of preference:
1. **Put `SilentPaymentAddress` in `chia-sdk-utils`** and add the two deps (cleanest for consumers — addresses live with `Address`/`Bech32`). Risk: pulls BLS into utils, which has been kept thin.
2. **Put `SilentPaymentAddress` in `chia-sdk-types`** (already has both deps). Tradeoff: it's an address, not a "type"; weakens the boundary.
3. **Put it in `chia-sdk-driver`** (already has both deps). Tradeoff: addresses aren't driver code — they're encoding.

Recommend **(1)** with the `chia-bls`/`chia-puzzle-types` deps gated by `optional = true` and `dep:chia-bls` in the feature, so utils stays slim when `chip-0057` is off:

```toml
# crates/chia-sdk-utils/Cargo.toml (option 1)
[features]
chip-0057 = ["dep:chia-bls", "dep:chia-puzzle-types"]

[dependencies]
chia-bls = { workspace = true, optional = true }
chia-puzzle-types = { workspace = true, optional = true }
# ... existing deps
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `chia-bls 0.36.1` (workspace) | `chia-bls 0.41` (prototype's pick) | **Never** — for this work. Bumping `chia-bls` would force a coordinated bump of `chia-protocol`, `chia-puzzle-types`, `chia-traits`, `chia-secp`, `clvm-traits`, `clvm-utils`, `chia-sha2`, `chia-consensus` (all pinned at 0.36.1, the chia-rs release train). PROJECT.md explicitly says no. 0.36.1 has every method 0.41 has for our use case (verified at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chia-bls-0.36.1/src/public_key.rs` and `secret_key.rs`). |
| `num-bigint::BigUint % BigUint` for ScalarField | `bls12_381 0.8` / `blst` / `ark-bls12-381` (proper Fr type with constant-time arithmetic) | If the codebase ever moves to a constant-time Fr (recommended long-term — see PITFALLS.md "Side channels"). Today: blocked. `bls12_381` would need `unsafe_code = "allow"` opt-out (`blst` uses FFI; `ark-bls12-381` works on stable but adds a *third* BLS library to the workspace alongside chia-bls and chia-secp — `cargo machete` won't reject but the auditor will). The prototype's num-bigint approach is what `chia-puzzle-types::derive_synthetic` itself uses (signed variant) — keeping parity with that idiom is more important than constant-time-ness at the synthetic-key + ECDH-scalar level (the secrets here are wallet-derived, never user-controlled-input-driven). |
| `chia-sha2 0.36.1` | `sha2 0.10.9` (prototype's pick) | If you specifically need the trait-based `Digest` API (`Sha256::digest(data)`). The SDK convention is `chia-sha2`; mixing them in one crate is a clippy-pedantic smell. Both pass `unsafe_code = "deny"`. |
| `bech32 0.9.1` (workspace) | `bech32 0.11.x` (latest) | If we needed bech32m-only-by-default API ergonomics. 0.9.1 requires explicit `Variant::Bech32m` checks (which `chia-sdk-utils::Bech32` already wraps). Bumping is a workspace-wide change for marginal benefit. |
| Roll our own crypto in-tree | External crate (e.g. `bip352`, `silentpayments`, `rust-bip352`) | **Never.** All three existing silent-payments Rust crates target **BIP-352 (secp256k1)**, not BLS12-381. Different curve, different serialisation (33-byte compressed vs 48-byte G1), different tagged-hash tag prefix (`BIP0352/` vs `Chia_SP/`), different group order. They share zero code. Verified via crates.io search 2026-05-15: `bip352`, `silentpayments` (cygnet3/rust-silentpayments), `rust-bip352` (jirijakes) — all secp256k1. |
| `rstest` for test vectors | `#[test]` per vector | If parametric grouping makes the vector code harder to map back to the CHIP spec section. The CHIP has only 4 vectors (TV1–TV4), each with 3–6 intermediate assertions; per-vector `#[test]` may actually be more readable. Pick at write time based on what reads cleanly. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `chia_puzzle_types::derive_synthetic::mod_by_group_order` for protocol scalars | **CRITICAL.** This function uses `BigInt::from_signed_bytes_be` — it interprets a high bit set as negative. CHIP-0057 mandates **unsigned** reduction for `input_hash`, `output_tweak`, `label_scalar` (spec §"Synthetic Key Computation Note" calls out the asymmetry explicitly: "Implementations MUST use signed interpretation [for synthetic offsets]" — implying the opposite elsewhere). The two functions agree exactly half the time and disagree on the other half (whenever the SHA-256 output has bit 255 set). Silent disagreement = unspendable payments. | A new `ScalarField` newtype in `chia-sdk-types` (CRYPTO-01) calling `BigUint::from_bytes_be` (unsigned) explicitly. Make the type non-`From`-convertible to/from raw `[u8; 32]` outside the constructor so mixing is a type error. |
| `sha2::Sha256` directly in new code | Not the SDK convention. Mixing `chia_sha2::Sha256` and `sha2::Sha256` in the same crate triggers a smell during review and `cargo machete` may eventually flag the duplicate if `sha2` becomes a transitive-only dep. | `chia_sha2::Sha256` (already in the relevant Cargo.tomls). API: `let mut h = Sha256::new(); h.update(b"..."); let out: [u8; 32] = h.finalize();`. |
| The prototype's `puzzle_hash_for_pk` | Duplicates `StandardArgs::curry_tree_hash(pk.derive_synthetic())` already in `chia-puzzle-types`. Risks drift if the standard puzzle hash ever changes upstream. | `chia_puzzle_types::standard::StandardArgs::curry_tree_hash(synthetic_pk)`. The PROJECT.md "Key Decisions" table calls this out. |
| A fresh `BigInt`/`BigUint` scalar everywhere | Allocation per multiply / add. Hot path on the scanner side (one `(input_hash * a_sum) * B_scan` and SHA-256 per *spend group* — bounded by *K*<sub>max</sub> = 2,400 outputs/group times many groups per block). | One `ScalarField` newtype wrapping `[u8; 32]` with `mul`/`add` methods that hide the BigUint allocation. Matches the prototype's `scalar.rs` shape. Future optimization: swap the inner impl to a constant-time backend without touching call sites. |
| `bech32 0.11.x` API patterns | The crate at 0.11 has a different module layout (`bech32::primitives`, removed `Variant`, encoder/decoder splits). Workspace is pinned at 0.9.1. | The 0.9.1 API: `bech32::decode(addr)` → `(hrp, Vec<u5>, Variant)`; `bech32::convert_bits(&data, 5, 8, false)`; `bech32::encode(&hrp, data_u5s, Variant::Bech32m)`. Exactly what `chia-sdk-utils::Bech32` already wraps. |
| Hand-written hex string parsing in tests | Runtime parse failures hide as test panics with poor diagnostics. | `hex_literal::hex!("...")` for compile-time `[u8; N]` constants. Test bodies use the constant directly. |
| `unsafe` blocks anywhere in CHIP-0057 code | `unsafe_code = "deny"` at workspace root. Crypto FFI is already encapsulated in `chia-bls` upstream. | Pure-Rust everything. `num-bigint`, `chia-bls` (FFI hidden), `chia-sha2`, `bech32` are all safe Rust at the call site. |

## Stack Patterns by Variant

**If implementing in `chia-sdk-types` (`ScalarField`, `tagged_hash`, `compute_input_hash`, key derivation):**
- Use `chia-bls` for `SecretKey`/`PublicKey`, `chia-sha2::Sha256` for hashing, `num-bigint::BigUint` (private to the `ScalarField` module) for reduction.
- Gate everything behind `#[cfg(feature = "chip-0057")]`.
- Add a `silent_payments` (or `chip_0057`) module to `chia-sdk-types/src/`; re-export from `lib.rs` under the same feature gate.
- Tests use `rstest` (already a dev-dep) or plain `#[test]`; constants via `hex_literal::hex!`.

**If implementing in `chia-sdk-driver` (`derive_one_time_puzzle_hash`, `SilentPaymentSend` action, `scan_from_tweaks`):**
- Add `chip-0057 = ["chia-sdk-types/chip-0057"]` to driver Cargo.toml — no new optional deps needed (everything we need is already in unconditional `[dependencies]`).
- Use `chia_puzzle_types::standard::StandardArgs::curry_tree_hash` for the one-time puzzle hash.
- Use `chia_puzzle_types::DeriveSynthetic::derive_synthetic` if/when a SecretKey needs synthesis (the sender's `aggregated_sender_sk` aggregates *already-synthetic* SKs per the CONCERNS.md "Synthetic vs raw keys" note — likely no synthesis happens inside silent-payments code itself).

**If implementing in `chia-sdk-utils` (`SilentPaymentAddress` encode/decode):**
- Pick option (1) above: add `chia-bls` and `chia-puzzle-types` as `optional = true` workspace deps activated by `chip-0057`.
- Reuse `chia_sdk_utils::Bech32::new(payload_96_bytes, "spxch".into())` for the underlying bech32m encoding.
- Validate inside `decode`: payload length == 96, both 48-byte halves are valid `PublicKey`s (use `PublicKey::from_bytes(&bytes)`), neither is the identity element (`!pk.is_inf()`). The `from_bytes` call performs G1 subgroup-membership check (verified at `chia-bls-0.36.1/src/public_key.rs:94`).

**If implementing in `chia-sdk-bindings`:**
- No new top-level deps. PublicKey/SecretKey are already exposed as bindable "class with remote=true" via `bindings/bls.json:65-133`.
- Create `bindings/silent_payments.json` mirroring `bindings/bls.json` / `bindings/address.json` shape. Address-style types use `Bytes32`/`Bytes48`/`Vec<...>` mappings.
- Bindings are unconditional (build with all CHIP features ON) per the existing pattern — `bindy-macro` resolves `PublicKey` from the workspace `chia-bls`, same flow used for the existing BLS facade.

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `chia-bls 0.36.1` | `chia-puzzle-types 0.36.1`, `chia-protocol 0.36.1`, `chia-traits 0.36.1`, `chia-sha2 0.36.1` | All part of the chia-rs 0.36.1 release; **mismatch causes type-incompatibility errors** (`PublicKey` from two different chia-bls majors is two different types). Verified by Cargo.lock: every chia-* crate in the workspace resolves to 0.36.1. |
| `chia-bls 0.36.1` | `num-bigint 0.4.6` | Compatible. `chia-bls` does not depend on num-bigint at all — they live in disjoint trees and interop via raw `[u8; 32]` (chia-bls's `scalar_multiply` takes `&[u8]` and num-bigint's `BigUint::to_bytes_be()` produces it). |
| `bech32 0.9.1` | (everything else) | No transitive coupling. The 0.9 → 0.11 API change is significant; do not let a new crate pull in 0.11. `cargo tree -i bech32` should show a single 0.9.1 node. |
| `bip39 2.2.0` | `chia-bls 0.36.1` | Compatible. `bip39::Mnemonic::to_seed("")` returns `[u8; 64]`; `chia_bls::SecretKey::from_seed(&seed)` takes `&[u8]`. Same pairing used in `chia-sdk-bindings/src/mnemonic.rs`. |
| `hex_literal 0.4.1` | All Rust 1.90.0 code in workspace | No MSRV concerns. Macro is purely syntactic. |
| `rstest 0.22.0` | All test code in workspace | Already in workspace dev-deps; used in `chia-sdk-types/src/merkle_tree.rs`. |

## Blocker Check — Verified None

| Item | Required Behavior | Status in chia-bls 0.36.1 | Verdict |
|------|-------------------|---------------------------|---------|
| In-place scalar mul on PublicKey (G1) | `pk.scalar_multiply(int_bytes: &[u8])` | `public_key.rs:141` — exact signature match | OK |
| Point addition `B_spend + label_pk` (G1) | `PublicKey + &PublicKey` | `public_key.rs:250,262` — `impl Add<&PublicKey> for &PublicKey` and `for PublicKey` | OK |
| Subtraction (only if needed for change detection) | `PublicKey -= &PublicKey` | `public_key.rs:240` — `impl SubAssign<&PublicKey>` | OK |
| Identity-element check (spec §Edge Cases) | `pk.is_inf() -> bool` | `public_key.rs:131` — exact match | OK |
| SecretKey + SecretKey mod r | `SecretKey + &SecretKey` | `secret_key.rs:184,196,206` — `impl Add` and `impl AddAssign` | OK |
| SK → PK | `sk.public_key() -> PublicKey` | `secret_key.rs:142` | OK |
| Master SK from seed | `SecretKey::from_seed(&seed_bytes)` | `secret_key.rs:94` | OK |
| Unhardened derivation | `sk.derive_unhardened(idx) -> SecretKey` and `pk.derive_unhardened(idx) -> PublicKey` | Via `DerivableKey` trait: `secret_key.rs:224`, `public_key.rs:282`, re-exported from `derive_keys.rs` | OK |
| Synthetic key | `pk.derive_synthetic() -> PublicKey`, ditto SecretKey | `chia_puzzle_types::DeriveSynthetic` (`derive_synthetic.rs:27,33`) | OK — already used in `chia-sdk-bindings/src/bls.rs:69-78,169-178` |
| G1 ser/deser (48-byte compressed) | `PublicKey::from_bytes(&[u8;48])`, `pk.to_bytes() -> [u8;48]` | `public_key.rs:94,117` — subgroup-membership-checked deserialisation | OK |
| BLS subgroup order constant | `r = 0x73eda753299d7d4833...01` | Reproduced in `chia_puzzle_types::derive_synthetic::GROUP_ORDER_BYTES` (line 8-9). Copy to a public const in the silent-payments module. | OK |

**Conclusion:** No blockers. The CHIP-0057 protocol is fully implementable on top of chia-bls 0.36.1 with zero upstream changes.

## Sources

- **chia-bls 0.36.1 source** (`/home/kdc/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/chia-bls-0.36.1/src/`) — HIGH. Inspected `public_key.rs`, `secret_key.rs`, `derive_keys.rs`, `lib.rs` line-by-line for every API the protocol needs.
- **chia-puzzle-types 0.36.1 source** (`/home/kdc/.cargo/registry/src/.../chia-puzzle-types-0.36.1/src/derive_synthetic.rs`, `puzzles/standard.rs`) — HIGH. Confirmed `DeriveSynthetic`, `StandardArgs::curry_tree_hash`, signed `mod_by_group_order`.
- **chia-sha2 0.36.1 source** (`/home/kdc/.cargo/registry/src/.../chia-sha2-0.36.1/src/lib.rs`) — HIGH. Confirmed `Sha256::new/update/finalize -> [u8;32]` API.
- **SDK Cargo.toml** (`/home/kdc/chia-wallet-sdk/Cargo.toml`) — HIGH. Verified workspace pins for chia-bls, chia-puzzle-types, chia-sha2, num-bigint, bip39, bech32, hex, hex-literal, rstest.
- **SDK existing crates** (`crates/chia-sdk-types/Cargo.toml`, `crates/chia-sdk-driver/Cargo.toml`, `crates/chia-sdk-utils/Cargo.toml`) — HIGH. Verified each crate already has the relevant deps for the CHIP-0057 module it will host.
- **SDK bindings** (`crates/chia-sdk-bindings/src/bls.rs`, `bindings/bls.json`) — HIGH. Confirmed PublicKey/SecretKey are already exposed as bindable classes; `bindings/silent_payments.json` follows the same template.
- **SDK Bech32 helper** (`crates/chia-sdk-utils/src/bech32.rs`) — HIGH. Confirmed `Bech32 { data: Bytes, prefix: String }` is generic over payload length, suitable for the 96-byte silent-payment payload.
- **Reference prototype** (`/home/kdc/silent-payments/crates/sp-common/src/*.rs`) — HIGH. Working Rust implementation; confirms the chia-bls + num-bigint + sha2 + bip39 stack composes correctly. Differs only in chia-bls version (0.41 vs 0.36.1) and SHA-256 crate (`sha2` vs `chia-sha2`), neither of which is a behavioral difference.
- **CHIP-0057 spec** (`/home/kdc/silent-payments/chip-silent-payments.md` §§Tagged Hash, Synthetic Key Computation Note, Identity Element, Test Cases TV1–TV4) — HIGH. Defines the four `Chia_SP/*` tags, the signed-vs-unsigned distinction, the zero-sum check, and four end-to-end test vectors with all intermediate values.
- **PROJECT.md and CONCERNS.md** (`.planning/PROJECT.md`, `.planning/codebase/CONCERNS.md`) — HIGH. Establish the "no new workspace deps" constraint and the type-level signed-vs-unsigned scalar separation.
- **WebSearch for BIP-352/CHIP-0057 Rust crates** (2026-05-15, via tools) — MEDIUM. Confirmed all existing rust silent-payments crates (`bip352`, `silentpayments`/`cygnet3`, `rust-bip352`/`jirijakes`) are secp256k1-only; none target BLS12-381 / Chia. Roll-our-own is the only path. Search results: [chips PR #198](https://github.com/Chia-Network/chips/pull/198), [bip352 on crates.io](https://crates.io/crates/bip352), [docs.rs/silentpayments](https://docs.rs/silentpayments/), [cygnet3/rust-silentpayments](https://github.com/cygnet3/rust-silentpayments), [jirijakes rust-bip352](https://jirijakes.com/code/rust-bip352/doc/trunk/README.md).

---
*Stack research for: CHIP-0057 silent payments integration into chia-wallet-sdk*
*Researched: 2026-05-15*
