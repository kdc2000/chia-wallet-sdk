---
phase: 01-crypto-primitives-workspace-integration
plan: 01
subsystem: infra
tags: [cargo, workspace, feature-flags, chip-0057, silent-payments]

requires: []
provides:
  - "chip-0057 workspace feature gate at the root Cargo.toml"
  - "chip-0057 = [] feature entry in chia-sdk-types"
  - "chip-0057 cascade entry in chia-sdk-driver (depends on chia-sdk-types/chip-0057)"
  - "First-ever [features] block in chia-sdk-utils with chip-0057 = []"
affects:
  - 01-02 (crypto primitives module — adds first code behind the gate)
  - 01-03 (tagged_hash + test vectors)
  - 01-04 (ScalarField)
  - 01-05 (cargo machete + clippy gate closure)
  - 02-XX (address & key types — gates SilentPaymentKeys / SilentPaymentAddress under the feature)

tech-stack:
  added: []
  patterns:
    - "Feature-flag scaffolding before code (mirrors chip-0037 precedent bbc7f57f)"
    - "Cascade pattern: root feature -> per-crate feature -> downstream cascades within crates"

key-files:
  created:
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-01-SUMMARY.md"
  modified:
    - "Cargo.toml"
    - "crates/chia-sdk-types/Cargo.toml"
    - "crates/chia-sdk-driver/Cargo.toml"
    - "crates/chia-sdk-utils/Cargo.toml"

key-decisions:
  - "chip-0057 feature added without dep:chia-sdk-types edge on chia-sdk-utils (Phase 2 pre-flight audit; per STATE.md Q8 deferral)"
  - "chia-sdk-driver chip-0057 cascade is pure (no dep:* activations) — Phase 1 adds no driver-side code"
  - "Empty no-op feature in chia-sdk-utils anticipates Phase 2 SilentPaymentAddress without requiring another Cargo.toml edit later"

patterns-established:
  - "Single-line cascade pattern: chip-NNNN = [\"chia-sdk-driver/chip-NNNN\", \"chia-sdk-types/chip-NNNN\", ...]"
  - "Per-crate empty feature pattern for crates that will gain code later: chip-NNNN = []"

requirements-completed: [WS-01]

duration: 10min
completed: 2026-05-15
---

# Phase 01 Plan 01: Workspace chip-0057 feature flag scaffolding Summary

**Added chip-0057 workspace feature cascading from root Cargo.toml to chia-sdk-types/driver/utils, mirroring the chip-0037 precedent byte-for-byte; no Rust source changed.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-05-15T15:45:53Z
- **Completed:** 2026-05-15T15:56:45Z
- **Tasks:** 1
- **Files modified:** 4

## Accomplishments

- Root `Cargo.toml` `[features]` block now exposes `chip-0057` and cascades to all three downstream crates (`chia-sdk-driver`, `chia-sdk-types`, `chia-sdk-utils`).
- `chia-sdk-types/Cargo.toml` declares `chip-0057 = []` alongside the existing `chip-0035` / `chip-0037` empty-list entries.
- `chia-sdk-driver/Cargo.toml` declares `chip-0057 = ["chia-sdk-types/chip-0057"]` (cascade-only, no `dep:*` activations — no driver-side code yet).
- `chia-sdk-utils/Cargo.toml` gains its first-ever `[features]` block with `chip-0057 = []`, positioned between the existing `[lints]` and `[dependencies]` blocks.
- All six plan-level gate commands pass: per-crate `-F chip-0057` builds (types, driver, utils), `--workspace --all-features`, `--workspace` (no features), and `cargo fmt --check`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add chip-0057 to root and per-crate Cargo.toml files** — `79af49c8` (feat)

**Plan metadata commit:** (added at the end of this plan, includes SUMMARY.md, STATE.md, ROADMAP.md, REQUIREMENTS.md)

## Files Created/Modified

- `Cargo.toml` — appended `chip-0057 = ["chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]` to `[features]` (line 75)
- `crates/chia-sdk-types/Cargo.toml` — appended `chip-0057 = []` to `[features]` (line 20)
- `crates/chia-sdk-driver/Cargo.toml` — appended `chip-0057 = ["chia-sdk-types/chip-0057"]` to `[features]` (line 23)
- `crates/chia-sdk-utils/Cargo.toml` — inserted new `[features]` block (`chip-0057 = []`) between `[lints]` and `[dependencies]` (lines 17-19)

## Decisions Made

- **No `dep:chia-sdk-types` edge added to `chia-sdk-utils`.** STATE.md Q8 lists this as a Phase 2 pre-flight audit item (downstream consumer impact). Phase 1 leaves the `chip-0057` feature in `chia-sdk-utils` purely empty so the audit can decide between adding the optional dep or keeping `SilentPaymentAddress` self-contained.
- **`chia-sdk-driver/chip-0057` is a pure cascade.** No `dep:*` activations are needed because Phase 1 adds no `chip-0057`-gated code to `chia-sdk-driver`. Driver-side code (the `SilentPaymentSend` action) lands in Phase 4 and any new optional deps will be added at that time.
- **Single-line cascade format.** Matches the inline shape of the existing `chip-0035` / `chip-0037` entries (not multi-line arrays). Keeps the `[features]` block visually uniform.

## Deviations from Plan

None — plan executed exactly as written. Mirrored the chip-0037 precedent commit `bbc7f57f` literally for all three pre-existing files; the new `[features]` block in `chia-sdk-utils` was placed exactly between `[lints]` and `[dependencies]` as the plan specified.

## Issues Encountered

None. All five build permutations and the format check passed on first run.

## Self-Check: PASSED

Verified all four file modifications and the task commit exist:

- `Cargo.toml` line 75 contains `chip-0057 = ["chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]` — FOUND
- `crates/chia-sdk-types/Cargo.toml` line 20 contains `chip-0057 = []` — FOUND
- `crates/chia-sdk-driver/Cargo.toml` line 23 contains `chip-0057 = ["chia-sdk-types/chip-0057"]` — FOUND
- `crates/chia-sdk-utils/Cargo.toml` line 17 contains `[features]` and line 18 contains `chip-0057 = []` — FOUND
- Commit `79af49c8` ("feat(01-01): add chip-0057 workspace feature scaffolding") — FOUND in `git log`
- `git diff --stat` for the pre-commit state showed exactly 4 files / 6 insertions / 0 deletions — matched plan expectation
- No new `[workspace.dependencies]` entries introduced — VERIFIED
- No new `[package.metadata.cargo-machete]` entries introduced — VERIFIED

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 01-02 (`silent_payments` module skeleton in `chia-sdk-types`) can now reference `#[cfg(feature = "chip-0057")]` and have it activate cleanly.
- Plans 01-03 / 01-04 (tagged_hash and ScalarField) inherit the same gate with no additional Cargo.toml work.
- Plan 01-05 (machete + clippy closure) will run `cargo machete` against the cumulative state once code is added; this plan added no Rust so machete cannot regress here.
- No blockers for downstream plans.

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
