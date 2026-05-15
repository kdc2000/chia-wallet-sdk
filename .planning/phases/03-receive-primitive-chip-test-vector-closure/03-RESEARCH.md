# Phase 3: Receive primitive & CHIP test-vector closure — Research

**Researched:** 2026-05-15
**Domain:** Transport-agnostic silent-payment scanner (BLS12-381 ECDH on a pre-computed tweak point; k-iteration with labeled-detection branch and `K_max` DOS guard; CHIP test-vector closure)
**Confidence:** HIGH — every recommendation in this document is anchored to (a) Phase 1's shipped primitives, which I read end-to-end (`crates/chia-sdk-types/src/silent_payments/{scalar,tagged_hash,paths,mod}.rs`); (b) Phase 2's shipped types, which I read end-to-end (`crates/chia-sdk-utils/src/silent_payments/{mod,address,keys,labels,error}.rs`); (c) the reference scanner at `~/silent-payments/crates/sp-client/src/scanner.rs` and supporting primitives in `~/silent-payments/crates/sp-common/src/{protocol,ecdh,puzzle}.rs`; (d) the four canonical CHIP test vectors in `~/silent-payments/tests/test_vectors.py` (byte-for-byte values reproduced below); (e) CHIP-0057 §"Edge Cases" and §"K_max" verbatim. Open uncertainties are flagged inline and never as HIGH-confidence claims.

## Summary

Phase 3 ships the wallet-side receive primitive in `chia-sdk-driver` (NOT `chia-sdk-utils` — the scanner needs `chia-puzzle-types::standard::StandardArgs::curry_tree_hash` + `DeriveSynthetic`, which already live as non-optional deps of `chia-sdk-driver`, and the SDK convention is that primitives that produce a `puzzle_hash` from a `PublicKey` live in driver, not utils). The design lands four artefacts:

1. **Three small protocol primitives** that the scanner reuses and that Phases 4/6 will also reuse: `compute_shared_secret_from_tweak`, `derive_output_tweak`, and a `puzzle_hash_for_pk` helper. All sit in `crates/chia-sdk-driver/src/silent_payments/` behind `chip-0057`. They mirror the byte-exact behaviour of `sp-common::{ecdh,protocol,puzzle}` but use `chia_sdk_types::silent_payments` (not `sp_common`) and `chia_sha2::Sha256` (not bare `sha2`).
2. **Two transport-agnostic wire types**: `TweakData { tweak_points: Vec<PublicKey>, outputs: Vec<OutputMeta> }` and `OutputMeta { puzzle_hash: Bytes32, coin_id: Bytes32, amount: u64, parent_coin_id: Bytes32 }`. Plus the detected-coin output type `DetectedSpCoin { coin_id, puzzle_hash, amount, parent_coin_id, onetime_sk, k, label: Option<u32> }`. All three are Bytes32 + Vec<PublicKey> + scalar shapes that `bindings/silent_payments.json` can express in Phase 5 with no schema surgery.
3. **The scanner itself**: `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, Option<&LabelRegistry>, K_max) -> Vec<DetectedSpCoin>`. Implements the BIP-352 k-iteration with the labeled-termination rule from `sp-client/scanner.rs:74-135`, plus two CHIP-mandated guards the reference impl is missing: (a) skip a tweak point that is the identity element (CHIP §459 — DOS / privacy critical); (b) cap k at `K_max` per spend group (CHIP §416). Errors propagate via a new `DriverError::SilentPayment(SilentPaymentError)` variant added once.
4. **Eleven named unit tests** that close CRYPTO-03 and the five RECV-* requirements: four pinning the canonical CHIP test vectors (TV1 + TV3 + TV4 + a derived TV3-style labeled scan), a bespoke `k=1` vector (catches `ser32(k)` endianness bugs — TV1/TV3/TV4 all hit `k=0`), an adversarial `[0xff;32]` scalar test (verifies the `ScalarField` boundary actually fires through the protocol), an identity-`tweak_point` test, a malformed-pubkey-bytes test (caught at deserialization, not in the scanner — the scanner takes `PublicKey`, not bytes), a 10,000-forged-match DOS-guard test, and a labeled k-termination test.

**Primary recommendation:** Land the scanner in `crates/chia-sdk-driver/src/silent_payments/{mod,protocol,scanner,types,error}.rs` behind `chip-0057`. Define `TweakData`, `OutputMeta`, `DetectedSpCoin` as plain `#[derive(Clone, Debug, ...)]` structs with `pub` fields (matching `chia-protocol::Coin` style, easy to bind). Implement `scan_from_tweaks` exactly per the reference scanner's branching logic, with two CHIP-spec additions (`is_inf()` skip + `K_max` cap). Use `K_MAX_DEFAULT: usize = 2400` from CHIP §446 (NOT 32 — the `<additional_context>` value of 32 contradicts the CHIP and is too tight for legitimate multi-output silent-payment batches). All eleven tests pin byte-exact values from `~/silent-payments/tests/test_vectors.py`. Total estimated surface: ~600 lines of source + ~400 lines of tests + 1 new `DriverError` variant + 1 new CI build line (`-p chia-sdk-driver -F chip-0057`).

<user_constraints>
## User Constraints (from CONTEXT.md)

There is no CONTEXT.md for Phase 3 (no `/gsd:discuss-phase` has been run yet — `config.json` has `discuss_mode: "discuss"` but `skip_discuss: false`, and `STATE.md` reports `stopped_at: ...Ready for Phase 3 (receive primitive)` with the receive primitive not yet started). Constraints therefore come from `./CLAUDE.md`, `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, and `.planning/ROADMAP.md`. If a `/gsd:discuss-phase` is run before this planner consumes the research, copy the result here verbatim and treat anything below as superseded by the locked decisions.

### Locked Decisions (from PROJECT.md, REQUIREMENTS.md, ROADMAP.md)

- **Feature flag:** `chip-0057` is the only umbrella. No `silent-payments`, no name-aliases, no sub-features within a feature.
- **No new workspace dependencies.** Available already: `chia-bls = 0.36.1`, `chia-puzzle-types = 0.36.1`, `chia-sha2 = 0.36.1`, `clvm-utils = 0.36.1`, `chia-protocol = 0.36.1`, `num-bigint = 0.4.6`, `hex = 0.4.3`, `hex-literal = 0.4.1`, `thiserror = 2.0.17`, `bip39 = 2.2.0`. (Bumping `chia-protocol` or `chia-puzzles` is permanently out of scope.)
- **No new top-level crate.** Phase 3 code lives in `chia-sdk-driver/src/silent_payments/` behind `chip-0057`, mirroring how `chip-0035`/`chip-0037`/`action-layer` slot into the driver crate today.
- **`ScalarField` is the only mod-r reducer.** Phase 1 grep ban (`! grep -r 'mod_by_group_order' silent_payments/`) extends to every file Phase 3 ships. No `From<[u8;32]>` impl on `ScalarField`. Output-tweak scalar uses `ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_SHARED_SECRET, ...))`.
- **`chia-sha2`, never bare `sha2`.** Phase 1 defense-in-depth grep ban (`! grep -rE '^use sha2::' silent_payments/`) holds. The reference impl at `~/silent-payments/crates/sp-client/src/scanner.rs:11` uses `sha2::{Sha256, Digest}` — this is the LINE that must change when porting.
- **Workspace lint policy (WS-03):** every chip-0057-gated file passes `deny clippy::all`, `warn pedantic`, `warn cargo`, `deny unsafe_code`, `deny dead_code`, plus `cargo machete` with zero new `[package.metadata.cargo-machete] ignored` entries.
- **Transport-agnostic forward compat with CHIP-0058.** `TweakData` has no transport fields — no `height`, no JSON envelope, no `sp-service` URL. Phase 3 owns the definition; Phase 5/6 carry the constraint.
- **`K_max` is a per-spend-group cap, configurable, default per CHIP §446.** CHIP §416 mandates the cap; CHIP §446 sets `K_max = 2400` for Chia (BIP-352 uses 2323). The receive primitive accepts this as a parameter or struct field — picking a sensible default with `K_MAX_DEFAULT: usize = 2400` is the recommendation.
- **Identity-element guards (CHIP §459).** Scanner MUST skip any tweak point that is `point at infinity` (serialize == `0xc000...00`). Detected via `PublicKey::is_inf()`. This is NOT in the reference scanner; this is the SDK's contribution.
- **m=0 change-label discipline (Phase 2, ADDR-06):** the public `SilentPaymentKeys::labeled_address` rejects `m=0`, but `LabelRegistry::register(scan_sk, 0)` is intentionally accepted (Phase 2 design choice). The scanner thus correctly detects change outputs when the wallet has registered `m=0` against itself — Phase 3 simply uses `LabelRegistry::iter()` and the rejection lives at the higher boundary.
- **`puzzle_hash_for_pk` is NOT hand-rolled.** Reuse `chia_puzzle_types::standard::StandardArgs::curry_tree_hash(pk.derive_synthetic())`, matching `crates/chia-sdk-driver/src/layers/standard_layer.rs:118`. The reference's `sp-common/puzzle.rs:13-17` is the exact algorithm.

### Claude's Discretion (planner can choose)

- **Module file layout inside `crates/chia-sdk-driver/src/silent_payments/`.** Recommended split is `mod.rs` (barrel + crate docs), `protocol.rs` (`derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`, `compute_shared_secret_from_tweak`), `scanner.rs` (`scan_from_tweaks` + `K_MAX_DEFAULT`), `types.rs` (`TweakData`, `OutputMeta`, `DetectedSpCoin`). The planner may collapse `types.rs` into `scanner.rs` if it stays under 300 lines; the split is cosmetic.
- **`K_max` API surface.** Options: (a) free parameter `k_max: usize`, (b) `ScanOptions { k_max: usize, … }` struct, (c) `TweakData::k_max: usize` field with `Default` impl. Recommend (a) for Phase 3 (small surface, easy to bind) and a constructor on `TweakData` if a struct is needed later. The `ScanOptions` approach (b) is future-proof but adds binding-descriptor surface area in Phase 5; defer until Phase 5 actually needs more options.
- **Whether `scan_from_tweaks` returns `Vec<DetectedSpCoin>` or `Result<Vec<DetectedSpCoin>, DriverError>`.** Recommend `Vec` (no error paths in the scan itself — identity-element guard skips silently per CHIP §459; malformed input is caught at deserialization before reaching the scanner). If the planner wants a result type for future expansion, fine — but it isn't required by current behaviour.
- **Whether to expose `derive_onetime_pk` and `derive_onetime_sk` publicly.** Phase 4 will need both for the send-side construction (`derive_one_time_puzzle_hash`). Phase 3 should ship them `pub` so Phase 4 doesn't have to relocate them. Keep `derive_output_tweak` `pub` for the same reason — Phase 4 needs it.
- **Whether `LabelRegistry::iter()` returns `(u32, &PublicKey)` or borrowed `(&u32, &PublicKey)`.** Phase 2 ships `(u32, &PublicKey)`. Phase 3's scanner uses this verbatim; no change needed.
- **Test mnemonic constants.** Phase 2 ships `TV1_MNEMONIC` as a `const &str` in `keys.rs::tests`. Phase 3 can duplicate that string or extract a `pub(crate) const` test helper. Recommend duplication — keeping each `tests::` module self-contained is the SDK style (see `chia-sdk-driver/src/actions/send.rs:148` vs `:280` for duplicated `bob.pk` setup).

### Deferred Ideas (OUT OF SCOPE for Phase 3)

- **`bindings/silent_payments.json` descriptor.** Phase 5 (BIND-01, BIND-02). Phase 3 designs `TweakData`/`DetectedSpCoin`/`OutputMeta` with the binding constraint in mind ("Bytes32 + lists of (PublicKey, OutputMeta) is the granularity") but does not author the JSON.
- **napi/pyo3/wasm builds.** Phase 5 (BIND-02). Phase 3 only verifies `cargo build -p chia-sdk-driver -F chip-0057` and the workspace `--all-features` build.
- **Simulator integration helper `tweak_data_from_simulator_block`.** Phase 6 (SIM-01). Phase 3's scanner takes `TweakData` from any source; building one from a `Simulator` block is Phase 6.
- **`SilentPaymentSend` action + `Spends` integration.** Phase 4 (SEND-04). Phase 3 ships the protocol-level helpers (`derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`); Phase 4 wires them into the action system.
- **`compute_input_hash` + `aggregate_sender_sks`.** Phase 4 (SEND-02, SEND-03). The scanner does NOT need these — it receives `tweak_point` already computed (`tweak_point = input_hash * A_sum` happens on the indexer/sender side). This is the entire point of the transport-agnostic design.
- **CHIP-0058 transport client.** v2 deferred per REQUIREMENTS.md "v2 Requirements (deferred)".
- **Pubkey-deserialization-failure path in the scanner.** The scanner takes `PublicKey`, not `[u8; 48]`. Malformed bytes are caught by `PublicKey::from_bytes` upstream (in whoever constructs the `TweakData`). Phase 3's adversarial test (Q4 in success criteria) covers this by asserting the deserialization failure happens at the right boundary — but the scanner itself never panics on a malformed pubkey because it never sees raw bytes.

## Project Constraints (from CLAUDE.md)

Actionable directives the planner must honor:

1. **Rust 1.90.0, edition 2024** (`rust-toolchain.toml`). No nightly features.
2. **`unsafe_code = "deny"`.** No `unsafe` blocks anywhere in Phase 3 source.
3. **Workspace clippy:** `deny clippy::all` + `warn pedantic` + `warn cargo`. Local strict gate: `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`. Inline fixes only — no `#[allow(...)]` attributes anywhere in `silent_payments/`.
4. **`cargo machete` clean.** Zero new `[package.metadata.cargo-machete] ignored` entries on `chia-sdk-driver`.
5. **`cargo fmt --check`** clean.
6. **`[lints] workspace = true`** in `chia-sdk-driver/Cargo.toml` (line 18 — already set). Do not add a per-crate override.
7. **Every dep is `{ workspace = true }`**, never a literal version. `chia-puzzle-types`, `chia-bls`, `chia-sha2`, `clvm-utils`, `num-bigint`, `hex`, `hex-literal`, `thiserror` are already listed in `chia-sdk-driver/Cargo.toml` and need no new entries.
8. **`chia-sha2`, never bare `sha2`.** Phase 1 grep ban. The reference impl violates this (`sp-client/scanner.rs:11`, `sp-common/ecdh.rs:4`) — these are the lines that change when porting.
9. **No `From<[u8;32]> for ScalarField`.** Already enforced by Phase 1.
10. **No `mod_by_group_order` literal** anywhere in `chip-0057`-gated code. Phase 1 grep ban.
11. **No `unwrap()` on user-data deserialization paths.** Scanner internals may `.expect("ScalarField::from_bytes_unsigned guarantees value < r")` since that's an invariant violation, but pubkey-byte parsing in test setup uses `.expect("test vector pubkey")`.
12. **GSD workflow enforcement** — all edits go through a GSD command. Phase 3 is invoked via `/gsd:plan-phase` → spawns gsd-planner (which consumes this research).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **RECV-01** | `TweakData { tweak_points, outputs }` is the transport-agnostic input type for the scanner. Decoupled from any wire format so a CHIP-0058 transport client (or today's `sp-service` adapter) can construct it without breaking the SDK API. | §3 (type design), §4 (binding granularity), §5 (CHIP-0058 forward compat). Two `Vec` fields, no transport fields. Mirrors `sp-client/scanner.rs:16-22` (`OutputMeta`) and the implicit `Vec<PublicKey>` input that `scan_block` consumes (`scanner.rs:62-65`). |
| **RECV-02** | `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels) -> Vec<DetectedSpCoin>` returns `(coin, onetime_sk, k, label: Option<u32>)` per detection. Implements BIP-352 k-iteration with labeled-termination rule. | §3 (`DetectedSpCoin` shape), §6 (k-loop algorithm verbatim from `sp-client/scanner.rs:73-135`), §7 (labeled-termination rule), §10 (TV1/TV3/TV4 vectors). Function signature in §3 line 18. |
| **RECV-03** | `compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` is the cheap wallet-side ECDH primitive (one scalar-multiply + SHA-256). Used internally by `scan_from_tweaks` AND exposed publicly. | §3 (signature), §6 (algorithm `SHA256(scan_sk * tweak_point)`), §10 line "TV1 shared_secret". Pinned value `d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6` from `~/silent-payments/crates/sp-client/src/scanner.rs:200-204`. |
| **RECV-04** | Labeled detection: when an unlabeled candidate at `k` misses, the scanner tries each registered `label_pk`. Termination rule: break the `k` loop ONLY when neither unlabeled NOR any labeled candidate matches at the current `k`. | §7 (rule), §8 (`LabelRegistry::iter()` usage), §10 line "TV3 labeled scan", §11 test "labeled_k_termination_rule". Algorithm: `sp-client/scanner.rs:94-135` plus the SDK's `LabelRegistry` lookup-by-iter approach. |
| **RECV-05** | A `K_max` per-spend-group iteration cap (configurable, default ~2400 per CHIP §446) prevents DOS by an adversarial tweak source. | §6 ("K_max guard" subsection), §9 (CHIP §446 quote — 2400 not 32), §11 test "dos_guard_caps_at_k_max". Cap fires after `K_max` iterations regardless of "found" status, not after `K_max` consecutive misses. |
| **CRYPTO-03** | All CHIP test vectors pass: TV1 (unlabeled single-input), TV3 (labeled single-input), TV4 (multi-input aggregation). Plus a bespoke `k=1` vector (catches `ser32(k)` endianness bugs that TV1/TV3/TV4 — all `k=0` — cannot detect). Plus an adversarial `[0xff;32]` scalar test that fails on signed reduction but passes on unsigned reduction (verifies the `ScalarField` boundary fires through the full protocol). | §10 (all four canonical TVs reproduced byte-for-byte), §11 (11-test plan), §12 (validation map). The `k=1` test uses TV1 inputs + `derive_output_tweak(shared_secret, 1)` and pins the (currently unpublished, must be locally computed) k=1 one-time PK + puzzle hash. The adversarial `[0xff;32]` test feeds `tweak_data` whose shared secret reduces to a known-different value under unsigned vs signed interpretation. |
</phase_requirements>

## 1. Where the types live

**Decision: `crates/chia-sdk-driver/src/silent_payments/` (NOT `chia-sdk-utils`, NOT `chia-sdk-types`).**

Rationale:

- The scanner needs `chia_puzzle_types::standard::StandardArgs::curry_tree_hash` and `chia_puzzle_types::DeriveSynthetic` to convert a one-time PK to a puzzle hash. Both are already non-optional deps of `chia-sdk-driver` (`Cargo.toml` lines ~33-37). Adding them to `chia-sdk-utils` would mean a new edge and a per-crate-feature cascade test like the Q8 Phase 2 audit.
- Driver-crate primitives that compose `Layer`s and produce coin spends are the SDK's "wire-level transaction" surface. `SilentPaymentSend` (Phase 4) and `scan_from_tweaks` (Phase 3) are both transaction-side, not address-side.
- The CHIP-0035 (datalayer / vault) and `action-layer` precedents both live in `chia-sdk-driver/src/primitives/`. Phase 3's scanner is analogous: a "primitive operation on a coin" composed of layered primitives.
- Phase 2's `SilentPaymentKeys`, `SilentPaymentAddress`, `LabelRegistry`, `SilentPaymentError`, `SilentPaymentNetwork` stay in `chia-sdk-utils` (CLAUDE.md: "wallet API goes in the silent-payments crate Phase 1 established" — utils is the wallet-API home). The driver code consumes them via `use chia_sdk_utils::silent_payments::{SilentPaymentKeys, LabelRegistry}`.
- The umbrella prelude is already laid out for this — Phase 2's prelude block (`src/prelude.rs:34-38`) takes `chip-0057`-gated re-exports; Phase 3 simply appends to that block.

**Module file layout** (recommended; planner may collapse):

```
crates/chia-sdk-driver/src/silent_payments/
├── mod.rs            # barrel + crate docs; pub use {protocol::*, scanner::*, types::*}
├── protocol.rs       # derive_output_tweak, derive_onetime_pk, derive_onetime_sk,
│                     #  puzzle_hash_for_pk, compute_shared_secret_from_tweak
├── scanner.rs        # scan_from_tweaks, K_MAX_DEFAULT
└── types.rs          # TweakData, OutputMeta, DetectedSpCoin
```

`crates/chia-sdk-driver/src/lib.rs` gets a new `#[cfg(feature = "chip-0057")] pub mod silent_payments;` declaration mirroring how `chia-sdk-types/src/lib.rs` does it.

**`DriverError` extension.** Phase 3 adds ONE variant to `crates/chia-sdk-driver/src/driver_error.rs`:

```rust
#[cfg(feature = "chip-0057")]
#[error("silent payment error: {0}")]
SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError),
```

Even though `scan_from_tweaks` itself returns `Vec<DetectedSpCoin>` (no error path — see §3 below), the new variant exists for Phase 4's `SilentPaymentSend` and for any caller that wants to thread silent-payment errors through `DriverError`. Land it in Phase 3 to avoid touching `driver_error.rs` twice.

## 2. Phase 1 + 2 inheritance

What Phase 3 imports (resolved paths, confirmed by `Read` of the actual files):

```rust
use chia_sdk_types::silent_payments::{
    CHIA_SP_SHARED_SECRET,   // tagged_hash domain tag for output tweak (Phase 1)
    ScalarField,             // unsigned mod-r scalar newtype (Phase 1)
    tagged_hash,             // BIP-340 tagged hash (Phase 1)
};
use chia_sdk_utils::silent_payments::LabelRegistry;  // bidirectional u32 ↔ PublicKey (Phase 2)
```

Phase 1's `mod_by_group_order` grep ban (`! grep -r 'mod_by_group_order' silent_payments/`) and `sha2::` grep ban (`! grep -rE '^use sha2::' silent_payments/`) extend to every file Phase 3 ships. Already verified Phase 2 honors both (`02-PHASE-SUMMARY.md` lines 127-128).

Phase 2 also already added the umbrella prelude block at `src/prelude.rs:34-38`; Phase 3 extends that block:

```rust
#[cfg(feature = "chip-0057")]
pub use chia_sdk_driver::silent_payments::{
    DetectedSpCoin, OutputMeta, TweakData,
    compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk,
    derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks, K_MAX_DEFAULT,
};
```

## 3. The wire-protocol type surface

These are the four user-visible types and one constant. Every field is sized so it round-trips cleanly through `bindings/silent_payments.json` (Phase 5) with `bindy-macro`'s existing type-group mappings (`bindings.json` lines 4-21: `{bytes}` covers `Bytes32`, `{bigint}` covers `u64`, `Vec<PublicKey>` is already a known shape from `bindings/bls.json:75,81`).

### 3a. `TweakData` (RECV-01)

```rust
/// Transport-agnostic input to the silent-payment scanner.
///
/// `tweak_points` are the pre-computed per-spend-group ECDH multipliers
/// `tweak_point[i] = input_hash[i] * A_sum[i]` (the indexer or sender computes these).
/// `outputs` are the candidate coin metadata to scan against — typically all
/// outputs in a single Chia block, but the primitive does not care about block
/// boundaries.
///
/// No transport fields (no `height`, no `block_hash`, no JSON envelope). A
/// future CHIP-0058 transport client constructs `TweakData` from its wire
/// messages without breaking this struct's shape.
#[derive(Clone, Debug)]
pub struct TweakData {
    pub tweak_points: Vec<chia_bls::PublicKey>,
    pub outputs: Vec<OutputMeta>,
}
```

### 3b. `OutputMeta` (RECV-01)

```rust
/// Coin metadata the scanner needs to identify a detected silent-payment coin.
///
/// `puzzle_hash` is what the scanner matches against; the remaining fields
/// flow through to the returned `DetectedSpCoin` so the wallet can act on the
/// coin without re-parsing the block.
#[derive(Clone, Debug)]
pub struct OutputMeta {
    pub puzzle_hash: chia_protocol::Bytes32,
    pub coin_id: chia_protocol::Bytes32,
    pub amount: u64,
    pub parent_coin_id: chia_protocol::Bytes32,
}
```

NOTE: the reference scanner uses `[u8; 32]` (not `Bytes32`). The SDK convention is `chia_protocol::Bytes32` everywhere (it's `Copy`, derefs to `[u8; 32]`, and is the type the bindings JSON already maps via `{bytes}`). Use `Bytes32`.

### 3c. `DetectedSpCoin` (RECV-02)

```rust
/// A silent-payment coin detected by `scan_from_tweaks`, carrying enough
/// information for the wallet to immediately compose a follow-on spend.
#[derive(Clone, Debug)]
pub struct DetectedSpCoin {
    pub coin_id: chia_protocol::Bytes32,
    pub puzzle_hash: chia_protocol::Bytes32,
    pub amount: u64,
    pub parent_coin_id: chia_protocol::Bytes32,
    /// The one-time secret key for this output — `(b_spend + t_k) mod r` for
    /// unlabeled detections; `(b_spend + t_k + label_scalar) mod r` for labeled.
    pub onetime_sk: chia_bls::SecretKey,
    /// The k counter at which this output was detected (per spend group).
    pub k: u32,
    /// `None` for unlabeled detections; `Some(m)` for label-index `m`.
    pub label: Option<u32>,
}
```

### 3d. `scan_from_tweaks` signature (RECV-02)

```rust
/// Scan a block's worth of tweak points + outputs for silent payments addressed
/// to this wallet.
///
/// For each `tweak_point` in `data.tweak_points`, performs one ECDH operation
/// (`scan_sk * tweak_point`, hashed to a 32-byte shared secret) and iterates
/// k = 0, 1, 2, ... up to `k_max`, deriving the candidate one-time puzzle hash
/// and checking against `data.outputs`. Labeled detection (per `LabelRegistry`)
/// is interleaved per CHIP §RECV-04.
///
/// Identity-element tweak points are skipped silently per CHIP §459. The
/// `k_max` cap (CHIP §416, default `K_MAX_DEFAULT = 2400` per CHIP §446)
/// bounds the per-spend-group iteration count so an adversarial tweak source
/// cannot force unbounded scanning.
pub fn scan_from_tweaks(
    scan_sk: &chia_bls::SecretKey,
    spend_sk: &chia_bls::SecretKey,
    spend_pk: &chia_bls::PublicKey,
    data: &TweakData,
    labels: Option<&chia_sdk_utils::silent_payments::LabelRegistry>,
    k_max: usize,
) -> Vec<DetectedSpCoin>;
```

### 3e. `K_MAX_DEFAULT`

```rust
/// Default per-spend-group iteration cap per CHIP-0057 §446.
///
/// Derived from Chia's mempool 5.5B spend-bundle cost limit; 2,400 is the
/// theoretical maximum number of silent-payment outputs a single spend bundle
/// can fit at standard mempool policy. Callers may pass a smaller value to
/// `scan_from_tweaks` (e.g., 32 for a fast pre-scan in resource-constrained
/// environments) but should not exceed this in production scans.
pub const K_MAX_DEFAULT: usize = 2400;
```

The `<additional_context>` for this research mentions `K_max = 32` for the DOS-guard test. This value is FINE as a TEST input (forces the loop to terminate after 32 forged matches, much faster than running 2400 iterations) but should NOT be the production default. The CHIP-spec default is 2400. The DOS-guard test passes `k_max = 32` explicitly.

### 3f. The three protocol-helper signatures (RECV-03 + Phase 4 deps)

```rust
pub fn derive_output_tweak(shared_secret: &[u8; 32], k: u32) -> ScalarField;
pub fn derive_onetime_pk(spend_pk: &PublicKey, tweak: &ScalarField) -> PublicKey;
pub fn derive_onetime_sk(spend_sk: &SecretKey, tweak: &ScalarField) -> SecretKey;
pub fn puzzle_hash_for_pk(pk: &PublicKey) -> Bytes32;
pub fn compute_shared_secret_from_tweak(scan_sk: &SecretKey, tweak_point: &PublicKey) -> [u8; 32];
```

All five public — Phase 4 needs `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk` for send-side construction; Phase 5 exposes `compute_shared_secret_from_tweak` and `puzzle_hash_for_pk` for callers that compute manually.

## 4. Binding granularity (CHIP-0058 forward compat)

The `<additional_context>` specifies "Bytes32 + lists of (PublicKey, OutputMeta) is the granularity to design around." Let me trace exactly what `bindings/silent_payments.json` will look like in Phase 5 — this validates the type design now:

| Rust type / field | bindings.json type-group | wasm | napi | pyo3 |
|---|---|---|---|---|
| `chia_protocol::Bytes32` | `{bytes}` | `Vec<u8>` (`bindings.json:31`) | `napi::bindgen_prelude::Uint8Array` (`:26`) | `Vec<u8>` (`:51`) |
| `u64` | `{bigint}` | `js_sys::BigInt` (`:32`) | `napi::bindgen_prelude::BigInt` (`:27`) | `num_bigint::BigInt` (`:52`) |
| `u32` | `{number}` | `u32` (default — no override) | (default) | (default) |
| `Option<u32>` | (composed from `{number}`) | `Option<u32>` (default) | (default) | (default) |
| `chia_bls::PublicKey` | `clvm_types` (`bindings.json:64`) | direct exposure | direct exposure | direct exposure |
| `chia_bls::SecretKey` | (already in `bindings/bls.json:1-40`) | direct | direct | direct |
| `Vec<PublicKey>` | (already in `bindings/bls.json:75,81`) | `js_sys::Array` (`bindings.json:33`) | direct | direct |
| `Vec<OutputMeta>` | new — but a `Vec<UserType>` lowering already exists (e.g., `RequestedPayments`) | direct | direct | direct |
| `Vec<DetectedSpCoin>` | new — same shape as `Vec<OutputMeta>` | direct | direct | direct |

**No new entries in `bindings.json` (top-level) are needed.** All shapes Phase 3 ships are expressible by the existing type-groups. Phase 5 only needs to add `bindings/silent_payments.json` (the per-concept descriptor), not modify the top-level mapping file. This is a strong signal the design is binding-clean.

The open architectural question (Q3 in STATE.md Blockers — "Phase 5 (pre-flight): Verify `bindy-macro` `'type': 'static_functions'` schema support") is about how Phase 5 exposes free functions like `scan_from_tweaks` and `compute_shared_secret_from_tweak`. Phase 3 ships them as free `pub fn`s; Phase 5 will either group them under a zero-field carrier type (the "SilentPayments" pattern) or distribute them onto `SilentPaymentKeys` as methods. **Phase 3 is not affected by this choice** — the Rust API is identical either way.

## 5. CHIP-0058 forward compatibility

Per cross-cutting concern #5 in ROADMAP.md (line 152): `TweakData` has no transport fields (no `height`, no sp-service JSON envelope). The `<additional_context>` reinforces: "Phase 6's example uses only the simulator helper, not a WS client."

Concretely, this means:

- **`TweakData` does NOT contain a `height: u32`.** If a wallet wants to track which block a `TweakData` came from, that's the indexer adapter's concern; the scanner is height-agnostic.
- **`OutputMeta` does NOT contain a `confirmed_height`, `created_timestamp`, or `spent_height`.** The Phase 6 simulator helper can attach those fields client-side after detection if it wants — they're not part of the protocol.
- **No transport-error variants on `DriverError`.** The new `DriverError::SilentPayment(SilentPaymentError)` variant only wraps `chia_sdk_utils::silent_payments::SilentPaymentError`, which has 6 variants — none of them transport-shaped.

A future CHIP-0058 transport client constructs `TweakData` by:

1. Pulling wire messages from a server (probably WebSocket — see `sp-client/ws_client.rs` for the eventual shape).
2. Parsing each message's `tweak_point` field (48-byte compressed pubkey) into a `chia_bls::PublicKey`.
3. Parsing each output's `puzzle_hash` / `coin_id` / `amount` / `parent_coin_id` into `OutputMeta`.
4. Stuffing both into a `TweakData { tweak_points, outputs }` struct.

The boundary is unambiguous: the SDK does not know about wire formats; the transport client does not know about ECDH. Phase 3 ships the SDK side; Phase 6's example uses only the simulator path (`chia_sdk_test::silent_payments::tweak_data_from_simulator_block`, SIM-01).

## 6. The scanner algorithm — byte-for-byte

This is THE algorithm Phase 3 ships. Verbatim from `~/silent-payments/crates/sp-client/src/scanner.rs:58-140`, adapted for the SDK's `chia_sha2`/`chia_sdk_types::silent_payments`/`LabelRegistry` shape and with the two CHIP-spec guards added.

```rust
pub fn scan_from_tweaks(
    scan_sk: &SecretKey,
    spend_sk: &SecretKey,
    spend_pk: &PublicKey,
    data: &TweakData,
    labels: Option<&LabelRegistry>,
    k_max: usize,
) -> Vec<DetectedSpCoin> {
    // Build a lookup set of the candidate puzzle hashes for O(1) match.
    let output_phs: HashSet<Bytes32> = data.outputs.iter().map(|o| o.puzzle_hash).collect();

    let mut detected = Vec::new();

    for tweak_point in &data.tweak_points {
        // CHIP §459 guard: skip identity-element tweak points.
        // ECDH with the identity element produces a predictable shared secret,
        // a catastrophic privacy failure if not skipped.
        if tweak_point.is_inf() {
            continue;
        }

        let shared_secret = compute_shared_secret_from_tweak(scan_sk, tweak_point);

        for k in 0..u32::try_from(k_max).unwrap_or(u32::MAX) {
            let output_tweak = derive_output_tweak(&shared_secret, k);
            let candidate_pk = derive_onetime_pk(spend_pk, &output_tweak);
            let candidate_ph = puzzle_hash_for_pk(&candidate_pk);

            let mut found = false;

            // Unlabeled match — prefer this over a labeled re-derivation.
            if output_phs.contains(&candidate_ph) {
                if let Some(out) = data.outputs.iter().find(|o| o.puzzle_hash == candidate_ph) {
                    let onetime_sk = derive_onetime_sk(spend_sk, &output_tweak);
                    detected.push(DetectedSpCoin {
                        coin_id: out.coin_id,
                        puzzle_hash: out.puzzle_hash,
                        amount: out.amount,
                        parent_coin_id: out.parent_coin_id,
                        onetime_sk,
                        k,
                        label: None,
                    });
                    found = true;
                }
            }

            // Labeled detection branch — only if no unlabeled match at this k.
            if !found {
                if let Some(label_map) = labels {
                    for (m, label_pk) in label_map.iter() {
                        let labeled_pk = &candidate_pk + label_pk;
                        let labeled_ph = puzzle_hash_for_pk(&labeled_pk);
                        if output_phs.contains(&labeled_ph) {
                            if let Some(out) = data.outputs.iter().find(|o| o.puzzle_hash == labeled_ph) {
                                // Labeled one-time SK = base SK + label_scalar.
                                let base_sk = derive_onetime_sk(spend_sk, &output_tweak);
                                let (label_scalar, _) = chia_sdk_utils::silent_payments::generate_label_for_scan(scan_sk, m);
                                // ↑ generate_label is pub(super) in Phase 2; Phase 3 needs it pub(crate)
                                //   or duplicated locally. See §13 for the simplest fix.
                                let base_scalar = ScalarField::from_bytes_raw(base_sk.to_bytes());
                                let labeled_scalar = base_scalar.add(&label_scalar);
                                let labeled_sk = SecretKey::from_bytes(labeled_scalar.as_bytes())
                                    .expect("labeled scalar < r by ScalarField boundary");
                                detected.push(DetectedSpCoin {
                                    coin_id: out.coin_id,
                                    puzzle_hash: out.puzzle_hash,
                                    amount: out.amount,
                                    parent_coin_id: out.parent_coin_id,
                                    onetime_sk: labeled_sk,
                                    k,
                                    label: Some(m),
                                });
                                found = true;
                                break;
                            }
                        }
                    }
                }
            }

            // CHIP §RECV-04 termination rule: stop the k loop ONLY when
            // neither unlabeled nor any labeled candidate matched at this k.
            if !found {
                break;
            }
        }
        // If the k loop hit k_max without a gap, control falls through here.
        // No special handling needed — CHIP §416 explicitly allows stopping
        // at k_max regardless of "found" status.
    }

    detected
}
```

### Component primitives

```rust
pub fn compute_shared_secret_from_tweak(scan_sk: &SecretKey, tweak_point: &PublicKey) -> [u8; 32] {
    let mut point = *tweak_point;                       // PublicKey is Copy in chia-bls 0.36.1
    point.scalar_multiply(&scan_sk.to_bytes());         // in-place scalar multiply
    let mut h = chia_sha2::Sha256::new();
    h.update(point.to_bytes());                         // 48-byte compressed PK serialization
    h.finalize()                                        // 32-byte SHA-256 digest
}

pub fn derive_output_tweak(shared_secret: &[u8; 32], k: u32) -> ScalarField {
    let mut data = [0u8; 36];
    data[..32].copy_from_slice(shared_secret);
    data[32..].copy_from_slice(&k.to_be_bytes());       // ser32(k) — big-endian per CHIP §169
    let hash = tagged_hash(CHIA_SP_SHARED_SECRET, &data);
    ScalarField::from_bytes_unsigned(hash)              // unsigned mod-r reduction (Phase 1 boundary)
}

pub fn derive_onetime_pk(spend_pk: &PublicKey, tweak: &ScalarField) -> PublicKey {
    let tweak_sk = SecretKey::from_bytes(tweak.as_bytes())
        .expect("tweak < r by ScalarField boundary");
    let tweak_pk = tweak_sk.public_key();
    spend_pk + &tweak_pk
}

pub fn derive_onetime_sk(spend_sk: &SecretKey, tweak: &ScalarField) -> SecretKey {
    let sk_scalar = ScalarField::from_bytes_raw(spend_sk.to_bytes());
    let result = sk_scalar.add(tweak);
    SecretKey::from_bytes(result.as_bytes())
        .expect("result < r by ScalarField boundary")
}

pub fn puzzle_hash_for_pk(pk: &PublicKey) -> Bytes32 {
    use chia_puzzle_types::DeriveSynthetic;
    use chia_puzzle_types::standard::StandardArgs;
    let synthetic_pk = pk.derive_synthetic();
    StandardArgs::curry_tree_hash(synthetic_pk).into()
}
```

## 7. Labeled k-termination rule (RECV-04) — formalized

Quoting REQUIREMENTS.md RECV-04 + cross-checking against `~/silent-payments/crates/sp-client/src/scanner.rs:94-135` and `~/silent-payments/shared.py:365-398`:

**Rule:** at each `k`, the scanner tries unlabeled first (`output_phs.contains(&candidate_ph)`). If that misses, it iterates registered `(m, label_pk)` pairs in `LabelRegistry::iter()` and checks `output_phs.contains(&labeled_ph)` where `labeled_ph = puzzle_hash_for_pk(&candidate_pk + label_pk)`. The scanner increments `k` and continues the loop IF EITHER unlabeled or any labeled match was found at `k`. It breaks ONLY when *both* the unlabeled candidate AND every registered labeled candidate miss at the current `k`.

**Why this matters:** a sender can emit outputs at `k=0` (unlabeled) and `k=1` (labeled `m=1`) in the same spend group. A naive "break on unlabeled miss" implementation would miss the labeled output at `k=1`. The CHIP test vectors don't catch this (TV3 only tests `k=0` labeled), so the SDK needs a bespoke test (`labeled_k_termination_rule` in §11). The Python prototype has the correct logic; the reference Rust scanner has the correct logic; Phase 3 ships the correct logic.

## 8. `LabelRegistry::iter()` — Phase 2 surface

Phase 2 ships:

```rust
pub fn iter(&self) -> impl Iterator<Item = (u32, &PublicKey)> {
    self.forward.iter().map(|(&m, pk)| (m, pk))
}
```

(`crates/chia-sdk-utils/src/silent_payments/labels.rs:104-106`)

This is exactly the shape Phase 3 needs. No Phase 2 changes required.

**Phase 2 cross-cutting reach-through (`super::super::*`):** Phase 2's `labels.rs::tests::labels_preserve_scan_pk` test (line 171) demonstrates the reach pattern `use super::super::{SilentPaymentKeys, SilentPaymentNetwork};`. Phase 3 will use the analogous pattern from `chia-sdk-driver` to `chia-sdk-utils` via the public re-exports — `use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentKeys}`.

## 9. The two CHIP-spec guards the reference scanner is missing

**Identity-element guard (CHIP §459).** The reference scanner at `sp-client/scanner.rs:70` does `for tweak_point in tweaks` with no infinity check. Per CHIP §459: "If *A*<sub>sum</sub> == *O* (identity element after point addition), the scanner MUST skip this spend group." The check is `tweak_point.is_inf()`. Phase 3 adds this as the first action inside the outer loop.

**Reason:** if a tweak point is the identity, `scan_sk * O = O` (since `O` is the additive identity on the group). The hash `SHA256(serialize(O))` = `SHA256([0xc0, 0, 0, ..., 0])` is a constant. Every wallet with any scan key gets the same predictable shared secret for that tweak. A sufficiently motivated adversary can pre-compute candidate puzzle hashes from the predictable shared secret and force false positives. Skip is mandatory.

**`K_max` cap (CHIP §416).** The reference scanner at `sp-client/scanner.rs:73-136` uses an unbounded `loop { ... }` that breaks only on miss. Per CHIP §416: "implementations SHOULD enforce a maximum output count *K*<sub>max</sub> per spend group. If *k* reaches *K*<sub>max</sub> without a gap, the scanner stops iterating."

**Recommended default:** `K_MAX_DEFAULT: usize = 2400` per CHIP §446 (Chia mempool max). The `<additional_context>` mentions `K_max = 32` — this is FINE for test inputs (the DOS-guard test fires the cap at 32 to keep test time bounded) but is too tight as a production default and will silently miss legitimate `k > 32` outputs.

**Test vector consequences:** every published CHIP test vector hits `k = 0`. The bespoke `k = 1` test (CRYPTO-03) is the only test we have that exercises the loop's continuation. The DOS-guard test uses `k_max = 32` AND constructs adversarial inputs that force every `k ∈ [0, 31]` to match (forged matches via a colluding "indexer"); the test asserts the function returns within bounded time and produces at most 32 detections per tweak point.

## 10. CHIP test vectors — byte-exact reproductions

These are reproduced verbatim from `~/silent-payments/tests/test_vectors.py` (lines 41-393), `~/silent-payments/crates/sp-common/src/protocol.rs:tests` (lines 197-323), and `~/silent-payments/crates/sp-client/src/scanner.rs:tests` (lines 147-462). The reference impl runs these as Python pytest and as Rust unit tests; both are green.

### 10a. TV1 — unlabeled single-input (`k=0`)

Mnemonic (both sender and recipient): `"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"`

| Item | Value (32 or 48 bytes hex) |
|---|---|
| `coin_id` (= `SHA256("test-vector-1-coin")`) | `5d759d2d97c03b1f6fe0657e91d25f6b7dd1311d6023271a1bcd35978a94a175` |
| sender `wallet_sk` (`m/12381/8444/2/0`) | `6c8d1a9f97413f8d8e8c158f5bc875b58b498de05c9109b4dc240280d32e2a31` |
| sender `syn_sk` | `5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a` |
| sender `syn_pk` (`A_sum` for single-input) | `8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c51b3f2d87dc03a657e07f1828ab760fa` |
| recipient `scan_sk` (`b_scan`) | `132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6` |
| recipient `scan_pk` (`B_scan`) | `a04f404bfbfdc9311736899fe32d2275bb007814510c3523529487ad7573607573ade20d31c75107b40331fff79ac896` |
| recipient `spend_sk` (`b_spend`) | `53d140b312a0e16316314274eb6398e15706d100fe8a754990540febd931b087` |
| recipient `spend_pk` (`B_spend`) | `8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527dbf6f31e22e0831ba5088c952625a75c74` |
| `input_hash` | `38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411` |
| `ECDH_point` (= `(input_hash * a_syn) * B_scan`) | `aa15516b2b572ebedcd3c048c07189485f8374449923389534125e99763845700d6d84d8edaf73b6516874d9a798de09` |
| `tweak_point` (= `input_hash * A_sum`) | (recomputable: scalar_multiply A_sum by input_hash_bytes; the value the scanner's `compute_shared_secret_from_tweak` consumes) |
| `shared_secret` (= `SHA256(ECDH_point)`) | `d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6` |
| `t_0` (= `derive_output_tweak(shared_secret, 0)`) | `5c560301c50fa309ad43d0f82cd1af143f6e3769659c80e8c14a072331582ab1` |
| `onetime_pk` (= `B_spend + t_0 * G`) | `b671487c1d275842f529f7a73a63a32a9a1a49e1dbabcac4058cc48626b6db31f48dc49e769a6f8076a9111ff14e964d` |
| `puzzle_hash` (= `StandardArgs::curry_tree_hash(onetime_pk.derive_synthetic())`) | `23adba149dd9000d65e0f8e21b6975364cbe89a63caf56533df4b7664c21fbf5` |
| `onetime_sk` (= `(b_spend + t_0) mod r`) | `3c399c61ae130724903b3b650e936ff042b7646764289a33519e17100a89db37` |

### 10b. TV2 — multi-output, two recipients (NOT a Phase-3 closure target; reproduced for completeness)

Sender uses TEST_MNEMONIC_1; recipient A uses TEST_MNEMONIC_1; recipient B uses `"zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"`. Two distinct `shared_secret`s, two distinct `tweak`s, two distinct `puzzle_hash`es. Phase 3 doesn't need TV2 because the scanner is a single-recipient function (the wallet is one party); TV2 verifies create-side correctness only. Mentioned here only so the planner doesn't accidentally try to land it.

### 10c. TV3 — labeled single-input, `m=1`, `k=0`

Sender mnemonic, recipient mnemonic identical to TV1. Recipient computes labeled `B_m = B_spend + label_pk(m=1)`.

| Item | Value |
|---|---|
| `coin_id` (= `SHA256("test-vector-3-coin")`) | `4504f59ea184be18924f95244649287382ec6cdc13f333a8990f648c803a6dac` |
| `label_scalar` (= `tagged_hash("Chia_SP/Label", b_scan ‖ ser32(1))` mod r) | `48fa440acca87f501b9984b5d23327d0b7766a4baa913dfb3001d412c48ce465` |
| `label_pk` (= `label_scalar * G`) | `a6dcff3646739745ef7f3ba8e51808dac13765fa9d5e73386d3fbd7841e0773e02a0f8d91baf57d337954322bd06d80c` |
| `B_m` (= `B_spend + label_pk`) | `965250fb8503cff4c244f360ab84075bfe2da01091745d0e8ce36024ab12e96277d1f02fbbe01cee412dd2ce1b7414c2` |
| `input_hash` | `58a1875602949aa6bfaf9cb4837957e7175ffb0b14422dbc8d371799f98e66f5` |
| `shared_secret` | `3d1eabb622c40142d4b2557fc222a22cd93d98550255cecb2b6a84985f49215d` |
| `t_0` | `301e842ace534f7de854dcc5a48a656d7e9a6d8b8f93db9fb8277f4d1889bdf1` |
| `onetime_pk` (= `B_m + t_0 * G`) | `97e7466509081a3ed6e50ba0231a6fa1b48d8c910ac6ec933e26cd5091569c615f299726c91a730dbf51a26cb249f17c` |
| `puzzle_hash` | `ba271d218d487e8e5dc994a09a8580e1e8a0559a615bd5805cff11b5a343441c` |
| `base_onetime_sk` (= `(b_spend + t_0) mod r` — without label) | `10021d8ab756b398cb4c4732864c264981e39a898e1ff4ea487b8f39f1bb6e77` |
| `labeled_onetime_sk` (= `(base + label_scalar) mod r`) | `58fc619583ff32e8e6e5cbe8587f4e1a395a04d538b132e5787d634cb64852dc` |

### 10d. TV4 — multi-input aggregation (2 sender coins, `k=0`)

Same mnemonic for sender; sender spends two coins at wallet indices 0 and 1; sender aggregates the two synthetic keys before computing `input_hash`.

| Item | Value |
|---|---|
| `coin_id_0` (= `SHA256("test-vector-4-coin-0")`) | `2b9857e0307ebfbe51829e3be8c992ae57f6a8debe06a5deab429ddae83a8c1a` |
| `coin_id_1` (= `SHA256("test-vector-4-coin-1")`) | `209bb03a4cd165785e6149bc6dcb27e35829006f02ec927ab5a20521fd27d21a` |
| `coin_id_min` (lex-smallest — `coin_id_1` < `coin_id_0`) | `209bb03a4cd165785e6149bc6dcb27e35829006f02ec927ab5a20521fd27d21a` |
| `syn_sk_0` (wallet index 0) | `5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a` |
| `syn_pk_0` | `8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c51b3f2d87dc03a657e07f1828ab760fa` |
| `syn_sk_1` (wallet index 1) | `05fded8808216b65d439fc41cb07c7270e37ed743e0745652afe055cfe91cf0f` |
| `syn_pk_1` | `94c5c19f4343bc2655af729469285a392de9048851363b0b1329a4539a46ab4c6e8bfb39d32da25bffe4d9cdbe3e1061` |
| `a_sum` (= `(syn_sk_0 + syn_sk_1) mod r`) | `5600d8781de32f0f3d86bc96b46a3a4ea56ae26da168b55dc66b503acbf95b89` |
| `A_sum` (= `syn_pk_0 + syn_pk_1`) | `a223ab27f801044cd98c8314014b8073347b0e5aae43c69b78b5ca2a562ee9f799b8efad179b34da1b306ca4d62bad40` |
| `input_hash` (= `tagged_hash("Chia_SP/Inputs", coin_id_min ‖ A_sum)` mod r) | `3f1071552b7f2f5e49b68166cb204f0a1b6a23b0c30a28bcba59a9c3f766e166` |
| `shared_secret` | `e729dea8c4732747d0e5e930607c52ddfce01ff7c72eaec9ee7c84131e078494` |
| `t_0` | `18fafd6001bef3fece078f469731b40a5f362994795f9ff6b9339aa235fee312` |
| `onetime_pk` | `b71f484e6d90a657b215ad7bff6f96a8d9bff07e0133d74917cc6c3ef6fa273a706aa56e1fd6da19ed5466f16450ccb1` |
| `puzzle_hash` | `5d7fc7d7447c746cfb400e801a169fc7bfd1c13e03bc7866e6b743860a53ac6b` |
| `onetime_sk` | `6ccc3e13145fd561e438d1bb82954cebb63cfa9577ea15404987aa8e0f309399` |

### 10e. The bespoke `k=1` test vector (NEW for this SDK)

No published CHIP vector hits `k = 1`. Phase 3 generates one locally and pins its value. **This is a Phase 3 deliverable** — the values below are NOT yet pinned in any reference; the planner schedules a one-time computation task (Python script + Rust assertion) to compute and pin them.

**Construction:** reuse TV1's `shared_secret = d3ac1e8f...0ba2c6` (so the test setup is short — just feed the literal bytes) + `derive_output_tweak(shared_secret, 1)` and recompute downstream.

| Item | How to compute |
|---|---|
| `shared_secret` (reused from TV1) | `d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6` |
| `t_1` | `ScalarField::from_bytes_unsigned(tagged_hash("Chia_SP/SharedSecret", shared_secret ‖ [0x00, 0x00, 0x00, 0x01]))` — pin the resulting 32-byte value (TBD; Plan-3 Task 1 computes and pins) |
| `onetime_pk_k1` | `B_spend + t_1 * G` — pin the resulting 48-byte value (TBD) |
| `puzzle_hash_k1` | `StandardArgs::curry_tree_hash(onetime_pk_k1.derive_synthetic())` — pin the resulting 32-byte value (TBD) |
| `onetime_sk_k1` | `(b_spend + t_1) mod r` — pin the resulting 32-byte value (TBD) |

**Test assertion:** `scan_from_tweaks` against a `TweakData` containing TV1's `tweak_point` + an `OutputMeta` whose `puzzle_hash` equals the (pinned-at-test-write-time) `puzzle_hash_k1` returns exactly one `DetectedSpCoin` with `k == 1` and `onetime_sk.to_bytes() == onetime_sk_k1`.

**What this catches:** an implementation that swaps `k.to_be_bytes()` for `k.to_le_bytes()` produces `t_0 = tagged_hash(..., shared_secret ‖ [0x01, 0x00, 0x00, 0x00])` for `k = 1`. Since `k = 0` serializes to `[0, 0, 0, 0]` under either BE or LE, every TV passes regardless. Catching this requires `k > 0`.

### 10f. Recipient-side scan check — TV1

`scan_from_tweaks` invocation (single-tweak, single-output, unlabeled, no labels):

```rust
let detections = scan_from_tweaks(
    &scan_sk_from_TV1_bytes,
    &spend_sk_from_TV1_bytes,
    &spend_pk_from_TV1_bytes,
    &TweakData {
        tweak_points: vec![tv1_tweak_point()],   // see §11 — computed from sender_pk * input_hash
        outputs: vec![OutputMeta {
            puzzle_hash: hex!("23adba149dd9000d65e0f8e21b6975364cbe89a63caf56533df4b7664c21fbf5").into(),
            coin_id: hex!("5d759d2d97c03b1f6fe0657e91d25f6b7dd1311d6023271a1bcd35978a94a175").into(),
            amount: 1000,
            parent_coin_id: [0u8; 32].into(),
        }],
    },
    None,                            // labels
    K_MAX_DEFAULT,
);
assert_eq!(detections.len(), 1);
assert_eq!(detections[0].puzzle_hash, hex!("23adba14...4c21fbf5").into());
assert_eq!(detections[0].onetime_sk.to_bytes(), hex!("3c399c61...0a89db37"));
assert_eq!(detections[0].k, 0);
assert!(detections[0].label.is_none());
```

This is the literal shape of the TV1 test. The other vectors follow the same template with their own pinned values.

## 11. The eleven-test plan

Phase 3's tests live in `crates/chia-sdk-driver/src/silent_payments/scanner.rs::tests`. Each pins byte-exact values from §10 (or, for the `k=1` test, computed-and-pinned values).

| # | Test name | Maps to | What it pins |
|---|-----------|---------|--------------|
| 1 | `tv1_shared_secret_matches` | RECV-03, CRYPTO-03 | `compute_shared_secret_from_tweak` produces TV1's `d3ac1e8f...0ba2c6` from TV1's `tweak_point` |
| 2 | `tv1_scan_detects_unlabeled_k0` | RECV-02, CRYPTO-03 | `scan_from_tweaks` returns exactly 1 detection with `k=0`, `label=None`, and TV1's pinned `onetime_sk` |
| 3 | `tv3_scan_detects_labeled_k0` | RECV-02, RECV-04, CRYPTO-03 | `scan_from_tweaks` with `LabelRegistry` containing `m=1` returns 1 detection with `k=0`, `label=Some(1)`, and TV3's pinned labeled `onetime_sk = 58fc6195...b64852dc` |
| 4 | `tv4_scan_detects_multi_input_aggregation` | RECV-02, CRYPTO-03 | A `tweak_point` computed from TV4's `A_sum` and `input_hash` produces 1 detection with `k=0`, no label, and TV4's pinned `onetime_sk = 6ccc3e13...e0f309399` |
| 5 | `bespoke_k1_detection` | RECV-02, CRYPTO-03 (k=1) | After populating an `OutputMeta` whose `puzzle_hash` equals the locally-computed `puzzle_hash_k1` (§10e), `scan_from_tweaks` returns 1 detection with `k=1`. Catches `ser32(k)` endianness bugs. |
| 6 | `adversarial_ff32_scalar_reduces_unsigned` | CRYPTO-03 | A constructed scenario where the `shared_secret` SHA-256 output (or the subsequent tagged_hash input) has high-bit-set 32 bytes, forcing the protocol path to exercise `ScalarField::from_bytes_unsigned` end-to-end. Asserts the produced `onetime_pk` matches the unsigned-reduction value, NOT the signed-reduction value. (Implementation note: simplest construction is to feed an artificial `tweak_point` whose SHA-256 output has the high bit set — Phase 3 picks any seed that produces such an output; the test is deterministic on whatever seed is chosen.) |
| 7 | `identity_tweak_point_skipped` | RECV-02 (CHIP §459) | `scan_from_tweaks` against a `TweakData { tweak_points: vec![PublicKey::default()], outputs: ... }` returns `Vec::new()` without panicking. Verifies the `is_inf()` guard. |
| 8 | `malformed_pubkey_caught_at_deserialization` | (defensive) | NOT a scanner test — a test in `tests.rs` or `types.rs` that asserts `PublicKey::from_bytes(&[0xff; 48])` returns `Err(...)`, demonstrating that malformed bytes never reach the scanner. Documents the boundary. |
| 9 | `dos_guard_caps_at_k_max` | RECV-05, CHIP §416 | A `TweakData` constructed by computing the legitimate one-time puzzle hashes at `k = 0..=10000` for TV1 (the scanner's perspective: every `k` in `[0, 10000]` matches a candidate output), passed to `scan_from_tweaks` with `k_max = 32`. Asserts that `detections.len() <= 32` and the function returns within a bounded time. |
| 10 | `labeled_k_termination_rule` | RECV-04 | A `TweakData` carrying TWO outputs: an unlabeled output for k=0 (matches base puzzle hash) and a labeled output for k=1 (matches `puzzle_hash_for_pk(candidate_pk + label_pk)`). `LabelRegistry` registers `m=1`. Assert `detections.len() == 2`, `detections[0].k == 0`/`label = None`, `detections[1].k == 1`/`label = Some(1)`. If the scanner broke after the unlabeled match at k=0 (incorrect implementation), it would miss the labeled output at k=1. |
| 11 | `unlabeled_preferred_over_labeled_at_same_k` | RECV-04 (corner) | When the unlabeled candidate at `k=0` matches AND a labeled candidate at `k=0` ALSO matches the same output, the scanner reports the unlabeled detection (`label = None`). Mirrors `sp-client/scanner.rs:test_scan_block_unlabeled_preferred` (line 462). |

**Optional bonus tests** (planner discretion — close coverage gaps without blocking the requirement):

- `empty_tweak_data_returns_empty` — `scan_from_tweaks(&TweakData { tweak_points: vec![], outputs: vec![] }, ...)` returns `Vec::new()`. Trivial; one line.
- `empty_outputs_returns_empty` — non-empty tweaks but empty outputs returns `Vec::new()`. Trivial; one line.
- `wrong_scan_sk_no_detections` — using a different `scan_sk` against TV1's tweak_point + output yields no detections. Defensive.

## 12. Runtime State Inventory

Not applicable — Phase 3 is a greenfield code-only addition. No rename, no refactor, no migration, no stored data, no live service config, no OS-registered state, no secrets, no build artifacts to invalidate. (Per the GSD researcher protocol, this section is included with an explicit "N/A" justification rather than omitted, so the planner doesn't need to ask.)

## 13. The `generate_label` reach-through detail (small Phase 2 surface-area question)

Phase 2 shipped `generate_label` as `pub(super)` in `crates/chia-sdk-utils/src/silent_payments/labels.rs:31`. Phase 3's scanner needs to compute the `label_scalar` for a detected labeled match (to build the labeled `onetime_sk`). Two options:

**Option A (recommended):** promote `generate_label` to `pub(crate)` in `chia-sdk-utils` AND add a tiny `pub fn generate_label_for_scan(scan_sk, m) -> (ScalarField, PublicKey)` public reach-through in `chia_sdk_utils::silent_payments` (in `mod.rs` or `labels.rs`) that Phase 3's driver code can import. One-line change to Phase 2's labels.rs (`pub(super)` → `pub`), one-line public re-export.

**Option B:** duplicate the 10-line `generate_label` implementation in `chia-sdk-driver`. Avoids the Phase 2 surface change but creates a maintenance hazard (two places to update if the CHIP changes the label-scalar construction).

**Option C:** compute the label_scalar on-the-fly inside the scanner from `(scan_sk, m, label_pk)` — but the algorithm IS `tagged_hash("Chia_SP/Label", b_scan ‖ ser32(m))`, so this is exactly Option B.

Recommend **Option A**. The reach-through name should be `generate_label` (matching the `sp-common::generate_label` precedent) — there's no reason to rename. The Phase 2 plan summary documents the `pub(super)` choice as "exposed via `pub(super)` so only the `silent_payments::*` module tree can reach it"; Phase 3 broadens this to "the silent-payments code tree across the workspace can reach it" which is the next-narrowest scope.

## 14. CI matrix touch (WS-02 parallel for Phase 3)

Phases 1 and 2 added per-crate `chip-0057` build lines for `chia-sdk-types` and `chia-sdk-utils` respectively (`.github/workflows/rust.yml`). Phase 3 ADDs:

```yaml
cargo build --release -p chia-sdk-driver -F chip-0057
```

placed immediately after the existing `chia-sdk-driver -F chip-0035`, `-F chip-0037`, `-F action-layer` lines, with 10-space indent matching the surrounding lines. No new CI job; just one new build-command line.

## 15. State of the Art

| Old approach (would be wrong here) | Current approach | Why |
|---|---|---|
| Hand-rolled `puzzle_hash_for_pk` via direct CLVM serialization | `StandardArgs::curry_tree_hash(pk.derive_synthetic())` | `chia-puzzle-types::standard` already does the curry. Hand-rolling means re-deriving the standard-puzzle mod hash and is exactly the kind of "don't hand-roll" anti-pattern the SDK explicitly forbids (REQUIREMENTS.md "Out of Scope (permanently): Replacing the existing `StandardArgs::curry_tree_hash` + `derive_synthetic` flow with a hand-rolled `puzzle_hash_for_pk`"). |
| Storing `PublicKey` bytes as `[u8; 48]` in `TweakData` | `chia_bls::PublicKey` in `TweakData` | The bindings infrastructure already maps `PublicKey` cleanly (it's in `clvm_types` in `bindings.json:64`). Bytes would force a deserialization pass at every call site. |
| Using `[u8; 32]` for `coin_id`/`puzzle_hash`/`parent_coin_id` in `OutputMeta` | `chia_protocol::Bytes32` | SDK-wide convention. `Bytes32` is `Copy`, derefs to `[u8; 32]`, AND maps to `{bytes}` in the bindings type group. |
| Using `sha2::Sha256` directly (as the reference impl does in `sp-client/scanner.rs:11`) | `chia_sha2::Sha256` | Workspace Phase-1 grep ban `! grep -rE '^use sha2::' silent_payments/`. Already enforced for Phase 2; extends. |
| `loop { ... if !found { break; } }` unbounded k iteration (`sp-client/scanner.rs:73-135`) | `for k in 0..k_max` bounded iteration with a `break` on miss | CHIP §416 mandates the cap. The reference impl predates this CHIP requirement. |
| Returning `Result<Vec<DetectedSpCoin>, ...>` from `scan_from_tweaks` | Returning `Vec<DetectedSpCoin>` directly | No error paths inside the scan; identity-element guard skips silently per CHIP §459, malformed inputs are caught before they reach the function. Keeps the binding signature trivial. |

**Deprecated/outdated:** none. CHIP-0057 is current; the reference implementation tracks the latest draft.

## 16. Common Pitfalls

### Pitfall 1: `ser32(k)` endianness silently wrong on all CHIP TVs
**What goes wrong:** Implementer writes `k.to_le_bytes()` instead of `k.to_be_bytes()` for the `ser32(k)` step inside `derive_output_tweak`. Since every published CHIP TV has `k = 0` (which serializes to `[0,0,0,0]` under either byte order), every TV passes. Detection failures only manifest in production when a wallet sends two outputs to the same recipient in one batch (which triggers `k = 1` on the second output).
**Why it happens:** The `to_be_bytes` / `to_le_bytes` choice is invisible from a reading of the success criteria for TV1/TV3/TV4.
**How to avoid:** The bespoke `k=1` test (§10e, §11 test #5). The test pins the `k=1` output's `onetime_sk` and `puzzle_hash` so any LE implementation fails the assertion.
**Warning signs:** A scanner that detects exactly one output per `tweak_point` regardless of how many were sent.

### Pitfall 2: Forgetting the labeled-termination rule
**What goes wrong:** Implementer writes `if unlabeled_match { detected.push(...); k += 1; } else { break; }` and only checks labels when the unlabeled candidate matches. Labeled-only detections (`k=1` labeled when `k=0` is unlabeled) are missed.
**Why it happens:** TV3 only exercises labeled at `k=0`; the labeled-termination rule is not visible from a TV that hits `k=0`.
**How to avoid:** Test #10 in §11. The test constructs a TweakData with both an unlabeled k=0 output AND a labeled k=1 output and asserts both are detected.
**Warning signs:** Labeled detections that work fine when there's exactly one labeled output, but mysteriously miss labeled outputs that follow an unlabeled match.

### Pitfall 3: Adversarial scalar with high bit set silently reduces wrong
**What goes wrong:** Implementer routes the `tagged_hash` output through `chia_puzzle_types::derive_synthetic`'s `mod_by_group_order` (signed reducer) instead of `ScalarField::from_bytes_unsigned` (unsigned reducer). On inputs whose first byte is `>= 0x80`, the two routes disagree silently. CHIP TVs do NOT trigger this — none of the published `t_k` values have high-bit-set hashes.
**Why it happens:** Easy temptation to "reuse the existing reducer" without noticing the sign interpretation.
**How to avoid:** Phase 1 already eliminated this via the type-system boundary on `ScalarField`. Phase 3's adversarial test (#6 in §11) forces a high-bit-set scalar through the full protocol path and pins the unsigned-reduction result.
**Warning signs:** `grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` returns hits. Phase 1 grep ban catches this in CI.

### Pitfall 4: `compute_shared_secret_from_tweak` runs on identity-element tweak point
**What goes wrong:** A malicious indexer constructs a `TweakData` where one `tweak_point` is the identity element `O`. ECDH with `O` produces `O`; its compressed serialization is the constant `0xc000...00`. Every wallet that scans the same `TweakData` derives the same predictable shared secret. The attacker pre-computes candidate puzzle hashes and stuffs them into the `outputs` list; the scanner reports false-positive detections.
**Why it happens:** Reference impl doesn't have this guard. The exposure is real but easily prevented.
**How to avoid:** `if tweak_point.is_inf() { continue; }` at the top of the outer loop (§6 code). Test #7 in §11.
**Warning signs:** Scanner produces detections from an obviously-zero tweak point in a unit test.

### Pitfall 5: `K_max` cap missing — DOS via forged matches
**What goes wrong:** A malicious indexer constructs a `TweakData` where every `k ∈ [0, ∞)` for a particular `tweak_point` matches an attacker-supplied puzzle hash (the attacker pre-computes `puzzle_hash_for_pk(candidate_pk_at_k)` for every `k`). Without a cap, the scanner runs forever. The attacker controls the wallet's scanner.
**Why it happens:** The reference impl's unbounded `loop { ... }` predates the CHIP §416 cap requirement.
**How to avoid:** Bounded `for k in 0..k_max` loop (§6 code). Test #9 in §11.
**Warning signs:** Scanner hangs on a crafted test input; `cargo test --timeout` fires.

### Pitfall 6: Identity-element pubkey halves in `TweakData` deserialization
**What goes wrong:** Caller deserializes a 48-byte tweak-point bytestring whose value is the identity element (`0xc000...00`) into a `PublicKey`. `chia_bls::PublicKey::from_bytes` accepts this successfully (returns the identity point), and the scanner then either panics on the scalar multiply (it doesn't — `scalar_multiply` of `O` is `O`) or produces predictable shared secrets (Pitfall 4).
**Why it happens:** `chia-bls 0.36.1`'s `PublicKey::from_bytes` does NOT reject infinity (Phase 2 confirmed this empirically). The check must happen one layer up.
**How to avoid:** The `is_inf()` guard inside the scanner (covers the case where the bytes successfully deserialize but encode the identity). The TweakData construction site (Phase 6's `tweak_data_from_simulator_block` and any future CHIP-0058 transport client) is the second line of defense and should also reject identity-element tweak points up front.
**Warning signs:** Phase 6's E2E simulator E2E test starts producing zero detections from a manually crafted `TweakData`.

### Pitfall 7: `chia_bls::PublicKey` Copy semantics on `scalar_multiply`
**What goes wrong:** `scalar_multiply` is `&mut self` (in-place). The reference impl writes `let mut point = *tweak_point; point.scalar_multiply(&scan_sk.to_bytes());`. If a contributor refactors to `tweak_point.scalar_multiply(&scan_sk.to_bytes())` (treating it as a method call) — they accidentally mutate the caller's data. Phase 2's `02-PHASE-SUMMARY.md` line 162 notes "PublicKey is `Copy` in `chia-bls 0.36.1`" — clippy::clone_on_copy may flag explicit `.clone()` calls.
**Why it happens:** The two patterns (`let mut point = *tweak_point; point.scalar_multiply(...)` vs `tweak_point.scalar_multiply(...)`) look interchangeable.
**How to avoid:** Always explicit copy: `let mut point = *tweak_point;`. Lint via `cargo clippy --features chip-0057 --all-targets -- -D warnings`.
**Warning signs:** `error[E0596]: cannot borrow ... as mutable` (caught at compile time — never silent).

## 17. Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---|---|---|---|
| Convert a `PublicKey` to a standard p2 puzzle hash | Custom `tree_hash` of a `(standard mod, pk)` cons cell | `StandardArgs::curry_tree_hash(pk.derive_synthetic())` from `chia-puzzle-types` | The standard mod hash and synthetic-offset construction are pinned by Chia consensus; rebuilding either is footgun-rich. The SDK already does this in `crates/chia-sdk-driver/src/layers/standard_layer.rs:118`. |
| BIP-340 tagged hash | Custom `SHA256(...)` calls | `chia_sdk_types::silent_payments::tagged_hash` (Phase 1) | Identical construction across all 3 CHIP-0057 tags; Phase 1 already ships and tests the primitive. |
| Unsigned mod-r scalar reduction | `num_bigint::BigUint::from_bytes_be(&bytes) % r` | `ScalarField::from_bytes_unsigned` (Phase 1) | The whole point of the `ScalarField` type boundary is preventing the unsigned-vs-signed crossover hazard. Bypassing it reintroduces the bug. |
| BLS12-381 G1 point compression / decompression | Custom 48-byte unpack | `chia_bls::PublicKey::{to_bytes, from_bytes}` | Inherits the well-tested chia-bls implementation. |
| BLS12-381 scalar multiplication | Hand-rolled point doubling | `chia_bls::PublicKey::scalar_multiply` | Same. Plus it's in-place which matters for perf. |
| BLS12-381 point addition | Hand-rolled | `&pk + &other_pk` | `chia-bls` provides `impl Add for PublicKey` — `+` is point addition. Concise and well-tested. |
| Bidirectional label-pk → m mapping | Custom `HashMap` shape | `chia_sdk_utils::silent_payments::LabelRegistry` (Phase 2) | Phase 2 already ships the bidirectional structure with the correct `[u8; 48]` reverse key (a `HashMap<PublicKey, u32>` wouldn't compile — `PublicKey` doesn't implement `Hash`). |
| SHA-256 hashing | `sha2::Sha256` | `chia_sha2::Sha256` | Workspace Phase-1 grep ban. |
| `Err` type for silent-payment failures | New top-level error enum | `chia_sdk_utils::silent_payments::SilentPaymentError` (Phase 2) — extend with a new variant if needed, OR introduce `DriverError::SilentPayment(#[from] SilentPaymentError)` | Phase 2 shipped a 6-variant error enum. Phase 3's scanner doesn't return errors directly (see §3), but Phase 4 needs `DriverError::SilentPayment(...)` for send-side. Defining it now (one new variant in `driver_error.rs`) avoids a second touch in Phase 4. |

## 18. Code Examples

### Phase-1 primitive use (verified from real files)

```rust
// crates/chia-sdk-types/src/silent_payments/tagged_hash.rs:29-39
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

### Phase-2 generate_label (the helper Phase 3 will reach through to)

```rust
// crates/chia-sdk-utils/src/silent_payments/labels.rs:31-41
pub(super) fn generate_label(scan_sk: &SecretKey, m: u32) -> (ScalarField, PublicKey) {
    let mut data = [0u8; 36];
    data[..32].copy_from_slice(&scan_sk.to_bytes());
    data[32..].copy_from_slice(&m.to_be_bytes());
    let hash = tagged_hash(CHIA_SP_LABEL, &data);
    let label_scalar = ScalarField::from_bytes_unsigned(hash);
    let label_sk = SecretKey::from_bytes(label_scalar.as_bytes())
        .expect("ScalarField::from_bytes_unsigned guarantees value < r");
    (label_scalar, label_sk.public_key())
}
```

Phase 3 promotes this to `pub(crate)` + adds a one-line public reach-through (§13).

### Reference scanner core loop (verbatim source — `sp-client/scanner.rs:74-135`)

(Reproduced in §6 above with the two CHIP-spec guards added.)

### Phase-3 scanner usage from a Phase-6 simulator E2E (template)

```rust
let mnemonic = Mnemonic::parse(TV1_MNEMONIC).unwrap();
let recipient = SilentPaymentKeys::from_mnemonic(&mnemonic);

// (Phase 6 helper — not Phase 3's responsibility)
let tweak_data = chia_sdk_test::silent_payments::tweak_data_from_simulator_block(&sim, height);

let detections = chia_sdk_driver::silent_payments::scan_from_tweaks(
    recipient.scan_sk(),
    recipient.spend_sk(),
    recipient.spend_pk(),
    &tweak_data,
    None,
    chia_sdk_driver::silent_payments::K_MAX_DEFAULT,
);
```

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain 1.90.0 | Build / test | ✓ | `rust-toolchain.toml` pins | — |
| `cargo` | Build / test | ✓ | bundled with rustup | — |
| `chia-bls` (BLS12-381 point ops) | Crypto primitives | ✓ | 0.36.1 (workspace dep) | — |
| `chia-puzzle-types` (`StandardArgs`, `DeriveSynthetic`) | `puzzle_hash_for_pk` | ✓ | 0.36.1 (workspace dep) | — |
| `chia-sha2` | SHA-256 for shared secret | ✓ | 0.36.1 (workspace dep) | — |
| `chia-protocol` (`Bytes32`) | `OutputMeta` / `DetectedSpCoin` fields | ✓ | 0.36.1 (workspace dep) | — |
| `clvm-utils` (`TreeHash`) | Puzzle-hash construction | ✓ | 0.36.1 (workspace dep) | — |
| `num-bigint` (used by `ScalarField`) | (transitive via `chia-sdk-types`) | ✓ | 0.4.6 | — |
| `hex-literal` | Test vectors | ✓ | 0.4.1 (workspace dep) | — |
| `hex` | Test vectors | ✓ | 0.4.3 (workspace dep) | — |
| `thiserror` | `DriverError` variant | ✓ | 2.0.17 (workspace dep) | — |
| `~/silent-payments/` reference | Algorithm cross-check | ✓ | local clone, last fetched 2026-03-30 | — |
| `~/silent-payments/tests/test_vectors.py` | Test vector source | ✓ | local | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

The entire dependency set is already in place from Phases 1 and 2. Phase 3 needs zero new workspace deps and zero new optional deps — every `use` chain resolves through `chia-sdk-driver`'s existing `Cargo.toml`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in libtest); `rstest 0.22.0` available as a workspace dev-dep for parametric coverage if the planner wants it |
| Config file | None — `cargo test --release -p chia-sdk-driver --features chip-0057` is the canonical invocation |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments` |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RECV-01 | `TweakData { tweak_points, outputs }` struct exists, fields exposed | compile-time | `cargo build -p chia-sdk-driver -F chip-0057` | ❌ Wave 0 — `crates/chia-sdk-driver/src/silent_payments/types.rs` |
| RECV-01 | `OutputMeta` struct exists | compile-time | (same build) | ❌ Wave 0 |
| RECV-01 | `DetectedSpCoin` struct exists | compile-time | (same build) | ❌ Wave 0 |
| RECV-02 | `scan_from_tweaks(TV1) -> 1 detection at k=0, label=None, onetime_sk=3c399c61...0a89db37` | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::scanner::tests::tv1_scan_detects_unlabeled_k0 -- --exact` | ❌ Wave 0 — `crates/chia-sdk-driver/src/silent_payments/scanner.rs` |
| RECV-02 | `scan_from_tweaks(TV4 multi-input) -> 1 detection at k=0, onetime_sk=6ccc3e13...e0f309399` | unit | `cargo test ... silent_payments::scanner::tests::tv4_scan_detects_multi_input_aggregation -- --exact` | ❌ Wave 0 |
| RECV-02 | bespoke `k=1` detection produces correct labeled `onetime_sk` | unit | `cargo test ... silent_payments::scanner::tests::bespoke_k1_detection -- --exact` | ❌ Wave 0 |
| RECV-03 | `compute_shared_secret_from_tweak(TV1) == d3ac1e8f...0ba2c6` | unit | `cargo test ... silent_payments::scanner::tests::tv1_shared_secret_matches -- --exact` | ❌ Wave 0 |
| RECV-04 | labeled detection at k=0 returns `Some(1)` with TV3's `labeled_onetime_sk = 58fc6195...b64852dc` | unit | `cargo test ... silent_payments::scanner::tests::tv3_scan_detects_labeled_k0 -- --exact` | ❌ Wave 0 |
| RECV-04 | k loop continues past unlabeled match if labeled match exists at next k | unit | `cargo test ... silent_payments::scanner::tests::labeled_k_termination_rule -- --exact` | ❌ Wave 0 |
| RECV-04 | unlabeled preferred when both match at same k | unit | `cargo test ... silent_payments::scanner::tests::unlabeled_preferred_over_labeled_at_same_k -- --exact` | ❌ Wave 0 |
| RECV-05 | `K_max = 32` adversarial 10k-forged-match TweakData returns ≤ 32 detections per tweak point | unit (bounded) | `cargo test ... silent_payments::scanner::tests::dos_guard_caps_at_k_max -- --exact` | ❌ Wave 0 |
| RECV-02 (CHIP §459) | identity-element tweak point skipped silently | unit | `cargo test ... silent_payments::scanner::tests::identity_tweak_point_skipped -- --exact` | ❌ Wave 0 |
| (defensive) | malformed pubkey bytes rejected at `PublicKey::from_bytes`, never reach scanner | unit | `cargo test ... silent_payments::types::tests::malformed_pubkey_caught_at_deserialization -- --exact` | ❌ Wave 0 — `crates/chia-sdk-driver/src/silent_payments/types.rs` |
| CRYPTO-03 | adversarial `[0xff;32]` scalar fed through full protocol uses unsigned reduction, not signed | unit | `cargo test ... silent_payments::scanner::tests::adversarial_ff32_scalar_reduces_unsigned -- --exact` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments`
- **Per wave merge:** `cargo test --release -p chia-sdk-driver --features chip-0057` (whole crate to catch regressions in unrelated chip-0035/chip-0037 tests)
- **Phase gate:** Full workspace suite green (`cargo test --release --workspace --all-features --exclude ...`) before `/gsd:verify-work`. Plus the strict scoped clippy `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`. Plus `cargo fmt --check`. Plus `cargo machete` with zero new ignored entries on `chia-sdk-driver`. Plus the three Phase 1+2 grep bans (`! grep -r 'mod_by_group_order' silent_payments/`, `! grep -rE '^use sha2::' silent_payments/`, `! grep -rE 'Sha256::digest' silent_payments/`).

### Wave 0 Gaps
- [ ] `crates/chia-sdk-driver/src/silent_payments/mod.rs` — module barrel
- [ ] `crates/chia-sdk-driver/src/silent_payments/protocol.rs` — `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`, `compute_shared_secret_from_tweak`
- [ ] `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — `scan_from_tweaks`, `K_MAX_DEFAULT`, all 11 tests
- [ ] `crates/chia-sdk-driver/src/silent_payments/types.rs` — `TweakData`, `OutputMeta`, `DetectedSpCoin`, optional `malformed_pubkey_caught_at_deserialization` test
- [ ] `crates/chia-sdk-driver/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- [ ] `crates/chia-sdk-driver/src/driver_error.rs` — `#[cfg(feature = "chip-0057")] SilentPayment(#[from] SilentPaymentError)` variant
- [ ] `crates/chia-sdk-utils/src/silent_payments/labels.rs` — promote `generate_label` from `pub(super)` to `pub(crate)` AND add a `pub fn generate_label(scan_sk, m)` reach-through in `mod.rs` (§13 Option A)
- [ ] `src/prelude.rs` — extend the `#[cfg(feature = "chip-0057")]` block with the 7 driver re-exports (§2)
- [ ] `.github/workflows/rust.yml` — one new line: `cargo build --release -p chia-sdk-driver -F chip-0057`
- [ ] Test framework: already present. No install command required.

## Open Questions

1. **`generate_label` reach-through scope.** Recommend Option A (`pub(crate)` + public reach-through) per §13. This needs Phase 2 surface modification (single one-line `pub(super)` → `pub` change). Planner should confirm before landing.
   - What we know: Phase 2's `pub(super)` choice was deliberate ("only the `silent_payments::*` module tree can reach it" — `02-PHASE-SUMMARY.md` line 161).
   - What's unclear: whether broadening scope is acceptable to Phase 2's design intent.
   - Recommendation: confirm with a one-line planner question, OR fall back to Option B (duplicate the 10-line implementation in `chia-sdk-driver`) if uncertain.

2. **Bespoke `k=1` test-vector value pinning.** The TV doesn't exist in any reference; Phase 3 must compute it locally (Python or Rust scratchpad) and pin it. The planner must allocate task time for this computation OR commit to deriving the value in the test body itself (e.g., compute `t_1`, derive `onetime_pk_k1`, derive `puzzle_hash_k1`, then construct the `TweakData` from these — no pinned hex literals required, just self-consistent derivation).
   - What we know: the construction is fully determined by TV1's `shared_secret` + `k = 1`.
   - What's unclear: whether to pin hex literals (more rigorous, catches future protocol drift) or to compute-and-feed (less typing, but a bug in `derive_output_tweak` would silently pass the test).
   - Recommendation: pin hex literals. Spend the 15 minutes computing them via a Python one-liner against the reference impl. The resulting test catches more bugs.

3. **`scan_from_tweaks` parameter order / shape.** Phase 3 RESEARCH proposes `(scan_sk, spend_sk, spend_pk, &TweakData, labels, k_max)` matching `sp-client/scanner.rs:58-64`. Alternative: take a `&SilentPaymentKeys` for `(scan_sk, spend_sk, spend_pk)` (Phase 2's bundle type). The bundle is more ergonomic for the wallet author; the raw args are more flexible for a watch-only flow where `spend_sk` is on a hardware device.
   - What we know: Phase 2 ships `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` — even a hardware-split wallet can construct a transient `SilentPaymentKeys` from imported keys.
   - Recommendation: ship BOTH. A free function `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels, k_max)` for the raw flow + a method `SilentPaymentKeys::scan(&self, &TweakData, labels, k_max)` for the bundled flow. The method is a one-line wrapper. Adds minimal surface.

## Sources

### Primary (HIGH confidence)
- `Read` of `crates/chia-sdk-types/src/silent_payments/{mod,scalar,tagged_hash,paths}.rs` (Phase 1 shipped surface — full files read)
- `Read` of `crates/chia-sdk-utils/src/silent_payments/{mod,error,address,keys,labels}.rs` (Phase 2 shipped surface — full files read)
- `Read` of `~/silent-payments/crates/sp-common/src/{protocol,ecdh,puzzle}.rs` (reference protocol primitives)
- `Read` of `~/silent-payments/crates/sp-client/src/scanner.rs` (reference scanner, including tests)
- `Read` of `~/silent-payments/tests/test_vectors.py` (canonical CHIP test vectors — byte-exact values)
- `Read` of `~/silent-payments/chip-silent-payments.md` (CHIP §416 K_max, §446 K_MAX=2400, §459 identity-element guard, §125-§130 labels, §169 ser32 endianness)
- `Read` of `crates/chia-sdk-driver/src/driver_error.rs` (extension point for the new variant)
- `Read` of `crates/chia-sdk-driver/Cargo.toml` lines 18-23 (`chip-0057` feature already declared; deps verified)
- `Read` of `src/prelude.rs` (Phase 3 re-export extension point)
- `Read` of `bindings.json` + `bindings/bls.json` + `bindings/address.json` (Phase 5 design constraint surface)
- `Read` of `.planning/phases/01-crypto-primitives-workspace-integration/01-PHASE-SUMMARY.md` (Phase 1 inheritance)
- `Read` of `.planning/phases/02-address-key-types/02-PHASE-SUMMARY.md` (Phase 2 inheritance + design-decision context)
- `Read` of `.planning/{STATE,ROADMAP,REQUIREMENTS}.md` (project context)

### Secondary (MEDIUM confidence)
- `Read` of `~/silent-payments/shared.py` lines 333-401 (Python prototype `scan_for_silent_payment` — confirms reference Rust scanner is faithful to the Python protocol)
- `Read` of `~/silent-payments/tests/test_protocol.py` lines 240-310 (multi-output / labeled / k-counter protocol coverage — confirms `k=1` is the protocol path, not just a sanity test)
- `Read` of `crates/chia-sdk-driver/src/layers/standard_layer.rs:116-119` (`StandardArgs::curry_tree_hash` SDK convention)

### Tertiary (LOW confidence)
- (none) — every claim in this document either reads a real file or quotes a CHIP spec passage. The `<additional_context>` line "DOS-guard test: 10,000 forged matches → must stop at K_max=32" required interpretation: 32 is a TEST input, NOT the production default; CHIP §446 mandates 2400. Flagged in §3e.

## Metadata

**Confidence breakdown:**
- Type surface (`TweakData`/`OutputMeta`/`DetectedSpCoin` shapes): HIGH — directly modeled on the reference scanner + Phase-5 binding-descriptor constraints.
- Scanner algorithm: HIGH — verbatim from `sp-client/scanner.rs:58-140` with two CHIP-mandated guards added; tested end-to-end in the reference impl.
- Test vectors (TV1, TV3, TV4): HIGH — pinned byte-for-byte from the reference impl's passing test suite.
- Bespoke `k=1` vector: MEDIUM — construction is deterministic, but the specific resulting bytes need to be computed and pinned during Plan 3-01. (No pre-computed value in any reference.)
- DOS-guard test design: MEDIUM — straightforward but the exact construction (forging 10,000 candidates) requires generating per-k expected puzzle hashes. Recommend writing a tiny helper that produces a forged `TweakData` deterministically.
- Identity-element guard correctness: HIGH — `chia-bls::PublicKey::is_inf()` is a one-line check on the underlying `blst_p1_is_inf`; behaviour is unambiguous.

**Research date:** 2026-05-15
**Valid until:** ~2026-08-15 (90 days for the algorithmic content — the CHIP-0057 draft is stable; the chia-bls / chia-puzzle-types APIs are pinned at 0.36.1). If a CHIP-0057 amendment or a chia-bls major bump lands, re-validate §6 and §9.
