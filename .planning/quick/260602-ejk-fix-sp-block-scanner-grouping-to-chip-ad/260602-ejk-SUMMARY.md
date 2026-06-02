---
phase: quick-260602-ejk
plan: 01
subsystem: payments
tags: [chip-0057, silent-payments, scanner, tweak-data, scc, bls12-381]

requires:
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: tweak_data_from_block_spends + simulator adapter + e2e harness
provides:
  - Additive CHIP-0057 ScanBlock grouping (Pass-1-for-all + Pass-2a kept + Pass-2b over all removals) with byte-equality dedup
  - BUG-1 (mixed-PH multi-input) + BUG-2 (single-input PH collision) regression tests
  - External SP/CHIP-team findings doc (bugs + balanced drop-2a proposal + CHIP-history corroboration)
affects: [silent-payments, scanner, receive]

tech-stack:
  added: []
  patterns:
    - "Additive/overlapping multi-pass grouping (union, not partition) with compressed-point dedup"

key-files:
  created:
    - ISSUE-sp-block-scanner-grouping-conformance.md
  modified:
    - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs

key-decisions:
  - "Receiver-only fix: sender (emit_relation / AssertConcurrent cycle / sp_finish_branch) left byte-for-byte unchanged"
  - "Kept Pass 2a (matches current CHIP); drop-2a is a separate upstream proposal documented only in the findings doc"
  - "Dedup by 48-byte compressed point keeping first occurrence; pinned/oracle tests updated to the new additive emission rather than preserving old counts"
  - "BUG-1 e2e mirror lives inline in block_tweak_data.rs; existing test_simulator_e2e_multi_input already covers the e2e multi-input path"

patterns-established:
  - "Pass 1 emits a singleton candidate for EVERY standard removal (only pass that detects single-input sends)"
  - "Pass 2b SCC graph + coin_id->pos map built over ALL standard removals so cross-PH cycles form one SCC"

requirements-completed: [SP-SCAN-CONFORMANCE-01]

duration: 22min
completed: 2026-06-02
---

# Quick 260602-ejk: SP Block-Scanner Additive Grouping + Conformance Findings Doc Summary

**Rewrote `tweak_data_from_block_spends` from a mutually-exclusive three-pass partition to the additive CHIP-0057 ScanBlock union (Pass-1 singleton for every removal + Pass-2a same-PH groups + Pass-2b SCC over all removals, byte-equality deduped), fixing BUG-1 (mixed-PH multi-input fragmentation) and BUG-2 (single-input PH collision), and shipped a self-contained findings doc for the silent-payments/CHIP team.**

## Performance

- **Duration:** ~22 min
- **Tasks:** 2/2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

### Task 1 — Findings/issue doc (commit 0fef228e)
Created `ISSUE-sp-block-scanner-grouping-conformance.md` (158 lines, repo root), matching the voice of the existing `ISSUE-silent-payment-synthetic-key-guard.md`:
- BUG-1 (mixed-PH multi-input fragmentation) + BUG-2 (single-input sharing a PH), each traced through the worked 3-coin / 2-PH example against both the CHIP ScanBlock (detects) and the pre-fix SDK partition (fragments).
- Single-input fact: Pass 2b (SCC size >= 2) structurally cannot detect single-input sends; Pass 1 is mandatory. Backed by sender citations `spends.rs:401` (cycle early-return at <= 1 coin) and `spends.rs:632` (cycle only required for >= 2 non-ephemeral XCH inputs).
- Balanced drop-2a proposal (separate, optional upstream ask) with the efficiency correction (a complete scanner runs CLVM for Pass 2b anyway, so 2a saves nothing) and the fingerprint dimension.
- CHIP-history corroboration citing reference commits dd0e683 and f19f75d.
- CHIP citations: ScanBlock 295-332, Pass 2b 319-330, Edge Cases 466-474; `block_tweak_data.rs` file:line for the exclusion + singletons-for-ungrouped loop.
- Two-concern recommendation (SDK fixed now vs drop-2a is a separate spec proposal). No GSD planning vocabulary.

### Task 2 — Additive grouping rewrite + regression tests (commit 8e17b1f8)
`crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs`:
- **Pass 1** now emits a singleton candidate `vec![i]` for every standard spend (removed the `if !is_grouped` gate) — fixes BUG-2.
- **Pass 2a** same-PH buckets (size >= 2) kept as additional overlapping candidates; dropped the `grouped[]` exclusion bookkeeping entirely.
- **Pass 2b** SCC graph + `coin_id_to_pos` now built over ALL standard removals (removed the `surviving`/`!grouped` restriction) — fixes BUG-1; iterative Tarjan SCC + pollution resistance preserved.
- **Stage 3** byte-equality dedup via a `HashSet<[u8; 48]>` keeping first occurrence; CHIP §459 identity-element (`is_inf`) suppression preserved.
- Module doc rewritten: additive/overlapping union model + documented deterministic emission order (Pass 1 input order → Pass 2a PH-insertion order → Pass 2b Tarjan order → dedup-first).
- Regression tests: `test_bug1_mixed_ph_multi_input_full_cycle_detected` (3 coins, 2 at one PH + 1 at another, one AssertConcurrent cycle → 3-coin aggregate tweak_point present) and `test_bug2_single_input_sharing_ph_detected_via_singleton` (PH collision → single-input Pass-1 singleton present). Both documented inline as failing pre-fix, passing post-fix.
- Pinned tests updated: `test_multi_input_round_trip` 1 → 3 (2 singletons + 1 Pass-2a aggregate); `test_pass_2b_pollution_resistance` 2/1 → 4/3 with membership-based byte-invariance of the legit `{a,b}` SCC point.

## Pre-fix failure confirmation (BUG-1/BUG-2 genuinely caught)

- **BUG-1:** Pre-fix, the two same-PH coins were marked `grouped=true` by Pass 2a and excluded from Pass 2b's `surviving` graph; the third coin's opcode-64 edge into a `grouped` coin had no graph node, so no 3-coin SCC formed and the full `A_sum = pk_dup + pk_dup + pk_solo` was never produced. The test asserts presence of exactly that hand-computed aggregate point — absent under the old partition, present under the additive model.
- **BUG-2:** Pre-fix, both PH-colliding coins fell into the size-2 Pass-2a bucket (`grouped=true`), and singletons were emitted only `if !is_grouped`, so the single-input send's Pass-1 singleton was never produced. The test asserts presence of the hand-computed single-input singleton point — absent under the old partition (only the aggregated same-PH point existed), present under the additive model.

## Verification (full phase gate, all green)

- Planning-residue grep over the full SP source set + doc: **0**
- Findings doc present, >= 80 lines (158), cites dd0e683 + f19f75d + CHIP line ranges
- `cargo build -p chia-sdk-driver` (no features / `--all-features` / `-F chip-0057`): all clean
- `silent_payments::block_tweak_data::tests`: 8 passed (incl. BUG-1, BUG-2)
- `silent_payments_e2e`: 4 passed (unlabeled, multi_input distinct-PH, labeled, m0_self_change)
- Full CI workspace suite (`--workspace --all-features` minus binding crates): **2441 passed, 0 failed**
- `cargo clippy --workspace --all-features --all-targets`: exit 0 (only the 2 pre-existing chia-sdk-daemon client.rs:426-427 warnings remain)
- Scoped `cargo clippy -p chia-sdk-driver --all-features --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
- `cargo machete`: clean (zero unused deps)

## Deviations from Plan

None — plan executed as written. The simulator adapter `tweak_data_from_simulator_block` is a pure delegate and inherited the fix without edits (confirmed by the 4 e2e tests). The optional separate e2e mirror file edit was satisfied inline in `block_tweak_data.rs` per the plan's "may live here or inline" clause; the pre-existing `test_simulator_e2e_multi_input` already covers the end-to-end multi-input detection path, so `silent_payments_e2e.rs` needed no change.

## Hard-constraint compliance

No new workspace deps; no new `#[allow]` attributes; no `unsafe`; chip-0057 code compiles with and without `--all-features`. Sender (`emit_relation`, the AssertConcurrent cycle, `sp_finish_branch`) untouched. Pass 2a retained.

## Commits

- `0fef228e` docs(quick-260602-ejk): add SP block-scanner grouping conformance findings doc
- `8e17b1f8` fix(quick-260602-ejk): make SP block-scanner grouping additive per CHIP ScanBlock

## Self-Check: PASSED

- FOUND: ISSUE-sp-block-scanner-grouping-conformance.md
- FOUND: crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
- FOUND: .planning/quick/260602-ejk-fix-sp-block-scanner-grouping-to-chip-ad/260602-ejk-SUMMARY.md
- FOUND: commit 0fef228e
- FOUND: commit 8e17b1f8
