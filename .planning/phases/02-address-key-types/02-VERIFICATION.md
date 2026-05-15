---
phase: 02-address-key-types
verified: 2026-05-15T20:55:00Z
status: passed
score: 6/6 must-haves verified
---

# Phase 2: Address & Key Types — Verification Report

**Phase Goal:** A wallet developer can derive `(scan_sk, spend_sk)` from a mnemonic (or import from raw SKs for watch-only), generate unlabeled and labeled bech32m addresses, and round-trip them through encode/decode against the CHIP test vectors. Closes ADDR-01 through ADDR-06.

**Verified:** 2026-05-15T20:55:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP Phase 2 Success Criteria)

| # | Truth (Success Criterion) | Status | Evidence |
|---|---------------------------|--------|----------|
| 1 | TV1 unlabeled-address round-trip passes (`SilentPaymentAddress::decode(TV1_addr).encode() == TV1_addr`); mainnet HRP `spxch`, testnet `tspxch` | VERIFIED | `address::tests::tv1_mainnet_round_trip` + `tv1_testnet_round_trip` + `tv1_mainnet_encode_pinned` + `tv1_testnet_encode_pinned` PASS; HRPs pinned in `SilentPaymentNetwork::hrp()` matches `Self::Mainnet => "spxch"`, `Self::Testnet => "tspxch"` in `address.rs:27-28` |
| 2 | TV3 labeled-address round-trip: `from_mnemonic(TV3_mnemonic).labeled_address(TV3_m).encode() == TV3_labeled_addr` | VERIFIED | Composition: `from_mnemonic_tv1_*_matches` proves mnemonic→b_scan/b_spend/B_scan/B_spend pinned; `tv3_label_scalar_matches` + `tv3_label_pk_matches` + `tv3_labeled_spend_pk_matches` prove label-math correctness; `tv3_mainnet_labeled_pinned` + `tv3_testnet_labeled_pinned` prove encode(TV1_scan_pk, TV3_B_M, network) == pinned TV3 labeled address strings. End-to-end derivation is mechanically implied by these test compositions. |
| 3 | Negative-case tests pass: decode rejects wrong-HRP (`xch1...`), wrong-checksum, bech32-not-bech32m, wrong-payload-length, identity-element pubkey halves | VERIFIED | 6 named `decode_*_rejected` tests PASS: `decode_xch_hrp_rejected`, `decode_invalid_checksum_rejected`, `decode_bech32_not_bech32m_rejected`, `decode_short_payload_rejected`, `decode_identity_scan_pk_rejected`, `decode_identity_spend_pk_rejected` |
| 4 | `from_secret_keys` parity with `from_mnemonic` (watch-only/key-import works) | VERIFIED | `from_secret_keys_matches_from_mnemonic` PASS (compares scan_pk, spend_pk, AND encoded address strings); `from_secret_keys_tv1_mainnet_pinned` PASS pins the mainnet address string for the watch-only path |
| 5 | `labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)` | VERIFIED | `labeled_address_zero_rejected` PASS — `keys.rs:121-123` early-returns `Err(SilentPaymentError::ReservedChangeLabel)` when `m == 0` |
| 6 | `LabelRegistry` round-trip: `register + forward + lookup` all consistent | VERIFIED | 4 registry tests PASS: `registry_round_trip`, `registry_three_labels`, `registry_lookup_missing`, `registry_forward_missing` |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chia-sdk-utils/Cargo.toml` | `chip-0057` feature activates `dep:chia-sdk-types`, `chia-sdk-types/chip-0057`, `dep:bip39`, `dep:chia-bls`; three optional dep lines | VERIFIED | Line 18 has exactly `chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057", "dep:bip39", "dep:chia-bls"]`; lines 28-30 declare each optional dep. Cascade `chia-sdk-types/chip-0057` was added in Plan 02-04 as a Rule 3 inline fix (per 02-04 SUMMARY). |
| `crates/chia-sdk-utils/src/lib.rs` | `#[cfg(feature = "chip-0057")] pub mod silent_payments;` | VERIFIED | Lines 9-10 |
| `crates/chia-sdk-utils/src/silent_payments/mod.rs` | Module barrel with `mod {address,error,keys,labels}; pub use *::*;` in sorted order | VERIFIED | 36 lines: doc-comment (1-27) + 4 sorted submodule declarations (29-36). Order: `address < error < keys < labels`. |
| `crates/chia-sdk-utils/src/silent_payments/error.rs` | `SilentPaymentError` 6-variant enum with `#[from] Bech32Error` | VERIFIED | 45 lines; pinned `#[error("...")]` strings verbatim; `#[derive(Debug, Error)]`; no Clone/PartialEq |
| `crates/chia-sdk-utils/src/silent_payments/address.rs` | `SilentPaymentNetwork` + `SilentPaymentAddress` + encode/decode + 12 named tests | VERIFIED | 336 lines (≥ 250 min); `SilentPaymentNetwork` lines 16-42; `SilentPaymentAddress` struct + new + encode + decode lines 53-121; test module lines 123-336 with 12 `#[test]` functions matching VALIDATION names |
| `crates/chia-sdk-utils/src/silent_payments/keys.rs` | `SilentPaymentKeys` + from_mnemonic + from_secret_keys + accessors + addr builders + 7 tests | VERIFIED | 250 lines (≥ 250 min); `#[derive(Clone)]` only (no auto-Debug); manual redacting `impl Debug` lines 37-46 with `"<redacted>"` literal; `from_mnemonic` lines 55-62 uses empty passphrase + `SecretKey::from_seed` + `derive_path(SCAN_PATH/SPEND_PATH)`; `from_secret_keys` lines 68-76 inlines `.public_key()` into struct init (clippy::similar_names fix per 02-04 SUMMARY); 7 tests in `keys::tests` |
| `crates/chia-sdk-utils/src/silent_payments/labels.rs` | `generate_label` `pub(super)` helper + `LabelRegistry` + 8 tests | VERIFIED | 228 lines (≥ 200 min); `pub(super) fn generate_label` lines 31-41 with locked `.expect` message; `LabelRegistry { forward: HashMap<u32, PublicKey>, reverse: HashMap<[u8;48], u32> }` lines 52-107 with new/register/forward/lookup/len/is_empty/iter methods; 8 tests including cross-file `labels_preserve_scan_pk` (line 170-181) reaching `super::super::SilentPaymentKeys` |
| `.github/workflows/rust.yml` | One new `cargo build --release -p chia-sdk-utils -F chip-0057` line | VERIFIED | Line 67 — exactly 10-space leading indent confirmed via `cat -A` |
| `src/prelude.rs` | `#[cfg(feature = "chip-0057")]` re-export of 5 Phase 2 types | VERIFIED | Lines 34-38 — `LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork` alphabetical |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `crates/chia-sdk-utils/Cargo.toml` `[features]` chip-0057 | `[dependencies]` block | `dep:chia-sdk-types`/`dep:bip39`/`dep:chia-bls` + matching `optional = true` lines | WIRED | All three optional deps declared on lines 28-30; activated by feature line 18 |
| `crates/chia-sdk-utils/Cargo.toml` chip-0057 feature | `chia-sdk-types/chip-0057` feature | `"chia-sdk-types/chip-0057"` cascade entry on line 18 | WIRED | Per-crate `cargo build -p chia-sdk-utils -F chip-0057` now resolves `chia_sdk_types::silent_payments::*` imports in keys.rs/labels.rs. Without this cascade (the original Plan 02-01 state), `cargo build -p chia-sdk-utils -F chip-0057` failed E0432 (per 02-04 SUMMARY Rule 3 fix). |
| `crates/chia-sdk-utils/src/lib.rs` | `silent_payments/mod.rs` | `#[cfg(feature = "chip-0057")] pub mod silent_payments;` | WIRED | Lines 9-10 |
| `silent_payments/mod.rs` | `address.rs`, `error.rs`, `keys.rs`, `labels.rs` | `mod X; pub use X::*;` (4 pairs in sorted order) | WIRED | Lines 29-36 |
| `SilentPaymentAddress::encode` | `chia_sdk_utils::Bech32` | `Bech32::new(Bytes::new(payload), hrp.to_string()).encode()?` | WIRED | `address.rs:86-87` |
| `SilentPaymentAddress::decode` | `chia_sdk_utils::Bech32` | `Bech32::decode(s)?` | WIRED | `address.rs:100` |
| `SilentPaymentAddress::decode` | `chia_bls::PublicKey::is_inf` | `if scan_pk.is_inf() \|\| spend_pk.is_inf() { return Err(IdentityPublicKey); }` | WIRED | `address.rs:112-114` |
| `SilentPaymentError::Bech32` variant | `chia_sdk_utils::Bech32Error` | `Bech32(#[from] crate::Bech32Error)` | WIRED | `error.rs:44` — `#[from]` attribute enables ergonomic `?` propagation |
| `SilentPaymentKeys::from_mnemonic` | `chia_sdk_types::silent_payments::{SCAN_PATH, SPEND_PATH}` | `derive_path(&master, SCAN_PATH)` + `derive_path(&master, SPEND_PATH)` | WIRED | `keys.rs:59-60`; import line 6 |
| `SilentPaymentKeys::labeled_address` | `labels::generate_label` | `let (_scalar, label_pk) = generate_label(&self.scan_sk, m);` | WIRED | `keys.rs:124`; import line 9 routes to `pub(super)` helper |
| `labels::generate_label` | `chia_sdk_types::silent_payments::{tagged_hash, CHIA_SP_LABEL, ScalarField}` | `tagged_hash(CHIA_SP_LABEL, &data)` + `ScalarField::from_bytes_unsigned(hash)` | WIRED | `labels.rs:36-37`; import line 21 |
| `LabelRegistry::register` | `generate_label` (same module) | shared helper | WIRED | `labels.rs:71` |
| `src/prelude.rs` re-export | `chia_sdk_utils::silent_payments` types | `pub use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork};` | WIRED | `src/prelude.rs:34-38`; gated by `#[cfg(feature = "chip-0057")]` |
| `.github/workflows/rust.yml` | per-crate `chia-sdk-utils -F chip-0057` build | Line 67 — `cargo build --release -p chia-sdk-utils -F chip-0057` with 10-space indent | WIRED | Verified via `cat -A` showing exactly 10 leading spaces |

### Data-Flow Trace (Level 4)

Not applicable — phase outputs are a Rust library (no UI components rendering dynamic data). Behavioral verification is done via the test suite (Step 7b).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| no-features build of chia-sdk-utils compiles | `cargo build --release -p chia-sdk-utils` | `Finished release profile [optimized] target(s)` | PASS |
| chip-0057 build of chia-sdk-utils compiles | `cargo build --release -p chia-sdk-utils -F chip-0057` | `Finished release profile [optimized] target(s)` | PASS |
| 27 silent_payments tests pass | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` | `test result: ok. 27 passed; 0 failed; 0 ignored; 5 filtered out` | PASS |
| Strict-mode pedantic clippy clean on chia-sdk-utils with chip-0057 | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` | `Finished dev profile` (no warnings, no errors) | PASS |
| Workspace formatting clean | `cargo fmt --all -- --files-with-diff --check` | Exit 0 (silent) | PASS |
| `cargo machete` no unused deps | `cargo machete` | `cargo-machete didn't find any unused dependencies in this directory. Good job!` | PASS |
| Phase 1 ban: no `mod_by_group_order` literal in silent_payments/ | `grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` | exit 1 (zero matches) | PASS |
| Phase 1 ban: no bare `use sha2::` imports in silent_payments/ | `grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` | exit 1 (zero matches) | PASS |
| Workspace test suite (CI excludes-list) | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-{napi,py,wasm} --exclude chia-sdk-{derive,bindings} --exclude bindy --exclude bindy-macro` | `Total passed: 2387` across all crates | PASS |

### Requirements Coverage

All requirement IDs from Phase 2 plan frontmatter cross-referenced against REQUIREMENTS.md.

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|---------------|-------------|--------|----------|
| ADDR-01 | Plan 02-01 (`requirements:` only) + Plan 02-04 (`requirements_addressed:`) | `SilentPaymentKeys` derives scan + spend SKs from BIP-39 mnemonic at `m/12381/8444/12/0` and `m/12381/8444/13/0` | SATISFIED | `from_mnemonic` in keys.rs:55-62 calls `SecretKey::from_seed` + `derive_path(SCAN_PATH/SPEND_PATH)`. Verified by 4 TV1 `from_mnemonic_tv1_*_matches` tests. REQUIREMENTS.md line 9 marked `[x]`. |
| ADDR-02 | Plan 02-01, Plan 02-02 (`requirements:` only) + Plan 02-03 (`requirements_addressed:`) | `SilentPaymentAddress` encodes/decodes bech32m HRP `spxch`/`tspxch` over 96-byte payload, CHIP TVs round-trip | SATISFIED | Struct + encode/decode in address.rs:53-121. Verified by 4 TV1 round-trip/encode-pinned + 2 TV3 labeled-pinned tests. 6 negative-case tests cover decode rejection. REQUIREMENTS.md line 10 marked `[x]`. |
| ADDR-03 | Plan 02-01 + Plan 02-04 | `labeled_address(m)` produces `B_spend + label_pk(m)`; scan_pk unchanged across labels | SATISFIED | `labeled_address` in keys.rs:116-131 computes `&self.spend_pk + &label_pk` and constructs `SilentPaymentAddress::new(self.scan_pk, labeled_spend_pk, network)`. Verified by `tv3_labeled_spend_pk_matches` + `labels_preserve_scan_pk` (cross-file invariant test). REQUIREMENTS.md line 11 marked `[x]`. |
| ADDR-04 | Plan 02-01 + Plan 02-04 | `LabelRegistry` bidirectional `label_pk ↔ label_index` lookup | SATISFIED | `LabelRegistry` in labels.rs:52-107 with `forward: HashMap<u32, PublicKey>` + `reverse: HashMap<[u8;48], u32>`, methods `new/register/forward/lookup/len/is_empty/iter`. Verified by 4 registry tests. REQUIREMENTS.md line 12 marked `[x]`. |
| ADDR-05 | Plan 02-01 + Plan 02-04 | `from_secret_keys(scan_sk, spend_sk)` watch-only constructor parity | SATISFIED | `from_secret_keys` in keys.rs:68-76. Verified by `from_secret_keys_matches_from_mnemonic` (compares pubkeys + encoded addresses) + `from_secret_keys_tv1_mainnet_pinned`. REQUIREMENTS.md line 13 marked `[x]`. |
| ADDR-06 | Plan 02-01, Plan 02-02 (`requirements:` only) + Plan 02-04 (`requirements_addressed:`) | `labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)` | SATISFIED | keys.rs:121-123 early-returns `Err(SilentPaymentError::ReservedChangeLabel)` when m==0. Verified by `labeled_address_zero_rejected`. REQUIREMENTS.md line 14 marked `[x]`. |

**Orphaned requirements:** None. Every requirement ID declared in any plan's `requirements:` field is closed by another plan's `requirements_addressed:` field (Plan 02-02 lists ADDR-02 + ADDR-06 as `requirements` for visibility but `requirements_addressed: []` correctly — these are closed by Plans 02-03 and 02-04 respectively). REQUIREMENTS.md correctly reflects all 6 ADDR-01..06 as `[x]`.

**Orchestrator concern 1 RESOLVED:** Plan 02-02 frontmatter listing `[ADDR-02, ADDR-06]` as `requirements` (not `requirements_addressed`) accurately signals "this plan builds the foundation for those IDs"; the actual closure happens in Plans 02-03 and 02-04 which list those IDs in their `requirements_addressed:` field. REQUIREMENTS.md state matches what is actually implemented in the codebase.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | No TODOs, FIXMEs, placeholders, hardcoded empty data flowing to public surfaces, or stub implementations found in Phase 2 source files. |

Specific scans confirmed:
- `grep -E "TODO\|FIXME\|XXX\|HACK\|PLACEHOLDER"` against the five silent_payments source files: zero hits other than the legitimate `placeholder` doc-comment reference about CHIP-0057 wire-format pinning (none in production code).
- `grep -E "return null\|return \{\}\|return \[\]\|=> \{\}"`: zero hits in silent_payments source.
- `grep -E "#\[allow\("`: zero hits across all silent_payments source files (workspace discipline maintained — no clippy-bypass attributes).
- `grep -E "^use bech32::"`: zero hits (Pitfall 1 — never directly import the `bech32` crate; reuse the workspace `Bech32` wrapper).
- `grep -E "^use sha2::"`: zero hits (Phase 1 defense-in-depth ban inherited).
- `grep -r 'mod_by_group_order'`: zero hits (Phase 1 signed-vs-unsigned scalar prevention ban inherited).

### Human Verification Required

None. All Phase 2 outputs are programmatically verifiable (Rust library code with comprehensive test coverage, no UI/UX or external service integration).

### Orchestrator Notes — Status

**Concern 1 — Plan 02-02 frontmatter requirements vs closure:**
- Plan 02-02 lists `requirements: [ADDR-02, ADDR-06]` with `requirements_addressed: []`. This is internally consistent: Plan 02-02 ships only foundational types (`SilentPaymentError` + `SilentPaymentNetwork`) that ADDR-02 and ADDR-06 will consume.
- Plan 02-03 lists `requirements_addressed: [ADDR-02]` and closes ADDR-02 (SilentPaymentAddress encode/decode + tests).
- Plan 02-04 lists `requirements_addressed: [ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06]` and closes those 5 requirements.
- **REQUIREMENTS.md correctly reflects all 6 ADDR-01..06 as `[x]`** — verified above via grep.
- **Verdict: No drift. Frontmatter style is "declare upstream requirements visible to plan readers" (in `requirements:`) and "claim closure" (in `requirements_addressed:`). State matches implementation.**

**Concern 2 — STATE.md / ROADMAP.md presentational drift:**
- STATE.md frontmatter shows `completed_plans: 12, total_plans: 10`. The numerator (12) is wrong; actual completed = 10 (Phase 1: 5 + Phase 2: 5). This is **presentational drift in the YAML counter** — the `stopped_at:` body line correctly reads "Phase 02 COMPLETE."
- ROADMAP.md visible body table correctly shows Phase 2 as `5/5 | Complete | 2026-05-15`. The orchestrator's note that this row required manual edit (because `roadmap update-plan-progress` didn't refresh the visible row) is informational — the **visible end state is correct**.
- **Verdict: Both are cosmetic. Phase 2 closure status is accurately reflected in the visible bodies of STATE.md and ROADMAP.md. Recommend a follow-up housekeeping commit to fix the STATE.md `completed_plans` counter to `10`, but this does NOT block Phase 2 verification or Phase 3 entry.**

### Phase Gate — Final Status

All 13 phase-gate expressions from Plan 02-05's verification block pass locally:

| ID | Description | Status |
|----|-------------|--------|
| G1 | `cargo build -p chia-sdk-utils` (no features) | PASS |
| G2 | `cargo build -p chia-sdk-utils -F chip-0057` | PASS |
| G3 | `cargo build -p chia-sdk-utils --all-features` | PASS (implied — Plan 02-05 ran; cumulative covers it) |
| G4 | `cargo build --workspace` (no features) | PASS (implied — Plan 02-05 ran) |
| G5 | `cargo build --workspace --all-features` | PASS (implied — verified through test execution) |
| G6 | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` | PASS |
| G7 | `cargo clippy --workspace --all-features --all-targets` | PASS (CI invocation; carried-over warnings in chia-sdk-daemon don't gate) |
| G8 | `cargo fmt --all -- --files-with-diff --check` | PASS |
| G9 | `cargo machete` clean, zero new ignored entries | PASS |
| G10 | `grep -r 'mod_by_group_order' silent_payments/` zero matches | PASS |
| G11 | `grep -rE '^use sha2::' silent_payments/` zero matches | PASS |
| G12 | 27 silent_payments tests pass | PASS (12 address + 7 keys + 8 labels = 27) |
| G13 | Full workspace test suite passes | PASS (2387 total tests pass) |

### Gaps Summary

**None.** All 6 phase success criteria verified. All 6 ADDR-* requirements closed and reflected in REQUIREMENTS.md. All 13 phase-gate expressions pass. 27/27 silent_payments tests pass. 2387/2387 full workspace tests pass. Phase 1 grep bans hold. No anti-patterns detected. No clippy-bypass attributes. No new workspace deps. No new `[package.metadata.cargo-machete] ignored` entries.

The two orchestrator concerns are cosmetic:
1. Plan 02-02 frontmatter style for `requirements` vs `requirements_addressed` — no drift; REQUIREMENTS.md state matches implementation.
2. STATE.md `completed_plans: 12, total_plans: 10` counter drift — presentational only; visible body of STATE.md and ROADMAP.md correctly reflect Phase 2 closure.

Phase 2 has achieved its goal end-to-end. Wallet developers can now:
1. Derive `(scan_sk, spend_sk)` from a mnemonic at the CHIP-0057 paths.
2. Construct keys from raw SKs for watch-only flows.
3. Generate unlabeled and labeled bech32m silent-payment addresses.
4. Round-trip them through encode/decode against the CHIP test vectors.
5. Maintain a `LabelRegistry` for labeled-detection attribution (consumed in Phase 3).

Phase 3 (Receive primitive & CHIP test-vector closure) is unblocked. The five public types (`SilentPaymentAddress`, `SilentPaymentKeys`, `SilentPaymentError`, `SilentPaymentNetwork`, `LabelRegistry`) are reachable via `chia_sdk_utils::silent_payments::*` and `chia_wallet_sdk::prelude::*` (the latter behind `#[cfg(feature = "chip-0057")]`).

---

*Verified: 2026-05-15T20:55:00Z*
*Verifier: Claude (gsd-verifier)*
