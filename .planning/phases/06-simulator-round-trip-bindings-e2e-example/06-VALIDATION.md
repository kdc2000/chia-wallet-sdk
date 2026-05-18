---
phase: 6
slug: simulator-round-trip-bindings-e2e-example
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-18
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework (Rust)** | `#[test]` + `rstest = 0.22.0` + `chia_sdk_test::Simulator` |
| **Framework (napi)** | AVA 7.0.0 (`.spec.ts`, `import test from "ava"`) |
| **Framework (pyo3)** | pytest (default discovery `tests/test_*.py`) |
| **Framework (wasm)** | AVA 6.4.1 + `wasm-pack build --target nodejs` |
| **Config file** | `napi/package.json`, `wasm/package.json`, `pyo3/pyproject.toml` — all pre-configured |
| **Quick run command** | `cargo test --release -p chia-sdk-driver --features chip-0057 -- silent_payments::e2e::` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Cross-language quick runs** | `cd napi && pnpm test -- --match='*BIND-03*'`; `cd pyo3 && pytest tests/test_silent_payments.py`; `cd wasm && pnpm test -- --match='*BIND-03*'` |
| **Estimated runtime** | ~90 seconds (Rust E2E) + ~20s (napi) + ~10s (pyo3) + ~15s (wasm) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::` + `cargo clippy -p <touched-crate> --all-features -- -D warnings`
- **After every plan wave:** Run full workspace test suite + the three cross-language test runners (napi `pnpm test`, pyo3 `pytest`, wasm `pnpm test`)
- **Before `/gsd:verify-work`:** Full suite green + all three binding test suites + `cargo build --examples --all-features` + `bash scripts/sp_descriptor_facade_drift.sh` + `cargo machete`
- **Max feedback latency:** ~90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-XX | 01 | 0 | chip-0057 cascade | smoke | `cargo build -p chia-sdk-test -F chip-0057` | ❌ Wave 0 (Plan 06-01) | ⬜ pending |
| 06-02-XX | 02 | 1 | SIM-01 | unit | `cargo test -p chia-sdk-test --features chip-0057 silent_payments::tweak_data::` | ❌ Wave 0 (Plan 06-02) | ⬜ pending |
| 06-03-XX | 03 | 2 | SIM-02 | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_unlabeled` | ❌ Wave 0 (Plan 06-03) | ⬜ pending |
| 06-03-XX | 03 | 2 | SIM-03 (labeled) | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_labeled` | ❌ Wave 0 (Plan 06-03) | ⬜ pending |
| 06-03-XX | 03 | 2 | SIM-03 (m=0) | integration | `cargo test -p chia-sdk-driver --features chip-0057 silent_payments::e2e::test_simulator_e2e_m0_self_change` | ❌ Wave 0 (Plan 06-03) | ⬜ pending |
| 06-04-XX | 04 | 3 | BIND-03 (napi) | integration | `cd napi && pnpm test -- --match='*BIND-03*napi*'` | ❌ Wave 0 (Plan 06-04) | ⬜ pending |
| 06-04-XX | 04 | 3 | BIND-03 (pyo3) | integration | `cd pyo3 && pytest tests/test_silent_payments.py::test_unlabeled_e2e` | ❌ Wave 0 (Plan 06-04) | ⬜ pending |
| 06-04-XX | 04 | 3 | BIND-03 (wasm) | integration | `cd wasm && pnpm test -- --match='*BIND-03*wasm*'` | ❌ Wave 0 (Plan 06-04) | ⬜ pending |
| 06-05-XX | 05 | 4 | EX-01 | smoke | `cargo build --examples --all-features && cargo run --example silent_payment --all-features` | ❌ Wave 0 (Plan 06-05) | ⬜ pending |
| 06-05-XX | 05 | 4 | descriptor↔facade drift | smoke | `bash scripts/sp_descriptor_facade_drift.sh` | ✅ | ⬜ pending |
| 06-05-XX | 05 | 4 | workspace lints | smoke | `cargo clippy --workspace --all-features --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 06-05-XX | 05 | 4 | cargo machete | smoke | `cargo machete` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Plan 06-01 prerequisites (the chip-0057 cascade onto `chia-sdk-test`):
- [ ] `crates/chia-sdk-test/Cargo.toml` — add `[features]` `chip-0057 = ["dep:chia-sdk-driver", "chia-sdk-driver/chip-0057"]` and `chia-sdk-driver = { workspace = true, optional = true }` dep
- [ ] Root `Cargo.toml` — add `"chia-sdk-test/chip-0057"` to the workspace `chip-0057` feature cascade
- [ ] `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-test -F chip-0057` CI line

Plan 06-02 prerequisites (the `tweak_data_from_simulator_block` helper):
- [ ] `crates/chia-sdk-test/src/silent_payments/mod.rs` — new module file (chip-0057 gated)
- [ ] `crates/chia-sdk-test/src/silent_payments/tweak_data.rs` — `pub fn tweak_data_from_simulator_block(simulator: &Simulator, height: u32) -> Result<TweakData, …>`
- [ ] `crates/chia-sdk-test/src/simulator.rs` — add `pub fn block_spends(height)` + `pub fn block_outputs(height)` accessors (NOT chip-0057 gated; just public surface)

Plan 06-03 prerequisites (Rust E2E tests):
- [ ] `crates/chia-sdk-driver/src/silent_payments/e2e.rs` — 3 E2E tests + `setup_e2e()` helper

Plan 06-04 prerequisites (Cross-language bindings + tests):
- [ ] `crates/chia-sdk-bindings/Cargo.toml` — add `"chip-0057"` to the `chia-sdk-test` dep's features
- [ ] `crates/chia-sdk-bindings/src/simulator.rs` — add `tweak_data_from_block` method on the Simulator facade
- [ ] `bindings/simulator.json` — add `tweak_data_from_block` entry
- [ ] `napi/__test__/silent_payments_e2e.spec.ts` — new file
- [ ] `pyo3/tests/test_silent_payments.py` — new file
- [ ] `wasm/__test__/silent_payments.spec.ts` — new file

Plan 06-05 prerequisites (Example + closeout):
- [ ] `examples/silent_payment.rs` — new runnable example
- [ ] `.planning/REQUIREMENTS.md` — mark SIM-01, SIM-02, SIM-03, BIND-03, EX-01 as `[x]`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Example output mirrors `cat_spends.rs` rhythm | EX-01 | Style + structure judgment | `cargo run --example silent_payment --all-features` — verify output prints address, send, scan, detect, spend stages in order with values |
| `Vec<chia_bls::PublicKey>` marshals correctly across FFI at runtime | BIND-03 | FFI marshaling correctness can only be observed at runtime through an actual cross-language call | The 3 BIND-03 tests above ARE the verification; manual = "does the test fail in a way that points to marshaling vs. logic" if it fails |

*All other phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s for unit/integration tests
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
