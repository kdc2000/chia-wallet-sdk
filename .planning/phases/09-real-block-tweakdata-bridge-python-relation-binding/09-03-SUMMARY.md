---
phase: 09-real-block-tweakdata-bridge-python-relation-binding
plan: 03
subsystem: bindings
tags: [bindings, action-system, relation, opaque-handle, bridge-03]
requires:
  - chia_sdk_driver::Relation (Phase 4.1 - pre-existing enum)
  - chia_sdk_bindings::Id (opaque-handle precedent)
  - chia_sdk_bindings::SendDestination (opaque-handle precedent with is_*/as_*)
provides:
  - chia_sdk_bindings::Relation (opaque-handle struct + 5 methods)
  - bindings/action_system.json "Relation" descriptor entry
affects:
  - Plan 09-04 (Spends.prepare(deltas, relation: Option<Relation>) signature extension - unblocked)
  - Plan 09-06 (cross-binding multi-input tests in pyo3/napi/wasm - unblocked)
tech-stack:
  added: []
  patterns:
    - opaque-handle facade (mirrors Id, SendDestination precedents)
    - bindy-macro descriptor-driven cross-target dispatch
key-files:
  created: []
  modified:
    - crates/chia-sdk-bindings/src/action_system.rs (+39 lines: struct + 5-method impl; 2 lines edited for sdk:: qualification)
    - bindings/action_system.json (+23 lines: Relation descriptor block)
decisions:
  - "Import-collision resolution: sdk:: prefix at the one call site (Task 1) — chose this over `use ... as SdkRelation` because it matches how `sdk::Id` and `sdk::SendDestination` are already referenced in this file (consistent local convention)."
  - "Doc surfacing: standalone CHIP-0057 / driver-side terminology in the binding rustdoc (no GSD planning-artifact refs). The driver-side enum's variant-level 'load-bearing for CHIP-0057' intent is preserved via independent wording on the binding struct."
  - "Descriptor location: bindings/action_system.json (NOT top-level bindings.json). Relation parameterizes Spends.prepare and conceptually belongs in the action_system module surface."
metrics:
  duration: "3min"
  completed: "2026-05-29T16:22:15Z"
  tasks_count: 2
  files_count: 2
  commits:
    - "cb1334cc: refactor(09-03): qualify Relation references with sdk:: prefix"
    - "59a00f3c: feat(09-03): add Relation opaque-handle binding + descriptor entry"
---

# Phase 9 Plan 3: Python `Relation` Binding (BRIDGE-03) Summary

Add a cross-binding `Relation` opaque-handle to `chia-sdk-bindings::action_system` + matching descriptor entry, mirroring the `Id` / `SendDestination` precedents so pyo3 / napi / wasm callers can construct `Relation.none()` / `Relation.assert_concurrent()` for the upcoming `Spends.prepare(deltas, relation)` signature extension.

## Objective

Closes BRIDGE-03. The Rust driver's `Spends::prepare` already takes a `Relation` argument; the binding layer's `prepare` call site at `crates/chia-sdk-bindings/src/action_system.rs:185` hardcodes `sdk::Relation::None`. Multi-input SP sends (≥2 non-ephemeral XCH inputs) need `Relation::AssertConcurrent` to satisfy the runtime gate at `crates/chia-sdk-driver/src/action_system/spends.rs:604` (returns `DriverError::SilentPaymentRequiresInputBinding` otherwise). This plan ships the typed handle so plan 09-04 can wire it into the `Spends.prepare` signature.

## Changes

### Task 1 — Name-collision pre-resolution

Two-line refactor to free the `Relation` identifier for the new binding-side struct:

- Dropped `Relation` from the `chia_sdk_driver` import at `action_system.rs:10-11`
- Qualified the one call site at `:185` to `sdk::Relation::None`

This isolates the import edit into a small, reviewable commit before the additive Task 2 lands. Build clean after Task 1 alone (verified before commit).

### Task 2 — Relation opaque-handle struct + descriptor

Appended a `pub struct Relation(pub(crate) sdk::Relation)` after the `SendDestination` block at `action_system.rs:540`. Five-method impl:

```rust
fn none() -> Result<Self>                        // factory
fn assert_concurrent() -> Result<Self>           // factory
fn is_none(&self) -> Result<bool>                // introspector
fn is_assert_concurrent(&self) -> Result<bool>   // introspector
fn equals(&self, other: Relation) -> Result<bool> // value comparison
```

Mirroring descriptor entry in `bindings/action_system.json` between `SendDestination` and `Outputs`:

```json
"Relation": {
  "type": "class",
  "methods": {
    "none":              { "type": "factory" },
    "assert_concurrent": { "type": "factory" },
    "is_none":              { "return": "bool" },
    "is_assert_concurrent": { "return": "bool" },
    "equals": { "args": { "other": "Relation" }, "return": "bool" }
  }
}
```

`#[derive(Clone, Debug)]` only — `sdk::Relation` is `Copy + Eq` so `equals` works via direct `self.0 == other.0`. Wrapped doc comment surfaces the multi-input SP semantics in CHIP-0057 / driver-side terminology (no GSD planning-artifact refs).

## Verification

| Gate                                                                                | Result |
| ----------------------------------------------------------------------------------- | ------ |
| `cargo build --release -p chia-sdk-bindings --all-features`                         | PASS   |
| `cargo build --release -p chia-sdk-bindings -F napi`                                | PASS   |
| `cargo build --release -p chia-sdk-bindings -F wasm`                                | PASS   |
| `cargo build --release -p chia-sdk-bindings -F pyo3`                                | PASS   |
| `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings`     | PASS   |
| `cargo fmt --all -- --files-with-diff --check`                                      | PASS   |
| `python3 -m json.tool bindings/action_system.json > /dev/null`                      | PASS   |
| `grep -q 'pub struct Relation(pub(crate) sdk::Relation)' ...action_system.rs`       | PASS   |
| All 5 method signatures grep-locatable                                              | PASS   |
| `grep -q '"Relation":' bindings/action_system.json`                                 | PASS   |
| No bare `Relation` left in action_system.rs (`grep -v 'sdk::Relation'`)             | PASS   |

## Deviations from Plan

None — plan executed exactly as written. No clippy fights, no Rule-1/2/3 auto-fixes, no architectural questions.

The plan's expectation that `is_none()` would not collide with `Option::is_none` autoderef proved correct — clippy is happy. The two-step Task 1 → Task 2 sequencing prevented any transient build break.

## Self-Check: PASSED

- File `crates/chia-sdk-bindings/src/action_system.rs`: FOUND, contains `pub struct Relation(pub(crate) sdk::Relation)` at line ~558
- File `bindings/action_system.json`: FOUND, contains `"Relation":` descriptor block
- Commit `cb1334cc` (Task 1): FOUND in `git log --oneline`
- Commit `59a00f3c` (Task 2): FOUND in `git log --oneline`
- Build, clippy, fmt, JSON validity all green
- All 5 method signatures present
- No bare `Relation` references remaining in action_system.rs
- No GSD planning-artifact refs added in new rustdoc

## Notes for Downstream Plans

- **Plan 09-04** can now reference `crate::Relation` (binding-side) as a typed parameter on `Spends::prepare`. The plan's locked signature `prepare(&self, deltas: Deltas, relation: Option<Relation>) -> Result<FinishedSpends>` slots in cleanly — unwrap with `relation.map(|r| r.0).unwrap_or(sdk::Relation::None)`.
- **Plan 09-06** can call `Relation.assertConcurrent()` (napi/wasm camelCase) or `Relation.assert_concurrent()` (pyo3 snake_case) from each binding target. The bindy-macro dispatch follows existing `Id` / `SendDestination` precedents.
- **Pre-existing `D-01` in `SendDestination` rustdoc** at `action_system.rs:496` is unchanged by this plan — that's Phase 5 vintage, out of scope for CLEANUP-01's `Plan 0[1-9]-` regex.
