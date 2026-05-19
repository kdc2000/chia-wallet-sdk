# CHIP-0057 Silent Payments — chia-wallet-sdk Integration

## What This Is

Wallet-facing support for [CHIP-0057 silent payments](https://github.com/Chia-Network/chips) inside `chia-wallet-sdk`. Silent payments let a recipient publish one static `spxch1...` address (and labeled sub-addresses) while every payment on chain lands at a fresh, unlinkable one-time puzzle hash derived via ECDH on BLS12-381. This work adds the cryptographic primitives, address types, send-side driver code, transport-agnostic receive primitive, and bindings exposure needed for wallets like Sage to display silent-payment addresses and send XCH to one.

Adapted from [BIP-352](https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki). Reference implementation lives at `~/silent-payments` (Python prototype + an `sp-common` / `sp-service` / `sp-client` Rust workspace) — only `sp-common`'s wallet-side primitives map into the SDK; `sp-service` and `sp-client` are external infrastructure that consume the SDK, not part of it.

## Core Value

A wallet developer can derive a silent-payment address from a mnemonic, display it, send XCH to a silent-payment address, and (once a CHIP-0058 tweak-data source exists) detect incoming silent payments — without re-implementing any cryptography and through the same idiomatic Layer/Primitive/Spends/bindings surface the SDK already uses for everything else.

## Requirements

### Validated

**Cryptographic primitives** (gated by `chip-0057` feature) — Validated in Phase 1
- [x] **CRYPTO-01**: `ScalarField` newtype performs **unsigned** mod-r reduction over the BLS12-381 subgroup order — distinct from the existing signed reducer used for synthetic-key offsets
- [x] **CRYPTO-02**: `tagged_hash(tag, data)` implements the BIP-340-style tagged hash with `Chia_SP/Inputs`, `Chia_SP/SharedSecret`, `Chia_SP/Label` domain tags

**Workspace integration** — Validated in Phase 1
- [x] **WS-01**: New `chip-0057` workspace feature in root `Cargo.toml` cascades to `chia-sdk-types/chip-0057`, `chia-sdk-driver/chip-0057`, `chia-sdk-utils/chip-0057` (and bindings always-on)
- [x] **WS-02**: `.github/workflows/rust.yml` builds each affected crate individually with `-F chip-0057` and `--all-features`
- [x] **WS-03**: All crate code under `chip-0057` compiles cleanly under workspace lint policy (deny clippy::all, warn pedantic, deny unsafe_code, deny dead_code) and passes `cargo machete`

**Address & key derivation** — Validated in Phase 2
- [x] **ADDR-01**: `SilentPaymentKeys` derives scan + spend secret keys from a BIP-39 mnemonic (paths `m/12381/8444/12/0` and `m/12381/8444/13/0`)
- [x] **ADDR-02**: `SilentPaymentAddress` encodes/decodes bech32m with HRP `spxch` (mainnet) / `tspxch` (testnet) over the 96-byte `scan_pk || spend_pk` payload
- [x] **ADDR-03**: `SilentPaymentKeys::labeled_address(m)` produces labeled sub-addresses where `B_spend` is replaced by `B_spend + label_pk(m)`; the scan key is unchanged across labels
- [x] **ADDR-04**: Wallet maintains a `label_pk → label_index` lookup so labeled detections can be attributed
- [x] **ADDR-05**: `SilentPaymentKeys::from_seed` derives keys from a raw seed (parallel constructor to `from_mnemonic`)
- [x] **ADDR-06**: `labeled_address(0)` is rejected at the API boundary with `ReservedChangeLabel` (label `m = 0` is reserved per CHIP-0057)

**Receive side (transport-agnostic primitive)** — Validated in Phase 3
- [x] **RECV-01**: `TweakData { tweak_points, outputs }` is the transport-agnostic input type for the scanner — same shape that a CHIP-0058 light-wallet protocol or today's `sp-service` would supply
- [x] **RECV-02**: `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels, k_max)` returns `Vec<DetectedSpCoin>` with `onetime_sk`, `k`, optional `label`, and the matching coin metadata (both unlabeled and labeled branches)
- [x] **RECV-03**: `compute_shared_secret_from_tweak(scan_sk, tweak_point)` is the cheap wallet-side ECDH primitive (one scalar-multiply + SHA-256 per group)
- [x] **RECV-04**: Labeled detection: when an unlabeled `k` candidate doesn't match, the scanner tries each registered `label_pk` and surfaces the labeled `(onetime_sk, label_index)` match — with the CHIP §425 k-termination rule (only break the k loop when BOTH unlabeled and every labeled candidate miss)
- [x] **RECV-05**: Bounded compute under adversarial input: `K_MAX_DEFAULT = 2400` (CHIP §446) caps the per-tweak-point inner loop; identity-element tweak points are skipped silently (CHIP §459); 10,000 forged matches at one tweak point still terminate in bounded time

**Cryptographic primitives** (gated by `chip-0057` feature) — Validated in Phase 3
- [x] **CRYPTO-03**: All CHIP test vectors from `chip-silent-payments.md` pass as Rust unit tests — TV1 (unlabeled), TV3 (labeled), TV4 (multi-input) byte-exact; plus bespoke `k=1` (catches `ser32(k)` endianness bugs), `[0xff;32]` adversarial scalar (proves `ScalarField::from_bytes_unsigned` boundary fires end-to-end), labeled k-termination rule, unlabeled-preferred-over-labeled at same k

**Send side (XCH)** — Validated in Phase 4 (SEND-06 re-validated in Phase 4.1; SEND-04 re-validated in Phase 4.2)
- [x] **SEND-01**: `derive_one_time_puzzle_hash` computes the recipient's per-payment puzzle hash from `(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k)` — TV1 / k=0 + k=1 byte-pinned
- [x] **SEND-02**: `compute_input_hash` computes the BIP-352 input hash using the lexicographically smallest spent coin id and the aggregated synthetic sender public key — TV1 byte-pin + lex-min selection + order-independent property tests
- [x] **SEND-03**: `aggregate_sender_sks` aggregates synthetic secret keys across wallet-controlled inputs (single-party only); the multi-party hazard hard-errors at the `Spends` level via `Err(DriverError::SilentPaymentMultiPartyUnsupported)` — multi-party flows must aggregate at sign time
- [x] **SEND-04**: Silent-payment send composes with the existing `Spends` action system; `spends.apply(&[Action::send(Id::Xch, SendDestination::SilentPayment(recipient), amount, memos)])` followed by `spends.with_silent_payment_keys(synthetic_pks, synthetic_sks)` + `spends.finish_with_keys(...)` produces a `SpendBundle` whose outputs land at the same puzzle hash `derive_one_time_puzzle_hash` returns (byte-for-byte round-trip test). *(Phase 4 originally landed via a dedicated `Action::silent_payment_send` + `Spends::finish_with_silent_payment_keys` overload; Phase 04.2 unified the surface into `Action::send` + `Spends::finish_with_keys` per the `SendDestination` enum refactor.)*
- [x] **SEND-05**: Two `SilentPaymentSend` actions targeting the same `scan_pk` in one batch produce outputs at `k=0` and `k=1` (per-scan_pk counter on `Spends`); distinct `scan_pk`s use independent counters
- [x] **SEND-06**: Multi-input sends across multiple wallet key indices are bound together via the SDK's existing `Relation::AssertConcurrent` cycle (opcode-64 closed cycle, one strongly connected component spanning every non-ephemeral coin), so the recipient's `compute_input_hash` matches the sender's. *(Phase 04.1 refactor: replaced the SP-specific opcode 60/61 announcement helper with the SDK's general cycle binding to remove the SP-uniquely-identifiable on-chain fingerprint. Enforced at runtime via `DriverError::SilentPaymentRequiresInputBinding` when ≥2 non-ephemeral XCH inputs are present but `Relation != AssertConcurrent`.)*
- [x] **SEND-07**: Memo-hint guard hard-errors with `Err(DriverError::SilentPaymentMemoHintForbidden)` when the first memo atom is exactly 32 bytes (puzzle-hash-hint deanonymization hazard); 1-byte sentinel + 32-byte payload bypasses the guard as the wallet-author escape hatch
- [x] **SEND-08**: Every public memo-bearing API in `crates/chia-sdk-driver/src/silent_payments/` and `actions/silent_payment_send.rs` carries `/// Privacy warning: memos are stored on-chain in plaintext and are visible to anyone holding the recipient's scan key.` — verified by `grep -L 'Privacy warning'` gate

**On-chain fingerprint** — Validated in Phase 4.1
- [x] **FINGERPRINT-01**: Multi-input SP transactions emit no SP-specific on-chain marker. The send path uses the SDK's general-purpose `Relation::AssertConcurrent` cycle binding (opcode 64, used pervasively across the SDK for atomic multi-coin bundles), not a SP-unique opcode 60/61 empty-message announcement. SP transactions disappear into the SDK's regular-traffic anonymity set. Enforced via grep gates (`emit_silent_payment_announcements` absent; `create_coin_announcement|assert_coin_announcement|announcement_id` absent in `silent_payments/send_keys.rs`) and a runtime gate that hard-errors `Relation != AssertConcurrent` for multi-input SP sends.

**Public action surface** — Validated in Phase 4.2
- [x] **ACTION-API-01**: Unified send-action public surface — a single `Action::send(id: Id, destination: impl Into<SendDestination>, amount: u64, memos: Memos)` constructor serves both regular puzzle-hash destinations and chip-0057 silent-payment destinations. SP-specific code is internal; no public `silent_payment_send` factory remains. `impl From<Bytes32> for SendDestination` keeps all 28 existing `Action::send(id, puzzle_hash, ...)` Rust callers (12 in Sage, 16 in SDK) compiling unchanged. Enforced at runtime via `DriverError::SilentPaymentRequiresXch` (SP destination + non-XCH `Id`) and `DriverError::SilentPaymentKeysNotRegistered` (SP destination applied without prior `Spends::with_silent_payment_keys`).

**Bindings (Rust facade + JSON descriptor)** — Validated in Phase 5
- [x] **BIND-01**: `bindings/silent_payments.json` descriptor + `chia-sdk-bindings::silent_payments` facade expose address generation (`SilentPaymentKeys::from_mnemonic`, `unlabeled_address`, `labeled_address`, `SilentPaymentAddress::encode`/`decode`) through the bindy macro. The full 11-entry descriptor + 428-line facade ship with chip-0057 unconditional on chia-sdk-driver/utils/types dependencies (no facade-side cfg gates). `ScalarField` exposed as a bindy class with `from_bytes_unsigned` only (D-03) so the unsigned-mod-r invariant survives FFI. napi/pyo3/wasm crates all build cleanly; generated `napi/index.d.ts` exposes the 5 SC1 SP types + 4 statics. AVA round-trip test (TV1 abandon×11+about mnemonic, mainnet + testnet) asserts `scan_pk`/`spend_pk` byte-equality across encode→decode.
- [x] **BIND-02**: Same descriptor exposes the send-side primitives (`scan_from_tweaks`, `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`) on a zero-field `SilentPayments` namespace class via bindy's native static_functions support (SC4 pre-flight gate PASSED — no fallback needed). `SendDestination` opaque-handle class added to `bindings/action_system.json` per D-04 with `puzzleHash(Bytes32)`/`silentPayment(SilentPaymentAddress)` factories + `is_*`/`as_*` introspectors mirrored from the `Id` precedent; `Action.send` signature rewired from `puzzleHash: Bytes32` → `destination: SendDestination`; `Spends.with_silent_payment_keys` exposed end-to-end so wallets can register SP keys before `Spends.prepare`. Descriptor↔facade drift audit script confirms 22 methods on both sides, zero drift.

**Simulator integration** — Validated in Phase 6
- [x] **SIM-01**: `chia-sdk-test::silent_payments::tweak_data_from_simulator_block(&Simulator, height) -> TweakData` (behind `chip-0057`) reaches the block's standard-puzzle spends via two new public `Simulator::block_spends(height)` + `block_outputs(height)` accessors (NOT chip-0057 gated — general-purpose surface), groups them by tx, computes `tweak_point = input_hash * A_sum` per group, applies the identity-element skip rule (CHIP §459), and pairs with the block's outputs as `OutputMeta`. Two defensive unit tests pin empty-block + out-of-range behavior.
- [x] **SIM-02**: `test_simulator_e2e_unlabeled` in `crates/chia-sdk-driver/src/silent_payments/e2e.rs` passes — sender → farm → extract → scan → detect (exactly one `DetectedSpCoin`, `label: None`) → spend via `StandardLayer::new(detected.onetime_sk.derive_synthetic().public_key())`. Simulator accepts the follow-on spend.
- [x] **SIM-03**: `test_simulator_e2e_labeled` passes (`labeled_address(SilentPaymentNetwork::Mainnet, 1)` → `label: Some(1)` detection). `test_simulator_e2e_m0_self_change` passes with the **RESEARCH §3b redesign**: it does NOT assume the SDK auto-emits an `m=0` self-change output (the original D-04 assumption was disproven by inspecting `Action::send` / `sp_finish_branch`); instead it verifies `LabelRegistry::register(scan_sk, 0)` is internally consistent and an unlabeled send still detects as `label: None`, citing the `labels.rs:14-16` doc-comment.

**Bindings (cross-language E2E)** — Validated in Phase 6
- [x] **BIND-03**: `Simulator.tweakDataFromBlock(height)` exposed on the existing `chia-sdk-bindings::Simulator` facade (unconditional — `chia-sdk-bindings` adds `"chip-0057"` to its `chia-sdk-test` dep features per D-03; no facade-side cfg gates). One new entry in `bindings/simulator.json`. Three sibling cross-language E2E tests run the unlabeled send + farm + scan + spend flow: `napi/__test__/silent_payments_e2e.spec.ts` (AVA), `pyo3/tests/test_silent_payments.py::test_unlabeled_e2e` (pytest, no conftest per D-08), `wasm/__test__/silent_payments.spec.ts` (AVA + `setPanicHook()`). First runtime proof that `Vec<chia_bls::PublicKey>` on `TweakData.tweakPoints` marshals correctly across all 3 FFI boundaries. Auto-fixed Wave-4 finding: added `pub fn finish_silent_payments(...)` on `chia_sdk_driver::Spends` (chip-0057 gated, no-op when no SP pending) and wired binding's `prepare` to call it so cross-language SP sends actually farm the SP coin. pyo3 `.pyi` stubs regenerated via `cargo run -p pyo3-stub-generator` to pick up Phase 5/6 SP types.

**Example** — Validated in Phase 6
- [x] **EX-01**: `examples/silent_payment.rs` (119 lines, within the 80–120 D-09 budget) demonstrates mnemonic → `SilentPaymentKeys::from_mnemonic` → unlabeled + `labeled_address(1)` Mainnet addresses → two `Action::send` calls in one tx → farm → `tweak_data_from_simulator_block` → `scan_from_tweaks` (detects both: one `label: None` at k=0, one `label: Some(1)` at k=1) → spends both via `StandardLayer::new(detected.onetime_sk.derive_synthetic().public_key())`. Five staged `println!` markers print a readable trace. Builds in CI (`cargo build --examples --all-features`); runs cleanly. Mirrors `cat_spends.rs` rhythm.

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
| v1 = send-side + transport-agnostic receive primitive (no WS client) | Today's `sp-service` JSON is a prototype, not a CHIP. Locking the SDK to it would force a breaking change when CHIP-0058 lands. Ship a `TweakData` input type instead so adapters can plug in. | — **Validated in Phase 6**: `TweakData` carries no transport fields; the example uses only `tweak_data_from_simulator_block`, never a WS client. Cross-cutting #5 grep gates in plans 06-04 + 06-05 enforce zero `chip[-_]0058|websocket|ws_client|sp_service|sp_client` references. |
| CAT2 send deferred to v2 | Sender-side works trivially; receiver-side requires indexer-level layer-aware extraction. Shipping CAT2 sends now would produce undetectable payments — a footgun. | — **Reaffirmed at v1 close**: v1 ships only XCH SP send + receive primitive. CAT2 stays in Out of Scope. |
| Labels fully supported in v1 (generation + detection) | Cheap to implement on top of unlabeled crypto; matches the CHIP's design intent for payment-source distinction. Skipping creates an asymmetric API. | — Generation validated in Phase 2 (Plan 02-04 `LabelRegistry` + `labeled_address`); detection validated in Phase 3 (Plan 03-04 — `pub(crate)` reach-through + scanner labeled branch + TV3 + labeled k-termination); end-to-end labeled detection on real on-chain coins validated in Phase 6 (`test_simulator_e2e_labeled` + example demonstrates `label: Some(1)` round-trip). |
| `chip-0057` feature flag (not `silent-payments`) | Matches the `chip-0035` / `chip-0037` precedent. The CHIP number is confirmed. | — Validated in Phase 1 (workspace) + Phase 2 (`chia-sdk-utils/chip-0057` cascades to `chia-sdk-types/chip-0057`) + **Phase 6** (cascade extended to `chia-sdk-test/chip-0057`; chia-sdk-bindings adds `features = ["chip-0057"]` to its chia-sdk-test dep). |
| `ScalarField` is a new SDK newtype (not a reused crate type) | Existing signed `mod_by_group_order` would silently break the protocol. The unsigned reduction needs a distinct type so the compiler enforces correctness. | — Validated in Phase 1 |
| Lives in `chia-sdk-types` + `chia-sdk-driver` + `chia-sdk-utils` (no new crate) | Mirrors how `chip-0035` and `chip-0037` integrate. Silent payments are not a transport, not a new puzzle layer — they're a key-derivation + driver action layered on the standard p2. | — Validated through Phase 3 (`chia-sdk-types` houses primitives; `chia-sdk-utils` houses address/key/label surface; `chia-sdk-driver/src/silent_payments/` houses receive primitive + `SilentPaymentScan` trait) + **Phase 6** (test helper lives in `chia-sdk-test/src/silent_payments/`; still no new top-level crate). |
| Bindings ship in v1, not deferred | Sage and other JS/Python wallets are the primary consumers. Shipping a Rust-only v1 would gate adoption on a separate "bindings phase" with no functional reason. | — Validated in Phase 5 (BIND-01 + BIND-02): chia-sdk-bindings::silent_payments facade + bindings/silent_payments.json descriptor + SendDestination opaque-handle class in action_system.json all ship; napi/pyo3/wasm all build cleanly. **Phase 6 closes BIND-03**: cross-language E2E tests pass on all 3 binding targets; first runtime proof that `Vec<chia_bls::PublicKey>` marshals correctly. |
| Full simulator round-trip (mocked tweak source) is the v1 test target | Test-vectors-only would validate crypto but not the wallet integration shape. The simulator round-trip is the cheapest way to prove the API actually composes with `Spends` + `StandardLayer` + the signer. | — **Validated in Phase 6**: three Rust E2E tests pass (`test_simulator_e2e_unlabeled` + `_labeled` + `_m0_self_change`); D-04's original m=0 self-change assumption was disproven by code inspection and the test was redesigned per RESEARCH §3b to verify `LabelRegistry::register(scan_sk, 0)` consistency instead. |
| m=0 sub-test redesign | The original D-04 assumption (sender==recipient auto-emits an `m=0` self-change output detectable as `label: Some(0)`) was disproven by direct inspection of `Action::send` + `sp_finish_branch`. The SDK does NOT auto-emit self-change. | — **Validated in Phase 6** by `test_simulator_e2e_m0_self_change`: the test now demonstrates `LabelRegistry::register(scan_sk, 0)` is internally consistent and an unlabeled detection still resolves to `label: None`, matching the `crates/chia-sdk-utils/src/silent_payments/labels.rs:14-16` doc-comment. The actual self-change mechanism, if/when added later, is a v2 concern. |
| `puzzle_hash_for_pk` from the prototype is dropped in favor of `StandardArgs::curry_tree_hash(pk.derive_synthetic())` | The SDK already has the helper. Re-shipping it would be redundant and would risk drift. | — **Validated through Phase 6**: every spending callsite (3 Rust E2E tests + 3 cross-language tests + example) uses `detected.onetime_sk.derive_synthetic()` feeding `StandardLayer::new(pk)`. Pitfall #6 enforced. |

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
*Last updated: 2026-05-19 after Phase 6 (Simulator round-trip + bindings E2E + example) complete — **v1 of CHIP-0057 silent payments is requirement-complete.** All 32 v1 requirements are `[x]` in `.planning/REQUIREMENTS.md`. Phase 6 landed: `chia-sdk-test::silent_payments::tweak_data_from_simulator_block(&Simulator, height) -> TweakData` (SIM-01) + three Rust E2E tests covering unlabeled / labeled / m=0 LabelRegistry consistency (SIM-02 + SIM-03) + cross-language E2E tests on napi/pyo3/wasm proving `Vec<chia_bls::PublicKey>` FFI marshaling correctness (BIND-03) + `examples/silent_payment.rs` (119 lines) (EX-01). chip-0057 cascade extended onto chia-sdk-test (Plan 06-01) and onto chia-sdk-bindings's chia-sdk-test dep (Plan 06-04). Verifier reports `status: passed`, 5/5 must-haves verified. The D-04 m=0 self-change assumption was disproven during research and the test was redesigned per RESEARCH §3b — disproof + redesign captured in Key Decisions. Pitfall #6 (`derive_synthetic` discipline) enforced across all 6 spending callsites. Cross-cutting concerns #5/#6/#7 all honored. Workspace cargo build/test/clippy/fmt/machete all green; pre-existing chia-sdk-daemon clippy nit remains documented out-of-scope in `deferred-items.md`. Previously: Last updated: 2026-05-18 after Phase 5 (Bindings — Rust facade + JSON descriptor) complete — BIND-01 + BIND-02 validated; descriptor↔facade drift zero.*
