---
quick_id: 260602-mhw
type: execute
mode: quick-full
autonomous: true
files_modified:
  - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
  - crates/chia-sdk-test/src/silent_payments/tweak_data.rs

must_haves:
  truths:
    - "The SP block scanner groups standard removals using ONLY Pass 1 (per-spend singletons) + Pass 2 (AssertConcurrentSpend SCC over all removals); the same-puzzle-hash bucketing stage is gone."
    - "A same-puzzle-hash multi-input send that carries the SDK's cyclic opcode-64 binding is still detected — now grouped by Pass 2's SCC instead of the removed bucketing pass."
    - "A single-input send sharing a puzzle hash with an unrelated coin is still detected via its Pass 1 singleton (test_bug2 passes unchanged)."
    - "A mixed-puzzle-hash full cycle is still detected as one SCC over all removals (test_bug1 passes unchanged)."
    - "Pollution resistance holds: a third-party one-way assert stays in its own trivial SCC and never contaminates the victim group's A_sum."
    - "tweak_point counts decrease where Pass 2a previously added an overlapping same-PH aggregate that no longer has a cycle; this is the intended outcome and oracle expectations are updated downward."
  artifacts:
    - path: "crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs"
      provides: "tweak_data_from_block_spends with Pass 1 + Pass 2 grouping only; updated module doc; repurposed/recomputed tests"
      contains: "iterative_tarjan_scc"
    - path: "crates/chia-sdk-test/src/silent_payments/tweak_data.rs"
      provides: "Simulator adapter whose module doc references the Pass 1 + Pass 2 model (no same-PH bucketing)"
      contains: "tweak_data_from_block_spends"
  key_links:
    - from: "crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs"
      to: "iterative_tarjan_scc"
      via: "AssertConcurrentSpend opcode-64 directed graph over ALL standard removals"
      pattern: "AssertConcurrentSpend"
    - from: "crates/chia-sdk-test/src/silent_payments/tweak_data.rs"
      to: "tweak_data_from_block_spends"
      via: "delegation; inherits grouping change"
      pattern: "tweak_data_from_block_spends"
---

<objective>
Drop the Pass 2a same-puzzle-hash bucketing stage from the CHIP-0057 SP block
scanner (`tweak_data_from_block_spends`) so it matches the now-simplified CHIP
`ScanBlock` procedure: **Pass 1 (per-spend singletons) + Pass 2
(`AssertConcurrentSpend` SCC over all removals) only**.

Receiver-only. The sender is NOT touched — the SDK already forces
`Relation::AssertConcurrent` for any >=2 non-ephemeral XCH inputs
(`spends.rs:632` gate) and `emit_relation` (`spends.rs:392`) builds the cyclic
opcode-64 binding over them, so every multi-input send — INCLUDING same-PH ones
— carries the cycle and is detected by Pass 2. Same-PH-without-a-cycle is, by
design, no longer a detectable shape under the new CHIP.

Purpose: conformance with the updated CHIP (Pass 1 + Pass 2 only; see
`chip-silent-payments.md` ScanBlock lines 295-324 and Edge Cases 458-464).
Output: a smaller, simpler grouping function whose only multi-input mechanism is
the SCC over the concurrent-spend cycle the sender already emits.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@.planning/STATE.md

# THE FILE TO EDIT (read whole file — grouping fn, module doc, all #[cfg(test)] tests)
@crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs

# Adapter that delegates and inherits the change (doc-comment update only)
@crates/chia-sdk-test/src/silent_payments/tweak_data.rs

# Sender — DO NOT CHANGE. Confirms the cycle is always emitted for >=2 inputs.
@crates/chia-sdk-driver/src/action_system/spends.rs

# e2e oracles that MUST still pass (use detection counts, not raw tweak_point counts)
@crates/chia-sdk-driver/tests/silent_payments_e2e.rs

<facts>
Confirmed by reading the source (no codebase exploration needed by the executor):

1. SENDER (DO NOT TOUCH). `spends.rs:632`:
   `if non_ephemeral_xch_count >= 2 && !matches!(relation, Relation::AssertConcurrent) { return Err(SilentPaymentRequiresInputBinding); }`
   and `emit_relation` (`spends.rs:392-418`) emits a cyclic
   `assert_concurrent_spend` (coin 0 -> last, every other coin -> predecessor)
   for >=2 conditions-spends. => Every multi-input SP send carries the cycle.

2. CURRENT GROUPING (block_tweak_data.rs, lines 140-201) has THREE pass blocks
   pushing into one `groups: Vec<Vec<usize>>`:
   - Pass 1 (148-150): `for i in 0..len { groups.push(vec![i]); }`  -- KEEP
   - Pass 2a (152-163): `ph_buckets: IndexMap<Bytes32, Vec<usize>>` filled from
     `ss.puzzle_hash`, then each bucket of len>=2 pushed.  -- REMOVE ENTIRELY
   - Pass 2b (165-201): builds `coin_id_to_pos` + `adj`, runs
     `iterative_tarjan_scc`, pushes each scc of len>=2.  -- KEEP (this is the
     CHIP's "Pass 2"); rename internal comments "Pass 2b" -> "Pass 2".
   After grouping: Stage 3 (203-225) aggregates A_sum, computes input_hash,
   scalar_multiply, `is_inf` suppression (§454), and 48-byte byte-equality
   dedup via `seen: HashSet<[u8;48]>`.  -- KEEP unchanged.
   Stage 4 (227-236) flat OutputMeta.  -- KEEP unchanged.

3. `indexmap::IndexMap` is still needed for Pass 2's `coin_id_to_pos` map
   (line 171). It does NOT become unused after Pass 2a removal — keep the
   `use indexmap::IndexMap;` import (machete + clippy stay clean).

4. e2e oracles (silent_payments_e2e.rs) assert `detections.len()` /
   `!tweak_points.is_empty()`, NOT exact tweak_point counts. The multi-input
   e2e (164-238) sends two DISTINCT-PH coins through the SDK (emits the cycle),
   so it is detected by Pass 2. All four e2e oracles stay green.

5. The `StandardSpend.puzzle_hash` field becomes unused after Pass 2a removal
   (it was read ONLY by the bucketing loop; `dead_code = "deny"` will fail the
   build). RESOLVE by removing the `puzzle_hash` field from the `StandardSpend`
   struct AND its initializer in Stage 1 (`puzzle_hash: spend.coin.puzzle_hash`).
   Verify with LSP findReferences before deleting; if any surviving reader
   exists, keep it instead. Do NOT add `#[allow(dead_code)]`.
</facts>

<recomputed_expectations>
Model after change: N standard spends -> N Pass-1 singletons + one group per
Pass-2 SCC of size >= 2, minus byte-equal dedup, minus §454 is_inf suppression.

- test_pass_2a_round_trip_matches_simulator_helper (current 413-442): ONE
  simulator spend. Pass 1 -> 1 singleton; no cycle, no SCC>=2. Count = 1,
  UNCHANGED. It asserts equality with `tweak_data_from_simulator_block`, which
  delegates to the same fn, so it remains valid by construction. Action: RENAME
  (drop "2a" from the name, e.g. `single_input_round_trip_matches_simulator_helper`)
  + update doc to describe the Pass 1 singleton only.

- test_multi_input_round_trip (current 454-474): two same-PH spends. Fixture is
  hand-built same-PH WITHOUT a cycle (pure ex-Pass-2a). Pre-change it asserted
  3 (2 singletons + 1 Pass-2a aggregate). TASK-SCOPE choice: ADD the
  AssertConcurrent cycle to the fixture to reflect real, detectable SDK
  behaviour (spend_a asserts coin_b's id, spend_b asserts coin_a's id), so the
  same-PH multi-input round-trip still detects under Pass 2. Precompute coin ids
  via `StandardArgs::curry_tree_hash(alice_public)` + the two distinct parents,
  build the cyclic conditions, then assert. With the cycle: Pass 1 -> 2
  singletons (distinct points: same A_sum = alice_public, but different
  single-coin-id sets => different input_hash); Pass 2 -> SCC{a,b} -> 1
  aggregate (A_sum = 2*alice_public over both ids). All three byte-distinct.
  EXPECTED = 3, now produced via Pass 2 (not 2a). RENAME to
  `same_ph_multi_input_round_trip_via_concurrent_spend` and update the doc to
  explain the cycle + Pass 2 SCC.

- test_pass_2b_pollution_resistance (current 487-577): distinct PHs (a, b,
  polluter) + a<->b cycle + polluter->a one-way. NO same-PH, so Pass 2a never
  contributed. Counts UNCHANGED: polluted = 4 (3 singletons + 1 SCC{a,b}),
  clean = 3 (2 singletons + 1 SCC{a,b}). KEEP the membership/byte-invariant
  pollution assertion intact. Update the doc comment to drop the "additive
  model ... no Pass-2a groups" phrasing into the two-pass framing. Optional:
  rename "2b" -> "concurrent_spend" in the fn name/comments (no behaviour
  change) for naming consistency.

- test_bug1_mixed_ph_multi_input_full_cycle_detected (current 598-670): full
  3-coin cycle over mixed PHs. Never used Pass 2a (asserts PRESENCE of the
  3-coin SCC aggregate, which Pass 2 still produces). MUST PASS UNCHANGED. Only
  reword the historical "Pass-2a bucket / surviving/!grouped" prose in its doc
  comment to past tense or remove it; do NOT change the assertion or fixture.

- test_bug2_single_input_sharing_ph_detected_via_singleton (current 687-724):
  single-input send sharing a PH with an unrelated coin, NO cycle. Asserts the
  Pass 1 singleton is PRESENT. MUST PASS UNCHANGED. Only reword the historical
  "Pass-2a bucket" prose in its doc comment so it no longer implies a live
  Pass-2a path; the assertion is untouched.

- test_empty_block, test_non_standard_puzzle_skip, test_identity_element_guard:
  UNCHANGED.
</recomputed_expectations>

<residual_2a_check>
The residual-2a verification (used in Task 1 verify, Task 2 / `<verification>`,
`<done>`, and `<success_criteria>`) MUST be narrow and non-test-scoped so a
faithful execution yields 0. The OLD broad pattern wrongly matched legitimate
surviving lines (the kept curry assertion message
`"same synthetic_pk must curry to the same puzzle_hash"`, the new in-test
assertion message containing `"two same-PH spends…"`, and the renamed fn).

The corrected residual-2a command — IDENTICAL everywhere it appears:

```bash
cd /home/kdc/chia-wallet-sdk
# Scope each file to its NON-TEST region (everything before `#[cfg(test)]`),
# then grep ONLY for genuine dead artifacts: the removed `ph_buckets` variable
# and the literal `Pass 2a` / `Pass-2a` labels. No same-PH / same_ph /
# same.puzzle.hash alternatives — those legitimately survive in test messages,
# the renamed fn, and the kept curry assertion.
N=$( { sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs; \
       sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-test/src/silent_payments/tweak_data.rs; } \
     | grep -cE 'ph_buckets|Pass 2a|Pass-2a' )
echo "RESIDUAL_2A_NONTEST=$N (must be 0)"
```

(If a file has no `#[cfg(test)]` marker, `sed -n '1,/#\[cfg(test)\]/p'` prints
the whole file, which is correct — the entire file is non-test region.)
</residual_2a_check>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Remove Pass 2a from the grouping fn + struct field + rewrite the module doc to the Pass 1 + Pass 2 model</name>
  <files>crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs, crates/chia-sdk-test/src/silent_payments/tweak_data.rs</files>
  <behavior>
    After this task the grouping fn pushes ONLY Pass-1 singletons and Pass-2
    SCCs into `groups`. Compile-level + structural assertions (the test suite in
    Task 2 proves runtime behaviour):
    - No `ph_buckets` / same-PH bucketing code remains in the function body.
    - The `StandardSpend.puzzle_hash` field is gone (or, if LSP shows a
      surviving reader, kept with justification) — `dead_code = "deny"` clean.
    - Pass 1 still emits `groups.push(vec![i])` for every standard spend.
    - Pass 2 still builds `coin_id_to_pos` + `adj` over ALL spends, runs
      `iterative_tarjan_scc`, pushes each scc of len >= 2.
    - Stage 3 dedup (`seen: HashSet<[u8;48]>`) and §454 `is_inf` suppression
      are byte-for-byte unchanged.
    - `use indexmap::IndexMap;` stays (still used by `coin_id_to_pos`).
    - Builds with AND without `--all-features`.
  </behavior>
  <action>
    1. DELETE the Pass 2a block (current ~152-163): the `ph_buckets` IndexMap,
       its fill loop over `standard_spends`, and the
       `for (_ph, indices) in ph_buckets { if indices.len() >= 2 { groups.push(indices); } }`
       emission loop. Remove NOTHING else from the function body.

    2. Remove the now-unused `puzzle_hash: Bytes32` field from `StandardSpend`
       (struct def ~89-95) AND its initializer `puzzle_hash: spend.coin.puzzle_hash`
       in Stage 1 (~134). FIRST run LSP findReferences on the field; if a reader
       survives (it should not), keep it and note why. Do NOT use `#[allow]`.

    3. Renumber the surviving concurrent-spend pass from "Pass 2b" to "Pass 2"
       in its inline comments (~165-201). Keep the code identical: the
       `coin_id_to_pos` map, the `adj` adjacency build over all spends, the
       opcode-64 `Condition::AssertConcurrentSpend` edge extraction, the
       `iterative_tarjan_scc` call, and the `scc.len() >= 2` emission all stay.

    4. Update the `groups` accumulation comment (~140-142): a single coin may
       appear in its Pass-1 singleton AND/OR a Pass-2 SCC (drop the Pass-2a
       same-PH clause).

    5. REWRITE the module doc (`//!`, lines 1-76) to the two-pass model matching
       the new CHIP `ScanBlock`:
       - "Grouping algorithm": Stage 1 (defensive standard-puzzle filter), Pass 1
         (per-spend singletons — sole single-input detector, runs
         unconditionally), Pass 2 (AssertConcurrentSpend SCC over ALL removals;
         SCC-not-weak rationale for pollution resistance; note the sender emits
         the cyclic opcode-64 binding for >=2 inputs, so same-PH multi-input
         sends are covered here — same-PH-without-a-cycle is not a detectable
         shape by design). Keep Stage 3 (per-group aggregation + tweak emission,
         §454 is_inf suppression, 48-byte byte-equality dedup keeping first
         occurrence) and Stage 4 (flat OutputMeta). DELETE all
         "Pass 2a"/"same-puzzle-hash bucketing" prose.
       - "Group emission order": reduce to two passes — (1) Pass 1 singletons in
         `coin_spends` input order, then (2) Pass 2 SCCs in Tarjan finishing
         order. State this order is load-bearing for the byte-equality dedup and
         the simulator-helper round-trip oracle.
       - Keep the "additive and overlapping (not a partition)" framing (still
         true for Pass 1 + Pass 2: an SCC member also has its own singleton).
       - KEEP ALL PLANNING-TAG TOKENS OUT OF THE NEW PROSE. The rewritten module
         doc (and every comment you touch) must contain NONE of: `Phase N`,
         `D-N`, `Pitfall N`, `Plan NN-`, `SCn`, or the requirement prefixes
         `RECV-/SEND-/ADDR-/CRYPTO-/GUARD-/BIND-/BRIDGE-/POLISH-/CLEANUP-/SIM-`.
         Describe the algorithm in plain functional terms (Pass 1 / Pass 2 /
         Stage 3 / Stage 4, §454/§459 spec-section refs are fine; planning IDs
         are not). This keeps the SP-source planning-residue grep
         (`<verification>` #2) at 0.

    6. Do NOT touch Stage 3, Stage 4, `iterative_tarjan_scc`, imports (keep
       `IndexMap`), the public fn signature, or the `# Errors` doc.

    7. Update the chia-sdk-test adapter doc comment
       (crates/chia-sdk-test/src/silent_payments/tweak_data.rs line ~11-13):
       change "(same-puzzle-hash bucketing + `AssertConcurrentSpend` SCC)" to
       reference the Pass 1 + Pass 2 model only (e.g.
       "(per-spend Pass-1 singletons + Pass-2 `AssertConcurrentSpend` SCC)").
       No code change to that file. Same planning-tag rule applies — no planning
       IDs in the reworded comment.

    HARD constraints: no new deps; no new `#[allow]`; no `unsafe`. After
    editing, check LSP diagnostics and resolve before reporting done.
  </action>
  <verify>
    <automated>cd /home/kdc/chia-wallet-sdk; N=$( { sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs; sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-test/src/silent_payments/tweak_data.rs; } | grep -cE 'ph_buckets|Pass 2a|Pass-2a' ); echo "RESIDUAL_2A_NONTEST=$N (must be 0)"; cargo build --release -p chia-sdk-driver --all-features 2>&1 | tail -3; cargo build --release -p chia-sdk-driver 2>&1 | tail -3; cargo build --release -p chia-sdk-test --features peer-simulator 2>&1 | tail -3</automated>
  </verify>
  <done>
    `RESIDUAL_2A_NONTEST=0` (using the narrowed, non-test-scoped grep from
    `<residual_2a_check>`); both feature modes of chia-sdk-driver and
    chia-sdk-test build clean; the `StandardSpend.puzzle_hash` field is removed
    (no `dead_code` error); LSP diagnostics clean; no new `#[allow]`/`unsafe`/deps.
    The module doc and the chia-sdk-test adapter doc describe only the
    Pass 1 + Pass 2 model and carry no planning-tag tokens.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Repurpose/recompute the affected #[cfg(test)] oracles; confirm bug1/bug2 still pass</name>
  <files>crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs</files>
  <behavior>
    The in-file test module reflects the Pass 1 + Pass 2 model. Target outcomes:
    - `same_ph_multi_input_round_trip_via_concurrent_spend` (renamed from
      `test_multi_input_round_trip`): two same-PH spends bound by an a<->b
      AssertConcurrent cycle => exactly 3 tweak_points (2 Pass-1 singletons +
      1 Pass-2 SCC aggregate). Asserts `tweak_points.len() == 3`.
    - `single_input_round_trip_matches_simulator_helper` (renamed from
      `test_pass_2a_round_trip_matches_simulator_helper`): unchanged body; still
      asserts byte-equality with `tweak_data_from_simulator_block` (count = 1).
    - `test_pass_2b_pollution_resistance`: polluted == 4, clean == 3, legit SCC
      tweak_point present + byte-invariant in both (unchanged outcome; doc
      reworded to two-pass framing).
    - `test_bug1_mixed_ph_multi_input_full_cycle_detected`: PASSES unchanged
      (3-coin SCC aggregate present).
    - `test_bug2_single_input_sharing_ph_detected_via_singleton`: PASSES
      unchanged (Pass-1 singleton present).
    - `test_empty_block`, `test_non_standard_puzzle_skip`,
      `test_identity_element_guard`: PASS unchanged.
  </behavior>
  <action>
    Apply the per-test changes from `<recomputed_expectations>` above:

    1. `test_multi_input_round_trip` -> rename to
       `same_ph_multi_input_round_trip_via_concurrent_spend`. Build the two
       same-PH coins, precompute their coin ids
       (`Coin::new(parent_a, puzzle_hash, 100).coin_id()` etc., where
       `puzzle_hash = StandardArgs::curry_tree_hash(alice_public).into()`), then
       build the cyclic conditions: spend_a uses
       `Conditions::new().assert_concurrent_spend(id_b)`, spend_b uses
       `Conditions::new().assert_concurrent_spend(id_a)`. Keep the same-PH
       assertion. Assert `td.tweak_points.len() == 3` with the message
       "two same-PH spends bound by a cycle: 2 Pass-1 singletons + 1 Pass-2 SCC
       aggregate". Rewrite the doc comment to describe Pass 1 + Pass 2 (no
       Pass-2a).

    2. `test_pass_2a_round_trip_matches_simulator_helper` -> rename to
       `single_input_round_trip_matches_simulator_helper`. Body unchanged (one
       simulator spend; equality vs the helper). Update the doc comment to say
       the single spend produces one Pass-1 singleton and matches the helper.

    3. `test_pass_2b_pollution_resistance`: keep the fixture and the two count
       assertions (polluted == 4, clean == 3) and the legit-membership +
       byte-invariant assertions verbatim. Only reword the inline "Additive
       model ... no Pass-2a groups" comment to "Pass 1 emits a singleton per
       coin; Pass 2 emits the {a,b} SCC; the polluter sits in its own trivial
       SCC". Optional rename "2b" -> "concurrent_spend".

    4. `test_bug1_*` and `test_bug2_*`: do NOT change assertions or fixtures.
       Only past-tense / trim the historical "Pass-2a bucket" /
       "surviving/!grouped" prose in their doc comments so they describe the
       current two-pass model. These two MUST still pass.

    5. Leave `test_empty_block`, `test_non_standard_puzzle_skip`,
       `test_identity_element_guard` untouched.

    HARD constraints: no new deps; no new `#[allow]`; no `unsafe`. Check LSP
    diagnostics after editing.
  </action>
  <verify>
    <automated>cargo test --release -p chia-sdk-driver --all-features block_tweak_data 2>&1 | tail -25</automated>
  </verify>
  <done>
    All `block_tweak_data` unit tests pass, including
    `same_ph_multi_input_round_trip_via_concurrent_spend` (== 3),
    `single_input_round_trip_matches_simulator_helper`,
    `test_pass_2b_pollution_resistance` (4 / 3),
    `test_bug1_mixed_ph_multi_input_full_cycle_detected`, and
    `test_bug2_single_input_sharing_ph_detected_via_singleton`. No new
    `#[allow]`/`unsafe`/deps; LSP clean.
  </done>
</task>

</tasks>

<verification>
Run the full CI-mirroring gate after BOTH tasks. All must pass:

```bash
cd /home/kdc/chia-wallet-sdk

# 1. Pass 2a fully gone from NON-TEST source (narrowed, non-test-scoped grep).
#    Scope each file to everything before `#[cfg(test)]`, then grep ONLY for the
#    removed `ph_buckets` variable and the literal `Pass 2a` / `Pass-2a` labels.
#    (No same-PH / same_ph / same.puzzle.hash alternatives — they legitimately
#    survive in test messages, the renamed fn, and the kept curry assertion.)
N=$( { sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs; \
       sed -n '1,/#\[cfg(test)\]/p' crates/chia-sdk-test/src/silent_payments/tweak_data.rs; } \
     | grep -cE 'ph_buckets|Pass 2a|Pass-2a' )
echo "RESIDUAL_2A_NONTEST=$N"   # == 0

# 2. Prior-cleanup planning-residue grep over the full SP source set == 0.
SP_FILES=$(find crates -path '*silent_payments*' -name '*.rs')
grep -rnE '(Phase [0-9]|\b(RECV|SEND|ADDR|CRYPTO|GUARD|BIND|BRIDGE|POLISH|CLEANUP|SIM)-[0-9]|Pitfall [0-9]|Plan [0-9]{2}-|\bD-[0-9]|SC[0-9]\b)' $SP_FILES | wc -l   # == 0

# 3. Full CI workspace test suite (exact CI exclude set).
cargo test --release --workspace --all-features \
  --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive \
  --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm \
  --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro

# 4. Existing distinct-PH e2e oracles (unlabeled / labeled / m0 / multi-input).
cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e

# 5. Lint gates.
cargo clippy --workspace --all-features --all-targets   # only the 2 pre-existing chia-sdk-daemon warnings may remain
cargo fmt --all -- --check
cargo machete

# 6. chip-0057 compiles WITH and WITHOUT --all-features.
cargo build --release -p chia-sdk-driver
cargo build --release -p chia-sdk-driver --all-features
```
</verification>

<success_criteria>
- Pass 2a code path fully gone: no `ph_buckets` / same-PH bucketing in the
  grouping fn; `StandardSpend.puzzle_hash` removed; the narrowed non-test-scoped
  residual-2a grep (`<verification>` #1: `ph_buckets|Pass 2a|Pass-2a` over the
  pre-`#[cfg(test)]` region of both files) == 0.
- Grouping is Pass 1 (per-spend singletons) + Pass 2 (`AssertConcurrentSpend`
  SCC over all removals) only; module doc + chia-sdk-test adapter doc match and
  carry no planning-tag tokens.
- `test_bug1_*` and `test_bug2_*` pass UNCHANGED.
- The repurposed `same_ph_multi_input_round_trip_via_concurrent_spend` passes
  via Pass 2 (== 3); pollution-resistance passes (4 / 3) with the invariant
  intact; `single_input_round_trip_matches_simulator_helper` passes.
- Full CI workspace suite green; clippy clean (only the 2 pre-existing
  chia-sdk-daemon warnings allowed); fmt clean; machete clean.
- Existing silent_payments_e2e oracles pass.
- Planning-residue grep over the SP source set == 0.
- chip-0057 builds WITH and WITHOUT --all-features. No new `#[allow]`/`unsafe`/deps.
</success_criteria>

<output>
After completion, create
`.planning/quick/260602-mhw-drop-pass-2a-same-ph-bucketing-from-sp-b/260602-mhw-SUMMARY.md`.
</output>
