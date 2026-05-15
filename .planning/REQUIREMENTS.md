# Requirements — CHIP-0057 Silent Payments v1

Source of truth for scoped v1 requirements. Phase mapping is filled in by the roadmap (`ROADMAP.md`).

## v1 Requirements

### Address & Key Derivation (`chia-sdk-utils::silent_payments`)

- [ ] **ADDR-01** — `SilentPaymentKeys` derives scan + spend secret keys from a BIP-39 mnemonic at paths `m/12381/8444/12/0` and `m/12381/8444/13/0` using existing `bip39 = 2.2.0` and `chia-bls::SecretKey::from_seed` / `derive_unhardened`.
- [ ] **ADDR-02** — `SilentPaymentAddress` encodes and decodes a bech32m string with HRP `spxch` (mainnet) / `tspxch` (testnet) over the 96-byte `serialize(B_scan) || serialize(B_spend)` payload. Round-trips against the CHIP test vectors.
- [ ] **ADDR-03** — `SilentPaymentKeys::labeled_address(m)` produces a labeled sub-address where `B_spend` is replaced by `B_spend + label_pk(m)`; the scan key is unchanged across labels.
- [ ] **ADDR-04** — `LabelRegistry` (dedicated type, not a bare `HashMap`) maintains a `label_pk → label_index` lookup so labeled detections in `scan_from_tweaks` can be attributed back to their label index.
- [ ] **ADDR-05** — `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` constructor enables watch-only / key-import flows without re-deriving from a mnemonic.
- [ ] **ADDR-06** — `labeled_address(0)` is rejected (or hard-errors with a typed `SilentPaymentError::ReservedChangeLabel`). The CHIP designates `m=0` as the change label, never to be exposed as a public address.

### Send Side (`chia-sdk-driver::silent_payments` + `actions/silent_payment_send.rs`)

- [ ] **SEND-01** — `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) -> Bytes32` computes the recipient's per-payment puzzle hash; produces a standard p2 puzzle hash on the tweaked one-time public key.
- [ ] **SEND-02** — `compute_input_hash(coin_ids, sender_pk_aggregated) -> ScalarField` uses the lexicographically smallest spent coin ID (byte-string lex order) and the aggregated synthetic sender public key, per `tagged_hash("Chia_SP/Inputs", coin_id_min || serialize(A_sum))`.
- [ ] **SEND-03** — `aggregate_sender_sks(sks)` aggregates synthetic secret keys across all wallet-controlled inputs of a single transaction. Returns a `ScalarField`. Hard-errors (no silent fallback) when partial control is detected — multi-party flows must aggregate at sign time across parties.
- [ ] **SEND-04** — `SilentPaymentSend` action composes with the existing `Spends` action system. `spends.add(SilentPaymentSend { recipient, amount, memos })` records the deterministic pieces during `apply()` and defers ECDH to a `Spends::finish_with_silent_payment_keys(synthetic_pk_map, synthetic_sk_map)` overload (Option A from architecture research, subject to a phase-4 proof-of-concept).
- [ ] **SEND-05** — `SilentPaymentSend` accepts `Vec<Recipient>` so a single transaction can produce multiple silent-payment outputs sharing one `input_hash`. The `k` counter (per recipient scan_pk) is maintained by `Spends` so multiple sends in one batch increment correctly.
- [ ] **SEND-06** — Multi-input sends across multiple wallet key indices emit the SendMessage / ReceiveMessage announcements (opcodes 60 / 61) that bind the inputs together, so the recipient's `compute_input_hash` matches the sender's. Without this, cross-index sends are undetectable.
- [ ] **SEND-07** — Memo-position hint guard: the standard Chia wallet promotes a 32-byte memo at position 0 to a puzzle-hash hint, publishing the one-time PH to every indexer and defeating silent-payment privacy. `SilentPaymentSend` rejects or rewrites this memo shape; the API surface makes the hazard hard to hit by accident.
- [ ] **SEND-08** — Doc-comment privacy warnings on every memo-bearing API noting that memos are on-chain and visible to anyone holding the recipient's scan key.

### Receive Side — Transport-Agnostic Primitive (`chia-sdk-driver::silent_payments`)

- [ ] **RECV-01** — `TweakData { tweak_points, outputs }` is the transport-agnostic input type for the scanner. Decoupled from any specific wire format so a future CHIP-0058 transport client (or today's `sp-service` adapter) can construct it without breaking the SDK API.
- [ ] **RECV-02** — `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels) -> Vec<DetectedSpCoin>` returns `(coin, onetime_sk, k, label: Option<u32>)` for each detection. Implements the BIP-352 k-iteration: try `k=0,1,2,...` and stop at the first miss (with the labeled-termination rule below).
- [ ] **RECV-03** — `compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` is the cheap wallet-side ECDH primitive (one scalar-multiply + SHA-256 per spend group). Used internally by `scan_from_tweaks` and exposed for callers that want to compute shared secrets manually.
- [ ] **RECV-04** — Labeled detection: when an unlabeled candidate at `k` misses, the scanner tries each registered `label_pk`. Termination rule: break the `k` loop only when *neither* unlabeled *nor* any labeled candidate matches at the current `k`. This matches `sp-client/scanner.rs`.
- [ ] **RECV-05** — A `K_max` per-spend-group iteration cap (configurable, sensible default ~32) prevents DOS by an adversarial tweak source that would otherwise force unbounded scanning.

### Cryptographic Primitives (`chia-sdk-types::silent_payments`, gated by `chip-0057`)

- [x] **CRYPTO-01** — `ScalarField` newtype performs **unsigned** mod-r reduction over the BLS12-381 subgroup order using `num-bigint 0.4.6`. Distinct from the existing signed `mod_by_group_order` (in `chia-puzzle-types::derive_synthetic`); the type boundary is the prevention mechanism for the signed-vs-unsigned mixing hazard. No public constructor that accepts signed bytes.
- [x] **CRYPTO-02** — `tagged_hash(tag: &str, data: &[u8]) -> [u8; 32]` implements the BIP-340-style tagged hash `SHA256(SHA256(tag) || SHA256(tag) || data)` using `chia-sha2` (not bare `sha2`, per SDK convention). Domain tag constants `Chia_SP/Inputs`, `Chia_SP/SharedSecret`, `Chia_SP/Label` exposed as `&'static str` consts.
- [ ] **CRYPTO-03** — All CHIP test vectors from `chip-silent-payments.md` pass as Rust unit tests: TV1 unlabeled single-input, TV3 labeled single-input, TV4 multi-input aggregation. Plus a **bespoke `k = 1` test vector** generated and added to catch `ser32(k)` endianness bugs that TV1/TV3/TV4 (all `k=0`) cannot detect. Plus an **adversarial `[0xff; 32]` scalar test** that fails on signed reduction but passes on unsigned reduction.

### Bindings (`chia-sdk-bindings` + `bindings/silent_payments.json` + `napi`/`pyo3`/`wasm`)

- [ ] **BIND-01** — `bindings/silent_payments.json` descriptor + `chia-sdk-bindings::silent_payments` facade expose `SilentPaymentKeys` (from_mnemonic, from_secret_keys, scan_pk, spend_pk, unlabeled_address, labeled_address) and `SilentPaymentAddress` (encode, decode, fields) through the `bindy-macro`.
- [ ] **BIND-02** — Same descriptor exposes the send-side primitives (`SilentPaymentSend`, `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`) and the receive primitive (`scan_from_tweaks`, `TweakData`, `DetectedSpCoin`, `LabelRegistry`). Verify `bindy-macro`'s support for static-only function classes; fall back to distributing methods onto carrier types if unsupported.
- [ ] **BIND-03** — AVA tests under `napi/__test__/` and `wasm/__test__/` plus pytest under `pyo3/tests/` cover the full address-gen + send + scan-from-tweaks round trip from each target language. Test that `Vec<chia_bls::PublicKey>` (a new bindings shape introduced by `TweakData::tweak_points`) marshals correctly to each target.

### Workspace Integration

- [x] **WS-01** — A new `chip-0057` workspace feature in the root `Cargo.toml` cascades to `chia-sdk-types/chip-0057`, `chia-sdk-driver/chip-0057`, `chia-sdk-utils/chip-0057`. Bindings feature-default `chip-0057` so bindings ship enabled. Mirrors the `chip-0037` template (commit `bbc7f57f`).
- [ ] **WS-02** — `.github/workflows/rust.yml` adds per-crate builds with `-F chip-0057` matching the existing chip-0035/chip-0037 lines. The standard `--all-features` build already covers the cumulative case.
- [ ] **WS-03** — All `chip-0057`-gated code compiles cleanly under the workspace lint policy (`deny clippy::all`, `warn pedantic`, `warn cargo`, `deny unsafe_code`, `deny dead_code`) and passes `cargo machete`. No new entries in any crate's `[package.metadata.cargo-machete] ignored` list.

### Simulator Integration (`chia-sdk-test`)

- [ ] **SIM-01** — Test helper (`chia-sdk-test::silent_payments::tweak_data_from_simulator_block` or similar, behind `chip-0057`) generates `TweakData` from a simulator block by collecting that block's standard-puzzle spends, extracting their synthetic pubkeys, computing per-spend `tweak_point = input_hash * A_sum`, and pairing with the block's outputs. Lives in the public test crate so binding test suites can reach it via the public API.
- [ ] **SIM-02** — End-to-end test against `Simulator`: sender wallet sends XCH to an unlabeled silent-payment address → block farmed → `tweak_data_from_simulator_block` → recipient `scan_from_tweaks` finds the coin → recipient derives `onetime_sk` and spends the detected coin.
- [ ] **SIM-03** — Same end-to-end flow with a **labeled** recipient address to verify labeled detection works against a real on-chain coin.

### Example

- [ ] **EX-01** — `examples/silent_payment.rs` shows the full send → scan → spend flow runnable against the simulator. Structure mirrors `examples/cat_spends.rs` and `examples/spend_simulator.rs`.

## v2 Requirements (deferred)

- CAT2 send-side (recipient detection requires CHIP-0058 indexer support)
- CAT2 receive-side (requires layer-aware extraction of inner puzzle hashes from CAT-wrapped outputs)
- NFT silent payments (singleton lineage + metadata constraints, weaker privacy due to permanent launcher_id)
- Light-wallet transport client (waits for CHIP-0058 wire-format specification)
- `SilentPaymentTweakSource` async trait (lands with the transport client)
- GCS block-filter prefilter (waits for CHIP-0058 specification)
- Offers / multi-party silent-payment sends (multi-party aggregation requires sign-time coordination across parties)

## Out of Scope (permanently)

- Bulk-scanning the chain — that's `sp-service` / CHIP-0058-server territory, not the SDK.
- Block generator decompression — same.
- Sender pubkey extraction from puzzle reveals — same.
- Hardware-wallet scan-key custody UX — wallet-layer concern; SDK exposes both keys separately so wallet authors can split them.
- A new top-level crate (`chia-sdk-silent-payments`) — silent-payments code lives in existing crates behind `chip-0057`, mirroring `chip-0035` / `chip-0037`. A standalone crate may emerge later if a transport client lands.
- Replacing the existing `StandardArgs::curry_tree_hash` + `derive_synthetic` flow with a hand-rolled `puzzle_hash_for_pk` — re-use what the SDK already has.

## Traceability

Phase mapping assigned by `ROADMAP.md` (2026-05-15).

| REQ-ID | Phase | Notes |
|--------|-------|-------|
| ADDR-01 | Phase 2 | Mnemonic → SKs at fixed CHIP paths |
| ADDR-02 | Phase 2 | bech32m round-trip against CHIP test vectors |
| ADDR-03 | Phase 2 | Labeled-address constructor |
| ADDR-04 | Phase 2 | `LabelRegistry` bidirectional map |
| ADDR-05 | Phase 2 | Watch-only constructor |
| ADDR-06 | Phase 2 | m=0 change-label hard-error |
| SEND-01 | Phase 4 | `derive_one_time_puzzle_hash` |
| SEND-02 | Phase 4 | `compute_input_hash` with lex-min coin_id |
| SEND-03 | Phase 4 | Multi-party hard-error (no silent fallback) |
| SEND-04 | Phase 4 | Phase-4 proof-of-concept resolves Option A vs B for deferred ECDH |
| SEND-05 | Phase 4 | Multi-output `Vec<Recipient>` with per-scan_pk `k` counter |
| SEND-06 | Phase 4 | Opcode 60/61 announcement binding — closer to correctness bug fix than feature |
| SEND-07 | Phase 4 | Memo-position hint guard — Chia-specific |
| SEND-08 | Phase 4 | Doc-only privacy warnings |
| RECV-01 | Phase 3 | Transport-agnostic `TweakData` |
| RECV-02 | Phase 3 | `scan_from_tweaks` k-iteration with labeled-termination rule |
| RECV-03 | Phase 3 | `compute_shared_secret_from_tweak` ECDH primitive |
| RECV-04 | Phase 3 | Labeled k-termination rule |
| RECV-05 | Phase 3 | `K_max` DOS guard |
| CRYPTO-01 | Phase 1 | Signed-vs-unsigned scalar prevention — foundational |
| CRYPTO-02 | Phase 1 | Tagged-hash + tag constants |
| CRYPTO-03 | Phase 3 | TV1/TV3/TV4 + bespoke k=1 + adversarial `[0xff;32]` |
| BIND-01 | Phase 5 | Address/keys descriptor + facade |
| BIND-02 | Phase 5 | Send/receive primitives in descriptor; verify bindy-macro static-functions schema first |
| BIND-03 | Phase 6 | Cross-language E2E (uses Phase-6 simulator helper) |
| WS-01 | Phase 1 | `chip-0057` workspace feature cascade |
| WS-02 | Phase 1 | CI per-crate `-F chip-0057` lines |
| WS-03 | Phase 1 | Lint policy verified across all gated code |
| SIM-01 | Phase 6 | `tweak_data_from_simulator_block` helper |
| SIM-02 | Phase 6 | Unlabeled E2E |
| SIM-03 | Phase 6 | Labeled E2E |
| EX-01 | Phase 6 | `examples/silent_payment.rs` |

---
*Last updated: 2026-05-15 after roadmap creation*
