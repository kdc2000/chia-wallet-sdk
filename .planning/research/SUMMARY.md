# Research Summary — CHIP-0057 Silent Payments

**Project:** CHIP-0057 silent payments integration into chia-wallet-sdk
**Domain:** Cryptographic protocol addition to an existing multi-target Rust SDK (napi / pyo3 / wasm bindings, strict workspace lints)
**Researched:** 2026-05-15
**Confidence:** HIGH

---

## Executive Summary

CHIP-0057 is a Chia adaptation of BIP-352 (Bitcoin silent payments) onto BLS12-381. The wallet SDK needs to add: (1) cryptographic primitives (`ScalarField` unsigned mod-r, tagged hash), (2) an address type (`spxch`/`tspxch` bech32m over a 96-byte `scan_pk || spend_pk` payload), (3) a send-side `SilentPaymentSend` action that composes with the existing `Spends` builder, and (4) a transport-agnostic receive primitive (`scan_from_tweaks(TweakData)`). No new crates, no new workspace dependencies — every primitive needed is already pinned: `chia-bls 0.36.1`, `chia-sha2 0.36.1`, `num-bigint 0.4.6`, `bip39 2.2.0`, `bech32 0.9.1`. The only Cargo.toml changes are feature-flag cascade additions in three crates (`chia-sdk-types`, `chia-sdk-utils`, `chia-sdk-driver`) and one new optional dep on `chia-sdk-utils` (`chia-bls`, optional, activated by `chip-0057`).

The single hardest design decision is the timing of ECDH in the send path: `SilentPaymentSend::spend()` cannot complete the one-time puzzle hash derivation at action time because it needs the aggregated synthetic secret key, which is only available at signing time. The recommended resolution is a two-phase approach — action time records the deterministic bits (scan_pk, spend_pk, input_hash, k, amount, memos) into `Spends::silent_payments_pending`; a new `Spends::finish_with_silent_payment_keys` method performs ECDH and overwrites the placeholder puzzle hash before emitting CoinSpends. This design decision is flagged for validation in Phase 4.

The dominant correctness risk is the signed-vs-unsigned scalar reduction split: Chia's existing synthetic-key math uses signed `mod_by_group_order`; CHIP-0057 protocol scalars require unsigned reduction. Mixing them silently produces undetectable payments for ~50% of hashes (those with high bit set). This is mitigated architecturally by a `ScalarField` newtype that is the only mod-r reduction path inside `chip-0057` code. This newtype must land in Phase 1, before any protocol function is written. The CHIP test vectors (TV1, TV3, TV4) do not catch this bug; an adversarial test with `[0xff; 32]` input is required.

---

## Key Findings

### Stack

**Zero new workspace dependencies.** Every dep is already pinned at the workspace root and verified against on-disk crate source. The stack delta is purely feature-flag additions to existing Cargo.tomls.

| Technology | Role in CHIP-0057 | Critical detail |
|------------|-------------------|-----------------|
| `chia-bls 0.36.1` | G1 scalar multiply, point add/sub, identity check, SK derivation | `PublicKey::scalar_multiply` mutates **in place** — must clone first (Pitfall 9.1). `PublicKey::from_bytes` returns `Result` — never `.unwrap()` on TweakData inputs (Pitfall 9.2). |
| `chia-sha2 0.36.1` | All SHA-256 including tagged hash | SDK convention; do NOT mix in `sha2` directly. |
| `num-bigint 0.4.6` | Unsigned BigUint mod-r reduction inside `ScalarField` | Use `BigUint::from_bytes_be` (unsigned); NOT `BigInt::from_signed_bytes_be` (signed, Pitfall 1). |
| `bip39 2.2.0` | `Mnemonic::parse` + `to_seed("")` | Identical path already in `chia-sdk-bindings/src/mnemonic.rs`. |
| `bech32 0.9.1` | Bech32m encode/decode for 96-byte address payload | HRP `spxch`/`tspxch`; `Variant::Bech32m` (not Bech32, Pitfall 8). |
| `chia-puzzle-types 0.36.1` | `StandardArgs::curry_tree_hash(pk.derive_synthetic())` for one-time puzzle hash | Drops the prototype's `puzzle_hash_for_pk`; also exposes signed `mod_by_group_order` as a visible contrast point. |

**One structural note on `chia-sdk-utils`:** adding `SilentPaymentAddress` to `chia-sdk-utils` (where the existing `Address` lives) requires adding `chia-bls` as an `optional = true` dep activated by `chip-0057`. This is clean and consistent with how `chia-sdk-driver` gates `dep:sha3` under `chip-0037`. Do not put `SilentPaymentAddress` in `chia-sdk-driver` (wrong semantic layer) or `chia-sdk-types` (forces bip39 into the leaf crate).

Full stack detail: `.planning/research/STACK.md`

### Features

**PROJECT.md is the locked v1 scope.** Feature research surfaced six additions the locked requirements do not explicitly call out. These are all low-complexity but some are correctness-critical. The roadmapper should evaluate folding them into existing REQ-IDs or adding new ones.

**Table-stakes (locked in PROJECT.md):**
- Bech32m address encode/decode with HRP check (`ADDR-02`)
- Mnemonic key derivation (`ADDR-01`)
- Labeled addresses + detection (`ADDR-03`, `RECV-04`)
- Send to address via `Spends` action (`SEND-01..04`)
- Transport-agnostic `scan_from_tweaks(TweakData)` (`RECV-01..04`)
- Multi-input aggregation with single-party-control assertion (`SEND-03`)
- Identity-element zero-sum guard (implicitly in `CRYPTO-01`)
- CHIP test vectors TV1, TV3, TV4 pass (`CRYPTO-03`)
- Bindings napi/pyo3/wasm (`BIND-01..03`)

**Recommended additions to v1 surface (not in PROJECT.md — roadmapper must decide whether to add REQ-IDs):**
1. **`SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)`** — ~5 lines; enables watch-only wallets and raw-SK import. Universal in production BIP-352 wallets.
2. **`Label::CHANGE` constant (m=0) guard** — either a `change_address()` accessor or a runtime check in `labeled_address(m)` that warns/errors on m=0. CHIP mandates wallets never publish m=0 to external senders.
3. **`SilentPaymentSend` accepts `Vec<Recipient>` for multi-output sends** — multi-output (k>0) is first-class in BIP-352. One-action-per-recipient loses the invariant that all outputs to the same scan_pk share one `input_hash`.
4. **Multi-input cross-index announcement binding** — when sender inputs span different derivation indices, `SilentPaymentSend` must emit opcode 60/61 conditions so the recipient's Pass 2b scanner can reconstruct the group. Without this, cross-index sends are silently undetectable.
5. **Memo-position hint guard** — a 32-byte memo in position 0 of a `CREATE_COIN` is interpreted as a puzzle-hash hint by standard wallets, defeating privacy. `SilentPaymentSend` must either reject 32-byte memos or insert a non-hint at position 0.
6. **Doc-comment privacy warning on user-supplied memos** — memos are visible to all observers; no API change, just docstring.

**Anti-features — must not ship:**
- Auto-hint memos on `CREATE_COIN` (publishes one-time puzzle hash to all indexers — destroys privacy)
- CAT2 or NFT silent-payment sends (undetectable by receiver until CHIP-0058 indexer support)
- Hex-encoded addresses (no checksum, no network info, wrong)
- Custom HRPs (breaks cross-wallet interop)
- `SilentPaymentTweakSource` async trait (design-in-a-vacuum without real CHIP-0058 consumer)
- `get_coin_records_by_puzzle_hash` convenience scanner (privacy-leaking, out of SDK scope)

**Deferred to v1.x/v2+:**
- CHIP-0058 transport client / async trait
- GCS block-filter helpers (no CHIP yet)
- CAT2 silent-payment sends (after CHIP-0058 indexer)
- Encrypted recipient-facing memos
- Hardware-wallet split-custody workflow

Full feature detail: `.planning/research/FEATURES.md`

### Architecture

Silent payments add three modules (one per existing crate) plus a bindings facade, with no new crate and no new external service. Dependency graph is strictly acyclic: `types` <- `utils` <- `driver` <- `bindings`.

**Module map:**

| Module | Crate | Contents |
|--------|-------|----------|
| `silent_payments/scalar.rs` | `chia-sdk-types` | `ScalarField` newtype (unsigned mod-r only) |
| `silent_payments/tagged_hash.rs` | `chia-sdk-types` | `tagged_hash()` + `TAG_INPUTS`, `TAG_SHARED_SECRET`, `TAG_LABEL` consts |
| `silent_payments/paths.rs` | `chia-sdk-types` | `SCAN_PATH`, `SPEND_PATH` derivation-path arrays |
| `silent_payments/keys.rs` | `chia-sdk-utils` | `SilentPaymentKeys` (mnemonic -> SKs, address constructors) |
| `silent_payments/address.rs` | `chia-sdk-utils` | `SilentPaymentAddress` (bech32m, 96-byte payload) |
| `silent_payments/ecdh.rs` | `chia-sdk-driver` | `compute_shared_secret_*`, `compute_input_hash` |
| `silent_payments/protocol.rs` | `chia-sdk-driver` | `derive_output_tweak`, `derive_onetime_{pk,sk}`, `scan_from_tweaks` |
| `silent_payments/aggregate.rs` | `chia-sdk-driver` | `aggregate_sender_sks` (single-party only) |
| `silent_payments/labels.rs` | `chia-sdk-driver` | `generate_label`, `LabelRegistry` (bidirectional map) |
| `silent_payments/tweak_data.rs` | `chia-sdk-driver` | `TweakData`, `DetectedSpCoin`, `OutputMeta` (transport-agnostic) |
| `actions/silent_payment_send.rs` | `chia-sdk-driver` | `SilentPaymentSend impl SpendAction` |
| `action_system/action.rs` | `chia-sdk-driver` | `Action::SilentPaymentSend` variant (chip-0057 gated) |
| `silent_payments.rs` | `chia-sdk-bindings` | Re-export facade + `bindings/silent_payments.json` descriptor |

**Key integration design: `SilentPaymentSend` and `Spends`**

The send-side ECDH cannot complete at action time (`apply()`) because the aggregated synthetic secret key is only available at signing time (`finish_with_keys`). Recommended two-phase approach:
- `SilentPaymentSend::spend()` records `DeferredSpCrypto { scan_pk, spend_pk, input_hash, k, amount, memos }` into a new `Spends::silent_payments_pending` field, writes a placeholder puzzle hash into the `CreateCoin` condition.
- A new `Spends::finish_with_silent_payment_keys(ctx, deltas, relation, synthetic_pks, synthetic_sks)` resolves all pending entries, overwrites placeholder puzzle hashes, then runs the standard `finish_with_keys` signing path.
- `Spends::silent_payment_counters: HashMap<[u8; 48], u32>` tracks per-scan-pk `k` index for multi-output coordination.

This design decision is **flagged for validation in Phase 4** (design spike).

**SIM-01 helper placement:** `chia-sdk-test::silent_payments::extract_tweak_data_from_block()` behind a `chip-0057` feature on `chia-sdk-test`. Placing it there (not in driver's `#[cfg(test)]`) keeps it reachable from binding-side test code.

**Bindings pre-flight:** The `"type": "static_functions"` JSON schema pattern may not have a precedent in `bindy-macro`. Before Phase 5, read `crates/chia-sdk-bindings/bindy-macro/src/` to confirm. If unsupported, distribute free functions onto classes (`TweakData::scan_from_tweaks`, etc.).

Full architecture detail: `.planning/research/ARCHITECTURE.md`

### Pitfalls (Top 5)

Full list of 16 pitfalls with phase mapping: `.planning/research/PITFALLS.md`

1. **Signed-vs-unsigned scalar reduction** (Pitfall 1, CORRECTNESS) — Using the existing `mod_by_group_order` (signed) for protocol scalars produces undetectable payments for ~50% of hashes. TV1/TV3/TV4 do NOT catch this. Prevention: `ScalarField` newtype; adversarial `[0xff;32]` test. Phase 1.

2. **Synthetic vs raw key confusion** (Pitfall 2, API DESIGN -> CORRECTNESS) — `aggregate_sender_sks` must receive synthetic SKs (curried into p2 puzzles), not raw wallet keys. Sends succeed on-chain but payments are undetectable. Prevention: type-level `SyntheticSecretKey` newtype or documented param + SIM-02 catch. Phase 2/4.

3. **Multi-party aggregation silent fallback** (Pitfall 3, CORRECTNESS) — Using only local keys in a multi-party bundle produces undetectable payments. Offers are common on Chia. Prevention: hard-error `MultiPartyAggregationRequired` when input count exceeds local count. Phase 2/4.

4. **`k` iteration termination with labels** (Pitfall 7, CORRECTNESS) — The naive "break on first unlabeled miss" stops before checking labels at that k. Prevention: break only when both unlabeled AND all labeled candidates miss. K_max=2400 prevents DoS. Phase 3.

5. **`PublicKey::scalar_multiply` mutates in place** (Pitfall 9.1, CORRECTNESS) — `let result = pk.scalar_multiply(bytes)` binds `()`. Wrapper function makes this invisible. Phase 1.

---

## Cross-Cutting Concerns

These span multiple phases and must be tracked as first-class constraints throughout implementation. The roadmapper should record them as acceptance criteria on every affected milestone.

### 1. Signed-vs-unsigned scalar boundary

**Affects:** Phase 1 (ScalarField), Phase 2 (input_hash SEND-02), Phase 3 (output_tweak, label_scalar), all protocol functions.

`ScalarField::from_bytes_unsigned` is the only permitted mod-r reduction path inside `chip-0057` code. Enforce at code-review: `grep -r 'mod_by_group_order' crates/*/src/silent_payments/` must return zero hits.

### 2. Synthetic-vs-raw key boundary

**Affects:** Phase 2 (`aggregate_sender_sks`), Phase 4 (`SilentPaymentSend`), Phase 6 (simulator tests).

Input keys to `aggregate_sender_sks` must always be synthetic SKs. The simulator round-trip (Phase 6) is the canonical catch — it exercises the real signer path. Unit tests alone cannot catch this because the test author can just pass the right thing.

### 3. Multi-party aggregation hard error

**Affects:** Phase 2 (`aggregate_sender_sks`), Phase 4 (`Spends` integration).

When `Spends` has more inputs than the wallet locally controls, `SilentPaymentSend` must return `Err(DriverError::SilentPaymentMultiPartyUnsupported)`. No silent fallback. Required adversarial unit test: 2 inputs, 1 synthetic SK -> error.

### 4. Memo-position hint guard

**Affects:** Phase 2 (`SilentPaymentSend` struct definition), Phase 5 (bindings descriptor, doc-comment).

The first 32-byte memo entry on a `CREATE_COIN` is interpreted as a puzzle-hash hint by standard wallets and indexers. A user who attaches a 32-byte memo accidentally converts their silent-payment send to a public payment. `SilentPaymentSend` must either reject 32-byte memos or insert a non-hint at position 0.

### 5. CHIP-0058 forward-compatibility

**Affects:** Phase 3 (`TweakData` definition), Phase 5 (bindings serialization), Phase 7 (example code).

`TweakData` must not carry transport-layer fields (`height`, message envelope, sp-service JSON schema names). It is a plain Rust struct. The example must construct `TweakData` from the simulator helper only — not from a WS client.

### 6. m=0 change-label guard

**Affects:** Phase 2 (`labeled_address(m)` in `SilentPaymentKeys`), Phase 6 (labeled simulator test).

Label m=0 is reserved for change addresses. Wallets must never publish m=0 to external senders. The API must provide a `change_address()` method or emit a runtime check/warning when `labeled_address(0)` is called with external intent. The labeled simulator test (SIM-03) should include m=0 change detection.

---

## Implications for Roadmap

The Architecture research provides a 7-phase build order grounded in the dependency graph. This is the primary spine.

### Phase 1: Crypto Primitives (chia-sdk-types)

**Rationale:** Zero internal deps; everything else depends on this. A wrong scalar reduction here corrupts every downstream computation.

**Delivers:** `ScalarField` newtype, `tagged_hash` with all three `Chia_SP/*` tag constants, `SCAN_PATH`/`SPEND_PATH`, feature-flag cascade in types `Cargo.toml`.

**Requirements addressed:** CRYPTO-01, CRYPTO-02, WS-01 (types portion)

**Pitfalls addressed:** 1 (signed/unsigned), 4 (tag typos), 8 (HRP constants), 9 (chia-bls API wrappers), 10 (CHIP vs BIP-352 vectors)

**Non-negotiable acceptance criteria:**
- `ScalarField::from_bytes_unsigned([0xff; 32])` reduces to `r-1`, not `[0xff; 32]`
- CHIP TV1 partial intermediate values pass
- `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` returns zero hits

**Research flag:** Standard patterns. All API surfaces verified against on-disk crate source; no research-phase needed.

---

### Phase 2: Address and Key Types (chia-sdk-utils)

**Rationale:** Depends only on Phase 1. Having address encoding early means every subsequent phase can construct addresses from mnemonics.

**Delivers:** `SilentPaymentKeys` (mnemonic -> SKs, `unlabeled_address`, `labeled_address`, `from_secret_keys`), `SilentPaymentAddress` (encode/decode, HRP validation), m=0 change-label guard, feature-flag cascade in utils `Cargo.toml`.

**Requirements addressed:** ADDR-01, ADDR-02, ADDR-03, ADDR-04, WS-01 (utils portion)

**Pitfalls addressed:** 8 (bech32m checksum), 14 (separate accessors), 2 (partial — makes API testable with synthetic keys)

**Recommended additions (new REQ-IDs needed):** `SilentPaymentKeys::from_secret_keys`, `Label::CHANGE` constant + guard.

**Research flag:** Standard patterns. Follows existing `chia-sdk-utils::Address`/`Bech32` template exactly.

---

### Phase 3: Receive Primitive (chia-sdk-driver, no Spends touch)

**Rationale:** Can be developed in parallel with the Phase 4 design spike. Produces the transport-agnostic scanner as a pure function — no action system entanglement.

**Delivers:** `ecdh.rs`, `protocol.rs`, `aggregate.rs` (with hard-error multi-party check), `labels.rs` (`LabelRegistry` bidirectional), `tweak_data.rs` (`TweakData` plain struct), `scan_from_tweaks` (K_max=2400, identity guard, `?` on attacker-controlled bytes).

**Requirements addressed:** RECV-01, RECV-02, RECV-03, RECV-04, SEND-02 (compute_input_hash), SEND-03 (aggregate_sender_sks), CRYPTO-03 (full TV pass)

**Pitfalls addressed:** 3 (multi-party hard error), 5 (coin-ID lex ordering), 6 (ser32/ser256 endianness), 7 (k iteration with labels), 9 (chia-bls misuse), 13 (no RPC scanner), 15 (TweakData transport-agnostic)

**Non-negotiable acceptance criteria:**
- CHIP TV1 (unlabeled), TV3 (labeled), TV4 (multi-input) all pass end-to-end
- Custom k=1 test vector passes (required to catch ser32 endianness — TV1/TV3/TV4 all use k=0)
- Adversarial TweakData with `PublicKey::default()` tweak_point: no panic, no detections
- Adversarial TweakData with 10000 consecutive "matches": scanner stops at K_max=2400

**Research flag:** Needs attention on labeled k-iteration termination logic (Pitfall 7) — mirror `sp-client/src/scanner.rs` structure exactly.

---

### Phase 4: Send-Side Action (chia-sdk-driver, Spends integration)

**Rationale:** Depends on Phase 3 (ECDH primitives). Critical-path bottleneck: the deferred-ECDH design is the one unresolved architecture decision. Plan for a design spike at phase start.

**Delivers:** `derive_one_time_puzzle_hash` (SEND-01), `SilentPaymentSend impl SpendAction` (SEND-04), `Action::SilentPaymentSend` variant, `Spends::silent_payments_pending` field, `Spends::silent_payment_counters` field, `Spends::finish_with_silent_payment_keys`, multi-output `Vec<Recipient>` action shape.

**Requirements addressed:** SEND-01, SEND-04

**Pitfalls addressed:** 2 (synthetic key at signing time), 3 (multi-party check in Spends), 9.5 (deferred ECDH prevents premature coin id), 12 (action-system composition only)

**Recommended additions:** `SilentPaymentSend` accepts `Vec<Recipient>` for multi-output; memo-position hint guard; cross-index announcement binding (opcode 60/61).

**Research flag:** Needs design-spike time. The `Spends` integration is novel with no existing precedent. Write a minimal proof-of-concept before committing to full implementation.

---

### Phase 5: Bindings

**Rationale:** Mechanical re-export after Phases 2+3+4 stabilize. Bindings must follow the Rust API or they will churn (same sequencing used for CHIP-0037).

**Delivers:** `chia-sdk-bindings/src/silent_payments.rs` re-export facade, `bindings/silent_payments.json` descriptor, `Action::silent_payment_send` entry in `bindings/action_system.json`, per-target shim resolution.

**Requirements addressed:** BIND-01, BIND-02

**Pitfalls addressed:** 16 (PublicKey -> Uint8Array(48)/bytes(48) mapping), 11 (labeled-address linkability docstring), 12 (CAT doc warning), 13 (TweakData docstring)

**Pre-phase action item:** Read `crates/chia-sdk-bindings/bindy-macro/src/` to confirm static-functions schema support before writing the descriptor.

**Research flag:** Standard patterns — reuse existing `PublicKey` binding mapping. One spike needed on static-functions schema.

---

### Phase 6: Simulator Round-Trip

**Rationale:** Exercises the full real-on-chain flow including the actual signer path (the only reliable catch for the synthetic-vs-raw key confusion). Depends on Phases 4+5.

**Delivers:** `chia-sdk-test::silent_payments::extract_tweak_data_from_block` (SIM-01, chip-0057 gated on `chia-sdk-test`), unlabeled E2E test (SIM-02), labeled E2E test (SIM-03), AVA/pytest binding tests mirroring the Rust E2E (BIND-03).

**Requirements addressed:** SIM-01, SIM-02, SIM-03, BIND-03

**Pitfalls addressed:** 2 (catches synthetic-vs-raw through real signer), 3 (multi-party adversarial variant), 7 (labeled-at-k=1 in SIM-03)

**Research flag:** Standard patterns. Simulator test pattern established by dozens of existing driver tests.

---

### Phase 7: Example and Docs

**Rationale:** Proven flow (Phase 6) first. Example is the canonical reference for wallet authors.

**Delivers:** `examples/silent_payment.rs` (EX-01), `chia-sdk-driver` module-level docs mentioning `chip-0057`, prelude additions (gated), crate-level docstrings with CHIP §10 scan-key warning.

**Requirements addressed:** EX-01, WS-02, WS-03

**Research flag:** Standard patterns.

---

### Phase Ordering Rationale

- **Phases 1-3 are parallelizable with the Phase 4 design spike** as long as Phase 3 crypto functions stabilize before Phase 4 commits to full implementation.
- **Phase 4 is the critical-path bottleneck.** The deferred-ECDH decision is the only open architecture question. Schedule a design spike at Phase 4 start.
- **Phase 5 must follow Phase 4** to avoid descriptor churn.
- **Phase 6 requires Phase 5** because binding-side AVA/pytest tests need the descriptor.
- **Phases 1 and 2 can be executed together** if a single engineer is implementing.

### Research Flags

**Needs design-spike time during planning:**
- Phase 4: Deferred-ECDH architecture (Option A vs B)
- Phase 5: Confirm `bindy-macro` static-functions schema support
- Phase 3: Labeled k-iteration termination rule (Pitfall 7)

**Standard patterns (skip research-phase):**
- Phase 1: chia-bls API fully verified against on-disk source
- Phase 2: Follows existing `chia-sdk-utils::Address`/`Bech32` template
- Phase 6: Simulator test pattern well-established
- Phase 7: Example follows `examples/cat_spends.rs` template

---

## Looks-Done-But-Isn't Checklist

Preserved from PITFALLS.md for use as phase acceptance criteria. A phase milestone is not complete until its rows pass.

**Phase 1 gates:**
- [ ] `ScalarField` has `from_bytes_unsigned`; adversarial test with `[0xff; 32]` verifies reduction, not identity (Pitfall 1)
- [ ] Tagged-hash tags defined as single `const &str` per tag; CHIP TV1/TV3/TV4 pass (Pitfall 4)
- [ ] `PublicKey::scalar_multiply` wrapped so callers cannot accidentally bind `()` (Pitfall 9.1)
- [ ] No BIP-352 (secp256k1) test vectors in the test suite; all vectors use 48-byte BLS keys and `Chia_SP/` tags (Pitfall 10)
- [ ] `SilentPaymentAddress::decode` rejects bech32 (non-m), wrong HRP, wrong length; round-trip property test passes (Pitfall 8)

**Phase 2 gates:**
- [ ] `compute_input_hash` uses `iter().min()` for lex-smallest coin ID; TV4 multi-input test passes (Pitfall 5)
- [ ] At least one test uses k=1 for `derive_output_tweak` (not only k=0 which hides ser32 endianness) (Pitfall 6)
- [ ] `aggregate_sender_sks` hard-errors when called with fewer keys than bundle inputs; adversarial test returns `Err(MultiPartyAggregationRequired)` (Pitfall 3)

**Phase 3 gates:**
- [ ] Scanner does NOT break before checking labels at a missed unlabeled k; labeled-at-k=1 synthetic test passes (Pitfall 7)
- [ ] Scanner has `K_max = 2400` cap; malicious-TweakData test (10000 consecutive forged matches) stops at 2400 (Pitfall 7)
- [ ] Scanner uses `?` (not `.unwrap()`) on `PublicKey::from_bytes` for all TweakData inputs; malformed-48-byte test returns no panic (Pitfall 9.2)
- [ ] `TweakData` has no `height` field; no `sp-service` dep in any `chia-sdk-*` Cargo.toml (Pitfall 15)

**Phase 4 gates:**
- [ ] `SilentPaymentSend` composes through `Spends` + standard layer only; no CAT-layer composition path (Pitfall 12)
- [ ] 32-byte memo guard present; test that a 32-byte user memo does not silently become a puzzle-hash hint (memo-position concern)

**Phase 5 gates:**
- [ ] `bindings/silent_payments.json` uses existing `PublicKey` type (not a new `PublicKey48` alias); TS `index.d.ts` shows `scan_pk: Uint8Array` (48 bytes) (Pitfall 16)
- [ ] `SilentPaymentKeys` docstring references CHIP §10 on scan-key high-value-secret status (Pitfall 14)
- [ ] `derive_*` public helpers carry doc-comment warning about CAT undetectability (Pitfall 12)

**Phase 6 gates:**
- [ ] Unlabeled E2E (SIM-02): send -> farm -> extract TweakData -> scan -> spend; all steps pass against real Simulator (Pitfall 2 catch via real signer)
- [ ] Labeled E2E (SIM-03): same with labeled address; includes k=1 output and m=0 change detection (Pitfall 7, m=0 guard)

---

## Consolidated Open Questions

| # | Question | Source | When to Resolve |
|---|----------|--------|-----------------|
| Q1 | **Option A vs B for deferred ECDH:** `Spends::finish_with_silent_payment_keys` (Option A, recommended) vs `SilentPaymentSend::new` requires aggregated SK at construction (Option B)? | ARCHITECTURE.md | Start of Phase 4 design spike |
| Q2 | **`SyntheticSecretKey` newtype vs documented parameter name:** Type-level distinction (Option A, recommended) vs documented + debug-asserted `&[SecretKey]` (Option B)? | PITFALLS.md Pitfall 2 | Phase 2/4 design |
| Q3 | **`bindy-macro` static-functions schema:** Does the JSON schema support a zero-field class with only static methods? If not, what is the correct attachment point for `scan_from_tweaks`, `derive_one_time_puzzle_hash`, `compute_input_hash`? | ARCHITECTURE.md | Before Phase 5 starts |
| Q4 | **`Vec<Recipient>` vs single-recipient action shape:** Multi-output via `Vec<Recipient>` (features research recommendation) vs multiple actions + shared `Spends` counter state? | FEATURES.md, ARCHITECTURE.md | Phase 4 design |
| Q5 | **Cross-index announcement binding REQ-ID:** Features research recommends opcode 60/61 emission as a v1 correctness property. PROJECT.md does not list it. New REQ-ID or extension of SEND-04? | FEATURES.md, PROJECT.md | Roadmapper / project owner |
| Q6 | **`from_secret_keys` REQ-ID:** Low complexity (~5 lines), enables watch-only wallets. PROJECT.md ADDR-01 does not include it. Add as explicit requirement? | FEATURES.md, PITFALLS.md Pitfall 2 | Roadmapper / project owner |
| Q7 | **Bindings always-on vs feature-gated per binding crate:** Do `napi/`, `pyo3/`, `wasm/` enable `chip-0057` by default or behind an explicit feature? | ARCHITECTURE.md | Before Phase 5 |
| Q8 | **`chia-sdk-utils -> chia-sdk-types` new dep edge:** Optional dep activated by `chip-0057` is safe and confirmed one-directional. Any downstream consumer breakage to audit? | ARCHITECTURE.md | Before Phase 2 |

---

## Pitfall-to-Phase Mapping Table

| Pitfall | Severity | Phase | Verification |
|---------|----------|-------|--------------|
| 1. Signed/unsigned scalar reduction | CRITICAL | Phase 1 | `ScalarField` newtype; adversarial `[0xff;32]` test; TV1/TV3/TV4 intermediate values |
| 2. Synthetic vs raw key confusion | CRITICAL | Phase 2 + Phase 4 | `SyntheticSecretKey` newtype OR documented param + SIM-02 real-signer catch |
| 3. Multi-party aggregation silent fallback | CRITICAL | Phase 2 + Phase 4 | Unit test: 2 inputs, 1 SK -> `Err(MultiPartyAggregationRequired)` |
| 4. Tagged-hash domain-tag typos | CRITICAL | Phase 1 | Single `const &str` per tag; TV1/TV3/TV4 pass |
| 5. Coin-ID lexicographic ordering | CRITICAL | Phase 3 | TV4 passes; order-independence property test |
| 6. `ser32`/`ser256` endianness | CRITICAL | Phase 3 + Phase 2 (labels) | Custom k=1 test vector; TV3 label_scalar pin |
| 7. `k` iteration termination (labeled) | HIGH | Phase 3 | Labeled-at-k=1 synthetic test; K_max=2400 DoS test |
| 8. Bech32m HRP / checksum | MEDIUM | Phase 2 | Round-trip test; wrong-HRP / non-m negative tests |
| 9. chia-bls API misuse | HIGH | Phase 1 + Phase 3 | Wrapper fns; malformed TweakData adversarial test |
| 10. BIP-352 vs CHIP test vectors | LOW (CI) | Phase 1 | CHIP TVs only; review checklist |
| 11. Labeled-address linkability | LOW (docs) | Phase 5 + Phase 7 | Documentation review |
| 12. CAT2 silent-payment footgun | HIGH | Phase 4 + Phase 5 | Action-system-only composition; doc warnings |
| 13. Full-node query privacy leak | LOW (docs) | Phase 3 docs | No RPC-backed scan in SDK; TweakData docstring |
| 14. Scan-key compromise | MEDIUM (docs) | Phase 2 | Separate `scan_sk()`/`spend_sk()` accessors; docstring |
| 15. `sp-service` wire format lock-in | MEDIUM | Phase 3 | No `height` field; no sp-service dep |
| 16. Bindings type-mapping (PublicKey 48-byte) | MEDIUM | Phase 5 | BIND-03 round-trip tests in each target language |

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All deps verified against on-disk crate source at `~/.cargo/registry/src/`. No new deps needed. |
| Features | HIGH (protocol-derived) / MEDIUM (Bitcoin prior-art UX) | Protocol features from CHIP spec and prototype are HIGH. Bitcoin wallet UX patterns from WebSearch only (MEDIUM), but not material for SDK API shape. |
| Architecture | HIGH | Derived from direct reads of SDK crate sources, action system code, existing CHIP-0035/0037 integration points. One open design decision (Option A vs B) but both options are validated as viable. |
| Pitfalls | HIGH | 16 pitfalls, all verified against prototype, CHIP draft sections 4-11, and CONCERNS.md. All dangerous pitfalls (1, 2, 3, 7) have concrete test strategies. |

**Overall confidence: HIGH**

### Gaps to Address

- **Option A vs B for deferred ECDH** (Q1): Needs a proof-of-concept at Phase 4 start. Both paths are architecturally sound.
- **`bindy-macro` static-functions schema** (Q3): Needs code reading at Phase 5 start. Fallback strategy documented in ARCHITECTURE.md.
- **Announcement binding REQ-ID** (Q5): Cross-index opcode 60/61 feature recommended by Features research but absent from PROJECT.md. Roadmapper needs to decide.
- **`from_secret_keys` REQ-ID** (Q6): Low complexity, high value. Roadmapper needs to add explicit requirement or confirm it falls under ADDR-01 extension.

---

## Sources

### Primary (HIGH confidence)
- `~/silent-payments/chip-silent-payments.md` — CHIP-0057 draft spec; tagged hash, test vectors TV1-TV4, address encoding, security analysis
- `~/.cargo/registry/src/.../chia-bls-0.36.1/src/` — all API surfaces verified on-disk
- `~/.cargo/registry/src/.../chia-puzzle-types-0.36.1/src/` — `DeriveSynthetic`, `StandardArgs::curry_tree_hash`, signed `mod_by_group_order`
- `~/silent-payments/crates/sp-common/src/*.rs` — working Rust reference implementation
- SDK workspace sources (`crates/chia-sdk-types/`, `chia-sdk-utils/`, `chia-sdk-driver/`, `chia-sdk-bindings/`) — feature-gate patterns, action system, Bech32 helper, module layout conventions
- `.planning/PROJECT.md` — locked v1 scope, constraints, key decisions
- `.planning/codebase/CONCERNS.md`, `ARCHITECTURE.md`, `STRUCTURE.md`, `CONVENTIONS.md` — SDK-specific design hazards and conventions

### Secondary (MEDIUM confidence)
- BIP-352 spec — protocol structure cross-check (not test vectors; different curve)
- Bitcoin silent-payments wallet survey (silentpayments.xyz, Bitcoin Core, Cake, Sparrow, Silentium) — prior-art UX patterns
- `~/silent-payments/crates/sp-client/src/scanner.rs` — reference k-iteration termination rule

### Tertiary (LOW confidence)
- WebSearch results for Rust BIP-352 crates (2026-05-15) — confirmed all are secp256k1-only; no BLS12-381 silent-payments crate exists

---
*Research completed: 2026-05-15*
*Ready for roadmap: yes*
