---
phase: quick-260602-ejk
verified: 2026-06-02T00:00:00Z
status: passed
score: 8/8 must-haves verified
---

# Quick 260602-ejk: SP Block-Scanner Additive Grouping + Conformance Doc Verification Report

**Task Goal:** Fix the CHIP-0057 SP block-scanner grouping to match the CHIP's additive ScanBlock model (Pass-1 singleton for every standard spend + Pass 2a kept + Pass 2b SCC over ALL removals, overlapping/deduped, §459 identity suppression preserved), purely receiver-side, plus a self-contained findings/issue doc for the SP team.
**Verified:** 2026-06-02
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | Pass 2b SCC + coin_id_to_pos built over ALL standard removals (no surviving/!grouped subset; old exclusion gone from executable code) | ✓ VERIFIED | `block_tweak_data.rs:169-201`: `coin_id_to_pos` and `adj` built over `standard_spends.iter().enumerate()` (every index). Grep for `surviving|grouped|is_grouped` in executable code returns nothing (exit 1). |
| 2 | Pass 1 singleton emitted for EVERY standard spend (`if !is_grouped` gate gone) | ✓ VERIFIED | `block_tweak_data.rs:148-150`: `for i in 0..standard_spends.len() { groups.push(vec![i]); }` — unconditional. BUG-2 test passes. |
| 3 | Pass 2a KEPT — same-PH groups size>=2 emitted as additional overlapping candidates | ✓ VERIFIED | `block_tweak_data.rs:155-163`: IndexMap bucketing, `if indices.len() >= 2 { groups.push(indices); }`. Not dropped. |
| 4 | Additive/overlapping + dedup; §459 is_inf suppression preserved | ✓ VERIFIED | `block_tweak_data.rs:208-225`: groups accumulate from all 3 passes; `if tweak_point.is_inf() { continue; }` (§459); dedup via `HashSet<[u8;48]>` on `to_bytes()` keeping first. `test_identity_element_guard` passes. |
| 5 | Sender untouched (emit_relation / AssertConcurrent cycle / sp_finish_branch unchanged); no new deps/#[allow]/unsafe | ✓ VERIFIED | `git show 8e17b1f8 --name-only` = block_tweak_data.rs ONLY (272+/74-). spends.rs/relation.rs/send_destination.rs in NEITHER commit. Sender markers present (`emit_relation` spends.rs:392, `sp_finish_branch` spends.rs:614). No `#[allow]`/`unsafe` in file (grep exit 1). No Cargo.toml in diff. |
| 6 | Regression tests genuine (BUG-1 fails on pre-fix partition; BUG-2 via Pass-1 singleton) | ✓ VERIFIED | See genuineness analysis below. Both tests pass; both assert hand-computed points absent under the old partition. |
| 7 | Oracle tests updated in membership/byte-invariant way (not deletion); pollution resistance holds | ✓ VERIFIED | `test_multi_input_round_trip` 1→3 with rationale (`block_tweak_data.rs:469-473`); `test_pass_2b_pollution_resistance` 2/1→4/3 using `.iter().any(|p| p.to_bytes() == legit)` membership + byte-invariance across polluted/clean runs (lines 554-576). No assertions deleted. |
| 8 | Findings doc at repo root, self-contained, with BUG-1/BUG-2 worked example + citations, single-input fact, balanced drop-2a (marked SEPARATE), CHIP-history (dd0e683, f19f75d), two-concern recommendation | ✓ VERIFIED | `ISSUE-sp-block-scanner-grouping-conformance.md` (158 lines). See doc-content analysis below. |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `ISSUE-sp-block-scanner-grouping-conformance.md` | >= 80 lines, SP-team handoff | ✓ VERIFIED | 158 lines; commit 0fef228e; self-contained. |
| `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` | Additive grouping + dedup + emission-order doc + BUG-1/BUG-2 tests; contains `fn tweak_data_from_block_spends` | ✓ VERIFIED | Present at line 112; additive 3-pass union; module doc lines 11-77 documents overlapping union + deterministic emission order. |
| `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` | Optional BUG-1 e2e mirror (may live inline) | ✓ VERIFIED | Per plan's "may live here or inline" clause; BUG-1 lives inline in block_tweak_data.rs. 4 pre-existing e2e tests still present. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| Pass 1 | singleton per standard spend | unconditional `for i in 0..len` | ✓ WIRED | block_tweak_data.rs:148-150 |
| Pass 2b | SCC over ALL removals | coin_id_to_pos + adj over all indices, iterative_tarjan_scc | ✓ WIRED | block_tweak_data.rs:171-200 |
| tweak_data_from_simulator_block | tweak_data_from_block_spends | thin delegating adapter | ✓ WIRED | chia-sdk-test/.../tweak_data.rs:38 calls driver fn directly — inherits fix |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| All block_tweak_data unit tests incl. BUG-1/BUG-2 | `cargo test -p chia-sdk-driver --all-features -- silent_payments::block_tweak_data::tests` | 8 passed; 0 failed | ✓ PASS |
| Residue grep over SP files + doc | grep GSD-vocab pattern | 0 | ✓ PASS |
| Doc citations present | grep dd0e683 / f19f75d / line ranges | 1 / 1 / {295-332, 319-330, 466-474} | ✓ PASS |

### Regression-Test Genuineness Analysis

**BUG-1** (`test_bug1_mixed_ph_multi_input_full_cycle_detected`, lines 597-670): 2 coins curried over `pk_dup` (shared PH_x) + 1 coin at `pk_solo` (PH_y), bound by a cyclic AssertConcurrent (dup1→solo, dup2→dup1, solo→dup2). On the pre-fix partition, Pass 2a buckets `{dup1, dup2}` as size-2 and marks them grouped; Pass 2b's graph excludes grouped coins, so solo's and dup2's edges point at non-nodes — no 3-coin SCC forms, and the hand-computed `A_sum = pk_dup + pk_dup + pk_solo` over all three coin_ids is never produced. Test asserts presence of exactly that aggregate point → genuinely FAILS pre-fix, PASSES post-fix. Genuine.

**BUG-2** (`test_bug2_single_input_sharing_ph_detected_via_singleton`, lines 686-724): 2 coins share `pk_send` (same PH), no AssertConcurrent. Pre-fix both land in the size-2 Pass-2a bucket → `grouped=true` → singleton emitted only `if !is_grouped`, so the single-coin point (`A_sum = pk_send` over one coin_id) is never produced (only the aggregated 2-coin point). Test asserts the single-input point is present → genuinely FAILS pre-fix, PASSES post-fix via Pass-1 singleton. Genuine.

### Doc Content Analysis (must_have #8)

`ISSUE-sp-block-scanner-grouping-conformance.md` contains all required elements:
- Component/Version header block (SDK git short-hash 39da4549).
- BUG-1 worked 3-coin/2-PH example traced through CHIP ScanBlock (detects) and pre-fix SDK partition (fragments), with chip-0057.md:295-332/319-330/466-474 and block_tweak_data.rs file:line citations for the `surviving`/`!grouped` exclusion and the singletons-for-ungrouped loop.
- BUG-2 single-input-sharing-a-PH fact, with the Pass-1 mandatory / single-input structural argument (spends.rs:401, spends.rs:632).
- Balanced drop-Pass-2a proposal explicitly marked SEPARATE and "not what the SDK fix does," with efficiency correction and fingerprint dimension.
- CHIP-history corroboration citing dd0e683 and f19f75d (empty-message announcement fingerprint → opcode-64 SCC).
- Two-concern recommendation: (1) SDK bug fixed now, no CHIP change; (2) drop-2a a separate spec proposal.
- No GSD planning vocabulary (residue grep = 0).

### Anti-Patterns Found

None. No `#[allow]`, no `unsafe`, no TODO/FIXME/placeholder in block_tweak_data.rs. No Cargo.toml/dependency changes.

### Human Verification Required

None. All truths verified programmatically. Orchestrator already confirmed full CI (2441 passed, 0 failed), clippy (clean except 2 pre-existing chia-sdk-daemon warnings), fmt, machete. Re-confirmed the 8 block_tweak_data unit tests pass locally.

### Gaps Summary

No gaps. All 8 must-haves verified against live code and git history. The fix is purely receiver-side (commit 8e17b1f8 touched only block_tweak_data.rs), the three passes are additive/overlapping with byte-equality dedup and §459 suppression, both regression tests are genuine and would fail on the pre-fix partition, oracle tests were updated membership/byte-invariantly, and the findings doc is complete and self-contained.

---

_Verified: 2026-06-02_
_Verifier: Claude (gsd-verifier)_
