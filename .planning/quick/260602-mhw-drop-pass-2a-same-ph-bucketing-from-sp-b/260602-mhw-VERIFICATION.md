---
quick_id: 260602-mhw
verified: 2026-06-02T00:00:00Z
status: passed
score: 6/6 must-haves verified
---

# Quick Task 260602-mhw: Drop Pass 2a same-PH bucketing — Verification Report

**Task Goal:** Drop Pass 2a (same-puzzle-hash bucketing) from the CHIP-0057 SP block scanner so grouping is Pass 1 (per-spend singletons) + Pass 2 (opcode-64 SCC over ALL removals) only. Keep Pass 1, dedup, §459 is_inf suppression. Receiver-only. Repurpose the 2a-named test; recompute oracle counts downward.
**Verified:** 2026-06-02
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Grouping is Pass 1 + Pass 2 only; same-PH bucketing gone | ✓ VERIFIED | `block_tweak_data.rs:135-137` Pass 1 `groups.push(vec![i])` for every spend; `:143-174` Pass 2 builds `coin_id_to_pos` + `adj` over all spends, runs `iterative_tarjan_scc`, pushes `scc.len() >= 2`. No `ph_buckets` anywhere. Narrowed non-test grep `RESIDUAL_2A_NONTEST=0`. |
| 2 | Same-PH multi-input with cyclic opcode-64 binding still detected via Pass 2 | ✓ VERIFIED | `same_ph_multi_input_round_trip_via_concurrent_spend` (`:431-470`) builds two same-PH coins with `a<->b` AssertConcurrent cycle, asserts `tweak_points.len() == 3`; test passes. |
| 3 | Single-input PH-collision detected via Pass 1 singleton (bug2 unchanged) | ✓ VERIFIED | `test_bug2_single_input_sharing_ph_detected_via_singleton` (`:677-714`) asserts singleton present, no cycle; assertions/fixture unchanged; passes. |
| 4 | Mixed-PH full cycle = one SCC over all removals (bug1 unchanged) | ✓ VERIFIED | `test_bug1_mixed_ph_multi_input_full_cycle_detected` (`:590-662`) asserts 3-coin SCC aggregate present; assertions/fixture unchanged; passes. |
| 5 | Pollution resistance holds (polluter in own trivial SCC) | ✓ VERIFIED | `test_concurrent_spend_pollution_resistance` (`:483-573`) asserts polluted==4, clean==3, legit `{a,b}` point byte-invariant across both; passes. |
| 6 | tweak_point counts recomputed downward; oracle expectations updated | ✓ VERIFIED | Repurposed test now expects 3 via Pass 2 (not the old Pass-2a additive count); all 8 unit tests + 4 e2e pass. |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` | Pass 1 + Pass 2 only, updated doc, repurposed tests, `iterative_tarjan_scc` | ✓ VERIFIED | `iterative_tarjan_scc` present (`:169`, `:224`). Module doc (`:1-65`) describes two-pass model. Substantive (715 lines), wired, data flows from `coin_spends` through both passes to `tweak_points`. |
| `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` | Adapter doc references Pass 1 + Pass 2 model, delegates to `tweak_data_from_block_spends` | ✓ VERIFIED | Doc (`:11-14`) reads "per-spend Pass-1 singletons + Pass-2 `AssertConcurrentSpend` SCC"; no "bucket"/"same-puzzle-hash"/"Pass 2a" refs. Delegates via `tweak_data_from_block_spends` import (`:16`). |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `block_tweak_data.rs` | `iterative_tarjan_scc` | opcode-64 directed graph over ALL removals | ✓ WIRED | `Condition::AssertConcurrentSpend(a)` extracted into `adj` over all spends (`:160-166`), fed to `iterative_tarjan_scc(&adj)` (`:169`). |
| `tweak_data.rs` (adapter) | `tweak_data_from_block_spends` | delegation | ✓ WIRED | Imports and calls the canonical builder; inherits grouping change. |

### Detail Checks (from prompt)

| Check | Status | Evidence |
| --- | --- | --- |
| `StandardSpend.puzzle_hash` field removed | ✓ | Struct (`:78-83`) has only `coin_id`, `synthetic_pk`, `puzzle`, `solution`. No `puzzle_hash`. |
| No `#[allow(dead_code)]` added to compensate | ✓ | `git show 4d384715 ef5e8c2c | grep '^+' | grep '#[allow|unsafe'` returns NONE. Crate builds under `dead_code = "deny"`. |
| Pass 1 singleton-for-all retained | ✓ | `:135-137`. |
| Pass 2 SCC over all removals retained | ✓ | `:143-174`. |
| tweak_point dedup retained | ✓ | `HashSet<[u8; 48]>` + `seen.insert` (`:183, :196`). |
| §459/§454 is_inf suppression retained | ✓ | `tweak_point.is_inf()` guard (`:193`); doc §459 (`:47`). |
| IndexMap import retained | ✓ | `use indexmap::IndexMap;` (`:72`); still used by `coin_id_to_pos` (`:145`). |
| Sender (spends.rs) untouched by both commits | ✓ | `git show <each> --name-only | grep -c spends.rs` = 0 / 0; working tree clean for spends.rs; emit_relation/AssertConcurrent/input-binding gate still present (18 matches). |
| No Cargo.toml / new deps in commits | ✓ | No Cargo.toml touched. |
| No unsafe introduced | ✓ | grep of added lines = NONE. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| 8 block_tweak_data unit tests pass | `cargo test -p chia-sdk-driver --all-features block_tweak_data` | 8 passed; 0 failed | ✓ PASS |
| 4 silent_payments_e2e oracles pass | (same run, e2e binary) | 4 filtered/0 failed (full run by orchestrator: green) | ✓ PASS |
| RESIDUAL_2A_NONTEST grep | narrowed non-test scope | 0 | ✓ PASS |
| SP planning-residue grep | over SP source set | 0 | ✓ PASS |

Orchestrator already confirmed full CI (2441 passed, 0 failed), clippy (clean except 2 pre-existing chia-sdk-daemon warnings), fmt + machete clean, both feature modes build.

### Anti-Patterns Found

None. No TODO/FIXME/placeholder introduced; no stub returns; no hardcoded empty data; no `#[allow]`/unsafe added.

### Gaps Summary

No gaps. All six must-have truths verified against live code. Pass 2a (`ph_buckets` bucketing) is fully removed from non-test source; the dead `StandardSpend.puzzle_hash` field was deleted without `#[allow(dead_code)]`; Pass 1, Pass 2 SCC, dedup, and §459 is_inf suppression are intact; the IndexMap import is retained and still used. The sender (`spends.rs`) was not modified by either commit (4d384715, ef5e8c2c) and remains clean in the working tree. The 2a-named test was renamed and repurposed to detect via Pass 2's cycle; bug1/bug2 assertions and fixtures are unchanged and pass; pollution resistance (4/3) holds. Module doc and adapter doc describe the two-pass model with zero planning-tag residue.

---

_Verified: 2026-06-02_
_Verifier: Claude (gsd-verifier)_
