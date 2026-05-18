---
phase: 05-bindings-rust-facade-json-descriptor
plan: 01
subsystem: bindings
tags: [chip-0057, bindy, bindy-macro, napi, pyo3, wasm, silent-payments, wave-0, pre-flight-gate]

# Dependency graph
requires:
  - phase: 04.2-unify-sp-send-into-action-send-via-senddestination-enum
    provides: post-04.2 unified Action::send + SendDestination enum + Spends::with_silent_payment_keys (the final Rust API surface Phase 5 binds)
provides:
  - chip-0057 wired unconditionally onto chia-sdk-bindings's three workspace deps (driver, utils, types) per D-01
  - SilentPayments unit-struct facade in crates/chia-sdk-bindings/src/silent_payments.rs (Wave 0 stub with probe_noop static)
  - bindings/silent_payments.json Wave 0 stub (zero-field SilentPayments class with one static method)
  - empty napi/__test__/silent_payments.spec.ts (test.skip placeholder Plan 05-04 fills in)
  - Wave 0 pre-flight verdict NATIVE SUPPORT CONFIRMED (PASS) recorded in 05-PHASE-NOTES.md
  - 05-VALIDATION.md frontmatter wave_0_complete:true + nyquist_compliant:true + per-task map populated with concrete plan IDs (05-01..05-04)
affects:
  - 05-02 (Wave 1 expands stub to full 9-class descriptor + ~250-line facade)
  - 05-03 (Wave 2 builds the three target binding crates and asserts index.d.ts shape)
  - 05-04 (Wave 3 fills in the silent_payments.spec.ts AVA round-trip test)

# Tech tracking
tech-stack:
  added: []  # zero new workspace deps; chip-0057 feature already existed phase-1-onward
  patterns:
    - "Three-part bindy contribution: Rust facade + bindings/*.json descriptor + (rare) hand-written shim — Wave 0 lands the first two parts in minimal form"
    - "Unconditional feature wiring on chia-sdk-bindings dep declarations (D-01) — mirrors existing offer-compression and action-layer always-on pattern; NO chip-0057 cargo feature on chia-sdk-bindings itself; NO #[cfg(feature = \"chip-0057\")] gates inside the facade"
    - "Wave 0 pre-flight gate pattern: minimal stub + 4-target build probe before committing to the full descriptor surface (de-risks SC4)"

key-files:
  created:
    - crates/chia-sdk-bindings/src/silent_payments.rs (28 lines; stub facade)
    - bindings/silent_payments.json (11 lines; 1-class stub)
    - napi/__test__/silent_payments.spec.ts (8 lines; test.skip placeholder)
    - .planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-NOTES.md (Wave 0 verdict)
  modified:
    - crates/chia-sdk-bindings/Cargo.toml (3 dep-feature edits per D-01)
    - crates/chia-sdk-bindings/src/lib.rs (mod silent_payments + pub use silent_payments::* — alphabetical between secp and simulator)
    - .planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md (frontmatter flags + per-task map + sign-off block)

key-decisions:
  - "Wave 0 PASS — bindy-macro natively supports static methods on zero-field classes across napi/wasm/pyo3 (all 4 target builds exit 0); SC4 primary path confirmed; fallback strategy NOT needed"
  - "Honored CLAUDE.md GSD workflow + D-01 'no feature on chia-sdk-bindings, no #[cfg] gates in facade' constraint — chip-0057 reaches the facade purely via unconditional dep features"
  - "Probe ran with plain `cargo build -p chia-wallet-sdk-py` / `chia-wallet-sdk-wasm` (no --features flag) because neither crate declares any features; effect identical to the plan's listed command since features flow through the chia-sdk-bindings dep"

patterns-established:
  - "Wave 0 pre-flight gate ritual: stub one minimal class + run all target builds before authoring the full descriptor — keeps SC4 risk out of the critical path"
  - "Single source of feature truth: features = [\"chip-0057\"] on the chia-sdk-bindings -> chia-sdk-{driver,utils,types} dep edges, no propagation needed elsewhere because the per-target binding crates already inherit the bindings dep"

requirements-completed: []
# BIND-01 and BIND-02 are NOT closed by this plan — Wave 0 only proves the pipeline + lands stubs.
# The plan's frontmatter listed requirements: [BIND-01, BIND-02] but those close in Plan 05-04 (full surface + AVA + drift check).
# Per success_criteria: BIND-01 "Wave 0 portion" satisfied (facade + JSON + chip-0057 wiring); BIND-02 "Wave 0 portion" satisfied (static-functions schema confirmed available). Closure happens later.

# Metrics
duration: 3min
completed: 2026-05-17
---

# Phase 05 Plan 01: Wave 0 pre-flight gate Summary

**bindy-macro natively supports zero-field-class + static-method across napi/wasm/pyo3 — SC4 primary path confirmed; chip-0057 wired unconditionally onto chia-sdk-bindings's three workspace deps and Wave 0 scaffolding (stub facade + 1-class JSON + empty AVA test) committed; SC4 fallback strategy NOT needed.**

## Performance

- **Duration:** 3 min (146s wall; build time dominates: chia-sdk-bindings --all-features cold ≈31s, chia-wallet-sdk-napi ≈72s, chia-wallet-sdk-py ≈59s, chia-wallet-sdk-wasm ≈61s, but ran in parallel where possible)
- **Started:** 2026-05-18T01:09:55Z
- **Completed:** 2026-05-18T01:12:21Z
- **Tasks:** 2 of 2 complete
- **Files modified/created:** 7

## Accomplishments
- Wired `chip-0057` unconditionally onto chia-sdk-bindings's three SP-relevant workspace dep declarations (chia-sdk-driver, chia-sdk-utils, chia-sdk-types) per D-01; chia-sdk-coinset and chia-sdk-test left untouched (they don't have a chip-0057 feature)
- Landed minimal Wave 0 facade (`crates/chia-sdk-bindings/src/silent_payments.rs`, 28 lines) hosting the `SilentPayments` unit struct with a single `probe_noop -> u32` static method per D-02
- Landed minimal Wave 0 descriptor (`bindings/silent_payments.json`, 11 lines) with a zero-field `SilentPayments` class declaring `probe_noop` as `"type": "static"`
- Landed empty AVA test (`napi/__test__/silent_payments.spec.ts`) with a `test.skip` placeholder so AVA's file discovery picks it up before Plan 05-04 fills it
- Confirmed Wave 0 PASS verdict via 4 target builds: `chia-sdk-bindings --all-features`, `chia-wallet-sdk-napi`, `chia-wallet-sdk-py`, `chia-wallet-sdk-wasm` — all exit 0
- Recorded verdict in `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-NOTES.md` (with timing + cargo-citation evidence)
- Flipped 05-VALIDATION.md frontmatter (`wave_0_complete: true`, `nyquist_compliant: true`), replaced all 10 `TBD` plan IDs in the per-task verification map with concrete `05-01`..`05-04` IDs, marked Wave 0 row green, and signed off the sign-off block

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire chip-0057 unconditionally + scaffold Wave 0 stub files** — `2b073b49` (feat)
2. **Task 2: Record Wave 0 verdict + populate VALIDATION per-task map** — `f83c44f1` (docs)

**Plan metadata commit:** (added by final_commit step alongside this SUMMARY)

## Files Created/Modified
- `crates/chia-sdk-bindings/Cargo.toml` — added `"chip-0057"` to features lists on chia-sdk-driver (now `["offer-compression", "action-layer", "chip-0057"]`), chia-sdk-utils (now `["chip-0057"]`), and chia-sdk-types (now `["chip-0057"]`) dep declarations; left chia-sdk-coinset/chia-sdk-test/[features] block untouched
- `crates/chia-sdk-bindings/src/silent_payments.rs` (NEW, 28 lines) — Wave 0 stub: `#[derive(Clone)] pub struct SilentPayments;` + `impl SilentPayments { pub fn probe_noop() -> Result<u32> { Ok(0) } }`; module rustdoc points future readers at Plan 05-02 expansion list
- `crates/chia-sdk-bindings/src/lib.rs` — inserted `mod silent_payments;` (alphabetically between `mod secp;` and `mod simulator;`) and `pub use silent_payments::*;` (between `pub use secp::*;` and `pub use simulator::*;`)
- `bindings/silent_payments.json` (NEW, 11 lines) — single-class stub: SilentPayments class with one `probe_noop` static method returning `u32`; auto-discovered by `bindy_napi!/bindy_wasm!/bindy_pyo3!` via the macros' `bindings/*.json` scan
- `napi/__test__/silent_payments.spec.ts` (NEW, 8 lines) — `import test from "ava"; test.skip("silent-payment address round-trip (filled in Plan 05-04)", ...)`
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-NOTES.md` (NEW) — Wave 0 verdict + per-target build timings + bindy-macro line-citation evidence
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md` — frontmatter flags flipped, per-task map populated with concrete plan IDs, Wave 0 Requirements + Sign-Off checkboxes marked `[x]`

## Decisions Made
- **Wave 0 verdict = PASS (NATIVE SUPPORT CONFIRMED).** All four target builds exit 0 with the stub class and static method in place. Plan 05-02 will expand the descriptor to the full 9-class surface using the same pattern; the fallback strategy (distribute statics across carrier types) is NOT needed.
- **No `--features pyo3` or `--features wasm` on the per-target binding crate builds.** The plan listed those flags but neither `chia-wallet-sdk-py` nor `chia-wallet-sdk-wasm` declares any cargo features — feature activation flows through their dep on `chia-sdk-bindings` (which itself has `pyo3`/`wasm`/`napi` features). Plain `cargo build -p <crate>` is equivalent and exits 0; documented this discrepancy in 05-PHASE-NOTES.md so Plan 05-03 doesn't replicate the flag confusion.

## Deviations from Plan

None - plan executed exactly as written, with one minor command-form clarification (see Decisions Made bullet 2). No bugs found, no missing functionality, no blocking issues, no architectural changes needed. Two tasks ran exactly as specified.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 05-02 ready to start.** Wave 0 PASS unlocks Wave 1 expansion: descriptor (`bindings/silent_payments.json` 1-class stub → 9-class full surface) + facade (`silent_payments.rs` 28 lines → ~250 lines per RESEARCH §"Recommended Module/File Layout") + SendDestination entry in `action_system.json`.
- **No carried-forward concerns.** The chip-0057 unconditional wiring is locked in (D-01 invariant), the bindy-macro auto-discovery of `bindings/*.json` is verified, and the AVA file discovery is wired so Plan 05-04 can swap `test.skip` for `test` without restructuring.
- **One thing to remember in Plan 05-02:** delete `SilentPayments::probe_noop` (both the facade impl and the JSON entry) when adding the four real static methods (`scan_from_tweaks`, `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`) — leaving probe_noop alongside the real methods would pollute the public API surface.

## Self-Check: PASSED

All 5 created/modified files exist on disk:
- `crates/chia-sdk-bindings/src/silent_payments.rs` — FOUND
- `bindings/silent_payments.json` — FOUND
- `napi/__test__/silent_payments.spec.ts` — FOUND
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-NOTES.md` — FOUND
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-01-SUMMARY.md` — FOUND

Both task commits present in git log:
- `2b073b49` (Task 1 — feat(05-01): wire chip-0057 on chia-sdk-bindings deps + Wave 0 stubs) — FOUND
- `f83c44f1` (Task 2 — docs(05-01): record Wave 0 PASS verdict + populate VALIDATION per-task map) — FOUND

No stubs that would prevent plan goal completion — the `SilentPayments::probe_noop` static is intentionally a Wave 0 stub probe, documented as such in both the facade module rustdoc and the SUMMARY, with explicit instruction for Plan 05-02 to delete it when adding the four real static methods.

---
*Phase: 05-bindings-rust-facade-json-descriptor*
*Completed: 2026-05-17*
