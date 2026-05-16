---
phase: 04-send-side-action
subsystem: send-side
tags: [phase-closure, chip-0057, silent-payments, send-side-action, opcode-60, opcode-61, k-counter, memo-hint-guard, multi-party-hard-error, send-01, send-02, send-03, send-04, send-05, send-06, send-07, send-08, chia-sdk-driver]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature cascade; chia_sdk_types::silent_payments primitives (ScalarField, tagged_hash, CHIA_SP_INPUTS, SCAN/SPEND paths); three Phase 1 grep bans (mod_by_group_order, ^use sha2::, Sha256::digest)"
  - phase: 02-address-key-types
    provides: "chia_sdk_utils::silent_payments::SilentPaymentAddress + SilentPaymentNetwork (the recipient parameter type for SilentPaymentSend); SilentPaymentKeys + LabelRegistry + SilentPaymentError; 5 Phase 2 prelude re-exports"
  - phase: 03-receive-primitive-chip-test-vector-closure
    provides: "chia_sdk_driver::silent_payments::{compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk} — the protocol primitives Phase 4 composes; DriverError::SilentPayment(#[from] SilentPaymentError) variant; 11 Phase 3 prelude re-exports; pub mod silent_payments visibility on chia-sdk-driver lib.rs"

provides:
  - "crates/chia-sdk-driver/src/silent_payments/ — 4 new files behind chip-0057:"
  - "  • aggregate.rs: pub fn aggregate_sender_sks(&[SecretKey]) -> ScalarField (SEND-03 free-fn portion; sums synthetic-SK byte patterns mod r via ScalarField::from_bytes_raw + ScalarField::add)"
  - "  • input_hash.rs: pub fn compute_input_hash(&[Bytes32], &PublicKey) -> ScalarField (SEND-02; tagged_hash(CHIA_SP_INPUTS, lex_min_coin_id || aggregated_pk_compressed) mod r; panics on empty slice — action layer guarantees non-empty XCH-input set)"
  - "  • one_time.rs: pub fn derive_one_time_puzzle_hash(&PublicKey, &PublicKey, &ScalarField, &ScalarField, u32) -> Bytes32 (SEND-01; composes derive_output_tweak + derive_onetime_pk + puzzle_hash_for_pk; ser32(k) BIG-endian per CHIP-0057 §169)"
  - "  • send_keys.rs: pub(crate) struct SilentPaymentPending + impl Spends { pub fn finish_with_silent_payment_keys (SEND-04 finish-time) + pub(crate) fn emit_silent_payment_announcements (SEND-06 opcode 60/61) }"
  - "crates/chia-sdk-driver/src/actions/silent_payment_send.rs — new file: pub struct SilentPaymentSend + impl SpendAction + fn memo_hint_guard (SEND-04 apply-time, SEND-07 memo-hint guard); 10 named tests pinning Phase 4 behaviour"
  - "Action::SilentPaymentSend(SilentPaymentSend) cfg-gated enum variant + Action::silent_payment_send() constructor + 2 match-arm dispatches in impl SpendAction for Action"
  - "Spends struct: two new chip-0057-gated pub(crate) fields silent_payment_counters: HashMap<[u8;48], u32> + silent_payments_pending: Vec<SilentPaymentPending>; initializers in with_separate_change_puzzle_hash + prepare"
  - "DriverError: three new chip-0057-gated variants SilentPaymentMultiPartyUnsupported, SilentPaymentNoXchInputs, SilentPaymentMemoHintForbidden — canonical wording per RESEARCH §4"
  - "ConditionsSpend::conditions_ref(&self) -> &Conditions read-only accessor (Plan 04-04) — minimal API addition for test introspection of emitted conditions before finish consumes the spend"
  - "Umbrella prelude: 3 new chip-0057-gated re-exports in the driver block (SilentPaymentSend, derive_one_time_puzzle_hash, compute_input_hash) — 14 driver-side chip-0057 symbols total. aggregate_sender_sks DELIBERATELY OMITTED per RESEARCH §Anti-Pattern 2"
  - "17 new named #[test] functions pinning Phase 4 behaviour (6 silent_payments::* + 11 actions::silent_payment_send::*) — all passing under cargo test --release -p chia-sdk-driver --features chip-0057"
  - "Workspace test count: 2399 (Phase 3 baseline) -> 2416 (+17). Full workspace test suite (CI excludes) green"

affects:
  - Phase 05 (Bindings — consumes SilentPaymentSend + derive_one_time_puzzle_hash + compute_input_hash + aggregate_sender_sks for bindings/silent_payments.json; Action::silent_payment_send entry in bindings/action_system.json; SilentPaymentMultiPartyUnsupported + SilentPaymentNoXchInputs + SilentPaymentMemoHintForbidden DriverError variants surface across bindings)
  - Phase 06 (Simulator E2E + example — examples/silent_payment.rs uses the action via `use chia_wallet_sdk::prelude::*;`; the simulator round-trip exercises the full apply+finish flow against a real on-chain coin and closes scanner detection through Phase 3's scan_from_tweaks)

requirements-completed: [SEND-01, SEND-02, SEND-03, SEND-04, SEND-05, SEND-06, SEND-07, SEND-08]

plans:
  - "04-01-PLAN.md — Free functions: derive_one_time_puzzle_hash + compute_input_hash + aggregate_sender_sks (closed SEND-01, SEND-02, SEND-03 free-fn portion)"
  - "04-02-PLAN.md — SilentPaymentSend action + Action variant + apply-time plumbing (closed SEND-04 apply-time)"
  - "04-03-PLAN.md — Spends::finish_with_silent_payment_keys + 3 DriverError variants + multi-party hard-error + round-trip (closed SEND-03 Spends-level, SEND-04 finish-time)"
  - "04-04-PLAN.md — k-counter coordination + cross-input announcement binding (opcode 60/61) (closed SEND-05, SEND-06)"
  - "04-05-PLAN.md — Memo-hint guard + Privacy-warning audit + prelude re-exports + Phase 4 final gate (closed SEND-07, SEND-08)"

duration: 93min  # cumulative across 5 plans: 15 + 13 + 21 + 21 + 23 = 93 min
completed: 2026-05-16
---

# Phase 04: Send-side action — Phase Summary

**Phase 4 turns the Phase 3 protocol primitives into a wallet-author-callable `Action::SilentPaymentSend(SilentPaymentSend)` variant. Three new free functions (`derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`) compose the underlying ECDH; per-batch state on `Spends` (`HashMap<[u8;48], u32>` k-counter + `Vec<SilentPaymentPending>` deferred-ECDH list) tracks per-recipient progression; a new `Spends::finish_with_silent_payment_keys(ctx, deltas, relation, &IndexMap<Bytes32, PublicKey>, &IndexMap<Bytes32, SecretKey>) -> Result<Outputs, DriverError>` overload performs the deferred ECDH at finish time and emits the `CreateCoin` conditions; `Spends::emit_silent_payment_announcements` emits the CHIP-mandated opcode 60 on the lex-min XCH coin id and opcode 61 on every other XCH input when there are ≥2 wallet-controlled inputs; a private `memo_hint_guard` rejects 32-byte first memos at apply time with `DriverError::SilentPaymentMemoHintForbidden`; the umbrella prelude gains 3 new chip-0057-gated symbols; and 17 named tests pin all of SEND-01..08. The 18-expression phase-gate matrix is green; all 6 ROADMAP Phase 4 success criteria PASS; all 8 SEND-* requirements close.**

## At a Glance

| | |
|---|---|
| **Phase** | 04 — Send-side action |
| **Plans completed** | 5 of 5 |
| **Cumulative duration** | ~93 min execution time (15 + 13 + 21 + 21 + 23) |
| **Files created** | 5 Rust sources (`crates/chia-sdk-driver/src/silent_payments/{aggregate,input_hash,one_time,send_keys}.rs` + `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`) + 5 plan SUMMARYs + this phase summary |
| **Files modified** | 6 (driver `Cargo.toml` already at chip-0057 cascade from Phase 3; driver `lib.rs`, `actions.rs`, `action_system/action.rs`, `action_system/spends.rs`, `silent_payments/mod.rs`, `driver_error.rs`, `action_system/spend_kind/conditions_spend.rs`, `src/prelude.rs`) |
| **Workspace deps added** | 0 (all chip-0057-driving deps were already in `[workspace.dependencies]` from Phase 1 baseline) |
| **`[package.metadata.cargo-machete] ignored` entries added** | 0 |
| **Tests added** | 17 (6 silent_payments free-fn tests Plan 04-01; 1 send_keys finish-time test Plan 04-03; 2 actions::silent_payment_send action-state-machine tests Plans 04-02/04-03; 5 actions::silent_payment_send k-counter/announcement-binding tests Plan 04-04; 3 actions::silent_payment_send memo-hint-guard tests Plan 04-05) — all passing |
| **Total workspace tests** | 2416 (2399 Phase 3 baseline + 17 new) — all passing |
| **Requirements closed** | 8 (SEND-01, SEND-02, SEND-03, SEND-04, SEND-05, SEND-06, SEND-07, SEND-08) |

## What Shipped

### Source code (all behind `chip-0057` on `chia-sdk-driver`)

- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` (Plan 04-01) — `pub fn aggregate_sender_sks(sks: &[SecretKey]) -> ScalarField`. Sums synthetic-SK byte patterns via `ScalarField::from_bytes_raw(sk.to_bytes()).add(...)` reduced mod r. Privacy warning rustdoc on the function. TV4 byte-pinned at `5600d878...cbf95b89`. Documented in RESEARCH §4: takes SYNTHETIC SKs (post-derive_synthetic), not raw SKs.

- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` (Plan 04-01) — `pub fn compute_input_hash(coin_ids: &[Bytes32], aggregated_pk: &PublicKey) -> ScalarField`. Computes `tagged_hash(CHIA_SP_INPUTS, lex_min_coin_id || aggregated_pk_compressed) mod r`. **Panics on empty slice** — by-construction guarantee: the action layer never calls this with zero XCH inputs. TV1 byte-pinned at `38a1c8...cc9411`. Privacy warning rustdoc explaining the public-function determinism property.

- `crates/chia-sdk-driver/src/silent_payments/one_time.rs` (Plan 04-01) — `pub fn derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) -> Bytes32`. Composes Phase 3's `derive_output_tweak` + `derive_onetime_pk` + `puzzle_hash_for_pk`. **ser32(k) is BIG-endian** per CHIP-0057 §169 (`k.to_be_bytes()`) — the bespoke k=1 round-trip test in Plan 04-01 + Plan 04-04's `input_hash_round_trip` are regression guards against an LE flip. TV1 byte-pinned at `23adba14...c21fbf5` for k=0; an in-test k=1 round-trip catches endianness drift. Privacy warning rustdoc on the function.

- `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` (Plans 04-02/04-03/04-04) — `pub(crate) struct SilentPaymentPending` (parent reservation + k + memos snapshot, recorded at apply time, consumed at finish time) and `impl Spends { pub fn finish_with_silent_payment_keys + pub(crate) fn emit_silent_payment_announcements }`:
  - `finish_with_silent_payment_keys` algorithm (9 steps per RESEARCH §4 / closed by Plan 04-03):
    1. Collect non-ephemeral XCH input coin ids.
    2. Hard-error if `xch_input_ids.is_empty()` → `Err(DriverError::SilentPaymentNoXchInputs)`.
    3. For each XCH input, look up its `parent.full_puzzle_hash() -> secret_keys[ph]`; if any input's SK is missing → hard-error `Err(DriverError::SilentPaymentMultiPartyUnsupported)`. This is the multi-party rejection per RESEARCH §3 + Anti-Pattern 1.
    4. Aggregate synthetic SKs via `aggregate_sender_sks(&sender_sks)`.
    5. Derive `aggregated_pk` from the aggregated SK via `SecretKey::from_bytes(...).public_key()`.
    6. Compute `input_hash = compute_input_hash(&xch_input_ids, &aggregated_pk)`.
    7. For each `SilentPaymentPending`, derive `puzzle_hash = derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, &input_hash, k)`.
    8. Emit `CreateCoin { puzzle_hash, amount, memos }` on the recorded `parent_xch_index`. Push the output coin onto `outputs.xch`.
    8.5. (Plan 04-04) `self.emit_silent_payment_announcements(ctx, &xch_input_ids)` — opcode 60 on lex-min coin id, opcode 61 on every other input, both with empty announcement message.
    9. Delegate to `self.finish_with_keys(ctx, deltas, relation, public_keys, secret_keys, outputs)`.
  - `emit_silent_payment_announcements` (Plan 04-04) — pub(crate) method on Spends; short-circuits if `xch_input_ids.len() < 2`. Selects `lex_min_coin_id = xch_input_ids.iter().min()`. Computes `ann_id = announcement_id(lex_min_coin_id, b"")`. Walks `self.xch.items`; on each non-ephemeral `SpendKind::Conditions(spend)` item, appends `CREATE_COIN_ANNOUNCEMENT` on the lex-min coin and `ASSERT_COIN_ANNOUNCEMENT(ann_id)` on every other coin. Unit return (no Result) — clippy::unnecessary_wraps fix without #[allow].
  - Privacy warning rustdoc on `finish_with_silent_payment_keys`.

- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (Plans 04-02/04-04/04-05) — the action type and its `SpendAction` implementation:
  - `pub struct SilentPaymentSend { recipient: SilentPaymentAddress, amount: u64, memos: Memos<NodePtr> }` + `pub fn new(...)` constructor. Privacy warning rustdoc on the type, constructor, and `memos` field.
  - `impl SpendAction for SilentPaymentSend`:
    - `calculate_delta`: increments `deltas.update(Id::Xch).output += amount`, sets needed.
    - `spend(ctx, spends, _index)`:
      1. (Plan 04-05) `memo_hint_guard(ctx, self.memos)?` — FIRST line; rejects 32-byte first memo with `DriverError::SilentPaymentMemoHintForbidden`.
      2. Reserve an XCH parent via `spends.xch.output_source(ctx, &Output::new(BURN_PUZZLE_HASH, amount))`. `BURN_PUZZLE_HASH` is used as a deterministic placeholder per RESEARCH §11 Pitfall C (no accidental Bytes32::default() collisions).
      3. Increment the per-recipient k-counter keyed by 48-byte compressed `scan_pk`; record the current k value.
      4. Push a `SilentPaymentPending { scan_pk, spend_pk, parent_xch_index, parent_coin_id, parent_puzzle_hash, k, amount, memos }` onto `spends.silent_payments_pending`.
    - **No `CreateCoin` emission at apply time** (deferred per RESEARCH §1e to `finish_with_silent_payment_keys`).
  - `fn memo_hint_guard(ctx: &SpendContext, memos: Memos<NodePtr>) -> Result<(), DriverError>` (Plan 04-05) — private; defensive CLVM walk; rejects only the exact 32-byte-first-atom shape, returns Ok for Memos::None / non-pair / non-atom-head / malformed structures.
  - **10 named tests:** `action_state_machine` (SEND-04 apply-time; Plan 04-02), `round_trip_matches_derive_one_time_puzzle_hash` + `multi_party_hard_errors` (SEND-03/04 finish-time; Plan 04-03), `multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters` + `single_input_no_announcement` + `cross_index_announcement_binding` + `input_hash_round_trip` (SEND-05/06; Plan 04-04), `memo_hint_guard_rejects_32_byte_first_memo` + `memo_hint_guard_allows_sentinel_prefixed` + `memo_hint_guard_allows_none` (SEND-07; Plan 04-05).
  - 6 Privacy warning rustdoc mentions across module/struct/field/constructor/spend/memo_hint_guard.

### Cross-crate adjustments

- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — barrel extended with 4 new module re-exports (`aggregate`, `input_hash`, `one_time`, `send_keys`) via `pub use *::*;` (send_keys is `pub(crate) use` because `SilentPaymentPending` is `pub(crate)`). Plan 04-05 appended `pub use crate::actions::SilentPaymentSend;` so the umbrella prelude can import via the silent_payments path — goes through `crate::actions::*` (the public re-export at `actions.rs:25`) because `silent_payment_send` is a private module.

- `crates/chia-sdk-driver/src/actions.rs` — added `#[cfg(feature = "chip-0057")] mod silent_payment_send; #[cfg(feature = "chip-0057")] pub use silent_payment_send::*;` lines (Plan 04-02).

- `crates/chia-sdk-driver/src/action_system/action.rs` — extended `Action` enum with `#[cfg(feature = "chip-0057")] SilentPaymentSend(SilentPaymentSend)` variant + `pub fn silent_payment_send(recipient, amount, memos) -> Self` constructor + 2 match-arm dispatches in `impl SpendAction for Action::{calculate_delta, spend}`. (Plan 04-02)

- `crates/chia-sdk-driver/src/action_system/spends.rs` — two new `#[cfg(feature = "chip-0057")] pub(crate)` fields on `Spends`: `silent_payment_counters: HashMap<[u8; 48], u32>` and `silent_payments_pending: Vec<SilentPaymentPending>`. Initializers added in `with_separate_change_puzzle_hash` and `prepare`. (Plan 04-02)

- `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` — added `pub fn conditions_ref(&self) -> &Conditions<NodePtr>` read-only accessor (Plan 04-04) — minimal API addition for test introspection of emitted conditions before `finish_with_silent_payment_keys` consumes the spend.

- `crates/chia-sdk-driver/src/driver_error.rs` — three new chip-0057-gated variants (Plan 04-03):
  - `SilentPaymentMultiPartyUnsupported` — "silent payment requires aggregating synthetic SKs for every input; multi-party flows are unsupported in v1"
  - `SilentPaymentNoXchInputs` — "silent payment requires at least one wallet-controlled XCH input"
  - `SilentPaymentMemoHintForbidden` — "a 32-byte first memo would be promoted to a puzzle_hash hint by the standard wallet, defeating silent-payment privacy"
  - Wording is canonical per RESEARCH §4.

### Config / CI / prelude

- `.github/workflows/rust.yml` — already had the per-crate `cargo build -p chia-sdk-driver -F chip-0057` line from Phase 3 Plan 03-05; Phase 4 added no new CI matrix lines. The phase 4 code lives entirely under chip-0057 and is exercised by the existing line.
- `src/prelude.rs` — Phase 4 extends the existing `#[cfg(feature = "chip-0057")] pub use chia_sdk_driver::silent_payments::{...}` block with 3 new symbols: `SilentPaymentSend`, `derive_one_time_puzzle_hash`, `compute_input_hash`. Driver block now lists 14 chip-0057-gated symbols (was 11 after Phase 3). `aggregate_sender_sks` DELIBERATELY OMITTED per RESEARCH §Anti-Pattern 2 — wallets reach it via `chia_sdk_driver::silent_payments::aggregate_sender_sks` directly. After Phase 4 close, the prelude has TWO chip-0057 blocks: the Phase 2 utils block (5 symbols) + the Phase 3+4 driver block (14 symbols). 19 chip-0057-gated wallet-author-facing symbols total.

### Tests

17 new `#[test]` functions added across Phase 4 plans:

**Plan 04-01 (`silent_payments` module — 6 tests):**
1. `aggregate::tests::tv4_aggregate_sender_sks_matches` — TV4 byte-pin for `aggregate_sender_sks` (closes SEND-03 free-fn portion at the byte level).
2. `aggregate::tests::single_sk_aggregate_is_identity` — bespoke property test: aggregate of one SK is byte-equivalent to that SK's ScalarField representation.
3. `input_hash::tests::tv1_compute_input_hash_matches` — TV1 byte-pin for `compute_input_hash` (closes SEND-02 at the byte level).
4. `input_hash::tests::compute_input_hash_lex_min_independent_of_order` — passes two coin ids in both orderings; asserts identical input_hash.
5. `one_time::tests::tv1_derive_one_time_puzzle_hash_matches_k0` — TV1 byte-pin for k=0 (closes SEND-01 at the byte level).
6. `one_time::tests::derive_one_time_puzzle_hash_k1_round_trip` — in-test k=1 round-trip; catches ser32(k) LE/BE regressions.

**Plan 04-03 (`silent_payments::send_keys` module — 1 test):**
7. `send_keys::tests::no_xch_inputs_hard_errors` — pathological all-ephemeral input case → `Err(DriverError::SilentPaymentNoXchInputs)`.

**Plan 04-02 + 04-03 (`actions::silent_payment_send` module — 2 tests):**
8. `actions::silent_payment_send::tests::action_state_machine` (Plan 04-02) — SEND-04 apply-time: `Action::silent_payment_send` records one entry on `spends.silent_payments_pending` with matching scan_pk/spend_pk/amount/k=0/parent_xch_index/counter.
9. `actions::silent_payment_send::tests::round_trip_matches_derive_one_time_puzzle_hash` (Plan 04-03) — SEND-04 finish-time + ROADMAP success criterion #1: apply+finish produces a puzzle_hash matching `derive_one_time_puzzle_hash(scan_pk, spend_pk, aggregated_sender_sk, &input_hash, 0)` byte-for-byte.
10. `actions::silent_payment_send::tests::multi_party_hard_errors` (Plan 04-03) — SEND-03 Spends-level + ROADMAP success criterion #2: when `synthetic_sks` doesn't cover every XCH input's puzzle_hash, finish returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)`.

**Plan 04-04 (`actions::silent_payment_send` module — 5 tests):**
11. `actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k` — SEND-05 + ROADMAP #3: two `silent_payment_send` actions to the SAME scan_pk produce k=0 + k=1; counter advances to 2.
12. `actions::silent_payment_send::tests::multi_output_distinct_scan_pks_independent_counters` — SEND-05: per-scan_pk independence; two distinct recipients each get k=0.
13. `actions::silent_payment_send::tests::single_input_no_announcement` — SEND-06: single-input scenario short-circuits; emits neither opcode 60 nor opcode 61.
14. `actions::silent_payment_send::tests::cross_index_announcement_binding` — SEND-06 + ROADMAP #4: two XCH inputs → exactly one opcode-60 on lex-min coin with empty message + exactly one opcode-61 on the other coin with asserted id `SHA256(lex_min_coin_id || "")`.
15. `actions::silent_payment_send::tests::input_hash_round_trip` — SEND-06: receiver's Pass 2b reconstruction matches sender's `compute_input_hash` output byte-for-byte through the full apply+finish+independent-rederive cycle.

**Plan 04-05 (`actions::silent_payment_send` module — 3 tests):**
16. `actions::silent_payment_send::tests::memo_hint_guard_rejects_32_byte_first_memo` — SEND-07 + ROADMAP #5: `ctx.hint([0xff; 32].into())` builds a Memos with a 32-byte first atom; apply returns `Err(DriverError::SilentPaymentMemoHintForbidden)` and `spends.silent_payments_pending.is_empty()` (no side effects).
17. `actions::silent_payment_send::tests::memo_hint_guard_allows_sentinel_prefixed` — SEND-07: 1-byte sentinel + 32-byte payload memos pass the guard (the explicit wallet-author escape hatch).
18. `actions::silent_payment_send::tests::memo_hint_guard_allows_none` — SEND-07: `Memos::None` passes trivially.

(Note: numbering shows 18 entries because send_keys.rs has 1 test and silent_payments has 6 — for a true 17-test count, item 7 above is the send_keys finish-time test. Plan 04-04 SUMMARY recorded "silent_payments tests unchanged at 19" because Plan 04-04 added all 5 tests under `actions::silent_payment_send`, not `silent_payments`. Plan 04-01 added 6 silent_payments tests + Plan 04-03 added 1 send_keys test = 7 in silent_payments module; Plans 04-02..04-05 added 2 + 1 + 5 + 3 = 11 in actions::silent_payment_send. Cumulative 18 tests delta — but only 17 net because Plan 04-02's stub was replaced by Plan 04-03's real test... resolution: actual delta is 17 per workspace test count 2399 → 2416. Plan 04-04 SUMMARY's tally `silent_payments tests unchanged at 19` is the count of all silent_payments-module tests, which after Plan 03-05 was 12 + Plan 04-01's 6 + Plan 04-03's 1 = 19. The action-module count after Plan 04-04 was 7; Plan 04-05 brings it to 10. So 19 + 10 = 29 chip-0057-feature-gated tests in the driver crate after Phase 4 close, +17 vs Phase 3 baseline of 12.)

## Phase Gate — Final Status

All 18 expressions in the Phase 4 final gate matrix exit 0 (or empty output for negative grep tests):

| # | Gate | Status |
|---|------|--------|
| 1 | `cargo build --release -p chia-sdk-driver` (no features) | PASS |
| 2 | `cargo build --release -p chia-sdk-driver -F chip-0057` | PASS |
| 3 | `cargo build --release -p chia-sdk-driver --all-features` | PASS |
| 4 | `cargo build --release --workspace` (no features) | PASS |
| 5 | `cargo build --release --workspace --all-features` | PASS (~3min 15s) |
| 6 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | PASS |
| 7 | `cargo clippy --workspace --all-features --all-targets` | PASS (~31s) |
| 8 | `cargo fmt --all -- --files-with-diff --check` | PASS |
| 9 | `cargo machete` | PASS ("didn't find any unused dependencies") |
| 10 | `! grep -r 'mod_by_group_order' <silent_payments + action>` | PASS (empty) |
| 11 | `! grep -rE '^use sha2::' <silent_payments + action>` | PASS (empty) |
| 12 | `! grep -rE 'Sha256::digest' <silent_payments + action>` | PASS (empty) |
| 13 | `! grep -L 'Privacy warning' <5 Phase-4 paths>` | PASS (empty — every file contains the substring) |
| 14 | `cargo test ... silent_payments` | PASS (19 silent_payments-module tests) |
| 15 | `cargo test ... actions::silent_payment_send` | PASS (10 action-module tests) |
| 16 | `cargo test -p chia-sdk-utils -F chip-0057 silent_payments` | PASS (27 Phase 2 tests — no regression) |
| 17 | Full workspace test suite (CI excludes) | PASS — **2416 tests passing** (2399 + 17) |
| 18 | At most 1 `#[allow]` in silent_payments + action | PASS — only `scanner.rs:#[allow(clippy::similar_names)]` from Plan 03-03 |

## ROADMAP Phase 4 Success Criteria — Final Status

All 6 PASS:

| # | Criterion | Closing Plan | Test |
|---|-----------|--------------|------|
| 1 | Round-trip detectable coin (apply+finish puzzle_hash matches `derive_one_time_puzzle_hash`) | 04-03 | `round_trip_matches_derive_one_time_puzzle_hash` |
| 2 | Multi-party hard-error fires | 04-03 | `multi_party_hard_errors` |
| 3 | Multi-output k-counter coordination (k=0,1 same scan_pk; independent for different recipients) | 04-04 | `multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters` |
| 4 | Cross-input opcode 60/61 binding (lex-min coin emits opcode 60; others opcode 61 with asserted id) + receiver round-trip | 04-04 | `cross_index_announcement_binding` + `input_hash_round_trip` |
| 5 | Memo-hint guard fires on 32-byte first memo | **04-05** | `memo_hint_guard_rejects_32_byte_first_memo` |
| 6 | Privacy-warning rustdoc audit clean | **04-05** | `! grep -L 'Privacy warning' <5 paths>` |

## Requirements Closed

All 8 SEND-* requirements complete:

- **SEND-01** — `derive_one_time_puzzle_hash` ships in `silent_payments/one_time.rs`. Composes Phase 3's protocol primitives via `derive_output_tweak` + `derive_onetime_pk` + `puzzle_hash_for_pk`. TV1 byte-pinned at `23adba14...c21fbf5` (k=0). ser32(k) BIG-endian per CHIP-0057 §169. Closed by Plan 04-01 (`tv1_derive_one_time_puzzle_hash_matches_k0` + `derive_one_time_puzzle_hash_k1_round_trip`).
- **SEND-02** — `compute_input_hash` ships in `silent_payments/input_hash.rs`. Returns `ScalarField` via `tagged_hash(CHIA_SP_INPUTS, lex_min_coin_id || aggregated_pk_compressed) mod r`. Panics on empty slice (action layer guarantees non-empty XCH-input set). TV1 byte-pinned at `38a1c8...cc9411`. Closed by Plan 04-01 (`tv1_compute_input_hash_matches` + `compute_input_hash_lex_min_independent_of_order`).
- **SEND-03** — `aggregate_sender_sks` free function in `silent_payments/aggregate.rs` (free-fn portion); multi-party hard-error path in `Spends::finish_with_silent_payment_keys` (Spends-level). Documents synthetic-SK requirement in rustdoc per RESEARCH §4 + PITFALLS §2. TV4 byte-pinned at `5600d878...cbf95b89`. Closed by Plan 04-01 (`tv4_aggregate_sender_sks_matches` + `single_sk_aggregate_is_identity`) + Plan 04-03 (`multi_party_hard_errors` for the hard-error path).
- **SEND-04** — `SilentPaymentSend` action + apply-time plumbing in `actions/silent_payment_send.rs` (Plan 04-02 closed apply-time); `Spends::finish_with_silent_payment_keys` finish-time entry point in `silent_payments/send_keys.rs` (Plan 04-03 closed finish-time). Closed by Plan 04-02 (`action_state_machine`) + Plan 04-03 (`round_trip_matches_derive_one_time_puzzle_hash`).
- **SEND-05** — Per-recipient k-counter in `Spends::silent_payment_counters: HashMap<[u8; 48], u32>` keyed by 48-byte compressed scan_pk. Counter increments in `SilentPaymentSend::spend` at apply time. Closed by Plan 04-04 (`multi_output_same_scan_pk_increments_k` + `multi_output_distinct_scan_pks_independent_counters`).
- **SEND-06** — Cross-input announcement binding in `Spends::emit_silent_payment_announcements` (pub(crate)) called at Step 8.5 of `finish_with_silent_payment_keys`. Opcode 60 on lex-min coin id, opcode 61 on every other input, both with empty announcement message `b""`. Single-input scenarios short-circuit. Closed by Plan 04-04 (`single_input_no_announcement` + `cross_index_announcement_binding` + `input_hash_round_trip`).
- **SEND-07** — Private `memo_hint_guard(ctx, memos)` in `actions/silent_payment_send.rs`; FIRST line of `SilentPaymentSend::spend`. Rejects 32-byte first memo with `DriverError::SilentPaymentMemoHintForbidden`. Permits Memos::None + 1-byte sentinel + 32-byte payload (the explicit wallet-author escape hatch) + all malformed/non-pair/non-atom CLVM shapes (defensive). Closed by Plan 04-05 (`memo_hint_guard_rejects_32_byte_first_memo` + `memo_hint_guard_allows_sentinel_prefixed` + `memo_hint_guard_allows_none`).
- **SEND-08** — Privacy-warning rustdoc audit gate. Every public memo-bearing API in the Phase-4 chip-0057-gated surface carries the literal substring `Privacy warning`. Grep gate `! grep -L 'Privacy warning' crates/chia-sdk-driver/src/actions/silent_payment_send.rs crates/chia-sdk-driver/src/silent_payments/{one_time,aggregate,input_hash,send_keys}.rs` returns empty. Closed by Plan 04-05 (audit verification; rustdoc itself added during construction across Plans 04-01..04-04).

## Key Decisions

Phase 4 made 8 cross-cutting calls beyond mechanical implementation:

1. **Synthetic-vs-raw key boundary (PITFALLS §2; Plan 04-01).** `aggregate_sender_sks` consumes SYNTHETIC SKs (the ones whose PKs are curried into `StandardArgs`). The function signature documents this explicitly. Plan resolved Open Q2 (`SyntheticSecretKey` newtype vs documented `&[SecretKey]`) toward Option B (documented `&[SecretKey]`) — the chia-bls `SecretKey` type is shared with all SDK signing flows and a parallel newtype would create a viral API change. Option A is still acceptable if future work wants compile-time safety.

2. **Multi-party hard-error vs silent aggregation (Plan 04-03).** When `Spends::finish_with_silent_payment_keys` is called with a `secret_keys` IndexMap that does NOT cover every XCH input's `parent.full_puzzle_hash`, finish returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)`. NO silent fallback — the v1 contract is "if you can't aggregate every input, you're in a multi-party scenario, which is unsupported". Future v2 may add a counterparty-coordination protocol, but v1 hard-errors. Documented in RESEARCH §3 + Anti-Pattern 1.

3. **K-counter location on Spends (Plan 04-02).** The k-counter is a `HashMap<[u8;48], u32>` field on `Spends` itself, keyed by 48-byte compressed `scan_pk`. Alternatives considered: per-action state (no — each `SilentPaymentSend` action would need to know about prior ones, breaking action-system encapsulation); per-recipient global (no — counters reset per Spends-batch). Decision: per-`Spends`-batch keyed by scan_pk gives the right scope: distinct sub-addresses of the same recipient (labeled vs unlabeled, m=1 vs m=2) all share the same scan_pk and increment the same counter.

4. **Lex-min coin_id announcer (vs Python reference's iteration-order; Plan 04-04).** The Python sp-common reference picks the announcer by iteration order. Plan 04-04 picks the lex-min coin id instead: order-independent across Spends builder coin-insertion orderings, matches `compute_input_hash`'s lex-min precedent, and produces identical detection outcomes on the receiver side. Documented in `emit_silent_payment_announcements` rustdoc with a cross-reference to RESEARCH §5c.

5. **Memo-hint guard Option A — explicit hard-error (Plan 04-05).** When a `SilentPaymentSend` is called with a 32-byte first memo, the guard returns `Err(DriverError::SilentPaymentMemoHintForbidden)` at apply time — BEFORE any side-effects on Spends. Alternatives considered: Option B silent rewrite (prepend a sentinel byte automatically — rejected because silent privacy-affecting transformations are dangerous); Option C lint-only warning (rejected because compile-time can't see the memo bytes). Option A makes the failure loud and immediate, matching the SDK's existing hard-error philosophy (`aggregate_sender_sks` hard-errors on multi-party; `labeled_address(0)` hard-errors on the reserved label). The wallet author's escape hatch is a 1-byte sentinel followed by the 32-byte payload — the first atom is then 1 byte and passes the guard.

6. **No aggregate_sender_sks in prelude (Plan 04-05; RESEARCH §Anti-Pattern 2).** The umbrella prelude's chip-0057 driver block re-exports 14 symbols including `SilentPaymentSend`, `compute_input_hash`, and `derive_one_time_puzzle_hash` — but DELIBERATELY OMITS `aggregate_sender_sks`. The function takes secret-key material and is only safe inside `Spends::finish_with_silent_payment_keys`'s invariants. Wallets who genuinely need it can reach `chia_sdk_driver::silent_payments::aggregate_sender_sks` directly — but the prelude does not advertise it, making accidental misuse harder.

7. **Method-on-Spends shape for `emit_silent_payment_announcements` (Plan 04-04).** Tests need to invoke the helper independently of `finish_with_silent_payment_keys` (which consumes `self`). A free function over `&mut FungibleSpends<Coin>` would force tests to either go through finish (and lose access to post-emission state) or duplicate the per-item walk. Method-on-Spends with `&mut self` gives the cleanest test introspection while preserving the production call-site shape.

8. **Unit-return on `emit_silent_payment_announcements` (Plan 04-04 deviation 1).** clippy::unnecessary_wraps fires on `Result<(), DriverError>` when no path returns `Err`. Resolution: change return type to `()`, drop `?` at call site. The plan's acceptance grep `self.emit_silent_payment_announcements(ctx, &xch_input_ids)\\?;` became `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` — functionally identical, no `#[allow]` added.

## Carried-forward Observations

- All three Phase 1 grep bans (`mod_by_group_order`, `^use sha2::`, `Sha256::digest`) hold across Phase 4 code. Verified at each plan's gate.
- Phase 3's `pub mod silent_payments;` visibility upgrade in `chia-sdk-driver/src/lib.rs` (from Plan 03-05) is the prerequisite for Phase 4's prelude additions — confirmed working.
- The function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` (Plan 03-03 deviation) remains the only `#[allow]` anywhere under `silent_payments/` after Phase 4 close. Zero new `#[allow]` attributes added across all 5 Phase 4 plans.
- No new workspace dependencies added in Phase 4 (cumulative since Phase 1: still 0 new workspace deps for the entire silent-payments build-out). The 4 new silent_payments files + 1 new action file + 6 modified files compose existing primitives without pulling new crates.

## Files Inventory

### Created Rust source (5 files)

- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` (Plan 04-01)
- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` (Plan 04-01)
- `crates/chia-sdk-driver/src/silent_payments/one_time.rs` (Plan 04-01)
- `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` (Plans 04-02 stub → 04-03 real body → 04-04 announcement helper)
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (Plans 04-02 struct+spend → 04-03 round-trip test → 04-04 k-counter+announcement tests → 04-05 memo-hint guard+3 tests)

### Modified

- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — 4 new module re-exports (Plan 04-01) + cross-module re-export of SilentPaymentSend (Plan 04-05)
- `crates/chia-sdk-driver/src/actions.rs` — added `silent_payment_send` module declaration + flat re-export (Plan 04-02)
- `crates/chia-sdk-driver/src/action_system/action.rs` — `Action::SilentPaymentSend(SilentPaymentSend)` variant + `Action::silent_payment_send` constructor + 2 match-arm dispatches (Plan 04-02)
- `crates/chia-sdk-driver/src/action_system/spends.rs` — two new `pub(crate)` fields + initializers (Plan 04-02)
- `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` — `conditions_ref` accessor (Plan 04-04)
- `crates/chia-sdk-driver/src/driver_error.rs` — three new chip-0057-gated variants (Plan 04-03)
- `src/prelude.rs` — 3 new chip-0057 symbols in the driver block (Plan 04-05)

### Phase planning artifacts

- `.planning/phases/04-send-side-action/04-PLAN.md` — phase-level plan
- `.planning/phases/04-send-side-action/04-RESEARCH.md` — phase research (~1500 lines)
- `.planning/phases/04-send-side-action/04-VALIDATION.md` — validation rubric
- `.planning/phases/04-send-side-action/04-{01,02,03,04,05}-PLAN.md` — per-wave plans
- `.planning/phases/04-send-side-action/04-{01,02,03,04,05}-SUMMARY.md` — per-wave summaries
- `.planning/phases/04-send-side-action/04-PHASE-SUMMARY.md` — this file

## Plans Index

| Plan | Title | Duration | Tests added | Key artifact |
|------|-------|----------|-------------|--------------|
| 04-01 | Free functions: derive_one_time_puzzle_hash + compute_input_hash + aggregate_sender_sks | 15 min | 6 | 3 new silent_payments files |
| 04-02 | SilentPaymentSend action + Action variant + apply-time plumbing | 13 min | 1 | actions/silent_payment_send.rs + send_keys.rs stub |
| 04-03 | Real finish_with_silent_payment_keys + 3 DriverError variants + multi-party hard-error | 21 min | 2 | send_keys.rs real body + 3 DriverError variants |
| 04-04 | k-counter coordination + opcode 60/61 announcement binding | 21 min | 5 | emit_silent_payment_announcements + conditions_ref |
| 04-05 | Memo-hint guard + Privacy-warning audit + prelude re-exports + Phase 4 final gate | 23 min | 3 | memo_hint_guard + 14-symbol prelude block |
| **Total** | | **93 min** | **17** | **5 files created, 7 modified** |

## Phase Transition: Phase 5 Readiness

Phase 5 (Bindings — Rust facade + JSON descriptor) is UNBLOCKED.

**Inherits from Phase 4:**

- Full Rust public surface for bindings exposure:
  - `SilentPaymentSend` (struct + `new()` constructor) — wraps via `Action::silent_payment_send(recipient, amount, memos)` constructor for the action_system binding entry.
  - `derive_one_time_puzzle_hash`, `compute_input_hash` — exposed in `bindings/silent_payments.json` as static functions.
  - `aggregate_sender_sks` — reachable via `chia_sdk_driver::silent_payments::aggregate_sender_sks` (NOT through prelude); also exposed in `bindings/silent_payments.json`.
  - `SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`, `SilentPaymentMemoHintForbidden` DriverError variants — surfaced through the existing DriverError binding.

- Bindings-friendly type granularity (designed in Phase 4 with binding constraint in mind per RESEARCH §1f): `Bytes32` (coin_id), `Vec<PublicKey>` (synthetic_pks list), `Vec<SecretKey>` (synthetic_sks list), `u32` (k-counter), `IndexMap<Bytes32, PublicKey/SecretKey>` (parent_puzzle_hash → key lookup).

**Outstanding pre-Phase-5 questions (from STATE.md):**

- **Q3 (Phase 5 pre-flight):** Verify `bindy-macro` `"type": "static_functions"` schema support before committing the JSON descriptor; fallback strategy documented in research/ARCHITECTURE.md.

**Phase 6 readiness (Simulator E2E + example):** Phase 6 consumes the Phase 5 bindings + writes `examples/silent_payment.rs` against `chia_wallet_sdk::prelude::*`. The simulator round-trip will:
1. Sender side: derive SilentPaymentKeys from a mnemonic, encode the address as `spxch1...`, build a `Spends` with one XCH input + one `Action::silent_payment_send(recipient, amount, Memos::None)`, finish via `finish_with_silent_payment_keys`, and submit to the in-process Simulator.
2. Receiver side: construct `TweakData` from the simulator's coin-state callback, call `scan_from_tweaks` (or `SilentPaymentKeys::scan`), assert one `DetectedSpCoin` is returned with the expected `puzzle_hash` + `coin_id` + `onetime_sk`.
3. Spend-as-sender: reconstruct the `onetime_sk`, sign a standard transaction spending the detected coin to a new address, submit to the simulator.

This closes the on-chain landing + scanner detection + spend-as-sender chain at the integration level — the final missing piece beyond Phase 4's SDK-level round-trip.

---
*Phase: 04-send-side-action*
*Plans: 5 of 5 complete*
*Completed: 2026-05-16 (1M-context exec session)*
