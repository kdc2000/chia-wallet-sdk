---
phase: 5
slug: bindings-rust-facade-json-descriptor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-17
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | AVA (TypeScript, napi/__test__) + cargo test (Rust facade unit tests) |
| **Config file** | `napi/package.json` (AVA config), `Cargo.toml` (workspace) |
| **Quick run command** | `cargo build -p chia-sdk-bindings --all-features` |
| **Full suite command** | `cargo build --workspace --all-features && (cd napi && pnpm install && pnpm build && pnpm test) && (cd pyo3 && maturin develop) && (cd wasm && pnpm install && pnpm test)` |
| **Estimated runtime** | ~5-12 minutes (full triple-target binding build dominates) |

---

## Sampling Rate

- **After every task commit:** Run `cargo build -p chia-sdk-bindings --all-features` (≤90s feedback)
- **After every plan wave:** Run target-specific build for the surface touched in that wave (napi build / maturin develop / wasm-pack build)
- **Before `/gsd:verify-work`:** Full suite (all 3 targets + AVA round-trip test) must be green
- **Max feedback latency:** ≤90 seconds for the quick command; ≤12 min for the full suite

---

## Per-Task Verification Map

> Filled by the planner per concrete task IDs assigned in the PLAN.md files. Skeleton below names the verification class for each in-scope work item.

| Work item | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|-----------|------|------|-------------|-----------|-------------------|-------------|--------|
| Wave-0 bindy static_functions probe | TBD | 0 | BIND-01 (SC4) | compile probe | `cargo build -p chia-sdk-bindings --all-features` | ❌ W0 | ⬜ pending |
| `silent_payments.rs` facade compiles | TBD | 1 | BIND-01 (SC1) | unit (build) | `cargo build -p chia-sdk-bindings --all-features` | ❌ W0 | ⬜ pending |
| `bindings/silent_payments.json` resolves under bindy_napi! | TBD | 1 | BIND-01 (SC1) | unit (build) | `cargo build -p chia-wallet-sdk-napi --all-features` | ❌ W0 | ⬜ pending |
| `bindings/action_system.json` SendDestination entry | TBD | 1 | BIND-02 (SC3) | unit (build) | `cargo build -p chia-wallet-sdk-napi --all-features` | ❌ W0 | ⬜ pending |
| napi build produces `index.d.ts` with required symbols | TBD | 2 | BIND-01 (SC1) | shape check | `cd napi && pnpm build && grep -E 'SilentPaymentAddress\|SilentPaymentKeys\|TweakData\|DetectedSpCoin\|LabelRegistry\|scanFromTweaks\|deriveOneTimePuzzleHash\|computeInputHash\|aggregateSenderSks' index.d.ts` | ❌ W0 | ⬜ pending |
| pyo3 maturin develop succeeds | TBD | 2 | BIND-01 (SC1) | unit (build) | `cd pyo3 && maturin develop` | ❌ W0 | ⬜ pending |
| wasm-pack build succeeds (nodejs target) | TBD | 2 | BIND-01 (SC1) | unit (build) | `cd wasm && wasm-pack build --target nodejs` | ❌ W0 | ⬜ pending |
| AVA address round-trip test | TBD | 3 | BIND-01 (SC2) | integration (TS) | `cd napi && pnpm test -- silent_payments` | ❌ W0 | ⬜ pending |
| SendDestination TS construction smoke | TBD | 3 | BIND-02 (SC3) | integration (TS) | `cd napi && pnpm test -- silent_payments` (same file, second test) | ❌ W0 | ⬜ pending |
| Descriptor↔facade drift check | TBD | 3 | BIND-01/02 | grep audit | Planner-defined script: every JSON entry has a `chia-sdk-bindings::silent_payments::*` symbol, and every public facade symbol has a JSON entry | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/chia-sdk-bindings/src/silent_payments.rs` — empty/stub facade file (the static-functions probe lives here)
- [ ] `bindings/silent_payments.json` — minimal stub with the `SilentPayments` zero-field class + one `static_functions` entry (the probe input)
- [ ] Pre-flight gate **PASS verdict recorded** in PHASE-NOTES.md (set `wave_0_complete: true` here once the probe builds cleanly)
- [ ] `napi/__test__/silent_payments.ts` — empty file checked in so AVA discovers it before the implementation lands

*Frameworks already present (AVA via pnpm test in napi/, pytest in pyo3/, AVA in wasm/, cargo test) — no installation required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| pyo3 stub generator emits SilentPayments classes in `.pyi` | BIND-01 | Stub generation is opt-in / not part of normal test loop | Run `cargo run -p pyo3-stub-generator` (or equivalent) and grep the produced `.pyi` for `class SilentPaymentAddress` |
| wasm bundle openable in browser | BIND-01 | AVA only tests the nodejs target; browser smoke is out of scope for Phase 5 (Phase 6 concern) | Skip in Phase 5; documented for Phase 6 verifier |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (stub facade + stub JSON + empty test file + PHASE-NOTES verdict)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s for quick command
- [ ] `nyquist_compliant: true` set in frontmatter once planner has filled actual task IDs into the verification map

**Approval:** pending
