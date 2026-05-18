# Phase 06 — Deferred Items

Out-of-scope issues discovered during Phase 6 execution. Logged per the GSD executor scope-boundary rule ("Only auto-fix issues DIRECTLY caused by the current task's changes").

## Pre-existing clippy::pedantic warnings in `chia-sdk-daemon`

**Discovered:** 2026-05-18 during Plan 06-01 Task 3 sweep.

**Location:** `crates/chia-sdk-daemon/src/client.rs` lines 426-427.

**Lints firing:**
1. `clippy::match_same_arms` — two `tungstenite::Message::*` match arms have identical bodies.
2. `clippy::match_wildcard_for_single_variants` — `_ => continue` only covers `tungstenite::Message::Frame(_)`.

**Behavior:**
- CI's clippy invocation (`cargo clippy --workspace --all-features --all-targets`, `.github/workflows/rust.yml:77`) does NOT pass `-D warnings`, so these surface as warnings only and the CI clippy step exits 0 today.
- Local stricter gate `cargo clippy --workspace --all-features --all-targets -- -D warnings` (specified in this plan's Task 3) treats them as errors.

**Why deferred:**
- `chia-sdk-daemon/src/client.rs` was last touched in commit `ec1a3517` ("Add Chia daemon websocket client"), which predates Phase 1 entirely.
- Plan 06-01 touched only `crates/chia-sdk-test/Cargo.toml`, root `Cargo.toml`, and `.github/workflows/rust.yml`. None of those edits reach `chia-sdk-daemon`.
- Already recorded in `.planning/phases/01-crypto-primitives-workspace-integration/deferred-items.md`; this entry is the Phase 6 reference.
- Per the GSD scope-boundary rule, pre-existing lint warnings in unrelated files are not auto-fixed during a plan.

**Recommended disposition:**
- Open a separate small PR (e.g., `chore: silence pedantic match warnings in chia-sdk-daemon`) that consolidates the match arms.
- OR: align CI's clippy step with the local gate by adding `-- -D warnings` (currently CI is permissive). Either resolves the discrepancy.

**NOT a Plan 06-01 blocker.** All five build permutations pass; CI clippy (without `-D warnings`) passes; scoped clippy on chia-sdk-test passes.

## Pre-existing `missing_copy_implementations` on `SendDestination` (no-features build)

**Discovered:** 2026-05-18 during Plan 06-03 Task 3 verification sweep.

**Location:** `crates/chia-sdk-driver/src/action_system/send_destination.rs:31`.

**Behavior:**
- Without `chip-0057`, `SendDestination` only has one variant (`PuzzleHash(Bytes32)`); the workspace `warn missing_copy_implementations` lint suggests `impl Copy`.
- Under `chip-0057`, the enum gains `SilentPayment(Box<SilentPaymentAddress>)`, which is not `Copy`, so the lint correctly does not fire.
- Workspace lint policy treats this as a `warning`, not `deny`. `cargo build --release -p chia-sdk-driver` (no features) reports the warning but exits 0. The default workspace `warn` level for missing_copy_implementations is correct.

**Why deferred:**
- The lint pre-existed at Plan 06-03 start (verified by `git checkout HEAD~2 -- crates/chia-sdk-driver/src/silent_payments/` + `cargo build -p chia-sdk-driver`).
- Originates from Phase 04.2 Plan 04.2-01's introduction of `SendDestination`; not in scope for Plan 06-03 (Wave 3 E2E tests).
- Adding `#[derive(Copy)]` to `SendDestination` would require it to be unconditional and would break under `chip-0057` (Box is not Copy). The proper fix is either a feature-gated conditional derive (complex) or accepting the warning.

**NOT a Plan 06-03 blocker.** Workspace-level `cargo build --release -p chia-sdk-driver` exits 0 (warning only). All 3 Plan 06-03 E2E tests pass; scoped `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` passes cleanly because under chip-0057 the lint does not fire.
