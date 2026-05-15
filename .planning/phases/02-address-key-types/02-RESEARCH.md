# Phase 2: Address & key types — Research

**Researched:** 2026-05-15
**Domain:** BLS12-381 key derivation + bech32m address encoding (wallet-API surface)
**Confidence:** HIGH (every claim is grounded in either Phase 1's shipped code, the existing `chia-sdk-utils::Address`, the canonical Python prototype at `~/silent-payments/shared.py`, or the locally-recomputed CHIP test-vector address strings included verbatim below)

## Summary

Phase 2 lands the wallet-developer-facing key-and-address layer on top of Phase 1's primitives. The CHIP-0057 reference implementation (`~/silent-payments/shared.py` and the partial Rust port `~/silent-payments/crates/sp-common`) plus the Phase-1 derivation paths (`SCAN_PATH = &[12381, 8444, 12, 0]`, `SPEND_PATH = &[12381, 8444, 13, 0]`) dictate the exact construction: `mnemonic → BIP-39 seed → chia_bls::SecretKey::from_seed → derive_unhardened over path → SecretKey` for each of scan and spend, then `bech32m(spxch | tspxch, serialize(B_scan) || serialize(B_spend))` for the address (96-byte payload).

The dependency line `chia-sdk-utils → chia-sdk-types` does need to be added (Q8 from STATE.md): `SilentPaymentKeys::from_mnemonic` calls `derive_unhardened_path(SCAN_PATH)` and `derive_unhardened_path(SPEND_PATH)`, which live in `chia_sdk_types::silent_payments::paths`. Labels use the existing `chia_sdk_types::silent_payments::{tagged_hash, CHIA_SP_LABEL, ScalarField::from_bytes_unsigned}` — no new crypto primitives are required. The bech32m layer reuses `chia_sdk_utils::Bech32` directly (extending the existing pattern from `Address`), so the implementation is straightforward and well-scoped.

**Primary recommendation:** Build a new module `crates/chia-sdk-utils/src/silent_payments/` containing `keys.rs` (`SilentPaymentKeys`), `address.rs` (`SilentPaymentAddress` + `SilentPaymentNetwork`), `labels.rs` (`LabelRegistry` + `generate_label` helper), and `error.rs` (`SilentPaymentError`). Add `chia-sdk-types = { workspace = true, optional = true }` + `bip39 = { workspace = true, optional = true }` + `chia-bls = { workspace = true, optional = true }` to `chia-sdk-utils/Cargo.toml` and gate them all behind the existing empty `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]` feature. Pin all test inputs to the CHIP TV1 / TV3 byte values reproduced verbatim in section 8 below; round-trip the four address strings reproduced in section 8 (mainnet + testnet × unlabeled + labeled).

## User Constraints (from CONTEXT.md)

There is no CONTEXT.md for Phase 2 (no `/gsd:discuss-phase` was run). Constraints come from `./CLAUDE.md`, `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, and `.planning/ROADMAP.md`:

### Locked Decisions (from PROJECT.md "Key Decisions" + REQUIREMENTS.md)

- **Feature flag**: `chip-0057` is the ONLY umbrella feature for this work. No `silent-payments`, no name-aliases, no sub-features.
- **No new workspace dependencies**: `bip39 = 2.2.0`, `chia-bls = 0.36.1`, `bech32 = 0.9.1`, `hex = 0.4.3`, `hex-literal = 0.4.1`, `num-bigint = 0.4.6`, `chia-sha2 = 0.36.1`, `thiserror = 2.0.17` are already in `[workspace.dependencies]`.
- **No new top-level crate**: Phase 2 code lives in **existing crates** behind `chip-0057`, mirroring `chip-0035` / `chip-0037`. REQUIREMENTS.md annotates ADDR-01..06 as `chia-sdk-utils::silent_payments` — this is where the module lives.
- **`ScalarField` is the only mod-r reducer in `chip-0057` code paths**. No `mod_by_group_order` literal anywhere in `silent_payments/`. Label scalar computation uses `ScalarField::from_bytes_unsigned(tagged_hash(...))`.
- **HRP**: mainnet = `spxch`, testnet = `tspxch` (CHIP §153, §206, Python prototype line 460, 482). Bech32m variant only.
- **Payload layout**: 96 bytes = `serialize(B_scan) || serialize(B_spend)` (48 + 48). CHIP §206, Python prototype line 467.
- **Label `m = 0` is reserved for change** (CHIP §125-§130; Python prototype `generate_address.py:41`): `labeled_address(0)` MUST hard-error with a typed variant. (ADDR-06.)
- **`puzzle_hash_for_pk` from the prototype is DROPPED**. Reuse `chia_puzzle_types::standard::StandardArgs::curry_tree_hash(pk.derive_synthetic())`. This is Phase 4's concern, not Phase 2's; flagged here because it appears in `sp-common`.

### Claude's Discretion

- **Where each file lives within `chia-sdk-utils/src/silent_payments/`**. Recommended split is `mod.rs`/`keys.rs`/`address.rs`/`labels.rs`/`error.rs`; the planner may collapse `labels.rs` into `keys.rs` if it's small. The split is a wash either way.
- **Whether `SilentPaymentKeys` caches `scan_pk` / `spend_pk` eagerly or computes lazily**. `chia_bls::SecretKey::public_key()` is a non-trivial scalar-multiply, so caching is the slightly-faster option, but neither is wrong; pick caching for ergonomics (no `&mut self` needed for `scan_pk()`).
- **`LabelRegistry` internal data structure**. `HashMap<[u8; 48], u32>` (key = `label_pk.to_bytes()`) is the obvious choice for `lookup`. The reverse direction (`forward(m) -> PublicKey`) can either store a second `HashMap<u32, PublicKey>` or re-derive on demand from a stored `scan_sk`. Storing both directions is faster; re-derivation is smaller. Recommend storing both — labels are at most a few hundred per wallet.
- **Whether `Zeroize` is implemented on `SilentPaymentKeys`**. Phase 1 did NOT zeroize `ScalarField`'s inner `[u8; 32]`. Existing SDK types like `chia_bls::SecretKey` and `chia_sdk_test::BlsPair` also do not implement `Zeroize`. Recommend matching the SDK norm: NO `Zeroize` impl in Phase 2. Document in a doc-comment that the wallet author is responsible for secret-key lifetime hygiene.
- **`Debug` impl for `SilentPaymentKeys`**. `chia_bls::SecretKey` already implements `Debug` (it prints hex — leaks the key in logs). Either match this (`#[derive(Debug, Clone)]`) or write a manual `impl Debug` that prints `"SilentPaymentKeys {{ scan: <redacted>, spend: <redacted> }}"`. Recommend manual `Debug` for hygiene; the workspace lint `missing_debug_implementations` is `warn`, so this is mandatory either way.

### Deferred Ideas (OUT OF SCOPE)

- **Bindings** for `SilentPaymentKeys` / `SilentPaymentAddress` / `LabelRegistry` (Phase 5, BIND-01).
- **Bech32m round-trip in `napi`/`pyo3`/`wasm`** (Phase 5, BIND-01 — same descriptor).
- **`derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`** (Phase 4, SEND-01..03).
- **`scan_from_tweaks`, `TweakData`, `DetectedSpCoin`** (Phase 3, RECV-01..05). NOTE: `LabelRegistry` is built in Phase 2 (ADDR-04) but **consumed** by Phase 3's scanner — Phase 2 ships the data structure, Phase 3 plugs it into the scan loop.
- **CHIP-0058 transport-format wire types**. `TweakData` is transport-agnostic by design; nothing in Phase 2 touches transport.
- **Hardware-wallet scan-key custody UX**. `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` (ADDR-05) is what enables hot-online-scan-key + cold-offline-spend-key splits; we ship the constructor, the wallet author decides the policy.

## Project Constraints (from CLAUDE.md)

These are the actionable directives the planner must honor:

1. **Rust 1.90.0, edition 2024.** `rust-toolchain.toml` pins this. No nightly features.
2. **`unsafe_code = "deny"`.** No `unsafe` blocks anywhere in Phase 2 source.
3. **Workspace clippy** = `deny clippy::all` + `warn pedantic` + `warn cargo`. Local strict gate is `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings`.
4. **`cargo machete` clean**, no new `[package.metadata.cargo-machete] ignored` entries on `chia-sdk-utils`. (Currently zero entries on this crate.)
5. **`cargo fmt --all -- --files-with-diff --check`** clean.
6. **`[lints] workspace = true`** is already set in `chia-sdk-utils/Cargo.toml:14-15`. Do not add a per-crate override.
7. **Workspace dependency declaration pattern**: every dep is `{ workspace = true }`, never a literal version string in member `Cargo.toml`.
8. **`chia-sha2`, never bare `sha2`** (Phase 1 grep ban). Not load-bearing for Phase 2 (no SHA-256 in the address layer itself — labels delegate to Phase 1's `tagged_hash`), but if a Phase 2 file ever needs SHA-256 directly, use `chia_sha2::Sha256::{new, update, finalize}`.
9. **No `From<[u8; 32]> for ScalarField`** (Phase 1 type-boundary discipline). Phase 2 callers convert label-hash bytes via the existing `ScalarField::from_bytes_unsigned`.
10. **No `mod_by_group_order` literal** anywhere in `chip-0057`-gated code (Phase 1 grep ban).
11. **Examples are tested implicitly** via `cargo build --all-features` in CI. Phase 2 ships no example (EX-01 is Phase 6).
12. **GSD workflow enforcement**: before any Edit/Write, work goes through a GSD command.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ADDR-01 | `SilentPaymentKeys` derives scan + spend SKs from a BIP-39 mnemonic at `m/12381/8444/12/0` and `m/12381/8444/13/0` using `bip39 = 2.2.0` + `chia-bls::SecretKey::from_seed` + `derive_unhardened`. | §1 (location), §2 (`SilentPaymentKeys::from_mnemonic` signature + path constants), §11 (public API), §12 (validation map). Phase 1 already shipped `SCAN_PATH` / `SPEND_PATH` (`crates/chia-sdk-types/src/silent_payments/paths.rs`). The CHIP TV1 vectors at lines 496-499 of `chip-silent-payments.md` give the exact byte values to assert against. |
| ADDR-02 | `SilentPaymentAddress` encodes / decodes a bech32m string with HRP `spxch` (mainnet) / `tspxch` (testnet) over the 96-byte `serialize(B_scan) \|\| serialize(B_spend)` payload. Round-trips against CHIP test vectors. | §3 (type design), §4 (HRP + payload layout), §8 (recomputed canonical TV1 / TV3 address strings), §9 (bech32 0.9 API), §11, §12. The Python prototype `shared.py:460-505` is the byte-exact reference; we reuse `chia_sdk_utils::Bech32` from `bech32.rs:31-53` for the encode / decode primitive. |
| ADDR-03 | `SilentPaymentKeys::labeled_address(m)` produces a labeled sub-address where `B_spend` is replaced by `B_spend + label_pk(m)`. Scan key is unchanged across labels. | §2 (`labeled_address` signature), §5 (label tweak math + tag verification), §11. Reference: `~/silent-payments/crates/sp-common/src/protocol.rs:42-51` (`generate_label`) and CHIP TV3 lines 615-619. Label scalar = `ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_LABEL, b_scan_bytes ∥ ser32(m)))`. Pin against `label_scalar = 48fa440a...c48ce465`, `label_pk = a6dcff36...d80c`, `B_m = 965250fb...414c2` from TV3. |
| ADDR-04 | `LabelRegistry` (dedicated type, not bare `HashMap`) maintains `label_pk → label_index`. | §6 (data structure + API). Stores both directions: `forward: HashMap<u32, PublicKey>` and `reverse: HashMap<[u8; 48], u32>`. Built from `(scan_sk, &[m])` via `LabelRegistry::register(scan_sk, m)`. |
| ADDR-05 | `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` enables watch-only / key-import. | §2 (constructor signature), §11, §12. A trivial constructor — the public-key derivation is the same code path as `from_mnemonic` post-derivation. Validation: build from raw SKs that match TV1's `b_scan`/`b_spend` bytes; assert `unlabeled_address()` produces the same string as the `from_mnemonic` path. |
| ADDR-06 | `labeled_address(0)` rejected with `SilentPaymentError::ReservedChangeLabel`. | §2 (signature returns `Result`), §5 (CHIP §125-§130 designates m=0 as change), §7 (error enum), §11, §12. Single-branch test: `SilentPaymentKeys::from_mnemonic(...).labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)`. |

## 1. Where the types live

**Decision: `crates/chia-sdk-utils/src/silent_payments/` (NOT `chia-sdk-types`).**

Rationale:

- REQUIREMENTS.md ADDR-01..06 explicitly annotates the destination as `chia-sdk-utils::silent_payments`.
- The Address-family precedent (`chia_sdk_utils::Address`, `chia_sdk_utils::Bech32`) lives in `chia-sdk-utils`. The wire-protocol / type-definition crate is `chia-sdk-types`; the wallet-API / encoding-helper crate is `chia-sdk-utils`.
- Phase 1 shipped `chia-sdk-utils/Cargo.toml` with its first-ever `[features]` block (`chip-0057 = []`) precisely so Phase 2 could populate it.
- The CHIP `TweakData` / `DetectedSpCoin` wire types (Phase 3) belong in `chia-sdk-types` (or `chia-sdk-driver`); the wallet-API key/address types (Phase 2) belong in `chia-sdk-utils`. The split mirrors CLAUDE.md's "wire-protocol surfaces go in `chia-sdk-types`; wallet API goes in the silent-payments crate Phase 1 established" rule.

**Q8 resolution** (from STATE.md Blockers): Yes, Phase 2 ADDS a `dep:chia-sdk-types` edge on `chia-sdk-utils`. The dependency is required because:

- `SilentPaymentKeys::from_mnemonic` needs `SCAN_PATH` and `SPEND_PATH` from `chia_sdk_types::silent_payments::paths` (the single source of truth pinned in Phase 1).
- Labels need `tagged_hash`, `CHIA_SP_LABEL`, and `ScalarField::from_bytes_unsigned` from `chia_sdk_types::silent_payments` (Phase 1's tag constants + the unsigned mod-r reducer that enforces the protocol's signed-vs-unsigned discipline).

This edge is optional (`optional = true` on the dep, activated by the existing `chip-0057` feature) so the no-features build of `chia-sdk-utils` continues to compile without `chia-sdk-types`. The downstream impact is nil — no other consumer of `chia-sdk-utils` cares about its `chip-0057` feature unless they themselves enable it.

**Module file layout** (recommended; the planner may collapse):

```
crates/chia-sdk-utils/src/silent_payments/
├── mod.rs            # barrel + `pub use *` from submodules; doc-comment about chip-0057
├── error.rs          # SilentPaymentError + #[from] conversions
├── address.rs        # SilentPaymentAddress + SilentPaymentNetwork enum
├── keys.rs           # SilentPaymentKeys with from_mnemonic + from_secret_keys + scan_pk/spend_pk + unlabeled_address + labeled_address
└── labels.rs         # LabelRegistry + private generate_label helper (or fold into keys.rs)
```

`crates/chia-sdk-utils/src/lib.rs` gets a new `#[cfg(feature = "chip-0057")] pub mod silent_payments;` declaration (mirrors how `chia-sdk-types/src/lib.rs:3-4` does it).

## 2. `SilentPaymentKeys` type design

**Fields** (caching the public keys for ergonomic `scan_pk()` / `spend_pk()`):

```rust
#[derive(Clone)]
pub struct SilentPaymentKeys {
    scan_sk: chia_bls::SecretKey,
    spend_sk: chia_bls::SecretKey,
    // Cached at construction; both are scalar-multiplies on chia-bls G1.
    scan_pk: chia_bls::PublicKey,
    spend_pk: chia_bls::PublicKey,
}

// Manual Debug impl that redacts secret material:
impl core::fmt::Debug for SilentPaymentKeys {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SilentPaymentKeys")
            .field("scan_pk", &self.scan_pk)
            .field("spend_pk", &self.spend_pk)
            .field("scan_sk", &"<redacted>")
            .field("spend_sk", &"<redacted>")
            .finish()
    }
}
```

**Constructors:**

```rust
impl SilentPaymentKeys {
    /// Derive (scan_sk, spend_sk) from a BIP-39 mnemonic at the CHIP-0057 paths
    /// m/12381/8444/12/0 (scan) and m/12381/8444/13/0 (spend). Uses an empty
    /// passphrase, matching `chia_sdk_test::BlsPair::new` and
    /// `chia_sdk_bindings::Mnemonic`.
    #[must_use]
    pub fn from_mnemonic(mnemonic: &bip39::Mnemonic) -> Self {
        let seed = mnemonic.to_seed("");
        let master = chia_bls::SecretKey::from_seed(&seed);
        let scan_sk = derive_path(&master, chia_sdk_types::silent_payments::SCAN_PATH);
        let spend_sk = derive_path(&master, chia_sdk_types::silent_payments::SPEND_PATH);
        Self::from_secret_keys(scan_sk, spend_sk)
    }

    /// Build from raw scan + spend secret keys. Enables watch-only and
    /// key-import flows where the consumer holds keys derived elsewhere.
    #[must_use]
    pub fn from_secret_keys(
        scan_sk: chia_bls::SecretKey,
        spend_sk: chia_bls::SecretKey,
    ) -> Self {
        let scan_pk = scan_sk.public_key();
        let spend_pk = spend_sk.public_key();
        Self { scan_sk, spend_sk, scan_pk, spend_pk }
    }

    #[must_use] pub fn scan_sk(&self)  -> &chia_bls::SecretKey { &self.scan_sk }
    #[must_use] pub fn spend_sk(&self) -> &chia_bls::SecretKey { &self.spend_sk }
    #[must_use] pub fn scan_pk(&self)  -> &chia_bls::PublicKey { &self.scan_pk }
    #[must_use] pub fn spend_pk(&self) -> &chia_bls::PublicKey { &self.spend_pk }

    /// Build the unlabeled bech32m silent-payment address.
    #[must_use]
    pub fn unlabeled_address(&self, network: SilentPaymentNetwork) -> SilentPaymentAddress {
        SilentPaymentAddress::new(self.scan_pk, self.spend_pk, network)
    }

    /// Build a labeled bech32m sub-address. `m = 0` is reserved for change
    /// and hard-errors with `SilentPaymentError::ReservedChangeLabel`.
    pub fn labeled_address(
        &self,
        network: SilentPaymentNetwork,
        m: u32,
    ) -> Result<SilentPaymentAddress, SilentPaymentError> {
        if m == 0 {
            return Err(SilentPaymentError::ReservedChangeLabel);
        }
        let (_label_scalar, label_pk) = generate_label(&self.scan_sk, m);
        let labeled_spend_pk = self.spend_pk + &label_pk;
        Ok(SilentPaymentAddress::new(self.scan_pk, labeled_spend_pk, network))
    }
}

/// Private helper: apply unhardened BIP-32 derivation across a path slice.
/// Matches `chia_sdk_bindings::SecretKeyExt::derive_unhardened_path`'s
/// per-index call pattern.
fn derive_path(sk: &chia_bls::SecretKey, path: &[u32]) -> chia_bls::SecretKey {
    use chia_bls::DerivableKey;
    let mut out = sk.clone();
    for &index in path {
        out = out.derive_unhardened(index);
    }
    out
}
```

**Note on `account` parameter**: The CHIP fixes the path at `m/12381/8444/12/0` and `m/12381/8444/13/0` — no account index, no chain index, no address index. The Python prototype (`shared.py`), the partial Rust port (`sp-common/src/keys.rs`), and the SCAN/SPEND constants pinned in Phase 1 all use exactly these paths. The phase ROADMAP success criterion 1 references `TV1_mnemonic` → fixed-path derivation. `from_mnemonic` takes **only** `&bip39::Mnemonic` — no account index. A future spec amendment could add multiple accounts, but that's out of scope for v1.

**`Zeroize` / `ZeroizeOnDrop`**: Not implemented. `chia_bls::SecretKey` does not implement `Zeroize`; matching the SDK norm. Document the hygiene caveat in a doc-comment on the `SilentPaymentKeys` struct.

## 3. `SilentPaymentAddress` type design

**Internal representation: two `PublicKey`s.** This is the natural shape for both the encoded form (concat them) and downstream consumption (the address conveys exactly what a sender + scanner need to look up — scan + spend public keys). It also makes labeled vs unlabeled invisible on the wire (per CHIP §375: "the labeled silent payment address is (B_scan, B_m). Note that B_scan is unchanged"). The `spend_pk` field simply holds `B_m` for a labeled address; the type carries no labeled-vs-unlabeled flag, because senders / scanners can't tell the difference from the address bytes alone.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilentPaymentNetwork {
    Mainnet,  // HRP "spxch"
    Testnet,  // HRP "tspxch"
}

impl SilentPaymentNetwork {
    #[must_use]
    pub fn hrp(self) -> &'static str {
        match self {
            Self::Mainnet => "spxch",
            Self::Testnet => "tspxch",
        }
    }

    pub fn from_hrp(hrp: &str) -> Result<Self, SilentPaymentError> {
        match hrp {
            "spxch"  => Ok(Self::Mainnet),
            "tspxch" => Ok(Self::Testnet),
            other    => Err(SilentPaymentError::WrongHrp(other.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SilentPaymentAddress {
    pub scan_pk: chia_bls::PublicKey,
    pub spend_pk: chia_bls::PublicKey,   // B_spend (unlabeled) OR B_m (labeled)
    pub network: SilentPaymentNetwork,
}

impl SilentPaymentAddress {
    #[must_use]
    pub fn new(
        scan_pk: chia_bls::PublicKey,
        spend_pk: chia_bls::PublicKey,
        network: SilentPaymentNetwork,
    ) -> Self {
        Self { scan_pk, spend_pk, network }
    }

    /// Encode as bech32m: HRP || "1" || base32(scan_pk || spend_pk || checksum).
    /// Payload is the 96-byte concatenation `serialize(B_scan) || serialize(B_spend)`.
    pub fn encode(&self) -> Result<String, SilentPaymentError> {
        let mut payload = Vec::with_capacity(96);
        payload.extend_from_slice(&self.scan_pk.to_bytes());   // 48 bytes
        payload.extend_from_slice(&self.spend_pk.to_bytes());  // 48 bytes
        debug_assert_eq!(payload.len(), 96);

        // Reuse the existing Bech32 helper. It already enforces bech32m.
        let b = chia_sdk_utils::Bech32::new(
            chia_protocol::Bytes::new(payload),
            self.network.hrp().to_string(),
        );
        Ok(b.encode()?)
    }

    /// Decode a bech32m silent-payment address. Rejects:
    ///   - HRP not in {"spxch", "tspxch"}                 → WrongHrp
    ///   - bech32 / bech32m parse failure                 → Bech32(...)
    ///   - non-bech32m variant (i.e. plain bech32)        → Bech32(InvalidFormat) (carried from sdk-utils' Bech32::decode)
    ///   - payload length != 96 bytes                     → PayloadLength
    ///   - either pubkey half is invalid bytes            → InvalidPublicKey
    ///   - either pubkey half is the identity element     → IdentityPublicKey
    pub fn decode(s: &str) -> Result<Self, SilentPaymentError> {
        let b = chia_sdk_utils::Bech32::decode(s)?;
        let network = SilentPaymentNetwork::from_hrp(&b.prefix)?;
        let payload: Vec<u8> = b.data.to_vec();
        if payload.len() != 96 {
            return Err(SilentPaymentError::PayloadLength(payload.len()));
        }
        let scan_bytes:  [u8; 48] = payload[..48].try_into().expect("96 == 48 + 48");
        let spend_bytes: [u8; 48] = payload[48..].try_into().expect("96 == 48 + 48");
        let scan_pk  = chia_bls::PublicKey::from_bytes(&scan_bytes)
            .map_err(|_| SilentPaymentError::InvalidPublicKey)?;
        let spend_pk = chia_bls::PublicKey::from_bytes(&spend_bytes)
            .map_err(|_| SilentPaymentError::InvalidPublicKey)?;
        if scan_pk.is_inf() || spend_pk.is_inf() {
            return Err(SilentPaymentError::IdentityPublicKey);
        }
        Ok(Self { scan_pk, spend_pk, network })
    }
}
```

**Labeled-vs-unlabeled distinguishability**: Confirmed NONE. The CHIP §375 explicitly says: "The labeled silent payment address is (B_scan, B_m). Note that B_scan is unchanged — the scan key is shared across all labels." A labeled address looks identical on the wire to an unlabeled one — only the spend-pk-half is different (a curve point that the sender can't distinguish from any other valid spend pubkey). Recipient detection works by trying each registered label_pk in turn (Phase 3, RECV-04).

## 4. HRP + payload layout

| Property | Value | Source |
|---|---|---|
| Mainnet HRP | `spxch` | CHIP §153, §206, `shared.py:482`, ROADMAP success criterion 1 |
| Testnet HRP | `tspxch` | CHIP §153, §206, `shared.py:482` |
| Variant | bech32**m** (NOT plain bech32) | CHIP §153 ("bech32m"); `shared.py:469` uses `BECH32M_CONST = 0x2bc830a3`; existing `chia_sdk_utils::Bech32::decode` requires `Variant::Bech32m` (`bech32.rs:34`) |
| Payload | 96 bytes = `serialize(B_scan) ‖ serialize(B_spend)` | CHIP §206, `shared.py:467` |
| Pubkey serialization | `chia_bls::PublicKey::to_bytes() -> [u8; 48]` (compressed G1, big-endian) | `chia-bls 0.36.1`; verified in `chia_sdk_bindings::bls::PublicKeyExt::to_bytes` returns `Bytes48` |
| Witness-version byte | NONE | The CHIP does NOT use a witness-version prefix. The 96-byte payload IS the data. Compare to the SDK's existing `xch1...` address: `chia_sdk_utils::Address` also does not use a witness-version byte (32 raw puzzle-hash bytes). |
| Encoded length | 153 chars (testnet) / 152 chars (mainnet) | Recomputed locally — see §8 |
| Max-length limit | None enforced by `bech32 = 0.9.1` for bech32m | Verified: `bech32-0.9.1/src/lib.rs:367-369` only enforces `1..=83` on the HRP and `>= 6` on the data part. The "soft 90-char rule" from BIP-173 is not enforced. See §13 Pitfall 4. |
| Mixed case | Forbidden by bech32 (the crate rejects with `Error::MixedCase`) | `bech32-0.9.1` `Error::MixedCase` exists; the existing `chia_sdk_utils::Bech32::decode` propagates it via `#[from] bech32::Error` |

**Confirmation of exact byte layout** (from `shared.py:467` cross-referenced with CHIP §206):

```text
payload[0..48]  = scan_pk.to_bytes()        # B_scan, compressed G1
payload[48..96] = spend_pk.to_bytes()       # B_spend (unlabeled) OR B_m (labeled)
```

There is no version byte, no length prefix, no separator inside the payload. Just two 48-byte BLS G1 public keys concatenated.

## 5. Label tweak math

**Exact formula** (CHIP §125-§130; `shared.py:268-278`; `~/silent-payments/crates/sp-common/src/protocol.rs:42-51`):

```text
label_data    = ser256(b_scan) || ser32(m)      # 32 + 4 = 36 bytes
label_scalar  = int(tagged_hash("Chia_SP/Label", label_data)) mod r
label_pk      = label_scalar * G                # i.e. SecretKey::from_bytes(label_scalar).public_key()
B_m           = B_spend + label_pk              # G1 point addition
```

Where:

- `ser256(b_scan)` = the 32-byte big-endian `chia_bls::SecretKey::to_bytes()` of the scan SK.
- `ser32(m)` = `m.to_be_bytes()` (4 bytes, big-endian).
- `"Chia_SP/Label"` is pinned in Phase 1 as `chia_sdk_types::silent_payments::CHIA_SP_LABEL: &'static str = "Chia_SP/Label"` (verified at `chia-sdk-types/src/silent_payments/tagged_hash.rs:21`).
- The tagged-hash output (32 bytes) is reduced **unsigned** mod `r` via `ScalarField::from_bytes_unsigned` (Phase 1's CRYPTO-01 boundary). This is the third instance of unsigned-mod-r-via-`ScalarField` in the protocol (input_hash, output_tweak, label_scalar).
- `label_scalar * G` is realized in `chia-bls` by constructing a `SecretKey` from the 32-byte scalar and calling `public_key()` — same pattern as `~/silent-payments/crates/sp-common/src/protocol.rs:48` (`SecretKey::from_bytes(label_scalar.as_bytes()).public_key()`).
- `B_spend + label_pk` is `chia_bls::PublicKey + &chia_bls::PublicKey` (the bindings layer uses this exact pattern at `bls.rs:114`: `result += &pk;`).

**Tag verification** (Phase 1 pin):

- `CHIA_SP_LABEL = "Chia_SP/Label"` (literal). Confirmed in `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs:21`.
- `SHA256("Chia_SP/Label")` = `c63c8bd2123be129023b8f24c0249bacbd83da15e2b883ec5a03aa62b0f94554` — pinned in Phase 1's `tag_label_hash_pinned` test, cross-verified vs Python and `sha256sum`.
- The tag is **passed as a string** to `tagged_hash(tag: &str, data: &[u8])`; the helper hashes the tag-bytes inside per BIP-340. Don't pre-hash; pass the string constant.

**Helper function signature** (private, in `labels.rs` or `keys.rs`):

```rust
/// Generate the label scalar and label public key for label index `m`.
///
/// Returns (label_scalar_bytes, label_pk).
/// Used internally by SilentPaymentKeys::labeled_address and by LabelRegistry::register.
///
/// `m = 0` is the change-label sentinel in CHIP-0057 and must be rejected at
/// the public boundary (`labeled_address`); this helper does NOT enforce that
/// because change-detection (Phase 6, m=0 internal-only) legitimately needs it.
fn generate_label(scan_sk: &chia_bls::SecretKey, m: u32)
    -> (chia_sdk_types::silent_payments::ScalarField, chia_bls::PublicKey)
{
    let mut data = [0u8; 36];
    data[..32].copy_from_slice(&scan_sk.to_bytes());
    data[32..].copy_from_slice(&m.to_be_bytes());

    let hash = chia_sdk_types::silent_payments::tagged_hash(
        chia_sdk_types::silent_payments::CHIA_SP_LABEL,
        &data,
    );
    let label_scalar = chia_sdk_types::silent_payments::ScalarField::from_bytes_unsigned(hash);
    // Reuse chia-bls to do scalar * G. SecretKey::from_bytes accepts any
    // 32-byte value < r, and from_bytes_unsigned reduces mod r so the
    // result is always in range.
    let label_sk = chia_bls::SecretKey::from_bytes(label_scalar.as_bytes())
        .expect("ScalarField::from_bytes_unsigned guarantees value < r");
    (label_scalar, label_sk.public_key())
}
```

**Pin against TV3** (CHIP `chip-silent-payments.md:617-619`):

- `b_scan = 132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6`
- For `m = 1`: `label_scalar = 48fa440acca87f501b9984b5d23327d0b7766a4baa913dfb3001d412c48ce465`
- For `m = 1`: `label_pk = a6dcff3646739745ef7f3ba8e51808dac13765fa9d5e73386d3fbd7841e0773e02a0f8d91baf57d337954322bd06d80c`
- `B_spend + label_pk = B_m = 965250fb8503cff4c244f360ab84075bfe2da01091745d0e8ce36024ab12e96277d1f02fbbe01cee412dd2ce1b7414c2`

These are the canonical assertions for the label-generation unit tests.

## 6. `LabelRegistry` design

**Lives in Phase 2 (per ROADMAP success criterion 6 and ADDR-04), consumed by Phase 3.** Phase 2 ships the data structure + `register` + `forward` + `lookup`; Phase 3's scanner will plug `lookup(label_pk)` into its k-iteration loop (see `~/silent-payments/crates/sp-common/src/protocol.rs:148-169` for the consumption pattern).

```rust
/// A bidirectional registry of label-index → label-public-key mappings,
/// keyed by a scan secret key. Used by scanner code to attribute a labeled
/// detection back to its label index.
///
/// Storage cost: two HashMap entries per registered label. For a wallet with
/// a few hundred labels (the realistic upper bound), total memory is < 100 KB.
#[derive(Clone, Debug, Default)]
pub struct LabelRegistry {
    forward: std::collections::HashMap<u32, chia_bls::PublicKey>,
    reverse: std::collections::HashMap<[u8; 48], u32>,
}

impl LabelRegistry {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Register label `m` against the scan secret key `scan_sk`. Computes the
    /// label public key and stores it bidirectionally.
    ///
    /// `m = 0` is the change-label sentinel and IS allowed here (the scanner
    /// needs to register the change label internally to detect its own change
    /// outputs in Phase 6's labeled E2E test). The public-API change-label
    /// rejection happens in `SilentPaymentKeys::labeled_address`, NOT here.
    pub fn register(&mut self, scan_sk: &chia_bls::SecretKey, m: u32) {
        let (_label_scalar, label_pk) = generate_label(scan_sk, m);
        let bytes = label_pk.to_bytes();
        self.forward.insert(m, label_pk);
        self.reverse.insert(bytes, m);
    }

    /// Look up the label public key for a registered label index.
    #[must_use]
    pub fn forward(&self, m: u32) -> Option<&chia_bls::PublicKey> {
        self.forward.get(&m)
    }

    /// Look up the label index that registers a given label public key.
    /// Used by the scanner to attribute a labeled detection.
    #[must_use]
    pub fn lookup(&self, label_pk: &chia_bls::PublicKey) -> Option<u32> {
        self.reverse.get(&label_pk.to_bytes()).copied()
    }

    /// Number of registered labels.
    #[must_use]
    pub fn len(&self) -> usize { self.forward.len() }

    /// True if no labels are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.forward.is_empty() }

    /// Iterate registered (m, label_pk) pairs. Useful for the scanner's
    /// labeled-detection branch (Phase 3, RECV-04).
    pub fn iter(&self) -> impl Iterator<Item = (u32, &chia_bls::PublicKey)> {
        self.forward.iter().map(|(&m, pk)| (m, pk))
    }
}
```

**Round-trip property** (ADDR-04 success criterion 6):

```text
let mut reg = LabelRegistry::new();
reg.register(&scan_sk, 1);
let label_pk = reg.forward(1).unwrap().clone();
assert_eq!(reg.lookup(&label_pk), Some(1));
```

This is the canonical test for ADDR-04.

## 7. `SilentPaymentError` enum variants

```rust
#[derive(Debug, thiserror::Error)]
pub enum SilentPaymentError {
    /// Decoded HRP is neither "spxch" (mainnet) nor "tspxch" (testnet).
    /// Specifically catches "xch1..." (the standard Chia address) being
    /// passed to SilentPaymentAddress::decode.
    #[error("invalid silent-payment HRP '{0}' (expected 'spxch' or 'tspxch')")]
    WrongHrp(String),

    /// The decoded payload is not 96 bytes. Either too short or too long.
    #[error("invalid silent-payment payload length: expected 96 bytes, got {0}")]
    PayloadLength(usize),

    /// One of the 48-byte pubkey halves failed `chia_bls::PublicKey::from_bytes`
    /// (e.g., not a valid compressed G1 point).
    #[error("invalid silent-payment public-key encoding")]
    InvalidPublicKey,

    /// Either the scan or spend pubkey decoded to the BLS identity element
    /// (point at infinity, all-zero compressed bytes + infinity flag). This
    /// catches `B_scan = 0 * G` or `B_spend = 0 * G`, which the CHIP requires
    /// rejecting to prevent trivial-secret-key griefing.
    #[error("silent-payment public key is the identity element")]
    IdentityPublicKey,

    /// `labeled_address(0)` was called. `m = 0` is reserved as the change
    /// label and must not appear in a publicly-shared address.
    #[error("label index 0 is reserved for change outputs and cannot be exposed")]
    ReservedChangeLabel,

    /// bech32 / bech32m parse, checksum, or character error. Wraps the
    /// existing `chia_sdk_utils::Bech32Error` so the error message stays
    /// uniform with the standard-address path.
    #[error("bech32m error: {0}")]
    Bech32(#[from] chia_sdk_utils::Bech32Error),
}
```

**`#[from]` conversions:**

- `From<chia_sdk_utils::Bech32Error>` (handles wrong-prefix lookups through the existing `Bech32` decoder, bad checksum, mixed case, invalid char, invalid length, padding errors). This brings the whole `bech32::Error` family under the umbrella for free, because `Bech32Error::Decode(#[from] bech32::Error)` is already wired (`crates/chia-sdk-utils/src/bech32.rs:17`).

**Why a separate `WrongHrp` (not delegated to `Bech32::expect_prefix`)**: the existing `Bech32::expect_prefix` only checks ONE prefix at a time. We need to accept TWO (spxch + tspxch). Easier to inspect `b.prefix` ourselves and emit `SilentPaymentError::WrongHrp` than to call `expect_prefix` twice. Same outcome semantically.

**Why a separate `InvalidPublicKey` and `IdentityPublicKey`**: malformed-bytes-vs-identity-element are observably different decode failures. The CHIP-mandated rejection (CHIP §215 "implementations MUST reject the point at infinity") deserves its own variant for clarity.

## 8. CHIP test vectors

**Source**: `~/silent-payments/chip-silent-payments.md` (the spec text) — lines 477 (TV1), 589 (TV3). The spec does NOT include the encoded `spxch1...` / `tspxch1...` strings directly. The intermediate hex values are present; the encoded address strings have been **recomputed locally** by running the Python reference `encode_silent_payment_address(scan_pk, spend_pk, prefix)` from `~/silent-payments/shared.py:460-471` against the published TV1 / TV3 hex pubkeys. The recomputed strings are reproduced below as the canonical assertions for Phase 2.

**TV1 (Test Vector 1) — Single Output, Unlabeled**

Inputs:

```text
Mnemonic:       "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                (BIP-39 test vector)
SCAN_PATH:      m/12381/8444/12/0
SPEND_PATH:     m/12381/8444/13/0
```

Derived bytes (assert these against `SilentPaymentKeys::from_mnemonic(...)`):

```text
b_scan  (32 B): 132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6
B_scan  (48 B): a04f404bfbfdc9311736899fe32d2275bb007814510c3523529487ad7573607573ade20d31c75107b40331fff79ac896
b_spend (32 B): 53d140b312a0e16316314274eb6398e15706d100fe8a754990540febd931b087
B_spend (48 B): 8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527dbf6f31e22e0831ba5088c952625a75c74
```

Encoded unlabeled silent-payment addresses (recomputed via `shared.py:460-471`):

```text
Mainnet (spxch):
  spxch15p85qjlmlhynz9ek3x07xtfzwkasq7q52yxr2g6jjjr66atnvp6h8t0zp5cuw5g8kspnrllhntyfdzhutqqe9az04d3y7cfnd8me9mlnyg828j5z96urn2evjvy72f7m7me3ughqsvd62zyvj5nztf6uwsfn2u2q

Testnet (tspxch):
  tspxch15p85qjlmlhynz9ek3x07xtfzwkasq7q52yxr2g6jjjr66atnvp6h8t0zp5cuw5g8kspnrllhntyfdzhutqqe9az04d3y7cfnd8me9mlnyg828j5z96urn2evjvy72f7m7me3ughqsvd62zyvj5nztf6uwsnlcstc
```

(Note: the two strings differ only in (a) the HRP and (b) the 6-character bech32m checksum at the end, exactly as expected.)

**TV3 (Test Vector 3) — Single Output, Labeled `m = 1`**

Sender + recipient base keys identical to TV1. Additional label derivation:

```text
label_scalar (32 B): 48fa440acca87f501b9984b5d23327d0b7766a4baa913dfb3001d412c48ce465
label_pk     (48 B): a6dcff3646739745ef7f3ba8e51808dac13765fa9d5e73386d3fbd7841e0773e02a0f8d91baf57d337954322bd06d80c
B_m          (48 B): 965250fb8503cff4c244f360ab84075bfe2da01091745d0e8ce36024ab12e96277d1f02fbbe01cee412dd2ce1b7414c2
                     (= B_spend + label_pk)
```

Encoded labeled silent-payment addresses (recomputed via `shared.py:460-471` against scan_pk + B_m):

```text
Mainnet (spxch, m=1):
  spxch15p85qjlmlhynz9ek3x07xtfzwkasq7q52yxr2g6jjjr66atnvp6h8t0zp5cuw5g8kspnrllhntyfd9jj2rac2q707npyfumq4wzqwkl79ksppyt5t58gecmqyj4396tzwlglqtamuqwwusfd6t8pkaq5cg4qwg5m

Testnet (tspxch, m=1):
  tspxch15p85qjlmlhynz9ek3x07xtfzwkasq7q52yxr2g6jjjr66atnvp6h8t0zp5cuw5g8kspnrllhntyfd9jj2rac2q707npyfumq4wzqwkl79ksppyt5t58gecmqyj4396tzwlglqtamuqwwusfd6t8pkaq5cg0vuy4r
```

**How to vendor in the test file**: paste these strings as `const TV1_MAINNET_ADDR: &str = "spxch1..."`; `const TV3_TESTNET_ADDR: &str = "tspxch1..."`; etc. No JSON vendoring needed — the values are small and there are exactly 4 of them. The same goes for the hex byte sequences: `hex_literal::hex!("...")` is the cleanest pattern (already a workspace dep, already imported in `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs:44`).

**Recomputation instructions** for the verifier (so the verifier can re-derive the same strings):

```bash
cd /home/kdc/silent-payments
python3 -c "
import sys
sys.path.insert(0, '.')
from shared import encode_silent_payment_address
scan_pk  = bytes.fromhex('a04f404bfbfdc9311736899fe32d2275bb007814510c3523529487ad7573607573ade20d31c75107b40331fff79ac896')
spend_pk = bytes.fromhex('8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527dbf6f31e22e0831ba5088c952625a75c74')
b_m      = bytes.fromhex('965250fb8503cff4c244f360ab84075bfe2da01091745d0e8ce36024ab12e96277d1f02fbbe01cee412dd2ce1b7414c2')
print('TV1 main:', encode_silent_payment_address(scan_pk, spend_pk, 'spxch'))
print('TV1 test:', encode_silent_payment_address(scan_pk, spend_pk, 'tspxch'))
print('TV3 main:', encode_silent_payment_address(scan_pk, b_m,      'spxch'))
print('TV3 test:', encode_silent_payment_address(scan_pk, b_m,      'tspxch'))
"
```

(This command was executed during research; the output is reproduced verbatim above.)

## 9. bech32 crate version

**Pinned at `bech32 = 0.9.1`** in `[workspace.dependencies]` (root `Cargo.toml:139`). Already a dependency of `chia-sdk-utils` (`crates/chia-sdk-utils/Cargo.toml:23`). No version bump required.

**API surface (0.9.1):**

```rust
// Encoding (single 5-bit-data + variant)
pub fn encode<T: AsRef<[u5]>>(hrp: &str, data: T, variant: Variant) -> Result<String, Error>

// Decoding (returns 5-bit data + variant — caller picks the variant they want)
pub fn decode(s: &str) -> Result<(String, Vec<u5>, Variant), Error>

// 5-bit / 8-bit conversion (lossless with pad=true on encode, pad=false on decode)
pub fn convert_bits(data: &[u8], from_bits: u32, to_bits: u32, pad: bool)
    -> Result<Vec<u8>, Error>

// Variant
pub enum Variant { Bech32, Bech32m }

// Error
pub enum Error {
    MissingSeparator, InvalidChecksum, InvalidLength, InvalidChar(char),
    InvalidData(u8), InvalidPadding, MixedCase,
}
```

**Length-limit check** (source-confirmed): bech32 0.9.1 enforces only HRP length 1..=83 and data part `>= 6` (checksum). It does NOT enforce the 90-character total-string limit from BIP-173. A 96-byte payload converts to 154 base32 characters + 6 checksum chars = ~160 chars of data → ~167 chars total including HRP and separator. This compiles and decodes cleanly. (Compare: 0.11 changed both the API and added stricter checks; we do NOT use 0.11.)

**Reuse pattern**: `chia_sdk_utils::Bech32::{new, encode, decode}` already wraps `bech32::{encode, decode}` (`crates/chia-sdk-utils/src/bech32.rs:30-53`). Phase 2 SHOULD reuse `chia_sdk_utils::Bech32` rather than calling `bech32::*` directly:

- `SilentPaymentAddress::encode` calls `Bech32::new(payload_bytes, hrp.to_string()).encode()`.
- `SilentPaymentAddress::decode` calls `Bech32::decode(s)?`, then validates `prefix ∈ {spxch, tspxch}`, then validates payload length == 96, then deserializes the two pubkeys.

Reuse keeps the bech32m enforcement (`Bech32::decode` rejects plain bech32 with `Bech32Error::InvalidFormat` — `bech32.rs:34-36`) in one place. The Phase 2 code adds the silent-payment-specific validations on top.

## 10. Feature gating

**Phase 1 shipped the cascade:**

- Root `Cargo.toml:75` — `chip-0057 = ["chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]`
- `crates/chia-sdk-types/Cargo.toml:20` — `chip-0057 = []` (with code)
- `crates/chia-sdk-driver/Cargo.toml:23` — `chip-0057 = ["chia-sdk-types/chip-0057"]` (cascade only, no code yet)
- `crates/chia-sdk-utils/Cargo.toml:17-18` — `[features] chip-0057 = []` (empty, awaiting Phase 2 code)

**Phase 2 modifies `crates/chia-sdk-utils/Cargo.toml`:**

```toml
[features]
chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]

[dependencies]
# ... existing deps ...
chia-sdk-types = { workspace = true, optional = true }
bip39          = { workspace = true, optional = true }
chia-bls       = { workspace = true, optional = true }
```

The three new deps are `optional = true` so the no-features build of `chia-sdk-utils` continues to compile without them. The feature line activates them only when `chip-0057` is on. This is the canonical Cargo idiom for feature-gated dependencies.

**Verification commands** (planner must include all five in the gate):

```bash
cargo build --release -p chia-sdk-utils                          # no features
cargo build --release -p chia-sdk-utils -F chip-0057             # phase-2 feature on
cargo build --release -p chia-sdk-utils --all-features           # cumulative
cargo build --release --workspace                                # no features, all crates
cargo build --release --workspace --all-features                 # cumulative, all crates
```

**Umbrella crate (`chia-wallet-sdk`)**: `Cargo.toml:75` already cascades `chip-0057` to all three target crates. No change needed at the umbrella. The umbrella's re-export (`pub use chia_sdk_utils as utils;` in `src/lib.rs:13`) does not need a `#[cfg]` because the module re-exports the whole crate; the silent-payments submodule will only be reachable when the feature is on.

**CI line**: `.github/workflows/rust.yml:65` already has `cargo build --release -p chia-sdk-types -F chip-0057`. Phase 2 ADDS one more line: `cargo build --release -p chia-sdk-utils -F chip-0057`. (Phase 1's plan 01-05 deliberately did not add this line because the feature was empty at the end of Phase 1 — adding it now is the right time.)

## 11. Public API surface

Everything below is `pub` and behind `#[cfg(feature = "chip-0057")]`.

**Module `chia_sdk_utils::silent_payments`** (newly exposed; via `chia-sdk-utils/src/lib.rs`):

```rust
// Re-exported from error.rs
pub enum SilentPaymentError {
    WrongHrp(String),
    PayloadLength(usize),
    InvalidPublicKey,
    IdentityPublicKey,
    ReservedChangeLabel,
    Bech32(chia_sdk_utils::Bech32Error),
}

// Re-exported from address.rs
pub enum SilentPaymentNetwork { Mainnet, Testnet }
impl SilentPaymentNetwork {
    pub fn hrp(self) -> &'static str;
    pub fn from_hrp(hrp: &str) -> Result<Self, SilentPaymentError>;
}

pub struct SilentPaymentAddress {
    pub scan_pk: chia_bls::PublicKey,
    pub spend_pk: chia_bls::PublicKey,
    pub network: SilentPaymentNetwork,
}
impl SilentPaymentAddress {
    pub fn new(scan_pk: PublicKey, spend_pk: PublicKey, network: SilentPaymentNetwork) -> Self;
    pub fn encode(&self) -> Result<String, SilentPaymentError>;
    pub fn decode(s: &str)  -> Result<Self,   SilentPaymentError>;
}

// Re-exported from keys.rs
pub struct SilentPaymentKeys { /* fields private */ }
impl SilentPaymentKeys {
    pub fn from_mnemonic(mnemonic: &bip39::Mnemonic) -> Self;
    pub fn from_secret_keys(scan_sk: chia_bls::SecretKey,
                            spend_sk: chia_bls::SecretKey) -> Self;
    pub fn scan_sk(&self)  -> &chia_bls::SecretKey;
    pub fn spend_sk(&self) -> &chia_bls::SecretKey;
    pub fn scan_pk(&self)  -> &chia_bls::PublicKey;
    pub fn spend_pk(&self) -> &chia_bls::PublicKey;
    pub fn unlabeled_address(&self, network: SilentPaymentNetwork) -> SilentPaymentAddress;
    pub fn labeled_address(&self, network: SilentPaymentNetwork, m: u32)
        -> Result<SilentPaymentAddress, SilentPaymentError>;
}

// Re-exported from labels.rs
pub struct LabelRegistry { /* fields private */ }
impl LabelRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, scan_sk: &chia_bls::SecretKey, m: u32);
    pub fn forward(&self, m: u32) -> Option<&chia_bls::PublicKey>;
    pub fn lookup(&self, label_pk: &chia_bls::PublicKey) -> Option<u32>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = (u32, &chia_bls::PublicKey)>;
}
impl Default for LabelRegistry { fn default() -> Self; }
```

**Re-exports through the umbrella crate** (`chia-wallet-sdk`): `chia_wallet_sdk::utils::silent_payments::*` is the path consumers will use (since `chia_wallet_sdk::utils` is `chia_sdk_utils`'s short alias from `src/lib.rs:13`). The prelude (`src/prelude.rs:32`) currently re-exports `Address`, `Bech32`, `parse_hex`, `select_coins` from `chia_sdk_utils`. Phase 2 SHOULD add a feature-gated prelude line:

```rust
#[cfg(feature = "chip-0057")]
pub use chia_sdk_utils::silent_payments::{
    LabelRegistry, SilentPaymentAddress, SilentPaymentError,
    SilentPaymentKeys, SilentPaymentNetwork,
};
```

This puts the four types in the curated `chia_wallet_sdk::prelude::*` surface when `chip-0057` is on. (Optional but consistent with how `chia_sdk_utils::Address` is exposed.)

**Hidden / private** (NOT in the public API):

- `fn generate_label(scan_sk: &SecretKey, m: u32) -> (ScalarField, PublicKey)` — internal helper, used by both `SilentPaymentKeys::labeled_address` and `LabelRegistry::register`.
- `fn derive_path(sk: &SecretKey, path: &[u32]) -> SecretKey` — internal BIP-32 path-walker, used by `from_mnemonic`.

## 12. Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in libtest); `rstest = 0.22.0` available as workspace dev-dep if parametric coverage is wanted |
| Config file | None — `cargo test --release -p chia-sdk-utils --features chip-0057` is the canonical invocation |
| Quick run command | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` (matches CI; verified in CLAUDE.md) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ADDR-01 | Mnemonic → (scan_sk, spend_sk) matches TV1 bytes | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_mnemonic_tv1_scan_sk_matches -- --exact` | Wave 0 |
| ADDR-01 | Mnemonic → spend_sk matches TV1 bytes | unit | `... from_mnemonic_tv1_spend_sk_matches --exact` | Wave 0 |
| ADDR-01 | Mnemonic → scan_pk matches TV1 bytes | unit | `... from_mnemonic_tv1_scan_pk_matches --exact` | Wave 0 |
| ADDR-01 | Mnemonic → spend_pk matches TV1 bytes | unit | `... from_mnemonic_tv1_spend_pk_matches --exact` | Wave 0 |
| ADDR-02 | TV1 unlabeled address round-trips encode → decode → encode (mainnet) | unit | `... silent_payments::address::tests::tv1_mainnet_round_trip --exact` | Wave 0 |
| ADDR-02 | TV1 unlabeled address round-trips encode → decode → encode (testnet) | unit | `... tv1_testnet_round_trip --exact` | Wave 0 |
| ADDR-02 | TV1 encoded mainnet address pins exactly to the recomputed spxch1... constant | unit | `... tv1_mainnet_encode_pinned --exact` | Wave 0 |
| ADDR-02 | TV1 encoded testnet address pins exactly to the recomputed tspxch1... constant | unit | `... tv1_testnet_encode_pinned --exact` | Wave 0 |
| ADDR-03 | TV3 label_scalar matches pinned bytes (m=1) | unit | `... silent_payments::labels::tests::tv3_label_scalar_matches --exact` | Wave 0 |
| ADDR-03 | TV3 label_pk matches pinned bytes (m=1) | unit | `... tv3_label_pk_matches --exact` | Wave 0 |
| ADDR-03 | TV3 B_m (B_spend + label_pk) matches pinned bytes | unit | `... tv3_labeled_spend_pk_matches --exact` | Wave 0 |
| ADDR-03 | TV3 labeled mainnet address pins exactly | unit | `... silent_payments::address::tests::tv3_mainnet_labeled_pinned --exact` | Wave 0 |
| ADDR-03 | TV3 labeled testnet address pins exactly | unit | `... tv3_testnet_labeled_pinned --exact` | Wave 0 |
| ADDR-03 | scan_pk is preserved across all labels (unlabeled-scan_pk == labeled-scan_pk) | unit | `... labels_preserve_scan_pk --exact` | Wave 0 |
| ADDR-04 | `register(scan_sk, m) ; lookup(forward(m).unwrap())` returns `Some(m)` | unit | `... silent_payments::labels::tests::registry_round_trip --exact` | Wave 0 |
| ADDR-04 | Multiple labels coexist: register m=1, m=2, m=3; each lookup returns the correct index | unit | `... registry_three_labels --exact` | Wave 0 |
| ADDR-04 | Unregistered label_pk returns `None` from `lookup` | unit | `... registry_lookup_missing --exact` | Wave 0 |
| ADDR-04 | `forward(m)` on unregistered m returns `None` | unit | `... registry_forward_missing --exact` | Wave 0 |
| ADDR-05 | `from_secret_keys(scan_sk, spend_sk)` produces the same unlabeled address as `from_mnemonic` | unit | `... silent_payments::keys::tests::from_secret_keys_matches_from_mnemonic --exact` | Wave 0 |
| ADDR-05 | `from_secret_keys` followed by `unlabeled_address(Mainnet).encode()` pins to the same TV1 mainnet string | unit | `... from_secret_keys_tv1_mainnet_pinned --exact` | Wave 0 |
| ADDR-06 | `labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)` | unit | `... silent_payments::keys::tests::labeled_address_zero_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | `decode("xch1...")` returns `Err(SilentPaymentError::WrongHrp("xch"))` | unit | `... silent_payments::address::tests::decode_xch_hrp_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | `decode("spxch1...")` with a deliberately-corrupted final character returns `Err(SilentPaymentError::Bech32(_))` (invalid checksum) | unit | `... decode_invalid_checksum_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | `decode` of a bech32-NOT-bech32m string returns `Err(SilentPaymentError::Bech32(Bech32Error::InvalidFormat))` | unit | `... decode_bech32_not_bech32m_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | A bech32m-encoded `spxch1...` string with payload < 96 bytes returns `Err(SilentPaymentError::PayloadLength(N))` | unit | `... decode_short_payload_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | A bech32m-encoded `spxch1...` string with the scan-pubkey-half = all-zero (identity) returns `Err(SilentPaymentError::IdentityPublicKey)` | unit | `... decode_identity_scan_pk_rejected --exact` | Wave 0 |
| ADDR-02 (neg) | Same as above for spend-pubkey-half | unit | `... decode_identity_spend_pk_rejected --exact` | Wave 0 |

**Property-test candidates (optional, low-cost)**:

- Round-trip property: `for any (scan_pk, spend_pk, network), decode(encode(addr)) == addr`. With `rstest`, this can be a `#[rstest] #[case(...)]` parameterised over a handful of random + the TV1 + TV3 cases. Recommend including as one `roundtrip_property` test seeded over a fixed set of 8-10 cases — no real need to bring in `proptest` (not a workspace dep).
- Registry round-trip: `for m ∈ [1, 2, 100, 4_294_967_294], lookup(forward(m)) == Some(m)`. Single parameterised `#[rstest]`.

### Sampling Rate

- **Per task commit**: `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` (the quick run command). Local; under 5 seconds incremental.
- **Per wave merge**: `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` PLUS `cargo build --release --workspace --all-features` PLUS `cargo build --release --workspace` PLUS `cargo build --release -p chia-sdk-utils -F chip-0057` PLUS `cargo build --release -p chia-sdk-utils` (5 builds, mirroring Phase 1's plan 01-05 gate sweep).
- **Phase gate**: Full suite green AND the five build permutations AND `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean AND `cargo machete` clean AND `grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` returns zero hits AND `grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` returns zero hits.

### Wave 0 Gaps

- [ ] `crates/chia-sdk-utils/src/silent_payments/mod.rs` — module barrel + doc-comment, declared in `lib.rs`. Mirrors `chia-sdk-types/src/silent_payments/mod.rs` shape.
- [ ] `crates/chia-sdk-utils/src/silent_payments/error.rs` — `SilentPaymentError` enum.
- [ ] `crates/chia-sdk-utils/src/silent_payments/address.rs` — `SilentPaymentAddress` + `SilentPaymentNetwork` + all decode/encode tests (positive + negative).
- [ ] `crates/chia-sdk-utils/src/silent_payments/keys.rs` — `SilentPaymentKeys` + `from_mnemonic` / `from_secret_keys` / `labeled_address` tests (positive + m=0 rejection).
- [ ] `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `LabelRegistry` + label-scalar / label_pk / B_m pinned tests + registry round-trip tests. (May be folded into `keys.rs` if small.)
- [ ] `crates/chia-sdk-utils/Cargo.toml` — `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]`, three new `optional = true` deps.
- [ ] `crates/chia-sdk-utils/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`.
- [ ] `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-utils -F chip-0057` line near the existing per-crate chip lines.
- [ ] (Optional, recommended) `src/prelude.rs` — feature-gated re-export block of the four public Phase-2 types.

## 13. Pitfalls / gotchas

### Pitfall 1: bech32 0.9 vs 0.11 API mismatch (HIGH)

The workspace pins `bech32 = 0.9.1`. The 0.11 release (2024) refactored the entire API: removed `Variant`, replaced the `(String, Vec<u5>, Variant)` return tuple with an iterator, and introduced explicit `Hrp` / `Checksum` types. **Do NOT search docs.rs or general AI training data for "rust bech32 0.11"** — you will write code that does not compile. Use only 0.9.1 patterns:

```rust
// CORRECT (0.9.1):
bech32::encode(&hrp, data, Variant::Bech32m)?
let (hrp, data, variant) = bech32::decode(&s)?;

// WRONG (0.11.x):
bech32::encode::<Bech32m>(hrp, &data)?            // does not exist in 0.9
bech32::decode_checksummed(s)?                    // does not exist in 0.9
```

Easier: don't call `bech32::*` directly — call `chia_sdk_utils::Bech32::{encode, decode}`, which already wraps the right 0.9 surface and is version-pinned alongside the workspace.

### Pitfall 2: Compressed pubkey serialization across crates (HIGH)

CHIP-0057 uses **BLS12-381 G1** (via `chia-bls`), NOT SECP256k1. Verify:

- `~/silent-payments/crates/sp-common/Cargo.toml:7` — `chia-bls = "0.41"` (the reference impl uses chia-bls; our SDK uses 0.36.1 but the type is the same).
- `chia-bls::PublicKey` is a 48-byte compressed G1 element. `PublicKey::to_bytes() -> [u8; 48]`. Verified at `chia-sdk-bindings/src/bls.rs:135-137`.
- The CHIP §174 explicitly states "BLS12-381 G1" for the silent payment keys, distinct from BIP-352's secp256k1.

This is unusual — BIP-352 uses secp256k1 + x-only pubkeys (32 bytes); CHIP-0057 uses BLS12-381 G1 + 48-byte compressed pubkeys. The payload size difference (66 bytes for BIP-352 vs 96 bytes for CHIP-0057) drives the longer encoded-string length. Do NOT cross-reference BIP-352 implementations for the pubkey format — they use a different curve.

`chia-secp` (the SDK's SECP256k1/r1 crate) and `k256` are NOT used for silent payments. Only `chia-bls` is used.

### Pitfall 3: `chia_bls::PublicKey + chia_bls::PublicKey` ergonomics (MEDIUM)

`chia-bls::PublicKey` implements `Add<&PublicKey>` for `PublicKey` (consumes self). The reference Rust impl at `~/silent-payments/crates/sp-common/src/protocol.rs:28` does `spend_pk + &tweak_pk` — this consumes `spend_pk`. To keep the cached `self.spend_pk` field intact, Phase 2's `labeled_address` must use `&self.spend_pk + &label_pk`:

```rust
// chia-bls 0.36.1 has impl Add<&PublicKey> for &PublicKey (returns PublicKey)
let labeled_spend_pk = &self.spend_pk + &label_pk;
```

If a clippy warning fires (`needless_borrow`), explicit `Add::add(&self.spend_pk, &label_pk)` is a fallback. Verify against the workspace clippy gate before committing.

### Pitfall 4: bech32 90-char "soft limit" myth (LOW)

A common false belief is that bech32 strings cannot exceed 90 characters. BIP-173 RECOMMENDS this limit but does NOT enforce it; bech32m (BIP-350) explicitly relaxes it. The `bech32 = 0.9.1` crate (verified at source `bech32-0.9.1/src/lib.rs:367-369`) enforces HRP `1..=83` and data `>= 6` only. The 96-byte payload → ~155-char address is well within the crate's actual limits. Don't preemptively add a length cap.

### Pitfall 5: Identity-element rejection on decode (HIGH)

The CHIP §215 mandates rejection of identity-element pubkeys (`B_scan = 0 * G` or `B_spend = 0 * G`). `chia_bls::PublicKey::default()` IS the identity element (verified at `chia-sdk-bindings/src/bls.rs:103`). `chia_bls::PublicKey::is_inf()` tests for it (verified at `bls.rs:148`).

**Use `pk.is_inf()` (NOT `pk == PublicKey::default()`)** — equality on `PublicKey` may be byte-equality on serialized form, which is more brittle than the dedicated check. `is_inf()` is the canonical predicate.

`chia_bls::PublicKey::from_bytes(&[0; 48])` does NOT necessarily return an error: the all-zero-bytes encoding may be interpreted as a valid encoding of the identity point depending on `chia-bls`'s internal flag convention. **The defensive ordering is**: parse bytes → check `is_inf()` → reject. Do NOT rely on `from_bytes` failing on identity.

### Pitfall 6: Label scalar `m = 0` rejection scope (MEDIUM)

`m = 0` is rejected ONLY at the public-API boundary (`SilentPaymentKeys::labeled_address`). The internal `generate_label` helper and `LabelRegistry::register` MUST accept `m = 0` — Phase 6's labeled E2E test (SIM-03 sub-test "m=0 change-detection variant") requires the scanner to register the change label internally to detect its own change outputs. The change-label is a real label; it just must never appear in a publicly-shared address. Document this distinction with a comment on both `generate_label` and `LabelRegistry::register`.

### Pitfall 7: `SilentPaymentAddress::PartialEq` semantics (LOW)

If a wallet calls `addr1 == addr2` on two `SilentPaymentAddress` instances, the field-wise comparison via `#[derive(PartialEq)]` only compares (scan_pk, spend_pk, network). This is correct — two addresses with the same pubkeys but different network HRPs should NOT be equal (a mainnet address is a different identity than its testnet counterpart). Just be aware: the derived `PartialEq` does the right thing.

### Pitfall 8: `chia-bls 0.36.1 SecretKey::from_bytes` validates `< r` (LOW)

`chia_bls::SecretKey::from_bytes(&label_scalar.as_bytes())` returns `Result<SecretKey, _>`. The internal check is "value < group order". Because `ScalarField::from_bytes_unsigned` already reduces mod `r`, the result is guaranteed `< r` and `from_bytes` cannot fail. Use `.expect("ScalarField::from_bytes_unsigned guarantees value < r")` with that exact message — the panic is structurally unreachable.

### Pitfall 9: Workspace `unsafe_code = "deny"` (LOW)

No `unsafe` blocks needed in Phase 2. `bytes.try_into()` is the idiomatic safe way to go from `&[u8]` to `[u8; N]`. `from_bytes` / `to_bytes` round-trips on pubkeys do not require unsafe.

### Pitfall 10: Avoiding `mod_by_group_order` literal anywhere in `silent_payments/` (Phase-1 grep ban)

Phase 1 enforces `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` returns zero hits. Phase 2 inherits the same defense-in-depth: `grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` MUST also return zero hits. Use descriptive phrases like "unsigned mod-r reduction (via ScalarField)" in doc-comments — never the literal token. The plan template should include this grep as a phase-gate verification step.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hand-roll bech32m encode/decode (`shared.py:405-505`) | Reuse `chia_sdk_utils::Bech32` | This phase | One less round of pure-bech32 testing; we inherit bech32m enforcement |
| Pass `(scan_pk_bytes, spend_pk_bytes)` tuples around | A single `SilentPaymentAddress { scan_pk, spend_pk, network }` struct | This phase | Stronger typing; network is part of the address identity |
| Two HRPs handled by `if hrp == "spxch" or hrp == "tspxch"` (Python) | Dedicated `SilentPaymentNetwork` enum | This phase | Exhaustive matching; impossible to forget a network |
| Label registry as `HashMap<[u8; 48], u32>` (Python prototype) | Dedicated `LabelRegistry` type with `forward` + `lookup` + `iter` (ADDR-04 mandates this) | This phase | Phase-3 scanner can take `&LabelRegistry` directly; no leaky abstraction |

**No deprecated approaches in scope for Phase 2.** The CHIP is a draft, not a previous version — we're implementing fresh. The Python prototype is the reference, not a precedent we're departing from.

## Open Questions

1. **Should `SilentPaymentKeys` implement `Serialize` / `Deserialize`?**
   - What we know: `chia_bls::SecretKey` and `chia_bls::PublicKey` implement `Serialize` / `Deserialize` when the `serde` feature is on (verified in root `Cargo.toml:92` — `chia-bls = { workspace = true, features = ["serde"] }`). The bindings layer would benefit from serde-roundtrip. `chia_sdk_utils::Address` does NOT derive serde.
   - What's unclear: Wallet authors may want to serialize a `SilentPaymentKeys` to disk encrypted. Or, they may want to serialize a `SilentPaymentAddress` for storage / transmission. We don't have a concrete consumer to anchor the decision.
   - Recommendation: Add `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` to `SilentPaymentAddress` and `SilentPaymentNetwork` ONLY (not `SilentPaymentKeys`). The address is a public-API artifact that wants to round-trip; keys should be handled by wallet-author-chosen at-rest encryption. NOTE: Phase 2 does not currently need to gate behind serde — defer this until Phase 5 (bindings) surfaces a concrete need. If the planner wants to play it safe, leave serde out entirely in Phase 2; it can be added in Phase 5 without breaking anyone.

2. **Should `SilentPaymentNetwork` include a `from_hrp` that's permissive about case?**
   - What we know: Bech32 forbids mixed case (`Error::MixedCase`) but allows all-lowercase or all-uppercase. The standard Chia tooling normalizes to lowercase. The existing `chia_sdk_utils::Bech32::decode` propagates `Error::MixedCase` for mixed-case input.
   - What's unclear: Whether `SILENT_PAYMENT::from_hrp("SPXCH")` should accept the uppercase HRP. Behavior on the existing `Address` type is "whatever bech32 says" — `decode("XCH1...")` decodes successfully because bech32 normalizes case internally.
   - Recommendation: Don't add case-insensitivity at the `from_hrp` boundary; rely on bech32's behavior. `Bech32::decode` returns the prefix in lowercase, so `SilentPaymentNetwork::from_hrp` sees the lowercase form already. Add a unit test (`tspxch1...` decodes; `TSPXCH1...` ALSO decodes due to bech32 case-normalization) to document the implicit behavior.

3. **Does `from_mnemonic` need an `account_index` parameter (BIP-44 style)?**
   - What we know: The CHIP fixes the path at `m/12381/8444/12/0` and `m/12381/8444/13/0` with no account dimension. The Python prototype, the partial Rust port, and Phase 1's SCAN/SPEND constants all use exactly these. The standard Chia wallet uses `m/12381/8444/2/<index>` with an index parameter, but that's for a different scheme.
   - What's unclear: Whether the SDK should preemptively support multi-account silent-payment wallets, in case the CHIP later adds an index dimension.
   - Recommendation: Don't add `account_index`. The CHIP doesn't have one; adding a parameter that's always passed `0` is API debt. If the spec changes later, add `SilentPaymentKeys::from_mnemonic_with_index` then.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust 1.90.0 toolchain | Phase 2 compile | ✓ | 1.90.0 (rust-toolchain.toml) | — |
| `chia-bls` (workspace dep) | Key derivation, pubkey arithmetic | ✓ | 0.36.1 | — |
| `bech32 = 0.9.1` (workspace dep) | Address encode/decode | ✓ | 0.9.1 | — |
| `bip39 = 2.2.0` (workspace dep) | Mnemonic → seed | ✓ | 2.2.0 | — |
| `num-bigint = 0.4.6` (workspace dep) | (Transitive via ScalarField in chia-sdk-types) | ✓ | 0.4.6 | — |
| `chia-sha2 = 0.36.1` (workspace dep) | (Transitive via tagged_hash in chia-sdk-types) | ✓ | 0.36.1 | — |
| `hex-literal = 0.4.1` (workspace dep) | Test-vector pinning | ✓ | 0.4.1 | — |
| `hex = 0.4.3` (workspace dep) | Test helpers | ✓ | 0.4.3 | — |
| `thiserror = 2.0.17` (workspace dep) | `SilentPaymentError` derive | ✓ | 2.0.17 | — |
| `rstest = 0.22.0` (workspace dev-dep) | Parameterised tests (optional) | ✓ | 0.22.0 | — |
| Python 3 + the `~/silent-payments` venv | Recomputing TV1/TV3 strings (one-time, during research only) | ✓ | already used | Phase 2 code does not depend on Python at all |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None.

All required tooling and crates are already in place. Phase 2 introduces zero net-new workspace dependencies.

## Sources

### Primary (HIGH confidence)
- `~/silent-payments/chip-silent-payments.md` (CHIP-0057 spec text, sections referenced inline) — authoritative for HRP, payload layout, label math, m=0 reservation, identity-element rejection, test-vector hex values
- `~/silent-payments/shared.py` (Python reference impl) lines 268-505 — authoritative for the bech32m encoding scheme and the byte-exact `encode_silent_payment_address` / `decode_silent_payment_address` / `generate_label` semantics
- `~/silent-payments/crates/sp-common/src/keys.rs` and `protocol.rs` — Rust-port reference for `SilentPaymentKeys` shape and label generation
- `crates/chia-sdk-types/src/silent_payments/{paths,scalar,tagged_hash}.rs` (Phase 1) — pinned constants we consume
- `crates/chia-sdk-utils/src/bech32.rs` (existing) — the `Address` / `Bech32` precedent we extend
- `crates/chia-sdk-bindings/src/{bls,mnemonic,address}.rs` — existing API patterns for chia-bls and bip39 usage
- Phase 1 `01-PHASE-SUMMARY.md` + `01-VERIFICATION.md` — locked Phase 1 outputs and follow-ups
- `bech32-0.9.1/src/lib.rs` (read directly from `~/.cargo/registry/src/...`) — authoritative for the 0.9.1 API surface and length-limit behavior

### Secondary (MEDIUM confidence)
- BIP-352 spec text (only consulted for general silent-payment vocabulary; the CHIP departs from BIP-352 on curve and key serialization, so cross-references are scoped to vocabulary)
- `docs.rs/bech32/0.9.1` Error variant listing (cross-checked against in-repo source)

### Tertiary (LOW confidence)
- None — every claim in this document is grounded in either a Phase-1 artifact, a Phase-2 directly-readable file, or a locally-recomputed test-vector value.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new deps; every dep is pinned in `[workspace.dependencies]`.
- Architecture: HIGH — directly extends an existing pattern (`chia_sdk_utils::Address`) in the same crate; Phase 1 left a feature-flag scaffold deliberately empty for this.
- Pitfalls: HIGH — every pitfall is either grounded in source-confirmed library behavior or in Phase 1's already-enforced grep bans.
- Test vectors: HIGH — TV1 / TV3 hex values are quoted from the CHIP spec; `tspxch1...` / `spxch1...` strings are recomputed locally from the Python prototype and reproduced verbatim above. The verifier can re-run the recomputation command and match byte-for-byte.

**Research date:** 2026-05-15
**Valid until:** ~2026-06-15 (stable domain — BIP-39, chia-bls, bech32 0.9, and the CHIP draft are all stable on a 30-day horizon). If the CHIP draft changes (re-numbered paths, HRP rename, payload reordering), revisit immediately.

## RESEARCH COMPLETE

Phase 2 sits cleanly on top of Phase 1's primitives and the existing `chia-sdk-utils::Address` / `Bech32` pattern. The implementation surface is well-bounded: one new submodule (`crates/chia-sdk-utils/src/silent_payments/`), three new optional dependencies activated by the existing `chip-0057` feature, four public types (`SilentPaymentKeys`, `SilentPaymentAddress`, `SilentPaymentNetwork`, `LabelRegistry`) plus a `SilentPaymentError` enum. All seven CHIP-0057 success-criteria-aligned behaviors (mnemonic derivation, watch-only construction, scan/spend pubkey accessors, unlabeled address encode/decode, labeled address encode/decode, m=0 rejection, label registry round-trip) map to 26 named unit tests grounded in CHIP TV1 / TV3 byte values, with the canonical `spxch1...` / `tspxch1...` strings recomputed via the Python reference implementation and reproduced verbatim in §8 for the planner to paste straight into test files. The Q8 pre-flight question (whether `chia-sdk-utils` needs a `dep:chia-sdk-types` edge) resolves YES, gated `optional = true` behind the same `chip-0057` feature, so the no-features build of `chia-sdk-utils` continues to compile cleanly. No new workspace dependencies, no `unsafe` blocks, no nightly features, no chip-feature-aliases — Phase 2 should fit in a similar plan-count as Phase 1 (4-5 plans: feature wiring, `SilentPaymentError` + `SilentPaymentNetwork`, `SilentPaymentAddress` + decode/encode tests, `SilentPaymentKeys` + `LabelRegistry` + label tests, final gate verification).
