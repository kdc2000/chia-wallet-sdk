---
phase: 04-send-side-action
plan: 04
subsystem: driver-action-system
tags: [silent-payments, chip-0057, action-system, spends, send-side, announcement-binding, opcode-60, opcode-61, k-counter]

# Dependency graph
requires:
  - phase: 04-send-side-action
    plan: 03
    provides: Spends::finish_with_silent_payment_keys real 9-step body — Plan 04-04 inserts emit_silent_payment_announcements between Step 8 and Step 9
  - phase: 04-send-side-action
    plan: 02
    provides: Spends::silent_payment_counters + silent_payments_pending fields — Plan 04-04's tests inspect them directly
  - phase: 04-send-side-action
    plan: 01
    provides: aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash — used in input_hash_round_trip test to independently re-derive the expected one-time puzzle hash
provides:
  - "Spends::emit_silent_payment_announcements — pub(crate) method-on-Spends; emits opcode 60 on lex-min XCH coin_id input + opcode 61 on every other non-ephemeral XCH input with message = b\"\" (single-input scenarios are a no-op)"
  - "Step 8.5 wiring inside Spends::finish_with_silent_payment_keys — self.emit_silent_payment_announcements(ctx, &xch_input_ids) inserted between Step 8 (per-pending CreateCoin emission loop) and Step 9 (finish_with_keys delegation)"
  - "ConditionsSpend::conditions_ref(&self) -> &Conditions — new public accessor for introspection; tests use it to assert opcode 60/61 emission shapes before finish consumes the spend"
  - "actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k — SEND-05 k=0,1 counter increment test for same scan_pk"
  - "actions::silent_payment_send::tests::multi_output_distinct_scan_pks_independent_counters — SEND-05 per-scan_pk independence test (both k=0)"
  - "actions::silent_payment_send::tests::single_input_no_announcement — SEND-06 single-input short-circuit test (no opcode 60/61)"
  - "actions::silent_payment_send::tests::cross_index_announcement_binding — SEND-06 multi-input announcer/asserter assignment test (lex-min coin emits opcode 60; other coin emits opcode 61 asserting SHA256(lex_min_coin_id || \"\"))"
  - "actions::silent_payment_send::tests::input_hash_round_trip — SEND-06 input_hash reconstruction test (receiver's Pass 2b path closes byte-for-byte with the sender's compute_input_hash output)"
affects:
  - "04-05 — Plan 04-04 was the last plan to touch finish_with_silent_payment_keys's production code path. Plan 04-05 only adds the apply-time memo-hint guard + Privacy-warning audit grep + prelude re-exports + phase gate; no further send_keys.rs edits."
  - "ROADMAP Phase 4 success criterion #3 (k-counter increments per recipient) — CLOSED"
  - "ROADMAP Phase 4 success criterion #4 (cross-input announcement binding) — CLOSED"

# Tech tracking
tech-stack:
  added: []  # no new workspace deps
  patterns:
    - "Method-on-Spends shape (vs free function over &mut FungibleSpends<Coin>): tests need to invoke emit_silent_payment_announcements independently of finish_with_silent_payment_keys, which consumes self. Per RESEARCH §5d the method-on-Spends design was chosen from the start (iteration-2 plan fix); a free-function shape would force tests to either go through finish (and lose access to the spends builder) or duplicate the per-item walk."
    - "Unit-return shape over Result<(), DriverError>: clippy::unnecessary_wraps fires when no path returns Err. The plan acknowledges signature-level adaptation as in-scope (`Inline fix any clippy fires`); changing the return type to () (and dropping the `?` at the call site) satisfies clippy without an #[allow]. Documented in the helper's rustdoc."
    - "ConditionsSpend::conditions_ref read-only accessor: tests need to introspect emitted conditions before finish_with_silent_payment_keys consumes the Spends. The existing `finish(self) -> Conditions` method consumes; only `add_conditions(&mut self)` writes. Adding `conditions_ref(&self) -> &Conditions` is the smallest API addition that unblocks the introspection tests. Documented in rustdoc that the accessor is for tests + instrumentation."
    - "Vec intermediate workaround for clippy::cloned_ref_to_slice_refs (Plan 04-03 deviation 5 precedent generalized to multi-SK case): `let sender_sks = vec![alice.sk.clone(), bob.sk.clone()]; aggregate_sender_sks(&sender_sks)` instead of `aggregate_sender_sks(&[alice.sk.clone(), bob.sk.clone()])`. The Vec preserves the per-SK `.clone()` byte sequence in the source for grep while satisfying clippy."
    - "Lex-min coin_id announcer selection (vs Python reference's iteration-order): order-independent across Spends builder coin-insertion orderings; matches compute_input_hash's lex-min selection (same precedent). Receiver's Pass 2b doesn't care which input is the announcer; both choices produce identical detection."

key-files:
  modified:
    - crates/chia-sdk-driver/src/silent_payments/send_keys.rs
    - crates/chia-sdk-driver/src/actions/silent_payment_send.rs
    - crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs

key-decisions:
  - "Unit-return signature on emit_silent_payment_announcements: clippy::unnecessary_wraps fires on Result<(), DriverError> because no path returns Err. Two resolutions considered: (1) keep Result and add `#[allow(clippy::unnecessary_wraps)]` — forbidden by Plan 04-03's no-new-#[allow] rule; (2) genuinely return Err from some path — no current failure mode exists, would require fabricating one. Resolution: change return type to (), drop the `?` at the call site. The plan's acceptance grep `self.emit_silent_payment_announcements(ctx, &xch_input_ids)\\?;` becomes `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` — functionally identical. Doc-comment documents the rationale."
  - "ConditionsSpend::conditions_ref accessor added: tests for single_input_no_announcement and cross_index_announcement_binding need to walk emitted conditions on `spends.xch.items[i].kind` before finish_with_silent_payment_keys (which consumes self) runs. Existing API had only `finish(self) -> Conditions` (consuming) and `add_conditions(&mut self)` (write). Adding `conditions_ref(&self) -> &Conditions` is the minimal API addition that unblocks introspection. The accessor is `pub` (not `pub(crate)`) — same visibility as `finish` and `add_conditions`. The rustdoc explicitly calls out test/instrumentation as the primary use case."
  - "Recipient cloning in multi_output_same_scan_pk_increments_k: `Action::silent_payment_send` moves `SilentPaymentAddress` by value (it's not Copy). The k=0/k=1 test needs the same recipient in two consecutive actions plus a final assertion via the scan_pk. Resolution: capture `let scan_pk = recipient.scan_pk;` before the first action, then pass `recipient.clone()` to the first action and the moved `recipient` to the second. Functionally identical; documents the move pattern in a comment."
  - "Lex-min coin_id selection (per RESEARCH §5c) instead of Python reference's iteration-order announcer choice: lex-min is order-independent across Spends builder coin-insertion orderings, matches compute_input_hash's lex-min precedent, and produces identical detection outcomes on the receiver side. The doc-comment on the helper cross-references RESEARCH §5c for the full tradeoff analysis."

patterns-established:
  - "Method-on-Spends shape for opcode 60/61 emission helpers: the test surface needs the helper to be callable independently of finish (which consumes self). Free-function-over-FungibleSpends would force tests to go through finish or duplicate logic; method-on-Spends with &mut self gives the cleanest test introspection while preserving the production call-site shape."
  - "ConditionsSpend introspection pattern: when a test needs to inspect emitted conditions before finish consumes the spend, walk `spends.xch.items[i].kind` matching `SpendKind::Conditions(spend)`, then call `spend.conditions_ref()` for a `&Conditions<NodePtr>` borrow, iterate over `Condition::CreateCoinAnnouncement(_)`/`Condition::AssertCoinAnnouncement(_)` variants for opcode-specific assertions."
  - "Bytes::new(vec![]) for empty announcement message: chia_protocol::Bytes is the message type for CreateCoinAnnouncement. `Bytes::new(vec![])` (zero-allocation when used in announcement_id's &impl AsRef<[u8]>) is the canonical empty-message construction; `b\"\".to_vec().into()` is the equivalent."

# Metrics
duration: 21min
completed: 2026-05-16

requirements-completed: [SEND-05, SEND-06]
# SEND-05: k=0 + k=1 same scan_pk + per-scan_pk independence — both behaviors verified at the integration level.
# SEND-06: single-input short-circuit + multi-input opcode 60/61 emission (lex-min announcer + SHA256(coin_id || "") asserted id) + input_hash round-trip — all three behaviors verified.
---

# Phase 04 Plan 04: opcode 60/61 announcement binding + k-counter coordination tests Summary

**Wave 4 of Phase 4: a new `pub(crate)` method `Spends::emit_silent_payment_announcements` is added to `silent_payments/send_keys.rs` and wired into `Spends::finish_with_silent_payment_keys` at Step 8.5 (between the per-pending `CreateCoin` emission loop and the `finish_with_keys` delegation); five new integration tests in `actions::silent_payment_send::tests` close SEND-05 (k-counter coordination) and SEND-06 (cross-input announcement binding + receiver-side input_hash round-trip); a small `ConditionsSpend::conditions_ref` accessor lands to support the test introspection. ROADMAP Phase 4 success criteria #3 and #4 close.**

## Performance

- **Duration:** ~21 min
- **Started:** 2026-05-16T02:33:43Z
- **Completed:** 2026-05-16T02:55:02Z
- **Tasks:** 2 (all `type="auto"`, `tdd="true"`) — executed in declared order 1 -> 2
- **Files modified:** 3 (`silent_payments/send_keys.rs`, `actions/silent_payment_send.rs`, `action_system/spend_kind/conditions_spend.rs`)
- **Files created:** 0
- **Tests added:** +5 (`multi_output_same_scan_pk_increments_k`, `multi_output_distinct_scan_pks_independent_counters`, `single_input_no_announcement`, `cross_index_announcement_binding`, `input_hash_round_trip`) — driver lib test count 1072 -> 1077
- **`actions::silent_payment_send` test set:** 2 -> 7 (+5)
- **`silent_payments` test set:** 19 -> 19 (unchanged; this plan's tests all land under `actions::silent_payment_send`)

## Accomplishments

- **`Spends::emit_silent_payment_announcements` method** (`silent_payments/send_keys.rs`):
  - `pub(crate)` visibility — external callers never invoke it directly; they go through `finish_with_silent_payment_keys` which calls it at Step 8.5.
  - Method-on-`Spends` shape (vs. a free function over `&mut FungibleSpends<Coin>`): tests must invoke it independently of `finish_with_silent_payment_keys` (which consumes `self`).
  - Algorithm: if `xch_input_ids.len() < 2`, short-circuit (single-input scenarios don't need announcement binding). Otherwise pick `lex_min_coin_id = xch_input_ids.iter().min()`, compute `ann_id = announcement_id(lex_min_coin_id, b"")`, then walk `self.xch.items` and for each non-ephemeral item with `SpendKind::Conditions(spend)`, emit `CREATE_COIN_ANNOUNCEMENT` (opcode 60) on the lex-min coin or `ASSERT_COIN_ANNOUNCEMENT` (opcode 61) on every other coin.
  - Returns `()` (not `Result<(), DriverError>`): no reachable failure mode; doc-comment documents the rationale.

- **Step 8.5 wiring** in `finish_with_silent_payment_keys`: `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` inserted between Step 8's per-pending `CreateCoin` emission loop and Step 9's `self.finish_with_keys(...)` delegation. The Step 9 comment that previously documented "Plan 04-04 will insert..." has been updated.

- **`ConditionsSpend::conditions_ref(&self) -> &Conditions` accessor** (`action_system/spend_kind/conditions_spend.rs`): minimal read-only accessor used by the new tests to introspect emitted conditions before finish consumes the spend. Public (matches the visibility of `add_conditions` and `finish`); doc-comment explicitly calls out the test/instrumentation use case.

- **Five new integration tests** (`actions/silent_payment_send.rs`):
  - `multi_output_same_scan_pk_increments_k` — SEND-05: two `Action::silent_payment_send(recipient, ...)` actions to the SAME recipient in one `Spends::apply` call produce outputs at k=0 and k=1; the counter on `spends.silent_payment_counters` keyed by 48-byte `scan_pk` advances past 1.
  - `multi_output_distinct_scan_pks_independent_counters` — SEND-05: two actions to DIFFERENT recipients both produce k=0 (per-`scan_pk` independence).
  - `single_input_no_announcement` — SEND-06: a `Spends` with one XCH input + one silent_payment_send, after manually invoking `emit_silent_payment_announcements`, emits neither opcode-60 nor opcode-61 (helper short-circuits).
  - `cross_index_announcement_binding` — SEND-06: a `Spends` with two XCH inputs (alice + bob at different derivation indices), after manually invoking `emit_silent_payment_announcements`, emits exactly one `CreateCoinAnnouncement` (opcode 60) on the lex-min coin with empty message AND exactly one `AssertCoinAnnouncement` (opcode 61) on the other coin with `announcement_id = SHA256(lex_min_coin_id || "")`.
  - `input_hash_round_trip` — SEND-06: build a `Spends` with two XCH inputs + one silent_payment_send, run full `finish_with_silent_payment_keys`, then independently re-derive the expected one-time puzzle hash via `aggregate_sender_sks(&[alice.sk, bob.sk]) -> SecretKey::from_bytes round-trip -> compute_input_hash(&[alice_coin_id, bob_coin_id], &agg_pk) -> derive_one_time_puzzle_hash(..., k=0)` and assert byte-equality with the emitted output's puzzle_hash. Closes the receiver-side reconstruction round-trip across the announcement linkage.

## Task Commits

Each task committed atomically:

1. **Task 1: `emit_silent_payment_announcements` method on `Spends` + Step 8.5 wiring** — `6b32fb90` (`feat`)
2. **Task 2: 5 integration tests + `ConditionsSpend::conditions_ref` accessor** — `402d16fb` (`test`)

The two commits cleanly stack atop Plan 04-03's HEAD. Each task verified independently:

- Task 1: `cargo build --release -p chia-sdk-driver -F chip-0057` clean; pre-existing tests (`multi_party_hard_errors`, `action_state_machine`, `round_trip_matches_derive_one_time_puzzle_hash`) still pass; clippy `-D warnings` clean.
- Task 2: all 7 silent_payment_send tests pass (2 existing + 5 new); clippy `-D warnings` clean; `cargo build --release --workspace --all-features` clean (3m 14s); `cargo machete` clean.

## Files Modified

- `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` — **+82 lines**. New `pub(crate)` method `emit_silent_payment_announcements` on `Spends` (with full doc-comment cross-referencing RESEARCH §5d); one-line call inside `finish_with_silent_payment_keys` at Step 8.5; comment update on Step 9.
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — **+253 lines**. Five new `#[test]` functions appended to the `mod tests` block.
- `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` — **+8 lines**. New `pub fn conditions_ref(&self) -> &Conditions` read-only accessor.

## Decisions Made

- **Unit-return signature (`fn ... ()`) on `emit_silent_payment_announcements`:** the helper has no reachable failure mode — `xch_input_ids.iter().min()` is infallible after the `len() < 2` guard, and `add_conditions` is a pure append. clippy::unnecessary_wraps fires when no path returns `Err`. The plan's acceptance criteria included `self.emit_silent_payment_announcements(ctx, &xch_input_ids)?;` (with `?`); resolving the clippy fire without a function-level `#[allow]` requires changing the return type to `()` and dropping the `?` at the call site. The plan explicitly anticipates this kind of clippy-driven signature adjustment (`Inline fix any clippy fires`). Doc-comment documents the rationale.

- **`ConditionsSpend::conditions_ref` accessor added:** tests for `single_input_no_announcement` and `cross_index_announcement_binding` need to walk emitted conditions before finish consumes the spend. Existing API had only `finish(self) -> Conditions` (consuming) and `add_conditions(&mut self)` (write). Adding `conditions_ref(&self) -> &Conditions` is the minimal addition that unblocks introspection. Public visibility matches `add_conditions` and `finish`; the rustdoc explicitly documents the test/instrumentation use case. `#[must_use]` was tried initially but removed — `Conditions` is already `#[must_use]`, so a `#[must_use]` on the accessor is redundant per clippy.

- **Recipient cloning in `multi_output_same_scan_pk_increments_k`:** `Action::silent_payment_send` moves `SilentPaymentAddress` by value (it's not `Copy`). To pass the same recipient to two consecutive actions plus a final `scan_pk_bytes` assertion, capture `let scan_pk = recipient.scan_pk;` before the first action, then pass `recipient.clone()` to the first action and the moved `recipient` to the second. Functionally identical; documented in a comment in the test.

- **Lex-min coin_id announcer (vs Python reference's iteration-order):** order-independent across Spends builder coin-insertion orderings, matches `compute_input_hash`'s lex-min precedent, and produces identical detection outcomes on the receiver side. Documented in detail in the helper's rustdoc with a cross-reference to RESEARCH §5c.

- **Method-on-`Spends` shape (vs free function over `&mut FungibleSpends<Coin>`):** the iteration-2 plan fix locked this shape because tests need to invoke the helper independently of `finish_with_silent_payment_keys` (which consumes `self`). A free function would force tests to either go through finish (and lose access to the post-emission state of `spends.xch.items`) or duplicate the per-item walk. The method-on-`Spends` design is also cleaner at the call site (`self.emit_silent_payment_announcements(...)` reads naturally inside `finish_with_silent_payment_keys`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - clippy::unnecessary_wraps on the helper's `Result<(), DriverError>` return type]**

- **Found during:** Task 1 (initial clippy run after Edit 1+2+3 landed)
- **Issue:** The plan specified `Result<(), DriverError>` for forward-compat with possible future failure modes. clippy::unnecessary_wraps fired because no path in the body returns `Err`. The lint can be suppressed only with `#[allow]`, which violates the no-new-#[allow] rule.
- **Fix:** Changed the helper signature to return `()` (unit), dropped the `?` at the call site in `finish_with_silent_payment_keys`. Added a doc-comment paragraph documenting the rationale (`clippy::unnecessary_wraps` + no future failure mode + the call site invokes the helper as a statement).
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/send_keys.rs`
- **Verification:** `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean.
- **Committed in:** `6b32fb90` (Task 1)
- **Rationale:** Plan 04-03's no-new-#[allow] rule still holds in Plan 04-04. The plan's acceptance grep `self.emit_silent_payment_announcements(ctx, &xch_input_ids)\\?;` becomes `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` — functionally identical (the helper would return `Ok(())` on every path even if it kept the `Result`). The plan explicitly anticipated this kind of clippy-driven signature adjustment in the `<important>` notes (`Inline fix any clippy fires`).

**2. [Rule 1 - clippy::doc_markdown on test rustdoc comments]**

- **Found during:** Task 2 (clippy `-D warnings` after the 5 tests landed)
- **Issue:** Several test doc-comments referenced symbol names without backticks: `SilentPaymentSend`, `scan_pk`, `Spends`, `CreateCoinAnnouncement`, `AssertCoinAnnouncement`, `coin_ids`, `compute_input_hash`, `input_hash`. clippy::doc_markdown fires on each.
- **Fix:** Wrapped each in backticks. Also wrapped `SHA256(coin_id_min || "")` and converted the bullet-list lines to proper continuation indentation where rustdoc complained about list-item indentation.
- **Files modified:** `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Verification:** clippy clean.
- **Committed in:** `402d16fb` (Task 2)
- **Rationale:** Plan 04-03 deviation 4 set the precedent — backticking is the standard fix; no `#[allow]` needed.

**3. [Rule 1 - clippy::needless_pass_by_value-style on `must_use_candidate` for `conditions_ref`]**

- **Found during:** Task 2 (clippy `-D warnings`)
- **Issue:** Initially added `#[must_use]` to the new `conditions_ref` accessor. clippy fired `redundant_pub_crate`-style "this function has a #[must_use] attribute with no message, but returns a type already marked as #[must_use]" — because `Conditions` itself is `#[must_use]`.
- **Fix:** Removed the `#[must_use]` from `conditions_ref`. The accessor returns `&Conditions`, and `Conditions` carries the `#[must_use]` already.
- **Files modified:** `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs`
- **Verification:** clippy clean.
- **Committed in:** `402d16fb` (Task 2)
- **Rationale:** Following clippy's guidance — redundant attribute removed; the parent type's `#[must_use]` still propagates.

**4. [Rule 1 - `SilentPaymentAddress` is not `Copy`; `Action::silent_payment_send` consumes it]**

- **Found during:** Task 2 (test compilation, `multi_output_same_scan_pk_increments_k`)
- **Issue:** The plan's draft test body called `Action::silent_payment_send(recipient, 1, ...)` twice with the same `recipient` variable. Compilation fails: `recipient` is moved on the first call and cannot be reused. The plan acceptance criterion `grep -E '^    fn multi_output_same_scan_pk_increments_k'` still passes; the body construction is the issue.
- **Fix:** Capture `let scan_pk = recipient.scan_pk;` before the first action, pass `recipient.clone()` to the first action and the moved `recipient` to the second. Documented in a comment.
- **Files modified:** `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Verification:** all 7 tests pass.
- **Committed in:** `402d16fb` (Task 2)
- **Rationale:** `SilentPaymentAddress` derives `Clone` (verified by reading `chia-sdk-utils/src/silent_payments/address.rs`); the `.clone()` call is cheap (two `PublicKey`s + a `Network` enum). Not adding `Copy` to `SilentPaymentAddress` because the type contains `PublicKey`s which are not `Copy`.

**5. [Rule 1 - Unused `Asset` import in test modules]**

- **Found during:** Task 2 (first build run)
- **Issue:** The plan's draft test bodies imported `crate::Asset` for `item.asset.coin_id()` access. `Coin` has an inherent `coin_id` method (from `chia_protocol::Coin`), so the `Asset` trait import is redundant; the workspace's strict `unused_imports = "deny"` fires.
- **Fix:** Replaced `use crate::{Asset, SpendKind};` with `use crate::SpendKind;` in both `single_input_no_announcement` and `cross_index_announcement_binding`.
- **Files modified:** `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Verification:** clippy clean; all tests pass.
- **Committed in:** `402d16fb` (Task 2)
- **Rationale:** `Coin::coin_id()` is provided inherently by `chia_protocol::Coin`; the `Asset` trait is only needed when calling `coin_id` through a generic asset type.

**6. [Rule 1 - rustfmt reflow on the long-line `Conditions::new().create_coin_announcement(...)` chain and on inline `assert_eq!` macros]**

- **Found during:** Task 2 (post-test `cargo fmt --all --check`)
- **Issue:** rustfmt reflowed the long-line `spend.add_conditions(Conditions::new().create_coin_announcement(Bytes::new(vec![])));` in send_keys.rs across two lines, and inlined a multi-line `assert_eq!` block in cross_index_announcement_binding.
- **Fix:** `cargo fmt --all` applied automatically.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/send_keys.rs`, `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Verification:** `cargo fmt --all --check` clean.
- **Committed in:** `6b32fb90` (Task 1, send_keys.rs reflow) + `402d16fb` (Task 2, silent_payment_send.rs reflow)

---

**Total deviations:** 6 auto-fixed (all Rule 1 — workspace strict-mode lint tightening or rustfmt reflow). Zero scope creep; zero new `#[allow]` attributes; zero functional changes (the return-type tweak preserves byte-equivalent behavior — no path could have returned `Err` even with the `Result<(), DriverError>` shape). Zero deviation against the plan's algorithmic semantics or grep-checkable acceptance criteria except:

**Impact on plan acceptance grep criteria:**
- `grep -E 'self\.emit_silent_payment_announcements\(ctx, &xch_input_ids\)\?;'` does NOT pass exactly — the `?;` was dropped per Deviation 1. The functional equivalent `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` appears at the documented insertion point.
- All other 16 plan acceptance greps pass exactly.

## Issues Encountered

The `clippy::unnecessary_wraps` deviation was the only meaningful tension between the plan's draft signature and the workspace's strict-mode lint policy. The plan anticipated signature-level adaptation (`Inline fix any clippy fires`); the resolution (drop the `Result` + drop the `?`) is mechanically clean and documented in the helper's rustdoc.

The `SilentPaymentAddress` non-`Copy` issue surfaced as a test compilation error rather than a plan-level concern; the `.clone()` fix is the standard workaround for non-`Copy` types and adds negligible runtime cost (two `PublicKey` clones per second action — `PublicKey::clone` is cheap).

`Coin::coin_id()` having both an inherent method (from `chia_protocol::Coin`) and a trait method (from `crate::Asset`) caused the unused-import surface; the inherent method wins in lookup, so the trait import is unnecessary unless you're calling through a generic `T: Asset` bound. The plan's draft assumed the trait was needed; resolution was to drop the unused import.

No simulator-level round-trip was attempted in Plan 04-04 (Phase 6 owns that). The `input_hash_round_trip` test closes the receiver-side reconstruction at the puzzle-hash byte level; it does NOT verify on-chain landing or scanner detection via the Phase 3 `SilentPaymentKeys::scan` method. Phase 6's simulator round-trip will close those gaps.

## Self-Check: PASSED

**Files modified:**
- `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` — FOUND (Spends::emit_silent_payment_announcements method present; Step 8.5 call present in finish_with_silent_payment_keys)
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — FOUND (5 new test functions present)
- `crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` — FOUND (conditions_ref accessor present)

**Commits exist:**
- `6b32fb90` — FOUND (Task 1)
- `402d16fb` — FOUND (Task 2)

**Tests green:**
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_same_scan_pk_increments_k -- --exact` -> 1 passed
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::multi_output_distinct_scan_pks_independent_counters -- --exact` -> 1 passed
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::single_input_no_announcement -- --exact` -> 1 passed
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::cross_index_announcement_binding -- --exact` -> 1 passed
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send::tests::input_hash_round_trip -- --exact` -> 1 passed
- `cargo test --release -p chia-sdk-driver --features chip-0057 actions::silent_payment_send` -> 7 passed (2 existing + 5 new)
- `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment` -> 26 passed (Phase 3's 12 + Plan 04-01's 6 + Plan 04-03's 1 send_keys test + 2 + 5 new action tests)
- `cargo test --release --workspace --all-features --exclude binding-crates` -> full suite green, no regressions

**Gate sweep clean:**
- `cargo build --release -p chia-sdk-driver` (no features) clean
- `cargo build --release -p chia-sdk-driver --features chip-0057` clean
- `cargo build --release --workspace --all-features` clean (~3m 14s)
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean
- `cargo fmt --all --check` clean
- `cargo machete` clean

**Phase 1 grep bans hold:**
- `! grep -E 'mod_by_group_order|^use sha2::|Sha256::digest' crates/chia-sdk-driver/src/silent_payments/send_keys.rs crates/chia-sdk-driver/src/actions/silent_payment_send.rs crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` -> empty.

**No `#[allow]` attributes added:**
- `! grep -E '^[[:space:]]*#\\[allow' crates/chia-sdk-driver/src/silent_payments/send_keys.rs crates/chia-sdk-driver/src/actions/silent_payment_send.rs crates/chia-sdk-driver/src/action_system/spend_kind/conditions_spend.rs` -> empty. The only `#[allow]` anywhere under `silent_payments/` remains the function-scoped one on `scan_from_tweaks` from Plan 03-03.

**Plan acceptance greps (Task 1):**
- `pub(crate) fn emit_silent_payment_announcements(\n        &mut self,` — present.
- `^fn emit_silent_payment_announcements\(` (free-function shape) — NOT present (only the method-on-Spends form exists, per iteration-2 plan fix).
- `xch_input_ids: &[Bytes32]` — present.
- `if xch_input_ids.len() < 2 {` — present.
- `xch_input_ids.iter().min()` — present (in the lex-min selection line).
- `self.xch.items` access pattern — present.
- `create_coin_announcement(Bytes::new(vec![]))` — present.
- `assert_coin_announcement(ann_id)` — present.
- `announcement_id(lex_min_coin_id, &empty_message)` — present.
- `self.emit_silent_payment_announcements(ctx, &xch_input_ids);` — present (without `?` per Deviation 1).

**Plan acceptance greps (Task 2):**
- All 5 test function names present.
- All 5 test invocations pass `-- --exact` on their own.
- `7 passed` on the full `actions::silent_payment_send` test set.
- `cargo build --release --workspace --all-features` clean.
- `cargo clippy ... -D warnings` clean.
- No `#[allow]` added.

## Next Plan Readiness

**Plan 04-05 (Memo-hint guard + Privacy-warning audit + prelude re-exports + phase gate):** UNBLOCKED.

- **Memo-hint guard:** `DriverError::SilentPaymentMemoHintForbidden` is already in place from Plan 04-03 Task 1. Plan 04-05 only needs to add the apply-time guard call site at the action layer (`SilentPaymentSend::spend` or `SilentPaymentSend::new`). The guard is a single check: if `memos` is `Memos::Some(node)` AND the first memo (extracted via the `Memos` projection) is exactly 32 bytes, return `Err(DriverError::SilentPaymentMemoHintForbidden)` before the action records the pending entry.
- **Privacy-warning audit grep:** Plan 04-04 introduced no new public memo-bearing API; the public surface remains as it was after Plan 04-03 (`SilentPaymentSend` struct + `Action::silent_payment_send` constructor + `Spends::finish_with_silent_payment_keys`). The audit grep should find all of Plan 04-02/04-03's "Privacy warning" mentions plus any new ones Plan 04-05 adds.
- **Prelude re-exports:** `SilentPaymentSend`, `Action::silent_payment_send`, and possibly the three free functions from Plan 04-01 (`aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash`) — Plan 04-05's must-haves list will specify the exact surface.
- **Final phase gate:** at the end of Plan 04-05, all SEND-01..SEND-08 requirements should close. Phase 4 then awaits Phase 6's simulator round-trip and bindings E2E to close the on-chain landing + scan-detection chain at the integration level.

**Phase 4 functional completeness:** with Plan 04-04 landed, the send-side action system can:
1. Aggregate sender SKs across multiple XCH inputs ([04-01]).
2. Compute the canonical `input_hash` binding to lex-min `coin_id` + aggregated synthetic PK ([04-01]).
3. Derive one-time puzzle hashes per (scan_pk, spend_pk, aggregated_sender_sk, input_hash, k) ([04-01]).
4. Record per-output deterministic state at apply time + increment per-`scan_pk` k-counters ([04-02 + this plan's tests close SEND-05]).
5. Emit `CreateCoin` conditions at finish time with the derived puzzle hashes ([04-03]).
6. Emit cross-input announcement binding (opcode 60/61) for >=2-input scenarios ([this plan closes SEND-06]).
7. Hard-error on multi-party scenarios (missing SK in the synthetic_sks map) ([04-03]).
8. Hard-error on no-XCH-inputs scenarios (pathological all-ephemeral input case) ([04-03]).

Plan 04-05 adds the memo-hint guard (a 9th capability, defensive) + the prelude re-exports (the umbrella crate surface) + the final phase gate.

---
*Phase: 04-send-side-action*
*Completed: 2026-05-16 (1M-context exec session)*
