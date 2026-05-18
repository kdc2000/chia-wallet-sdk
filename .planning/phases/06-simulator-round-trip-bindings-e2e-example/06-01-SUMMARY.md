---
phase: 06-simulator-round-trip-bindings-e2e-example
plan: 01
subsystem: infra
tags: [chip-0057, cargo-features, ci, workspace, chia-sdk-test]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature + chia-sdk-types/driver/utils cascade pattern (WS-01..03)"
  - phase: 02
    provides: "chia-sdk-utils/chip-0057 cascade precedent (Plan 02-01 dep:chia-sdk-types pattern)"
  - phase: 03
    provides: "chia-sdk-driver/chip-0057 cascade precedent (Plan 03-01 dep:chia-sdk-utils pattern)"
provides:
  - "chia-sdk-test [features].chip-0057 cascading to chia-sdk-driver + chia-sdk-types + chia-sdk-utils"
  - "chia-sdk-driver and chia-sdk-utils as optional workspace deps on chia-sdk-test"
  - "root Cargo.toml workspace chip-0057 feature now cascades to chia-sdk-test/chip-0057"
  - "CI build matrix per-crate -F chip-0057 line for chia-sdk-test"
  - "Wave 0 prerequisite for Plan 06-02 (tweak_data_from_simulator_block + 2 unit tests in chia-sdk-test::silent_payments)"
affects: [06-02, 06-03, 06-04, 06-05, BIND-03, EX-01]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "First-time chip-0057 wiring for a new consumer crate (chia-sdk-test) — mirrors the chip-0035/chip-0037/chip-0057 cascade precedent applied to types/driver/utils in Phases 1-3"
    - "Optional workspace dep + dep: feature activation pattern: `dep:chia-sdk-driver` + `chia-sdk-driver/chip-0057` together in the chip-0057 feature list, with `chia-sdk-driver = { workspace = true, optional = true }` in [dependencies]. Keeps no-features build unaffected."

key-files:
  created:
    - ".planning/phases/06-simulator-round-trip-bindings-e2e-example/deferred-items.md"
  modified:
    - "crates/chia-sdk-test/Cargo.toml"
    - "Cargo.toml"
    - "Cargo.lock"
    - ".github/workflows/rust.yml"

key-decisions:
  - "Made BOTH chia-sdk-driver AND chia-sdk-utils optional + dep:-activated in chip-0057 (plan only specified dep:chia-sdk-driver explicitly, but adding dep:chia-sdk-utils prevents Cargo's implicit-activation warning that fires when chia-sdk-utils/chip-0057 references an optional dep — keeps the feature explicit and forward-compatible with future Cargo behavior shifts)"
  - "Documented cargo machete false-positive on chia-sdk-driver+chia-sdk-utils per plan's Task 1 step 4 + Task 3 step 8: do NOT add to [package.metadata.cargo-machete] ignored. Plan 06-02 consumes both deps, closing the gap."
  - "chia-sdk-daemon/src/client.rs:426-427 clippy::pedantic warnings are pre-existing (commit ec1a3517 predates Phase 6); confirmed against Phase 1's deferred-items.md and recorded fresh in Phase 6's deferred-items.md. CI's clippy step without -D warnings exits 0 today."

patterns-established:
  - "Wave 0 feature-wiring plan pattern: when a new crate gains a chip-XXXX feature for the first time, the three landing surfaces are (1) per-crate [features].chip-XXXX with cascade + optional deps, (2) root Cargo.toml workspace chip-XXXX feature update, (3) .github/workflows/rust.yml per-crate -F chip-XXXX line — committed across 2-3 atomic commits."

requirements-completed: [SIM-01]  # SIM-01 prerequisite-portion: chia-sdk-test gains chip-0057 access path so Plan 06-02 can place tweak_data_from_simulator_block.

# Metrics
duration: 14min
completed: 2026-05-18
---

# Phase 06 Plan 01: chip-0057 Wave 0 wiring on chia-sdk-test Summary

**chia-sdk-test gains a chip-0057 feature with optional chia-sdk-driver + chia-sdk-utils deps, workspace cascade updated, CI matrix gains the per-crate -F chip-0057 build line — Wave 1 (Plan 06-02 tweak_data_from_simulator_block) is unblocked.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-05-18T22:47:38Z
- **Completed:** 2026-05-18T23:01:46Z
- **Tasks:** 3 (3 commits + 1 verification commit; SUMMARY commit is the 4th)
- **Files modified:** 4 (3 modified + 1 created)

## Accomplishments

- New `[features].chip-0057` block in `crates/chia-sdk-test/Cargo.toml` cascading `dep:chia-sdk-driver`, `dep:chia-sdk-utils`, `chia-sdk-driver/chip-0057`, `chia-sdk-types/chip-0057`, and `chia-sdk-utils/chip-0057` — the first chip-0057 entry on chia-sdk-test, mirroring the workspace cascade pattern.
- `chia-sdk-driver` and `chia-sdk-utils` declared as optional workspace deps on chia-sdk-test (neither existed before); no-features build unchanged.
- Root `Cargo.toml` workspace `chip-0057` feature updated to include `chia-sdk-test/chip-0057` (alphabetic order: driver, test, types, utils).
- `.github/workflows/rust.yml` CI build matrix gains `cargo build --release -p chia-sdk-test -F chip-0057` line, properly placed between the `chia-sdk-test --all-features` and `chia-sdk-types` lines at 10-space indent.
- Full 8-gate Phase-gate verification sweep: 5 build permutations, fmt, scoped clippy under -D warnings, machete (with documented false-positive per plan's explicit guidance).

## Task Commits

Each task was committed atomically:

1. **Task 1: Add chip-0057 feature + optional chia-sdk-driver dep to chia-sdk-test/Cargo.toml** — `37cd7a25` (chore)
2. **Task 2: Cascade chip-0057 to chia-sdk-test in root Cargo.toml + add CI build line** — `7a0064f7` (chore)
3. **Task 3: Phase-1-gate verification — full workspace lint + machete sweep** — `3dde255d` (docs)

**Plan metadata commit:** [to be created]

## Files Created/Modified

- `crates/chia-sdk-test/Cargo.toml` — Added `[features].chip-0057` with 5-element cascade; added `chia-sdk-driver = { workspace = true, optional = true }` and `chia-sdk-utils = { workspace = true, optional = true }` to `[dependencies]`. 11 net insertions.
- `Cargo.toml` (workspace root) — Workspace `chip-0057` feature line now `["chia-sdk-driver/chip-0057", "chia-sdk-test/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]`. 1 line modified.
- `Cargo.lock` — Auto-regenerated; chia-sdk-test deps now include chia-sdk-driver and chia-sdk-utils entries.
- `.github/workflows/rust.yml` — Inserted `cargo build --release -p chia-sdk-test -F chip-0057` between the existing `chia-sdk-test --all-features` and `chia-sdk-types` lines. 1 net insertion.
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/deferred-items.md` — Records pre-existing chia-sdk-daemon/src/client.rs:426-427 clippy::pedantic warnings as out-of-scope; documents reasoning + recommended disposition.

## Decisions Made

- **Added `dep:chia-sdk-utils` to the chip-0057 cascade list** even though the plan's verbatim block only listed `dep:chia-sdk-driver`. Rationale: with `chia-sdk-utils` declared as `optional = true`, referencing `chia-sdk-utils/chip-0057` without an explicit `dep:` activator would rely on Cargo's legacy implicit-activation behavior (which emits a future-deprecation warning). Adding the explicit `dep:chia-sdk-utils` line keeps the feature future-proof and matches the explicit `dep:chia-sdk-driver` pattern. This is additive — the plan's locked acceptance grep counts still all return exactly the expected values (chip-0057 = [, dep:chia-sdk-driver, etc.); no acceptance criterion specifies `dep:chia-sdk-utils` count = 0.
- **Followed plan's documented false-positive path for cargo machete** rather than adding `[package.metadata.cargo-machete] ignored = ["chia-sdk-driver", "chia-sdk-utils"]`. Plan 06-02 lands tweak_data_from_simulator_block within Wave 1, consuming both deps and closing the gap. Documented in Task 1 + Task 3 commit messages and deferred-items.md.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added `dep:chia-sdk-utils` to chip-0057 feature cascade**

- **Found during:** Task 1 (Cargo.toml edits)
- **Issue:** The plan's verbatim feature block listed `dep:chia-sdk-driver` explicitly but not `dep:chia-sdk-utils`, while instructing the planner to add `chia-sdk-utils = { workspace = true, optional = true }` to `[dependencies]`. In modern Cargo (resolver 2 / edition 2024), referencing `chia-sdk-utils/chip-0057` in a feature list while `chia-sdk-utils` is an optional dep activates the dep implicitly but emits a future-deprecation warning. Plan's locked grep acceptance criteria do NOT specify `dep:chia-sdk-utils` count, so adding the explicit activator is purely additive — all locked grep counts still return their expected values.
- **Fix:** Added `"dep:chia-sdk-utils"` as the second element in the `chip-0057 = [...]` array (between `dep:chia-sdk-driver` and `chia-sdk-driver/chip-0057`).
- **Files modified:** crates/chia-sdk-test/Cargo.toml
- **Verification:** All 5 build permutations pass; scoped clippy under -D warnings clean; all plan-locked grep counts (chip-0057 = [, dep:chia-sdk-driver, chia-sdk-driver/chip-0057, chia-sdk-types/chip-0057, chia-sdk-utils/chip-0057, optional dep declarations) return exactly the expected values.
- **Committed in:** 37cd7a25 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical: forward-proofing optional-dep activation pattern)
**Impact on plan:** Pure addition; no scope creep. The plan's grep-locked acceptance criteria all pass; the change makes the cascade explicit-by-design rather than relying on legacy Cargo implicit-activation.

## Issues Encountered

- **cargo machete reports false-positive unused-deps for chia-sdk-driver + chia-sdk-utils on chia-sdk-test.** Anticipated by the plan: machete version 0.9.2 DOES flag optional deps that are not yet consumed (the plan's prose says it doesn't, but empirically that's outdated). Plan explicitly forbids adding these to `[package.metadata.cargo-machete] ignored` because Plan 06-02 will consume both deps via `tweak_data_from_simulator_block`. Documented as a false-positive in Task 1's commit message and in the new Phase 6 deferred-items.md. CI's `cargo machete` step (.github/workflows/rust.yml line 80) WILL fail on main between Plan 06-01 and Plan 06-02 landing — this is an accepted transient state per the plan's authoring intent (Wave 1 closes the gap).
- **chia-sdk-daemon/src/client.rs:426-427 clippy::pedantic warnings** fire under workspace `cargo clippy --workspace --all-features --all-targets -- -D warnings` but exit 0 under CI's permissive `cargo clippy --workspace --all-features --all-targets` (no `-D warnings`). Confirmed pre-existing (last touched in commit ec1a3517; predates Phase 6); already recorded in Phase 1's deferred-items.md; freshly logged in Phase 6's deferred-items.md.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Wave 1 (Plan 06-02) is **unblocked**: chia-sdk-test now has a chip-0057 feature, and `chia-sdk-driver` is reachable from chia-sdk-test under that feature so Plan 06-02 can place `tweak_data_from_simulator_block` and call into `compute_input_hash` + `aggregate_sender_sks` + `OutputMeta` + `TweakData` types.
- CI matrix already includes the new build line; no additional CI plumbing needed for Plan 06-02.
- No new workspace deps introduced — Phase 6's "no new workspace deps" constraint upheld.
- Zero new `#[allow]` attributes anywhere; zero `unsafe_code`; per-crate scoped clippy clean under -D warnings.

## Verification Summary (Task 3 sweep)

| Gate | Command | Result |
|------|---------|--------|
| 1 | `cargo build --release --workspace --all-features` | PASS (1m04s) |
| 2 | `cargo build --release --workspace` | PASS (1m03s) |
| 3 | `cargo build --release -p chia-sdk-test -F chip-0057` | PASS (1.72s) |
| 4 | `cargo build --release -p chia-sdk-test` | PASS (1.40s) |
| 5 | `cargo build --release -p chia-sdk-test --all-features` | PASS (12.87s) |
| 6 | `cargo clippy -p chia-sdk-test --all-features --all-targets -- -D warnings` (scoped to touched crate) | PASS |
| 6' | `cargo clippy --workspace --all-features --all-targets` (CI invocation, no -D warnings) | PASS (chia-sdk-daemon pre-existing warnings remain — documented in deferred-items.md) |
| 7 | `cargo fmt --all --check` | PASS |
| 8 | `cargo machete` | DOCUMENTED FALSE-POSITIVE (chia-sdk-driver, chia-sdk-utils — Plan 06-02 closes gap; per plan's explicit Task 1 step 4 + Task 3 step 8) |

## Plan Acceptance Criteria

All success criteria from the plan met:

- [x] chia-sdk-test/Cargo.toml has [features].chip-0057 cascading to chia-sdk-driver + chia-sdk-types + chia-sdk-utils
- [x] chia-sdk-test/Cargo.toml has chia-sdk-driver as an optional workspace dep
- [x] Root Cargo.toml's workspace chip-0057 feature includes chia-sdk-test/chip-0057
- [x] CI rust.yml has the per-crate chia-sdk-test -F chip-0057 build line
- [x] All build/clippy/fmt gates pass (machete documented false-positive per plan)
- [x] No new #[allow] attributes anywhere
- [x] Wave 1 (Plan 06-02) is unblocked

## Self-Check: PASSED

All claimed files exist on disk:
- crates/chia-sdk-test/Cargo.toml (modified)
- Cargo.toml (modified)
- Cargo.lock (modified)
- .github/workflows/rust.yml (modified)
- .planning/phases/06-simulator-round-trip-bindings-e2e-example/deferred-items.md (created)
- .planning/phases/06-simulator-round-trip-bindings-e2e-example/06-01-SUMMARY.md (this file)

All claimed commits exist in git history:
- 37cd7a25 (Task 1)
- 7a0064f7 (Task 2)
- 3dde255d (Task 3)

---
*Phase: 06-simulator-round-trip-bindings-e2e-example*
*Completed: 2026-05-18*
