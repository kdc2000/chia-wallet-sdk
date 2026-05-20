# Phase 7: Code review cleanup - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-20
**Phase:** 07-code-review-cleanup
**Areas discussed:** CLEANUP-02 split target, CLEANUP-03 leak resolution, CLEANUP-04 e2e.rs strategy, CLEANUP-05 scope, CLEANUP-01 replacement style

---

## Gray area selection

| Option | Description | Selected |
|--------|-------------|----------|
| CLEANUP-02 split target | Where does the SP arm go? Flat sibling vs subdir vs no-suffix | ✓ |
| CLEANUP-03 leak resolution | Push into prepare / doc(hidden) / restructure bindings | ✓ |
| CLEANUP-04 e2e.rs strategy | Relocate / try-in-place / relocate + document | ✓ |
| CLEANUP-05 scope (do/skip) | Drop / split anyway / redirect to tests target | ✓ |

**User's choice:** All 4 selected for discussion.

---

## CLEANUP-02 split target

| Option | Description | Selected |
|--------|-------------|----------|
| `actions/silent_payment_send.rs` (Recommended) | Flat sibling. Mirrors mint_nft.rs / issue_cat.rs / settle.rs convention. Same name Phase 4.2 deleted in commit 4cd99543 — fine because the file is now an internal arm, not a public action factory. | ✓ |
| `actions/silent_payment.rs` | Drop _send suffix. Inconsistent with verb-noun pattern (mint_nft, issue_cat, melt_singleton, create_did). | |
| `actions/send/{mod.rs, silent_payment.rs}` | Subdirectory for related variants. Cleaner if multiple send arms emerge, but introduces a structural pattern no other actions/* file uses. | |

**User's choice:** `actions/silent_payment_send.rs` (flat sibling, recommended).
**Notes:** Matches existing convention exactly. Resurrecting the filename Phase 4.2 deleted is acceptable because the role is different (internal arm vs public factory).

---

## CLEANUP-03 leak resolution

| Option | Description | Selected |
|--------|-------------|----------|
| Push sp_finish_branch into Spends::prepare (Recommended) | Both Rust finish_with_keys path AND binding path converge through prepare. Removes finish_silent_payments entirely. Cleanest. | ✓ |
| Mark #[doc(hidden)] + rename | Keep the public method, hide from docs. Leaves the symbol reachable. Less invasive. | |
| Re-export sp_finish_branch / restructure bindings | Make sp_finish_branch reachable from chia-sdk-bindings so the binding wrapper invokes it directly. Trades one architectural concern for another. | |

**User's choice:** Push sp_finish_branch into Spends::prepare.
**Notes:** Acknowledges audit requirement for existing prepare callsites. Side effect: deletes `spends.finish_silent_payments(&mut ctx, Relation::None)?;` line at `chia-sdk-bindings/src/action_system.rs:191`.

---

## CLEANUP-04 e2e.rs strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Relocate to crates/chia-sdk-driver/tests/silent_payments_e2e.rs (Recommended) | Integration test target sidesteps cyclic-dev-dep type-confusion entirely. Tests call canonical helper. Introduces first tests/ dir in chia-sdk-driver. | ✓ |
| Try in place first, fall back to relocate | Confirm the cycle was real before relocating. Adds an attempt step that likely fails. | |
| Relocate AND document tests/ convention in CLAUDE.md | Same as A plus document. Slightly more scope. | |

**User's choice:** Relocate (recommended). Documenting the tests/ convention in CLAUDE.md is deferred (conventional Rust practice doesn't need project-level documentation).

---

## CLEANUP-05 scope (do/skip)

| Option | Description | Selected |
|--------|-------------|----------|
| Drop CLEANUP-05 (Recommended) | Existing pattern IS inline tests. CAT has 8 inline; SP scanner has 9 inline. Convention preserved. Remove CLEANUP-05 from REQUIREMENTS + ROADMAP + Phase 7 scope. | ✓ |
| Split despite convention drift | Move scanner.rs `mod tests` to sibling scanner_tests.rs even though no other file uses that pattern. | |
| Keep CLEANUP-05 but redirect to integration tests | Move spec-compliance tests to crates/chia-sdk-driver/tests/scanner_spec.rs alongside CLEANUP-04 e2e tests. | |

**User's choice:** Drop CLEANUP-05.
**Notes:** Convention discovery during analysis — CAT/NFT/Singleton primitives all use inline `#[cfg(test)] mod tests {}`. Splitting `scanner.rs` would be the convention violation, not the file size.

---

## Follow-up question: CLEANUP-01 replacement style

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid: strip + reread + repair (Recommended) | Pass 1: delete GSD label prefixes. Pass 2: read each surviving comment — keep if technical, rewrite to cite CHIP/BIP if orphaned, delete if vacuous. | ✓ |
| Just strip labels, preserve content | Pass 1 only. Lower effort, lower polish — awkward orphan sentences stay awkward. | |
| Strip + cite CHIP/BIP everywhere | Aggressive rewrite. Some SDK-internal invariants have no spec; would force awkward "SDK invariant:" phrasings. | |

**User's choice:** Hybrid strip + reread + repair.
**Notes:** Optimizes for surviving comments carrying technical value standalone. 128 hits across SP source files. Production rustdoc (`///`) and line comments (`//`) both treated.

---

## Claude's Discretion (decisions not surfaced for discussion)

- **CLEANUP-06 scope:** Both — manually flip the 7 stale VALIDATION.md frontmatter files AND patch gsd-tools `phase complete` to auto-flip going forward. Manual flip alone leaves the bug in place; CLI patch alone leaves existing audit gap. Both is the obvious-correct default.
- **Phase 7 task ordering and wave assignment:** Planner picks. Suggested natural ordering: CLEANUP-01 → CLEANUP-02 → CLEANUP-03 (Wave 0 audit + Wave 1 change) → CLEANUP-04 → CLEANUP-06.
- **Test continuity gate:** Every CLEANUP-* commit must keep the full workspace test suite green. Phase-wide invariant.
- **Exact `pub(crate) fn` name extracted by CLEANUP-02:** `spend_silent_payment` / `apply_silent_payment_send` / similar — planner picks per existing internal-fn conventions.
- **No `#[deprecated]` shims** — per CLAUDE.md, no backwards-compat shims. Methods deleted by CLEANUP-03 and CLEANUP-04 are gone outright (matches how Phase 4.2 deleted `Action::silent_payment_send`).

## Deferred Ideas

- **CLEANUP-05:** dropped from Phase 7; revisit as a v1.1 broader "split large test modules" rule if file size becomes a real maintenance pain
- **Document `tests/` integration-target convention in CLAUDE.md:** conventional Rust practice; not worth the scope creep
- **Maintainer-approval gate:** not part of Phase 7; upstream merge is downstream of GSD
- **Other v1.1 polish:** any further nits found during actual upstream review go to v1.1 backlog
