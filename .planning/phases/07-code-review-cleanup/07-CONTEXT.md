# Phase 7: Code review cleanup - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Address the maintainer-facing issues flagged by the post-v1 code review (2026-05-19). v1 of CHIP-0057 silent payments closed requirement-complete on 2026-05-19; this phase polishes the surface so the work is ready for upstream merge without follow-up nits.

**In scope:** 5 cleanup tasks (CLEANUP-01, -02, -03, -04, -06). No behavior change. Pure refactor + comment hygiene + a small CLI process fix.

**Out of scope (per discussion):**
- CLEANUP-05 (split `scanner.rs` tests) — DROPPED. The existing repo convention IS inline `#[cfg(test)] mod tests {}` at file bottoms; CAT/NFT/Singleton all do this. SP code already follows the pattern. The 733-line `scanner.rs` is justified by CHIP-0057 spec-compliance test breadth, not by convention drift. This phase removes CLEANUP-05 from ROADMAP.md, REQUIREMENTS.md, and the Phase 7 success criteria.
- New features, new tests, new APIs, behavior changes. Any of those belong in v1.1 or v2.

</domain>

<decisions>
## Implementation Decisions

### CLEANUP-02 — Split target for the SP arm of `Action::send`

- **D-01:** The chip-0057 arm of `Action::send` moves to **`crates/chia-sdk-driver/src/actions/silent_payment_send.rs`** (flat sibling). Same filename Phase 4.2 deleted in commit `4cd99543` — that's fine; the file now exists as an internal arm of `Action::send`, NOT as a separate public action factory. Module declared in `actions.rs` as `mod silent_payment_send;` with a `pub(crate)` function (name TBD by planner — likely `spend_silent_payment` or similar) that `send.rs`'s SP match arm calls.

  **Why this naming:** mirrors the existing flat-sibling convention exactly (`mint_nft.rs`, `issue_cat.rs`, `create_did.rs`, `melt_singleton.rs`, `update_did.rs`, `update_nft.rs`, `settle.rs`, `fee.rs`, `run_tail.rs` are all flat siblings). No subdirectories anywhere in `actions/`. Verb-noun pattern (`silent_payment_send` matches `mint_nft`/`issue_cat`).

  **Target size:** `actions/send.rs` ≤ 600 lines after the split. Public surface unchanged — `Action::send(id, SendDestination::SilentPayment(addr), amount, memos)` still works identically.

### CLEANUP-03 — Resolve the `Spends::finish_silent_payments` bindings leak

- **D-02:** **Push `sp_finish_branch` invocation into `Spends::prepare` itself.** The `if !self.silent_payments_pending.is_empty() { sp_finish_branch(ctx, self, relation)?; }` block currently duplicated at `spends.rs:151` (the public `finish_silent_payments`) and `spends.rs:575` (inside `finish_with_keys`) moves into `Spends::prepare` directly. Both paths converge:
  - Rust `Spends::finish_with_keys` → calls `prepare` → `prepare` runs `sp_finish_branch` automatically
  - Binding `Spends::prepare` wrapper → calls `prepare` directly → same behavior
  - `Spends::finish_silent_payments` is **deleted** (public surface shrinks by one method)

  **Why this and not the doc(hidden) option:** the bindings leak isn't just a doc concern; it's a public method that exposes internal SP-branch sequencing. Doc-hiding leaves the symbol reachable. Pushing into `prepare` removes the leak entirely.

  **Audit requirement:** All `Spends::prepare` callsites must be audited to ensure the new behavior (SP branch fires automatically when `silent_payments_pending` is non-empty) doesn't break existing flows. Likely no behavioral change — the SP branch is gated on `silent_payments_pending` being non-empty, which is only set by `Action::send(SilentPayment, ...)`. Any `prepare` call where no SP action was applied is a no-op. Planner enumerates the callsites.

  **Side effect:** `chia_sdk_bindings::Spends::prepare` no longer needs the explicit `spends.finish_silent_payments(&mut ctx, Relation::None)?;` line at `action_system.rs:191`. That line gets deleted as part of this CLEANUP.

### CLEANUP-04 — Drop the inlined `build_tweak_data` workaround in `e2e.rs`

- **D-03:** **Relocate the 3 e2e tests to `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`** (integration-test target). Integration tests compile as their own crate, sidestepping the chia-sdk-driver → chia-sdk-test → chia-sdk-driver cyclic-dev-dep type-confusion that forced the in-place inlining in Phase 6. Tests call the canonical `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` directly — the inlined `build_tweak_data` helper is deleted. `crates/chia-sdk-driver/src/silent_payments/e2e.rs` is removed; the `#[cfg(test)] mod e2e;` declaration in `silent_payments/mod.rs` is removed.

  **Why relocate (not try in-place first):** the cyclic-dev-dep type-confusion is a well-known Cargo behavior, not a coincidence. Trying in-place first wastes a cycle confirming what's already documented in the Phase 6 e2e.rs module rustdoc. Going straight to integration target is the cleanest fix.

  **Convention:** This introduces the first `tests/` directory in `chia-sdk-driver`. Conventional Rust practice — no need to document in CLAUDE.md unless planner finds the pattern worth surfacing.

  **Verification:** all 3 e2e tests pass via `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` (note: `--test` targets the integration binary, not the lib's `#[cfg(test)]` module).

### CLEANUP-01 — Strip planning-artifact references from source comments

- **D-04:** **Hybrid: strip + reread + repair.** Two-pass process:
  - **Pass 1:** Delete every GSD label prefix from comments. Targets the grep set: `CONTEXT.md`, `RESEARCH.md`, `RESEARCH §N`, `Plan NN-MM`, `D-NN`, `Pitfall N`, `Pattern N`, `Phase N` references that exist *as labels* (not as descriptive text). 128 hits across new SP source files.
  - **Pass 2:** Read each surviving comment.
    - If it reads as standalone technical content → keep.
    - If it reads as orphaned ("per the original plan, …", "see RESEARCH for rationale") → rewrite to cite the CHIP spec section (`CHIP §425`), BIP-352, or describe the constraint directly. Or delete the comment entirely if the code is self-evident.
    - SDK-internal invariants with no spec → use plain English ("This must be a synthetic SK because StandardArgs currys the synthetic key").

  **Acceptance grep:** `grep -rE 'CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-6]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]'` over the SP source set returns 0 hits.

  **Not in scope:** Planning artifacts in `.planning/phases/**/*-PLAN.md` / `*-CONTEXT.md` / `*-RESEARCH.md` / etc. — those documents are the planning artifacts; they correctly reference each other. Only the *production code comments* get stripped.

### CLEANUP-06 — VALIDATION.md frontmatter flip + `phase complete` CLI patch

- **D-05:** **Both — flip stale files AND patch the CLI to auto-flip going forward.**
  - **Part A:** Manually flip frontmatter on the 7 stale VALIDATION.md files (Phase 1, 2, 3, 4, 4.1, 4.2, 6): set `nyquist_compliant: true` and `wave_0_complete: true`. Phase 5 is already correct.
  - **Part B:** Patch the gsd-tools `phase complete` command (or the `execute-phase.md` workflow caller) so future phases auto-flip these flags when their VERIFICATION.md reports `status: passed`. Specific patch location TBD — planner inspects `$HOME/.claude/get-shit-done/bin/gsd-tools.cjs` to identify the right hook.

  **Why both:** Part A closes the existing audit-cleanliness gap. Part B prevents future drift, so Phase 8+ phases don't accumulate the same template debt. Doing Part A without Part B leaves the same bug in place.

  **Verification:** `grep -l 'nyquist_compliant: true' .planning/phases/*/*-VALIDATION.md | wc -l` returns 8 (one per phase). For Part B: a regression test or direct inspection of the modified CLI confirms the flip behavior.

### Claude's Discretion

- **Phase 7 task ordering and atomicity.** Planner picks wave assignments. Natural ordering suggested: CLEANUP-01 (comment hygiene, low coupling) → CLEANUP-02 (extract SP arm, no behavior change) → CLEANUP-03 (push `sp_finish_branch` into `prepare`, audit callsites) → CLEANUP-04 (relocate e2e tests) → CLEANUP-06 (frontmatter flips + CLI patch). Each can probably be its own plan; CLEANUP-03 might benefit from a Wave 0 audit task before the actual change.
- **Test continuity gate.** Every CLEANUP-* commit must keep the full workspace test suite green (2335 driver + 26 + 24 + 32 + 7 + 2 Rust + 52 napi + 2 pyo3 + 8 wasm). This is a phase-wide invariant, not a per-task decision.
- **Whether to add a regression test for CLEANUP-06 Part B.** Planner decides whether the CLI patch ships with a test or whether direct code inspection is sufficient. Most gsd-tools changes don't have tests; consistency argues for inspection-only.
- **The exact pub(crate) function name CLEANUP-02 extracts.** `spend_silent_payment`, `apply_silent_payment_send`, `silent_payment_send_arm` — planner picks per the existing internal-fn naming patterns in `actions/`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Code review findings (the source of this phase's scope)
- Inline conversation review on 2026-05-19 — captured in this CONTEXT.md's decisions above. No external doc.

### Existing repo conventions to mirror
- `CLAUDE.md` (project root) — Rust 1.90.0, edition 2024, workspace lint policy (`deny clippy::all`, `warn pedantic`, `deny unsafe_code`, `deny dead_code`, `cargo machete`), per-crate `--all-features` CI matrix
- `crates/chia-sdk-driver/src/actions/` — flat sibling convention (mint_nft.rs, issue_cat.rs, etc.) that CLEANUP-02 must follow
- `crates/chia-sdk-driver/src/primitives/cat.rs` — inline-tests convention (8 `#[test]` blocks) that confirms CLEANUP-05's drop

### Phase 6 artifacts that defined what to clean up
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-SUMMARY.md`-equivalent reasoning lives in this phase's CONTEXT.md (a maintainer-style review, not a separate review doc)
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` — the inlined helper that CLEANUP-04 deletes (module rustdoc explains the Cargo cycle)
- `crates/chia-sdk-driver/src/action_system/spends.rs:151` — `pub fn finish_silent_payments` (CLEANUP-03 target)
- `crates/chia-sdk-driver/src/action_system/spends.rs:516,563,625` — `Spends::prepare`, `finish_with_keys`, `sp_finish_branch` (the three sites involved in CLEANUP-03 refactor)
- `crates/chia-sdk-bindings/src/action_system.rs:191` — the binding-side call to `finish_silent_payments` that gets deleted in CLEANUP-03

### Stale VALIDATION.md files for CLEANUP-06
- `.planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md`
- `.planning/phases/02-address-key-types/02-VALIDATION.md`
- `.planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md`
- `.planning/phases/04-send-side-action/04-VALIDATION.md`
- `.planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md`
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md`
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md`
- (Phase 5's `05-VALIDATION.md` is already `nyquist_compliant: true` — the only correct one)

### gsd-tools CLI for CLEANUP-06 Part B
- `$HOME/.claude/get-shit-done/bin/gsd-tools.cjs` — `phase complete` subcommand entry point
- `$HOME/.claude/get-shit-done/workflows/execute-phase.md` — `update_roadmap` step (the caller)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Flat-sibling convention in `crates/chia-sdk-driver/src/actions/`** — 11 existing flat siblings (`create_did.rs`, `fee.rs`, `issue_cat.rs`, `melt_singleton.rs`, `mint_nft.rs`, `mint_option.rs`, `run_tail.rs`, `send.rs`, `settle.rs`, `update_did.rs`, `update_nft.rs`). CLEANUP-02 adds the 12th.
- **`pub(crate)` action-arm helper pattern** — every `actions/*.rs` file exposes a `pub(crate) fn` that `action.rs::Action::spend` matches into. CLEANUP-02's extracted SP arm follows this exactly.
- **`sp_finish_branch` private free function at `action_system/spends.rs:625`** — already correctly scoped (file-private). CLEANUP-03 doesn't need to change its visibility; it just changes who calls it (everyone via `prepare`, instead of `finish_with_keys` and `finish_silent_payments`).
- **Integration-test target pattern** — no `tests/` directory in `chia-sdk-driver` today, but conventional Rust; CLEANUP-04 introduces it.

### Established Patterns
- **Inline `#[cfg(test)] mod tests` at file bottom** — across CAT, NFT, Singleton, Vault. CLEANUP-05 dropped after confirming this. SP scanner.rs (733 lines, 9 inline tests) already follows.
- **`#[from]` error conversion in `DriverError`** — the existing `DriverError::SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError)` line. CLEANUP-03's refactor doesn't add any new error variants.
- **No backwards-compat shims** (per CLAUDE.md) — when CLEANUP-03 deletes `Spends::finish_silent_payments`, no `#[deprecated]` shim is added. The method is gone; callers update. This is consistent with how Phase 4.2 deleted `Action::silent_payment_send` outright.

### Integration Points
- **`crates/chia-sdk-driver/src/action_system.rs`** — currently declares `mod spends;` and re-exports `Spends`. Unchanged by Phase 7.
- **`crates/chia-sdk-driver/src/actions.rs`** — currently declares `mod send;`. CLEANUP-02 adds `mod silent_payment_send;` next to it.
- **`crates/chia-sdk-bindings/src/action_system.rs:191`** — explicit `finish_silent_payments` callsite that CLEANUP-03 removes.
- **`.github/workflows/rust.yml`** — no changes expected (CLEANUP-04's integration test target picks up via the existing `cargo test --workspace --all-features` line).
- **gsd-tools CLI** — CLEANUP-06 Part B modifies `phase complete` to auto-flip VALIDATION.md frontmatter. Hook location TBD.

</code_context>

<specifics>
## Specific Ideas

- **CLEANUP-02 module declaration in `actions.rs`** — mirror the exact `mod send;` shape: `#[cfg(feature = "chip-0057")] mod silent_payment_send;` if the SP arm is feature-gated (it must be, since it depends on `SilentPaymentAddress`). Re-export only if the existing `pub(crate)` send arm is re-exported (it isn't — internal to action.rs).
- **CLEANUP-03 ordering inside `prepare`** — `sp_finish_branch` runs BEFORE the rest of prepare's body. Mirrors the current `finish_with_keys` ordering at `spends.rs:575`. Don't change ordering — it's load-bearing (the SP-emitted `CreateCoin` conditions must land on parents' `payment_assertions` before `emit_conditions` fires inside `prepare`).
- **CLEANUP-04 integration-test target naming** — match the integration test naming for other crates if any exist (`chia-sdk-test` itself, `chia-sdk-utils`, etc.). Recommend `silent_payments_e2e.rs` for clarity; planner picks if a different name fits better.
- **CLEANUP-01 doc-comment vs line-comment distinction** — production rustdoc (`///`) carries forward to docs.rs and IDEs; line comments (`//`) are read-by-developers-only. Both get the strip-reread-repair treatment, but the bar for "is this comment carrying value" is higher for `///` (must be useful as published documentation).
- **CLEANUP-06 the regression-test question** — gsd-tools likely doesn't have tests. Direct code inspection + manual verification (run phase complete on a test phase, check the flag flip) is probably sufficient. Planner decides.

</specifics>

<deferred>
## Deferred Ideas

- **CLEANUP-05 (split scanner.rs tests)** — dropped from Phase 7 scope. Existing inline-tests convention is universal across chia-sdk-driver; one file violating it would set a worse precedent than the file size itself. If file size becomes a maintenance pain later, revisit as a v1.1 cleanup with a broader "split large test modules" rule. Removed from REQUIREMENTS.md and ROADMAP.md as part of this phase's first task.
- **Document the `tests/` integration-target convention in CLAUDE.md** — considered for CLEANUP-04 option (C). Decided not worth the scope creep; conventional Rust practice doesn't need explicit project documentation. Reconsider if a second integration-test-target file emerges later.
- **Add `#[deprecated]` shims for removed methods** — explicitly NOT done. CLAUDE.md forbids backwards-compat shims. Both `Spends::finish_silent_payments` (CLEANUP-03) and the inline `e2e.rs::build_tweak_data` (CLEANUP-04) get deleted outright.
- **Maintainer-approval gate** — not part of Phase 7. The phase closes when all 5 CLEANUP-* requirements are `[x]` and the verifier reports `status: passed`. Upstream merge / maintainer review is a downstream concern (not GSD-managed).
- **Other v1.1 polish items** — anything else the maintainer might raise in actual upstream review (bikeshedding on names, doc style, etc.) goes to a v1.1 backlog. This phase only addresses the 5 explicit concerns from the 2026-05-19 review.

</deferred>

---

*Phase: 07-code-review-cleanup*
*Context gathered: 2026-05-20*
