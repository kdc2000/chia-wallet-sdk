# Roadmap: CHIP-0057 Silent Payments

## Overview

Six phases deliver send-side + transport-agnostic-receive silent-payment support into `chia-wallet-sdk` behind a `chip-0057` workspace feature. The build order follows the dependency graph: pure crypto primitives (Phase 1) → address layer (Phase 2) → transport-agnostic scanner + CHIP test-vector closure (Phase 3) → send-side action with Spends integration (Phase 4) → bindings descriptor + per-target compile (Phase 5) → simulator round-trip + cross-language E2E + example (Phase 6). The `ScalarField` type boundary lands in Phase 1 to prevent the signed-vs-unsigned scalar-reduction hazard from contaminating every downstream phase. Workspace integration (feature cascade, CI, lint policy) ships with the first code landing per the CHIP-0037 precedent (commit `bbc7f57f`).

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Crypto primitives & workspace integration** — `ScalarField` newtype, `tagged_hash` + Chia_SP/* tag constants, derivation paths, `chip-0057` feature cascade across types/utils/driver, CI lines, lint policy verification. (completed 2026-05-15)
- [x] **Phase 2: Address & key types** — `SilentPaymentKeys` (mnemonic + watch-only), `SilentPaymentAddress` (bech32m), labels generation, `LabelRegistry`, `m=0` change-label guard. (completed 2026-05-15)
- [ ] **Phase 3: Receive primitive & CHIP test-vector closure** — `TweakData`/`DetectedSpCoin`/`OutputMeta`, `compute_shared_secret_from_tweak`, `scan_from_tweaks` with labeled k-termination + `K_max` DOS guard, all CHIP TVs + bespoke `k=1` + adversarial `[0xff;32]` tests pass.
- [ ] **Phase 4: Send-side action** — `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks` (multi-party hard-error), `SilentPaymentSend` action with `Spends` integration, multi-output `Vec<Recipient>`, opcode 60/61 announcement binding, 32-byte memo-hint guard, doc-comment privacy warnings.
- [x] **Phase 4.1: Sage-style send-side binding refactor (INSERTED)** — drop opcode 60/61 empty-message announcement emission from `silent_payments/send_keys.rs`; rely on the SDK's existing `Relation::AssertConcurrent` cycle (opcode 64 SCC) for multi-input atomicity; add runtime gate rejecting `Relation::None` for multi-input SP sends; pin `Relation::AssertConcurrent` shape; matches the companion `~/silent-payments` Phase 2 design. (completed 2026-05-17)
- [ ] **Phase 4.2: Unify SP send into Action::send via SendDestination enum (INSERTED)** — fold `SilentPaymentSend` into `Action::send` via new `SendDestination` enum; drop dedicated SP action/finish-method/file; SP keys move onto `Spends` via `with_silent_payment_keys` builder; `From<Bytes32>` keeps all 28 existing Rust callers unchanged. Refines the API shape Phase 5 will expose.
- [ ] **Phase 5: Bindings (Rust facade + JSON descriptor)** — `bindings/silent_payments.json` descriptor, `chia-sdk-bindings::silent_payments` re-export facade, `SendDestination` opaque-handle class entry in `action_system.json` (factory + introspector methods per `Id` precedent), napi/pyo3/wasm builds green, AVA address round-trip test.
- [ ] **Phase 6: Simulator round-trip + bindings E2E + example** — `chia-sdk-test::silent_payments::tweak_data_from_simulator_block`, unlabeled and labeled simulator round-trip tests, AVA/pytest/wasm cross-language E2E (address-gen + send + scan-from-tweaks), `examples/silent_payment.rs`.

## Phase Details

### Phase 1: Crypto primitives & workspace integration
**Goal**: The cryptographic foundation — `ScalarField` type boundary, tagged-hash, derivation paths — is in place under `chip-0057` and every CI permutation builds cleanly.
**Depends on**: Nothing (first phase)
**Requirements**: CRYPTO-01, CRYPTO-02, WS-01, WS-02, WS-03
**Success Criteria** (what must be TRUE):
  1. `cargo build -p chia-sdk-types -F chip-0057` succeeds; `cargo build --workspace --all-features` succeeds; `cargo build --workspace` (no features) succeeds; `cargo clippy --workspace --all-features --all-targets` is clean.
  2. Adversarial unit test passes: `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes()` equals `r - 1` (big-endian), NOT `[0xff; 32]`. (Verifies unsigned reduction over BLS12-381 subgroup order.)
  3. Tag-pin unit test passes: `chia_sha2::Sha256::digest("Chia_SP/Inputs")`, `"Chia_SP/SharedSecret"`, `"Chia_SP/Label"` each produce their pinned 32-byte SHA-256 (typos in any tag constant fail the test before any protocol code runs).
  4. `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` returns zero hits.
  5. `cargo machete` passes with no new `[package.metadata.cargo-machete] ignored` entries.
**Plans**: 5 plans
- [x] 01-01-PLAN.md — Workspace chip-0057 feature flag scaffolding (root + types + driver + utils Cargo.toml; new [features] block on utils)
- [x] 01-02-PLAN.md — ScalarField newtype + GROUP_ORDER + unsigned mod-r reduction with 5 named tests
- [x] 01-03-PLAN.md — tagged_hash primitive + Chia_SP/* tag constants + tag-pin tests (with computed pinned bytes)
- [x] 01-04-PLAN.md — Derivation path constants (SCAN_PATH, SPEND_PATH)
- [x] 01-05-PLAN.md — CI matrix update + final gate verification (WS-02, WS-03 closure)

### Phase 2: Address & key types
**Goal**: A wallet developer can derive `(scan_sk, spend_sk)` from a mnemonic (or import from raw SKs for watch-only), generate unlabeled and labeled bech32m addresses, and round-trip them through encode/decode against the CHIP test vectors.
**Depends on**: Phase 1 (ScalarField, tagged_hash, derivation paths)
**Requirements**: ADDR-01, ADDR-02, ADDR-03, ADDR-04, ADDR-05, ADDR-06
**Success Criteria** (what must be TRUE):
  1. CHIP test-vector unlabeled-address round-trip passes: `SilentPaymentAddress::decode(TV1_addr).encode()` equals `TV1_addr` exactly; mainnet HRP is `spxch`, testnet is `tspxch`.
  2. CHIP test-vector labeled-address round-trip passes: `SilentPaymentKeys::from_mnemonic(TV3_mnemonic).labeled_address(TV3_m).encode()` equals `TV3_labeled_addr`.
  3. Negative-case tests pass: `decode` rejects wrong-HRP (`xch1...` rejected), wrong-checksum (bech32 non-m), wrong-payload-length, identity-element pubkey halves.
  4. `SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` reconstructs the same addresses as `from_mnemonic` would for matching SKs (watch-only/key-import path works).
  5. `labeled_address(0)` returns `Err(SilentPaymentError::ReservedChangeLabel)` (m=0 change-label guard).
  6. `LabelRegistry` round-trip: `register(scan_sk, m)` then `lookup(label_pk)` returns `Some(m)`; `forward(m)` returns the same `label_pk`.
**Plans**: 5 plans
- [x] 02-01-PLAN.md — Wiring & feature gate: chip-0057 deps on chia-sdk-utils + silent_payments module barrel
- [x] 02-02-PLAN.md — SilentPaymentError + SilentPaymentNetwork foundational types
- [x] 02-03-PLAN.md — SilentPaymentAddress encode/decode + 12 address tests (TV1 round-trip, pinned strings, 6 negative cases)
- [x] 02-04-PLAN.md — SilentPaymentKeys + LabelRegistry + 15 named tests (TV1 keys, TV3 labels, m=0 reject, registry round-trip)
- [x] 02-05-PLAN.md — CI matrix + final gate verification (per-crate `-F chip-0057` build line, prelude re-export, 5-build sweep, full Phase 2 gate)
**UI hint**: no

### Phase 3: Receive primitive & CHIP test-vector closure
**Goal**: A wallet can take a `TweakData` blob from any indexer adapter and detect every payment to the wallet's scan/spend key pair (unlabeled and labeled), with bounded compute under adversarial input. All CHIP test vectors pass end-to-end.
**Depends on**: Phase 1 (crypto), Phase 2 (only for tests that materialize addresses)
**Requirements**: RECV-01, RECV-02, RECV-03, RECV-04, RECV-05, CRYPTO-03
**Success Criteria** (what must be TRUE):
  1. CHIP test vectors pass as Rust unit tests: TV1 (unlabeled single-input), TV3 (labeled single-input), TV4 (multi-input aggregation) — all intermediate values (input_hash, shared_secret, t_k, one_time_pk, one_time_puzzle_hash) match the spec.
  2. Bespoke `k=1` test vector passes (catches `ser32(k)` endianness bugs that TV1/TV3/TV4 — all `k=0` — cannot detect).
  3. Adversarial scalar test passes: a `tagged_hash` input whose first byte is `>= 0x80` reduces to the unsigned `BigUint::from_bytes_be(&bytes) % r` value, NOT the signed value (verifies the `ScalarField` boundary actually fires through the full protocol).
  4. Adversarial-`TweakData` test: a `TweakData` whose `tweak_points[i] == PublicKey::default()` (identity element) returns `Vec::new()` from `scan_from_tweaks` with no panic; a `TweakData` containing malformed 48-byte pubkey bytes propagates a typed error (no `unwrap()` panic).
  5. DOS-guard test: a `TweakData` constructed to produce 10,000 consecutive forged matches at a single tweak point causes `scan_from_tweaks` to stop at `K_max = 32` (or the configured cap) within bounded time.
  6. Labeled k-termination test: `scan_from_tweaks` correctly detects a labeled output at `k = 1` when `k = 0` is unlabeled (verifies the rule "break the k loop only when neither unlabeled nor any labeled candidate matches at the current k").
**Plans**: 5 plans
- [x] 03-01-PLAN.md — Type surface + module scaffold + feature cascade extension (RECV-01)
- [x] 03-02-PLAN.md — Protocol primitives (compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) + adversarial scalar test (CRYPTO-03)
- [x] 03-03-PLAN.md — Scanner core (scan_from_tweaks + K_MAX_DEFAULT) + CHIP TV1, TV4 + identity-element guard (RECV-02, RECV-03)
- [x] 03-04-PLAN.md — Labeled detection branch (Option A reach-through) + TV3 + bespoke k=1 + labeled k-termination rule (RECV-04)
- [x] 03-05-PLAN.md — DOS-guard test + SilentPaymentScan trait + CI matrix line + prelude re-exports + final phase gate (RECV-05)

### Phase 4: Send-side action
**Goal**: A wallet developer can call `spends.add(SilentPaymentSend { recipient, amount, memos })` (or multi-recipient equivalent), call the `Spends` finish flow, and obtain a signed `SpendBundle` whose outputs land at correctly-derived one-time puzzle hashes that a Phase-3 scanner can detect. Multi-party and footgun cases hard-error.
**Depends on**: Phase 3 (ECDH primitives, `derive_output_tweak`, scanner for unit-level catch)
**Requirements**: SEND-01, SEND-02, SEND-03, SEND-04, SEND-05, SEND-06, SEND-07, SEND-08
**Success Criteria** (what must be TRUE):
  1. Round-trip on simulator produces a detectable coin: a hand-built scenario inside `chia-sdk-driver` tests sends to a `SilentPaymentAddress`, computes the expected one-time puzzle hash via `derive_one_time_puzzle_hash`, and confirms the resulting `CoinSpend` output's `puzzle_hash` matches that value bit-for-bit. (Full simulator round-trip is Phase 6's concern; Phase 4 proves the action composes correctly.)
  2. Multi-party hard-error fires: a `Spends` builder with 2 XCH inputs but only 1 registered local synthetic SK returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)` (or named variant) on `finish_with_silent_payment_keys` — NOT a silent single-input aggregation.
  3. Multi-output coordination test passes: two `SilentPaymentSend` actions targeting the same `scan_pk` in one `Spends` batch produce outputs at `k=0` and `k=1` respectively (counter increments correctly).
  4. Cross-index announcement test: a `Spends` with XCH inputs at two different derivation indices emits `CREATE_COIN_ANNOUNCEMENT` (opcode 60) on one input and `ASSERT_COIN_ANNOUNCEMENT` (opcode 61) on the other, binding the inputs so the recipient's `compute_input_hash` matches the sender's.
  5. Memo-hint guard test passes: passing a 32-byte memo to `SilentPaymentSend` either returns `Err(DriverError::SilentPaymentMemoHintForbidden)` OR is rewritten by the action so the first memo position is non-hint (verified by re-parsing the emitted `CreateCoin` condition). The hazard is hard to hit by accident.
  6. Doc-comment audit: every public memo-bearing API in `crates/chia-sdk-driver/src/silent_payments/` and `actions/silent_payment_send.rs` carries `/// Privacy warning: ...` text noting that memos are on-chain and visible to anyone holding the recipient's scan key.
**Plans**: TBD

### Phase 04.2: Unify SP send into Action::send via SendDestination enum (INSERTED)
**Goal**: Fold `SilentPaymentSend` into `Action::send` via a new `SendDestination` enum so wallets have a single send-action surface for both regular and silent-payment destinations. SP becomes a *destination type*, not a separate action variant. Matches Sage's existing model where every wallet operation (XCH/CAT/DID/NFT/option/multi-send/offers) goes through one `Action::send(Id, p2_puzzle_hash, amount, memos)` call shape — under this refactor, Sage's existing 12 `Action::send` call sites continue to compile unchanged via `impl From<Bytes32> for SendDestination`, and adding SP-recipient support becomes a one-line `recipient.into()` on the destination argument.
**Depends on**: Phase 4 (the action surface this refactor reshapes) + Phase 4.1 (the cycle binding 4.1 landed is preserved; `Relation::AssertConcurrent` gate moves but the requirement stays). Phase 5 (Bindings) consumes the final action shape, so this MUST land before Phase 5 plans solidify.
**Requirements**: SEND-04 (re-validated under new construction shape); ACTION-API-01 (new — unified send-action public surface: single `Action::send` constructor for both regular and SP destinations; SP-specific code is internal, not on the public action surface).
**Success Criteria** (what must be TRUE):
  1. `Action::SilentPaymentSend` enum variant and `Action::silent_payment_send` constructor removed from `crates/chia-sdk-driver/src/action_system/action.rs`. Their dispatch arms (currently lines 274 + 297) removed.
  2. New `SendDestination` enum (in `action_system/send_destination.rs` or `actions/send_destination.rs` — exact location is a fork to discuss at planning time): `PuzzleHash(Bytes32)` always-on + `SilentPayment(SilentPaymentAddress)` `#[cfg(feature = "chip-0057")]`-gated. `impl From<Bytes32> for SendDestination` provided so all 28 existing Rust `Action::send(id, ph, ...)` callers (12 in Sage, 16 in SDK) compile unchanged.
  3. `Action::send` signature morphs to `(id: Id, destination: SendDestination, amount: u64, memos: Memos<NodePtr>)`. `SendAction` struct field `puzzle_hash: Bytes32` becomes `destination: SendDestination`.
  4. `Spends::with_silent_payment_keys(pks, sks)` builder method (chip-0057-gated) registers SP keys on `Spends`. Optional chip-0057-gated fields on `Spends` hold them. Existing `silent_payment_counters` and `silent_payments_pending` fields preserved.
  5. `Spends::finish_with_silent_payment_keys` deleted. Its derivation pipeline (SK-coverage check, input-binding gate from 04.1, input_hash computation, per-pending derive, push CreateCoin, push to outputs.xch) absorbed into a `#[cfg(feature = "chip-0057")]` branch of `Spends::finish_with_keys` (or whichever finish entry point the planner picks; this is a fork to discuss at planning time).
  6. `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (640 lines) deleted. Apply-time SP logic (reserve XCH parent via `output_source`, per-`scan_pk` `k` counter, memo-hint guard, push to `silent_payments_pending`) absorbed into `actions/send.rs::SendAction::spend` chip-0057-gated branch dispatched on `self.destination`.
  7. `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` (236 lines) either deleted entirely or shrunk to a chip-0057-gated derivation-helper module called by `Spends::finish_with_keys`.
  8. New `DriverError::SilentPaymentKeysNotRegistered` (chip-0057-gated) — fires at finish time if `SendDestination::SilentPayment` was applied but `with_silent_payment_keys` was never called. Replaces the compile-time enforcement the dedicated `finish_with_silent_payment_keys` method used to provide.
  9. New `DriverError::SilentPaymentRequiresXch` (chip-0057-gated) — fires when `SendDestination::SilentPayment(...)` is used with `Id != Id::Xch`. The type system can't enforce SP-only-on-XCH because `Id` and `SendDestination` are independent.
  10. Phase 4.1 work preserved: `DriverError::SilentPaymentRequiresInputBinding` gate, the `Relation::AssertConcurrent` requirement for multi-input SP, the parametric cycle pinning test in `action_system/spends.rs::tests`, and the `Relation` rustdoc all continue to hold under the new construction pattern.
  11. All existing SP tests pass under the new construction pattern (call sites updated from `Action::silent_payment_send(...)` to `Action::send(Id::Xch, recipient.into(), ...)` + `spends.with_silent_payment_keys(pks, sks); spends.finish(...)` instead of `finish_with_silent_payment_keys(..., pks, sks)`): the 2 gate tests (`multi_input_requires_assert_concurrent_relation`, `single_input_accepts_relation_none`), 3 cycle pinning tests, `input_hash_round_trip`, `multi_party_hard_errors`, `action_state_machine`, `multi_output_same_scan_pk_increments_k`, `multi_output_distinct_scan_pks_independent_counters`, 3 memo guard tests.
  12. Bindings descriptor pattern verified ahead of Phase 5: `SendDestination` slots into bindy's existing opaque-handle pattern (see `Id`, `Action` in `bindings/action_system.json`) with factory methods (`puzzle_hash`, `silent_payment`) + introspectors (`is_puzzle_hash`/`as_puzzle_hash`/`is_silent_payment`/`as_silent_payment`). No new bindy-macro work required — confirmed during 04.2 design discussion. Phase 5 carries the descriptor update.
  13. Workspace gates green: `cargo build --release --workspace --all-features`, `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo machete`. Phase 1/4/4.1 grep bans still hold (`mod_by_group_order`, `^use sha2::`, `Sha256::digest` in `silent_payments/`; FINGERPRINT-01 grep gates from 04.1; Phase 4 Privacy-warning coverage).
**Plans**: 3 plans
- [x] 04.2-01-PLAN.md — Wave A foundation: 2 DriverError variants + SendDestination enum + 2 Spends chip-0057 fields + with_silent_payment_keys builder (additive only; consumers wired in Plan 02)
- [x] 04.2-02-PLAN.md — Wave A wire-up: Action::send Into<SendDestination> + SendAction.destination + chip-0057 SP arm in SendAction::spend + sp_finish_branch in Spends::finish_with_keys + reshape 3 send_keys.rs tests (kills dead_code-deny)
- [x] 04.2-03-PLAN.md — Wave B atomic delete-and-migrate: drop old SilentPaymentSend API + actions/silent_payment_send.rs + finish_with_silent_payment_keys; relocate 8 tests + add 2 NEW Wave 0 tests to actions/send.rs; prelude swap; REQUIREMENTS.md ACTION-API-01 entry; final phase gate
**UI hint**: no

### Phase 04.1: Sage-style send-side binding refactor (INSERTED)
**Goal**: Multi-input SP transactions land at the same on-chain shape as ordinary Sage multi-input wallet transactions. The SDK-emitted opcode 60/61 empty-message announcement binding is removed; detection relies on the SDK's existing `Relation::AssertConcurrent` cycle binding (opcode 64 SCC) which is what every SDK-built multi-input spend already emits. SP transactions therefore disappear into the SDK's regular-traffic anonymity set, removing the SP-specific fingerprint identified in this milestone's design review and matching the companion CHIP-0057 reference repo's Phase 2 work (`~/silent-payments`).
**Depends on**: Phase 4 (the binding helper this phase removes was landed in Plan 04-04). Phase 5 (Bindings) consumes the resulting API shape, so this refactor SHOULD ship before Phase 5 plans solidify.
**Requirements**: SEND-06 (re-validated under new binding scheme); FINGERPRINT-01 (new — multi-input SP transactions emit no SP-specific on-chain marker).
**Success Criteria** (what must be TRUE):
  1. `Spends::emit_silent_payment_announcements` and its Step-8.5 call site in `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` are deleted. No `CreateCoinAnnouncement` / `AssertCoinAnnouncement` is emitted by the SP send path.
  2. The 5 announcement-binding tests in `crates/chia-sdk-driver/src/actions/silent_payment_send.rs::tests` (`cross_index_announcement_binding`, `single_input_no_announcement`, plus the three that verified the empty-message shape) are deleted or rewritten to assert the opcode-64 cycle is present and the multi-input round-trip closes.
  3. `Spends::finish_with_silent_payment_keys` returns `Err(DriverError::SilentPaymentRequiresInputBinding)` (new chip-0057-gated variant) when `relation != Relation::AssertConcurrent` AND `silent_payments_pending.len() > 0` AND ≥2 non-ephemeral XCH inputs are present. Single-input SP sends accept any `Relation`.
  4. A pinning test on `Relation::AssertConcurrent` asserts the exact cyclic emission shape for N=2, N=3, N=4 inputs: coin 0 emits `assert_concurrent_spend(coin_ids[N-1])`, coin i emits `assert_concurrent_spend(coin_ids[i-1])` for i ≥ 1. Catches accidental refactors away from the SCC-detectable pattern.
  5. The `Relation::AssertConcurrent` variant gains public rustdoc identifying it as the CHIP-0057 Pass 2b detection guarantee and noting that downstream silent-payment scanning depends on its emitted cycle pattern.
  6. All Phase 4 round-trip and multi-output tests (`round_trip_matches_derive_one_time_puzzle_hash`, `multi_output_same_scan_pk_increments_k`, `multi_output_distinct_scan_pks_independent_counters`, `input_hash_round_trip`) still pass against the cycle binding alone, with no explicit announcement emission.
  7. Workspace gates green: `cargo build --release --workspace --all-features`, `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo machete`. Phase 4 grep bans still hold.
**Plans**: 2 plans
- [x] 04.1-01-PLAN.md — Drop opcode 60/61 announcement emission + update multi-input tests to Relation::AssertConcurrent (closes SC1, SC2; partial SC6; FINGERPRINT-01 deletion half)
- [x] 04.1-02-PLAN.md — Runtime input-binding gate + DriverError variant + Relation rustdoc + cycle pinning test + final phase gate (closes SC3, SC4, SC5, SC7; FINGERPRINT-01 enforcement half; SEND-06 re-validation)

### Phase 5: Bindings (Rust facade + JSON descriptor)
**Goal**: The full Rust silent-payments surface (address generation, send-action constructor, receive primitive) is exposed through `chia-sdk-bindings::silent_payments` and `bindings/silent_payments.json`; napi/pyo3/wasm crates build cleanly and an address round-trip test passes in TypeScript.
**Depends on**: Phases 2, 3, 4, 4.2 (Phase 4.2 finalizes the action surface this phase exposes — `SendDestination` replaces the dropped `silent_payment_send` factory).
**Requirements**: BIND-01, BIND-02
**Success Criteria** (what must be TRUE):
  1. `bindings/silent_payments.json` descriptor exists; `cargo build --workspace --all-features` builds `chia-sdk-bindings` cleanly with `chip-0057` enabled-by-default; `napi/` builds via `napi build`, `pyo3/` builds via `maturin develop`, `wasm/` builds via `wasm-pack build`. Generated `napi/index.d.ts` exposes `SilentPaymentAddress`, `SilentPaymentKeys`, `TweakData`, `DetectedSpCoin`, `LabelRegistry`, and the static functions `scanFromTweaks`/`deriveOneTimePuzzleHash`/`computeInputHash`/`aggregateSenderSks`.
  2. AVA test in `napi/__test__/silent_payments.ts` asserts address round-trip: generate a `SilentPaymentKeys` from a fixed mnemonic, encode the unlabeled address, decode it back, and assert `scan_pk`/`spend_pk` bytes match the originals. (Cross-language send + scan round-trip is Phase 6's concern; Phase 5 proves the descriptor compiles and a basic call works.)
  3. `bindings/action_system.json` gains a `SendDestination` opaque-handle class entry with factory methods `puzzle_hash(Bytes32)` + `silent_payment(SilentPaymentAddress)` and introspectors `is_puzzle_hash`/`as_puzzle_hash`/`is_silent_payment`/`as_silent_payment`. (Phase 04.2 SC12 pre-committed this pattern; supersedes the original SC3 wording, which referenced the `silent_payment_send` factory that 04.2 deleted.) A TS caller constructs an SP send via `Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos)`.
  4. `bindy-macro` static-functions schema verified ahead of descriptor commit (pre-flight action item from the architecture research): either confirmed natively supported (the `SilentPayments` zero-field class with `static_functions` works) or the fallback strategy is applied (free functions distributed onto carrier types).
**Plans**: 4 plans
- [ ] 05-01-PLAN.md — Wave 0 pre-flight: chip-0057 unconditional deps wiring (D-01) + zero-field SilentPayments stub probe + Wave 0 scaffolding + VALIDATION.md per-task map populated
- [ ] 05-02-PLAN.md — Full chia-sdk-bindings::silent_payments facade (~250 lines, 9 types per D-02/D-03) + bindings/silent_payments.json (9 entries) + SendDestination opaque-handle class added to action_system.json per D-04 + Action.send signature change + Spends.with_silent_payment_keys
- [ ] 05-03-PLAN.md — Cross-target build verification: napi build (pnpm build), pyo3 build (maturin develop), wasm-pack build (--target nodejs) with Vec<PublicKey> marshaling fallback if Open Q1 fires
- [ ] 05-04-PLAN.md — AVA round-trip test (silent_payments.spec.ts: 4 named tests for SC2+SC3) + descriptor↔facade drift audit script + REQUIREMENTS.md/STATE.md updates + 05-PHASE-SUMMARY.md (closes BIND-01 + BIND-02)
**UI hint**: no

### Phase 6: Simulator round-trip + bindings E2E + example
**Goal**: The full real-on-chain flow works end-to-end: sender wallet sends XCH to a silent-payment address (unlabeled and labeled), `Simulator` farms a block, the test helper extracts `TweakData`, the recipient's `scan_from_tweaks` detects the coin, and the recipient signs and spends it. The same flow works from TypeScript, Python, and WASM. A runnable example mirrors the flow.
**Depends on**: Phases 4, 5
**Requirements**: SIM-01, SIM-02, SIM-03, BIND-03, EX-01
**Success Criteria** (what must be TRUE):
  1. `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` is reachable via the public API of `chia-sdk-test` behind `chip-0057`; given a `Simulator` and a block height, it returns a `TweakData` constructed by grouping standard-puzzle spends, computing per-spend `tweak_point = input_hash * A_sum`, and collecting `OutputMeta` for the block's outputs.
  2. Unlabeled simulator E2E test passes: sender → farm → `tweak_data_from_simulator_block` → recipient `scan_from_tweaks` finds exactly one `DetectedSpCoin`; recipient builds and submits a follow-on spend of that coin using `StandardLayer::new(detected.onetime_sk.public_key().derive_synthetic())`; the simulator accepts the spend.
  3. Labeled simulator E2E test passes: same flow as (2) using a labeled address (`m >= 1`), with the detected coin carrying `label: Some(m)`; AND a separate sub-test confirms a `m=0` change-detection variant works internally (recipient detecting their own change output).
  4. AVA test in `napi/__test__/silent_payments.ts` runs the full address-gen + send + `scan_from_tweaks` round trip from TypeScript against a JS-side `TweakData` (built via the bindings-exposed simulator helper or a fixture); pytest in `pyo3/tests/test_silent_payments.py` runs the same flow from Python; `wasm/__test__/silent_payments.ts` runs it from WASM. Test asserts the detected coin's `coin_id` matches the sender's expected one-time output coin id, and that `Vec<chia_bls::PublicKey>` marshals correctly across the binding boundary for `TweakData::tweak_points`.
  5. `examples/silent_payment.rs` builds in CI (`cargo build --examples --all-features`) and runs to completion against the simulator, mirroring the structure of `examples/cat_spends.rs` and `examples/spend_simulator.rs`. Output demonstrates: mnemonic → address → send → farm → scan → detect → spend.
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Crypto primitives & workspace integration | 5/5 | Complete    | 2026-05-15 |
| 2. Address & key types | 5/5 | Complete    | 2026-05-15 |
| 3. Receive primitive & CHIP test-vector closure | 5/5 | Complete    | 2026-05-16 |
| 4. Send-side action | 5/5 | Complete    | 2026-05-16 |
| 4.1. Sage-style send-side binding refactor (INSERTED) | 2/2 | Complete    | 2026-05-17 |
| 4.2. Unify SP send into Action::send via SendDestination enum (INSERTED) | 3/3 | Complete    | 2026-05-17 |
| 5. Bindings (Rust facade + JSON descriptor) | 0/4   | Not started | - |
| 6. Simulator round-trip + bindings E2E + example | 0/TBD | Not started | - |

## Coverage

All 32 v1 requirements mapped to exactly one phase. See `REQUIREMENTS.md` Traceability table for full REQ-ID → phase mapping.

| REQ Category | Count | Phase(s) |
|---|---|---|
| CRYPTO-01, CRYPTO-02 | 2 | Phase 1 |
| WS-01, WS-02, WS-03 | 3 | Phase 1 |
| ADDR-01..06 | 6 | Phase 2 |
| RECV-01..05 | 5 | Phase 3 |
| CRYPTO-03 | 1 | Phase 3 |
| SEND-01..08 | 8 | Phase 4 |
| BIND-01, BIND-02 | 2 | Phase 5 |
| SIM-01..03 | 3 | Phase 6 |
| BIND-03 | 1 | Phase 6 |
| EX-01 | 1 | Phase 6 |
| **Total** | **32** | **6 phases** |

## Cross-Cutting Concerns

These are NOT phases — they apply to every phase as acceptance gates. Sourced from `.planning/research/PITFALLS.md` and `.planning/research/SUMMARY.md`.

1. **Signed-vs-unsigned scalar boundary** — `ScalarField` is the only mod-r reduction path in `chip-0057` code. Phase 1 establishes; Phases 2, 3, 4 honor (no `mod_by_group_order` import in any `silent_payments/` module).
2. **Synthetic-vs-raw key boundary** — `aggregate_sender_sks` and `SilentPaymentSend` only consume synthetic SKs; the real-signer catch is the Phase-6 simulator round-trip.
3. **Multi-party aggregation hard-error** — Defined in Phase 4, regression-tested in Phase 4 (unit) and Phase 6 (adversarial sim).
4. **Memo-position hint guard** — Defined in Phase 4, doc-comment carried through Phase 5 bindings.
5. **CHIP-0058 forward-compatibility** — `TweakData` has no transport fields (no `height`, no sp-service JSON envelope). Phase 3 owns the definition; Phase 5 carries the constraint into the JSON descriptor; Phase 6's example uses only the simulator helper, not a WS client.
6. **m=0 change-label guard** — Phase 2 implements; Phase 6 labeled E2E exercises change detection.
7. **Workspace lint policy (`WS-03`)** — Every phase's code must pass `deny clippy::all`, `warn pedantic`, `deny unsafe_code`, `deny dead_code`, and `cargo machete`. Phase 1 establishes the feature-gating skeleton; later phases inherit.

---
*Last updated: 2026-05-17 after Phase 5 planning — 4 plans across 4 waves (Wave 0 pre-flight: 01; Wave 1 facade+descriptor: 02; Wave 2 cross-target builds: 03; Wave 3 AVA tests+drift audit+close-out: 04); BIND-01 + BIND-02 queued for closure. Previously: *Last updated: 2026-05-17 after Phase 04.2 planning — 3 plans across 3 waves (Wave A additive: 01; Wave A wire-up + dead_code-deny resolution: 02; Wave B atomic delete-and-migrate + 8 test relocations + 2 NEW Wave 0 tests: 03); ACTION-API-01 newly traced; SEND-04 re-validation queued.*
