# Phase 01 — Deferred Items

Out-of-scope issues discovered during Phase 1 execution. Logged per the GSD executor scope-boundary rule ("Only auto-fix issues DIRECTLY caused by the current task's changes").

## Pre-existing clippy::pedantic warnings in `chia-sdk-daemon`

**Discovered:** 2026-05-15 during Plan 01-05 final gate run.

**Location:** `crates/chia-sdk-daemon/src/client.rs` lines 426-427.

**Lints firing:**
1. `clippy::match_same_arms` — two `tungstenite::Message::*` match arms have identical bodies (`Ping`, `Pong` both `continue`, and `_ => continue` follows).
2. `clippy::match_wildcard_for_single_variants` — the `_ => continue` arm only covers `tungstenite::Message::Frame(_)`.

**Behavior:**
- CI's clippy invocation (`cargo clippy --workspace --all-features --all-targets`, line 75 of `.github/workflows/rust.yml`) does NOT pass `-D warnings`, so these surface as warnings only and the CI clippy step exits 0 today.
- Local stricter gate `cargo clippy --workspace --all-features --all-targets -- -D warnings` (specified in 01-05-PLAN.md) treats them as errors.
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` (scoped to Phase-1's only touched crate) IS clean.

**Why deferred:**
- `chia-sdk-daemon/src/client.rs` was last touched in commit `ec1a3517` ("Add Chia daemon websocket client"), which predates Phase 1 entirely.
- Phase 1 introduced no changes to `chia-sdk-daemon`. The warnings are pre-existing and entirely outside the silent_payments scope.
- Per the GSD scope-boundary rule, pre-existing lint warnings in unrelated files are not auto-fixed during a plan.

**Recommended disposition:**
- Open a separate small PR (e.g., `chore: silence pedantic match warnings in chia-sdk-daemon`) that collapses the `Ping | Pong | _ => continue` arms into a single `_ => continue` and replaces the bare `_` with `tungstenite::Message::Frame(_) | tungstenite::Message::Ping(..) | tungstenite::Message::Pong(..) => continue` (or similar). This is a 2-line change in a single function and has no behavioral impact.
- OR: align CI's clippy step with the local gate by adding `-- -D warnings` (currently CI is permissive). Either resolves the discrepancy.

**NOT a Phase 1 blocker.** All five build permutations pass, scoped clippy passes, and the existing CI clippy step (without `-D warnings`) passes.
