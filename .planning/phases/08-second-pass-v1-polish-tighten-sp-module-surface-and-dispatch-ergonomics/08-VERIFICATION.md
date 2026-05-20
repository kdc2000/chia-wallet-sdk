---
phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
verified: 2026-05-20T20:39:47Z
status: passed
score: 4/4 must-haves verified
requirements_verified:
  - POLISH-01
  - POLISH-02
  - POLISH-03
  - POLISH-04
---

# Phase 8: Second-pass v1 polish — tighten SP module surface and dispatch ergonomics Verification Report

**Phase Goal:** Address 4 residual structural nits in the chip-0057 silent-payments code identified by a post-Phase-7 quality survey (2026-05-20). Distinct from the 5 issues Phase 7 already fixed; same shape (pure refactor, no behavior change, no API removals). Closes the polish gap before upstream merge.

**Verified:** 2026-05-20T20:39:47Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| - | ----- | ------ | -------- |
| 1 | POLISH-01: `silent_payments/mod.rs` has zero public `pub use foo::*;` wildcards; full --all-features workspace build clean | ✓ VERIFIED | `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` = **0** (down from 6 baseline). `pub(crate) use send_keys::*;` survives (exactly 1, D-01 permitted). `cargo build --release --workspace --all-features` finished in 3m 21s, exit 0. |
| 2 | POLISH-02: `aggregate.rs`, `input_hash.rs`, `one_time.rs` deleted; contents folded into `protocol.rs`; line count ≤ 700; test count unchanged; all callsites resolve | ✓ VERIFIED | `test ! -f` confirmed for all 3 deletion targets (only `mod.rs`, `protocol.rs`, `scanner.rs`, `send_keys.rs`, `types.rs` remain). `wc -l protocol.rs` = **500** (well under 700; slightly under D-02's ~640 target). chip-0057 driver lib tests: **1085 passed, 0 failed**; e2e integration target: **3 passed, 0 failed**. All 8 named protocol tests present + passing. |
| 3 | POLISH-03: enum-level "Cannot derive Copy" paragraph removed; `large_enum_variant` reference retained exactly once (variant-level) | ✓ VERIFIED | `grep -c 'Cannot derive .Copy.' send_destination.rs` = **0**; `grep -c 'large_enum_variant' send_destination.rs` = **1** (variant-level rustdoc at line 32). |
| 4 | POLISH-04: `Action::send` chip-0057 dispatch is a single exhaustive `match` with SP arm `return`-ing from inside; no `unreachable!()` | ✓ VERIFIED | `grep -c 'unreachable!' send.rs` = **0**; `grep -c 'handle_silent_payment_send' send.rs` = **1** (at line 45, inside the SP match arm). Visual confirmation: single match at send.rs:41-54, SP arm returns directly. |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/chia-sdk-driver/src/silent_payments/mod.rs` | Named re-exports only; final D-01 post-fold shape | ✓ VERIFIED | 43 lines; 4 sub-modules declared (`protocol`, `scanner`, `send_keys`, `types`); 3 named `pub use` blocks + 1 `pub(crate) use send_keys::*;`. Body matches locked D-01 shape (cargo-fmt-canonical wrapping). |
| `crates/chia-sdk-driver/src/silent_payments/protocol.rs` | Grown to host 3 folded pub fns + 5 protocol primitives + merged tests | ✓ VERIFIED | 500 lines. 8 `pub fn` exported (5 primitives + 3 send-side compositions). Single `#[cfg(test)] mod tests {}` at line 251 with all 8 named tests. |
| `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` | Deleted | ✓ VERIFIED | File absent. |
| `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` | Deleted | ✓ VERIFIED | File absent. |
| `crates/chia-sdk-driver/src/silent_payments/one_time.rs` | Deleted | ✓ VERIFIED | File absent. |
| `crates/chia-sdk-driver/src/action_system/send_destination.rs` | Variant-level Boxing doc only; enum-level Copy/Boxing paragraph removed | ✓ VERIFIED | 49 lines. Enum doc retains purpose + privacy warning (lines 14-27). Variant rustdoc at lines 31-39 references `large_enum_variant` (the only mention in file). |
| `crates/chia-sdk-driver/src/actions/send.rs` | Single exhaustive match for `SendDestination`; SP arm `return`s; no `unreachable!()` | ✓ VERIFIED | Lines 41-54 contain the single match. SP arm calls `crate::actions::silent_payment_send::handle_silent_payment_send(...)` and returns. `unreachable!` absent from file. |
| `src/prelude.rs:41-45` (umbrella prelude) | 13 SP names byte-identical | ✓ VERIFIED | 13 names exposed (`DetectedSpCoin`, `K_MAX_DEFAULT`, `OutputMeta`, `SilentPaymentScan`, `TweakData`, `compute_input_hash`, `compute_shared_secret_from_tweak`, `derive_one_time_puzzle_hash`, `derive_onetime_pk`, `derive_onetime_sk`, `derive_output_tweak`, `puzzle_hash_for_pk`, `scan_from_tweaks`). `cargo check -p chia-wallet-sdk --all-features` clean. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `chia-sdk-bindings/src/silent_payments.rs:416,429,433` | `chia_sdk_driver::{derive_one_time_puzzle_hash, compute_input_hash, aggregate_sender_sks}` | Top-level driver re-export through `silent_payments::*` named list | ✓ WIRED | All 3 names appear in `pub use protocol::{...}` of mod.rs. Bindings facade compiles. |
| `chia-sdk-test/src/silent_payments/tweak_data.rs:10` | `chia_sdk_driver::silent_payments::{OutputMeta, TweakData, compute_input_hash}` | Direct module path import | ✓ WIRED | All 3 names re-exported via `protocol`/`types` named blocks; file compiles. |
| `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` | `chia_sdk_driver::silent_payments::{K_MAX_DEFAULT, scan_from_tweaks}` | Direct module path import | ✓ WIRED | Both names in scanner named-list; 3 e2e tests pass. |
| `crates/chia-sdk-driver/src/actions/silent_payment_send.rs:138` | `silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash}` | Direct module path import (inside test mod) | ✓ WIRED | All 3 names in protocol named-list; clippy `-D warnings` clean. |
| `crates/chia-sdk-driver/src/action_system/spends.rs:596` | `crate::silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash}` | Direct module path import | ✓ WIRED | All 3 names resolve; full chip-0057 test target passes 1085 tests. |
| `actions/send.rs::SendAction::spend` chip-0057 arm | `actions/silent_payment_send.rs::handle_silent_payment_send` | `pub(crate)` function call from inside single match arm | ✓ WIRED | send.rs:45 calls into silent_payment_send.rs:21 (`pub(crate) fn handle_silent_payment_send`). Behavior identical to Phase 7's pre-POLISH-04 if-let dispatch. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `silent_payments/protocol.rs::derive_one_time_puzzle_hash` | scan_pk/spend_pk/aggregated_sender_sk/input_hash/k | Caller (send-side action) | Yes | ✓ FLOWING |
| `silent_payments/protocol.rs::aggregate_sender_sks` | `sks` (synthetic SKs from caller's keystore) | Caller | Yes | ✓ FLOWING |
| `silent_payments/protocol.rs::compute_input_hash` | `coin_ids` (real coin IDs from spends) + `aggregated_sender_pk` | Caller (spends builder + aggregate fold) | Yes | ✓ FLOWING |
| `tests/silent_payments_e2e.rs` 3 e2e tests | `tweak_data` from live simulator | `tweak_data_from_simulator_block(&sim, height_before)` | Yes | ✓ FLOWING |
| `actions/send.rs` single match dispatch | `self.destination` (PuzzleHash or SilentPayment) | Caller's `Action::send(...)` | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| POLISH-01 acceptance grep — 0 public wildcards | `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` | `0` | ✓ PASS |
| send_keys.rs crate-private wildcard preserved | `grep -c '^pub(crate) use send_keys::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` | `1` | ✓ PASS |
| POLISH-02 deletions | `test ! -f` for aggregate.rs, input_hash.rs, one_time.rs | all 3 absent | ✓ PASS |
| POLISH-02 protocol.rs line ceiling ≤ 700 | `wc -l protocol.rs` | `500` | ✓ PASS |
| POLISH-03 enum-level "Cannot derive Copy" removed | `grep -c 'Cannot derive .Copy.' send_destination.rs` | `0` | ✓ PASS |
| POLISH-03 `large_enum_variant` referenced exactly once | `grep -c 'large_enum_variant' send_destination.rs` | `1` | ✓ PASS |
| POLISH-04 no `unreachable!` in send.rs | `grep -c 'unreachable!' send.rs` | `0` | ✓ PASS |
| POLISH-04 `handle_silent_payment_send` referenced exactly once | `grep -c 'handle_silent_payment_send' send.rs` | `1` | ✓ PASS |
| Full workspace build under --all-features | `cargo build --release --workspace --all-features` | exit 0, 3m 21s | ✓ PASS |
| Umbrella crate (prelude) compiles | `cargo check --release -p chia-wallet-sdk --all-features` | exit 0 | ✓ PASS |
| Scoped clippy on chia-sdk-driver chip-0057 -D warnings | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | exit 0 | ✓ PASS |
| Scoped clippy on chia-sdk-bindings all-features -D warnings | `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` | exit 0 | ✓ PASS |
| Workspace fmt check | `cargo fmt --all --check` | exit 0 | ✓ PASS |
| Cargo machete (unused deps) | `cargo machete` | "didn't find any unused dependencies. Good job!" | ✓ PASS |
| All 8 named silent-payments-protocol tests run + pass | `cargo test --release -p chia-sdk-driver --features chip-0057 --lib silent_payments::protocol` | `8 passed; 0 failed; 0 ignored` | ✓ PASS |
| 3 e2e integration tests pass | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` | `3 passed; 0 failed; 0 ignored` | ✓ PASS |
| Full chip-0057 driver test suite | `cargo test --release -p chia-sdk-driver --features chip-0057` | `1085 (lib) + 3 (e2e) = 1088 passed; 0 failed` | ✓ PASS |
| Full workspace test suite (excluding binding crates) | `cargo test --release --workspace --all-features --exclude <binding crates>` | 2332 lib + integration targets, 0 failures | ✓ PASS |
| Risk R8: POLISH-02 single atomic 5-file commit | `git show --name-status cab84ad2` | D aggregate.rs, D input_hash.rs, M mod.rs, D one_time.rs, M protocol.rs (5 files in 1 commit) | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| POLISH-01 | 08-01-PLAN | Replace `pub use foo::*;` wildcards in `silent_payments/mod.rs` with named re-exports | ✓ SATISFIED | `grep -cE '^pub use [a-z_]+::\*;'` returns 0. `pub(crate) use send_keys::*;` survives (1 hit, D-01 permitted). All 13 prelude SP names + binding-facade-consumed `aggregate_sender_sks` remain reachable. Workspace build + scoped clippy clean. Commit 75d56116. |
| POLISH-02 | 08-04-PLAN | Atomic fold of `aggregate.rs` + `input_hash.rs` + `one_time.rs` into `protocol.rs` | ✓ SATISFIED | 3 files deleted; `protocol.rs` is 500 lines (≤ 700 ceiling). All 8 named tests live in single `#[cfg(test)] mod tests {}` at line 251 and pass. External callsites (bindings, test, prelude, internal driver consumers, e2e integration target) resolve identically. Single atomic 5-file commit cab84ad2 (Risk R8 satisfied). |
| POLISH-03 | 08-02-PLAN | Delete duplicated `SendDestination` Boxing rationale (keep variant-level only) | ✓ SATISFIED | Enum-level `Cannot derive Copy` paragraph removed (grep returns 0). Variant-level `large_enum_variant` reference preserved at line 32 (grep returns 1). 49-line file vs pre-phase ~52 lines (3 lines saved as planned). Commit 14e7ecd8. |
| POLISH-04 | 08-03-PLAN | Restructure `Action::send` chip-0057 dispatch to a single `match`; remove `unreachable!()` | ✓ SATISFIED | `grep -c 'unreachable!' send.rs` returns 0. `grep -c 'handle_silent_payment_send' send.rs` returns 1 (the single SP match arm at line 45). Behavior unchanged: PuzzleHash falls through to fungible dispatch, SilentPayment returns from inside the arm. Commit 54176195. |

No orphaned requirements. All 4 POLISH-* IDs declared in plan frontmatter and confirmed against `.planning/REQUIREMENTS.md` lines 79-82 (Phase 8 functional block) and lines 148-151 (registry table). Both REQUIREMENTS.md locations show `[x]` and map to Phase 8.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/chia-sdk-daemon/src/client.rs` | 426 | `match_wildcard_for_single_variants` (clippy pedantic) | ℹ️ Info | Pre-existing on `main` (documented in Phase 1, Phase 7 Plan 03, Phase 7 VERIFICATION deferred-items). Untouched by Phase 8. |
| `crates/chia-sdk-daemon/src/client.rs` | 427 | `match_same_arms` (clippy pedantic) | ℹ️ Info | Pre-existing on `main` (same provenance as above). Untouched by Phase 8. |
| `crates/chia-sdk-driver/src/action_system/send_destination.rs` (no-features build) | — | `missing_copy_implementations` on `SendDestination` | ℹ️ Info | Pre-existing on `main` (chip-0057 off → enum collapses to single Copy-eligible variant; out-of-scope per Phase 6 and Phase 7 deferred-items precedent). |

No blocker or warning-level anti-patterns introduced by Phase 8. All 3 findings are pre-existing, documented in prior phases' deferred-items, and exempted by Phase 8's scoped clippy gates passing with `-D warnings`.

### Human Verification Required

None. All must-haves are programmatically verifiable via grep oracles, file-existence checks, test execution, and clippy/fmt/machete gates. Phase 8 is a refactor + audit-cleanup phase with no UX or visual surface.

### Gaps Summary

None. All 4 observable truths verified. All 8 required artifacts pass file-existence, substantive-content, wiring, and data-flow levels. All 6 key links verified (3 named callsites in bindings/test/e2e + 2 internal driver consumers + the send.rs → silent_payment_send.rs `pub(crate)` call). All 4 POLISH-* requirements satisfied. All 19 behavioral spot-checks pass. Full chip-0057 driver test suite: 1088 passed, 0 failed (1085 lib + 3 e2e), same count as pre-phase. Full workspace test suite under `--all-features` (excluding binding crates per CI convention): 2332 lib tests + integration targets, 0 failures. Scoped clippy with `-D warnings` clean on `chia-sdk-driver` (chip-0057) and `chia-sdk-bindings` (all-features). `cargo fmt --all --check` and `cargo machete` clean. Risk R8 (POLISH-02 atomic single commit) confirmed: commit cab84ad2 deletes the 3 source files + grows protocol.rs + updates mod.rs in one atomic step.

The format diff between the locked D-01 final shape and current `silent_payments/mod.rs` is purely cargo-fmt-canonical line wrapping of the same 8 protocol symbols (7 single-name lines collapsed into 2 multi-name lines). `cargo fmt --all --check` exits 0, confirming this is the workspace-correct formatting; no semantic change.

Phase 8 is requirement-complete. The v1+polish silent-payments work (Phases 1–6 functional + Phase 7 first-pass polish + Phase 8 second-pass polish) is ready for upstream merge.

---

_Verified: 2026-05-20T20:39:47Z_
_Verifier: Claude (gsd-verifier)_
