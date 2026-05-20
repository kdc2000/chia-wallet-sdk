---
phase: 7
slug: code-review-cleanup
researched: 2026-05-20
domain: rust-refactor / comment-hygiene / gsd-tooling
confidence: HIGH
---

# Phase 7: Code review cleanup — Research

**Researched:** 2026-05-20
**Domain:** Rust refactor + comment hygiene + gsd-tools CLI patch (no behavior change)
**Confidence:** HIGH — All five cleanup targets investigated against current source; locked CONTEXT.md decisions enable prescriptive plans.

## Summary

Phase 7 polishes the CHIP-0057 silent-payments v1 surface ahead of upstream merge. It is **pure refactor + comment hygiene + one small CLI process fix** — zero behavior change. Five cleanup targets (CLEANUP-01..04, -06) with locked execution paths from `07-CONTEXT.md`:

1. **CLEANUP-01** — strip 63 grep hits across 18 files (the `128 hits` figure in CONTEXT.md was the count BEFORE pre-CLEANUP-01 work shrank some early planning-artifact references; current count is 63 against the canonical 7-target grep set, but downstream consumers should re-run the grep at execution time as the authoritative number).
2. **CLEANUP-02** — extract chip-0057 SP arm of `Action::send` from `actions/send.rs` (1080 lines) to a flat-sibling `actions/silent_payment_send.rs`. **Note:** the SP arm in `send.rs` is small (≈90 LOC of helpers + 9 LOC dispatch); `send.rs` is 1080 lines mostly because of test bodies (lines 217–1080). The pure-source half is ≈215 lines; extraction shrinks `send.rs`'s source half closer to ≈125 lines.
3. **CLEANUP-03** — push `sp_finish_branch` invocation into `Spends::prepare`; delete `Spends::finish_silent_payments` (`spends.rs:150–160`) and the binding-side caller at `chia-sdk-bindings/src/action_system.rs:191`.
4. **CLEANUP-04** — relocate `silent_payments/e2e.rs` to `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` integration target. Requires adding `chia-sdk-test/chip-0057` to `chia-sdk-driver`'s chip-0057 feature, so the cross-crate `tweak_data_from_simulator_block` helper compiles into the same dependency graph as the integration tests.
5. **CLEANUP-06** — flip 7 stale VALIDATION.md frontmatter files (Phase 1, 2, 3, 4, 4.1, 4.2, 6) to `nyquist_compliant: true` + `wave_0_complete: true`; patch `gsd-tools.cjs phase complete` (entry: `lib/phase.cjs:cmdPhaseComplete`) to auto-flip these flags going forward.

**Primary recommendation:** Sequence is CLEANUP-01 → CLEANUP-02 → CLEANUP-03 → CLEANUP-04 → CLEANUP-06 per CONTEXT.md's natural-ordering note. Each can be its own plan/wave; CLEANUP-03 benefits from a Wave 0 callsite-audit task (4 callsites to enumerate) before refactoring.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (CLEANUP-02 target):** chip-0057 arm of `Action::send` moves to `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (flat sibling). Module declared as `mod silent_payment_send;` in `actions.rs` with a `pub(crate)` function that `send.rs`'s SP match arm calls. `actions/send.rs` ends ≤ 600 lines.
- **D-02 (CLEANUP-03 approach):** Push `sp_finish_branch` invocation into `Spends::prepare` itself. Delete public `finish_silent_payments`. Delete binding-side `spends.finish_silent_payments(&mut ctx, Relation::None)?;` line at `chia-sdk-bindings/src/action_system.rs:191`.
- **D-03 (CLEANUP-04 relocation):** Relocate 3 e2e tests to `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` integration target. Tests call canonical `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` directly. Delete inlined `build_tweak_data` helper. Delete `crates/chia-sdk-driver/src/silent_payments/e2e.rs` + the `#[cfg(test)] mod e2e;` declaration in `silent_payments/mod.rs:47`.
- **D-04 (CLEANUP-01 method):** Hybrid strip-reread-repair pass. Pass 1 deletes GSD label prefixes. Pass 2 rereads each comment; rewrites orphans to cite CHIP spec / BIP-352, plain English, or deletes if self-evident.
- **D-05 (CLEANUP-06 split):** Both manual frontmatter flips on 7 stale files AND patch `gsd-tools phase complete` to auto-flip going forward.

### Claude's Discretion

- Phase 7 task ordering and atomicity (suggested: CLEANUP-01 → 02 → 03 → 04 → 06).
- Whether to add a regression test for CLEANUP-06 Part B (gsd-tools changes typically don't have tests; inspection sufficient).
- The exact `pub(crate)` function name CLEANUP-02 extracts (e.g. `spend_silent_payment`, `apply_silent_payment_send`, `silent_payment_send_arm`).

### Deferred Ideas (OUT OF SCOPE)

- **CLEANUP-05 (split scanner.rs tests)** — DROPPED. Inline `#[cfg(test)] mod tests {}` IS the universal pattern in `chia-sdk-driver`.
- Documenting the `tests/` integration-target convention in CLAUDE.md.
- Adding `#[deprecated]` shims for removed methods.
- Maintainer-approval gate.
- Other v1.1 polish items.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLEANUP-01 | Source comments contain zero references to GSD planning artifacts. Enforced by 7-target grep returning 0 hits. | §CLEANUP-01 below enumerates per-file hit counts (current: 63 hits across 18 files) and classifies rustdoc vs line comments. |
| CLEANUP-02 | Extract chip-0057 SP arm of `Action::send` to a flat-sibling file; `actions/send.rs` ≤ 600 lines (source half ≤ ~125 lines). | §CLEANUP-02 below gives exact line ranges, signatures, imports, and the cfg-gate strategy. |
| CLEANUP-03 | Remove or hide `Spends::finish_silent_payments`. Bindings reach SP finish via a less-leaky mechanism. | §CLEANUP-03 below enumerates all 4 `Spends::prepare` callsites, the line ranges of `finish_with_keys` / `sp_finish_branch`, and the ordering constraint. |
| CLEANUP-04 | Remove `silent_payments/e2e.rs`'s inlined `build_tweak_data`; tests call canonical helper, relocating to `tests/silent_payments_e2e.rs` if needed. | §CLEANUP-04 below describes the cyclic-dev-dep mechanic, the 3 tests, and the required Cargo.toml feature addition. |
| CLEANUP-06 | All 8 prior phases' VALIDATION.md frontmatter has `nyquist_compliant: true` and `wave_0_complete: true`; `phase complete` CLI auto-flips going forward. | §CLEANUP-06 below identifies the exact patch site (`lib/phase.cjs:cmdPhaseComplete` after line 869) and the frontmatter API to use. |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Rust 1.90.0** pinned via `rust-toolchain.toml`, edition 2024.
- **Workspace lint policy:** `deny clippy::all`, `warn pedantic`, `warn cargo`, `deny unsafe_code`, `deny dead_code`. Phase 7 must not introduce new `#[allow(...)]` attributes.
- **`cargo machete` (unused-dep)** must remain clean. No new entries in `[package.metadata.cargo-machete] ignored`.
- **CI builds each crate individually** with and without `--all-features`. New code behind `chip-0057` must compile in every permutation (without features = excluded; with chip-0057 = compiles; with all-features = compiles).
- **`cargo fmt --all -- --files-with-diff --check`** must pass.
- **No `unsafe` blocks** — does not apply here (refactor).
- **No backwards-compat shims** — when CLEANUP-03 deletes `Spends::finish_silent_payments`, NO `#[deprecated]` wrapper. Direct deletion is the convention (Phase 4.2 precedent).
- **LSP-first navigation** — for code investigation prefer `workspaceSymbol`/`findReferences` over `grep`.
- **GSD workflow enforcement** — phase work goes through GSD commands; no direct edits outside workflow.
- **Run clippy after edits** — `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets` should be re-runnable locally per plan.

## Standard Stack

No new dependencies. This phase is a pure refactor + comment edit + small JS CLI patch.

### Core (already in workspace, used unchanged)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| chia-sdk-driver | 0.33.0 (workspace) | Driver, action system, silent_payments module | Crate being refactored |
| chia-sdk-test | 0.33.0 (workspace, chip-0057) | `tweak_data_from_simulator_block` helper | Already wired; CLEANUP-04 just makes it actually-reachable from chia-sdk-driver integration tests |
| chia-sdk-bindings | 0.33.0 (workspace) | Binding facade | One-line edit in CLEANUP-03 |
| Node.js stdlib (`fs`, `path`) + `lib/frontmatter.cjs` | — | gsd-tools VALIDATION.md flip | Built-in YAML serializer at `frontmatter.cjs:90` (`reconstructFrontmatter`); no `js-yaml` dep |

### Supporting
None.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CLEANUP-03 push-into-prepare | `#[doc(hidden)]` shim (CONTEXT.md option c) | LOCKED OUT per D-02: the leak isn't just a doc concern; doc-hiding leaves symbol reachable |
| CLEANUP-04 integration-target relocation | In-place fix attempt first (CONTEXT.md option a) | LOCKED OUT per D-03: cyclic-dev-dep is well-known Cargo behavior, going straight to integration target avoids a wasted cycle |
| CLEANUP-06 regression test | Direct inspection only | Discretionary; CONTEXT.md "Specifics" recommends inspection-only (gsd-tools has no test suite) |

**Installation:** N/A — no new deps.

**Version verification:** N/A — no new packages.

## Architecture Patterns

### Flat-Sibling `actions/*.rs` Convention
- `crates/chia-sdk-driver/src/actions/` contains 11 flat siblings today:
  - `create_did.rs`, `fee.rs`, `issue_cat.rs`, `melt_singleton.rs`, `mint_nft.rs`, `mint_option.rs`, `run_tail.rs`, `send.rs`, `settle.rs`, `update_did.rs`, `update_nft.rs`
- `actions.rs` (23 lines) declares each as `mod X;` and `pub use X::*;`.
- CLEANUP-02 inserts the 12th: `mod silent_payment_send;` (chip-0057 gated). Per CONTEXT.md `<specifics>`: declare as `#[cfg(feature = "chip-0057")] mod silent_payment_send;` since the SP code depends on `SilentPaymentAddress`. Do NOT re-export via `pub use silent_payment_send::*;` — the public surface stays `Action::send(...)` only; the new module's `pub(crate) fn` is internal.
- Verb-noun naming: `mint_nft.rs`, `issue_cat.rs`, `create_did.rs`. The recommended pub(crate) fn name in `silent_payment_send.rs` is `spend_silent_payment` (already used as the helper name inside `send.rs:55` and `send.rs:136`). `apply_silent_payment_send` is also plausible. Planner picks.

### Integration-Test Target Convention (`tests/*.rs`)
- No `crates/*/tests/` directories exist in this workspace today (verified via `find`). CLEANUP-04 introduces the first.
- Conventional Rust pattern: any `.rs` file directly under `crates/<crate>/tests/` becomes an integration test binary. Cargo auto-discovers; no `[[test]]` block needed in Cargo.toml.
- Integration tests compile as their own crate, importing `chia_sdk_driver` as a regular external dep — breaks the dev-dep cycle (`chia-sdk-driver → chia-sdk-test → chia-sdk-driver`) that confuses type identity inside `src/silent_payments/e2e.rs`.
- Standard invocation: `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e`.
- The CI line `cargo test --release --workspace --all-features ...` picks up `tests/*.rs` automatically.

### Pattern 1: Push-Into-Prepare (CLEANUP-03)
**What:** Move `sp_finish_branch` invocation from caller-must-remember to one always-runs site (`Spends::prepare`), then delete the helper that taught the bindings layer the wrong invariant.

**Why:** The current design (`finish_silent_payments` public + `prepare` doesn't run SP branch) requires every binding-side caller to know "if you used `prepare` instead of `finish_with_keys`, you must also call `finish_silent_payments` before". That invariant is undocumented in the type system. Pushing into `prepare` makes the invariant unforgeable: any caller that completes a spend goes through `prepare`, and `prepare` runs the SP branch when (and only when) there are pending SP outputs.

**Example:** Existing pattern at `spends.rs:573–576` inside `finish_with_keys`:
```rust
// chip-0057 SP derivation branch — runs BEFORE prepare() so CreateCoin
// emissions feed into the parents' payment_assertions before
// emit_conditions.
#[cfg(feature = "chip-0057")]
if !self.silent_payments_pending.is_empty() {
    sp_finish_branch(ctx, &mut self, relation)?;
}

let spends = self.prepare(ctx, deltas, relation)?;
```

This identical block migrates into `prepare()` itself (at the top, before `create_change`). `finish_with_keys` then loses the inline copy. Binding-side `prepare` no longer needs the separate call.

### Pattern 2: Strip-Reread-Repair (CLEANUP-01)
**What:** Two-pass comment edit. Pass 1 deletes mechanical GSD-label substrings; Pass 2 reads each surviving comment in context and either keeps, rewrites, or deletes it.

**Why:** A pure-mechanical strip would leave orphaned references ("per the original plan, X" with no antecedent). A pure-judgmental rewrite is too expensive on 63 hits. Hybrid front-loads the cheap deletions and reserves judgment for the harder cases.

**Bar for keep/rewrite/delete:**
- **Keep** if comment reads standalone after label deletion (e.g. `// Per CHIP §459 …` after removing `Plan 03-03:`).
- **Rewrite** if comment was substantively tied to GSD doc ("see RESEARCH §10a for pinned bytes") — cite CHIP spec, BIP-352, or restate the constraint directly. Or delete if the surrounding code is self-evident.
- **Delete** if the comment was pure scaffolding meta ("// PLAN 03-04 APPEND POINT").

### Anti-Patterns to Avoid
- **Touching planning artifacts** — `.planning/phases/**/*-PLAN.md`, `*-CONTEXT.md`, `*-RESEARCH.md`, `*-SUMMARY.md` correctly reference each other and are OUT OF CLEANUP-01 SCOPE. Only production code comments get stripped.
- **Hand-editing `napi/index.d.ts` or `napi/index.js`** — these are generated by `napi build`; never edit by hand (per project CLAUDE.md).
- **Touching `crates/chia-sdk-driver/src/silent_payments/scanner.rs`'s inline test bodies** (733 lines, 9 tests) — splitting them was CLEANUP-05, which is DROPPED. The 14 grep hits in scanner.rs are comment-only; CLEANUP-01 cleans them in place.
- **Adding `#[deprecated]` shims** when CLEANUP-03 deletes `Spends::finish_silent_payments` — CLAUDE.md forbids backwards-compat shims; Phase 4.2 deleted `Action::silent_payment_send` outright with no shim.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML frontmatter parse/write for CLEANUP-06 CLI patch | A custom regex-based YAML writer | `lib/frontmatter.cjs`'s `extractFrontmatter` + `spliceFrontmatter` (already used by `gsd-tools frontmatter merge`) | Built-in. The same module is loaded at `gsd-tools.cjs:151`. Use `cmdFrontmatterMerge` shape internally. |
| `TweakData` extraction from a simulator block | Re-inlining or extracting another local copy | Call `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` directly | Canonical helper exists at `crates/chia-sdk-test/src/silent_payments/tweak_data.rs:41`; the cyclic-dev-dep blocker is exactly what relocating to `tests/` fixes |
| SP-branch sequencing | Re-deriving the gate ordering | The existing `sp_finish_branch` private function at `spends.rs:625` | All gate ordering (Pitfall 7 from CONTEXT.md) already correct |
| Per-phase VALIDATION.md flip | A shell `sed` script | The `frontmatter.cjs` module via a small node script invoked from `phase complete` | Idempotent, preserves frontmatter ordering, handles edge cases |

**Key insight:** Three of the five CLEANUPs are *deletions* (CLEANUP-03 deletes a method + a binding line, CLEANUP-04 deletes a helper + an entire test module). The discipline is to *remove* code, not add it — keep the diffs subtractive.

## CLEANUP-01 — Strip planning-artifact references from source comments

### Acceptance grep (canonical)
```bash
grep -rE 'CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-6]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]' \
  crates/*/src/silent_payments/ \
  crates/chia-sdk-bindings/src/silent_payments.rs \
  crates/chia-sdk-driver/src/action_system/send_destination.rs \
  examples/silent_payment.rs \
  napi/__test__/silent_payments*.ts \
  pyo3/tests/test_silent_payments.py \
  wasm/__test__/silent_payments.spec.ts
```
Must return **0 hits** at completion.

### Current per-file hit counts (2026-05-20, total 63 across 18 files)

| File | Hits | Doc/Line | Notes |
|------|------|----------|-------|
| `crates/chia-sdk-driver/src/silent_payments/scanner.rs` | 14 | mixed | Plan 03-04 refs in rustdoc + `RESEARCH §10a/§10c/§10d/§10e` test pin labels |
| `crates/chia-sdk-bindings/src/silent_payments.rs` | 9 | mixed | `CONTEXT.md D-01/D-02/D-03`, `RESEARCH Pattern 4`, `Pitfall 4` |
| `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` | 4 | mixed | `Plan 04-04`, `04-RESEARCH.md Section 5`, `Plan 04-03` |
| `crates/chia-sdk-driver/src/silent_payments/e2e.rs` | 4 | mixed | `06-RESEARCH §3b`, `Plan 06-02/05/04`, `D-04` — entire file deleted in CLEANUP-04 so these self-resolve if CLEANUP-04 lands first |
| `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` | 4 | mixed | `04-RESEARCH.md §11`, `Plan 04-03`, `Pitfall A`, `Pitfall H` |
| `wasm/__test__/silent_payments.spec.ts` | 3 | line | TS test comments |
| `pyo3/tests/test_silent_payments.py` | 3 | line | Python test comments |
| `napi/__test__/silent_payments.spec.ts` | 3 | line | TS test comments |
| `crates/chia-sdk-utils/src/silent_payments/keys.rs` | 3 | mixed | |
| `crates/chia-sdk-driver/src/silent_payments/one_time.rs` | 3 | mixed | |
| `napi/__test__/silent_payments_e2e.spec.ts` | 2 | line | |
| `examples/silent_payment.rs` | 2 | line | |
| `crates/chia-sdk-utils/src/silent_payments/labels.rs` | 2 | mixed | |
| `crates/chia-sdk-utils/src/silent_payments/address.rs` | 2 | mixed | |
| `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` | 2 | mixed | |
| `crates/chia-sdk-driver/src/silent_payments/mod.rs` | 2 | rustdoc | Crate-level `//!` doc |
| `crates/chia-sdk-driver/src/silent_payments/types.rs` | 1 | rustdoc | |
| `crates/chia-sdk-driver/src/silent_payments/protocol.rs` | 1 | rustdoc | |

**Discrepancy with CONTEXT.md's "128 hits":** The 128 figure dates from earlier in the session; the current count is 63. Re-run the canonical grep at execution time as the authoritative number.

**Bar for rustdoc (`///`) vs line (`//`) comments:**
- Rustdoc renders to docs.rs and shows in IDE tooltips — the bar for "is this comment carrying value to a public consumer" is higher. Most rustdoc hits will need REWRITING (cite CHIP §, BIP-352, or describe the constraint plainly).
- Line comments are read-by-developers-only — these can usually be either KEPT (after label deletion) or DELETED if the surrounding code is self-evident.

**File ordered by edit cost (lightest to heaviest):**
1. `protocol.rs` / `types.rs` (1 hit each, single-line edits) → easy
2. The TS / Python test files (3 hits each, simple label deletions) → easy
3. `examples/silent_payment.rs`, `*-VALIDATION.md` peers, `address.rs`, `labels.rs`, `send_keys.rs`, `mod.rs`, `keys.rs`, `one_time.rs` (2–3 hits) → moderate
4. `aggregate.rs`, `input_hash.rs` (4 hits, references to Pitfall A/H + plan numbers) → moderate
5. `bindings/silent_payments.rs` (9 hits, lots of `CONTEXT.md D-01/D-02/D-03`) → moderate-heavy
6. `scanner.rs` (14 hits, many test-vector pin labels `RESEARCH §10a/c/d/e`) → heaviest
7. `e2e.rs` (4 hits) — SKIPPED if CLEANUP-04 deletes the file first

**Optimization:** If CLEANUP-04 runs before CLEANUP-01, the 4 hits in `e2e.rs` become moot (file gone). Suggested wave order in CONTEXT.md is 01 → 02 → 03 → 04 → 06, so e2e.rs gets cleaned and then deleted. That's redundant work but not harmful. Planner may flip 04 before 01 to avoid the redundant scrub if preferred — discretionary.

### Concrete rewrite playbook

Examples of mechanical strip + rewrite patterns from `scanner.rs`:

```rust
// BEFORE
// ─── TV1 pinned bytes (RESEARCH §10a) ──────────────────────────────────

// AFTER (rewrite to cite source)
// ─── TV1 pinned bytes (CHIP-0057 test vector 1) ────────────────────────
```

```rust
// BEFORE
//! The labeled-detection branch is added in Plan 03-04 (CHIP §RECV-04 labeled

// AFTER (delete the Plan label, keep the CHIP cite)
//! The labeled-detection branch implements CHIP-0057 §RECV-04 labeled
```

```rust
// BEFORE
// genuinely unavoidable here: Plan 03-03 hard-locks the function signature
// (the `&PublicKey, &SecretKey` parameter pair triggers clippy::similar_names)
// and Plan 03-04 references both names directly in the labeled-detection
// branch, so a local rebinding would break the locked signature or break
// Plan 03-04's structural expectations.

// AFTER (restate the technical constraint without the Plan numbers)
// genuinely unavoidable here: the function signature's `&PublicKey, &SecretKey`
// parameter pair triggers clippy::similar_names, and the labeled-detection
// branch below references both names directly — local rebinding would either
// break the signature or break the labeled-branch's structural expectations.
```

## CLEANUP-02 — Extract chip-0057 SP arm of `Action::send`

### Mechanics

**File sizes / line ranges (verified 2026-05-20):**
- `crates/chia-sdk-driver/src/actions/send.rs` — **1080 lines total**
  - Source half: lines 1–215 (the SP arm + helpers consume lines 5–13 imports + 49–56 dispatch + 123–214 helpers)
  - Test half: lines 216–494 (`mod tests`) + lines 496–1080 (`mod silent_payment_tests`, chip-0057 gated)
- `crates/chia-sdk-driver/src/actions.rs` — 23 lines (mod declarations + pub re-exports)
- `crates/chia-sdk-driver/src/action_system/action.rs` — 297 lines (NOT touched by CLEANUP-02; the SP arm extraction is from `SendAction::spend` in `send.rs`, not from `Action::send`'s constructor)

**What moves to `actions/silent_payment_send.rs`:**

1. The chip-0057 dispatch block at `send.rs:49–56` becomes a single `pub(crate) fn` call from `SendAction::spend`:
   ```rust
   // send.rs (NEW)
   #[cfg(feature = "chip-0057")]
   if let SendDestination::SilentPayment(addr) = &self.destination {
       return crate::actions::silent_payment_send::handle_silent_payment_send(
           ctx, spends, &self.id, addr, self.amount, self.memos,
       );
   }
   ```
   (function name TBD by planner per CONTEXT.md "Claude's Discretion")

2. The two private helpers `spend_silent_payment` (lines 135–179) and `memo_hint_guard` (lines 192–214), with their rustdoc, move into the new file.

3. The `pub(crate) fn handle_silent_payment_send` (name TBD) in the new file:
   - Takes the Id reference (or owned), the `&SilentPaymentAddress`, amount, and memos.
   - Fires the `Id != Id::Xch` check (`return Err(DriverError::SilentPaymentRequiresXch)`) — moved from `send.rs:51–53`.
   - Calls the relocated `memo_hint_guard`.
   - Calls the relocated `spend_silent_payment`.

**Imports needed in the new `silent_payment_send.rs`:**
```rust
use chia_puzzle_types::Memos;

use chia_sdk_utils::silent_payments::SilentPaymentAddress;

use crate::{
    BURN_PUZZLE_HASH, DriverError, Id, Output, SpendContext, Spends,
    silent_payments::SilentPaymentPending,
};
```
(The `Asset` / `Deltas` / `SendDestination` / `SingletonDestination` / `SpendAction` imports stay in `send.rs` because they're still used by the non-SP arm.)

**Module declaration in `actions.rs`:**
```rust
// Add after line 8 (mod send;):
#[cfg(feature = "chip-0057")]
mod silent_payment_send;
```
Do NOT add a `pub use silent_payment_send::*;` line — the helper is internal-only (`pub(crate)`).

**Feature-gate strategy:** Module-level `#[cfg(feature = "chip-0057")]` on the `mod silent_payment_send;` declaration in `actions.rs`. Per-item `#[cfg]` is NOT needed inside the new file because the whole file is feature-gated at the mod level.

**Resulting `send.rs` shape:**
- Lines 1–14: imports (chip-0057 imports for `SilentPaymentAddress` / `BURN_PUZZLE_HASH` / `SilentPaymentPending` go away from `send.rs`)
- Lines 15–32: `SendAction` struct + impl
- Lines 33–~70: `SpendAction for SendAction` — chip-0057 arm shrinks from 8 lines to ~5 lines (single function-call statement)
- Test bodies unchanged (lines 217–1080).
- New source-half length: ~125 lines. **WELL under D-01's 600-line ceiling** (the 600-line target counts the whole file including test bodies; the test bodies are huge but unrelated to the SP arm). Verify the 600-line bound at execution time including tests.

**Important:** `mod silent_payment_tests` at `send.rs:496–1080` contains 8 relocated SP tests + 2 Wave-0 acceptance tests. These STAY in `send.rs` — they're test bodies, not source code, and they test the public surface `Action::send(..., SendDestination::SilentPayment(...), ...)` which doesn't move. **Do NOT relocate this test module to `silent_payment_send.rs`.** Doing so would violate the no-behavior-change rule by changing test discovery paths.

**Files modified by CLEANUP-02:**
- `crates/chia-sdk-driver/src/actions.rs` (1-line insert)
- `crates/chia-sdk-driver/src/actions/send.rs` (delete 2 helpers + ~90 lines, shrink dispatch arm)
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` (new file, ~110–130 lines)

## CLEANUP-03 — Push `sp_finish_branch` into `Spends::prepare`; delete `finish_silent_payments`

### Mechanics

**Line ranges in `crates/chia-sdk-driver/src/action_system/spends.rs` (968 lines total, verified 2026-05-20):**

| Symbol | Lines | Notes |
|--------|-------|-------|
| `Spends::with_silent_payment_keys` | 112–121 | chip-0057 gated — UNCHANGED |
| `Spends::finish_silent_payments` | 150–160 | **DELETE** (incl. its rustdoc lines 123–149) |
| `Spends::apply` | 162–172 | UNCHANGED |
| `Spends::create_change` | 174–222 | UNCHANGED |
| `Spends::emit_conditions` | 311–408 | UNCHANGED |
| `Spends::emit_relation` | 410–436 | UNCHANGED |
| `Spends::prepare` | 516–546 | **MODIFY**: insert chip-0057 SP branch block at top of method body, before `self.create_change(...)` |
| `Spends::finish_with_keys` | 563–606 | **MODIFY**: delete the inline SP branch block at lines 573–576 (now runs inside `prepare`) |
| `sp_finish_branch` (private free fn) | 624–722 | UNCHANGED — still file-private, still called the same way |

**Ordering constraint:** the SP branch MUST run BEFORE `create_change` and `emit_conditions`. The current ordering in `finish_with_keys` (lines 573–578) is:
1. `sp_finish_branch(ctx, &mut self, relation)?;` — emits `CreateCoin` conditions onto `xch.payment_assertions`
2. `self.prepare(ctx, deltas, relation)?;` — runs `create_change` → `emit_conditions` → `emit_relation`

After the refactor, `prepare()` runs sp-branch first internally:
```rust
pub fn prepare(
    mut self,
    ctx: &mut SpendContext,
    deltas: &Deltas,
    relation: Relation,
) -> Result<Spends<Finished>, DriverError> {
    // chip-0057 SP derivation branch — runs FIRST so CreateCoin emissions
    // feed into the parents' payment_assertions before emit_conditions.
    #[cfg(feature = "chip-0057")]
    if !self.silent_payments_pending.is_empty() {
        sp_finish_branch(ctx, &mut self, relation)?;
    }

    self.create_change(ctx, deltas)?;
    self.emit_conditions(ctx)?;
    self.emit_relation(relation);

    // ... existing Spends<Finished> construction unchanged ...
}
```

`finish_with_keys` becomes:
```rust
pub fn finish_with_keys(
    self,                       // mut no longer needed
    ctx: &mut SpendContext,
    deltas: &Deltas,
    relation: Relation,
    synthetic_keys: &IndexMap<Bytes32, PublicKey>,
) -> Result<Outputs, DriverError> {
    // SP branch now runs inside prepare().
    let spends = self.prepare(ctx, deltas, relation)?;
    let mut coin_spends = HashMap::new();
    // ... rest unchanged ...
}
```

The `#[cfg_attr(not(feature = "chip-0057"), allow(unused_mut))] mut self,` parameter annotation at `spends.rs:564` becomes unnecessary and is **deleted**. `finish_with_keys` no longer needs `mut self`. Verify with clippy after the change.

### `Spends::prepare` callsites — full audit

Workspace-wide search (verified 2026-05-20):

| # | File | Line | Context | Impact of pushing sp_finish_branch into prepare |
|---|------|------|---------|--------------------------------------------------|
| 1 | `crates/chia-sdk-driver/src/action_system/spends.rs` | 578 | Inside `Spends::finish_with_keys` | Calls itself; existing flow. After refactor: the inline sp-branch lines 573–576 are deleted. |
| 2 | `crates/chia-sdk-driver/src/action_system/spends.rs` | 908 | Inside `#[cfg(test)] mod tests::assert_concurrent_cycle_for_n` | Test of `Relation::AssertConcurrent` cycle emission. No SP destination used → `silent_payments_pending` empty → SP branch is no-op. Test continues to pass unchanged. |
| 3 | `crates/chia-sdk-driver/src/test_wallet.rs` | 147 | Inside `#[cfg(test)] mod test_wallet` test-utility module | No SP destination used in the tests this module supports → SP branch no-op. Unchanged behavior. |
| 4 | `crates/chia-sdk-bindings/src/action_system.rs` | 193 | Inside `Spends::prepare` (binding wrapper) | **PRIMARY TARGET.** Line 191's `spends.finish_silent_payments(&mut ctx, Relation::None)?;` is DELETED. The `prepare` call at line 193 now runs the SP branch internally. Net effect: zero behavior change, one fewer leak. |

**Conclusion of audit:** All 4 callsites are safe. The SP branch is no-op when `silent_payments_pending` is empty (CONTEXT.md D-02 invariant). The binding-side caller at action_system.rs:191 is the only one currently relying on the explicit call — its line is the only deletion at a callsite.

### `sp_finish_branch` signature (UNCHANGED)
```rust
#[cfg(feature = "chip-0057")]
fn sp_finish_branch(
    ctx: &mut SpendContext,
    spends: &mut Spends,
    relation: Relation,
) -> Result<(), DriverError>
```
- Takes `&mut Spends<Unfinished>` (the parameter type is `Spends`, defaulting to `Unfinished`).
- Mutates `spends.silent_payments_pending` (via `std::mem::take`), `spends.xch.items[i].kind.create_coin_with_assertion(...)`, and `spends.outputs.xch.push(...)`.
- Returns 4 error variants on the gate path: `SilentPaymentRequiresInputBinding`, `SilentPaymentKeysNotRegistered`, `SilentPaymentMultiPartyUnsupported`, `SilentPaymentNoXchInputs`.

Pushing into `prepare` doesn't change the function's mutability requirements. `prepare`'s `mut self` already covers it.

### Binding-side cleanup (the one extra deletion)

`crates/chia-sdk-bindings/src/action_system.rs:177–217` is the binding's `Spends::prepare` method. After CLEANUP-03:

```rust
pub fn prepare(&self, deltas: Deltas) -> Result<FinishedSpends> {
    let mut spends = self.spends.lock().unwrap();
    let change_puzzle_hash = spends.change_puzzle_hash;
    let mut spends = std::mem::replace(&mut *spends, sdk::Spends::new(change_puzzle_hash));
    let mut ctx = self.clvm.lock().unwrap();

    // DELETE these 4 lines (CLEANUP-03):
    //   // Run the chip-0057 silent-payment finish branch BEFORE prepare()
    //   // (mirrors `sdk::Spends::finish_with_keys` ordering ...).
    //   // No-op when no `Action::send(SilentPayment, ...)` has been applied.
    //   spends.finish_silent_payments(&mut ctx, Relation::None)?;

    let spends = spends.prepare(&mut ctx, &deltas.0, Relation::None)?;
    // ... rest unchanged ...
}
```

Also delete the rustdoc reference at `chia-sdk-bindings/src/action_system.rs:62–75` that documents the now-removed function (or rewrite to point at `Spends::prepare`'s new behavior).

Also delete the unused `use chia_sdk_driver::Relation` if `Relation::None` was its only use — verify after the edit (clippy will flag).

## CLEANUP-04 — Relocate e2e tests to integration target

### Mechanics

**File contents (verified 2026-05-20):** `crates/chia-sdk-driver/src/silent_payments/e2e.rs` is 358 lines with 3 tests:

| Test | Lines | What it tests |
|------|-------|---------------|
| `test_simulator_e2e_unlabeled` | 140–216 | SIM-02: unlabeled SP send → farm → extract → scan → detect → follow-on spend |
| `test_simulator_e2e_labeled` | 221–286 | SIM-03 labeled half: labeled (m=1) SP send → same flow → `label: Some(1)` |
| `test_simulator_e2e_m0_self_change` | 304–358 | SIM-03 m=0 half: registers m=0 in LabelRegistry, asserts unlabeled detection still resolves to `label: None` |

**Inlined helper to delete:** `build_tweak_data(sim: &Simulator, height: u32) -> TweakData` at lines 63–114. Algorithm matches `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` byte-for-byte (verified by side-by-side comparison with `crates/chia-sdk-test/src/silent_payments/tweak_data.rs:41`).

**Canonical helper signature (UNCHANGED):**
```rust
// crates/chia-sdk-test/src/silent_payments/tweak_data.rs:41
#[must_use]
pub fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> TweakData
```
Returns `chia_sdk_driver::silent_payments::TweakData` (re-exported via `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`).

### Cyclic-dev-dep mechanic (for the planner's understanding)

The cycle: `chia-sdk-driver` (lib) declares `chia-sdk-test` as a dev-dep (`crates/chia-sdk-driver/Cargo.toml:57`). `chia-sdk-test` (lib) declares `chia-sdk-driver` as an optional dep behind `chip-0057` (`crates/chia-sdk-test/Cargo.toml:64`).

When `cargo test -p chia-sdk-driver --features chip-0057` runs:
1. Cargo builds **chia-sdk-driver (lib)** as compile unit A.
2. To build the `cfg(test)` portion, Cargo also builds **chia-sdk-test (lib)** as compile unit B, which transitively builds **chia-sdk-driver (lib)** as compile unit C (because chia-sdk-test depends on chia-sdk-driver).
3. Compile units A and C are structurally distinct (same source, different build IDs). A `TweakData` returned from B (using C's types) is NOT type-equivalent to a `TweakData` consumed by A.
4. The `cfg(test) mod e2e;` lives inside compile unit A. Calling B's `tweak_data_from_simulator_block` returns a "wrong type" from A's perspective.

**Integration test target breaks the cycle:**
- `cargo test -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` builds:
  - **chia-sdk-driver (lib)** as compile unit A.
  - **chia-sdk-test (lib)** as compile unit B (depending on chia-sdk-driver-as-A; chia-sdk-test's dep on chia-sdk-driver resolves to A, no second copy).
  - The integration test binary as compile unit T, depending on both A and B.
- All `TweakData` references in T resolve to A's `TweakData`, including the one returned by B's helper. No type confusion.

This is Cargo's documented behavior; the integration target IS the recommended fix for the cycle.

### What CLEANUP-04 changes

**Cargo.toml change required:** `crates/chia-sdk-driver/Cargo.toml` must enable `chia-sdk-test/chip-0057` for the integration test to compile. Add to the `chip-0057` feature line:

```toml
# Current (line 23):
chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]

# After CLEANUP-04:
chip-0057 = [
    "chia-sdk-types/chip-0057",
    "dep:chia-sdk-utils",
    "chia-sdk-utils/chip-0057",
    "chia-sdk-test/chip-0057",
]
```

`chia-sdk-test` is in `[dev-dependencies]`; that's fine — Cargo accepts feature references to dev-deps in feature definitions. The integration test target builds with `chip-0057` enabled, which now activates `chia-sdk-test/chip-0057`, which makes `tweak_data_from_simulator_block` available.

**Alternative considered:** Adding `features = ["chip-0057"]` directly to `chia-sdk-test = { workspace = true }` at `crates/chia-sdk-driver/Cargo.toml:57`. This would always activate it for dev builds (including unit tests). Slightly broader scope but functionally equivalent. Recommendation: **feature-cascade approach** (modify the chip-0057 feature line, not the dev-dep declaration). Cleaner because it follows the existing cascade pattern used in `crates/chia-sdk-driver/Cargo.toml:23` and `crates/chia-sdk-test/Cargo.toml:37`.

**New file `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`:**

Top-of-file feature gate (because the entire test depends on `chip-0057`-only types):
```rust
#![cfg(feature = "chip-0057")]
```

Imports — instead of `crate::silent_payments::...` (which only works inside lib), use full crate paths:
```rust
use chia_sdk_driver::silent_payments::{K_MAX_DEFAULT, scan_from_tweaks};
use chia_sdk_driver::{Action, Id, Relation, SendDestination, SpendContext, Spends, StandardLayer};
use chia_sdk_test::silent_payments::tweak_data_from_simulator_block;
use chia_sdk_test::{BlsPairWithCoin, Simulator};
// ... etc, lifting from e2e.rs imports lines 29–47
```

The 3 test bodies relocate verbatim. Replace every `build_tweak_data(&sim, height_before)` call with `tweak_data_from_simulator_block(&sim, height_before)`. **No other code changes.** The tests' logic is correct as written.

**Cargo configuration:** None beyond the feature-cascade Cargo.toml line above. No `[[test]]` block needed.

**Existing integration-test convention to mirror:** None — no `crates/*/tests/` directories exist today. The convention is just "conventional Rust" — a `tests/<name>.rs` file. Recommended filename: `silent_payments_e2e.rs` per CONTEXT.md `<specifics>` "Recommend `silent_payments_e2e.rs` for clarity".

**File deletions in CLEANUP-04:**
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` — DELETE entirely.
- `crates/chia-sdk-driver/src/silent_payments/mod.rs:46–47`:
  ```rust
  #[cfg(test)]
  mod e2e;
  ```
  → DELETE these two lines (plus the blank-line separator above as appropriate).

### Verification

```bash
cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e
```
Must report 3 tests passed. Note: `--test silent_payments_e2e` targets the integration binary, not the lib's `#[cfg(test)]` module.

The CI line `cargo test --release --workspace --exclude ... --all-features` (from `.github/workflows/rust.yml:75`) auto-discovers the new integration target — no workflow file edit required.

## CLEANUP-06 — VALIDATION.md frontmatter flip + CLI patch

### Part A — Manual flip on 7 stale files

Files (verified 2026-05-20, in roadmap order):

| Phase | File | Current `nyquist_compliant` | Current `wave_0_complete` |
|-------|------|---------------------------|----------------------------|
| 1 | `.planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md` | `false` | `false` |
| 2 | `.planning/phases/02-address-key-types/02-VALIDATION.md` | `false` | `false` |
| 3 | `.planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md` | `false` | `false` |
| 4 | `.planning/phases/04-send-side-action/04-VALIDATION.md` | `false` | `false` |
| 4.1 | `.planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md` | `false` | `false` |
| 4.2 | `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md` | `false` | `false` |
| 5 | `.planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md` | **`true`** | **`true`** | (already correct — skip)
| 6 | `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md` | `false` | `false` |

7 files need flipping. The flip can be done by `gsd-tools.cjs frontmatter merge` per file:
```bash
node "$HOME/.claude/get-shit-done/bin/gsd-tools.cjs" frontmatter merge \
    .planning/phases/<DIR>/<N>-VALIDATION.md \
    --data '{"nyquist_compliant": true, "wave_0_complete": true}'
```
Or edit by hand. Both leave identical output (the CLI is idempotent and preserves frontmatter ordering — `frontmatter.cjs:299–311`).

### Part B — Patch `gsd-tools phase complete` to auto-flip going forward

**Entry-point file:** `$HOME/.claude/get-shit-done/bin/gsd-tools.cjs` (918 lines), command dispatcher only.

**Implementation file:** `$HOME/.claude/get-shit-done/bin/lib/phase.cjs` (889 lines), function `cmdPhaseComplete` at lines 629–877.

**Available helpers in that file's scope:**
- `frontmatter` module imported at `gsd-tools.cjs:151` (`const frontmatter = require('./lib/frontmatter.cjs');`) — exposes `extractFrontmatter`, `spliceFrontmatter`, `cmdFrontmatterMerge`.
- `phase.cjs` already uses `fs.readFileSync` / `fs.writeFileSync` extensively (e.g. line 658, 712, 745, 858) — same file-IO pattern applies.
- `phaseInfo.directory` (line 653) is the phase directory; `phaseInfo.phaseNum` (or just `phaseNum`) is the input phase number.

**Patch site (recommended):** Inside `cmdPhaseComplete`, after the STATE.md write at line 858 (`writeStateMd(statePath, stateContent, cwd);`) and BEFORE the `const result = {...}` block at line 861. This is after all primary side-effects succeed, so a failure to flip the VALIDATION.md doesn't roll back the rest.

**Inputs the patch needs:**
- `phaseInfo.directory` — absolute path to the phase dir
- The VALIDATION.md filename — derivable from `phaseNum`: glob `<phaseDir>/*-VALIDATION.md` and pick the one whose phase prefix matches `phaseNum`.

**Verification trigger:** Per D-05 wording — "auto-flip these flags when their VERIFICATION.md reports `status: passed`". So the patch reads the phase's VERIFICATION.md frontmatter first, checks `status === 'passed'`, and only then flips VALIDATION.md. The existing code at lines 664–668 already reads VERIFICATION.md content to scan for `human_needed` / `gaps_found` — same file-glob pattern applies.

**Proposed pseudocode insertion (after line 858 `writeStateMd(...)`):**
```javascript
// CLEANUP-06 Part B: auto-flip VALIDATION.md frontmatter when VERIFICATION
// reports status: passed. Idempotent — already-true flags are a no-op.
try {
  const phaseFullDir = path.join(cwd, phaseInfo.directory);
  const files = fs.readdirSync(phaseFullDir);
  const verFile = files.find(f => f.includes('-VERIFICATION') && f.endsWith('.md'));
  const valFile = files.find(f => f.includes('-VALIDATION') && f.endsWith('.md'));
  if (verFile && valFile) {
    const verContent = fs.readFileSync(path.join(phaseFullDir, verFile), 'utf-8');
    const verFm = frontmatter.extractFrontmatter(verContent);
    if (verFm.status === 'passed') {
      const valPath = path.join(phaseFullDir, valFile);
      const valContent = fs.readFileSync(valPath, 'utf-8');
      const valFm = frontmatter.extractFrontmatter(valContent);
      valFm.nyquist_compliant = true;
      valFm.wave_0_complete = true;
      const newValContent = frontmatter.spliceFrontmatter(valContent, valFm);
      fs.writeFileSync(valPath, newValContent, 'utf-8');
    }
  }
} catch (e) { /* non-fatal — phase completion still succeeds */ }
```

**Module imports in `phase.cjs`:** Add at the top of the file (lines 1–15 are the require block):
```javascript
const frontmatter = require('./frontmatter.cjs');
```
The `frontmatter.cjs` module is already a sibling of `phase.cjs` in `lib/`.

**YAML serializer:** Built into `frontmatter.cjs` (function `reconstructFrontmatter` at line 90, called by `spliceFrontmatter`). No `js-yaml` dependency needed and none exists in the gsd-tools install.

**Testing:**
- `gsd-tools.cjs` has no existing test suite (verified — only `bin/` and `bin/lib/` exist; no `test/` or `tests/` directories under `~/.claude/get-shit-done/`).
- Per CONTEXT.md "Claude's Discretion" + `<specifics>`, regression test for CLEANUP-06 Part B is OPTIONAL. Recommended: direct code inspection + a single manual smoke run against a test phase. CONTEXT.md's verification command serves as the integration test:
  ```bash
  grep -l 'nyquist_compliant: true' .planning/phases/*/*-VALIDATION.md | wc -l
  # Expected: 8 (one per phase)
  ```

**Workflow caller (NOT touched):** `~/.claude/get-shit-done/workflows/execute-phase.md` step `update_roadmap` (lines 702–731) calls `gsd-tools.cjs phase complete` — does NOT need modification because the change is inside the CLI's `phase complete` implementation. The workflow continues to invoke the same command.

### Reporting from the modified CLI

Add to the `result` object returned at lines 861–874:
```javascript
const result = {
  // ... existing fields ...
  validation_flipped: <true if flip happened, false otherwise>,
};
```
So the workflow can confirm the flip occurred. (Discretionary — not strictly required by D-05.)

## Code Examples

### Spends::prepare after CLEANUP-03
```rust
// crates/chia-sdk-driver/src/action_system/spends.rs (refactored prepare)
pub fn prepare(
    mut self,
    ctx: &mut SpendContext,
    deltas: &Deltas,
    relation: Relation,
) -> Result<Spends<Finished>, DriverError> {
    // chip-0057 silent-payment derivation branch — runs FIRST so the
    // emitted `CreateCoin` conditions land on the parents'
    // `payment_assertions` before `emit_conditions` fires below. No-op when
    // no `Action::send` with a `SendDestination::SilentPayment` destination
    // has been applied.
    #[cfg(feature = "chip-0057")]
    if !self.silent_payments_pending.is_empty() {
        sp_finish_branch(ctx, &mut self, relation)?;
    }

    self.create_change(ctx, deltas)?;
    self.emit_conditions(ctx)?;
    self.emit_relation(relation);

    Ok(Spends {
        xch: self.xch,
        cats: self.cats,
        // ... rest verbatim ...
        _state: Finished,
    })
}
```

### actions/silent_payment_send.rs (skeleton after CLEANUP-02)
```rust
// crates/chia-sdk-driver/src/actions/silent_payment_send.rs
use chia_puzzle_types::Memos;
use chia_sdk_utils::silent_payments::SilentPaymentAddress;

use crate::{
    BURN_PUZZLE_HASH, DriverError, Id, Output, SpendContext, Spends,
    silent_payments::SilentPaymentPending,
};

/// Apply-time chip-0057 silent-payment send. Fires guards in cheapest-first
/// order ([`DriverError::SilentPaymentRequiresXch`] before memo-hint guard
/// before parent reservation), then reserves an XCH parent, increments the
/// per-`scan_pk` k counter on `Spends`, and pushes a `SilentPaymentPending`
/// entry. ECDH math is deferred to the chip-0057 SP branch of
/// `Spends::prepare`.
pub(crate) fn handle_silent_payment_send(
    ctx: &mut SpendContext,
    spends: &mut Spends,
    id: &Id,
    recipient: &SilentPaymentAddress,
    amount: u64,
    memos: Memos,
) -> Result<(), DriverError> {
    if !matches!(id, Id::Xch) {
        return Err(DriverError::SilentPaymentRequiresXch);
    }
    memo_hint_guard(ctx, memos)?;
    spend_silent_payment(ctx, spends, recipient, amount, memos)
}

// spend_silent_payment + memo_hint_guard relocated verbatim from
// send.rs:135–214, with their existing rustdoc.
fn spend_silent_payment(...) -> Result<(), DriverError> { ... }
fn memo_hint_guard(ctx: &SpendContext, memos: Memos) -> Result<(), DriverError> { ... }
```

### tests/silent_payments_e2e.rs (skeleton after CLEANUP-04)
```rust
// crates/chia-sdk-driver/tests/silent_payments_e2e.rs
#![cfg(feature = "chip-0057")]

//! Phase 6 end-to-end simulator tests for CHIP-0057 silent payments.
//!
//! Lives in the integration-test target so it can call the canonical
//! `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`
//! helper. The cross-crate cycle that would surface as type confusion
//! inside the lib's `#[cfg(test)]` module is resolved at this layer:
//! the integration binary builds against the lib's published types.

use anyhow::Result;
use bip39::Mnemonic;
use chia_protocol::Coin;
use chia_puzzle_types::{DeriveSynthetic, Memos};
use chia_sdk_driver::silent_payments::{K_MAX_DEFAULT, scan_from_tweaks};
use chia_sdk_driver::{
    Action, Id, Layer, Relation, SendDestination, SpendContext, Spends, StandardLayer,
};
use chia_sdk_test::silent_payments::tweak_data_from_simulator_block;
use chia_sdk_test::{BlsPairWithCoin, Simulator};
use chia_sdk_types::Conditions;
use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentKeys, SilentPaymentNetwork};
use indexmap::indexmap;

const TV1_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn setup_e2e() -> Result<(Simulator, SpendContext, BlsPairWithCoin, SilentPaymentKeys)> {
    let mut sim = Simulator::new();
    let ctx = SpendContext::new();
    let sender = sim.bls(1_000);
    let mnemonic = Mnemonic::parse(TV1_MNEMONIC)?;
    let recipient = SilentPaymentKeys::from_mnemonic(&mnemonic);
    Ok((sim, ctx, sender, recipient))
}

#[test]
fn test_simulator_e2e_unlabeled() -> Result<()> {
    let (mut sim, mut ctx, sender, recipient) = setup_e2e()?;
    let recipient_address = recipient.unlabeled_address(SilentPaymentNetwork::Testnet);
    let height_before = sim.height();

    let mut spends = Spends::new(sender.puzzle_hash);
    spends.add(sender.coin);
    let deltas = spends.apply(/* ... */)?;
    // ... rest of body verbatim from e2e.rs:140–216,
    //     but s/build_tweak_data/tweak_data_from_simulator_block/
}
// ... test_simulator_e2e_labeled + test_simulator_e2e_m0_self_change
//     follow the same shape.
```

## Common Pitfalls

### Pitfall 1: Forgetting to enable chia-sdk-test/chip-0057 cascade in CLEANUP-04
**What goes wrong:** Integration test fails to compile with "unresolved module `silent_payments` in chia_sdk_test".
**Why it happens:** `chia-sdk-test`'s `silent_payments` module is feature-gated; the chia-sdk-driver chip-0057 feature doesn't cascade to chia-sdk-test today.
**How to avoid:** Add `chia-sdk-test/chip-0057` to the chip-0057 feature line in `crates/chia-sdk-driver/Cargo.toml:23`.
**Warning signs:** `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` returns "unresolved module" or "tweak_data_from_simulator_block not found".

### Pitfall 2: Pushing sp_finish_branch into prepare WITHOUT removing the inline call in finish_with_keys
**What goes wrong:** SP branch runs twice — first inside `finish_with_keys` BEFORE `prepare`, then again inside `prepare` itself. Second run will hit `SilentPaymentRequiresInputBinding` (if applicable) or push duplicate `CreateCoin` conditions.
**Why it happens:** Refactor seen as "add a new call site" instead of "move the call site".
**How to avoid:** When inserting the SP block into `prepare()`, simultaneously delete `spends.rs:573–576` from `finish_with_keys`. Atomic edit.
**Warning signs:** A pre-existing SP test starts failing with `SilentPaymentRequiresInputBinding`, OR `outputs.xch` has 2× the expected number of recipient coins.

### Pitfall 3: Relocating SP test bodies during CLEANUP-02
**What goes wrong:** Tests at `send.rs:496–1080` (`mod silent_payment_tests`) get moved to `silent_payment_send.rs`, breaking the `cargo test ... --test send` / module-level test discovery.
**Why it happens:** "These tests are for the SP arm, they should live near the SP code."
**How to avoid:** Test bodies test the **public surface** `Action::send(Id::Xch, SendDestination::SilentPayment(...), ...)` which doesn't move. The tests stay in `send.rs`. The `mod silent_payment_tests` block at line 496 is OUT OF SCOPE for CLEANUP-02.
**Warning signs:** Test count for `cargo test -p chia-sdk-driver --features chip-0057` changes after CLEANUP-02 lands. (Should be unchanged.)

### Pitfall 4: Deleting `Relation::None` import in bindings without checking other uses
**What goes wrong:** Compile error "unresolved import `chia_sdk_driver::Relation`".
**Why it happens:** After deleting `spends.finish_silent_payments(&mut ctx, Relation::None)?;` at action_system.rs:191, the `Relation` import may become unused.
**How to avoid:** Check what else uses `Relation` in `chia-sdk-bindings/src/action_system.rs`. The `prepare` call at line 193 still passes `Relation::None`, so the import stays. Verify with `cargo build -p chia-sdk-bindings --all-features` after the edit.

### Pitfall 5: CLEANUP-01 over-aggressive deletion in scanner.rs test-vector pin labels
**What goes wrong:** Comments like `// ─── TV1 pinned bytes (RESEARCH §10a) ──────` get fully deleted, removing useful test context.
**Why it happens:** The label `RESEARCH §10a` matches the grep, so a mechanical strip kills the whole comment.
**How to avoid:** Pass 2 of the strip-reread-repair is the safety net. Each surviving comment gets read in context. Rewrite to cite the actual CHIP test vector (e.g. `// CHIP-0057 TV1 pinned bytes`) — keeps the context, removes the GSD reference.

### Pitfall 6: gsd-tools.cjs patch reading VERIFICATION before it's written
**What goes wrong:** `phase complete` runs before VERIFICATION.md is finalized; flip doesn't fire because `status` is undefined or `verifying`.
**Why it happens:** In `execute-phase.md`, `update_roadmap` runs AFTER VERIFICATION is written (verified at workflow line 702–731). So this is not a real concern under the standard workflow. But: if someone invokes `gsd-tools phase complete <X>` manually before running `/gsd:verify-work`, the flip won't fire. That's correct behavior — VALIDATION.md should NOT be flipped if verification hasn't passed.
**How to avoid:** Document the precondition in the patch's rustdoc. Don't add a "fallback" path that flips regardless. The whole point is that the flip is conditional on `verification.status === 'passed'`.

### Pitfall 7: machete false-positives from removed dev-deps
**What goes wrong:** After CLEANUP-04 deletes `e2e.rs`, some dev-dep imports (`indoc`, `bip39`, `bincode`, etc.) may become unused. `cargo machete` flags them.
**Why it happens:** `e2e.rs` was the only consumer of certain dev-deps. After deletion, they're unused.
**How to avoid:** After CLEANUP-04 lands, run `cargo machete` and either (a) remove the now-unused dev-dep from `chia-sdk-driver/Cargo.toml` OR (b) add to `[package.metadata.cargo-machete] ignored` ONLY IF the dep is genuinely needed elsewhere (e.g., used in a different test file). Check `bip39`, `indexmap`, `chia-sdk-test`, `chia-sdk-utils` dev-dep usage post-deletion. `chia-sdk-test` and `chia-sdk-utils` will continue to be used by the integration test, so they stay. `bip39` may or may not still be used — verify.
**Warning signs:** CI's `cargo machete` step fails.

## Runtime State Inventory

> Phase 7 is pure refactor + comment hygiene + JS CLI patch. Source-only changes; no migrations of stored data, live service config, OS-registered state, secrets, or build artifacts.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — verified by inspection. No DB/registry references being renamed; CLEANUP-01 strips comments only. Phase 7 doesn't touch any field names, on-chain serialization, or persistent state. | None |
| Live service config | None — verified. The gsd-tools CLI patch is a process change but the only persistent state it touches is `.planning/phases/*/*-VALIDATION.md` files in this repo, which are git-tracked. | None |
| OS-registered state | None — verified. No daemons, scheduled tasks, systemd units, or pm2 entries reference Phase 7's targets. The gsd-tools CLI runs ad-hoc, not as a registered service. | None |
| Secrets / env vars | None — verified. No env var renames; no SOPS keys; no GitHub Actions secrets affected. CI workflow `.github/workflows/rust.yml` is unmodified. | None |
| Build artifacts / installed packages | One marginal concern: after CLEANUP-04 deletes `e2e.rs` and CLEANUP-02 adds `silent_payment_send.rs`, the `target/` directory carries stale build artifacts. Resolved automatically by Cargo's incremental rebuild — `cargo test` invalidates affected units. No `*.egg-info`, no published packages affected. | None — Cargo handles automatically |

**Canonical answer to "what runtime systems still have the old string cached":** Nothing. This phase moves code within the workspace and edits comments; no runtime state outside `target/` (which Cargo manages) is affected.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc | All Rust compilation | ✓ | 1.90.0 (pinned via `rust-toolchain.toml`) | — |
| cargo | All Rust commands | ✓ | (matches rustc) | — |
| `cargo clippy` | Phase verification | ✓ (component of rustup) | matches toolchain | — |
| `cargo fmt` | Phase verification | ✓ (component of rustup) | matches toolchain | — |
| `cargo machete` | Phase verification | ✓ (CI installs via binstall) | latest | — |
| node | gsd-tools CLI runtime | ✓ | Node ≥ 14 (per `napi/package.json` engine constraint) | — |
| `~/.claude/get-shit-done/bin/gsd-tools.cjs` | CLEANUP-06 Part B target file | ✓ | 918 lines, current | — |
| Git | CLEANUP-06 manual flip commits | ✓ | — | — |

**All deps available.** Phase 7 has no exotic tool requirements.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace standard) + grep-based oracles for CLEANUP-01/02/03/04 acceptance + manual frontmatter inspection for CLEANUP-06 |
| Config file | none (Cargo auto-discovers); `rust-toolchain.toml` pins toolchain |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 -- --test-threads=1` |
| Full suite command | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLEANUP-01 | Source comments contain no GSD planning-artifact references | grep oracle | `grep -rE 'CONTEXT\.md\|RESEARCH(\.md)?\|Plan 0[1-6]-\|\bD-0[1-9]\b\|Pitfall [0-9]\|Pattern [0-9]' crates/*/src/silent_payments/ crates/chia-sdk-bindings/src/silent_payments.rs crates/chia-sdk-driver/src/action_system/send_destination.rs examples/silent_payment.rs napi/__test__/silent_payments*.ts pyo3/tests/test_silent_payments.py wasm/__test__/silent_payments.spec.ts` returns 0 hits | ✅ (oracle is a grep; targets exist) |
| CLEANUP-01 | Workspace test suite still green | regression | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` | ✅ |
| CLEANUP-02 | `actions/silent_payment_send.rs` exists; `actions/send.rs` ≤ 600 lines | grep / wc oracle | `test -f crates/chia-sdk-driver/src/actions/silent_payment_send.rs && [ $(wc -l < crates/chia-sdk-driver/src/actions/send.rs) -le 600 ]` | ✅ |
| CLEANUP-02 | Existing SP tests still pass via the relocated arm | regression | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payment_tests` | ✅ (10 tests in send.rs:496–1080) |
| CLEANUP-02 | Public surface unchanged — examples + bindings compile | smoke | `cargo build --release --workspace --all-features` and `cargo build --release --examples --all-features` | ✅ |
| CLEANUP-03 | `Spends::finish_silent_payments` removed | grep oracle | `grep -c 'pub fn finish_silent_payments\b' crates/chia-sdk-driver/src/action_system/spends.rs` returns 0 | ✅ |
| CLEANUP-03 | Binding-side leak gone | grep oracle | `grep -c 'finish_silent_payments' crates/chia-sdk-bindings/src/action_system.rs` returns 0 | ✅ |
| CLEANUP-03 | All cross-language E2E tests still pass | regression | `cd napi && pnpm test`; `cd pyo3 && pytest`; `cd wasm && pnpm test` | ✅ |
| CLEANUP-03 | SP branch fires automatically inside `prepare()` | unit | `cargo test --release -p chia-sdk-driver --features chip-0057 round_trip_matches_derive_one_time_puzzle_hash` (existing test at `send.rs:593`) | ✅ |
| CLEANUP-04 | `build_tweak_data` removed; `e2e.rs` deleted | grep oracle | `grep -c 'fn build_tweak_data' crates/chia-sdk-driver/src/silent_payments/e2e.rs 2>/dev/null` returns 0 (file should not exist); `test ! -f crates/chia-sdk-driver/src/silent_payments/e2e.rs` | ✅ |
| CLEANUP-04 | 3 e2e tests pass from integration target | integration | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` runs 3 tests, all pass | ✅ (target file created by CLEANUP-04) |
| CLEANUP-06 | All 8 phases' VALIDATION.md frontmatter `nyquist_compliant: true` | grep oracle | `grep -l 'nyquist_compliant: true' .planning/phases/*/*-VALIDATION.md \| wc -l` returns 8 | ✅ |
| CLEANUP-06 | Same for `wave_0_complete: true` | grep oracle | `grep -l 'wave_0_complete: true' .planning/phases/*/*-VALIDATION.md \| wc -l` returns 8 | ✅ |
| CLEANUP-06 | `phase complete` CLI flips flags going forward | code inspection | Visual review of `~/.claude/get-shit-done/bin/lib/phase.cjs` `cmdPhaseComplete` for the added VALIDATION flip block; optionally a manual run on a test phase confirms behavior | ✅ |

### Sampling Rate
- **Per task commit:** `cargo build --release -p chia-sdk-driver --features chip-0057` + the relevant CLEANUP-* oracle grep
- **Per wave merge:** `cargo test --release -p chia-sdk-driver --features chip-0057` + `cargo clippy --workspace --all-features --all-targets -- -D warnings`
- **Phase gate (before `/gsd:verify-work`):** Full workspace test suite (Rust) + bindings test suites (napi pnpm, pyo3 pytest, wasm pnpm) + all 5 acceptance greps green + `cargo fmt --check` + `cargo machete`

### Wave 0 Gaps
- None — the acceptance oracles are grep / wc / file-existence checks plus the existing workspace test suite. No new test framework, no new test files, no shared fixtures needed. Each CLEANUP-* requirement has a built-in oracle that's mechanical to check.

## State of the Art

Not applicable — Phase 7 is repo-internal cleanup. No external libraries / patterns being adopted or replaced.

**Deprecated/outdated within this repo:**
- `Spends::finish_silent_payments` (`chia-sdk-driver/src/action_system/spends.rs:150–160`) — pinned for deletion in CLEANUP-03.
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` — pinned for deletion in CLEANUP-04.
- `build_tweak_data` (private fn in e2e.rs) — pinned for deletion in CLEANUP-04.
- Inline chip-0057 SP arm in `crates/chia-sdk-driver/src/actions/send.rs` lines 49–56 + helpers 123–214 — pinned for relocation in CLEANUP-02.

## Open Questions

1. **Exact `pub(crate) fn` name for the relocated SP arm helper.**
   - What we know: CONTEXT.md "Claude's Discretion" leaves this to the planner. Candidates: `spend_silent_payment` (matches existing internal name at `send.rs:55`), `handle_silent_payment_send`, `apply_silent_payment_send`.
   - What's unclear: Which naming reads cleanest to a maintainer reviewing the PR.
   - Recommendation: `handle_silent_payment_send` — verb is generic enough to wrap both the `Id != Xch` check and the helper-internals dispatch.

2. **Whether to clean up CLEANUP-01 hits in `e2e.rs` before CLEANUP-04 deletes the file.**
   - What we know: 4 hits in `e2e.rs` will be auto-resolved by CLEANUP-04's delete.
   - What's unclear: If CLEANUP-04 fails for some reason, those 4 hits stay. CONTEXT.md's task ordering puts CLEANUP-01 first.
   - Recommendation: Skip the 4 hits in `e2e.rs` during CLEANUP-01 (note explicitly in the plan); rely on CLEANUP-04's delete. If CLEANUP-04 fails, swing back and clean. The acceptance grep is the gate.

3. **Whether to add `validation_flipped` to the `phase complete` CLI result object.**
   - What we know: Useful for the workflow to confirm the flip occurred.
   - What's unclear: Whether downstream consumers (other workflows) parse this field.
   - Recommendation: Add it (discretionary). The field is additive and breaks nothing.

4. **Whether `cargo machete` will flag a newly-unused dep after CLEANUP-04.**
   - What we know: `crates/chia-sdk-driver/Cargo.toml` dev-deps include `chia-sdk-test`, `chia-sdk-utils`, `anyhow`, `chia-consensus`, `hex`, `hex-literal`, `rstest`. The new integration test uses most of them, but verify.
   - What's unclear: Whether any single dev-dep is uniquely used by `e2e.rs` (e.g., `bip39` is in main deps; `indexmap` is also main). Inspection suggests no — all dev-deps are reused by other tests.
   - Recommendation: Run `cargo machete` at the end of CLEANUP-04 and document the outcome in VERIFICATION.

5. **Whether the patched gsd-tools.cjs change needs to be committed to the repo or to the user's home dir.**
   - What we know: `~/.claude/get-shit-done/bin/gsd-tools.cjs` lives in the user's home dir, NOT in this repo.
   - What's unclear: Process-wise, the patch is a user-side install change. There's no clean way to commit it inside this repo.
   - Recommendation: Patch in place at `~/.claude/get-shit-done/bin/lib/phase.cjs`. Document the change in `07-PHASE-SUMMARY.md` and in the related GSD project's repo if accessible. The repo's job is just to verify the file is patched (grep for the new flip block in the file).

## Sources

### Primary (HIGH confidence)
- `crates/chia-sdk-driver/src/actions/send.rs` (1080 lines, read in full) — CLEANUP-02 source
- `crates/chia-sdk-driver/src/actions.rs` (23 lines) — CLEANUP-02 mod declaration site
- `crates/chia-sdk-driver/src/action_system/spends.rs` (968 lines, read in full) — CLEANUP-03 source
- `crates/chia-sdk-bindings/src/action_system.rs:177–217` — CLEANUP-03 binding-side caller
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` (358 lines, read in full) — CLEANUP-04 source
- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` (lines 1–60) — CLEANUP-04 canonical helper
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — CLEANUP-04 module-declaration site
- `crates/chia-sdk-driver/Cargo.toml` (read in full) — CLEANUP-04 feature wiring site
- `crates/chia-sdk-test/Cargo.toml` (read in full) — chia-sdk-test feature surface
- `~/.claude/get-shit-done/bin/lib/phase.cjs` (lines 600–877 read) — CLEANUP-06 Part B patch target
- `~/.claude/get-shit-done/bin/lib/frontmatter.cjs` (lines 1–337 read) — CLEANUP-06 helper API
- `~/.claude/get-shit-done/workflows/execute-phase.md` (lines 700–780 read) — `update_roadmap` step
- `.github/workflows/rust.yml` (94 lines, read in full) — CI test invocation
- 7 stale `*-VALIDATION.md` files (frontmatter inspected) — CLEANUP-06 Part A targets
- CLEANUP-01 acceptance grep run against current source — 63 hits, 18 files (per-file table in §CLEANUP-01)
- Workspace-wide grep for `Spends::prepare` callsites — 4 sites enumerated

### Secondary (MEDIUM confidence)
- Cargo cyclic-dev-dep type-confusion mechanism — described in `e2e.rs:1–28` module rustdoc; matches Cargo's documented behavior

### Tertiary (LOW confidence)
- None — every claim in this RESEARCH.md is grounded in repo inspection

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps needed; existing workspace deps + Node.js stdlib
- Architecture: HIGH — all locked decisions verified against source; line ranges and signatures double-checked
- Pitfalls: HIGH — derived from inspecting the actual code being modified and from Phase 6's `e2e.rs` rustdoc which documented the cyclic-dev-dep gotcha
- Validation: HIGH — every CLEANUP-* has a mechanical oracle; existing workspace test suite is the regression gate

**Research date:** 2026-05-20
**Valid until:** 2026-06-20 (refactor scope; will only invalidate if Phase 7 doesn't ship within 30 days and other phases edit the same files)
