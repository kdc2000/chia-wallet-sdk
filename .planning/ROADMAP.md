# Roadmap: CHIP-0057 Silent Payments

## Overview

Seven phases deliver send-side + transport-agnostic-receive silent-payment support into `chia-wallet-sdk` behind a `chip-0057` workspace feature. Build order follows the dependency graph: pure crypto primitives (Phase 1) → address layer (Phase 2) → transport-agnostic scanner + CHIP test-vector closure (Phase 3) → send-side action with Spends integration (Phase 4) → bindings descriptor + per-target compile (Phase 5) → simulator round-trip + cross-language E2E + example (Phase 6) → maintainer-facing code review cleanup before upstream merge (Phase 7). The `ScalarField` type boundary lands in Phase 1 to prevent the signed-vs-unsigned scalar-reduction hazard from contaminating every downstream phase. Workspace integration (feature cascade, CI, lint policy) ships with the first code landing per the CHIP-0037 precedent (commit `bbc7f57f`). Phases 1–6 closed v1's 32 functional requirements; Phase 7 polishes the surface for merge.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Crypto primitives & workspace integration** — `ScalarField` newtype, `tagged_hash` + Chia_SP/* tag constants, derivation paths, `chip-0057` feature cascade across types/utils/driver, CI lines, lint policy verification. (completed 2007-05-15)
- [x] **Phase 2: Address & key types** — `SilentPaymentKeys` (mnemonic + watch-only), `SilentPaymentAddress` (bech32m), labels generation, `LabelRegistry`, `m=0` change-label guard. (completed 2007-05-15)
- [ ] **Phase 3: Receive primitive & CHIP test-vector closure** — `TweakData`/`DetectedSpCoin`/`OutputMeta`, `compute_shared_secret_from_tweak`, `scan_from_tweaks` with labeled k-termination + `K_max` DOS guard, all CHIP TVs + bespoke `k=1` + adversarial `[0xff;32]` tests pass.
- [ ] **Phase 4: Send-side action** — `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks` (multi-party hard-error), `SilentPaymentSend` action with `Spends` integration, multi-output `Vec<Recipient>`, opcode 60/61 announcement binding, 32-byte memo-hint guard, doc-comment privacy warnings.
- [x] **Phase 4.1: Sage-style send-side binding refactor (INSERTED)** — drop opcode 60/61 empty-message announcement emission from `silent_payments/send_keys.rs`; rely on the SDK's existing `Relation::AssertConcurrent` cycle (opcode 64 SCC) for multi-input atomicity; add runtime gate rejecting `Relation::None` for multi-input SP sends; pin `Relation::AssertConcurrent` shape; matches the companion `~/silent-payments` Phase 2 design. (completed 2007-05-17)
- [ ] **Phase 4.2: Unify SP send into Action::send via SendDestination enum (INSERTED)** — fold `SilentPaymentSend` into `Action::send` via new `SendDestination` enum; drop dedicated SP action/finish-method/file; SP keys move onto `Spends` via `with_silent_payment_keys` builder; `From<Bytes32>` keeps all 28 existing Rust callers unchanged. Refines the API shape Phase 5 will expose.
- [x] **Phase 5: Bindings (Rust facade + JSON descriptor)** — `bindings/silent_payments.json` descriptor, `chia-sdk-bindings::silent_payments` re-export facade, `SendDestination` opaque-handle class entry in `action_system.json` (factory + introspector methods per `Id` precedent), napi/pyo3/wasm builds green, AVA address round-trip test. (completed 2007-05-18)
- [x] **Phase 6: Simulator round-trip + bindings E2E + example** — `chia-sdk-test::silent_payments::tweak_data_from_simulator_block`, unlabeled and labeled simulator round-trip tests, AVA/pytest/wasm cross-language E2E (address-gen + send + scan-from-tweaks), `examples/silent_payment.rs`. (completed 2026-05-19)
- [ ] **Phase 7: Code review cleanup** — Strip planning-artifact references from source comments (hybrid strip+reread+repair); extract chip-0057 arm from `actions/send.rs` into a flat-sibling `actions/silent_payment_send.rs`; push `sp_finish_branch` into `Spends::prepare` and delete `Spends::finish_silent_payments`; relocate `silent_payments/e2e.rs` to `tests/silent_payments_e2e.rs` integration target so it can call the canonical helper; flip the 7 stale `VALIDATION.md` `nyquist_compliant` flags and patch `phase complete` CLI to auto-flip them going forward. (CLEANUP-05 dropped after convention discovery — inline tests is the universal pattern in chia-sdk-driver.)

## Phase Details

### Phase 1: Crypto primitives & workspace integration
**Goal**: The cryptographic foundation — `ScalarField` type boundary, tagged-hash, derivation paths — is in place under `chip-0057` and every CI permutation builds cleanly.
**Depends on**: Nothing (first phase)
**Requirements**: CRYPTO-01, CRYPTO-02, WS-01, WS-02, WS-03
**Success Criteria** (what must be TRUE):
  1. `cargo build -p chia-sdk-types -F chip-0057` succeeds; `cargo build --workspace --all-features` succeeds; `cargo build --workspace` (no features) succeeds; `cargo clippy --workspace --all-features --all-targets` is clean.
  2. Adversarial unit test passes: `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes()` equals `r - 1` (big-endian), NOT `[0xff; 32]`. (Verifies unsigned reduction over BLS07-381 subgroup order.)
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
- [x] 04.2-02-PLAN.md — Wave A wire-up: Action::send Into<SendDestination> + SendAction.destination replaces puzzle_hash + chip-0057 SP arm in SendAction::spend + sp_finish_branch in Spends::finish_with_keys + reshape 3 send_keys.rs tests (kills dead_code-deny)
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
- [x] 05-01-PLAN.md — Wave 0 pre-flight: chip-0057 unconditional deps wiring (D-01) + zero-field SilentPayments stub probe + Wave 0 scaffolding + VALIDATION.md per-task map populated
- [x] 05-02-PLAN.md — Full chia-sdk-bindings::silent_payments facade (~250 lines, 9 types per D-02/D-03) + bindings/silent_payments.json (9 entries) + SendDestination opaque-handle class added to action_system.json per D-04 + Action.send signature change + Spends.with_silent_payment_keys
- [x] 05-03-PLAN.md — Cross-target build verification: napi build (pnpm build), pyo3 build (maturin develop), wasm-pack build (--target nodejs) with Vec<PublicKey> marshaling fallback if Open Q1 fires
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
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Crypto primitives & workspace integration | 5/5 | Complete    | 2026-05-15 |
| 2. Address & key types | 5/5 | Complete    | 2026-05-15 |
| 3. Receive primitive & CHIP test-vector closure | 5/5 | Complete    | 2026-05-16 |
| 4. Send-side action | 5/5 | Complete    | 2026-05-16 |
| 4.1. Sage-style send-side binding refactor (INSERTED) | 2/2 | Complete    | 2026-05-17 |
| 4.2. Unify SP send into Action::send via SendDestination enum (INSERTED) | 3/3 | Complete    | 2026-05-17 |
| 5. Bindings (Rust facade + JSON descriptor) | 4/4   | Complete    | 2026-05-18 |
| 6. Simulator round-trip + bindings E2E + example | 5/5 | Complete    | 2026-05-19 |
| 7. Code review cleanup | 0/5 | Not started | - |

## Coverage

All 32 v1 functional requirements mapped to exactly one phase across Phases 1–6. Phase 7 adds 6 CLEANUP-* polish requirements that don't change behavior but address maintainer-review nits before upstream merge. See `REQUIREMENTS.md` Traceability table for full REQ-ID → phase mapping.

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
| CLEANUP-01..04, CLEANUP-06 | 5 | Phase 7 |
| POLISH-01..04 | 4 | Phase 8 |
| BRIDGE-01..06 | 6 | Phase 9 |
| **Total** | **47** | **9 phases** |

## Cross-Cutting Concerns

These are NOT phases — they apply to every phase as acceptance gates. Sourced from `.planning/research/PITFALLS.md` and `.planning/research/SUMMARY.md`.

1. **Signed-vs-unsigned scalar boundary** — `ScalarField` is the only mod-r reduction path in `chip-0057` code. Phase 1 establishes; Phases 2, 3, 4 honor (no `mod_by_group_order` import in any `silent_payments/` module).
2. **Synthetic-vs-raw key boundary** — `aggregate_sender_sks` and `SilentPaymentSend` only consume synthetic SKs; the real-signer catch is the Phase-6 simulator round-trip.
3. **Multi-party aggregation hard-error** — Defined in Phase 4, regression-tested in Phase 4 (unit) and Phase 6 (adversarial sim).
4. **Memo-position hint guard** — Defined in Phase 4, doc-comment carried through Phase 5 bindings.
5. **CHIP-0058 forward-compatibility** — `TweakData` has no transport fields (no `height`, no sp-service JSON envelope). Phase 3 owns the definition; Phase 5 carries the constraint into the JSON descriptor; Phase 6's example uses only the simulator helper, not a WS client.
6. **m=0 change-label guard** — Phase 2 implements; Phase 6 labeled E2E exercises change detection.
7. **Workspace lint policy (`WS-03`)** — Every phase's code must pass `deny clippy::all`, `warn pedantic`, `deny unsafe_code`, `deny dead_code`, and `cargo machete`. Phase 1 establishes the feature-gating skeleton; later phases inherit.

### Phase 7: Code review cleanup
**Goal**: Address the maintainer-facing issues flagged by the post-v1 code review (2026-05-19). The v1 silent-payments work is requirement-complete and architecturally sound, but ships with several expedient choices that a maintainer would push back on at PR review. This phase resolves them in priority order so the v1 surface is ready for upstream merge without follow-up nits.
**Depends on**: Phase 6
**Requirements**: CLEANUP-01, CLEANUP-02, CLEANUP-03, CLEANUP-04, CLEANUP-06
**Success Criteria** (what must be TRUE):
  1. **CLEANUP-01** — Strip planning-artifact references from source comments. Source comments contain zero references to GSD planning artifacts (`CONTEXT.md`, `RESEARCH.md`, `Plan NN-MM`, `D-NN`, `Pitfall N`, `Pattern N`, `Phase N.M`-style locals). Comments needing decision-rationale cite the CHIP spec section (`CHIP §425`), the BIP (`BIP-352`), or describe the constraint directly. Enforced by `grep -rE 'CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-6]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]' crates/*/src/silent_payments/ crates/chia-sdk-bindings/src/silent_payments.rs crates/chia-sdk-driver/src/action_system/send_destination.rs examples/silent_payment.rs napi/__test__/silent_payments*.ts pyo3/tests/test_silent_payments.py wasm/__test__/silent_payments.spec.ts` returning 0 hits. (Currently: 128 hits.)
  2. **CLEANUP-02** — Split `actions/send.rs`. The chip-0057 SP arm of `Action::send` moves out of `crates/chia-sdk-driver/src/actions/send.rs` (currently 1080 lines, up from 374) into a sibling file (e.g. `actions/silent_payment_send.rs`). Public surface unchanged — callers still use `Action::send(id, SendDestination::SilentPayment(addr), amount, memos)`. `actions/send.rs` ends at ≤ 600 lines.
  3. **CLEANUP-03** — Tighten `Spends::finish_silent_payments` bindings leak. The public `pub fn finish_silent_payments` added on `chia_sdk_driver::Spends` in Phase 6 Plan 06-04 is removed or replaced. Either: (a) binding-side `Spends::prepare` calls `Spends::finish_with_keys` directly (with empty secret keys when no SP is registered), OR (b) the SP finish branch becomes an internal trait method invoked by `prepare` itself, OR (c) the method is renamed `finish_silent_payments_for_bindings` and marked `#[doc(hidden)]` with the rationale in rustdoc. Acceptance: `grep -c 'pub fn finish_silent_payments\b' crates/chia-sdk-driver/src/action_system/spends.rs` returns 0, OR returns 1 with `#[doc(hidden)]` present immediately above it. Cross-language E2E tests still pass.
  4. **CLEANUP-04** — Drop `e2e.rs` inlined-helper workaround. The inlined `build_tweak_data()` helper in `crates/chia-sdk-driver/src/silent_payments/e2e.rs` is removed; tests call the canonical `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` instead. If Cargo's cyclic-dev-dep type confusion still blocks the direct call, the tests relocate to a top-level integration test target (`crates/chia-sdk-driver/tests/silent_payments_e2e.rs`) where the cycle resolves. Acceptance: `grep -c 'fn build_tweak_data' crates/chia-sdk-driver/src/silent_payments/e2e.rs` returns 0 (file may also be relocated); all 3 e2e tests still pass.
  5. **CLEANUP-06** — Nyquist VALIDATION.md frontmatter flip + process fix. All 8 prior phases' `*-VALIDATION.md` frontmatter has `nyquist_compliant: true` and `wave_0_complete: true` (currently only Phase 5 does — 7 of 8 carry template drift). The `phase complete` CLI command (or its caller in `execute-phase.md`) is patched so future phases auto-flip these flags when VERIFICATION.md is `status: passed`. Verified by inspecting the modified CLI code or by a regression test.
**Plans**: 5 plans
- [x] 07-01-PLAN.md — CLEANUP-01 strip planning-artifact references from source comments (17 files; e2e.rs handled by Plan 05)
- [x] 07-02-PLAN.md — CLEANUP-02 extract chip-0057 SP arm from actions/send.rs into actions/silent_payment_send.rs flat sibling
- [x] 07-03-PLAN.md — CLEANUP-03 push sp_finish_branch into Spends::prepare; delete Spends::finish_silent_payments and the binding-side caller
- [x] 07-04-PLAN.md — CLEANUP-06 flip 7 stale VALIDATION.md nyquist flags + patch gsd-tools phase complete to auto-flip going forward
- [x] 07-05-PLAN.md — CLEANUP-04 delete e2e.rs; relocate 3 e2e tests to tests/silent_payments_e2e.rs integration target using canonical helper

### Phase 8: Second-pass v1 polish — tighten SP module surface and dispatch ergonomics
**Goal**: Address 4 residual structural nits in the chip-0057 silent-payments code identified by a post-Phase-7 quality survey (2026-05-20). Distinct from the 5 issues Phase 7 already fixed; same shape (pure refactor, no behavior change, no API removals). Closes the polish gap before upstream merge.
**Depends on**: Phase 7
**Requirements**: POLISH-01, POLISH-02, POLISH-03, POLISH-04
**Success Criteria** (what must be TRUE):
  1. **POLISH-01** — `crates/chia-sdk-driver/src/silent_payments/mod.rs` contains zero `pub use foo::*;` wildcard re-exports (the existing `pub(crate) use send_keys::*;` is permitted — it's crate-private). All `pub use` lines name their re-exported symbols explicitly. The public surface (`scan_from_tweaks`, `SilentPaymentScan`, `K_MAX_DEFAULT`, `TweakData`, `OutputMeta`, `DetectedSpCoin`, `aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash`, and the 5 protocol primitives) and `src/prelude.rs:41-45` re-export list are byte-identical to pre-phase. Verified by `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returning 0 and `cargo build --release --workspace --all-features` clean.
  2. **POLISH-02** — `crates/chia-sdk-driver/src/silent_payments/aggregate.rs`, `input_hash.rs`, and `one_time.rs` are deleted. Their contents (single `pub fn` + tests each) live inside `crates/chia-sdk-driver/src/silent_payments/protocol.rs`. `wc -l crates/chia-sdk-driver/src/silent_payments/protocol.rs` ≤ 700 (target ~640). All external callsites (`chia-sdk-bindings`, `chia-sdk-test`, `actions/silent_payment_send.rs`, `action_system/spends.rs`, `tests/silent_payments_e2e.rs`, `src/prelude.rs`) resolve identically — verified by full workspace build + the chip-0057 driver test suite running the same test count as pre-phase.
  3. **POLISH-03** — `crates/chia-sdk-driver/src/action_system/send_destination.rs` contains the `Box<SilentPaymentAddress>` rationale in exactly one location (the variant-level rustdoc). The enum-level `/// Cannot derive Copy because SilentPaymentAddress is Clone-only.` paragraph (currently lines 21-23) is removed. Verified by `grep -c 'Cannot derive \`Copy\`' crates/chia-sdk-driver/src/action_system/send_destination.rs` returning 0 and `grep -c 'large_enum_variant' crates/chia-sdk-driver/src/action_system/send_destination.rs` returning exactly 1.
  4. **POLISH-04** — `crates/chia-sdk-driver/src/actions/send.rs` chip-0057 dispatch (currently lines 44-62) is restructured to a single exhaustive `match` where the `SilentPayment` arm calls `handle_silent_payment_send(...)` and `return`s directly from inside the arm. The `unreachable!("handled above")` arm is removed. Verified by `grep -c 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs` returning 0 and `grep -c 'handle_silent_payment_send' crates/chia-sdk-driver/src/actions/send.rs` returning exactly 1.

**Plans:** 4 plans

Plans:
- [x] 08-01-PLAN.md — POLISH-01: Replace public-module `pub use foo::*;` wildcards in `silent_payments/mod.rs` with explicit named re-exports (Wave 1)
- [x] 08-02-PLAN.md — POLISH-03: De-duplicate `SendDestination` Boxing rationale — drop enum-level "Cannot derive Copy" paragraph; keep variant-level (Wave 1)
- [x] 08-03-PLAN.md — POLISH-04: Restructure `Action::send` chip-0057 dispatch — single exhaustive match; remove `unreachable!("handled above")` (Wave 1)
- [x] 08-04-PLAN.md — POLISH-02: Atomic fold of `aggregate.rs` + `input_hash.rs` + `one_time.rs` into `protocol.rs`; collapse mod.rs to final D-01 shape (Wave 2)

### Phase 9: Real-block TweakData bridge + Python Relation binding

**Goal:** Close two downstream Python-consumer-blocking gaps in the v1 CHIP-0057 surface. (1) Extract the simulator helper's pure logic into a real-block-callable helper `chia_sdk_driver::silent_payments::tweak_data_from_block_spends(&[CoinSpend], &[Coin])` that implements full Pass 2a + Pass 2b SCC grouping (the existing simulator helper aggregates the whole block as one group — correct for simulator's 1-tx-per-block convention, wrong for real multi-tx blocks). (2) Bind `chia_sdk_driver::Relation` to pyo3/napi/wasm via the opaque-handle pattern and extend `Spends.prepare(deltas)` to `Spends.prepare(deltas, Option<Relation>)` so Python callers can drive multi-input SP sends. Pure additive; no API breakage to v1 surface.

**Requirements**: BRIDGE-01, BRIDGE-02, BRIDGE-03, BRIDGE-04, BRIDGE-05, BRIDGE-06

**Depends on:** Phase 8

**Success Criteria** (what must be TRUE):
  1. **BRIDGE-01** — `chia_sdk_driver::silent_payments::tweak_data_from_block_spends` exists in new sibling file `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` with the exact signature `pub fn tweak_data_from_block_spends(coin_spends: &[CoinSpend], additions: &[Coin]) -> Result<TweakData, DriverError>`. Inline `#[cfg(test)] mod tests {}` covers Pass 2a same-puzzle-hash grouping, Pass 2b multi-input SCC, Pass 2b pollution attack (canonical correctness oracle), non-standard-puzzle skip, CHIP §459 identity-element guard, empty-block edge case. Uses iterative Tarjan SCC (no recursion — adversarial-input safe).
  2. **BRIDGE-02** — `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` collapses to a ~5-line delegate over BRIDGE-01's helper. All 3 Phase 6 simulator e2e tests (`test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change`) and the 2 existing inline simulator-helper tests pass byte-identically pre/post refactor.
  3. **BRIDGE-03** — `Relation` opaque-handle binding lands in `chia-sdk-bindings::action_system` with 5 methods (`none`, `assert_concurrent`, `is_none`, `is_assert_concurrent`, `equals`). Descriptor entry in `bindings/action_system.json` (not top-level). All three binding targets (napi/pyo3/wasm) build cleanly and expose the new class.
  4. **BRIDGE-04** — `chia-sdk-bindings::Spends::prepare` signature extends from `prepare(deltas)` to `prepare(deltas, relation: Option<Relation>)`. `None` defaults to `sdk::Relation::None` preserving current behavior. Existing pyo3 single-input E2E `test_unlabeled_e2e` passes without any test source edits. Descriptor entry in `bindings/action_system.json::Spends.methods.prepare` updated.
  5. **BRIDGE-05** — `SilentPayments.tweakDataFromBlockSpends(coinSpends: CoinSpend[], additions: Coin[]) -> TweakData` static method bound on the `SilentPayments` namespace class. Descriptor entry in `bindings/silent_payments.json`. Drift-script `scripts/sp_descriptor_facade_drift.sh` exits 0.
  6. **BRIDGE-06** — Three new cross-binding multi-input round-trip tests (`napi/__test__/silent_payments_multi_input.spec.ts`, `pyo3/tests/test_silent_payments.py::test_multi_input_e2e`, `wasm/__test__/silent_payments_multi_input.spec.ts`) each: farm 2 XCH coins → `Action::send(Id::Xch, SendDestination::SilentPayment(addr), amount, None)` → `Spends.prepare(deltas, Relation.assert_concurrent())` → farm bundle → `SilentPayments.tweakDataFromBlockSpends(sim.blockSpends(h), sim.blockOutputs(h))` → scan → exactly one DetectedSpCoin lands. Plus `examples/silent_payment.rs` gains a multi-input section (Stages 6-9). Plus `Simulator.blockSpends(height)` + `Simulator.blockOutputs(height)` binding facade entries added so binding tests can construct helper inputs.
  7. Workspace gates: `cargo build --release --workspace --all-features` clean; `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean; `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` clean; `cargo fmt --all --check` clean; `cargo machete` clean (zero new ignored entries); CLEANUP-01 grep ban still holds on all new files; zero new `#[allow]` attributes anywhere.

**Plans:** 6 plans

Plans:
- [x] 09-01-PLAN.md — BRIDGE-01: tweak_data_from_block_spends helper + iterative Tarjan SCC + 6 inline tests + prelude wiring (Wave 1)
- [x] 09-02-PLAN.md — BRIDGE-02: collapse tweak_data_from_simulator_block to thin adapter over BRIDGE-01 (Wave 2)
- [x] 09-03-PLAN.md — BRIDGE-03: Relation opaque-handle binding + descriptor in bindings/action_system.json (Wave 1)
- [x] 09-04-PLAN.md — BRIDGE-04: extend Spends.prepare signature to accept Option<Relation> + descriptor update (Wave 2)
- [x] 09-05-PLAN.md — BRIDGE-05: bind SilentPayments.tweakDataFromBlockSpends as static method on namespace + descriptor (Wave 2)
- [x] 09-06-PLAN.md — BRIDGE-06: Simulator block_spends/block_outputs facade + 3 cross-binding multi-input tests + example multi-input section (Wave 2)

### Phase 09.3: non-breaking SP send: restore Action::send contract, make SP send additive (revert ACTION-API-01 unification) (INSERTED)

**Goal:** Restore the pre-Phase-4.2 `Action::send(id, Bytes32, amount, memos)` / `SendAction { puzzle_hash } + Copy` public contract byte-for-byte and make CHIP-0057 silent-payment send a separate, `chip-0057`-gated `Action::SilentPaymentSend` variant + `Action::silent_payment_send` constructor — reverting the 4.2 `SendDestination` unification that source-broke downstream consumers (Sage E0283/E0560). Additive-behind-flag, zero blast radius on the non-gated `send`; the detection oracle (one-time puzzle hash derivation) and GUARD-01/02/03 are preserved exactly.
**Requirements**: ACTION-API-01 (reframed), SEND-04 (reframed) — no new REQ-IDs; acceptance is CONTEXT.md AC1-AC5.
**Depends on:** Phase 9 (and 9.2 — builds on the GUARD-hardened SP send path)
**Plans:** 2/3 plans executed

Plans:
- [x] 09.3-01-PLAN.md — Rust core: restore SendAction/Action::send contract + Copy; relocate SP into a gated SilentPaymentSendAction (variant + 2 dispatch arms + ctor); delete SendDestination + SilentPaymentRequiresXch; migrate driver tests/e2e/example/prelude (Wave 1)
- [x] 09.3-02-PLAN.md — Bindings: revert Action.send to puzzle hash + add Action.silentPaymentSend facade/descriptor; delete SendDestination class; rebuild napi/pyo3/wasm; migrate 7 binding test files incl. the non-SP action_system.spec.ts revert (Wave 2)
- [ ] 09.3-03-PLAN.md — Reframe ACTION-API-01 + SEND-04 in REQUIREMENTS.md; full CI lint matrix (chip-0057 on/off); Sage zero-edit recompile harness (AC3 checkpoint) (Wave 3)

### Phase 09.2: Harden SP sender keys: synthetic-key runtime guard + SyntheticKey newtype + raw-key bindings (INSERTED)

**Goal:** Close the silent-fund-loss footgun reported in `ISSUE-silent-payment-synthetic-key-guard.md` (2026-06-01, hit by a downstream BIP-352 port via the Python bindings): `Spends::with_silent_payment_keys` requires *synthetic* sender keys but enforces it nowhere (neither type nor runtime). Passing raw wallet keys compiles, signs, broadcasts, and confirms — but the recipient's one-time puzzle hash is derived from the raw scalar, so the coin is undetectable and unspendable by any CHIP-0057 scanner. The fix is a layered defense, verified true in the live code (see issue): (1) a **runtime guard** (universal, covers all bindings + the newtype escape hatch), (2) a **`SyntheticSecretKey`/`SyntheticPublicKey` newtype** (compile-time prevention for Rust callers; resolves the long-deferred Q2), and (3) **raw-key-accepting binding methods on all three targets** (napi/pyo3/wasm) that synthesize internally with the runtime guard as backstop.

**Requirements**: GUARD-01 (runtime guard), GUARD-02 (newtype), GUARD-03 (raw-key bindings ×3). No silent failure remains in any language surface.

**Depends on:** Phase 9 (and 9.1 — builds on the post-cleanup SP send path)

**Plans:** 3/3 plans complete

Design (locked — see `09.2-CONTEXT.md` for detail):

1. **GUARD-01 — runtime guard (the universal backstop).** In `sp_finish_branch` (and/or at registration), for each registered key validate `StandardArgs::curry_tree_hash(registered_pk) == p2_puzzle_hash` (the `IndexMap` key already *is* the coin's p2 puzzle hash) AND `registered_sk.public_key() == registered_pk`. On mismatch, return a new `DriverError::SilentPaymentKeyNotSynthetic` *before* the bundle is signed. Validates against the actual coin, so it accepts any correct synthetic key (default OR custom hidden puzzle) and rejects raw keys. Catches single-input. This is the only guard that crosses the FFI boundary and the backstop for the newtype's `_unchecked` path — so it stays even though the newtype exists.

2. **GUARD-02 — `SyntheticSecretKey` / `SyntheticPublicKey` newtypes (Rust compile-time).** Wrap `chia_bls::{SecretKey,PublicKey}`. Ergonomic primary constructor `from_raw(&raw)` (does `derive_synthetic()`, default hidden) so Rust callers get the same "pass raw" convenience the bindings get; `from_synthetic_unchecked(k)` as the documented escape hatch (covered by GUARD-01); `.public_key() -> SyntheticPublicKey`. Change `Spends::with_silent_payment_keys` to take `IndexMap<Bytes32, SyntheticPublicKey>` / `IndexMap<Bytes32, SyntheticSecretKey>`. This makes the raw-key mistake a *compile error* in Rust and resolves STATE Q2 (`SyntheticSecretKey` newtype vs documented `&[SecretKey]`). Blast radius: `examples/silent_payment.rs` and `tests/silent_payments_e2e.rs` update to the newtype.

3. **GUARD-03 — raw-key binding methods on napi/pyo3/wasm.** The bindings can't carry the Rust newtype across FFI, so each target's `with_silent_payment_keys` (in `chia-sdk-bindings`) accepts **raw** `PublicKey`/`SecretKey`, calls `derive_synthetic()` internally (default hidden), and relies on GUARD-01 underneath. Doc the default-hidden assumption (custom-hidden coins fail loud via GUARD-01, not silently). Update `bindings/action_system.json` descriptor accordingly.

**Verification bar:** raw keys in Rust → compile error; raw keys via `_unchecked`/bindings → `Err(SilentPaymentKeyNotSynthetic)` before signing (proven by a test per surface, incl. single-input); a correct synthetic key still produces the byte-identical one-time PH as today (no regression to the Phase-3/6 detection oracle); all CI permutations + napi/pyo3/wasm green; zero new workspace deps.

Plans:
- [x] 09.2-01-PLAN.md — GUARD-02: SyntheticSecretKey/SyntheticPublicKey newtypes + with_silent_payment_keys signature change + update example/e2e/send_keys callers + fix stale protocol.rs comment (Wave 1)
- [x] 09.2-02-PLAN.md — GUARD-01: DriverError::SilentPaymentKeyNotSynthetic + per-input runtime guard in sp_finish_branch (curry_tree_hash==ph & sk.public_key()==pk) before signing, single-input covered (Wave 2)
- [x] 09.2-03-PLAN.md — GUARD-03: napi/pyo3/wasm with_silent_payment_keys accept raw keys + synthesize internally via derive_synthetic; descriptor/doc update; binding tests build synthetic-keyed coins + assert typed error crosses FFI (Wave 3)

### Phase 09.1: Fix 5 maintainer-flagged conformance issues in chip-0057 SP surface (INSERTED)

**Goal:** Resolve the 5 concrete pre-merge issues surfaced by the 2026-05-29 cross-cutting code-quality review of the silent-payments surface. All are small/mechanical, no architecture changes, no API removals — they harden the edges CI doesn't exercise (non-default feature permutations, the FFI boundary) and strip planning-process residue from shipped source. Closes the last gap before the chip-0057 work is upstream-merge clean.

**Requirements**: The 5 issues below, each fixed and verified under the relevant CI permutation (per-crate build with and without `--all-features`, `clippy -D warnings`, `cargo build --examples`/`cargo test` without `--all-features`).

**Depends on:** Phase 9

**Plans:** 2/2 plans complete

The 5 issues (with evidence from the review):

1. **FFI panic on empty input.** `compute_input_hash` (`crates/chia-sdk-driver/src/silent_payments/protocol.rs:183`) `assert!`s when `coin_ids` is empty, and it is reachable from all three bindings via `crates/chia-sdk-bindings/src/silent_payments.rs:425` inside a `Result`-returning method that passes the slice through unguarded. Violates the repo's "no panicking in library code." Fix: return a `DriverError` (or guard in the facade) instead of asserting across the FFI boundary.

2. **Example missing `required-features`.** `examples/silent_payment.rs` uses chip-0057-only API but root `Cargo.toml` has no `[[example]]` block, so `cargo build --examples` / `cargo test` without `--all-features` fails (14 compile errors); CI is green only because its exact lines avoid that combination. Fix: add `[[example]] name = "silent_payment"` / `required-features = ["chip-0057"]`.

3. **`SendDestination` trips `missing_copy_implementations` when chip-0057 is off.** It collapses to a single-variant enum (`crates/chia-sdk-driver/src/action_system/send_destination.rs:29`); `clippy -p chia-sdk-driver -D warnings` (no features) warns, masked only by the all-features CI clippy. Fix: gate the lint or derive `Copy` for the feature-off shape.

4. **`SilentPaymentError` drops `Clone, PartialEq, Eq`** that the wrapped `Bech32Error` carries (`crates/chia-sdk-utils/src/silent_payments/error.rs`), forcing tests into `matches!`+`panic!` instead of `assert_eq!`. Fix: add the derives, matching the in-crate `Bech32Error` precedent; simplify the affected tests.

5. **Planning-process residue in shipped source.** Comments referencing "VALIDATION.md grep matchers," "Plan 04.2," and "Pitfall 7," plus a test (`crates/chia-sdk-types/src/silent_payments/scalar.rs:121`) whose comment admits its name contradicts its own assertion. Fix: scrub the planning references; rename/correct the misleading test.

Plans:
- [x] 09.1-01-PLAN.md — ISSUE-2/3/4/5: [[example]] required-features + SendDestination missing_copy_implementations gate + SilentPaymentError Clone/PartialEq/Eq derives + scrub planning residue & rename misleading scalar test (Wave 1)
- [x] 09.1-02-PLAN.md — ISSUE-1: guard the FFI-reachable compute_input_hash facade to return Err on empty input (no panic) + boundary test + global regression bar (Wave 2)
