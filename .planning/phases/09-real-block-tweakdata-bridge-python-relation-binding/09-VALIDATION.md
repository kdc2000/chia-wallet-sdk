---
phase: 9
slug: real-block-tweakdata-bridge-python-relation-binding
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-29
updated: 2026-05-29
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust 1.90.0) + AVA (TS, napi+wasm) + pytest (Python, pyo3) |
| **Config file** | `Cargo.toml` (workspace), `napi/package.json` (AVA), `wasm/package.json` (AVA), `pyo3/pyproject.toml` (pytest) |
| **Quick run command (per change)** | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data` |
| **Full suite command (Rust)** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Full suite command (napi)** | `cd napi && pnpm install && pnpm build && pnpm test` |
| **Full suite command (wasm)** | `cd wasm && pnpm install && pnpm test` |
| **Full suite command (pyo3)** | `cd pyo3 && maturin develop && pytest` |
| **Lint gates** | `cargo fmt --all -- --files-with-diff --check`, `cargo clippy --workspace --all-features --all-targets`, `cargo machete` |
| **Estimated runtime (Rust quick)** | ~10 seconds |
| **Estimated runtime (Rust full)** | ~180 seconds |
| **Estimated runtime (each binding suite)** | ~90 seconds (napi/wasm), ~60 seconds (pyo3) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments` + `cargo clippy --workspace --all-features --all-targets` on touched crates
- **After Wave 1 completes:** Run full Rust workspace suite + clippy gate
- **After Wave 2 completes:** Run all three binding suites (napi, wasm, pyo3)
- **Before `/gsd:verify-work`:** All four suites green + `cargo fmt --check` + `cargo machete` green
- **Max feedback latency (Rust):** 180 seconds
- **Max feedback latency (binding parity):** 240 seconds per target

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 9-01-01 | 01 (BRIDGE-01) | 1 | BRIDGE-01 | build+lint | `cargo build --release -p chia-sdk-driver --features chip-0057 && cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings && cargo fmt --all -- --files-with-diff --check` | ❌ W0 (new file `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` with helper + iterative Tarjan SCC, no tests yet) | ⬜ pending |
| 9-01-02 | 01 (BRIDGE-01) | 1 | BRIDGE-01 | unit (6 named tests) | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::block_tweak_data::tests` (must report 6 passed: test_empty_block, test_non_standard_puzzle_skip, test_identity_element_guard, test_pass_2a_round_trip_matches_simulator_helper, test_multi_input_round_trip, test_pass_2b_pollution_resistance) | ❌ W0 (depends on 9-01-01 file + appends tests + prelude wiring) | ⬜ pending |
| 9-02-01 | 02 (BRIDGE-02) | 2 | BRIDGE-02 | regression (byte-equality) | `cargo test --release -p chia-sdk-test --features peer-simulator,chip-0057 silent_payments::tweak_data::tests && cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e` (existing Phase 6 oracle — `test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change` must stay byte-identical) | ✅ existing test infrastructure | ⬜ pending |
| 9-03-01 | 03 (BRIDGE-03) | 1 | BRIDGE-03 | build (name-collision fix) | `cargo build --release -p chia-sdk-bindings --all-features` after qualifying bare `Relation` references to `sdk::Relation` | ✅ existing crate | ⬜ pending |
| 9-03-02 | 03 (BRIDGE-03) | 1 | BRIDGE-03 | build (4 targets) + descriptor | `cargo build --release -p chia-sdk-bindings --all-features && cargo build --release -p chia-sdk-bindings -F napi && cargo build --release -p chia-sdk-bindings -F wasm && cargo build --release -p chia-sdk-bindings -F pyo3 && cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings && python3 -m json.tool bindings/action_system.json > /dev/null` | ❌ W0 (descriptor entry must land in bindings/action_system.json) | ⬜ pending |
| 9-04-01 | 04 (BRIDGE-04) | 2 | BRIDGE-04 | baseline (read-only) | `ls /tmp/09-04-baselines/09-04-baseline-napi.txt /tmp/09-04-baselines/09-04-baseline-pyo3.txt /tmp/09-04-baselines/09-04-baseline-wasm.txt` (pre-edit stub captures from each target) | ❌ W0 (captures generated during task; documents existing prepare() shape) | ⬜ pending |
| 9-04-02 | 04 (BRIDGE-04) | 2 | BRIDGE-04 | regression oracle + 3-target build | `cd pyo3 && python -m maturin develop && python -m pytest tests/test_silent_payments.py::test_unlabeled_e2e -v` (single-input pyo3 E2E MUST pass with zero source edits — this is the hard regression bar from CONTEXT.md) | ✅ existing | ⬜ pending |
| 9-05-01 | 05 (BRIDGE-05) | 2 | BRIDGE-05 | build + binding-surface | `cargo build --release -p chia-sdk-bindings --all-features && cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings && python3 -m json.tool bindings/silent_payments.json > /dev/null && cd napi && pnpm build && cd .. && grep -q 'tweakDataFromBlockSpends' napi/index.d.ts` | ❌ W0 (descriptor entry + facade method + new `tweakDataFromBlockSpends` line in generated .d.ts) | ⬜ pending |
| 9-06-01 | 06 (BRIDGE-06) | 2 | BRIDGE-06 | build + binding-surface (Simulator facade) | `cargo build --release -p chia-sdk-bindings --all-features && cd napi && pnpm build && cd .. && grep -q 'blockSpends' napi/index.d.ts && grep -q 'blockOutputs' napi/index.d.ts` (Simulator.block_spends + block_outputs facade additions) | ❌ W0 (descriptor entries + facade methods land) | ⬜ pending |
| 9-06-02 | 06 (BRIDGE-06) | 2 | BRIDGE-06 | e2e (3 targets) | `cd pyo3 && python -m maturin develop && python -m pytest tests/test_silent_payments.py -v` AND `cd napi && pnpm install && pnpm build && pnpm test` AND `cd wasm && pnpm install && wasm-pack build --target nodejs --release && pnpm test` (each suite passes incl. NEW multi-input test) | ❌ W0 (3 new test files — pyo3 appended to existing file; napi + wasm new spec files) | ⬜ pending |
| 9-06-03 | 06 (BRIDGE-06) | 2 | BRIDGE-06 | example | `cargo build --release --example silent_payment --all-features && cargo run --release --example silent_payment --all-features` (extended example with multi-input section per D-12 must build + exit 0) | ✅ example file exists; multi-input section is APPEND-ONLY | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 = file/scaffold creations that must exist before any test in the verification map can run.

- [ ] `crates/chia-sdk-driver/src/silent_payments/block_tweak_data.rs` — new sibling file containing `tweak_data_from_block_spends` + iterative Tarjan SCC + inline `#[cfg(test)] mod tests {}` shell (BRIDGE-01; landed by 9-01-01 + 9-01-02)
- [ ] `crates/chia-sdk-driver/src/silent_payments/mod.rs` — `mod block_tweak_data;` + `pub use block_tweak_data::tweak_data_from_block_spends;` (BRIDGE-01; landed by 9-01-01)
- [ ] `src/prelude.rs` — `tweak_data_from_block_spends` added to chip-0057 driver re-export block (BRIDGE-01; landed by 9-01-02)
- [ ] `bindings/action_system.json` — `Relation` descriptor entry (BRIDGE-03; landed by 9-03-02)
- [ ] `bindings/action_system.json` — `Spends.methods.prepare.args.relation: "Option<Relation>"` (BRIDGE-04; landed by 9-04-02)
- [ ] `bindings/silent_payments.json` — `SilentPayments.methods.tweak_data_from_block_spends` entry (BRIDGE-05; landed by 9-05-01)
- [ ] `bindings/simulator.json` — `Simulator.block_spends` + `Simulator.block_outputs` descriptor entries (BRIDGE-06; landed by 9-06-01)
- [ ] `napi/__test__/silent_payments_multi_input.spec.ts` — new spec file with AVA multi-input test (BRIDGE-06; landed by 9-06-02)
- [ ] `pyo3/tests/test_silent_payments.py::test_multi_input_e2e` — new pytest function appended to existing file (BRIDGE-06; landed by 9-06-02)
- [ ] `wasm/__test__/silent_payments_multi_input.spec.ts` — new spec file with AVA multi-input test (BRIDGE-06; landed by 9-06-02)
- [ ] `examples/silent_payment.rs` — multi-input section appended (Stages 6-9) (BRIDGE-06; landed by 9-06-03)

*Frameworks already installed: cargo test (workspace), AVA (napi+wasm pnpm), pytest (pyo3 maturin). No new framework installs needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Generated Python type stub readability | BRIDGE-03, BRIDGE-04 | `.pyi` content correctness is hard to assert mechanically beyond presence checks | After `maturin develop`, open `pyo3/python/chia_wallet_sdk/*.pyi` and visually confirm `class Relation:` with `none()`, `assert_concurrent()`, `is_none()`, `is_assert_concurrent()`, `equals()` factory/methods; confirm `Spends.prepare` shows `relation: Optional[Relation] = None` |
| Generated TypeScript `.d.ts` readability | BRIDGE-03, BRIDGE-04 | Same as above for napi | After `pnpm build`, open `napi/index.d.ts` and visually confirm `class Relation { static none(): Relation; static assertConcurrent(): Relation; ... }` and that `Spends.prepare`'s second arg is `relation?: Relation` |
| Multi-input SP scan output equivalence vs. simulator helper | BRIDGE-02 | Byte-equality already asserted by existing Phase 6 e2e tests, but a sanity diff on log output is faster for spotting subtle drift | After BRIDGE-02 lands, run `cargo test --release -p chia-sdk-driver --all-features --test silent_payments_e2e -- --nocapture` and confirm tweak_point hex strings unchanged vs. pre-refactor baseline (committed in git log) |

*Everything else has automated verification via `cargo test`, AVA, or pytest.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (every row in the per-task map binds to a concrete `cargo test`, `pnpm test`, `pytest`, or `cargo build` command; tasks producing new files are marked W0 with the file path)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (all 11 task rows have automated commands)
- [x] Wave 0 covers all MISSING references (11 file/descriptor creations enumerated)
- [x] No watch-mode flags (all commands single-shot)
- [x] Feedback latency < 240s per binding target, < 180s for Rust
- [x] `nyquist_compliant: true` set in frontmatter (per-task map populated with concrete `9-{plan}-{task}` IDs per actual PLAN.md files)

**Approval:** planner-approved (2026-05-29). Awaiting Wave 0 completion to flip `wave_0_complete: true`.
