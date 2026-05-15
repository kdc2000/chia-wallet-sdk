---
phase: 02-address-key-types
plan: 01
subsystem: infra
tags: [chip-0057, cargo-features, silent-payments, chia-sdk-utils, module-scaffold]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 feature flag pattern (chip-0037 precedent), chia-sdk-types/silent_payments primitives (ScalarField, tagged_hash, SCAN_PATH, SPEND_PATH), workspace cascade from root Cargo.toml chip-0057 -> chia-sdk-utils/chip-0057"
provides:
  - "chia-sdk-utils chip-0057 feature now activates dep:chia-sdk-types, dep:bip39, dep:chia-bls (gated)"
  - "chia_sdk_utils::silent_payments module exists as cfg-gated public barrel (empty, ready for submodules)"
  - "Q8 (Phase 2 pre-flight audit) resolved: dep:chia-sdk-types edge on chia-sdk-utils confirmed gated; no-features build unaffected"
affects: [02-02-error-types, 02-03-address-encoding, 02-04-keys-and-labels, 02-05-prelude-and-gate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Optional dep + feature pattern: `dep:<crate>` activation list on the feature line + `optional = true` on each dep line (Cargo features 2.0 idiom; matches chip-0035 / chip-0037 precedent elsewhere in the workspace)"
    - "cfg-gated module barrel: `#[cfg(feature = \"chip-0057\")] pub mod silent_payments;` exposes the submodule tree only when the feature is on"
    - "Doc-only module barrel pattern: mod.rs ships with rich rustdoc (linking to types that don't yet exist via intra-doc reference syntax) and a sentinel comment block reserving slots for sorted-order submodule declarations from downstream plans"

key-files:
  created:
    - "crates/chia-sdk-utils/src/silent_payments/mod.rs (31 lines, doc-comment + sentinel)"
  modified:
    - "crates/chia-sdk-utils/Cargo.toml (chip-0057 feature line + 3 optional dep lines)"
    - "crates/chia-sdk-utils/src/lib.rs (cfg-gated pub mod silent_payments)"

key-decisions:
  - "Plan 02-01 Task 3 (prelude.rs re-export block) NO-OP'd — deferred to Plan 02-05 to avoid chicken-and-egg compile error where pub use references not-yet-existing types"
  - "Module barrel ships EMPTY of submodule declarations — Plans 02-02/03/04 append in sorted alphabetical order (matches Phase 1's paths < scalar < tagged_hash convention in chia-sdk-types/src/silent_payments/mod.rs)"
  - "Doc-comment respects Phase 1 grep bans inherited from chia-sdk-types/silent_payments: no `mod_by_group_order` literal token, no `use sha2::` import; phrased as `signed mod-r reducer` instead"
  - "No new workspace deps added — chia-sdk-types, bip39, chia-bls already present in [workspace.dependencies] (verified at lines 87, 144, 127 of root Cargo.toml)"
  - "No new [package.metadata.cargo-machete] ignored entries — optional/feature-gated deps are visible to machete in the chip-0057 build path"

patterns-established:
  - "Phase 2 module-scaffold plan structure: 3 surgical edits (Cargo.toml feature wiring + lib.rs cfg-gate + new mod.rs barrel), 8 build/grep verification gates, defer prelude.rs to final phase gate plan"

requirements-completed: []  # Plan 02-01 frontmatter lists requirements [ADDR-01..06] as POSSIBLE eventual coverage, but requirements_addressed: [] explicitly — this is pure scaffolding. ADDR-01..06 close in Plans 02-02 through 02-05.

# Metrics
duration: 18min
completed: 2026-05-15
---

# Phase 02 Plan 01: Wiring & feature gate — chip-0057 deps on chia-sdk-utils + silent_payments module barrel Summary

**chia-sdk-utils crate now optionally depends on chia-sdk-types, bip39, chia-bls under the chip-0057 feature; silent_payments module barrel is in place for downstream plans to populate.**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-05-15T19:14:04Z
- **Completed:** 2026-05-15T19:31:56Z
- **Tasks:** 3 (2 implementing + 1 intentional NO-OP)
- **Files modified:** 3 (Cargo.toml + lib.rs + new mod.rs)

## Accomplishments
- Wired the `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]` activation pattern on `chia-sdk-utils`, with three matching `optional = true` dep lines.
- Resolved STATE.md Blocker Q8 ("Audit downstream consumers of chia-sdk-utils for the new optional `chia-sdk-types -> chia-sdk-utils` dep edge"): YES, gated by feature; no-features workspace build still compiles cleanly.
- Created `crates/chia-sdk-utils/src/silent_payments/mod.rs` as a doc-only barrel mirroring Phase 1's `chia-sdk-types/src/silent_payments/mod.rs` shape (descriptive doc-comment, no submodule declarations yet, sentinel comment reserving sorted-order slot for Plans 02-02/03/04).
- Added `#[cfg(feature = "chip-0057")] pub mod silent_payments;` to `crates/chia-sdk-utils/src/lib.rs` — the umbrella access path `chia_sdk_utils::silent_payments::*` is now reserved.
- All 8 plan-level gate commands pass: 4 build matrix permutations (per-crate ± chip-0057, workspace ± all-features), 1 fmt check, 2 grep bans (`mod_by_group_order`, `^use sha2::`), and 1 untouched-file check on `src/prelude.rs`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Promote chip-0057 feature on chia-sdk-utils and add three optional deps** — `3553e9d8` (feat)
2. **Task 2: Add silent_payments module declaration to lib.rs + create empty mod.rs barrel** — `81b8cf1e` (feat)
3. **Task 3: NO-OP (prelude.rs deferred to Plan 02-05)** — no commit, verified `git diff --quiet -- src/prelude.rs` exits 0

**Plan metadata commit:** to follow this SUMMARY.

## Files Created/Modified

- `crates/chia-sdk-utils/Cargo.toml` — Promoted `chip-0057 = []` to `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]`; appended three `optional = true` dep lines (`chia-bls`, `chia-sdk-types`, `bip39`).
- `crates/chia-sdk-utils/src/lib.rs` — Appended `#[cfg(feature = "chip-0057")] pub mod silent_payments;`.
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` — NEW. 31-line doc-comment + sentinel; module is a public barrel reserved for Plans 02-02/03/04.

## Decisions Made

- **Task 3 deferred to Plan 02-05.** The plan explicitly carries Task 3 as a documented NO-OP (lines 322-346): appending a `pub use chia_sdk_utils::silent_payments::{...}` block to `src/prelude.rs` now would reference five not-yet-existing public types and break the `chip-0057` build. Defer to Plan 02-05 (final gate) where all four types exist. Validated by `git diff --quiet -- src/prelude.rs` (exits 0 with no changes).
- **Doc-comment phrasing chosen to satisfy two inherited Phase 1 grep bans:** the literal `mod_by_group_order` token (banned across `silent_payments/` to prevent reintroducing the signed reducer route) and `^use sha2::` (defense-in-depth ban; `chia_sha2::Sha256` is the SDK-standard hasher). Doc-comment uses phrases like "signed mod-r reducer", "the standard-puzzle synthetic-key offset", and references `chia_puzzle_types::derive_synthetic` to convey the same content without tripping the bans.

## Deviations from Plan

None — plan executed exactly as written. Task 3 was a documented NO-OP per the plan's own decision (lines 329-345), not a deviation.

## Issues Encountered

None. Both builds-with-feature-on and builds-with-feature-off succeeded on first try with no compilation friction.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready for Plan 02-02 (error module):**
- The `chip-0057` feature on `chia-sdk-utils` is fully wired end-to-end. Plan 02-02 can `use chia_sdk_types::silent_payments::*`, `use chia_bls::*`, and `use bip39::*` inside `#[cfg(feature = "chip-0057")]` modules without further Cargo.toml edits.
- `silent_payments/mod.rs` is ready to accept the first submodule declaration. Sorted-order convention (matching Phase 1): `address < error < keys < labels`. Plan 02-02 lands `error` first (alphabetically second after a hypothetical `address`, but Plan 02-02 is scheduled before 02-03).
- All 8 plan-level gates green; no deferred items raised; no machete entries added; no clippy work needed yet (gated to Plan 02-05's final gate per plan's explicit deferral).

**Blockers/concerns:** None for Plan 02-02. The Q8 audit blocker (Phase 2 pre-flight) is resolved by this plan.

## Self-Check

- [x] `crates/chia-sdk-utils/Cargo.toml` modified — verified via `git diff --stat` (4 insertions, 1 deletion, 1 file changed)
- [x] `crates/chia-sdk-utils/src/lib.rs` modified — verified
- [x] `crates/chia-sdk-utils/src/silent_payments/mod.rs` created — verified via `test -f` and `wc -l` (31 lines)
- [x] Commit `3553e9d8` exists — verified via `git log --oneline -5`
- [x] Commit `81b8cf1e` exists — verified via `git log --oneline -5`
- [x] All 8 plan-level gate commands exit 0 (4 build permutations, fmt, 2 grep bans, untouched-prelude check)

## Self-Check: PASSED

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
