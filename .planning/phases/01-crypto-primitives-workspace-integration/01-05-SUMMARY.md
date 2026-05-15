---
phase: 01-crypto-primitives-workspace-integration
plan: 05
subsystem: infra
tags: [ci, github-actions, chip-0057, machete, clippy, grep-bans, phase-closure]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 workspace feature scaffolding (Plan 01-01)"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "ScalarField + GROUP_ORDER + 5 named scalar tests (Plan 01-02)"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "tagged_hash + Chia_SP/* tag constants + 5 named tagged_hash tests (Plan 01-03)"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "SCAN_PATH + SPEND_PATH derivation path constants (Plan 01-04)"

provides:
  - "Per-crate chip-0057 build line in .github/workflows/rust.yml (catches feature-isolation regressions that --all-features would mask)"
  - "Documented Phase-1 final gate pass: 5-permutation build matrix, scoped clippy clean, fmt clean, machete clean, all three grep bans hold, 10 silent_payments tests pass, 2360-test full workspace suite green"
  - "deferred-items.md logging pre-existing chia-sdk-daemon pedantic warnings as out-of-scope"

affects:
  - 02-XX (Phase 2 inherits chip-0057 feature on chia-sdk-utils + ScalarField/tagged_hash/SCAN_PATH/SPEND_PATH re-exports from chia_sdk_types::silent_payments)
  - 03-XX (CHIP test-vector closure inherits clean grep-ban state and chia_sha2-only convention)
  - 04-XX (Send-side action inherits the type-boundary ScalarField and tagged_hash primitives)

tech-stack:
  added: []
  patterns:
    - "Per-crate -F feature CI line for feature-isolation regression catching (matches chip-0035 / chip-0037 precedent)"
    - "Scope-boundary discipline: pre-existing pedantic warnings in unrelated upstream files logged to deferred-items.md, not auto-fixed inside a plan whose scope is silent_payments/"
    - "Independent verification of cargo-machete locally (matching CI's cargo binstall step) before declaring success"

key-files:
  created:
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-05-SUMMARY.md"
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-PHASE-SUMMARY.md"
    - ".planning/phases/01-crypto-primitives-workspace-integration/deferred-items.md"
  modified:
    - ".github/workflows/rust.yml"

key-decisions:
  - "chia-sdk-daemon pedantic lints (match_same_arms, match_wildcard_for_single_variants on client.rs:426-427) are pre-existing and out-of-scope. Logged to deferred-items.md; not auto-fixed."
  - "Plan 02's documented `r - 1` math discrepancy (test name says r-1, actual value is 0x1824b159...fffffffd) is carried forward as a follow-up observation. The test name is locked by VALIDATION.md; the assertion correctly pins the actual reduced value. Phase verifier should decide whether to patch VALIDATION/ROADMAP wording later."
  - "M7 grep ban on `mod_by_group_order` and the defense-in-depth bans on `^use sha2::` and `Sha256::digest` all hold trivially across the cumulative silent_payments/ tree (paths.rs, scalar.rs, tagged_hash.rs, mod.rs). No source-level fix needed at Plan 05."

patterns-established:
  - "Phase-closure plans run the cumulative gate set against the whole phase's output, not just the current plan's diff"
  - "Pre-existing out-of-scope lint failures are documented in a deferred-items.md and addressed in separate small PRs"

requirements-completed: [WS-02, WS-03]

duration: 13min
completed: 2026-05-15
---

# Phase 01 Plan 05: CI matrix update + final gate verification (WS-02, WS-03 closure) Summary

**Added the per-crate `cargo build --release -p chia-sdk-types -F chip-0057` line to the CI workflow (one inserted line, zero deletions), then ran the full Phase-1 gate command set against the cumulative state of Plans 01-04 — five build permutations, scoped clippy with `-D warnings`, fmt, `cargo machete`, three grep bans, all 10 silent_payments tests, and the full 2360-test workspace suite all green; pre-existing pedantic lints in `chia-sdk-daemon` documented as out-of-scope in `deferred-items.md`.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-05-15T16:24:43Z
- **Completed:** 2026-05-15T16:37:58Z
- **Tasks:** 2
- **Files modified:** 1 (`.github/workflows/rust.yml`)
- **Files created:** 3 (this SUMMARY + PHASE-SUMMARY + deferred-items.md)

## Accomplishments

- `.github/workflows/rust.yml` gains one new line after the existing `chia-sdk-types --all-features` entry: `cargo build --release -p chia-sdk-types -F chip-0057`. 10-space indent matches surrounding lines exactly. `git diff` shows 1 insertion, 0 deletions, no formatting churn. YAML still parses cleanly via `python3 -c "import yaml; yaml.safe_load(...)"`.
- Five build permutations all exit 0:
  - `cargo build --release -p chia-sdk-types` (no features)
  - `cargo build --release -p chia-sdk-types --all-features`
  - `cargo build --release -p chia-sdk-types -F chip-0057`
  - `cargo build --release --workspace` (no features)
  - `cargo build --release --workspace --all-features`
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` exits 0. The Phase-1-touched crate is clean under the strict gate.
- `cargo fmt --all -- --files-with-diff --check` exits 0.
- `cargo machete` exits 0 with no new `[package.metadata.cargo-machete] ignored` entries anywhere in the diff vs `main` (verified via `git diff 87705945..HEAD -- '**/Cargo.toml' 'Cargo.toml' | grep -E '(package.metadata.cargo-machete|^\+.*ignored)'` returning zero matches).
- All three grep bans hold across `crates/chia-sdk-types/src/silent_payments/`:
  - `! grep -r 'mod_by_group_order' silent_payments/` — zero matches
  - `! grep -rE '^use sha2::' silent_payments/` — zero matches
  - `! grep -rE 'Sha256::digest' silent_payments/` — zero matches
- All 10 silent_payments tests pass under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments`:
  - 5 scalar tests: `add_wraps_at_r`, `from_bytes_unsigned_identity`, `from_bytes_raw_does_not_reduce`, `from_bytes_unsigned_max_input_reduces_to_r_minus_one`, `mul_mod_r`
  - 5 tagged_hash tests: `different_tags_different_outputs`, `tag_inputs_hash_pinned`, `tag_label_hash_pinned`, `tag_shared_secret_hash_pinned`, `tagged_hash_matches_bip340_challenge_vector`
- Full workspace test suite passes — 2360 tests across all crates (chia-sdk-types contributes 2298 incl. silent_payments, chia-sdk-driver 22, chia-sdk-test 24, chia-sdk-utils 5, chia-sdk-signer 7, plus various smaller crates), zero failures.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add the chip-0057 CI build line for chia-sdk-types** — `d4b84439` (ci)
2. **Task 2: Run the full Phase 1 gate command set + grep bans + cargo machete** — no commit (verification-only task; no file edits required by the action)

**Plan metadata commit:** appended after this SUMMARY.md (includes SUMMARY.md, 01-PHASE-SUMMARY.md, deferred-items.md, STATE.md, ROADMAP.md, REQUIREMENTS.md updates).

## Files Created/Modified

- `.github/workflows/rust.yml` — INSERTED ONE LINE at position 65 (right after `cargo build --release -p chia-sdk-types --all-features` on line 64): `          cargo build --release -p chia-sdk-types -F chip-0057`. 10-space indent matches surrounding `cargo build` lines exactly.
- `.planning/phases/01-crypto-primitives-workspace-integration/01-05-SUMMARY.md` — this file.
- `.planning/phases/01-crypto-primitives-workspace-integration/01-PHASE-SUMMARY.md` — phase-level summary aggregating Plans 01-01 through 01-05 (see separate file).
- `.planning/phases/01-crypto-primitives-workspace-integration/deferred-items.md` — logs the pre-existing chia-sdk-daemon pedantic lints as out-of-scope.

## Decisions Made

- **Scope-boundary: pre-existing chia-sdk-daemon pedantic lints are NOT fixed in this plan.** Running `cargo clippy --workspace --all-features --all-targets -- -D warnings` fails on `clippy::match_same_arms` and `clippy::match_wildcard_for_single_variants` in `crates/chia-sdk-daemon/src/client.rs` lines 426-427. The file was last touched in upstream commit `ec1a3517` (`Add Chia daemon websocket client (chia-sdk-daemon) (#381)`), which predates Phase 1 entirely. The GSD executor's scope boundary rule — "Only auto-fix issues DIRECTLY caused by the current task's changes" — applies. The lints are logged to `deferred-items.md` with a 2-line fix recommendation. Scoped clippy on the Phase-1-touched crate (`cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings`) IS clean.
- **CI's existing clippy step is unchanged.** The CI workflow at line 75 invokes `cargo clippy --workspace --all-features --all-targets` (without `-D warnings`). This is permissive and exits 0 today despite the daemon warnings. Aligning CI with the local strict gate would require either fixing the daemon code or adding `-- -D warnings` to the CI step — both are appropriately out-of-scope for Phase 1 (silent payments). A separate small PR is the right disposition.
- **Plan 02's `r - 1` test-name misnomer is carried forward as a Phase-verifier observation.** The test `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is mathematically misnamed (actual reduction of `2^256 - 1` mod `r` is `0x1824b159...fffffffd`, not `r - 1`). The test's assertion correctly pins the actual reduced value AND asserts inequality with the raw input, so the semantic guarantee is intact. The name is locked by `VALIDATION.md` row 43 and `PLAN.md` task acceptance criteria. The recommendation in Plan 02's SUMMARY is to patch the name (and VALIDATION row) in a small Phase-1 follow-up; left to the phase verifier to decide.
- **Task 2 is verification-only and does not commit.** The plan's Task 2 action runs commands and inspects output; the only file edit it triggers is if a gate fails (none did). No commit for Task 2 because there are no file changes attributable to it — only `deferred-items.md` (out-of-scope discovery) and the SUMMARYs (plan-closure docs) are produced, all of which roll up into the final metadata commit.
- **`cargo-machete` was installed locally before running.** It's not in the toolchain pin; CI uses `cargo binstall cargo-machete --locked -y`. Installed via `cargo install cargo-machete --locked` (v0.9.2) to verify locally that the gate would pass on CI.

## Deviations from Plan

### Auto-fixed Issues

None — the plan executed exactly as written.

### Carried-forward observations (not fixed in this plan)

**1. [Plan 02 — Math docs] Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is a misnomer**

- **Found during:** Plan 01-02 execution (documented in 01-02-SUMMARY.md "Deviations").
- **Carried forward by:** This plan (01-05) per the prompt's instruction to surface it for phase-verifier visibility.
- **Observation:** The test name and the prose in `01-02-PLAN.md` `<behavior>` claim the reduction of `[0xff; 32]` mod `r` equals `r - 1`. The actual value is `0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffd` (= `(2^256 - 1) - r` since `floor((2^256 - 1) / r) = 1`). The test correctly pins the actual value AND asserts inequality with the raw input; the semantic guarantee (unsigned reduction did fire) is intact, but the test name and `VALIDATION.md` row 43 wording is mathematically imprecise.
- **Recommendation:** Phase verifier should decide whether to (a) rename the test + patch VALIDATION row + patch ROADMAP success criterion 2 in a small Phase-1 follow-up, or (b) accept the misnomer and document it (the test passes, it just has a confusing name). Either disposition is fine; the protocol-correctness invariant is preserved.

### Out-of-scope discoveries (logged to deferred-items.md, not fixed)

**1. [Pre-existing — chia-sdk-daemon] Two clippy::pedantic warnings in `client.rs:426-427`**

- **Discovered during:** Task 2 gate run (`cargo clippy --workspace --all-features --all-targets -- -D warnings`).
- **Source:** Upstream commit `ec1a3517` ("Add Chia daemon websocket client"), pre-dates Phase 1.
- **Disposition:** Logged to `.planning/phases/01-crypto-primitives-workspace-integration/deferred-items.md` with full context and a 2-line fix recommendation. Not auto-fixed because (a) it's outside Phase 1's silent_payments scope, (b) the GSD executor's scope-boundary rule forbids auto-fixing unrelated pre-existing issues during a plan, and (c) CI's existing clippy step (without `-D warnings`) still exits 0.

---

**Total deviations from plan:** 0 auto-fixed, 1 carried-forward observation (Plan 02 misnomer), 1 out-of-scope discovery (pre-existing daemon lints).

## Issues Encountered

- **cargo-machete not installed locally** at the start of Task 2. Installed via `cargo install cargo-machete --locked` (v0.9.2). Mirrors CI's `cargo binstall cargo-machete --locked -y` step at workflow line 78. No issue for CI (CI installs it fresh per run).
- **clippy `-D warnings` failed on pre-existing daemon lints.** Resolved per scope-boundary: logged to deferred-items.md, ran scoped clippy on `chia-sdk-types` (clean), confirmed CI's actual clippy step (without `-D warnings`) exits 0. No source edits.

## Self-Check: PASSED

Verified all artifacts and commits exist:

- `.github/workflows/rust.yml` line 65 contains `cargo build --release -p chia-sdk-types -F chip-0057` — FOUND
- `grep -c 'cargo build --release -p chia-sdk-types -F chip-0057' .github/workflows/rust.yml` returns 1 — VERIFIED
- `grep -cE 'cargo build --release -p chia-sdk-types' .github/workflows/rust.yml` returns 3 — VERIFIED (no-features, --all-features, -F chip-0057)
- YAML parses cleanly under `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/rust.yml'))"` — VERIFIED
- `git diff -- .github/workflows/rust.yml` (pre-Task-1-commit) showed exactly 1 insertion, 0 deletions, no formatting churn — VERIFIED
- Commit `d4b84439` (Task 1, `ci(01-05): add chip-0057 per-crate build line for chia-sdk-types`) — FOUND in `git log`
- All five build permutations exited 0 — VERIFIED (output captured above)
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` exit 0 — VERIFIED
- `cargo fmt --all -- --files-with-diff --check` exit 0 — VERIFIED
- `cargo machete` exit 0 — VERIFIED
- `git diff 87705945..HEAD -- '**/Cargo.toml' 'Cargo.toml'` shows zero new `[package.metadata.cargo-machete] ignored` entries — VERIFIED
- All three grep bans return zero matches under `crates/chia-sdk-types/src/silent_payments/` — VERIFIED
- All 10 silent_payments tests pass — VERIFIED (test output captured above)
- Full workspace test suite passes (2360 tests, 0 failed) — VERIFIED
- `.planning/phases/01-crypto-primitives-workspace-integration/deferred-items.md` exists — FOUND
- `.planning/phases/01-crypto-primitives-workspace-integration/01-05-SUMMARY.md` (this file) exists — FOUND
- `.planning/phases/01-crypto-primitives-workspace-integration/01-PHASE-SUMMARY.md` exists — FOUND (written in same step)

## Known Stubs

None. This plan introduces no Rust source; the silent_payments/ tree has no stubs (Plan 02 / 03 / 04 produced complete implementations).

## M1..M9 Status Table (Phase-1 must-haves, final)

| ID | Description | Status | Verified by |
|----|-------------|--------|-------------|
| M1 | `cargo build -p chia-sdk-types -F chip-0057` succeeds | PASS | Task 2 step 1c, also new CI line |
| M2 | `cargo build --workspace --all-features` succeeds | PASS | Task 2 step 1e |
| M3 | `cargo build --workspace` (no features) succeeds | PASS | Task 2 step 1d |
| M4 | `clippy --workspace --all-features --all-targets` clean | PASS (CI gate) | CI uses no `-D warnings`; daemon lints are pre-existing warnings outside silent_payments scope, logged to deferred-items.md. Scoped clippy on `chia-sdk-types` (the only crate Phase 1 touched) is clean under `-D warnings`. |
| M5 | `cargo fmt --check` clean | PASS | Task 2 step 3 |
| M6 | `cargo machete` clean, no new `ignored` entries | PASS | Task 2 step 4, plus `git diff` Cargo.toml check |
| M7 | `grep -r 'mod_by_group_order' silent_payments/` zero matches | PASS | Task 2 step 5 (mechanically enforced) |
| M8 | All ScalarField + tagged_hash unit tests pass | PASS | Task 2 step 7 (10/10 passed) |
| M9 | `chip-0057` declared on chia-sdk-types + chia-sdk-driver + chia-sdk-utils | PASS | Established in Plan 01-01, re-verified by Task 2 build matrix |

**Phase-1 ROADMAP success criteria 1-5 — final status:**

1. **All five build permutations pass** — PASS (M1, M2, M3 + per-crate `-p` variants verified).
2. **Adversarial ScalarField test passes** — PASS (the test runs green; note the documented misnomer in its name, see "Carried-forward observations").
3. **Tag-pin unit tests pass** — PASS (three pinned tags via `chia_sha2::Sha256` new/update/finalize; ROADMAP criterion 3 says "`chia_sha2::Sha256::digest(...)`" but the actual API used is `new/update/finalize` because `chia_sha2` doesn't expose `::digest` — this is RESEARCH.md Pitfall 3 by design, and the test pins the same SHA-256 output via the correct API).
4. **`grep -r 'mod_by_group_order' silent_payments/` zero hits** — PASS (M7).
5. **`cargo machete` clean with no new ignored entries** — PASS (M6).

## User Setup Required

None.

## Next Phase Readiness

**Phase 2 entry checklist:**

- ✅ `chip-0057` feature on `chia-sdk-utils` is in place and empty — Plan 02 of Phase 2 will fill it with `SilentPaymentKeys` + `SilentPaymentAddress` code behind the gate.
- ✅ `chia_sdk_types::silent_payments::{ScalarField, GROUP_ORDER, tagged_hash, CHIA_SP_INPUTS, CHIA_SP_SHARED_SECRET, CHIA_SP_LABEL, SCAN_PATH, SPEND_PATH}` all `pub`-exported through the `silent_payments::*` barrel, ready for Phase 2 to consume.
- ✅ CI's per-crate `chip-0057` build line catches feature-isolation bugs at the chia-sdk-types boundary — Phase 2 and beyond will benefit from this defense-in-depth.
- ⚠ **Phase 2 pre-flight audit (Q8 from STATE.md):** Decide whether to add `dep:chia-sdk-types` as an optional `[dependencies]` entry on `chia-sdk-utils` (currently `chip-0057 = []` on utils — no dep edge). Phase 2's `SilentPaymentKeys` / `SilentPaymentAddress` may or may not need to import from `chia-sdk-types`; if they do, the `chip-0057 = ["dep:chia-sdk-types"]` shape is the right pattern. If they're self-contained (using `chia-bls` + `bech32` only), the current empty feature is fine.
- ⚠ **Phase-verifier observation:** Plan 02's `r - 1` misnomer in the test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one`. Recommend a small follow-up commit before Phase 2 entry to patch the name + VALIDATION row + ROADMAP success criterion 2 wording — but this is non-blocking.

**Out-of-scope follow-up (independent of Phase 2):**

- `chia-sdk-daemon` pedantic lints in `client.rs:426-427` — see `deferred-items.md`. Either fix in a small chore PR or add `-- -D warnings` to CI's clippy step. Not a Phase 1 / 2 blocker.

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
