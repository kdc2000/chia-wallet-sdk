# Block-scanner grouping treated the three ScanBlock passes as a partition, not a union → missed multi-input and PH-colliding single-input payments

**Component:** `chia-sdk-driver` silent payments (CHIP-0057) — `tweak_data_from_block_spends`
**Version:** chia-wallet-sdk 0.33.0 (git `39da45494c0261e0771661233b846236b7663352`, short `39da4549`).

## Summary

The SDK's real-block tweak-data builder (`tweak_data_from_block_spends`, the function every
wallet scanner runs against post-decompression block data) implemented the CHIP-0057
**ScanBlock** procedure as a **mutually-exclusive partition** of the standard removals across
its three passes, rather than the **additive union** the CHIP specifies. The CHIP runs Pass 1,
Pass 2a, and Pass 2b independently and unions every detected coin (chip-0057.md:295-332); the
pre-fix SDK instead let an earlier pass *consume* a coin and exclude it from later passes. That
produced two conformance bugs against the published spec. **Both are now fixed in the SDK** (the
builder was rewritten to the additive model with byte-equality dedup); this report exists to
hand the silent-payments / CHIP team a precise writeup of the bugs, a corroborating
CHIP-history note, and a *separate, optional* spec-simplification proposal for the team to weigh.
It is not a request for the CHIP team to fix SDK code.

## The two bugs

### BUG-1 — mixed-puzzle-hash multi-input fragmentation

A multi-input send whose inputs span two or more distinct puzzle hashes — e.g. two coins at
`PH_x` and one coin at `PH_y`, all bound by one `ASSERT_CONCURRENT_SPEND` (opcode 64) cycle —
was fragmented. Pass 2a captured the `PH_x` pair as a same-PH group and marked those two coin
indices `grouped[i] = true`. Pass 2b then built its opcode-64 SCC graph only over the coins
that were *not* already grouped, so the two `PH_x` coins were **excluded from the SCC graph
entirely**. The full three-coin cycle was never recognised as one strongly-connected component,
its three synthetic keys were never summed into a single `A_sum`, and the `input_hash` the
sender computed over **all three** inputs was therefore never reconstructed by the scanner. The
recipient's one-time puzzle hash never appeared in the emitted `tweak_points`, and the payment
was silently missed.

#### Worked example: 3 coins, 2 puzzle hashes, one concurrent-spend cycle

Inputs: `coin_x1` and `coin_x2` at `PH_x` (two coins at one derivation index), `coin_y1` at
`PH_y` (a third coin at a different index). The sender binds all three with a cyclic opcode-64
pattern (each coin asserts its predecessor, coin 0 closes the cycle to coin N-1 — exactly the
shape the SDK sender emits, see "Single-input fact" below). The sender's `A_sum` is
`pk_x1 + pk_x2 + pk_y1` over the full three-coin input set.

- **What the CHIP says (DETECTS it).** ScanBlock runs all three passes and unions the results
  (chip-0057.md:295-332). Pass 2b builds the directed opcode-64 graph over **all** removals
  (chip-0057.md:319-330: "For each *removal* in *removals* … add edge … strongly connected
  components of the directed graph"), so the three-coin cycle is one SCC of size 3. That SCC
  aggregates `pk_x1 + pk_x2 + pk_y1`, computes the input hash over all three coin IDs, and emits
  the tweak point the sender used. Detection succeeds. The Edge-Cases section reinforces this:
  Pass 2b groups "coins at different derivation indices … spent together"
  (chip-0057.md:466-474).
- **What the pre-fix SDK did (FRAGMENTS it).** Pass 2a bucketed `{coin_x1, coin_x2}` by `PH_x`
  and set `grouped` true for both (`block_tweak_data.rs:117-126`). Pass 2b then restricted its
  graph to the *surviving* (ungrouped) coins via
  `surviving = (0..n).filter(|i| !grouped[i])` (`block_tweak_data.rs:128-131`) and built
  `coin_id_to_pos` only over `surviving` (`block_tweak_data.rs:134-138`). So `coin_y1`'s
  opcode-64 edge pointed at a `coin_x*` that was **not a node in the graph**, the edge was
  dropped, and no SCC of size ≥ 2 formed. The only groups emitted were the `{coin_x1, coin_x2}`
  Pass-2a pair (`A_sum = pk_x1 + pk_x2`, wrong) and the standalone `coin_y1` singleton. The
  correct three-coin `A_sum` was never produced — payment missed.

### BUG-2 — single-input send sharing a puzzle hash with an unrelated coin

Singletons were emitted **only** for coins that fell through every group: the standalone loop
was gated `if !is_grouped` (`block_tweak_data.rs:174-179`). So a *single-input* SP send whose
lone input happened to share a puzzle hash with an unrelated standard coin elsewhere in the same
block landed in a Pass-2a bucket of size 2, was marked `grouped = true`, and was therefore
**never emitted as a Pass-1 singleton**. The single-input payment was missed even though the
CHIP's Pass 1 (chip-0057.md:297-305, "Each spend as its own group … For each *removal* in
*removals*") would have caught it unconditionally.

The CHIP runs Pass 1 over **every** removal regardless of what 2a or 2b also do with that coin;
the SDK only emitted a singleton as a fallthrough.

## Single-input fact (the *why* behind BUG-2)

Pass 2b detects SCCs of size ≥ 2, so it **structurally cannot** detect a single-input send — a
lone coin forms a trivial SCC of size 1 and is filtered out. Pass 1 (per-spend singletons) is
therefore *mandatory* in any conformant scanner; it is the only pass that catches single-input
sends.

The SDK sender makes this concrete: `emit_relation` early-returns without emitting any
opcode-64 condition when there are `<= 1` coins (`spends.rs:401`), and the SP finish gate only
*requires* `Relation::AssertConcurrent` when there are `>= 2` non-ephemeral XCH inputs
(`spends.rs:632`). A single-input SP send therefore emits **no concurrent-spend cycle at all** —
neither Pass 2a (it is alone at its PH unless an unrelated coin collides) nor Pass 2b can be
relied upon, so only Pass 1 can catch it. Suppressing the Pass-1 singleton (BUG-2) is exactly
the failure that breaks single-input detection.

## Optional proposal for the CHIP team — drop Pass 2a, keep Pass 1 + Pass 2b (presented balanced)

This is a **separate, optional** spec-simplification idea, explicitly **not** what the SDK fix
does. The SDK keeps Pass 2a to match the *current* published CHIP. We raise it only for the
team's consideration.

- **What it would require.** Flip the sender guidance from "SHOULD prefer spending coins at the
  same derivation index" (chip-0057.md:474) to "MUST emit the concurrent-spend cycle for every
  multi-input send." Note the SDK sender **already** enforces this for `>= 2` non-ephemeral XCH
  inputs (`spends.rs:632`), so the SDK would need no sender change to comply.
- **Efficiency correction (this cuts against the obvious intuition).** Pass 2a looks like the
  cheap path: it buckets by `coin.puzzle_hash` and uncurries the synthetic PK straight from the
  puzzle reveal, with no CLVM execution. Pass 2b looks expensive: it must run each removal's
  puzzle+solution through the CLVM VM (`run_puzzle`) to read opcode-64 conditions. **But** a
  *complete* scanner has to run Pass 2b over all removals anyway — which means it already
  executes every standard puzzle through CLVM. Given that, Pass 2a saves **nothing** for a
  complete scanner: every coin's PK is already extracted while running 2b. 2a's apparent CPU win
  exists *only* in the buggy shortcut of skipping 2b for coins 2a already grouped — which is
  precisely the partition that caused BUG-1. The only legitimate reason to keep 2a is to support
  a *deliberately incomplete* cheap scanner (Pass 1 + Pass 2a, no CLVM at all) that knowingly
  misses cross-index multi-input sends.
- **Privacy / fingerprint dimension.** Mandating the cycle for every multi-input send is a
  negligible byte cost, but it interacts with transaction indistinguishability: if
  `ASSERT_CONCURRENT_SPEND` cycles are rare in normal (non-SP) traffic, mandating them for every
  multi-input SP send could become an SP-specific fingerprint. This needs the CHIP team's read
  on the base rate of opcode-64 cyclic patterns in ordinary block traffic before "MUST emit the
  cycle" is adopted.

## CHIP-history corroboration

An earlier revision of this work bound multi-input coins via **empty-message coin
announcements** (opcode 60 / 61) rather than `ASSERT_CONCURRENT_SPEND`. That approach created an
SP-specific on-chain fingerprint (an empty-message announcement pattern that ordinary spends do
not exhibit) and was deliberately replaced by the opcode-64 `ASSERT_CONCURRENT_SPEND` SCC
scheme. The reference `silent-payments` working copy records both steps:

- `dd0e683` — "fix: avoid empty-message announcement fingerprint in multi-input SP"
- `f19f75d` — "feat: rewrite CHIP Pass 2b around SCC over opcode-64 edges"

This history is why Pass 2b is specified as a **strongly-connected-component** computation over
a directed opcode-64 graph (not a weak/undirected connected component): the SCC requirement is
what gives pollution resistance — an adversary spending an unrelated coin in the same block and
emitting `[64, victim_coin_id]` creates only a one-way edge into the victim's group, lands in
its own trivial SCC, and is excluded (chip-0057.md:472). The same property is the reason the
SDK fix had to build the SCC graph over *all* removals: restricting the graph to a non-grouped
subset (the old partition) silently breaks the SCC's ability to span a cross-index cycle.

## Recommendation — two separate concerns

1. **SDK conformance bug — fixed now, no CHIP change needed.** `tweak_data_from_block_spends`
   was rewritten to the additive ScanBlock union: Pass 1 emits a singleton for **every**
   standard removal (fixes BUG-2); Pass 2a same-PH groups of size ≥ 2 are kept as additional
   overlapping candidates; Pass 2b builds its opcode-64 SCC graph over **all** removals with no
   `surviving`/`!grouped` exclusion (fixes BUG-1). Overlapping groups can produce byte-identical
   tweak points, which are de-duplicated by the 48-byte compressed point (first occurrence
   kept), and the CHIP §459 identity-element (point-at-infinity) suppression is preserved. This
   makes the SDK conform to the **current** published CHIP-0057 ScanBlock; no spec change is
   required for this fix.
2. **Drop-Pass-2a — a separate spec-evolution proposal.** Whether to simplify the CHIP to Pass 1
   + Pass 2b (with a "MUST emit the cycle" sender rule) is an independent decision for the CHIP
   team, with the efficiency and fingerprint trade-offs spelled out above. The SDK does **not**
   depend on this proposal being adopted — it conforms to the current three-pass spec as written.

## Context

We hit this porting a BIP-352 silent-payments flow onto chia-wallet-sdk. The SDK side is fixed
(additive grouping + dedup + regression tests for both bugs). We are surfacing the worked
analysis, the SCC-over-all-removals requirement, and the optional drop-2a idea so the CHIP team
has a precise record. Happy to provide a minimal reproduction or discuss the drop-2a trade-offs
further.
