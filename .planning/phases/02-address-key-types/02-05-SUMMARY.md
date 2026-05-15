---
phase: 02-address-key-types
plan: 05
subsystem: infra
tags: [chip-0057, silent-payments, ci, prelude, phase-gate, traceability, chia-wallet-sdk]

# Dependency graph
requires:
  - phase: 02-address-key-types
    provides: "Plan 02-01: chip-0057 feature wired on chia-sdk-utils; silent_payments module barrel exists as cfg-gated public module"
  - phase: 02-address-key-types
    provides: "Plan 02-02: SilentPaymentError (6-variant enum) + SilentPaymentNetwork (spxch/tspxch HRPs) public via chia_sdk_utils::silent_payments"
  - phase: 02-address-key-types
    provides: "Plan 02-03: SilentPaymentAddress + bech32m encode/decode + 12 named tests in chia-sdk-utils/silent_payments/address.rs"
  - phase: 02-address-key-types
    provides: "Plan 02-04: SilentPaymentKeys + LabelRegistry + 15 named tests in chia-sdk-utils/silent_payments/{keys,labels}.rs; chip-0057 feature cascade to chia-sdk-types/chip-0057"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "Phase 1 Plan 01-05 CI line precedent (chia-sdk-types -F chip-0057); workspace-level chip-0057 feature flag"
provides:
  - "CI workflow line: `cargo build --release -p chia-sdk-utils -F chip-0057` in `.github/workflows/rust.yml` 'Build individual crates' step (WS-02 equivalent for Phase 2)"
  - "Umbrella prelude re-export of the five Phase 2 public types behind `#[cfg(feature = \"chip-0057\")]`: LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork — accessible via `chia_wallet_sdk::prelude::*` when chip-0057 is on"
  - "Phase 2 final gate matrix passes locally: 5 build permutations, strict-mode + workspace clippy, fmt, machete, 2 grep bans, 27 silent_payments tests, 2387 full workspace tests"
  - "Phase 2 closure: all six ADDR-* requirements verified mechanically via the gate matrix"
affects: [phase-03-receive-primitive]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-crate chip-0057 CI build line: one literal `cargo build --release -p <crate> -F chip-0057` line per chip-0057-enabled workspace member. Phase 1 added the `chia-sdk-types` line; this plan adds the `chia-sdk-utils` line. Future phases extending chip-0057 to additional crates inherit the same pattern (one line per crate, inserted alphabetically after the no-features build line for that crate)."
    - "Feature-gated prelude re-export block at the end of `src/prelude.rs`: a single `#[cfg(feature = \"chip-0057\")] pub use chia_sdk_utils::silent_payments::{...};` re-exports the chip-0057 public surface. Pattern: keep prelude flat (no nested cfg blocks), gate the entire re-export block on the feature, list names alphabetically. The no-features umbrella build still compiles because the cfg hides the entire `pub use` statement."

key-files:
  created: []
  modified:
    - ".github/workflows/rust.yml (added 1 line: `cargo build --release -p chia-sdk-utils -F chip-0057` after the existing no-features chia-sdk-utils line, with 10-space leading indent matching the surrounding cargo build lines)"
    - "src/prelude.rs (added 6 lines net: 1 blank + 1 cfg attribute + 1 `pub use ... ::{` + 2 indented names + 1 `};` — re-exports the five Phase 2 public types behind chip-0057)"

key-decisions:
  - "Five names listed alphabetically inside the `pub use` block (LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork) — matches `prettyplease`-style sort. Two names per line (LabelRegistry+SilentPaymentAddress+SilentPaymentError+SilentPaymentKeys on line 1, SilentPaymentNetwork on line 2) because of rustfmt's 100-character line limit; the final rustfmt-confirmed shape is what landed."
  - "CI build line inserted AFTER the existing `cargo build --release -p chia-sdk-utils` (no-features) line, NOT after the chia-sdk-types -F chip-0057 line. Matches Phase 1's local-grouping convention (each crate's no-features + feature-on builds adjacent), keeps the chia-sdk-types group and chia-sdk-utils group separate."
  - "No additional `-F chip-0057` lines added for other crates (e.g., chia-sdk-driver) because Phase 2 added no chip-0057-gated code to driver. Phase 3 (receive primitive) is the next plan to extend chip-0057 to a new crate; its WS-equivalent line lands then."
  - "Task 2 produced ZERO file edits — verification-only task. The gate matrix exercised the cumulative output of Plans 02-01..02-04 (which was already clean from each plan's own per-task verify). No Rule 1/2/3 auto-fixes triggered; the previously-completed plans landed cleanly enough that the final sweep needed no remediation."
  - "Workspace clippy (CI invocation, no -D warnings) emits 2 pre-existing pedantic warnings in chia-sdk-daemon/src/client.rs:426-427 (match_same_arms + match_wildcard_for_single_variants). These are out of Phase 2 scope per Phase 1 Plan 01-05 carry-over (`01-PHASE-SUMMARY.md` Carried-forward observation 2). CI's clippy step exits 0 because it doesn't use -D warnings."

patterns-established:
  - "Phase-gate plan structure: minimal file edits (1 CI line + 1 prelude block) + one verification-only task that exercises the cumulative gate matrix on the work of prior plans. Matches Phase 1 Plan 01-05 precedent. Future per-phase gate plans inherit this 2-task shape."
  - "Defer-prelude-to-final-gate pattern: Plan 02-01 documented a NO-OP for prelude.rs (Task 3 deferred because types didn't exist yet); this plan completed the deferred edit once Plans 02-02/03/04 landed the types. Pattern reusable in future phases that want to defer a chicken-and-egg prelude re-export to the phase's final gate plan."

requirements-completed: []  # No new requirements close in this plan; Phase 2 requirements (ADDR-01..06) closed across Plans 02-03 (ADDR-02) and 02-04 (ADDR-01, 03, 04, 05, 06). This plan is the traceability sweep that verifies all six are mechanically reachable.

# Metrics
duration: 13min
completed: 2026-05-15
---

# Phase 02 Plan 05: CI matrix + final gate verification (prelude re-export + 5-build sweep + 13-expression phase gate) Summary

**Phase 2 closure: `.github/workflows/rust.yml` gains the per-crate `chia-sdk-utils -F chip-0057` build line (WS-02 equivalent for Phase 2); `src/prelude.rs` re-exports the five Phase 2 public types behind `#[cfg(feature = "chip-0057")]`; the full 13-expression phase-gate matrix (5 builds + strict clippy + workspace clippy + fmt + machete + 2 grep bans + silent_payments test suite + full workspace test suite) passes locally with 27 silent_payments tests green and 2387 total workspace tests passing.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-05-15T20:13:59Z
- **Completed:** 2026-05-15T20:27:07Z
- **Tasks:** 2 (1 file edit + 1 verification-only gate sweep)
- **Files modified:** 2 (`.github/workflows/rust.yml` + `src/prelude.rs`)

## Accomplishments

- **CI workflow extended:** Added one line `cargo build --release -p chia-sdk-utils -F chip-0057` to `.github/workflows/rust.yml` immediately after the existing no-features `chia-sdk-utils` build line. The new line has 10-space leading indent matching the surrounding `cargo build` lines exactly, lands inside the `run: |` block of the "Build individual crates" step. Mirrors Phase 1's WS-02 line for `chia-sdk-types`.
- **Prelude re-export landed:** Added a 5-name `#[cfg(feature = "chip-0057")] pub use chia_sdk_utils::silent_payments::{LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork};` block at the end of `src/prelude.rs`. Five names in alphabetical order; rustfmt wraps to two lines under the 100-character limit. The no-features umbrella build still compiles (cfg gate hides the re-export when chip-0057 is off).
- **Five-build sweep PASS:** `chia-sdk-utils` ± `chip-0057` ± `--all-features` (3 perms) + workspace ± `--all-features` (2 perms). All five exit 0.
- **Strict-mode clippy PASS:** `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` exits 0. Phase 2's silent_payments tree is pedantic-clean.
- **Workspace clippy PASS:** `cargo clippy --workspace --all-features --all-targets` exits 0. Two pre-existing pedantic warnings in `chia-sdk-daemon/src/client.rs:426-427` are carry-forwards from Phase 1 (out of Phase 2 scope per `01-PHASE-SUMMARY.md` observation 2); the CI clippy step does NOT use `-D warnings` so those warnings do not gate the build.
- **fmt + machete PASS:** `cargo fmt --all -- --files-with-diff --check` clean; `cargo machete` clean with zero new `[package.metadata.cargo-machete] ignored` entries across the workspace (verified by grepping the Phase 2 diff range `acecb625..HEAD` for new `cargo-machete` strings — zero hits).
- **Phase 1 grep bans HOLD:** `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` exits 0 (zero matches); `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` exits 0 (zero matches). Both bans inherited from Phase 1.
- **27/27 silent_payments tests PASS** under `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments`: 12 address + 7 keys + 8 labels = 27. Per-module counts verified individually via filtered invocations (12 in `address::tests`, 7 in `keys::tests`, 8 in `labels::tests`).
- **Full workspace test suite PASS:** `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` exits 0. Aggregate totals: **2387 passed, 0 failed, 0 ignored**. Confirms zero regressions: Phase 1 baseline was 2360 tests; Phase 2 adds 27 silent_payments tests → 2387 expected, 2387 observed.
- **All 13 gate expressions in the plan's `<verification>` block pass** (5 builds + 2 clippy + fmt + machete + 2 grep bans + 2 test commands = 13).

## Task Commits

Each task was committed atomically (where it produced file changes):

1. **Task 1: Add chip-0057 CI build line for chia-sdk-utils + add prelude re-export** — `2aacf626` (feat)
2. **Task 2: Verification-only gate sweep** — no commit (no file edits; all 13 gate expressions passed on first run with zero remediation required)

**Plan metadata commit:** to follow this SUMMARY.

## Files Created/Modified

- `.github/workflows/rust.yml` (modified, 1 net line added): Inserted `          cargo build --release -p chia-sdk-utils -F chip-0057` between the existing `chia-sdk-utils` (no-features) line and the first `chia-sdk-bindings -F napi` line. 10-space leading indent matches surrounding `cargo build` lines. `grep -c 'chia-sdk-utils -F chip-0057' .github/workflows/rust.yml` returns 1.
- `src/prelude.rs` (modified, 6 net lines added): Appended after the existing `pub use chia_sdk_utils::{Address, Bech32, parse_hex, select_coins};` line. Block content: 1 blank line, 1 `#[cfg(feature = "chip-0057")]` attribute, 1 `pub use chia_sdk_utils::silent_payments::{` opening line, 1 line with `LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys,`, 1 line with `SilentPaymentNetwork,`, and 1 closing `};` line.

## Decisions Made

- **CI line placement** — inserted immediately after the existing `chia-sdk-utils` (no-features) line at workflow line 66, NOT after the `chia-sdk-types -F chip-0057` line at line 65. Local-grouping convention: each crate's no-features and feature-on builds stay adjacent. This is the same shape Phase 1's Plan 01-05 chose for the chia-sdk-types pair.
- **Prelude name order** — alphabetical (LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork). Matches `prettyplease`-style sort. rustfmt wraps to two lines automatically under the 100-character line limit; the final shape is what landed.
- **No-features umbrella build still green** — confirmed by `cargo build --release --workspace` (no features), exit 0. The `#[cfg(feature = "chip-0057")]` gate hides the entire `pub use chia_sdk_utils::silent_payments::{...};` statement when chip-0057 is off, so the module path `chia_sdk_utils::silent_payments` is never referenced from prelude.rs in that build mode.
- **Task 2 verification-only — no commit produced** — the gate matrix exercised the cumulative output of Plans 02-01..02-04, which were already clean from each plan's per-task verify. No Rule 1/2/3 auto-fixes triggered. Working tree remained clean (`git status --short` empty) after the full 9-command-group sweep.
- **Workspace clippy warnings out of scope** — the 2 `chia-sdk-daemon/src/client.rs:426-427` pedantic warnings (`match_same_arms` + `match_wildcard_for_single_variants`) are pre-existing carry-overs from Phase 1 (per `01-PHASE-SUMMARY.md` observation 2). They are NOT regressions, NOT in scope for Phase 2 to fix, and do NOT gate CI (which runs `clippy` without `-D warnings`). Phase 3 or a separate chore commit can address them.

## Deviations from Plan

None — plan executed exactly as written. Both file edits landed on first attempt; all 13 gate expressions in the phase-gate matrix passed on first run with no remediation; no Rule 1/2/3 auto-fixes triggered.

## Issues Encountered

None substantial. One bash-shell quirk during the gate run (`! grep ...` inside a chained `&& echo` sequence terminated the script when grep returned exit 1, which is the success path for the negation) — resolved by running the two grep bans separately as standalone commands. This is a verification mechanics observation, not a code issue; both bans hold mechanically (zero matches).

## User Setup Required

None — no external service or environment configuration required.

## Next Phase Readiness

**Phase 3 (receive primitive + CHIP test-vector closure) is unblocked:**
- All six ADDR-* requirements (ADDR-01..06) are mechanically closed via Plans 02-03 (ADDR-02) and 02-04 (ADDR-01, 03, 04, 05, 06).
- The `chip-0057` feature on `chia-sdk-utils` is fully wired end-to-end with the chia-sdk-types cascade in place; Phase 3 can `use chia_sdk_utils::silent_payments::{SilentPaymentKeys, LabelRegistry, SilentPaymentAddress}` inside `#[cfg(feature = "chip-0057")]` modules without any further Cargo.toml edits.
- The umbrella crate's prelude exports the five Phase 2 public types under chip-0057; Phase 3's wallet-author-facing types (TweakData, DetectedSpCoin, scan_from_tweaks) can be added to the same prelude block when they land.
- The CI workflow's `chia-sdk-utils -F chip-0057` build line will exercise Phase 3's additions automatically (Phase 3 lands chip-0057-gated code on `chia-sdk-driver`, which gets its own WS-equivalent line in Phase 3's final gate plan).
- 27 silent_payments tests baseline + 2360 Phase 1 baseline = 2387 expected workspace test count; Phase 3 builds on top of this.

**Blockers/concerns:** None. The Q8 audit (Phase 2 pre-flight: chia-sdk-types -> chia-sdk-utils optional dep edge) was resolved in Plan 02-01. All entry blockers for Phase 3 are closed.

## Self-Check

- [x] `.github/workflows/rust.yml` modified — verified via `git diff --stat`: 1 file changed, 1 insertion
- [x] `src/prelude.rs` modified — verified via `git diff --stat`: 1 file changed, 6 insertions
- [x] `grep -c 'cargo build --release -p chia-sdk-utils -F chip-0057' .github/workflows/rust.yml` returns 1
- [x] `grep -cE 'cargo build --release -p chia-sdk-utils' .github/workflows/rust.yml` returns 2 (no-features + -F chip-0057)
- [x] CI line indentation is exactly 10 spaces (verified via awk)
- [x] `grep -q '#\[cfg(feature = "chip-0057")\]' src/prelude.rs` exits 0
- [x] All 5 type names present in prelude: LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork
- [x] Commit `2aacf626` exists — verified via `git log --oneline -3`
- [x] All 5 build permutations exit 0 (per-crate no-features, -F chip-0057, --all-features; workspace no-features, --all-features)
- [x] `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` exits 0
- [x] `cargo clippy --workspace --all-features --all-targets` exits 0
- [x] `cargo fmt --all -- --files-with-diff --check` exits 0
- [x] `cargo machete` exits 0
- [x] Zero new `[package.metadata.cargo-machete]` entries in Phase 2 diff range (`git diff acecb625..HEAD -- '**/Cargo.toml' | grep -E '^\+.*cargo-machete'` empty)
- [x] `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` holds (grep exit 1 = no matches)
- [x] `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` holds (grep exit 1 = no matches)
- [x] `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` runs 27 tests, all pass
- [x] Per-module counts: 12 address + 7 keys + 8 labels = 27
- [x] Full workspace test suite passes: 2387 passed, 0 failed, 0 ignored
- [x] Working tree clean after Task 2 verification (no untracked changes)

## Self-Check: PASSED

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
