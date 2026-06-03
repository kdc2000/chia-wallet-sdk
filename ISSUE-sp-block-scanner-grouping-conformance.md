# Block-scanner grouping treated the three ScanBlock passes as a partition, not a union → missed multi-input and PH-colliding single-input payments

**Component:** `chia-sdk-driver` silent payments (CHIP-0057) — `tweak_data_from_block_spends`
**Version:** chia-wallet-sdk 0.33.0. Bugs found at git `39da4549`; resolved through `4d384715`.
**Status:** RESOLVED — both bugs fixed in the SDK, and the drop-Pass-2a simplification this report
proposed was subsequently **adopted by the CHIP team** (chip-silent-payments.md `590d7e3`) and
**implemented in the SDK** (`4d384715`). This document is now a historical record, not an open ask.

## Summary

The SDK's real-block tweak-data builder (`tweak_data_from_block_spends`, the function every
wallet scanner runs against post-decompression block data) implemented the CHIP-0057
**ScanBlock** procedure as a **mutually-exclusive partition** of the standard removals across
its three passes, rather than the **additive union** the CHIP specifies. The CHIP runs Pass 1,
Pass 2a, and Pass 2b independently and unions every detected coin (chip-0057.md:295-332); the
pre-fix SDK instead let an earlier pass *consume* a coin and exclude it from later passes. That
produced two conformance bugs against the published spec. **Both bugs were fixed in the SDK**
first by rewriting the builder to the additive ScanBlock union with byte-equality dedup (quick
task `260602-ejk`, commit `8e17b1f8`). This report then proposed a *separate, optional*
spec-simplification — drop Pass 2a, keep Pass 1 + Pass 2. **That proposal has since been adopted**
by the CHIP team (chip-silent-payments.md `590d7e3` "collapse Pass 2a/2b into a single Pass 2"),
and the SDK was aligned to the simplified two-pass model (quick task `260602-mhw`, commit
`4d384715`) — so the SDK scanner is now **Pass 1 + Pass 2 only** and no longer carries Pass 2a.
This report is preserved as a precise writeup of the original bugs, a corroborating CHIP-history
note, and the now-adopted simplification rationale. It is not a request for the CHIP team to fix
SDK code.

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

## Spec-simplification — drop Pass 2a, keep Pass 1 + Pass 2 (ADOPTED)

This was raised as a *separate, optional* spec-simplification idea. **It has since been adopted:**
the CHIP collapsed Pass 2a/2b into a single Pass 2 (chip-silent-payments.md `590d7e3`), and the
SDK dropped Pass 2a to match (`4d384715`). The analysis below is preserved as the rationale that
motivated the change.

- **What it required (now done).** Flip the sender guidance from "SHOULD prefer spending coins at
  the same derivation index" to "MUST emit the concurrent-spend cycle for every multi-input send"
  — which the adopted CHIP now states. The SDK sender **already** enforced this for `>= 2`
  non-ephemeral XCH inputs (`spends.rs:632`), so the SDK needed no sender change to comply.
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

## Resolution — two concerns, both closed

1. **SDK conformance bug — fixed (`8e17b1f8`).** `tweak_data_from_block_spends` was first
   rewritten to the additive ScanBlock union: Pass 1 emits a singleton for **every** standard
   removal (fixes BUG-2); Pass 2a same-PH groups of size ≥ 2 were kept as additional overlapping
   candidates; Pass 2b built its opcode-64 SCC graph over **all** removals with no
   `surviving`/`!grouped` exclusion (fixes BUG-1). Overlapping groups can produce byte-identical
   tweak points, which are de-duplicated by the 48-byte compressed point (first occurrence kept),
   and the CHIP §459 identity-element (point-at-infinity) suppression is preserved. This made the
   SDK conform to the then-current three-pass CHIP-0057 ScanBlock.
2. **Drop-Pass-2a — adopted (`590d7e3`) and implemented (`4d384715`).** The CHIP team accepted the
   simplification, collapsing Pass 2a/2b into a single Pass 2 (with the "MUST emit the cycle"
   sender rule), and the SDK was aligned by removing Pass 2a entirely — its scanner is now **Pass 1
   + Pass 2 (opcode-64 SCC over all removals)**, matching the simplified CHIP byte-for-byte. The
   dedup and §459 suppression are unchanged and the sender was untouched. The efficiency and
   fingerprint trade-offs spelled out above were the deciding rationale.

## Context

We hit this porting a BIP-352 silent-payments flow onto chia-wallet-sdk. The SDK side is fully
resolved: the original undetectable-coin bugs were fixed (regression tests for both retained),
and after the CHIP adopted the drop-Pass-2a simplification the SDK was reduced to the matching
two-pass model (Pass 1 + Pass 2 SCC over all removals). This document is retained as the worked
analysis and the rationale behind the now-adopted change.
