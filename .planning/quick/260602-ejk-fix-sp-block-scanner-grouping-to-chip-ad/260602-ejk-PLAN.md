---
phase: quick-260602-ejk
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ISSUE-sp-block-scanner-grouping-conformance.md
  - crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs
  - crates/chia-sdk-driver/tests/silent_payments_e2e.rs
autonomous: true
requirements: [SP-SCAN-CONFORMANCE-01]
quick_full: true

must_haves:
  truths:
    - "A 3-coin / 2-puzzle-hash mixed multi-input SP send (2 inputs at PH_x, 1 at PH_y) bound by Relation::AssertConcurrent yields exactly one DetectedSpCoin via the Pass-2b SCC built over ALL standard removals (not just the ungrouped subset)."
    - "A single-input SP send whose input shares a puzzle hash with an unrelated standard coin in the same block is still detected, via the Pass-1 singleton tweak_point now emitted for EVERY standard spend."
    - "Pass 2a is retained (matches the CURRENT CHIP): same-PH groups of size >= 2 remain as additional overlapping candidate groups."
    - "tweak_points are emitted additively across all three passes with a documented deterministic emission order; identical resulting tweak_points are deduplicated; the CHIP §459 identity-element suppression is preserved."
    - "The sender side (emit_relation, the AssertConcurrent cycle, sp_finish_branch) is unchanged — the fix is purely receiver-side."
    - "A self-contained findings/issue markdown doc exists at repo root describing BUG-1 + BUG-2, the single-input fact, the balanced drop-2a proposal, the CHIP-history corroboration, and the two-concern recommendation, with exact CHIP + file:line citations."
    - "Full CI workspace suite, workspace clippy, fmt, machete, and the planning-residue grep all pass; existing distinct-PH e2e oracles still pass."
  artifacts:
    - path: "ISSUE-sp-block-scanner-grouping-conformance.md"
      provides: "External SP-team handoff doc (BUG-1/BUG-2 + drop-2a proposal + history + recommendation)"
      min_lines: 80
    - path: "crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs"
      provides: "Additive ScanBlock grouping (Pass1 singleton-for-all + Pass2a kept + Pass2b over all removals), dedup, updated module-doc emission-order section, BUG-1/BUG-2 regression tests"
      contains: "fn tweak_data_from_block_spends"
    - path: "crates/chia-sdk-driver/tests/silent_payments_e2e.rs"
      provides: "Optional e2e mirror of BUG-1 mixed-PH multi-input detection (may live here or inline in block_tweak_data.rs)"
      contains: "fn test_simulator_e2e"
  key_links:
    - from: "block_tweak_data.rs Pass 1"
      to: "tweak_points (singleton per standard spend)"
      via: "emit one tweak_point for EVERY StandardSpend, not only ungrouped"
      pattern: "singleton|Pass 1|for .* standard_spends"
    - from: "block_tweak_data.rs Pass 2b"
      to: "iterative_tarjan_scc over ALL standard removals"
      via: "remove the surviving/!grouped exclusion + coin_id_to_pos over all spends"
      pattern: "coin_id_to_pos|adj|iterative_tarjan_scc"
    - from: "tweak_data_from_simulator_block (chia-sdk-test)"
      to: "tweak_data_from_block_spends (driver)"
      via: "thin delegating adapter inherits the fix"
      pattern: "tweak_data_from_block_spends"
---

<objective>
Fix the CHIP-0057 silent-payment block-scanner grouping in the SDK so it matches the CHIP's
**additive** ScanBlock model (Pass 1 singletons + Pass 2a same-PH groups + Pass 2b
AssertConcurrent SCC, all OVERLAPPING and additive — not a mutually-exclusive partition), and
write a self-contained findings/issue doc for the external SP/CHIP team.

The pre-fix SDK treats the three passes as a *partition*: a coin grouped by Pass 2a (same-PH,
size >= 2) is excluded from Pass 2b's SCC graph (the `surviving`/`!grouped` filter and the
`coin_id_to_pos` map built only over `surviving`), and singletons are emitted *only* for
coins that fell through all groups. This produces two conformance bugs versus the CHIP:

- **BUG-1 (mixed-PH fragmentation):** A multi-input send where the inputs span >= 2 distinct
  puzzle hashes (e.g. 2 coins at PH_x + 1 coin at PH_y, all bound by one AssertConcurrent
  cycle) gets the PH_x pair captured by Pass 2a and then EXCLUDED from the Pass 2b SCC. The
  full cycle's A_sum is never aggregated, so the recipient's one-time puzzle hash (computed by
  the sender over ALL inputs) is never reconstructed — the payment is missed.
- **BUG-2 (single-input sharing a PH):** A single-input SP send whose lone input happens to
  share a puzzle hash with an unrelated standard coin in the same block lands in a Pass 2a
  bucket of size >= 2, gets `grouped[i] = true`, and is therefore NOT emitted as a Pass-1
  singleton — so the single-input send is missed.

The CHIP runs all three passes independently and unions the detected coins; the SDK must do
the same (emit a tweak_point per candidate group across all three passes, deduplicate
identical results).

Purpose: Make the SDK receive-side conform to the current published CHIP-0057 ScanBlock so
wallets relying on the SDK's `tweak_data_from_block_spends` detect every payment the CHIP says
they should; and hand the external SP team a precise, balanced writeup of the bugs plus a
separate (optional, for their consideration) spec-simplification proposal.

Output:
1. `ISSUE-sp-block-scanner-grouping-conformance.md` (repo root) — the handoff doc.
2. Rewritten additive grouping in `block_tweak_data.rs` + BUG-1/BUG-2 regression tests.

Two atomic, independently-committable tasks. NO sender changes.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

# The conformance target — read ScanBlock 295-332 + Edge Cases 466-474.
@/home/kdc/chips/CHIPs/chip-0057.md

# The file to fix (read the whole grouping fn + module doc + tests).
@crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs

# Sender side — DO NOT CHANGE. Understand the binding the receiver reconstructs.
@crates/chia-sdk-driver/src/action_system/spends.rs

# Existing e2e tests (distinct-PH multi-input test to mirror); regression tests may land here.
@crates/chia-sdk-driver/tests/silent_payments_e2e.rs

# Simulator adapter — delegates to the driver fn, inherits the fix.
@crates/chia-sdk-test/src/silent_payments/tweak_data.rs

# Precedent for the handoff doc format/voice (external GitHub-issue audience).
@ISSUE-silent-payment-synthetic-key-guard.md

<interfaces>
<!-- Key contracts the executor needs. Extracted from the codebase; no exploration required. -->

block_tweak_data.rs current shape (crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs):
```rust
struct StandardSpend {
    coin_id: Bytes32,
    synthetic_pk: PublicKey,
    puzzle_hash: Bytes32,
    puzzle: NodePtr,
    solution: NodePtr,
}

pub fn tweak_data_from_block_spends(
    coin_spends: &[CoinSpend],
    additions: &[Coin],
) -> Result<TweakData, DriverError>;

fn iterative_tarjan_scc(adj: &[Vec<usize>]) -> Vec<Vec<usize>>; // returns SCCs in finishing order
```

Pre-fix flow (the bug): Stage 2a marks `grouped[i]=true` for same-PH buckets of size>=2;
Stage 2b builds `surviving = (0..n).filter(|i| !grouped[i])` and a `coin_id_to_pos` map ONLY
over `surviving`, so AssertConcurrent edges into a 2a-grouped coin DROP (BUG-1); standalone
singletons are pushed ONLY `if !is_grouped` (BUG-2).

Aggregation+emit (Stage 3, reuse as-is per group):
```rust
let coin_ids: Vec<Bytes32> = group.iter().map(|&i| standard_spends[i].coin_id).collect();
let mut a_sum = standard_spends[group[0]].synthetic_pk;
for &i in &group[1..] { a_sum += &standard_spends[i].synthetic_pk; }
let input_hash = compute_input_hash(&coin_ids, &a_sum);
let mut tweak_point = a_sum;
tweak_point.scalar_multiply(&input_hash.to_bytes());
if !tweak_point.is_inf() { tweak_points.push(tweak_point); }  // CHIP §459 guard
```

Sender binding (spends.rs — DO NOT CHANGE, understand only):
- emit_relation (spends.rs:392): for Relation::AssertConcurrent, early-returns when
  coin_ids.len() <= 1 (line 401); otherwise emits a CYCLE — coin i asserts coin i-1, coin 0
  asserts coin N-1.
- sp_finish_branch (spends.rs:614): GATE at line 632 requires Relation::AssertConcurrent when
  non_ephemeral_xch_count >= 2 (so every multi-input SP send IS bound by the cycle).

Test helper in block_tweak_data.rs tests (reuse for regression tests):
```rust
fn build_standard_coin_spend(
    synthetic_pk: PublicKey,
    parent_coin_info: Bytes32,
    amount: u64,
    conditions: Conditions,  // e.g. Conditions::new().assert_concurrent_spend(other_coin_id)
) -> CoinSpend;
```
Existing tests in this file: test_empty_block, test_non_standard_puzzle_skip,
test_identity_element_guard, test_pass_2a_round_trip_matches_simulator_helper,
test_multi_input_round_trip (asserts len==1 for 2 same-PH spends),
test_pass_2b_pollution_resistance (asserts polluted.len()==2, clean.len()==1, index-0
invariance). These pinned-count tests change under the additive model — UPDATE them.
</interfaces>

<reference-commits>
<!-- ~/silent-payments working copy — verified to exist. Cite verbatim in the doc. -->
- dd0e683 "fix: avoid empty-message announcement fingerprint in multi-input SP"
- f19f75d "feat(02-01): rewrite CHIP Pass 2b around SCC over opcode-64 edges"
</reference-commits>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write the SP-team findings/issue doc (no code)</name>
  <files>ISSUE-sp-block-scanner-grouping-conformance.md</files>
  <action>
Create a self-contained markdown handoff at repo-root
`ISSUE-sp-block-scanner-grouping-conformance.md`, matching the voice/structure of the existing
`ISSUE-silent-payment-synthetic-key-guard.md` (external GitHub-issue audience; the user will
paste it into an issue on Chia-Network/chips). DO NOT post anything to GitHub — only write the
file. It must be self-contained (a reader with no SDK access understands the bug and proposal).

Open with a `**Component:**` + `**Version:**` header block like the precedent doc (use the
current SDK git short-hash; run `git rev-parse --short HEAD` and `git rev-parse HEAD` to fill
it).

Contents, with EXACT citations:

1. **The SDK conformance bug.** State BUG-1 (mixed-PH fragmentation) and BUG-2
   (single-input-sharing-a-PH). Trace the worked **3-coin / 2-PH** example (2 inputs at PH_x,
   1 at PH_y, bound by one AssertConcurrent cycle) through BOTH:
   - the CHIP ScanBlock (which DETECTS it: Pass 2b builds the opcode-64 SCC over ALL removals
     and the cycle is one SCC), and
   - the pre-fix SDK partition (which FRAGMENTS it: Pass 2a captures the PH_x pair and the
     `surviving`/`!grouped` exclusion drops those coins from Pass 2b's SCC graph, so the full
     3-coin A_sum is never aggregated).
   Cite `chip-0057.md`: ScanBlock procedure lines 295-332, Pass 2b iterating ALL removals
   lines 319-330, Edge Cases / Scanner Grouping Strategies lines 466-474. Cite
   `block_tweak_data.rs` by file:line: the `surviving`/`!grouped` exclusion (lines ~128-138),
   the `coin_id_to_pos` map built only over `surviving`, and the singletons-only-for-ungrouped
   loop (lines ~174-179). State clearly that this is **now FIXED in the SDK** (reference Task 2
   / the additive rewrite) — the doc reports a fixed bug, it is not asking the SP team to fix
   SDK code.

2. **Single-input fact.** Pass 2b (SCC, size >= 2) STRUCTURALLY cannot detect single-input
   sends; Pass 1 (singletons) is therefore mandatory in any conformant scanner. SDK evidence:
   sender `emit_relation` early-returns at <= 1 coin (`spends.rs:401`); the SP gate requires
   AssertConcurrent only for >= 2 non-ephemeral XCH inputs (`spends.rs:632`) — a single-input
   SP send emits NO cycle at all, so only Pass 1 can catch it. (This is the why behind BUG-2.)

3. **The CHIP-simplification PROPOSAL — drop Pass 2a, keep Pass 1 + Pass 2b — presented
   BALANCED** (this is a SEPARATE, optional ask for the SP team; explicitly NOT what the SDK
   fix does — the SDK keeps 2a to match the current CHIP):
   - Requires flipping the sender spec from "SHOULD prefer same-PH" (chip-0057.md:474) to
     "MUST emit the concurrent-spend cycle for every multi-input send." Note the SDK ALREADY
     enforces this (`spends.rs:632`).
   - Efficiency correction: Pass 2a is the CHEAP path (bucket by `coin.puzzle_hash` + uncurry
     the synthetic PK from the reveal — no CLVM). Pass 2b REQUIRES running each spend's
     puzzle+solution through CLVM (`run_puzzle`) for opcode-64. BUT a COMPLETE scanner must run
     Pass 2b over all removals anyway, so it already executes every standard puzzle — meaning
     2a saves nothing for a complete scanner and only adds the fragmentation bug. 2a's apparent
     CPU win exists ONLY in the buggy skip-2b-for-2a-coins shortcut. The only reason to keep 2a
     is supporting a deliberately INCOMPLETE cheap scanner (Pass 1 + 2a, no CLVM, misses
     cross-index sends).
   - Privacy/fingerprint dimension: mandating the cycle is negligible byte cost but interacts
     with indistinguishability (the FINGERPRINT concern) — flag that it needs the SP team's
     read on AssertConcurrent base-rate in normal traffic.

4. **CHIP history corroboration.** An earlier revision bound inputs via empty-message coin
   announcements (opcode 60/61), which created an SP-specific fingerprint and was replaced by
   the opcode-64 AssertConcurrent SCC. Cite the `~/silent-payments` working-copy commits
   **dd0e683** ("fix: avoid empty-message announcement fingerprint in multi-input SP") and
   **f19f75d** ("feat: rewrite CHIP Pass 2b around SCC over opcode-64 edges").

5. **Recommendation separating the two concerns:**
   (i) SDK bug FIXED now to match the CURRENT CHIP (additive ScanBlock, no CHIP change needed);
   (ii) drop-2a is a separate spec-evolution proposal for the SP team to weigh.

Do NOT reference internal GSD planning vocabulary (no Phase/Plan/requirement-ID/SC strings) —
the doc must satisfy the planning-residue grep in the verification gate.
  </action>
  <verify>
    <automated>test -f ISSUE-sp-block-scanner-grouping-conformance.md && wc -l ISSUE-sp-block-scanner-grouping-conformance.md | awk '$1>=80{exit 0}{exit 1}' && grep -q 'dd0e683' ISSUE-sp-block-scanner-grouping-conformance.md && grep -q 'f19f75d' ISSUE-sp-block-scanner-grouping-conformance.md && grep -qE 'BUG-1' ISSUE-sp-block-scanner-grouping-conformance.md && grep -qE 'BUG-2' ISSUE-sp-block-scanner-grouping-conformance.md && grep -qE '295|319|466' ISSUE-sp-block-scanner-grouping-conformance.md && echo DOC_OK</automated>
  </verify>
  <done>
ISSUE-sp-block-scanner-grouping-conformance.md exists at repo root, >= 80 lines, self-contained,
cites both reference commits (dd0e683, f19f75d), both bugs (BUG-1/BUG-2), CHIP line ranges
(ScanBlock 295-332, Pass 2b 319-330, Edge Cases 466-474) and block_tweak_data.rs file:line for
the exclusion + singletons-for-ungrouped loop, presents the drop-2a proposal balanced, and
separates the two recommendations. Contains no GSD planning vocabulary. Committed independently.
  </done>
</task>

<task type="auto">
  <name>Task 2: Rewrite block_tweak_data.rs to the additive model + BUG-1/BUG-2 regression tests</name>
  <files>crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs, crates/chia-sdk-driver/tests/silent_payments_e2e.rs</files>
  <action>
Edit `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` so the three passes are
ADDITIVE and OVERLAPPING, matching the current CHIP-0057 ScanBlock (chip-0057.md:295-332).
KEEP Pass 2a — do NOT drop it (dropping 2a is the upstream proposal in Task 1's doc, NOT this
fix). The fix is purely receiver-side: DO NOT touch the sender (`emit_relation`, the
AssertConcurrent cycle, `sp_finish_branch` in `action_system/spends.rs`).

Required changes inside `tweak_data_from_block_spends`:

- **Pass 1 (fixes BUG-2):** Emit a singleton candidate group `vec![i]` for EVERY standard
  spend `i` (0..standard_spends.len()) — NOT only the ungrouped ones. Remove the
  `if !is_grouped` gate on the standalone loop (current lines ~174-179). Single-input sends and
  every individual input now always get their own Pass-1 tweak_point.

- **Pass 2a (KEEP):** Same-PH buckets of size >= 2 remain as ADDITIONAL candidate groups. The
  `grouped[]` bookkeeping is no longer used to EXCLUDE coins from later passes — these groups
  are now purely additive overlapping candidates. (You may drop the `grouped` Vec entirely if
  it is no longer read; if you keep it for any reason it must NOT gate Pass 1 or Pass 2b.)

- **Pass 2b (fixes BUG-1):** Build the opcode-64 AssertConcurrentSpend SCC graph over ALL
  standard removals. Remove the `surviving = (0..n).filter(|i| !grouped[i])` restriction
  (current lines ~129-131) and the `coin_id_to_pos` restriction to `surviving` (current lines
  ~134-138): the adjacency graph and `coin_id_to_pos` must cover every StandardSpend index, so
  an AssertConcurrent edge pointing at a coin that ALSO happens to be in a Pass-2a bucket still
  forms its SCC. Emit each SCC of size >= 2 as an additional candidate group. Preserve the
  iterative Tarjan SCC (`iterative_tarjan_scc`) and the SCC-not-WCC pollution resistance
  (a polluter with a one-way edge stays in its own trivial SCC).

- **Emit per candidate group across all three passes (Stage 3, unchanged math).** For each
  group compute `A_sum`, `input_hash = compute_input_hash(coin_ids, A_sum)`,
  `tweak_point = A_sum.scalar_multiply(input_hash)`. PRESERVE the CHIP §459 identity-element
  (point-at-infinity) suppression (`if !tweak_point.is_inf()`).

- **Deduplicate identical resulting tweak_points.** Groups now overlap (a coin appears in its
  Pass-1 singleton, possibly its Pass-2a PH-group, and possibly its Pass-2b SCC), so the same
  byte-identical `tweak_point` can be produced more than once (e.g. a 2a same-PH pair and a 2b
  SCC over the same exact coin set produce the same A_sum + same coin_ids). Dedup by the
  48-byte compressed point (`tweak_point.to_bytes()`); keep FIRST occurrence to preserve
  emission order. Distinct groups that share a coin but differ in membership produce DIFFERENT
  tweak_points (different A_sum / coin_id set) and MUST both survive — dedup is byte-equality
  only, never by coin overlap.

- **Define + document a DETERMINISTIC emission order.** Update the module-doc "Group emission
  order (load-bearing for byte-equality tests)" section (currently lines ~39-47) to reflect the
  additive three-pass union. Pick and document a stable total order, e.g.:
  1. Pass 1 singletons in `coin_spends` input order,
  2. Pass 2a same-PH groups in puzzle-hash insertion order,
  3. Pass 2b SCCs (size >= 2) in Tarjan finishing order,
  then dedup keeping first occurrence. The exact order is your choice but it MUST be
  deterministic and MUST match what the updated tests assert.

- **Update the module-level "Grouping algorithm" doc** (lines ~11-37) so it describes the
  additive/overlapping union model instead of the partition model (Stage 2b no longer "over
  ungrouped"; Pass 1 now "every standard spend"). Keep it as plain API prose — no GSD planning
  vocabulary (the residue grep covers this file).

**Update the existing pinned/oracle tests in this file** that change under the additive model
(increasing tweak_point count is EXPECTED — update oracles, do NOT try to preserve old counts):
- `test_multi_input_round_trip` (2 same-PH spends): pre-fix asserted `len == 1`. Under the
  additive model it now emits 2 Pass-1 singletons + 1 Pass-2a group = 3 candidates (assuming
  all distinct A_sums). Recompute the expected count from the additive+dedup rules and update
  the assertion + its doc comment.
- `test_pass_2b_pollution_resistance` (a<->b cycle + polluter asserting a): pre-fix asserted
  `polluted.len() == 2`, `clean.len() == 1`, and index-0 invariance. Update counts for the
  additive model (each coin now also yields a Pass-1 singleton). PRESERVE the load-bearing
  invariant: the legit `{a,b}` SCC tweak_point must still be byte-equal between polluted and
  clean runs (polluter's key must NOT leak into A_sum) — re-derive which index it now lands at,
  or assert membership (`.iter().any(|p| p.to_bytes() == legit)`) rather than a fixed index.
- `test_pass_2a_round_trip_matches_simulator_helper` and `test_identity_element_guard` and
  `test_empty_block` / `test_non_standard_puzzle_skip`: re-check each against the new model and
  update only what changed (the single-spend round-trip helper comparison still holds because
  both sides run the same fn; identity/empty/non-standard still emit nothing).

**Regression tests (NEW)** — add inline in `block_tweak_data.rs` `#[cfg(test)] mod tests` using
the existing `build_standard_coin_spend` helper (and optionally mirror BUG-1 as a full e2e in
`tests/silent_payments_e2e.rs` following the `test_simulator_e2e_multi_input` structure):
- **BUG-1 (mixed-PH multi-input):** 3 coins — 2 distinct synthetic keys at PH_x (two coins
  curried over the SAME synthetic_pk so they share a puzzle hash) + 1 coin at PH_y — all bound
  by ONE AssertConcurrent cycle (each coin asserts the previous, coin 0 asserts coin 2, exactly
  as `emit_relation` builds it). Assert that the Pass-2b SCC over ALL THREE coins produces a
  candidate group of all three, and therefore a tweak_point whose A_sum is the sum of all three
  synthetic keys exists in the output. The crisp acceptance: the 3-coin-aggregate tweak_point
  equals the tweak_point computed by hand over all three keys + all three coin_ids, and is
  PRESENT in `td.tweak_points`. Add a comment asserting this test FAILS before the fix (the
  PH_x pair would be captured by 2a and excluded from the SCC, so the 3-coin A_sum would never
  appear) and PASSES after.
- **BUG-2 (single-input sharing a PH):** 2 coins — one is a single-input SP send's input
  (synthetic key K_send), the other is an UNRELATED standard coin curried over the SAME
  synthetic key (so identical puzzle_hash), with NO AssertConcurrent binding between them.
  Assert the SP input is still detected via its Pass-1 singleton: the singleton tweak_point for
  the SP input (A_sum = K_send over its single coin_id) is PRESENT in `td.tweak_points`. Add a
  comment: FAILS before the fix (both coins land in the size-2 Pass-2a bucket → `grouped=true`
  → no singleton emitted), PASSES after.

HARD constraints (CLAUDE.md): no new workspace deps; no new `#[allow]`; no `unsafe`; chip-0057
code must compile WITH and WITHOUT `--all-features`. After editing, check LSP diagnostics and
run scoped clippy with `-D warnings` before reporting done. Fix any clippy::pedantic inline
(rename locals for similar_names, add doc backticks for doc_markdown) — do NOT add `#[allow]`.

The simulator adapter `tweak_data_from_simulator_block` (chia-sdk-test) is a thin delegate and
inherits the fix automatically — do NOT edit it; just confirm the e2e oracles still detect.
  </action>
  <verify>
    <automated>cargo test --release -p chia-sdk-driver --all-features -- silent_payments::block_tweak_data::tests --nocapture 2>&1 | tail -30</automated>
    <automated>cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e 2>&1 | tail -20</automated>
  </verify>
  <done>
block_tweak_data.rs implements the additive three-pass union: Pass 1 emits a singleton for
EVERY standard spend; Pass 2a same-PH groups (size >= 2) kept as additional candidates; Pass 2b
SCC built over ALL standard removals (no `surviving`/`!grouped` exclusion); per-group A_sum +
input_hash + tweak_point with §459 identity suppression; byte-equality dedup keeping first;
documented deterministic emission order in the module doc. BUG-1 and BUG-2 regression tests pass
(and would fail pre-fix per their inline comments). Existing pinned tests updated for the new
counts; the pollution-resistance legit-SCC byte-invariance still holds. The 4 existing e2e
tests (unlabeled, multi_input distinct-PH, labeled, m0_self_change) still pass. Sender code
untouched. Scoped clippy `-D warnings` clean; no new `#[allow]`/`unsafe`/deps.
  </done>
</task>

</tasks>

<verification>
Run the full phase gate after BOTH tasks (mirrors CI exactly). All must pass:

```bash
cd /home/kdc/chia-wallet-sdk

# 1. Planning-residue grep across the full SP source set must be 0 (doc + code carry no GSD vocab).
SP_FILES=$(find crates -path '*silent_payments*' -name '*.rs')
SP_FILES="$SP_FILES crates/chia-sdk-driver/src/action_system/spends.rs crates/chia-sdk-driver/src/action_system/send_destination.rs crates/chia-sdk-driver/src/action_system/relation.rs crates/chia-sdk-bindings/src/silent_payments.rs ISSUE-sp-block-scanner-grouping-conformance.md"
grep -rnE '(Phase [0-9]|\b(RECV|SEND|ADDR|CRYPTO|GUARD|BIND|BRIDGE|POLISH|CLEANUP|SIM)-[0-9]|Pitfall [0-9]|Plan [0-9]{2}-|\bD-[0-9]|SC[0-9]\b)' $SP_FILES | wc -l   # == 0

# 2. Findings doc exists + self-contained citations.
test -f ISSUE-sp-block-scanner-grouping-conformance.md
grep -q dd0e683 ISSUE-sp-block-scanner-grouping-conformance.md && grep -q f19f75d ISSUE-sp-block-scanner-grouping-conformance.md

# 3. chip-0057 gating: build the affected crate WITH and WITHOUT --all-features.
cargo build -p chia-sdk-driver
cargo build -p chia-sdk-driver --all-features
cargo build -p chia-sdk-driver -F chip-0057

# 4. Targeted regression + e2e tests.
cargo test --release -p chia-sdk-driver --all-features -- silent_payments::block_tweak_data::tests
cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e

# 5. Full CI workspace suite green.
cargo test --release --workspace --all-features \
  --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive \
  --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm \
  --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro

# 6. Lint gates.
cargo clippy --workspace --all-features --all-targets   # only the 2 pre-existing chia-sdk-daemon client.rs:426-427 warnings may remain
cargo fmt --all -- --files-with-diff --check
cargo machete
```
</verification>

<success_criteria>
- ISSUE-sp-block-scanner-grouping-conformance.md exists at repo root, self-contained, >= 80
  lines, cites chip-0057.md line ranges (295-332, 319-330, 466-474), block_tweak_data.rs
  file:line for the exclusion + singletons-for-ungrouped loop, both reference commits
  (dd0e683, f19f75d), and separates the SDK-fix vs drop-2a-proposal concerns.
- block_tweak_data.rs grouping is additive/overlapping (Pass 1 singleton-for-all + Pass 2a kept
  + Pass 2b over ALL removals), with byte-equality dedup, §459 identity suppression preserved,
  and a documented deterministic emission order in the module doc.
- BUG-1 (3-coin/2-PH mixed multi-input → all-three-aggregate tweak_point present) and BUG-2
  (single-input sharing a PH → Pass-1 singleton present) regression tests pass, and would fail
  pre-fix (documented inline).
- Existing pinned tests updated for the new additive counts; pollution-resistance legit-SCC
  byte-invariance still holds; all 4 e2e tests still pass.
- Sender (emit_relation / AssertConcurrent cycle / sp_finish_branch) UNCHANGED.
- Full CI workspace suite green; clippy --all-features --all-targets clean except the 2
  pre-existing chia-sdk-daemon warnings; fmt clean; machete clean; residue grep == 0.
- No new workspace deps, no new #[allow], no unsafe; compiles with and without --all-features.
</success_criteria>

<output>
After completion, create
`.planning/quick/260602-ejk-fix-sp-block-scanner-grouping-to-chip-ad/260602-ejk-SUMMARY.md`.
</output>
