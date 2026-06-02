---
quick_id: 260602-mhw
type: execute
mode: quick-full
subsystem: silent-payments (receive-side block scanner)
tags: [chip-0057, silent-payments, receiver, scan-block, refactor]
files_modified:
  - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
  - crates/chia-sdk-test/src/silent_payments/tweak_data.rs
key-files:
  modified:
    - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
    - crates/chia-sdk-test/src/silent_payments/tweak_data.rs
commits:
  - 4d384715  # refactor: drop Pass 2a + dead field + module doc
  - ef5e8c2c  # test: repurpose/recompute oracles
metrics:
  tasks: 2
  files: 2
  completed: 2026-06-02
---

# Quick Task 260602-mhw: Drop Pass 2a same-PH bucketing from SP block scanner Summary

Removed the same-puzzle-hash bucketing stage from the CHIP-0057 SP block scanner
(`tweak_data_from_block_spends`) so grouping is now **Pass 1 (per-spend
singletons) + Pass 2 (`AssertConcurrentSpend` SCC over all removals) only**,
matching the simplified CHIP `ScanBlock` procedure. Same-PH multi-input sends
remain detectable because the sender already forces an opcode-64 cycle over any
two or more non-ephemeral inputs, so Pass 2's SCC covers them.

## What changed

### Task 1 — grouping fn + dead field + module doc (commit `4d384715`)
- Deleted the Pass 2a block: the `ph_buckets: IndexMap<Bytes32, Vec<usize>>`
  fill loop and its `if indices.len() >= 2 { groups.push(...) }` emission loop.
- Removed the now-dead `StandardSpend.puzzle_hash: Bytes32` field and its Stage 1
  initializer `puzzle_hash: spend.coin.puzzle_hash` (the bucketing loop was its
  only reader; `dead_code = "deny"` forbids leaving it). No `#[allow]` added.
- Renumbered the surviving concurrent-spend pass from "Pass 2b" to "Pass 2" in
  inline comments; code body (the `coin_id_to_pos` map, `adj` build over all
  spends, opcode-64 edge extraction, `iterative_tarjan_scc`, `scc.len() >= 2`
  emission) is byte-for-byte unchanged.
- Rewrote the module doc to the two-pass model. Stage 3 dedup
  (`seen: HashSet<[u8;48]>`) + §459 `is_inf` suppression and Stage 4 flat
  `OutputMeta` are unchanged. `use indexmap::IndexMap;` kept (still used by
  `coin_id_to_pos` — confirmed by clean build, which would fail on an unused
  import).
- Updated the chia-sdk-test adapter module doc to reference the Pass 1 + Pass 2
  model instead of "same-puzzle-hash bucketing".

### Task 2 — oracle tests (commit `ef5e8c2c`)
- `test_multi_input_round_trip` → `same_ph_multi_input_round_trip_via_concurrent_spend`:
  added an `a <-> b` `AssertConcurrent` cycle (precompute coin ids via
  `StandardArgs::curry_tree_hash(alice_public)` + the two distinct parents) so
  the same-PH pair is now detected via Pass 2 instead of the removed Pass 2a.
  Still asserts `tweak_points.len() == 3` (2 Pass-1 singletons + 1 Pass-2 SCC
  aggregate).
- `test_pass_2a_round_trip_matches_simulator_helper` →
  `single_input_round_trip_matches_simulator_helper`: body unchanged; doc now
  describes the lone Pass-1 singleton matching the simulator helper.
- `test_pass_2b_pollution_resistance` →
  `test_concurrent_spend_pollution_resistance`: fixture + both count assertions
  (polluted == 4, clean == 3) + membership/byte-invariant assertions unchanged;
  only the inline "Additive model … no Pass-2a groups" comment was reworded to
  two-pass framing.
- `test_bug1_*` and `test_bug2_*`: assertions and fixtures untouched; only the
  historical "Pass-2a bucket / surviving / !grouped" prose in their doc comments
  was reworded to the current two-pass model. Both pass unchanged.
- `test_empty_block`, `test_non_standard_puzzle_skip`,
  `test_identity_element_guard`: untouched.

## Verification (all gates green)

- `RESIDUAL_2A_NONTEST` (narrowed `ph_buckets|Pass 2a|Pass-2a` over the
  pre-`#[cfg(test)]` region of both files) = **0**.
- SP-source planning-residue grep = **0**.
- All 8 `block_tweak_data` unit tests pass (including the repurposed
  `same_ph_multi_input_round_trip_via_concurrent_spend` == 3, the renamed
  pollution test 4/3, bug1, bug2, single-input round-trip).
- 4 `silent_payments_e2e` oracles pass.
- Full CI workspace suite (`cargo test --release --workspace --all-features`
  minus the binding crates) green, no failures.
- `cargo clippy --workspace --all-features --all-targets`: only the 2
  pre-existing chia-sdk-daemon warnings remain.
- `cargo fmt --all -- --check` clean; `cargo machete` clean.
- chip-0057 builds WITH and WITHOUT `--all-features`.

## Deviations from Plan

None — plan executed exactly as written. No new deps, no new `#[allow]`, no
`unsafe`. The sender (`spends.rs`) was not touched.

## Self-Check: PASSED
- FOUND: crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
- FOUND: crates/chia-sdk-test/src/silent_payments/tweak_data.rs
- FOUND: commit 4d384715
- FOUND: commit ef5e8c2c
