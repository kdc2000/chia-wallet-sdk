# Phase 5: Bindings (Rust facade + JSON descriptor) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 05-bindings-rust-facade-json-descriptor
**Areas discussed:** chip-0057 feature wiring, Static functions carrier, ScalarField exposure, SC3 reconciliation

---

## chip-0057 feature wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Unconditional in deps | chia-sdk-bindings's deps on chia-sdk-driver/utils/types always-on with chip-0057. No new feature flag. Matches existing offer-compression + action-layer pattern. | ✓ |
| New default feature | Add `chip-0057` as a cargo feature on chia-sdk-bindings, put in `default = [...]`. Lets advanced consumers disable. | |
| Per-target wiring | Add chip-0057 separately to napi/pyo3/wasm Cargo.toml; chia-sdk-bindings stays conditional. Three places to keep in sync. | |

**User's choice:** Unconditional in deps (Recommended)
**Notes:** Captured as D-01. Matches the existing always-on pattern for `offer-compression` and `action-layer`. Eliminates accidental disablement and simplifies build matrix.

---

## Static functions carrier

| Option | Description | Selected |
|--------|-------------|----------|
| Zero-field SilentPayments class | Add a `SilentPayments` class with no fields/constructor + 4 static methods. TS call shape: `SilentPayments.scanFromTweaks(...)`. | ✓ |
| Distributed onto carrier types | Hang each function on its most-related type (`TweakData.scan`, `SilentPaymentAddress.derive`, etc.). | |
| Both — namespace + convenience methods | Ship both. Doubles descriptor surface; creates "which is canonical" ambiguity. | |

**User's choice:** Zero-field SilentPayments class (Recommended)
**Notes:** Captured as D-02. Confirms SC4's primary path. During discussion, verified bindy's `"type": "static"` mechanism already works on zero-field classes (precedents: `Mnemonic.verify`, `PublicKey.aggregate_verify`, 15+ uses in `puzzles.json`). No bindy macro changes needed; fallback strategy is not required.

---

## ScalarField exposure

| Option | Description | Selected |
|--------|-------------|----------|
| Expose as bindy class | Add `ScalarField` to silent_payments.json with `from_bytes` factory + `to_bytes` getter. Preserves type-system distinction across boundary. | ✓ |
| Type-group map to Bytes32 | Add `ScalarField` to bindings.json type_groups. Loses type-system safety; wallet authors might pass unreduced values. | |
| Reshape facade to convert at boundary | Facade returns Bytes32 and converts internally. Same loss-of-type with conversion buried. | |

**User's choice:** Expose as bindy class (Recommended)
**Notes:** Captured as D-03. SC1 listed the three primitives but not `ScalarField`; D-03 adds `ScalarField` to the exposed surface so the primitives compose cleanly across the binding boundary. Two of the three primitives (`compute_input_hash`, `aggregate_sender_sks`) return `ScalarField`; `derive_one_time_puzzle_hash` takes `&ScalarField` arguments — so without exposing `ScalarField`, composing in TS/Py/WASM would require manual byte-shuffling with no type-system safety.

---

## SC3 reconciliation

| Option | Description | Selected |
|--------|-------------|----------|
| Patch roadmap, supersede with 04.2 SC12 | Update ROADMAP.md Phase 5 SC3 to a SendDestination class entry per 04.2 SC12. Single source of truth. | ✓ |
| Note the supersession in CONTEXT.md only | Leave ROADMAP.md untouched, document supersession in CONTEXT.md. Creates confusing artifact in roadmap. | |
| Don't reconcile — add both | Add both a SendDestination class AND a silent_payment_send factory. Adds unnecessary surface. | |

**User's choice:** Patch roadmap, supersede with 04.2 SC12 (Recommended)
**Notes:** Captured as D-04. ROADMAP.md SC3 patched during this discuss-phase session — new wording explicitly notes it supersedes the original `silent_payment_send` factory wording that Phase 04.2 deleted.

---

## Claude's Discretion

Several lower-impact decisions deferred to the planner with explicit guidance in CONTEXT.md's `<decisions>` section:
- Module/file layout in `chia-sdk-bindings::silent_payments` (one file vs. directory)
- `SilentPaymentKeys` constructor shape (factory-only vs. `new` + factories)
- `SilentPaymentNetwork` shape (bindy enum vs. string-based)
- `LabelRegistry` exposed surface (full vs. minimal)
- `OutputMeta` declaration (planner picks 4-field class pattern per `Address` precedent)
- AVA test fixture mnemonic (reuse TV1 mnemonic from existing Rust tests)
- pyo3/wasm smoke tests for Phase 5 (optional, defaults to AVA-only per SC2)
- Privacy-warning rustdoc propagation to descriptor `description` fields

## Deferred Ideas

All items in `<deferred>` section of CONTEXT.md belong to other phases. No scope creep surfaced during the discussion.

---

*Discussion mode: standard interactive (no `--auto`, no `--text`, no `--batch`, no `--analyze`, advisor mode not active — no USER-PROFILE.md present)*
