---
phase: 07-code-review-cleanup
plan: 02
subsystem: chia-sdk-driver/actions
tags: [refactor, cleanup, chip-0057, action-system, code-organization]
requirements: [CLEANUP-02]
dependency-graph:
  requires:
    - "v1 silent-payments work (Phases 1-6) — feature-complete; refactor only"
    - "Phase 07-01 (comment cleanup) — provides clean baseline; non-blocking dep"
  provides:
    - "actions/silent_payment_send.rs — flat-sibling module hosting the chip-0057 SP arm of Action::send (helpers + pub(crate) entry point) AND the relocated mod silent_payment_tests test block"
  affects:
    - "crates/chia-sdk-driver/src/actions/send.rs — shrunk 1080 → 399 lines; SP arm reduced to single delegating call"
    - "crates/chia-sdk-driver/src/actions.rs — gained #[cfg(feature = chip-0057)] mod silent_payment_send; declaration (no pub re-export)"
tech-stack:
  added: []
  patterns:
    - "Flat-sibling module pattern — silent_payment_send.rs sits alongside send.rs in actions/ matching the existing mint_nft / mint_option / create_did topology"
    - "pub(crate) entry point — handle_silent_payment_send is internal only; public surface stays Action::send"
    - "Atomic helper-relocation — spend_silent_payment + memo_hint_guard moved verbatim with rustdoc"
    - "Test mod by-symbol discovery — cargo test silent_payment_tests filter unchanged because mod name + #[test] symbol names preserved"
key-files:
  created:
    - "crates/chia-sdk-driver/src/actions/silent_payment_send.rs (711 lines)"
  modified:
    - "crates/chia-sdk-driver/src/actions/send.rs (1080 → 399 lines)"
    - "crates/chia-sdk-driver/src/actions.rs (12 → 14 lines; added 2-line chip-0057-gated mod declaration)"
decisions:
  - "Tests relocated WITH the helpers (user override of original RESEARCH Pitfall 3 on 2026-05-20). Pitfall 3 said test discovery breaks if tests move; that was overstated — cargo test silent_payment_tests discovers by symbol name, not file path."
  - "Atomic commit (Tasks 1 + 2 in one git commit). Intermediate state of duplicated helpers would either trip dead_code-deny or require a #[allow] that violates workspace lint policy. One commit avoids the inconsistent state."
  - "No pub use on the new module — handle_silent_payment_send is pub(crate). Public API stays Action::send(id, SendDestination::SilentPayment(addr), amount, memos)."
metrics:
  duration: "8 min"
  completed_date: 2026-05-20T15:54:21Z
  tasks_completed: 2
  files_modified: 3
  files_created: 1
  lines_added: 427
  lines_deleted: 1475
  send_rs_line_count_before: 1080
  send_rs_line_count_after: 399
  silent_payment_send_rs_line_count: 711
---

# Phase 7 Plan 2: Extract chip-0057 SP arm to actions/silent_payment_send.rs Summary

## One-liner

Moved the chip-0057 silent-payment arm of `Action::send` — two private helpers + 10 SP-only tests + the SP arm dispatch — out of the bloated `actions/send.rs` (1080 lines) into a new flat-sibling `actions/silent_payment_send.rs` (711 lines), shrinking `send.rs` to 399 lines (well under the 600-line target). Public API surface unchanged.

## What was done

### Task 1: Create actions/silent_payment_send.rs with relocated helpers and pub(crate) entry point

Created `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (new file). Contents:

1. **`pub(crate) fn handle_silent_payment_send`** — new entry point that the SP arm of `SendAction::spend` calls. It fires the three SP-only guards in cheapest-first order:
   - `DriverError::SilentPaymentRequiresXch` (Id check — O(1))
   - `memo_hint_guard` (CLVM walk — O(1) worst-case, returns early on `Memos::None`)
   - `spend_silent_payment` (parent reservation, k-counter increment, `SilentPaymentPending` push)
2. **`fn spend_silent_payment`** — relocated verbatim from `send.rs:136-179` with its full rustdoc (privacy warning + 3-step body comment).
3. **`fn memo_hint_guard`** — relocated verbatim from `send.rs:193-214` with its full rustdoc (privacy warning + 32-byte first-memo defense).
4. **`#[cfg(all(test, feature = "chip-0057"))] mod silent_payment_tests`** — relocated from `send.rs:496-1080` with all 10 `#[test]` functions intact (8 SEND-* / SP-protocol tests + 2 Wave 0 acceptance tests `silent_payment_destination_requires_xch_id` and `silent_payment_keys_not_registered_errors_at_finish`).

Registered the new module in `crates/chia-sdk-driver/src/actions.rs`:

```rust
#[cfg(feature = "chip-0057")]
mod silent_payment_send;
```

Inserted between `mod settle;` and `mod update_did;` (preserving the alphabetical-by-base-name ordering). **No `pub use`** — the helper is `pub(crate)`, intended for use only by `send.rs`.

### Task 2: Shrink send.rs

Replaced the chip-0057 SP arm in `SendAction::spend` (was 8 inline lines, became 4 lines calling the new helper):

```rust
#[cfg(feature = "chip-0057")]
if let SendDestination::SilentPayment(addr) = &self.destination {
    return crate::actions::silent_payment_send::handle_silent_payment_send(
        ctx,
        spends,
        &self.id,
        addr,
        self.amount,
        self.memos,
    );
}
```

Deleted from `send.rs`:
- `fn spend_silent_payment` and its rustdoc (~57 lines including the `#[cfg]` attribute)
- `fn memo_hint_guard` and its rustdoc (~23 lines)
- `mod silent_payment_tests` and all 10 tests (~585 lines)
- Three SP-only imports: `use chia_sdk_utils::silent_payments::SilentPaymentAddress;` and the entire `#[cfg(feature = "chip-0057")] use crate::{BURN_PUZZLE_HASH, silent_payments::SilentPaymentPending};` block

### Test mod relocation note

Per user decision 2026-05-20 (overriding original RESEARCH Pitfall 3): test discovery in Cargo works by symbol name, not by source file path. Moving `mod silent_payment_tests { ... }` to `silent_payment_send.rs` preserves both the `cargo test silent_payment_tests` CLI filter AND the individual `#[test]` symbol names. The full test path is now `actions::silent_payment_send::silent_payment_tests::*` instead of `actions::send::silent_payment_tests::*`, but the test-name filter (which is what `silent_payment_tests` is on the CLI) still matches the mod name and discovers all 10 tests.

Cargo test output confirms — all 10 tests pass at the new location:

```
test actions::silent_payment_send::silent_payment_tests::memo_hint_guard_allows_none ... ok
test actions::silent_payment_send::silent_payment_tests::action_state_machine ... ok
test actions::silent_payment_send::silent_payment_tests::memo_hint_guard_allows_sentinel_prefixed ... ok
test actions::silent_payment_send::silent_payment_tests::multi_output_same_scan_pk_increments_k ... ok
test actions::silent_payment_send::silent_payment_tests::silent_payment_destination_requires_xch_id ... ok
test actions::silent_payment_send::silent_payment_tests::silent_payment_keys_not_registered_errors_at_finish ... ok
test actions::silent_payment_send::silent_payment_tests::memo_hint_guard_rejects_32_byte_first_memo ... ok
test actions::silent_payment_send::silent_payment_tests::multi_output_distinct_scan_pks_independent_counters ... ok
test actions::silent_payment_send::silent_payment_tests::round_trip_matches_derive_one_time_puzzle_hash ... ok
test actions::silent_payment_send::silent_payment_tests::input_hash_round_trip ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 1078 filtered out
```

## Line-count summary

| File | Before | After | Δ |
|------|--------|-------|---|
| `crates/chia-sdk-driver/src/actions/send.rs` | 1080 | 399 | -681 |
| `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` | — | 711 | +711 |
| `crates/chia-sdk-driver/src/actions.rs` | 24 | 26 | +2 |
| **Net SP-arm code volume** | 1080 | 1110 | +30 |

The net +30 comes from the new file's module-level rustdoc on `handle_silent_payment_send` plus the entry-point function body itself (5 lines + 17-line doc). The original code is moved verbatim; only the new entry point + dispatch trim add lines.

`send.rs` at 399 lines is well under the 600-line ROADMAP target (expected ~370; came in at 399 because `rustfmt` expanded the new dispatch call's argument list across 8 lines instead of the plan's projected 1-2 lines).

## Deviations from Plan

**1. [Rule 3 - Blocking] `Asset` trait not imported in initial silent_payment_send.rs draft**
- **Found during:** Task 2 build (Step 3 import cleanup)
- **Issue:** First compile produced `error[E0599]: no method named full_puzzle_hash found for struct Coin`. The relocated `spend_silent_payment` calls `parent.asset.full_puzzle_hash()` and `parent.asset.coin_id()`, both of which are trait methods on `crate::action_system::asset::Asset`. The plan's import list (Step 1 of Task 1) didn't include `Asset` — only `BURN_PUZZLE_HASH, DriverError, Id, Output, SpendContext, Spends, silent_payments::SilentPaymentPending`.
- **Fix:** Added `Asset,` to the `crate::{...}` import in silent_payment_send.rs (alongside the others).
- **Files modified:** `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Commit:** 190f4d5a (rolled into the atomic commit)

**2. [Rule 1 - Bug] `Memos<NodePtr>` vs `Memos` in plan skeleton**
- **Found during:** Task 1 (verifying signatures before writing)
- **Issue:** The plan's `<action>` skeleton (lines 134, 162, 180-186) used `Memos<NodePtr>` for the helper signatures. The actual code in `send.rs` uses `Memos` (the type alias from `chia_puzzle_types`), not `Memos<NodePtr>`. The current SDK has migrated to the unparameterized `Memos` type alias.
- **Fix:** Used `Memos` (matching the existing `send.rs:141, 193` signatures, the `SendAction` struct field at line 20, and the prevailing project convention).
- **Files modified:** `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`
- **Commit:** 190f4d5a

Aside from these two inline fixes, the plan executed exactly as written.

## Verification

Full verification matrix from plan `<verify>` block:

| Gate | Command | Result |
|------|---------|--------|
| File exists | `test -f crates/chia-sdk-driver/src/actions/silent_payment_send.rs` | PASS |
| Entry point present | `grep -c "pub(crate) fn handle_silent_payment_send" .../silent_payment_send.rs` | 1 |
| spend_silent_payment relocated | `grep -c "fn spend_silent_payment" .../silent_payment_send.rs` | 1 |
| memo_hint_guard relocated | `grep -c "^fn memo_hint_guard" .../silent_payment_send.rs` | 1 |
| spend_silent_payment removed from send.rs | `grep -c "fn spend_silent_payment" .../send.rs` | 0 |
| memo_hint_guard removed from send.rs | `grep -c "fn memo_hint_guard" .../send.rs` | 0 |
| mod silent_payment_tests removed from send.rs | `grep -c 'mod silent_payment_tests' .../send.rs` | 0 |
| mod silent_payment_tests in new file | `grep -c 'mod silent_payment_tests' .../silent_payment_send.rs` | 1 |
| New dispatch call present | `grep -c 'crate::actions::silent_payment_send::handle_silent_payment_send' .../send.rs` | 1 |
| Module declaration in actions.rs | `grep -c 'mod silent_payment_send;' actions.rs` | 1 |
| No public re-export | `grep -c 'pub use silent_payment_send' actions.rs` | 0 |
| send.rs ≤ 600 lines | `wc -l < .../send.rs` | 399 (PASS) |
| Build (driver, chip-0057) | `cargo build --release -p chia-sdk-driver --features chip-0057` | PASS |
| Build (workspace, all-features) | `cargo build --release --workspace --all-features` | PASS |
| Test (SP filter) | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_tests` | 10 ok |
| Test (full driver) | `cargo test --release -p chia-sdk-driver --features chip-0057` | 1088 ok |
| Clippy (driver chip-0057 -D warnings) | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | PASS |
| Clippy (workspace, all-features) | `cargo clippy --workspace --all-features --all-targets` | PASS |
| Format | `cargo fmt --all --check` | PASS |
| Machete | `cargo machete` | zero unused deps |
| Examples | `cargo build --release --examples --all-features` | PASS |

## Atomic commit

**Hash:** `190f4d5a`
**Message:** `refactor(07-02): extract chip-0057 SP arm to actions/silent_payment_send.rs`

Combined Task 1 (new file + module registration) and Task 2 (delete helpers + replace dispatch + move test mod) into one commit per the plan's `<action>` directive. The intermediate state of duplicated helpers across both files would either fail compilation under `deny dead_code` or require a `#[allow]` attribute that violates workspace lint policy.

## Self-Check: PASSED

- File created: `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — FOUND
- File modified: `crates/chia-sdk-driver/src/actions/send.rs` — FOUND (399 lines)
- File modified: `crates/chia-sdk-driver/src/actions.rs` — FOUND
- Commit: `190f4d5a` — FOUND in git log
- All 10 SP tests pass at new location — VERIFIED
- All structural grep oracles pass — VERIFIED
- Workspace build/clippy/fmt/machete all clean — VERIFIED
