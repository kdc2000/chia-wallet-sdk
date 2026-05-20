---
phase: 07-code-review-cleanup
plan: 05
subsystem: testing
tags: [chip-0057, silent-payments, integration-tests, cargo-features, cyclic-dev-dep]

# Dependency graph
requires:
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: chia_sdk_test::silent_payments::tweak_data_from_simulator_block canonical helper (SIM-01)
  - phase: 07-code-review-cleanup (Plan 01)
    provides: CLEANUP-01 strip-reread-repair on 17 silent_payments files; e2e.rs was deliberately deferred to this plan per RESEARCH Open Q2 sequencing
provides:
  - Integration-test target hosting the 3 silent_payments e2e tests (unlabeled, labeled, m=0 self-change)
  - Cyclic-dev-dep workaround root-cause fix: in-place inlined build_tweak_data helper deleted, tests now call canonical chia-sdk-test helper directly
  - chip-0057 feature cascade extended with chia-sdk-test/chip-0057 so integration target compiles
affects: [future code-review cycles, downstream consumers using tests/ as integration-test precedent]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cargo integration-test target as cyclic-dev-dep escape hatch: when crate A's #[cfg(test)] module needs to call a helper in crate B that itself depends on A, relocate to tests/ to resolve the cycle"
    - "Atomic two-task pattern: Task 1 ships the new target side-by-side with the old in-place tests; Task 2 deletes the old tests after the new target is verified passing"

key-files:
  created:
    - "crates/chia-sdk-driver/tests/silent_payments_e2e.rs (~250 lines; 3 #[test] functions + setup_e2e helper)"
  modified:
    - "crates/chia-sdk-driver/Cargo.toml (chip-0057 feature line extended with chia-sdk-test/chip-0057)"
    - "crates/chia-sdk-driver/src/silent_payments/mod.rs (#[cfg(test)] mod e2e; declaration removed)"
  deleted:
    - "crates/chia-sdk-driver/src/silent_payments/e2e.rs (358 lines — entire file + inlined build_tweak_data helper)"

key-decisions:
  - "Resolved cyclic-dev-dep type-confusion at the build-system level (integration-test target) rather than maintaining the Phase 6 in-place inlined workaround"
  - "Pre-existing missing_copy_implementations warning on SendDestination remains out-of-scope per Phase 6 deferred-items.md precedent — scoped clippy under chip-0057 with -D warnings is the relevant gate"

patterns-established:
  - "tests/ integration target as cycle breaker: when a chip-0057-gated test in crate A wants to call a chip-0057-gated helper in crate B (with B's chip-0057 cascade depending on A), the integration-test compile unit resolves to a single type identity"
  - "Feature cascade onto dev-dependencies: chip-0057 feature line can include chia-sdk-test/chip-0057 even though chia-sdk-test is in [dev-dependencies] only — Cargo accepts the syntax"

requirements-completed: [CLEANUP-04]

# Metrics
duration: 17min
completed: 2026-05-20
---

# Phase 07 Plan 05: Relocate silent_payments e2e tests to integration target Summary

**Cyclic-dev-dep workaround (inlined build_tweak_data, 358 lines) deleted; 3 e2e tests relocated to crates/chia-sdk-driver/tests/silent_payments_e2e.rs calling the canonical chia_sdk_test helper directly.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-05-20T16:34:35Z
- **Completed:** 2026-05-20T16:51:50Z
- **Tasks:** 2
- **Files modified:** 4 (1 created, 2 modified, 1 deleted)

## Accomplishments

- The Phase 6 expedient workaround in `crates/chia-sdk-driver/src/silent_payments/e2e.rs` (inlined `build_tweak_data` helper, byte-for-byte copy of the canonical `tweak_data_from_simulator_block`) is gone. The 3 tests live at `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`, and every `tweak_data_from_simulator_block` call site goes through the canonical chia-sdk-test helper.
- The cyclic-dev-dep type-confusion (chia-sdk-driver → chia-sdk-test → chia-sdk-driver) that originally forced the inlining is resolved by Cargo's integration-test build model — the new target builds against the lib's published types only, so `TweakData` is one type identity.
- CLEANUP-01's residual 4 grep hits inside `e2e.rs` (deliberately skipped by Plan 01 per RESEARCH Open Q2 sequencing) are resolved by the file deletion. The full CLEANUP-01 acceptance grep across all 17 target files now returns 0.
- Phase 7 is requirement-complete: CLEANUP-01 + CLEANUP-02 + CLEANUP-03 + CLEANUP-04 + CLEANUP-06 all `[x]`.

## Task Commits

1. **Task 1: Create the integration test target + extend chip-0057 cascade** - `e02989e7` (test)
2. **Task 2: Delete src/silent_payments/e2e.rs and remove mod declaration** - `35211a2a` (refactor)

## Files Created/Modified

- `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` — NEW; ~250 lines; module rustdoc explains the cycle-resolution rationale; 3 `#[test]` functions (`test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change`) plus shared `setup_e2e` helper. Imports `chia_sdk_test::silent_payments::tweak_data_from_simulator_block` directly.
- `crates/chia-sdk-driver/Cargo.toml` — chip-0057 feature line expanded from one entry to four entries (now a multi-line array): `chia-sdk-types/chip-0057`, `dep:chia-sdk-utils`, `chia-sdk-utils/chip-0057`, `chia-sdk-test/chip-0057`. The new last entry cascades chip-0057 onto the dev-dep so the integration target sees `chia_sdk_test::silent_payments` at compile time.
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — `#[cfg(test)] mod e2e;` declaration deleted (2 lines).
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` — DELETED (358 lines).

## Cargo.toml chip-0057 feature diff

Before:

```toml
chip-0057 = ["chia-sdk-types/chip-0057", "dep:chia-sdk-utils", "chia-sdk-utils/chip-0057"]
```

After:

```toml
chip-0057 = [
    "chia-sdk-types/chip-0057",
    "dep:chia-sdk-utils",
    "chia-sdk-utils/chip-0057",
    "chia-sdk-test/chip-0057",
]
```

## Integration target test results

```
running 3 tests
test test_simulator_e2e_m0_self_change ... ok
test test_simulator_e2e_unlabeled ... ok
test test_simulator_e2e_labeled ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Driver test count delta: -3 unit tests in `src/silent_payments/e2e.rs` + 3 integration tests in `tests/silent_payments_e2e.rs` = net 0 (1088 -> 1088 across the full chia-sdk-driver test binary set).

## cargo machete result

Clean — `cargo-machete didn't find any unused dependencies in this directory`. The five test-only dev-deps that previously supported `src/silent_payments/e2e.rs` (anyhow, bip39, indexmap, chia-sdk-test, chia-sdk-utils) are all still consumed by the integration test target, so no dev-dep needs removal. No new entries added to any `[package.metadata.cargo-machete] ignored` list.

## CLEANUP-01 acceptance grep result

```bash
grep -rE 'CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-6]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]' \
  crates/*/src/silent_payments/ \
  crates/chia-sdk-bindings/src/silent_payments.rs \
  crates/chia-sdk-driver/src/action_system/send_destination.rs \
  examples/silent_payment.rs \
  napi/__test__/silent_payments*.ts \
  pyo3/tests/test_silent_payments.py \
  wasm/__test__/silent_payments.spec.ts | wc -l
# => 0
```

Plan 01 deliberately left `e2e.rs` untouched per RESEARCH Open Q2 recommendation; this plan's file deletion closes the residual 4 hits. CLEANUP-01 is now fully satisfied without an exclusion clause.

## Decisions Made

- **Followed the plan's two-task atomic pattern** rather than committing both Cargo.toml + new file + deletion in one commit. Task 1's commit leaves the old in-place tests intact, so the workspace is fully green (both old and new tests passing) at Task 1's tip; Task 2 then performs the deletion against that known-good baseline. Both commits land within ~5 minutes of each other.
- **Did not auto-fix the pre-existing `missing_copy_implementations` warning on `SendDestination`** observed during `cargo clippy -p chia-sdk-driver --all-targets -- -D warnings` (no-features). Verified pre-existing on main via `git stash` + clippy re-run; matches the Phase 6 / Plan 06-03 deferred-items.md documentation. The relevant gate per established precedent is `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`, which is clean.
- **Did not auto-fix the pre-existing `match_wildcard_for_single_variants` warnings in chia-sdk-daemon** observed during `cargo clippy --workspace --all-features --all-targets -- -D warnings`. Same disposition: pre-existing on main (Phase 1 deferred-item), CI-style `cargo clippy --workspace --all-features --all-targets` (no `-D warnings`) exits 0.

## Deviations from Plan

None — plan executed exactly as written. No auto-fixes triggered. No new dependencies. No new `#[allow]` attributes. No `cargo machete` ignored-list additions.

The plan's anticipated `bip39` machete false-positive (RESEARCH Open Q4) did not materialize because `bip39` continues to be consumed by `tests/silent_payments_e2e.rs::TV1_MNEMONIC` parsing — same as it was consumed by the deleted `src/silent_payments/e2e.rs`.

## Issues Encountered

- **rustfmt re-wrap on `use` import** — initial integration test file had a multi-line `use chia_sdk_driver::{...}` block that rustfmt prefers to collapse to one line (78 columns under the default 100-column threshold). Fixed inline in Task 2's commit; Task 1's commit also adjusted retrospectively had no value, so the rustfmt fix landed alongside the deletion. `cargo fmt --all --check` exits 0 after Task 2.

## Self-Check

Verified:

- `crates/chia-sdk-driver/tests/silent_payments_e2e.rs` exists.
- `crates/chia-sdk-driver/src/silent_payments/e2e.rs` does not exist.
- Commits `e02989e7` and `35211a2a` both present in `git log --oneline --all`.
- All 3 integration tests pass via `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e`.
- Full workspace `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` exits 0 (2332 driver tests + 3 integration + utils/types/test crate tests all green).
- `cargo fmt --all --check` exits 0.
- `cargo machete` exits 0.
- Scoped `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0.
- CLEANUP-01 acceptance grep returns 0.

## Self-Check: PASSED

## Next Phase Readiness

**Phase 7 is requirement-complete.** All 5 CLEANUP-* requirements are now closed:

- CLEANUP-01 (Plan 07-01 + this plan's file deletion)
- CLEANUP-02 (Plan 07-02 — actions/silent_payment_send.rs extraction)
- CLEANUP-03 (Plan 07-03 — Spends::finish_silent_payments rolled into Spends::prepare)
- CLEANUP-04 (this plan)
- CLEANUP-06 (Plan 07-04 — VALIDATION.md frontmatter auto-flip + 7 stale files repaired)

v1 silent-payments work remains requirement-complete (34/34 functional + 5/5 polish). Ready for milestone close.

---
*Phase: 07-code-review-cleanup*
*Completed: 2026-05-20*
