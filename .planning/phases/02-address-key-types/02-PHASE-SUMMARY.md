---
phase: 02-address-key-types
subsystem: infra
tags: [phase-closure, chip-0057, silent-payments, address-encoding, key-derivation, label-registry, bech32m, bip39, chia-sdk-utils, ci, addr-01, addr-02, addr-03, addr-04, addr-05, addr-06]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature cascade (root Cargo.toml → chia-sdk-types/-driver/-utils); chia_sdk_types::silent_payments primitives (ScalarField, tagged_hash, CHIA_SP_LABEL, SCAN_PATH, SPEND_PATH); chia_sdk_utils::Bech32 wrapper with Variant::Bech32m enforcement + Bech32Error #[from] propagation; Phase 1 grep bans (mod_by_group_order, ^use sha2::)"

provides:
  - "chia_sdk_utils::silent_payments module gated by chip-0057, containing:"
  - "  • SilentPaymentNetwork enum (Mainnet=spxch, Testnet=tspxch) with hrp() + from_hrp()"
  - "  • SilentPaymentError six-variant enum (WrongHrp, PayloadLength, InvalidPublicKey, IdentityPublicKey, ReservedChangeLabel, Bech32 #[from])"
  - "  • SilentPaymentAddress struct (scan_pk, spend_pk, network) with bech32m encode/decode over 96-byte payload, identity-pubkey rejection via is_inf"
  - "  • SilentPaymentKeys struct (BIP-39 mnemonic → scan/spend SKs at m/12381/8444/{12,13}/0; from_secret_keys watch-only constructor; manual redacting Debug; unlabeled_address/labeled_address builders with m=0 ReservedChangeLabel reject)"
  - "  • LabelRegistry struct (bidirectional u32 ↔ PublicKey via two HashMaps with [u8;48] reverse key; new/register/forward/lookup/len/is_empty/iter)"
  - "  • pub(super) generate_label helper shared between SilentPaymentKeys::labeled_address and LabelRegistry::register"
  - "chip-0057 feature cascade on chia-sdk-utils now activates dep:chia-sdk-types (+ chia-sdk-types/chip-0057), dep:bip39, dep:chia-bls"
  - "Per-crate chip-0057 CI build line for chia-sdk-utils in .github/workflows/rust.yml (WS-02 equivalent for Phase 2)"
  - "Umbrella prelude re-export of the five Phase 2 public types behind #[cfg(feature = \"chip-0057\")]"
  - "27 named #[test] functions pinning CHIP-0057 TV1 / TV3 test vectors and 6 negative-case rejections — all passing under cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments"
  - "Documented Phase-2 final gate pass (5 builds, scoped clippy clean, workspace clippy clean, fmt clean, machete clean, two Phase 1 grep bans hold, 27 silent_payments tests, 2387-test full workspace suite)"

affects:
  - Phase 03 (Receive primitive — consumes SilentPaymentKeys (scan_sk for ECDH), LabelRegistry (label-pk lookup for labeled detection), SilentPaymentAddress (for round-trip tests) from chia_sdk_utils::silent_payments)
  - Phase 04 (Send-side action — consumes SilentPaymentAddress::decode (recipient parsing) and the same crypto primitives Phase 2 imports)
  - Phase 05 (Bindings — exposes the address layer types Phase 2 builds via napi/pyo3/wasm; bindy descriptor will map SilentPaymentAddress + SilentPaymentKeys via bindings/silent_payments.json)
  - Phase 06 (Simulator E2E — exercises the full bech32m round-trip and label-registry round-trip in an integration test against the simulator)

requirements-completed: [ADDR-01, ADDR-02, ADDR-03, ADDR-04, ADDR-05, ADDR-06]

plans:
  - "02-01-PLAN.md — Wiring & feature gate: chip-0057 deps on chia-sdk-utils + silent_payments module barrel (no requirements closed; pure scaffolding)"
  - "02-02-PLAN.md — SilentPaymentError + SilentPaymentNetwork foundational types (no requirements closed; foundation for 02-03/04)"
  - "02-03-PLAN.md — SilentPaymentAddress encode/decode + 12 address tests (closed ADDR-02)"
  - "02-04-PLAN.md — SilentPaymentKeys + LabelRegistry + 15 named tests (closed ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06)"
  - "02-05-PLAN.md — CI matrix + prelude re-export + final gate verification (Phase 2 closure; traceability sweep)"

duration: 59min  # cumulative across the 5 plans: 18 + 8 + 8 + 12 + 13 = 59 min
completed: 2026-05-15
---

# Phase 02: Address & key types — Phase Summary

**Phase 2 lands the CHIP-0057 wallet-side address + key surface in `chia_sdk_utils::silent_payments` behind the existing `chip-0057` workspace feature — `SilentPaymentNetwork` (spxch/tspxch HRPs), `SilentPaymentError` (six-variant enum with `#[from] Bech32Error`), `SilentPaymentAddress` (bech32m round-trip + identity-pubkey rejection), `SilentPaymentKeys` (BIP-39 derivation + watch-only constructor + manual redacting Debug + labeled-address builders), `LabelRegistry` (bidirectional u32 ↔ PublicKey) — then closes Phase 2 with the per-crate `chia-sdk-utils -F chip-0057` CI line, the umbrella prelude re-export of all five public types behind `chip-0057`, and a green 13-expression phase-gate matrix (5 builds, strict + workspace clippy, fmt, machete with zero new ignored entries, both Phase 1 grep bans hold, 27 silent_payments tests, full 2387-test workspace suite green). All six ADDR-* requirements close mechanically.**

## At a Glance

| | |
|---|---|
| **Phase** | 02 — Address & key types |
| **Plans completed** | 5 of 5 |
| **Cumulative duration** | ~59 min execution time (18 + 8 + 8 + 12 + 13) |
| **Files created** | 5 Rust sources (`silent_payments/{mod,error,address,keys,labels}.rs`) + 5 plan SUMMARYs + this phase summary |
| **Files modified** | 4 (`Cargo.toml`, `lib.rs`, `.github/workflows/rust.yml`, `src/prelude.rs`) — all in `chia-sdk-utils` plus umbrella crate |
| **Workspace deps added** | 0 (`chia-sdk-types`, `bip39`, `chia-bls` were already in `[workspace.dependencies]` from Phase 1 baseline) |
| **`[package.metadata.cargo-machete] ignored` entries added** | 0 |
| **Tests added** | 27 (12 address + 7 keys + 8 labels) — all passing |
| **Total workspace tests** | 2387 (2360 Phase 1 baseline + 27 new) — all passing |
| **Requirements closed** | 6 (ADDR-01 through ADDR-06) |

## What Shipped

### Source code (all behind `chip-0057` on `chia-sdk-utils`)

- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (36 lines) — module barrel in final sorted ordering `address < error < keys < labels` with `pub use *::*;` re-exports for each.
- `crates/chia-sdk-utils/src/silent_payments/error.rs` (45 lines) — `SilentPaymentError` six-variant enum: `WrongHrp(String)`, `PayloadLength(usize)`, `InvalidPublicKey`, `IdentityPublicKey`, `ReservedChangeLabel`, `Bech32(#[from] crate::Bech32Error)`. `#[derive(Debug, Error)]`; no `Clone`/`PartialEq` (RESEARCH §7).
- `crates/chia-sdk-utils/src/silent_payments/address.rs` (336 lines) — Two types in one file: `SilentPaymentNetwork` enum (Mainnet=spxch, Testnet=tspxch) with `hrp()` + `from_hrp()`; `SilentPaymentAddress { scan_pk, spend_pk, network }` struct with `#[derive(Clone, Debug, PartialEq, Eq)]` (no Copy — PublicKey is not Copy in `chia-bls` 0.36.1's struct shape), `new()` trusted constructor, `encode()` / `decode()` methods. Bech32m via `chia_sdk_utils::Bech32` wrapper (never direct `bech32::` imports). 12 named `#[test]` functions in `address::tests`: 4 TV1 positive (round-trip + encode-pinned, both networks), 2 TV3 labeled-positive (encode-pinned, both networks), 6 negative-case rejections.
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` (250 lines) — `SilentPaymentKeys { scan_sk, spend_sk, scan_pk, spend_pk }` private fields with `#[derive(Clone)]` + manual redacting `impl Debug` (SKs as `"<redacted>"`). Public surface: `from_mnemonic(&Mnemonic)` (empty-passphrase BIP-39 seed → `SecretKey::from_seed` → `derive_path` for `SCAN_PATH`/`SPEND_PATH`), `from_secret_keys(scan_sk, spend_sk)` (watch-only constructor), four `&`-returning accessors (`scan_sk`, `spend_sk`, `scan_pk`, `spend_pk`), `unlabeled_address(network) -> SilentPaymentAddress`, `labeled_address(network, m) -> Result<SilentPaymentAddress, SilentPaymentError>` (m=0 returns `Err(ReservedChangeLabel)`; otherwise `B_m = &spend_pk + &label_pk(m)`). 7 named `#[test]` functions in `keys::tests`: 4 TV1 `from_mnemonic_*_matches` (scan_sk/spend_sk/scan_pk/spend_pk), `from_secret_keys_matches_from_mnemonic` (parity), `from_secret_keys_tv1_mainnet_pinned`, `labeled_address_zero_rejected`.
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` (228 lines) — `pub(super) fn generate_label(scan_sk, m) -> (ScalarField, PublicKey)` shared helper computing the CHIP-0057 §125-§130 label scalar via `tagged_hash(CHIA_SP_LABEL, ser256(b_scan) || ser32(m))` + the `.expect("ScalarField::from_bytes_unsigned guarantees value < r")` invariant message. `LabelRegistry { forward: HashMap<u32, PublicKey>, reverse: HashMap<[u8; 48], u32> }` bidirectional registry with `new`/`register`/`forward`/`lookup`/`len`/`is_empty`/`iter`. 8 named `#[test]` functions in `labels::tests`: `tv3_label_scalar_matches`, `tv3_label_pk_matches`, `tv3_labeled_spend_pk_matches`, `labels_preserve_scan_pk` (cross-file via `super::super::{SilentPaymentKeys, SilentPaymentNetwork}`), `registry_round_trip`, `registry_three_labels`, `registry_lookup_missing`, `registry_forward_missing`.

### Config / CI

- `crates/chia-sdk-utils/Cargo.toml` — `chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057", "dep:bip39", "dep:chia-bls"]` activates the three optional deps AND cascades the feature into `chia-sdk-types` so `use chia_sdk_types::silent_payments::*` resolves under per-crate builds. Three corresponding `optional = true` dep lines added.
- `crates/chia-sdk-utils/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;` reserves the umbrella access path.
- `.github/workflows/rust.yml` — added one line `cargo build --release -p chia-sdk-utils -F chip-0057` in the "Build individual crates" step (immediately after the no-features `chia-sdk-utils` line, with 10-space indent matching the surrounding `cargo build` lines).
- `src/prelude.rs` — added `#[cfg(feature = "chip-0057")] pub use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork};` after the existing `chia_sdk_utils::{Address, Bech32, parse_hex, select_coins}` re-export.

### Tests

| Test | Module | What it pins |
|---|---|---|
| `tv1_mainnet_round_trip` | address | TV1 mainnet decode + re-encode equals original `spxch1...` |
| `tv1_testnet_round_trip` | address | TV1 testnet decode + re-encode equals original `tspxch1...` |
| `tv1_mainnet_encode_pinned` | address | from raw TV1 hex pubkeys, encode equals Python-reference mainnet string |
| `tv1_testnet_encode_pinned` | address | from raw TV1 hex pubkeys, encode equals Python-reference testnet string |
| `tv3_mainnet_labeled_pinned` | address | TV3 (labeled scan_pk = TV1 + B_m as spend_pk) mainnet encode-pinned |
| `tv3_testnet_labeled_pinned` | address | TV3 labeled testnet encode-pinned |
| `decode_xch_hrp_rejected` | address | Bech32m-valid xch1... rejected with `WrongHrp("xch")` |
| `decode_invalid_checksum_rejected` | address | corrupted checksum → `Err(Bech32(...))` |
| `decode_bech32_not_bech32m_rejected` | address | Bitcoin SegWit V0 (bech32, not bech32m) → variant-mismatch error |
| `decode_short_payload_rejected` | address | < 96 bytes → `PayloadLength(...)` |
| `decode_identity_scan_pk_rejected` | address | scan_pk = identity → `IdentityPublicKey` or `InvalidPublicKey` (permissive matches!) |
| `decode_identity_spend_pk_rejected` | address | spend_pk = identity → `IdentityPublicKey` or `InvalidPublicKey` (permissive matches!) |
| `from_mnemonic_tv1_scan_sk_matches` | keys | TV1 12-word mnemonic → `scan_sk` matches pinned `132567e4...690f6` |
| `from_mnemonic_tv1_spend_sk_matches` | keys | TV1 mnemonic → `spend_sk` matches pinned `53d140b3...31b087` |
| `from_mnemonic_tv1_scan_pk_matches` | keys | TV1 mnemonic → `scan_pk` matches pinned 48-byte serialization |
| `from_mnemonic_tv1_spend_pk_matches` | keys | TV1 mnemonic → `spend_pk` matches pinned 48-byte serialization |
| `from_secret_keys_matches_from_mnemonic` | keys | watch-only construction with TV1 SKs returns identical pubkeys to mnemonic path |
| `from_secret_keys_tv1_mainnet_pinned` | keys | TV1 SKs → `unlabeled_address(Mainnet).encode()` matches pinned `spxch1...` |
| `labeled_address_zero_rejected` | keys | `labeled_address(network, 0)` returns `Err(ReservedChangeLabel)` |
| `tv3_label_scalar_matches` | labels | `generate_label(b_scan, 1).0.to_bytes()` matches pinned 32-byte TV3 scalar |
| `tv3_label_pk_matches` | labels | `generate_label(b_scan, 1).1.to_bytes()` matches pinned 48-byte TV3 label_pk |
| `tv3_labeled_spend_pk_matches` | labels | `B_spend + label_pk` matches pinned TV3 labeled spend_pk |
| `labels_preserve_scan_pk` | labels | cross-file invariant: labeled and unlabeled addresses share the same `scan_pk` |
| `registry_round_trip` | labels | `register` then `forward(m).unwrap()` equals registered label_pk |
| `registry_three_labels` | labels | three independent labels register and retrieve correctly |
| `registry_lookup_missing` | labels | `lookup(unknown_label_pk_bytes)` returns `None` |
| `registry_forward_missing` | labels | `forward(unknown_m)` returns `None` |

All 27 pass under `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments`. Per-module counts: 12 address + 7 keys + 8 labels = 27.

## Phase Gate — Final Status

| ID | Description | Status |
|----|-------------|--------|
| G1 | `cargo build -p chia-sdk-utils` (no features) succeeds | PASS |
| G2 | `cargo build -p chia-sdk-utils -F chip-0057` succeeds | PASS |
| G3 | `cargo build -p chia-sdk-utils --all-features` succeeds | PASS |
| G4 | `cargo build --workspace` (no features) succeeds | PASS |
| G5 | `cargo build --workspace --all-features` succeeds | PASS |
| G6 | `clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean | PASS |
| G7 | `clippy --workspace --all-features --all-targets` clean | PASS (CI invocation; 2 pre-existing pedantic warnings in chia-sdk-daemon carry forward from Phase 1) |
| G8 | `cargo fmt --check` clean | PASS |
| G9 | `cargo machete` clean, no new `ignored` entries | PASS (zero new `[package.metadata.cargo-machete]` entries since Phase 1 baseline) |
| G10 | `grep -r 'mod_by_group_order' silent_payments/` zero matches | PASS (Phase 1 ban holds) |
| G11 | `grep -rE '^use sha2::' silent_payments/` zero matches | PASS (Phase 1 defense-in-depth ban holds) |
| G12 | 27 silent_payments tests pass | PASS (12 address + 7 keys + 8 labels) |
| G13 | Full workspace test suite (CI excludes-list) passes | PASS (2387 passed, 0 failed, 0 ignored) |

## ROADMAP Phase 2 Success Criteria — Final Status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | CHIP test-vector unlabeled round-trip | PASS (`tv1_*_round_trip` + `tv1_*_encode_pinned`) |
| 2 | CHIP test-vector labeled round-trip | PASS (`tv3_*_labeled_pinned` + `from_mnemonic_tv1_*_matches` derivation cascade) |
| 3 | Negative cases | PASS (6 named `decode_*_rejected` tests covering wrong HRP, invalid checksum, bech32-not-bech32m, short payload, identity scan_pk, identity spend_pk) |
| 4 | `from_secret_keys` parity | PASS (`from_secret_keys_matches_from_mnemonic` + `from_secret_keys_tv1_mainnet_pinned`) |
| 5 | `labeled_address(0)` rejection | PASS (`labeled_address_zero_rejected` asserts `Err(ReservedChangeLabel)`) |
| 6 | `LabelRegistry` round-trip | PASS (`registry_round_trip` + `registry_three_labels` + `registry_lookup_missing` + `registry_forward_missing`) |

## Requirements Closed

- **ADDR-01** — `SilentPaymentKeys::from_mnemonic` derives scan + spend SKs at `m/12381/8444/12/0` and `m/12381/8444/13/0` using `bip39 = 2.2.0` + `chia-bls::SecretKey::from_seed` + `derive_unhardened`. Closed in Plan 02-04, verified by 4 TV1 `from_mnemonic_tv1_*_matches` tests.
- **ADDR-02** — `SilentPaymentAddress` encode/decode (bech32m, HRP `spxch`/`tspxch`, 96-byte `B_scan || B_spend` payload). Closed in Plan 02-03, verified by 4 TV1 positive tests + 6 negative tests.
- **ADDR-03** — `SilentPaymentKeys::labeled_address(m)` produces `B_spend + label_pk(m)`. Closed in Plan 02-04, verified by `tv3_labeled_spend_pk_matches` and `labels_preserve_scan_pk`.
- **ADDR-04** — `LabelRegistry` bidirectional `label_pk ↔ label_index`. Closed in Plan 02-04, verified by 4 registry tests.
- **ADDR-05** — `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` watch-only constructor. Closed in Plan 02-04, verified by `from_secret_keys_matches_from_mnemonic` parity test.
- **ADDR-06** — `labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)`. Closed in Plan 02-04, verified by `labeled_address_zero_rejected`.

## Key Decisions

(See per-plan SUMMARYs for full rationale; this list aggregates the cross-cutting calls.)

1. **No new `[workspace.dependencies]` entries.** `chia-sdk-types`, `bip39`, `chia-bls` were already in `[workspace.dependencies]` from Phase 1; Phase 2 only added them as `optional = true` `[dependencies]` entries inside `crates/chia-sdk-utils/Cargo.toml` activated by the `chip-0057` feature.
2. **`chip-0057` feature cascade discipline.** The `chia-sdk-utils` `chip-0057` feature line activates both the optional dep (`dep:chia-sdk-types`) AND the dep's own chip-0057 feature (`chia-sdk-types/chip-0057`). Without the second, per-crate `-p chia-sdk-utils -F chip-0057` builds fail E0432 on `use chia_sdk_types::silent_payments::*`. This was a Rule 3 cleanup in Plan 02-04 — the missing cascade prevented the plan from compiling at all. Pattern documented for future per-crate-feature builds.
3. **Same-file stub-then-append pattern.** Plan 02-02 shipped `address.rs` as a 41-line stub holding only `SilentPaymentNetwork`. Plan 02-03 appended `SilentPaymentAddress` + 12 tests to the same file (336 lines total) without renaming or splitting. Reduces churn on `mod.rs` (the `pub use address::*;` re-export already covers anything added) and keeps the type group co-located from the moment the network discriminant exists.
4. **Bech32m via existing wrapper, never direct.** `SilentPaymentAddress::encode/decode` delegate to `chia_sdk_utils::Bech32::{new, encode, decode}` rather than importing the `bech32` crate. Grep-checked at Plan 02-03 acceptance (`! grep -E '^use bech32::' address.rs`). Inherits the wrapper's `Variant::Bech32m` enforcement and the `#[from]` propagation of `bech32::Error` through `Bech32Error → SilentPaymentError`.
5. **Identity-pubkey rejection ordering: parse → is_inf → reject.** NOT relying on `PublicKey::from_bytes` to fail on infinity (RESEARCH §13 Pitfall 5). Empirically confirmed: `chia-bls` 0.36.1's `PublicKey::from_bytes(&[0xc0, 0, ..., 0])` succeeds and `pk.is_inf()` returns true. The defensive ordering catches it. Permissive `matches!(Err(IdentityPublicKey | InvalidPublicKey))` in the two negative tests, since both satisfy CHIP §215 ("implementations MUST reject the point at infinity") equally.
6. **Manual redacting Debug for secret material.** `SilentPaymentKeys` uses an explicit `impl Debug` that prints the public keys normally but replaces the two secret-key fields with `"<redacted>"`. Auto-deriving Debug would call through to `chia_bls::SecretKey::Debug` which leaks the hex bytes — a privacy regression that pedantic clippy would not catch. The wallet-author-facing safety net.
7. **Shared `pub(super) generate_label` helper.** `keys.rs::labeled_address` and `labels.rs::LabelRegistry::register` both call the same helper. Co-located in `labels.rs` (the registry's home), exposed via `pub(super)` so only the `silent_payments::*` module tree can reach it. A future CHIP-0057 spec amendment to the label-scalar construction lands in one place and both callers update atomically.
8. **PublicKey is `Copy` in `chia-bls` 0.36.1.** Pedantic clippy's `clippy::clone_on_copy` fires on `.clone()` calls in 5 sites across `keys.rs` and `labels.rs` tests. All fixed structurally by removing `.clone()` (or dereferencing `&PublicKey` returns from `LabelRegistry::forward` via `*`). No `#[allow(...)]` attributes anywhere in `silent_payments/`.
9. **`labels_preserve_scan_pk` cross-file test via `super::super::`.** Test lives in `silent_payments::labels::tests::` per 02-VALIDATION.md row 62 lock, but constructs a `SilentPaymentKeys` from `keys.rs` via the `super::super::` reach. Demonstrates Phase 2's same-module-tree split supports cross-file tests without splitting into a top-level `silent_payments/tests/` directory. Phase 3 will reuse this pattern for scan-side tests in a new `scanner.rs`.

## Carried-forward Observations / Follow-ups

Phase 1 carry-overs remain open (out of Phase 2 scope):

### 1. Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is a misnomer (Phase 1 carry-forward)
- **Source:** Phase 1 Plan 01-02, re-verified passing at Plan 02-05 (still part of the full 2387-test suite).
- **Disposition:** Still NOT a blocker. The test value pinned is mathematically correct; only the name is misleading. Suggest a small follow-up rename (4-line PR) before or during Phase 3 entry.

### 2. Pre-existing chia-sdk-daemon clippy::pedantic warnings (Phase 1 carry-forward)
- **Source:** Phase 1 Plan 01-05; re-confirmed at Phase 2 Plan 02-05 workspace clippy step.
- **Location:** `crates/chia-sdk-daemon/src/client.rs:426-427` (`match_same_arms` + `match_wildcard_for_single_variants`).
- **Disposition:** Still NOT a Phase 2 blocker. CI's existing clippy step (without `-D warnings`) exits 0; scoped clippy on `chia-sdk-utils` is clean under `-D warnings`.

No NEW Phase 2 carry-overs.

## Files Inventory

### Created (Rust source)
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (36 lines)
- `crates/chia-sdk-utils/src/silent_payments/error.rs` (45 lines, incl. no tests — error variants exercised by address/keys tests)
- `crates/chia-sdk-utils/src/silent_payments/address.rs` (336 lines, incl. 12 named tests)
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` (250 lines, incl. 7 named tests)
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` (228 lines, incl. 8 named tests)
- **Total:** 895 lines across 5 source files

### Modified
- `crates/chia-sdk-utils/Cargo.toml` — `chip-0057` feature line promoted to activate `dep:chia-sdk-types`, `chia-sdk-types/chip-0057`, `dep:bip39`, `dep:chia-bls`; three `optional = true` dep lines added
- `crates/chia-sdk-utils/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- `.github/workflows/rust.yml` — one line: `cargo build --release -p chia-sdk-utils -F chip-0057`
- `src/prelude.rs` — `#[cfg(feature = "chip-0057")] pub use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork};`

### Phase planning artifacts
- `02-01-SUMMARY.md`, `02-02-SUMMARY.md`, `02-03-SUMMARY.md`, `02-04-SUMMARY.md`, `02-05-SUMMARY.md`
- `02-PHASE-SUMMARY.md` (this file)
- `02-RESEARCH.md` + `02-VALIDATION.md` (carried from planning)

## Plans Index

| Plan | Title | Duration | Source Commits | SUMMARY |
|------|-------|----------|----------------|---------|
| 02-01 | Wiring & feature gate — chip-0057 deps on chia-sdk-utils + silent_payments module barrel | 18 min | `3553e9d8`, `81b8cf1e`, + docs `8d2e86c5` | `02-01-SUMMARY.md` |
| 02-02 | SilentPaymentError + SilentPaymentNetwork foundational types | 8 min | `7f920eb7`, `94c74267`, `d3f6e2f8`, + docs `38945afc` | `02-02-SUMMARY.md` |
| 02-03 | SilentPaymentAddress encode/decode + 12 address tests | 8 min | `9eb57817`, `ffb97dee`, + docs `45f5d108` | `02-03-SUMMARY.md` |
| 02-04 | SilentPaymentKeys + LabelRegistry + 15 named tests | 12 min | `f6826d1c`, `46f79c0f`, `4a92ccba`, + docs `cd2146a0` | `02-04-SUMMARY.md` |
| 02-05 | CI matrix + prelude re-export + final gate verification | 13 min | `2aacf626`, + final docs commit | `02-05-SUMMARY.md` |

## Phase Transition: Phase 3 Readiness

**Inputs Phase 3 inherits from Phase 2 (in addition to all Phase 1 inputs):**

- `chia_sdk_utils::silent_payments::SilentPaymentKeys` — Phase 3's receive primitive consumes `scan_sk` for ECDH on the input tweak public key and `spend_pk` for the per-output one-time-puzzle-hash construction.
- `chia_sdk_utils::silent_payments::LabelRegistry` — Phase 3's `scan_from_tweaks` uses this to attribute labeled detections back to their `m` label index for wallet UI display.
- `chia_sdk_utils::silent_payments::SilentPaymentAddress` — Phase 3's integration tests will construct one via `SilentPaymentKeys::unlabeled_address`/`labeled_address` and exercise full round-trip detection against a known-tweak fixture.
- `chia_sdk_utils::silent_payments::SilentPaymentError` — Phase 3 extends with new variants for receive-side errors (e.g., `InvalidTweakData`, `UnknownLabel`); the `#[from] Bech32Error` chain already in place gives ergonomic conversion.
- The `chip-0057` feature on `chia-sdk-utils` cascades to `chia-sdk-types/chip-0057` already (fixed in Plan 02-04). Phase 3 will need to extend the same cascade pattern when it lands chip-0057-gated code on `chia-sdk-driver`.
- The umbrella prelude already re-exports the five Phase 2 types; Phase 3 adds `TweakData`, `DetectedSpCoin`, `scan_from_tweaks` to the same block when those types land.
- 27 silent_payments tests on `chia-sdk-utils` + 10 silent_payments tests on `chia-sdk-types` = 37 silent_payments tests baseline; Phase 3 builds on top of this.
- CI's `chia-sdk-utils -F chip-0057` build line (this plan) exercises the full Phase 2 surface on every commit; Phase 3 inherits this safety net.

**Outstanding for Phase 3 entry (per ROADMAP / RESEARCH consolidated open questions):**

- **Phase 3 (entry):** Tweak-data transport format pinning — the receive primitive accepts a transport-agnostic `TweakData` input, but the exact shape (per RESEARCH §X) needs to be locked before the primitive's public API is committed. CHIP-0058 dependency.
- **Phase 4 (entry):** Option A vs B for deferred ECDH in `SilentPaymentSend` (Q1) — still open.
- **Phase 4 (entry):** `SyntheticSecretKey` newtype vs documented `&[SecretKey]` parameter (Q2) — still open.

**Optional pre-Phase-3 housekeeping (carry-forwards from Phase 1):**

- Patch the `from_bytes_unsigned_max_input_reduces_to_r_minus_one` test name + VALIDATION row + ROADMAP success criterion 2 wording (Phase 1 observation #1).
- Open a 2-line chore PR to fix the pre-existing chia-sdk-daemon pedantic lints (Phase 1 observation #2).

Both are non-blocking for Phase 3 entry.

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
*Status: ALL 5 PLANS COMPLETE — READY FOR PHASE 3*
