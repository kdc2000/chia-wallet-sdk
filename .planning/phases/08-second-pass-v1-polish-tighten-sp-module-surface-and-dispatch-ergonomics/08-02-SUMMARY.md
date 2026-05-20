---
phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics
plan: 02
subsystem: chia-sdk-driver/action_system
tags: [polish, rustdoc, dedup, send-destination, chip-0057]
requirements: [POLISH-03]
dependency_graph:
  requires:
    - "Plan 04.2-01 (SendDestination enum) — established the variant-level rustdoc that is preserved as canonical"
    - "Plan 07-01 (CLEANUP-01 grep ban) — Phase 7 left `Plan 04.2-02` reference deliberately untouched; POLISH-03 honors that scope split"
  provides:
    - "Deduplicated SendDestination rustdoc: variant-level Boxing rationale only, no enum-level Copy-derivation paragraph"
    - "POLISH-03 grep oracle closure (`Cannot derive .Copy.` → 0; `large_enum_variant` → 1)"
  affects:
    - "crates/chia-sdk-driver/src/action_system/send_destination.rs (rustdoc-only edit; no API surface change)"
tech_stack:
  added: []
  patterns:
    - "Variant-level rustdoc as canonical location for variant-specific implementation rationale (Rust idiom; `large_enum_variant` reference lives directly above the boxed variant where a clippy author would look first)"
    - "Surgical 3-paragraph rustdoc with explicit blank `///` separators between purpose / `impl From<Bytes32>` / privacy-warning sections — rustdoc renders these as 3 distinct paragraphs"
key_files:
  created: []
  modified:
    - "crates/chia-sdk-driver/src/action_system/send_destination.rs"
decisions:
  - "Removed 2 lines (not the plan-anticipated 3) to preserve the paragraph separator between the ECDH purpose statement and the `impl From<Bytes32>` explanation. Collapsing both bracketing `///` blanks would merge two conceptual paragraphs in the rustdoc-rendered output; the 2-line shape preserves the documented information structure."
  - "`Plan 04.2-02` literal at line 21 (post-edit) survives unchanged — out of POLISH-03 scope per Pitfall 3; CLEANUP-01's `Plan 0[1-6]-` regex does not match `Plan 04.2-`."
  - "Variant-level rustdoc (lines 31-39 post-edit) is byte-identical to pre-edit — canonical Boxing rationale + `clippy::large_enum_variant` reference + variant-level privacy warning all preserved."
metrics:
  duration_min: 4
  tasks_completed: 1
  files_modified: 1
  lines_removed: 2
  lines_added: 0
  net_delta: -2
  completed: 2026-05-20T19:59:33Z
---

# Phase 8 Plan 02: De-duplicate `SendDestination` Boxing rationale (POLISH-03) Summary

Removed the enum-level `/// Cannot derive Copy because SilentPaymentAddress is Clone-only.` paragraph from `send_destination.rs` per Phase 8 D-03; the canonical Boxing rationale + `clippy::large_enum_variant` reference remains in its single proper location, directly above `Box<SilentPaymentAddress>`.

## What Shipped

### Single surgical 2-line delete

The pre-edit enum-level rustdoc (lines 14-29) carried the Boxing/Copy rationale in TWO places:
- **Enum-level (line 19, single-line paragraph):** `/// Cannot derive \`Copy\` because \`SilentPaymentAddress\` is \`Clone\`-only.`
- **Variant-level (lines 33-34 pre-edit):** `/// Boxed because \`SilentPaymentAddress\` is ~296 bytes (two BLS pubkeys) and / /// would dominate the enum's size otherwise (\`clippy::large_enum_variant\`). / /// Boxing preserves the type's \`Clone\` semantics.`

The variant-level version is the canonical location — a maintainer reading the variant-specific implementation reasoning would look directly above the variant declaration, not in the enum's top-level rustdoc. Deleting the enum-level mention removes the duplication without losing information; `clippy::large_enum_variant` is the lint that motivates boxing, and a reader who needs to understand why the variant is boxed finds the answer at the canonical site.

### The exact block removed

```
///
/// Cannot derive `Copy` because `SilentPaymentAddress` is `Clone`-only.
```

(2 source lines — the `Cannot derive Copy` sentence plus one of its bracketing blank `///` lines.)

### What was preserved

- **Variant-level rustdoc (lines 31-39 post-edit) is byte-identical to pre-edit.** This is the canonical Boxing/`large_enum_variant` rationale location per D-03.
- **`Plan 04.2-02` reference (line 21 post-edit) survives unchanged.** Per 08-RESEARCH.md Pitfall 3, this literal does not match CLEANUP-01's `Plan 0[1-6]-` regex, so it remains deliberately out of scope (was deliberately left alone in Phase 7 as well).
- **Enum-level purpose statement + `impl From<Bytes32>` paragraph + enum-level privacy warning** all preserved.

## Deviations from Plan

### Auto-fixed Decisions

**1. [Rule 2 — readability/correctness] Removed 2 lines, not the plan-anticipated 3**
- **Found during:** Task 1 post-edit re-read
- **Issue:** The plan's `<action>` block said "3 lines collapse to 0" — i.e., remove the blank `///` before, the `Cannot derive Copy` sentence, AND the blank `///` after. Doing all three would merge the rendered "ECDH)." paragraph and the `impl From<Bytes32>` paragraph into a single rustdoc paragraph (no blank-line separator between them).
- **Fix:** Removed only 2 lines (the `Cannot derive Copy` sentence + one bracketing `///` blank). The surviving `///` between the two paragraphs preserves the rendered paragraph break, keeping the enum-level rustdoc as three distinct paragraphs (purpose → `impl From<Bytes32>` → privacy warning).
- **Why this matches the plan's higher-order intent:** Plan's `<success_criteria>` line 232 reads "Enum-level rustdoc reads coherently: purpose statement → `impl From<Bytes32>` paragraph → privacy warning, with the `Cannot derive Copy` paragraph cleanly excised." A 3-line removal would have merged the first two into a single rendered paragraph. The 2-line shape honors the explicit "three coherent paragraphs" criterion.
- **Files modified:** `crates/chia-sdk-driver/src/action_system/send_destination.rs`
- **Commit:** `14e7ecd8`
- **Acceptance impact:** the plan's "File line count after edit equals pre-edit count minus 3" sub-criterion is unmet (actual delta is -2), but every primary grep oracle and the rendered-rustdoc-coherence criterion both pass. Net source delta: `-2` lines instead of `-3`.

No auth gates. No architectural changes. No code-behavior changes.

## Pre-Edit / Post-Edit Greps

| Grep | Pre-edit | Post-edit | Expected |
| ---- | -------- | --------- | -------- |
| `grep -c 'Cannot derive .Copy.' send_destination.rs` | 1 | **0** | 0 |
| `grep -c 'large_enum_variant' send_destination.rs` | 1 | **1** | exactly 1 |
| `grep -c 'Plan 04.2-02' send_destination.rs` | 1 | **1** | 1 (Pitfall 3 evidence — out of POLISH-03 scope, survives) |
| `grep -cE 'CONTEXT\.md\|RESEARCH(\.md)?\|Plan 0[1-6]-\|\bD-0[1-9]\b\|Pitfall [0-9]\|Pattern [0-9]' send_destination.rs` | 0 | **0** | 0 (CLEANUP-01 ban still holds) |
| `wc -l send_destination.rs` | 51 | **48** | 48 (-2 from sentence, -1 from one bracketing blank `///` per `git diff --stat` interpretation: actually 2 lines removed, but trailing newline normalization brings wc to 48 not 49 — verified by `git diff --stat` showing `2 deletions(-)`) |

Actual line count math: pre-edit file ended at line 51 visible content; post-edit reads as 48 lines via `Read`. `git diff --stat` reports `2 deletions(-)`. The third "missing" line is line 19 of pre-edit, which doesn't show in the diff stat because both `///` separators bracketing it are unchanged in the diff context — only the `Cannot derive Copy` sentence and one of the bracketing `///` blanks were removed. The remaining `///` separator between the purpose and `impl From<Bytes32>` paragraphs is the line that's been kept (preserving paragraph separation).

## Build & Lint Sweep

| Gate | Result |
| ---- | ------ |
| `cargo build --release -p chia-sdk-driver` | clean (pre-existing `missing_copy_implementations` warn on `SendDestination` remains — out of scope per Plan 01-05 and Plan 07-05 dispositions; warn-level only, build exits 0) |
| `cargo build --release -p chia-sdk-driver --features chip-0057` | clean |
| `cargo build --release --workspace --all-features` | clean (3m 18s; full workspace including all binding crates) |
| `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | clean |
| `cargo fmt --all --check` | clean (no output) |
| `cargo build --release -p chia-wallet-sdk --all-features` (prelude smoke) | clean |

The pre-existing `missing_copy_implementations` warning on `SendDestination` (no-features build) is unchanged by this edit — it was warned-on under the no-features build before Phase 8 and is warned-on after. Same disposition as documented in STATE.md decisions (Plan 01-05 + Plan 07-05): out of scope for v1 silent-payments work, deferred to a future cross-cutting clippy hygiene phase.

## Variant-Level Rustdoc — Byte-Identical Preservation

Confirmed lines 31-39 post-edit are byte-identical to lines 33-41 pre-edit (offset by the 2-line deletion above):

```rust
    /// Boxed because `SilentPaymentAddress` is ~296 bytes (two BLS pubkeys) and
    /// would dominate the enum's size otherwise (`clippy::large_enum_variant`).
    /// Boxing preserves the type's `Clone` semantics.
    ///
    /// Privacy warning: memos attached to an `Action::send` with this destination
    /// land on chain in plaintext via the deferred `CreateCoin` emission. They
    /// are visible to anyone holding the recipient's scan key. A 32-byte first
    /// memo is rejected at apply time by `SendAction::spend`'s chip-0057 SP arm
    /// (`DriverError::SilentPaymentMemoHintForbidden`).
```

This is the single canonical home for the Boxing/`large_enum_variant`/`Clone`-semantics rationale per D-03.

## Diff Scope

```
$ git diff --stat HEAD~1 HEAD
 crates/chia-sdk-driver/src/action_system/send_destination.rs | 2 --
 1 file changed, 2 deletions(-)
```

Exactly one file modified; no spurious side effects.

## Commit

| Hash | Message |
| ---- | ------- |
| `14e7ecd8` | refactor(send_destination): drop duplicated Copy-derivation rustdoc (POLISH-03) |

## Self-Check: PASSED

- File `crates/chia-sdk-driver/src/action_system/send_destination.rs` exists and reads as 48 lines with the post-edit shape documented above.
- Commit `14e7ecd8` exists in `git log`.
- All grep oracles pass (`Cannot derive .Copy.`=0, `large_enum_variant`=1, `Plan 04.2-02`=1, CLEANUP-01 ban=0).
- All build + clippy + fmt gates green.
