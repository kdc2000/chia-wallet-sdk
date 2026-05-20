---
phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
plan: 04
subsystem: refactoring

tags: [chip-0057, silent-payments, module-fold, file-deletion, atomic-commit]

# Dependency graph
requires:
  - phase: 08-01-polish-mod-rs-named-reexports
    provides: Intermediate mod.rs shape with named pub-use blocks for aggregate/input_hash/one_time/protocol (this plan collapses to a single pub use protocol::{...} block after the file deletions)
  - phase: 03-receive-primitive-chip-test-vector-closure
    provides: 5 protocol primitives (compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) in protocol.rs; the fold targets these as the canonical home for chip-0057 send-side compositions
  - phase: 04-send-side-action
    provides: 3 send-side compositions (aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash) originally landed as separate single-pub-fn modules — these are the deletion targets
provides:
  - Single canonical home for chip-0057 protocol primitives + their send-side compositions in protocol.rs (500 lines)
  - Final post-Phase-8 silent_payments/mod.rs shape (4 mod declarations, no public-module wildcards)
  - 8 named tests (2 protocol primitive + 6 send-side composition) co-located in one flat mod tests {} block
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Atomic multi-file commit pattern for cross-file refactors where partial states fail to compile (delete source files + update mod.rs + grow target file in one commit)"
    - "Flat mod tests {} block (variant a) absorbs tests from deleted sibling modules; constants deduplicate where byte-identical and same-named, otherwise kept under their original names to preserve semantic clarity"

key-files:
  created: []
  modified:
    - "crates/chia-sdk-driver/src/silent_payments/protocol.rs"
    - "crates/chia-sdk-driver/src/silent_payments/mod.rs"
  deleted:
    - "crates/chia-sdk-driver/src/silent_payments/aggregate.rs"
    - "crates/chia-sdk-driver/src/silent_payments/input_hash.rs"
    - "crates/chia-sdk-driver/src/silent_payments/one_time.rs"

key-decisions:
  - "Flat mod tests {} block (variant a per RESEARCH Open Q3) — all 8 tests inline, no nested sub-modules. Test names byte-identical to deletion-target sources; rustdoc preserved verbatim except brief edits removing dated planning references (one_time.rs:58-59 SilentPaymentSend::memos reference, input_hash.rs:44 opcode-60/61 announcement reference) since both refer to artifacts deleted in Phase 4.1/4.2."
  - "Constant deduplication: TV1_INPUT_HASH (3 byte-identical occurrences), TV1_A_SUM (2), TV1_SCAN_SK (2) each collapse to a single declaration. TV1_AGGREGATED_SENDER_SK and TV4_SENDER_SK_0 are byte-identical scalars (0x5002eaf0...) but BOTH kept by name because semantic meaning differs (TV1 single-input aggregated SK vs TV4 first sender SK)."
  - "Import dedup: chia_sdk_types::silent_payments named list extended to include CHIA_SP_INPUTS (previously only CHIA_SP_SHARED_SECRET, ScalarField, tagged_hash). one_time.rs's file-local `use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk}` dropped — those 3 names are now siblings in this file."
  - "Module rustdoc extended with a 'Send-side compositions (high-level)' subsection alongside the existing 'Protocol primitives (low-level)' bullet list — per RESEARCH Open Q5 recommendation."
  - "Risk R8 atomic commit honored: 5 file changes (3 deletions + 2 modifications) land in a single commit (cab84ad2). No transient build-break state visible in git log."

patterns-established:
  - "When folding single-pub-fn sibling modules into one canonical module, write the target file in full first, edit the umbrella mod.rs to point at the new shape second, delete the source files third, run a workspace --all-features build to verify atomicity, then stage all 5 paths and commit as one."

requirements-completed: [POLISH-02]

# Metrics
duration: 14min
completed: 2026-05-20
---

# Phase 8 Plan 04: POLISH-02 Atomic Fold of aggregate/input_hash/one_time into protocol.rs Summary

**Folded three single-`pub-fn` silent-payments modules into `protocol.rs` via a single atomic commit; the canonical home for chip-0057 protocol primitives + their send-side compositions is now one file (500 lines, well under the 700-line POLISH-02 ceiling).**

## Performance

- **Duration:** ~14 min
- **Tasks:** 2 (1 precondition/baseline + 1 atomic-fold)
- **Files changed:** 5 (3 deleted + 2 modified) — single commit `cab84ad2`
- **Lines:** +336 / -487 net (-151 — dedup payoff)

## Pre-Fold Inventory

| File | Lines | Contents |
|------|-------|----------|
| `aggregate.rs` | 91 | `pub fn aggregate_sender_sks` + 1 test (`tv4_aggregate_sender_sks_matches`) |
| `input_hash.rs` | 157 | `pub fn compute_input_hash` + 3 tests (`tv1_compute_input_hash_matches`, `input_hash_uses_lex_min_coin_id`, `input_hash_order_independent`) |
| `one_time.rs` | 193 | `pub fn derive_one_time_puzzle_hash` + 2 tests (`tv1_derive_one_time_puzzle_hash_matches`, `derive_one_time_puzzle_hash_k1_round_trip`) |
| `protocol.rs` | 199 | 5 protocol primitive pub fns + 2 tests (`tv1_shared_secret_matches`, `adversarial_ff32_scalar_reduces_unsigned`) |
| **Pre-fold total** | **640** | 8 pub fns + 8 tests across 4 files |

## Post-Fold Inventory

| File | Lines | Contents |
|------|-------|----------|
| `protocol.rs` | **500** | 8 pub fns (5 protocol primitives + 3 send-side compositions) + 8 tests in one flat `mod tests {}` block |
| `aggregate.rs` | — | **DELETED** |
| `input_hash.rs` | — | **DELETED** |
| `one_time.rs` | — | **DELETED** |
| **Post-fold total** | **500** | Single file; 22% line reduction from dedup of imports, module-level rustdoc, constants, and tests fixtures |

## Acceptance Oracles

All 9 oracles from `plan_specific_guidance` pass:

| # | Oracle | Result |
|---|--------|--------|
| 1 | `test ! -f aggregate.rs` | PASS |
| 2 | `test ! -f input_hash.rs` | PASS |
| 3 | `test ! -f one_time.rs` | PASS |
| 4 | `wc -l protocol.rs ≤ 700` | PASS (500) |
| 5 | `cargo test --release -p chia-sdk-driver --features chip-0057` test count unchanged | PASS (1085 unit + 3 e2e pre- and post-fold) |
| 6 | `cargo build --release --workspace --all-features` exit 0 | PASS (all 6 external callsites resolve) |
| 7 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exit 0 | PASS |
| 8 | `cargo fmt --all --check` exit 0 | PASS |
| 9 | `cargo doc -p chia-sdk-driver --features chip-0057 --no-deps` exit 0 (Risk R7) | PASS |

## All 8 Named Tests

| Test | Status | Origin (pre-fold) |
|------|--------|-------------------|
| `tv1_shared_secret_matches` | PASS | protocol.rs (existing) |
| `adversarial_ff32_scalar_reduces_unsigned` | PASS | protocol.rs (existing) |
| `tv4_aggregate_sender_sks_matches` | PASS | aggregate.rs (absorbed) |
| `tv1_compute_input_hash_matches` | PASS | input_hash.rs (absorbed) |
| `input_hash_uses_lex_min_coin_id` | PASS | input_hash.rs (absorbed) |
| `input_hash_order_independent` | PASS | input_hash.rs (absorbed) |
| `tv1_derive_one_time_puzzle_hash_matches` | PASS | one_time.rs (absorbed) |
| `derive_one_time_puzzle_hash_k1_round_trip` | PASS | one_time.rs (absorbed) |

All 8 tests now reside under `silent_payments::protocol::tests::*` (pre-fold they were spread across `silent_payments::{aggregate, input_hash, one_time, protocol}::tests::*`).

## Test Merger Shape

**Variant (a) per RESEARCH §Open Q3 recommendation** — flat `#[cfg(test)] mod tests {}` block at the bottom of `protocol.rs` with all 8 tests inline, no nested sub-modules. The block contains:

- 1 `use super::*;` + 2 inline `use` (`chia_bls::SecretKey`, `hex_literal::hex`)
- 10 deduplicated TV1/TV4 hex constants (3 byte-identical sets resolved to single declarations; 7 unique constants kept under their original names)
- 2 helper fns (`tv1_tweak_point()`, `scan_sk()` — used by `tv1_shared_secret_matches`)
- 8 `#[test]` fns organized into two visually-delineated sections: "Tests for the protocol primitives" (2 tests) and "Tests for the send-side compositions" (6 tests)

## Constant Deduplication Outcome

| Constant | Pre-fold locations | Post-fold |
|----------|-------------------|-----------|
| `TV1_INPUT_HASH` | input_hash.rs::tests, one_time.rs::tests, protocol.rs::tests | 1 declaration (deduped) |
| `TV1_A_SUM` | input_hash.rs::tests, protocol.rs::tests | 1 declaration (deduped) |
| `TV1_SCAN_SK` | one_time.rs::tests, protocol.rs::tests | 1 declaration (deduped) |
| `TV1_SCAN_PK` | one_time.rs::tests | 1 declaration (kept) |
| `TV1_SPEND_PK` | one_time.rs::tests | 1 declaration (kept) |
| `TV1_AGGREGATED_SENDER_SK` | one_time.rs::tests | 1 declaration (kept; byte-identical to `TV4_SENDER_SK_0` per Pitfall 1 but both names retained) |
| `TV1_PUZZLE_HASH` | one_time.rs::tests | 1 declaration (kept) |
| `TV1_SHARED_SECRET` | protocol.rs::tests | 1 declaration (kept) |
| `TV1_COIN_ID` | input_hash.rs::tests | 1 declaration (kept) |
| `TV4_SENDER_SK_0` | aggregate.rs::tests | 1 declaration (kept; byte-identical to `TV1_AGGREGATED_SENDER_SK` but semantically distinct) |
| `TV4_SENDER_SK_1` | aggregate.rs::tests | 1 declaration (kept) |
| `TV4_AGGREGATED_SK` | aggregate.rs::tests | 1 declaration (kept) |

Total: 12 unique constants (3 deduplicated, 9 kept-as-is).

## Import Dedup Outcome

| Import | Pre-fold (across 4 files) | Post-fold (in protocol.rs) |
|--------|---------------------------|----------------------------|
| `use chia_bls::{PublicKey, SecretKey};` | scattered (some files had `PublicKey` only, others `SecretKey` only) | merged — both imported once |
| `use chia_protocol::Bytes32;` | 3 files | 1 declaration |
| `use chia_puzzle_types::DeriveSynthetic;` | protocol.rs only | unchanged |
| `use chia_puzzle_types::standard::StandardArgs;` | protocol.rs only | unchanged |
| `use chia_sdk_types::silent_payments::{...}` named list | protocol.rs (3 names), input_hash.rs (3 names) | merged — 4 names (added `CHIA_SP_INPUTS`) |
| `use chia_sha2::Sha256;` | protocol.rs, one_time.rs | 1 declaration |
| `use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk};` (one_time.rs:36) | one_time.rs only | DROPPED — names are now siblings in this file |
| `use crate::silent_payments::compute_shared_secret_from_tweak;` (one_time.rs:157, inline in test) | one_time.rs::tests only | DROPPED — `use super::*;` covers it |

## Module Rustdoc Update

Extended the existing 5-bullet "Protocol primitives" list with a parallel "Send-side compositions" bullet list (per RESEARCH Open Q5 recommendation):

```text
//! Protocol primitives (low-level — operate on a single shared secret / scalar):
//!  - compute_shared_secret_from_tweak
//!  - derive_output_tweak
//!  - derive_onetime_pk
//!  - derive_onetime_sk
//!  - puzzle_hash_for_pk
//!
//! Send-side compositions (high-level — fold the primitives into the values the
//! send action and Spends::finish_with_keys need):
//!  - aggregate_sender_sks
//!  - compute_input_hash
//!  - derive_one_time_puzzle_hash
```

## mod.rs Collapse (Plan 01 Intermediate → Plan 04 Final)

**Pre-Plan-04 (Plan 01 intermediate, lines 31-54):**
```text
mod aggregate;
pub use aggregate::aggregate_sender_sks;
mod input_hash;
pub use input_hash::compute_input_hash;
mod one_time;
pub use one_time::derive_one_time_puzzle_hash;
mod protocol;
pub use protocol::{compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk,
                   derive_output_tweak, puzzle_hash_for_pk};
mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};
mod send_keys;
pub(crate) use send_keys::*;
mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
```

**Post-Plan-04 (final D-01 shape, lines 31-42):**
```text
mod protocol;
pub use protocol::{
    aggregate_sender_sks, compute_input_hash, compute_shared_secret_from_tweak,
    derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,
    puzzle_hash_for_pk,
};
mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};
mod send_keys;
pub(crate) use send_keys::*;
mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
```

4 mod declarations (down from 6 intermediate); 4 `pub use` blocks (down from 6); 1 `pub(crate) use` wildcard (preserved per D-01). Zero public-module wildcards.

## External Callsites (All 6 Verified)

`cargo build --release --workspace --all-features` exit 0 after the fold; all 6 paths still resolve through the named `pub use protocol::{...}` block in mod.rs:

- `crates/chia-sdk-bindings/src/silent_payments.rs` — 3 fns (`derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`)
- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` — 3 names (`OutputMeta`, `TweakData`, `compute_input_hash`)
- `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` — 2 names (`K_MAX_DEFAULT`, `scan_from_tweaks`)
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` — 3 names (`aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash`)
- `crates/chia-sdk-driver/src/action_system/spends.rs` — 3 names (`aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash`)
- `src/prelude.rs` — 13 names re-exported through `chia_sdk_driver::silent_payments`

## Atomic-Commit Oracle (Risk R8)

`git log -1 --name-status` shows exactly 5 entries:

```text
D    crates/chia-sdk-driver/src/silent_payments/aggregate.rs
D    crates/chia-sdk-driver/src/silent_payments/input_hash.rs
M    crates/chia-sdk-driver/src/silent_payments/mod.rs
D    crates/chia-sdk-driver/src/silent_payments/one_time.rs
M    crates/chia-sdk-driver/src/silent_payments/protocol.rs
```

Commit `cab84ad2`: 5 files changed, +336 insertions / -487 deletions. No transient build-break state visible in git log — the file deletions, the protocol.rs append, and the mod.rs collapse all land together.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy `doc_lazy_continuation` error in `derive_one_time_puzzle_hash_k1_round_trip` test rustdoc**
- **Found during:** Step 11 (lint sweep)
- **Issue:** The rustdoc text "Re-derives the expected puzzle hash via the same protocol-primitive chain (`compute_shared_secret_from_tweak` + `derive_output_tweak(.., 1)` / + `derive_onetime_pk` + `puzzle_hash_for_pk`)" wrapped across 3 lines where the continuation lines began with `+ ...` and ` Catches `, both of which clippy interpreted as broken list-item continuations under `-D warnings`.
- **Fix:** Replaced the `+`-joined fluent style with comma-separated `,`-joined list (`(fn_a, fn_b, fn_c, fn_d)`), eliminating the line-leading-`+` continuation ambiguity. Rustdoc renders identically; clippy passes.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (test rustdoc only)
- **Commit:** cab84ad2 (folded into atomic commit)

**2. [Rule 1 - Bug] rustfmt re-wraps the `pub use protocol::{...}` block in mod.rs**
- **Found during:** Step 11 (cargo fmt --check)
- **Issue:** Plan specified one-symbol-per-line for the 8-name `pub use protocol::{...}` block; rustfmt prefers fitting 3-on-a-line under 100-col width.
- **Fix:** Accepted rustfmt's preferred form (matches Plan 08-01's documented rustfmt-canonical-form precedent — "rustfmt re-wraps multi-line pub use protocol::{...} block to fit under 100-col width; this is the workspace-canonical form").
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/mod.rs`, `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (chia_sdk_types named-list import also re-wrapped to multi-line form)
- **Commit:** cab84ad2 (folded into atomic commit)

**3. [Rule 1 - Doc] Doc edits removing dated planning references**
- **Found during:** Step 1c (copying rustdoc verbatim from deletion targets)
- **Issue:** `one_time.rs:58-59` referenced `SilentPaymentSend::memos` (deleted in Phase 4.2) and `input_hash.rs:44` referenced `opcode-60/61 announcement linkage` (deleted in Phase 4.1 / FINGERPRINT-01).
- **Fix:** Replaced with current-architecture references — "the action-layer memo-hint guard" (generic) and "`Relation::AssertConcurrent` cycle (opcode 64 SCC) the SDK emits on multi-input bundles" respectively. Both edits were anticipated by the plan's Step 1c notes.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (rustdoc on `compute_input_hash` and `derive_one_time_puzzle_hash`)
- **Commit:** cab84ad2 (folded into atomic commit)

### Pre-existing Out-of-Scope Findings

- `cargo build --release -p chia-sdk-driver --no-default-features` emits 1 `missing_copy_implementations` warning on `SendDestination` enum. **Pre-existing** on main pre-fold (Plan 07-05 deferred to `deferred-items.md`, also Plan 04.2-01 boxed the variant to satisfy `clippy::large_enum_variant`). Out of scope for POLISH-02.

### Authentication Gates

None.

## Known Stubs

None.

## Self-Check: PASSED

- File created: `.planning/phases/08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics/08-04-SUMMARY.md` (this file)
- Commit verified: `cab84ad2` present in `git log --oneline` (`refactor(08-04): fold aggregate/input_hash/one_time into protocol.rs (POLISH-02)`)
- Files deleted: 3 deletion-target oracles all exit 0
- Build/test/lint/doc oracles: all 9 listed acceptance oracles pass
