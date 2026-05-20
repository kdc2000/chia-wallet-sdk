# Phase 8: Second-pass v1 polish — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-20
**Phase:** 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
**Areas discussed:** POLISH-01 (mod.rs surface), POLISH-02 (module fold), POLISH-03 (Boxing rationale dedup), POLISH-04 (dispatch restructure), POLISH-05 scope check

---

## POLISH-01 — `silent_payments/mod.rs` over-exports

| Option | Description | Selected |
|--------|-------------|----------|
| Switch wildcards to named `pub use`; keep primitives `pub` | Replace `pub use foo::*;` with explicit `pub use foo::{name1, name2};` for each submodule. Public surface unchanged, prelude unchanged — just kills the wildcards so the surface is visible at a glance. Honors mod.rs rustdoc commitment that protocol primitives are publicly reusable. | ✓ |
| Demote protocol primitives to `pub(crate)`; remove from prelude | More aggressive — admits the 5 primitives never had a real external use case. mod.rs rustdoc rewritten; `src/prelude.rs:43-44` drops 5 names. Public API break, but the crate hasn't merged upstream yet. Bindings unaffected. one_time.rs still uses them via `pub(crate)` paths. | |
| Demote primitives to `pub(crate)`; keep them in prelude | Hybrid — reachable via `chia_wallet_sdk::prelude::*` but not via `chia_sdk_driver::silent_payments::`. Creates two visibility levels; likely confuses more than helps. | |

**User's choice:** Switch wildcards to named `pub use`; keep primitives `pub` (Recommended)
**Notes:** Honors the mod.rs rustdoc commitment that protocol primitives are reusable; the maintainer concern was the wildcard pattern, not the surface size itself. Five protocol primitives stay `pub`.

---

## POLISH-02 — Fold single-function modules

| Option | Description | Selected |
|--------|-------------|----------|
| Fold all three into `protocol.rs` | protocol.rs: 199 → ~640 lines (still under scanner.rs's 735). Single module hosts primitives + the three compositions that use them. mod.rs loses 3 module declarations + 3 re-exports. Tests stay inline at file bottom. | ✓ |
| Fold into a new `send_protocol.rs` module | Dedicated module for send-side compositions. Cleaner separation between protocol primitives (used by both send + receive) and send-only compositions. But introduces a new module name and doesn't reduce file count vs. current. | |
| Keep separate; do only POLISH-01 (visibility narrowing) | Don't restructure. Safe no-op if the file-count concern doesn't actually bother the maintainer. Tests stay where they are. | |

**User's choice:** Fold all three into `protocol.rs` (Recommended)
**Notes:** receive-side scanner.rs already imports the protocol primitives from protocol.rs; folding the send-side compositions in alongside keeps "the primitive set + the composers that use it" co-located.

---

## POLISH-03 — `SendDestination` Boxing rationale duplication

| Option | Description | Selected |
|--------|-------------|----------|
| Keep variant-level; delete enum-level mention | Variant rustdoc sits directly above `Box<SilentPaymentAddress>` — canonical location. The enum-level 'Cannot derive Copy' note becomes redundant. Net: 3 fewer lines. | ✓ |
| Keep both; rewrite enum-level to point at variant doc | Conservative — keeps high-level orientation but says 'see SilentPayment variant for boxing rationale'. Slightly more boilerplate. | |

**User's choice:** Keep variant-level; delete enum-level mention (Recommended)
**Notes:** Canonical-location wins over orientation. Net ~3 lines saved.

---

## POLISH-04 — `Action::send` chip-0057 dispatch

| Option | Description | Selected |
|--------|-------------|----------|
| Single exhaustive `match`; SP arm `return`s from inside | Replace if-let + post-match-with-unreachable!() with one match. SP arm returns directly. Eliminates `unreachable!()` + its rationale comment. ~8 lines removed; dispatch reads top-to-bottom. | ✓ |
| Keep current structure; trim only the rationale comment | Leave the early-return pattern; just delete the comment about unreachable!(). Doesn't fix the structural smell. | |
| Extract puzzle-hash dispatch into a helper taking `Bytes32` | More invasive — pull the post-match Cat/Did/Nft/Option dispatch into a helper. Both match arms call the helper. Cleaner separation but a larger diff than the maintainer concern strictly warrants. | |

**User's choice:** Single exhaustive `match`; SP arm `return`s from inside (Recommended)
**Notes:** Eliminates the `unreachable!()` smell; behavior unchanged; ~8 lines saved.

---

## POLISH-05 — Scope check (extract scanner.rs tests)

| Option | Description | Selected |
|--------|-------------|----------|
| Leave scanner.rs as-is | Repo convention is inline tests — every other large primitive in the workspace (CAT, NFT, Singleton, Vault, scanner-equivalents) follows the same pattern. Extracting scanner tests would set a precedent that breaks consistency. If a future v1.1+ phase introduces a broader 'split large test modules' rule across the SDK, scanner.rs comes along then. Phase 8 stays focused on the 4 nits. | ✓ |
| Add as POLISH-05 — extract scanner.rs tests to `scanner/tests.rs` | Creates a submodule structure `silent_payments/scanner/mod.rs` + `silent_payments/scanner/tests.rs`. Sets the precedent of inline-vs-extracted on a per-file basis. ~530 lines move. | |

**User's choice:** Leave scanner.rs as-is (Recommended)
**Notes:** Same call as Phase 7's CLEANUP-05 drop. Repo convention precedent wins.

---

## Claude's Discretion

Items left to planner judgment, recorded in CONTEXT.md `<decisions>` D-02 Claude's Discretion subsection:
- Test-module merger shape in POLISH-02 (one flat `mod tests {}` vs. sub-mods preserving original groupings)
- Whether to dedup overlapping `use` statements after the fold
- Whether to opportunistically update `silent_payments/mod.rs` rustdoc to reflect smaller module count
- Phase ordering and wave assignment (suggested in CONTEXT.md but planner picks)

## Deferred Ideas

- Demoting protocol primitives to `pub(crate)` (rejected per POLISH-01 trade-off)
- Extracting `dispatch_puzzle_hash_target` helper (rejected per POLISH-04 scope)
- POLISH-05 scanner.rs test extraction (deferred per repo convention)
- Sage-side consumer audit (CONTEXT.md `<canonical_refs>` lists known consumers; planner discovers any unusual import paths at execution time)
- Other v1.1 polish items (out of scope for this phase)
