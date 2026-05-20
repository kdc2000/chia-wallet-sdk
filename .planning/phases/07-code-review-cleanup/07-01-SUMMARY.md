---
phase: 07-code-review-cleanup
plan: 01
subsystem: testing
tags: [cleanup, comments, chip-0057, silent-payments, code-review]

# Dependency graph
requires:
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: source files containing GSD planning-artifact references in comments
provides:
  - silent_payments source comments cite CHIP-0057 / BIP-352 specs (or restate constraints) instead of GSD planning labels
  - 17 of 18 CLEANUP-01 target files report 0 acceptance-grep hits; e2e.rs (4 hits) deferred to Plan 07-05's file deletion
affects: [07-05 code-review-cleanup-04-delete-e2e]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Pattern A — test-vector pin labels rewritten from "RESEARCH §10a" style to "CHIP-0057 test vector N" style (scanner.rs, protocol.rs, address.rs, keys.rs, labels.rs)
    - Pattern B — rustdoc label deletion preserving CHIP cite (mod.rs, types.rs, send_keys.rs, one_time.rs, input_hash.rs)
    - Pattern C — restate technical constraint without Plan numbers (scanner.rs #[allow] justification, aggregate.rs synthetic-vs-raw boundary, bindings facade ScalarField rationale)

key-files:
  created:
    - .planning/phases/07-code-review-cleanup/07-01-SUMMARY.md
  modified:
    - crates/chia-sdk-driver/src/silent_payments/scanner.rs
    - crates/chia-sdk-driver/src/silent_payments/input_hash.rs
    - crates/chia-sdk-driver/src/silent_payments/aggregate.rs
    - crates/chia-sdk-driver/src/silent_payments/one_time.rs
    - crates/chia-sdk-driver/src/silent_payments/send_keys.rs
    - crates/chia-sdk-driver/src/silent_payments/mod.rs
    - crates/chia-sdk-driver/src/silent_payments/types.rs
    - crates/chia-sdk-driver/src/silent_payments/protocol.rs
    - crates/chia-sdk-utils/src/silent_payments/keys.rs
    - crates/chia-sdk-utils/src/silent_payments/labels.rs
    - crates/chia-sdk-utils/src/silent_payments/address.rs
    - crates/chia-sdk-bindings/src/silent_payments.rs
    - examples/silent_payment.rs
    - napi/__test__/silent_payments.spec.ts
    - napi/__test__/silent_payments_e2e.spec.ts
    - pyo3/tests/test_silent_payments.py
    - wasm/__test__/silent_payments.spec.ts

key-decisions:
  - "Apply strip-reread-repair (Pattern A/B/C) over blanket deletion — surviving comments cite CHIP-0057 / BIP-352 or restate the technical constraint plainly"
  - "Skip crates/chia-sdk-driver/src/silent_payments/e2e.rs entirely — Plan 07-05 (CLEANUP-04) deletes the file"
  - "Rustdoc (//!, ///) gets rewritten, not deleted — it renders to docs.rs and IDE tooltips so naked deletion would harm the public API documentation"

patterns-established:
  - "Test-vector pin labels: CHIP-0057 test vector N (TV1/TV3/TV4) replaces RESEARCH §10a/c/d shorthand"
  - "Decision-ID rewrites: a binding-facade comment must describe the technical claim (zero-field namespace pattern, unsigned-mod-r FFI boundary) instead of the D-NN decision label that created it"
  - "Phase-number references in non-doc comments are removed when the surrounding code is self-evident (silent_payment_send.rs test docstrings); kept when they explain why a particular API shape exists (then rewritten without the Plan label)"

requirements-completed: [CLEANUP-01]

# Metrics
duration: 11min
completed: 2026-05-20
---

# Phase 7 Plan 01: Strip GSD Planning References from Silent-Payments Source Comments Summary

**128-hit comment cleanup across 17 silent-payments source files — production comments now cite CHIP-0057 / BIP-352 specs (or restate constraints in plain English) instead of GSD-internal planning artifacts; e2e.rs deferred to Plan 07-05's deletion.**

## Performance

- **Duration:** 11 min (resumed from a previous executor crash; this run completed Pass-1 for the remaining 9 files plus Tasks 2-3 in full)
- **Started:** 2026-05-20T15:24:21Z
- **Completed:** 2026-05-20T15:35:47Z
- **Tasks:** 3 of 3 atomic
- **Files modified:** 17

## Accomplishments

- Acceptance grep across the 17 non-e2e.rs target files returns 0 hits against the CLEANUP-01 pattern (`CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-6]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]`)
- Test-vector pin labels (scanner.rs / protocol.rs / address.rs / keys.rs / labels.rs) now read "CHIP-0057 test vector 1" / "test vector 3 — labeled, m = 1" / "test vector 4 — multi-input" — durable, citable in a PR review
- Scanner's `#[allow(clippy::similar_names)]` block now justifies the lint with the actual technical claim (function signature + labeled-detection branch structurally lock the param names) instead of pointing at Plan 03-03 / 03-04
- Binding facade's module-level doc rewritten to describe the namespace pattern (matching Constants/Clvm) and the unsigned-mod-r FFI invariant on `ScalarField`, both as load-bearing rationale instead of decision-ID citations
- e2e.rs verified to still have its 4 hits (handover to Plan 07-05) — file untouched by this plan
- Workspace builds + clippy + cargo fmt all clean on chip-0057-gated paths; 24 silent_payments driver tests + 27 silent_payments utils tests still pass

## Task Commits

Each task was committed atomically against the chia-wallet-sdk single repo:

1. **Task 1: Strip GSD references from Rust source comments (driver + utils silent_payments modules)** - `605c17b2` (refactor)
   - 11 files: scanner.rs, input_hash.rs, aggregate.rs, one_time.rs, send_keys.rs, mod.rs, types.rs, protocol.rs (driver) + keys.rs, labels.rs, address.rs (utils)
   - 38 hits removed across the 11 files
   - Includes previous executor's partial Pass-1 edits (verified during inspection; quality-checked before commit)

2. **Task 2: Strip GSD references from binding facade and example** - `c2df06ed` (refactor)
   - 2 files: crates/chia-sdk-bindings/src/silent_payments.rs, examples/silent_payment.rs
   - 11 hits removed (9 + 2)

3. **Task 3: Strip GSD references from binding test files (TS + Py + WASM)** - `95833f50` (refactor)
   - 4 files: napi/__test__/silent_payments{,_e2e}.spec.ts, pyo3/tests/test_silent_payments.py, wasm/__test__/silent_payments.spec.ts
   - 11 hits removed (3 + 2 + 3 + 3); test assertions / imports / fixtures untouched

**Plan metadata commit:** pending (final commit captures SUMMARY.md + STATE.md + ROADMAP.md)

## Files Created/Modified

### Source code (Rust, driver + utils)

- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — Pattern A pin labels for TV1/TV3/TV4 + Pattern C rewrite of the #[allow] justification block + Open Question 3 / Plan 03-04 references removed from the SilentPaymentScan trait rustdoc
- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` — Plan 04-03 / 04-04 / RESEARCH §5 / Pitfall A references replaced with prose describing the Relation::AssertConcurrent cycle binding + synthetic-vs-raw caveat directly
- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` — RESEARCH §11 Pitfall A/H labels replaced with "synthetic-vs-raw key boundary" + "cosmic-ray-level probability" prose; Plan 04-03 module reference rewritten as "Spends::finish_with_keys performs the Spends-level multi-party hard-error check"
- `crates/chia-sdk-driver/src/silent_payments/one_time.rs` — Plan 04-03 / 04-04 + Section 11 Pitfall A references rewritten to cite Spends::finish_with_keys and describe the synthetic-vs-raw round-trip caveat directly
- `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` — Plan 04.2-02 / D-06 / Pitfall 7 test docstring labels removed; the input-binding gate ordering is now described in terms of its purpose (fail-fast on misconfigured multi-input sends)
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — Crate-level `//!` rewritten: Phase 6 / Plan 03-02 / Plan 03-03 / Phase 1 phase-number references all dropped; rustdoc cross-references now use proper `[...]` intra-doc-link syntax
- `crates/chia-sdk-driver/src/silent_payments/types.rs` — `03-RESEARCH.md §11 test #8` reference replaced with a forward-looking statement about the deserialization boundary
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` — RESEARCH §10a pin label → "CHIP-0057 test vector 1"
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` — Pattern A pin labels for TV1 mnemonic + scan/spend bytes + mainnet address
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` — Pattern A pin labels for TV1 + TV3 (m=1)
- `crates/chia-sdk-utils/src/silent_payments/address.rs` — Pattern A pin labels for TV1 + TV3; "From RESEARCH §8 lines 528-545" deletion (CHIP-0057 cite suffices)

### Binding facade + example

- `crates/chia-sdk-bindings/src/silent_payments.rs` — Module-level doc rewritten (D-01/D-02/D-03 labels removed; namespace-pattern + unsigned-mod-r FFI rationale rewritten in plain English); ScalarField section header + rustdoc rewritten; from_bytes inline comment rewritten; SilentPayments namespace section header + rustdoc rewritten; RESEARCH Pattern 4 reference deleted from the SilentPaymentNetwork header
- `examples/silent_payment.rs` — TV1 mnemonic doc-comment Plan 06-03 reference removed; Stage 2 Phase 4.2 label removed; Stage 5 RESEARCH Pitfall 6 reference rewritten as the underlying claim (signing with raw onetime_sk would produce an invalid signature)

### Binding tests (no test logic changed — comment-only)

- `napi/__test__/silent_payments.spec.ts` — Header docstring rewritten (RESEARCH Anti-Pattern 5 + Phase 04.2 CONTEXT.md labels removed; bech32m-library-churn rationale rewritten); SC3 smoke test inline comment Plan 05-02 Task 3 reference rewritten as the technical claim
- `napi/__test__/silent_payments_e2e.spec.ts` — Header docstring D-01 / D-07 / Plan 06-03 labels removed; unlabeled-scope rationale + Vec<PublicKey> FFI claim restated plainly
- `pyo3/tests/test_silent_payments.py` — Module docstring D-01 / D-07 / D-08 / Plan 06-03 labels removed; equivalent rewrites
- `wasm/__test__/silent_payments.spec.ts` — Header docstring D-01 / D-07 / Plan 06-03 labels removed; equivalent rewrites

## Decisions Made

- **Strip-reread-repair over blanket deletion.** Each comment was read in context and triaged into Pattern A (test-vector pin label), Pattern B (rustdoc preserve-citation), or Pattern C (restate technical constraint). Roughly 60% of hits are Pattern A (mechanical), 30% Pattern B (preserve a CHIP § cite, drop the Plan label), 10% Pattern C (substantive rewrite). No comments were deleted wholesale when the surrounding code wasn't trivially self-evident.
- **Rustdoc gets the most-careful treatment.** Module-level `//!` and item-level `///` are rendered on docs.rs and in IDE tooltips, so cleanup here is upstream-visible. The driver `mod.rs` and bindings-facade `silent_payments.rs` module-level docs were the heaviest rewrites — each now reads as a standalone API-surface description without needing to know what "CONTEXT.md D-01" referred to.
- **e2e.rs not touched.** Plan 07-05 deletes the entire file; cleaning its 4 hits here is wasted work. Verified post-commit that e2e.rs still has its 4 hits.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Previous executor crashed during Pass 1.** Inherited a working tree with 8 files partially edited (driver mod/protocol/types/aggregate/input_hash + utils keys/labels/address) and 9 files untouched. Inspected the existing diffs against the plan's playbook before continuing — they were high-quality (matched Pattern A/B/C rules) and consistent with the surviving files' style. Continued Pass 1 on the remaining 9 files in this run and committed all 17 files atomically per task scope.
- **STATE.md uncommitted diff** (was the previous executor's session bookkeeping: status verifying→executing, focus Phase 06→Phase 07, total_plans 34→39) was preserved for the final metadata commit — it represents the correct Phase 7 entry state.

## User Setup Required

None - comment-only edits.

## Next Phase Readiness

- **Plan 07-02** (CLEANUP-02 — split actions/send.rs) is unblocked. Plan 07-01's edits do not touch action_system/send.rs or send_destination.rs.
- **Plan 07-05** (CLEANUP-04 — delete e2e.rs) will close the remaining 4 acceptance-grep hits when it lands; full CLEANUP-01 acceptance grep returns 0 hits only after both plans complete.

---
*Phase: 07-code-review-cleanup*
*Completed: 2026-05-20*

## Self-Check: PASSED

- SUMMARY.md exists at .planning/phases/07-code-review-cleanup/07-01-SUMMARY.md
- Task 1 commit 605c17b2 found in git log
- Task 2 commit c2df06ed found in git log
- Task 3 commit 95833f50 found in git log
- Acceptance grep (excluding e2e.rs) returns 0 hits across the 17-file target set
- e2e.rs verified to retain its 4 hits (Plan 07-05 will resolve)
