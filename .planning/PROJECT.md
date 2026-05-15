# CHIP-0057 Silent Payments — chia-wallet-sdk Integration

## What This Is

Wallet-facing support for [CHIP-0057 silent payments](https://github.com/Chia-Network/chips) inside `chia-wallet-sdk`. Silent payments let a recipient publish one static `spxch1...` address (and labeled sub-addresses) while every payment on chain lands at a fresh, unlinkable one-time puzzle hash derived via ECDH on BLS12-381. This work adds the cryptographic primitives, address types, send-side driver code, transport-agnostic receive primitive, and bindings exposure needed for wallets like Sage to display silent-payment addresses and send XCH to one.

Adapted from [BIP-352](https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki). Reference implementation lives at `~/silent-payments` (Python prototype + an `sp-common` / `sp-service` / `sp-client` Rust workspace) — only `sp-common`'s wallet-side primitives map into the SDK; `sp-service` and `sp-client` are external infrastructure that consume the SDK, not part of it.

## Core Value

A wallet developer can derive a silent-payment address from a mnemonic, display it, send XCH to a silent-payment address, and (once a CHIP-0058 tweak-data source exists) detect incoming silent payments — without re-implementing any cryptography and through the same idiomatic Layer/Primitive/Spends/bindings surface the SDK already uses for everything else.

## Requirements

### Validated

(None yet — ship to validate)

### Active

**Address & key derivation**
- [ ] **ADDR-01**: `SilentPaymentKeys` derives scan + spend secret keys from a BIP-39 mnemonic (paths `m/12381/8444/12/0` and `m/12381/8444/13/0`)
- [ ] **ADDR-02**: `SilentPaymentAddress` encodes/decodes bech32m with HRP `spxch` (mainnet) / `tspxch` (testnet) over the 96-byte `scan_pk || spend_pk` payload
- [ ] **ADDR-03**: `SilentPaymentKeys::labeled_address(m)` produces labeled sub-addresses where `B_spend` is replaced by `B_spend + label_pk(m)`; the scan key is unchanged across labels
- [ ] **ADDR-04**: Wallet maintains a `label_pk → label_index` lookup so labeled detections can be attributed

**Send side (XCH)**
- [ ] **SEND-01**: `derive_one_time_puzzle_hash` computes the recipient's per-payment puzzle hash from `(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k)`
- [ ] **SEND-02**: `compute_input_hash` computes the BIP-352 input hash using the lexicographically smallest spent coin id and the aggregated synthetic sender public key
- [ ] **SEND-03**: `aggregate_sender_sks` aggregates synthetic secret keys across all wallet-controlled inputs of a transaction (single-party only — multi-party flows must aggregate at sign time)
- [ ] **SEND-04**: A `SilentPaymentSend` action composes with the existing `Spends` action system so wallet code can `spends.add(SilentPaymentSend { recipient, amount, memos })` and have the signer/standard-layer path produce a correctly-signed spend bundle

**Receive side (transport-agnostic primitive)**
- [ ] **RECV-01**: `TweakData { tweak_points, outputs }` is the transport-agnostic input type for the scanner — same shape that a CHIP-0058 light-wallet protocol or today's `sp-service` would supply
- [ ] **RECV-02**: `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels)` returns `Vec<DetectedSpCoin>` with `onetime_sk`, `k`, optional `label`, and the matching coin metadata
- [ ] **RECV-03**: `compute_shared_secret_from_tweak(scan_sk, tweak_point)` is the cheap wallet-side ECDH primitive (one scalar-multiply + SHA-256 per group)
- [ ] **RECV-04**: Labeled detection: when an unlabeled `k` candidate doesn't match, the scanner tries each registered `label_pk` and surfaces the labeled `(onetime_sk, label_index)` match

**Cryptographic primitives** (gated by `chip-0057` feature)
- [ ] **CRYPTO-01**: `ScalarField` newtype performs **unsigned** mod-r reduction over the BLS12-381 subgroup order — distinct from the existing signed `mod_by_group_order` used for synthetic-key offsets
- [ ] **CRYPTO-02**: `tagged_hash(tag, data)` implements the BIP-340-style tagged hash with `Chia_SP/Inputs`, `Chia_SP/SharedSecret`, `Chia_SP/Label` domain tags
- [ ] **CRYPTO-03**: All CHIP test vectors from `chip-silent-payments.md` (TV1 unlabeled, TV3 labeled, multi-input vectors) pass as Rust unit tests

**Bindings**
- [ ] **BIND-01**: `bindings/silent_payments.json` descriptor + `chia-sdk-bindings::silent_payments` facade expose address generation (`SilentPaymentKeys::from_mnemonic`, `unlabeled_address`, `labeled_address`, `SilentPaymentAddress::encode`/`decode`) through the bindy macro
- [ ] **BIND-02**: Same descriptor exposes the send-side primitives (`derive_one_time_puzzle_hash` and the `SilentPaymentSend` action) and the receive primitive (`scan_from_tweaks`, `TweakData`)
- [ ] **BIND-03**: AVA tests (`napi/__test__/`, `wasm/__test__/`) and pytest tests (`pyo3/tests/`) cover the full address-gen + send + scan-from-tweaks round trip from each target language

**Workspace integration**
- [ ] **WS-01**: New `chip-0057` workspace feature in root `Cargo.toml` cascades to `chia-sdk-types/chip-0057`, `chia-sdk-driver/chip-0057`, `chia-sdk-utils/chip-0057` (and bindings always-on)
- [ ] **WS-02**: `.github/workflows/rust.yml` builds each affected crate individually with `-F chip-0057` and `--all-features`
- [ ] **WS-03**: All crate code under `chip-0057` compiles cleanly under workspace lint policy (deny clippy::all, warn pedantic, deny unsafe_code, deny dead_code) and passes `cargo machete`

**Simulator integration**
- [ ] **SIM-01**: Test helper (in `chia-sdk-test` or under the driver's `#[cfg(test)]`) generates `TweakData` from a simulator block by collecting that block's standard-puzzle spends, extracting their synthetic pubkeys, computing per-spend `tweak_point = input_hash * A_sum`, and pairing with the block's outputs
- [ ] **SIM-02**: End-to-end test against `chia-sdk-test::Simulator` demonstrates: sender wallet sends XCH to a silent-payment address → block farmed → tweak-data helper produces a `TweakData` → recipient wallet's `scan_from_tweaks` finds the coin → recipient signs and spends the detected coin
- [ ] **SIM-03**: Same end-to-end flow exists for a labeled-address recipient (verifies labeled detection works with real on-chain coins)

**Example**
- [ ] **EX-01**: `examples/silent_payment.rs` shows the full flow runnable against the simulator (mirrors how `examples/cat_spends.rs` and `examples/spend_simulator.rs` are structured)

### Out of Scope

- **CAT2 send-side** — Deferred to v2. CAT2 silent-payment sends are constructively possible today (the CAT layer's `morph_condition` wraps the bare one-time puzzle hash) but **recipients cannot detect them** until the indexer (and CHIP-0058) gain layer-aware extraction of inner puzzle hashes from CAT-wrapped outputs. Shipping CAT2 in v1 would produce undetectable payments; that's a footgun we'd rather not put in the API.
- **NFT silent payments** — Deferred. NFT singletons have lineage and metadata constraints that make sender construction high-complexity, and the privacy benefit is weaker (launcher_id is permanent and trackable across spends).
- **Light-wallet transport client** — Deferred to a follow-up that lands once CHIP-0058 is defined. Today's `sp-service` WS wire format (`ServerMessage::BlockData { height, tweaks, outputs }`) is a prototype; baking it into the SDK now would freeze the SDK on a pre-CHIP format. We ship the transport-agnostic `TweakData` type so wallets can implement adapters today and swap to a CHIP-0058 client later.
- **`SilentPaymentTweakSource` async trait** — Deferred with the transport client. Adding the trait without a real consumer would lock in an async shape (tokio, futures-util) that CHIP-0058 may not match.
- **Bulk-scanning the chain** — Out of scope for the SDK entirely; that's `sp-service`'s job. Wallets don't decompress generators or extract sender pubkeys; they consume tweak data.
- **GCS block-filter prefilter** — `sp-service` ships a GCS filter for cheap "could this block contain anything?" checks. Implementation-defined and not yet a CHIP — defer until CHIP-0058 specifies it.
- **Hardware-wallet scan-key custody UX** — The CHIP §10 discusses cold-storage of `b_spend` with `b_scan` online; the SDK exposes both keys separately so wallet authors can split them, but doesn't enforce or design that workflow.
- **A new top-level crate** — Silent payments code lives in `chia-sdk-types`, `chia-sdk-driver`, `chia-sdk-utils` modules behind `chip-0057`, mirroring `chip-0035` and `chip-0037`. A standalone `chia-sdk-silent-payments` crate may emerge later if the transport client lands.
- **Replacing the prototype's `puzzle_hash_for_pk`** — The SDK already has `chia_puzzle_types::standard::StandardArgs::curry_tree_hash(pk.derive_synthetic())`. Reuse it.

## Context

**Reference implementation**: `~/silent-payments` contains:
- A Python prototype (`shared.py`, `generate_address.py`, `send_payment.py`, `scan_coin.py`, `spend_coin.py`, `scanner.py`) — testnet-only.
- A Rust workspace with three crates:
  - `sp-common` — wallet-side protocol (mnemonic → scan/spend SKs, tagged_hash, `ScalarField`, ECDH, input_hash, output tweak, one-time PK/SK, label generation, multi-input aggregation, send/scan helpers). **This is what gets folded into the SDK.**
  - `sp-service` — server-side indexer (axum WS, block reader, generator zstd decompression, sqlite tweak index). Stays external; eventually becomes the reference CHIP-0058 server.
  - `sp-client` — sqlite-backed scanner CLI talking to `sp-service`. Stays external; can simplify post-SDK to a thin `TweakData` adapter.
- The draft `chip-silent-payments.md` spec (829 lines) including ECDH derivation, address encoding, labeling, security analysis, and test vectors (TV1 unlabeled, TV3 labeled).
- A feasibility doc (`docs/cat2_nft1_feasibility.md`) explaining why CAT2/NFT receive needs indexer support.

**Codebase context** (`.planning/codebase/*.md`):
- Rust 1.90.0, edition 2024, strict workspace lints (deny clippy::all, warn pedantic, deny unsafe_code, deny dead_code, `cargo machete` in CI).
- CHIP feature pattern already established by `chip-0035` (vault/MIPS) and `chip-0037` (EIP-712/controller puzzle, just landed in commit `bbc7f57f`) — feature lives on the workspace root and cascades to `chia-sdk-types/chip-XXXX` + `chia-sdk-driver/chip-XXXX`.
- Bindings are generated by `bindy-macro` from `bindings/*.json` descriptors. `napi/index.d.ts`, `napi/index.js`, and pyo3 stubs are produced artifacts — never hand-edit them.
- `chia-sdk-driver` houses Layer/Primitive code and the `Spends` action system, but also non-layer modules like `clear_signing` and `hashed_ptr` — silent-payments fits the latter category since outputs are bog-standard p2 coins on tweaked pubkeys.
- `chia-sdk-utils` already provides `Address` / `Bech32` for the standard `xch`/`txch` address — the silent-payment address is a different HRP and a 96-byte payload, same family.
- `chia-sdk-test::Simulator` is the canonical integration-test substrate; many driver primitives test against it via `chia-sdk-test::Simulator`.
- `bip39::Mnemonic` is already a workspace dep and is used in `chia-sdk-bindings/src/mnemonic.rs` and `chia-sdk-test/src/key_pairs.rs` — no new dep for mnemonic handling.

**Cryptographic subtleties to design around** (from `.planning/codebase/CONCERNS.md`):
- **Signed vs unsigned scalar reduction.** Existing SDK uses signed `mod_by_group_order` for synthetic-key offsets. CHIP-0057 requires **unsigned** reduction for `input_hash`, `output_tweak`, `label_scalar`. Mixing them silently breaks detection. The `ScalarField` newtype must enforce this at the type level.
- **Synthetic vs raw keys.** The sender's `sender_sk` in BIP-352 terms is the synthetic SK (the one curried into the standard puzzle, visible in the puzzle reveal — that's what scanners extract). The signer already produces synthetic SKs; the silent-payments code consumes those, not raw wallet keys.
- **Multi-input aggregation.** `aggregate_sender_sks` requires controlling all inputs of a transaction. For offers / multi-party bundles, aggregation must happen at sign time across parties. The SDK API must not silently fall back to single-input aggregation when partial control is detected — that would produce undetectable payments.

## Constraints

- **Tech stack**: Rust 1.90.0 (pinned in `rust-toolchain.toml`), edition 2024. Cannot use nightly-only features. Cryptographic implementations must compile under `unsafe_code = "deny"`.
- **Dependencies**: No new workspace deps. `bip39`, `num-bigint`, `hex`, `sha2`, `chia-bls`, `chia-puzzle-types`, `clvm-utils` are all already in `[workspace.dependencies]`. Bumping `chia-protocol` (0.36.1) or `chia-puzzles` (0.20.3) is out of scope for this work.
- **CI gates**: Workspace-level clippy (`deny clippy::all`, `warn pedantic`, `warn cargo`), `cargo machete` (unused-deps), `cargo fmt --check`, and per-crate builds with and without `--all-features`. New `chip-0057`-gated code must compile in every CI permutation.
- **Bindings format**: Wire-protocol surfaces (`TweakData`, `DetectedSpCoin`) must be expressible in `bindings/silent_payments.json` so napi/pyo3/wasm get them. Bytes32 + lists of (PublicKey, OutputMeta) is the granularity to design around.
- **Feature flag**: The `chip-0057` workspace feature is the only umbrella for this work. No feature within a feature, no name-aliases.
- **Forward compatibility with CHIP-0058**: The receive-side primitive accepts a transport-agnostic `TweakData` input. Any CHIP-0058 transport client built later must be able to construct `TweakData` from its wire messages without breaking the existing SDK API.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| v1 = send-side + transport-agnostic receive primitive (no WS client) | Today's `sp-service` JSON is a prototype, not a CHIP. Locking the SDK to it would force a breaking change when CHIP-0058 lands. Ship a `TweakData` input type instead so adapters can plug in. | — Pending |
| CAT2 send deferred to v2 | Sender-side works trivially; receiver-side requires indexer-level layer-aware extraction. Shipping CAT2 sends now would produce undetectable payments — a footgun. | — Pending |
| Labels fully supported in v1 (generation + detection) | Cheap to implement on top of unlabeled crypto; matches the CHIP's design intent for payment-source distinction. Skipping creates an asymmetric API. | — Pending |
| `chip-0057` feature flag (not `silent-payments`) | Matches the `chip-0035` / `chip-0037` precedent. The CHIP number is confirmed. | — Pending |
| `ScalarField` is a new SDK newtype (not a reused crate type) | Existing signed `mod_by_group_order` would silently break the protocol. The unsigned reduction needs a distinct type so the compiler enforces correctness. | — Pending |
| Lives in `chia-sdk-types` + `chia-sdk-driver` + `chia-sdk-utils` (no new crate) | Mirrors how `chip-0035` and `chip-0037` integrate. Silent payments are not a transport, not a new puzzle layer — they're a key-derivation + driver action layered on the standard p2. | — Pending |
| Bindings ship in v1, not deferred | Sage and other JS/Python wallets are the primary consumers. Shipping a Rust-only v1 would gate adoption on a separate "bindings phase" with no functional reason. | — Pending |
| Full simulator round-trip (mocked tweak source) is the v1 test target | Test-vectors-only would validate crypto but not the wallet integration shape. The simulator round-trip is the cheapest way to prove the API actually composes with `Spends` + `StandardLayer` + the signer. | — Pending |
| `puzzle_hash_for_pk` from the prototype is dropped in favor of `StandardArgs::curry_tree_hash(pk.derive_synthetic())` | The SDK already has the helper. Re-shipping it would be redundant and would risk drift. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-15 after initialization*
