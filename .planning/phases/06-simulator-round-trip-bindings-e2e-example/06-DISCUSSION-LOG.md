# Phase 6: Simulator round-trip + bindings E2E + example - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 06-simulator-round-trip-bindings-e2e-example
**Areas discussed:** TweakData access from cross-language tests, Labeled E2E + m=0 change-detection shape, Cross-language E2E parity, examples/silent_payment.rs scope

---

## Area selection (gray areas)

| Option | Description | Selected |
|--------|-------------|----------|
| TweakData access from cross-language tests | Bindings helper vs JSON fixture vs synthetic build | ✓ |
| Labeled E2E + m=0 change-detection shape | Test shape + m=0 semantics + Rust file structure | ✓ |
| Cross-language E2E parity | Full parity vs prioritized vs add-labeled | ✓ |
| examples/silent_payment.rs scope | Minimal vs standard vs comprehensive | ✓ |

**User's choice:** all four
**Notes:** none — user selected the full set

---

## TweakData access from cross-language tests

### Sub-question 1: Mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Bindings-exposed helper | `Simulator.tweakDataFromBlock(height)` or `SilentPayments.tweakDataFromSimulatorBlock(simulator, height)` — real FFI round-trip | ✓ |
| JSON fixture file | Rust dumps fixture; cross-language tests load it | |
| Synthetic build | Cross-language tests construct TweakData from raw types — no simulator across FFI | |

**User's choice:** Bindings-exposed helper (Recommended)
**Notes:** Strongest fidelity for BIND-03's Vec<PublicKey> marshaling assertion.

### Sub-question 2: Helper location

| Option | Description | Selected |
|--------|-------------|----------|
| Simulator.tweakDataFromBlock(height) | Method on Simulator class — locality with simulator data | ✓ |
| SilentPayments.tweakDataFromSimulatorBlock(simulator, height) | Static method on the SP namespace — keeps SP entries colocated | |
| Bindings free function | Less idiomatic for bindy macro | |

**User's choice:** Simulator.tweakDataFromBlock (Recommended)
**Notes:** TS callsite reads naturally; matches existing Simulator binding shape.

### Sub-question 3: Feature flag wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Add chip-0057 to chia-sdk-test dep in chia-sdk-bindings/Cargo.toml | Mirrors Phase 5 D-01 — unconditional facade, deps carry the flag | ✓ |
| Cfg-gate the binding method | `#[cfg(feature = "chip-0057")]` on the facade method — inconsistent with Phase 5 | |

**User's choice:** Add to features array (Recommended)
**Notes:** Consistent with Phase 5 D-01 wiring; one-line Cargo.toml change.

---

## Labeled E2E + m=0 change-detection shape

### Sub-question 1: m=0 semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Self-send: sender = recipient, internally generates m=0 change | alice.send_to(alice.address()) → scan → detect label=Some(0) | ✓ |
| Mixed-recipient: send to self (m=0) + send to bob (m=1) in one tx | Two outputs, two scans | |
| Skip m=0 sub-test entirely | Treat as covered by Phase 3 unit tests | |

**User's choice:** Self-send (Recommended)
**Notes:** Per ADDR-06, `labeled_address(0)` is rejected at API boundary; m=0 is reserved for internal change use. Test assumes SDK auto-emits m=0 for sender-to-self change — researcher MUST verify this assumption before planner writes the test task.

### Sub-question 2: Test file structure

| Option | Description | Selected |
|--------|-------------|----------|
| One test fn per scenario | Three separate `#[test]` fns + shared helper | ✓ |
| Parameterized rstest | Single `#[rstest]` over scenarios | |
| One shared test fn that walks all scenarios | Sequential single test | |

**User's choice:** One per scenario (Recommended)
**Notes:** Easiest to debug; clean SC2/SC3 mapping.

### Sub-question 3: Test location

| Option | Description | Selected |
|--------|-------------|----------|
| crates/chia-sdk-driver/src/silent_payments/ | Mirrors cat/nft pattern; tests next to primitive | ✓ |
| crates/chia-sdk-test/src/silent_payments/ | Tests next to tweak_data_from_simulator_block helper | |
| Top-level tests/ folder | Cargo's integration-test dir | |

**User's choice:** chia-sdk-driver (Recommended)
**Notes:** Matches established repo pattern.

---

## Cross-language E2E parity

### Sub-question 1: How much parity?

| Option | Description | Selected |
|--------|-------------|----------|
| Full parity — all 3 run identical E2E | napi + pyo3 + wasm | ✓ |
| Prioritize napi+wasm; pyo3 minimal smoke | pyo3 = key derive + scan against fixture | |
| Add labeled coverage in cross-language too | Doubles test surface in each language | |

**User's choice:** Full parity (Recommended)
**Notes:** Closes BIND-03 strongly. Labeled stays Rust-side (crypto identical to unlabeled).

### Sub-question 2: pyo3 scaffolding

| Option | Description | Selected |
|--------|-------------|----------|
| Single test_silent_payments.py, no conftest | Lightweight — matches existing pyo3 minimalism | ✓ |
| Add conftest.py with shared fixtures | More upfront cost; better long-term ergonomics | |
| Defer pyo3 scaffold | Inline test, leave scaffold to future expansion | |

**User's choice:** Single file, no conftest (Recommended)
**Notes:** First non-trivial pytest test. Conftest can wait for second consumer.

---

## examples/silent_payment.rs scope

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal — unlabeled only | Single recipient, single output. Mirrors `spend_simulator.rs` exactly (~50 lines) | |
| Standard — unlabeled + one labeled in one tx | mnemonic → keys → unlabeled + labeled_address → 2 SP outputs → scan → detect both → spend (~80–120 lines) | ✓ |
| Comprehensive tutorial | + m=0 self-change + multi-input + privacy callout (~200+ lines) | |

**User's choice:** Standard (Recommended)
**Notes:** Best teaching value without bloating CI `cargo build --examples`.

---

## Claude's Discretion

- Rust test module file name (`tests.rs` vs `e2e.rs` vs `simulator_tests.rs`) and shared helper signatures
- Exact `bindings/simulator.json` entry shape for `tweakDataFromBlock`
- TypeScript test fixture mnemonic (probably reuse Phase 5's abandon×11+about for consistency)
- AVA/pytest test file naming (extend existing `silent_payments.spec.ts` vs new `silent_payments_e2e.spec.ts`)
- Example details not crypto-load-bearing (amount, fee handling, multi-block farm timing)

## Deferred Ideas

- Multi-input SP send in the example — SEND-06 covers via unit tests; defer multi-input demo to v1.1
- Labeled coverage across napi+pyo3+wasm — defer to v1.1 if BIND-03 verification finds gaps
- pytest conftest.py with shared fixtures — defer until second pyo3 test arrives
- Comprehensive tutorial example — could become a `docs/silent_payments.md` doc page in v1.1
- CHIP-0058 transport client + GCS filter + bulk-scanning — already in PROJECT.md OOS; reaffirmed

## Open Assumption (flagged for researcher)

D-04 (m=0 change-detection sub-test) assumes the SDK's send-side code emits an `m=0` labeled output when sender == recipient. **Researcher MUST verify this assumption** by inspecting `crates/chia-sdk-driver/src/silent_payments/` send-side code (especially `Action::send`'s SP routing branch at `actions/send.rs:630`). If self-send produces an *unlabeled* detection instead, redesign the m=0 sub-test to match the actual change-detection mechanism — surface in RESEARCH.md before the planner writes the test task.
