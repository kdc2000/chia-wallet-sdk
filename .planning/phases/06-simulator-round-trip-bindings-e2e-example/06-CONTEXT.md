# Phase 6: Simulator round-trip + bindings E2E + example - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

<domain>
## Phase Boundary

The final v1 phase. Closes the last 5 requirements (SIM-01, SIM-02, SIM-03, BIND-03, EX-01) by integrating everything Phases 1–5 built against a real on-chain flow:

1. **`chia-sdk-test::silent_payments::tweak_data_from_simulator_block(&Simulator, height) -> TweakData`** (SIM-01) — extract TweakData from a simulator block by grouping standard-puzzle spends, computing `tweak_point = input_hash * A_sum` per spend, collecting `OutputMeta` for the block's outputs.
2. **Rust E2E tests** (SIM-02 unlabeled + SIM-03 labeled & m=0 change) — send→farm→extract→scan→detect→spend against the `Simulator`.
3. **Cross-language E2E** (BIND-03) — full parity AVA-napi + pytest-pyo3 + AVA-wasm tests running the same flow through the bindings.
4. **`examples/silent_payment.rs`** (EX-01) — runnable demo mirroring `cat_spends.rs` / `spend_simulator.rs` structure.

**Not in scope (per ROADMAP + PROJECT.md OOS):** CAT2 SP send, NFT SP, CHIP-0058 WS transport client, GCS filter, bulk-scanning. Phase 6 closes v1 — anything beyond goes to v2.

</domain>

<decisions>
## Implementation Decisions

### Cross-language TweakData access

- **D-01:** Cross-language tests obtain `TweakData` via a **bindings-exposed simulator helper** (NOT a JSON fixture, NOT synthetic construction). All 3 cross-language test suites run the identical send→farm→extract→scan→detect→spend flow as the Rust E2E. This gives BIND-03's "Vec<chia_bls::PublicKey> marshals correctly" claim full FFI fidelity — the cross-language scanner consumes a `TweakData` that was built on the Rust side and crossed the FFI boundary.

- **D-02:** Helper lives on the `Simulator` class as **`Simulator.tweakDataFromBlock(height)`**. TS callsite: `const tweakData = simulator.tweakDataFromBlock(height);`. Rust-side: free function `chia-sdk-test::silent_payments::tweak_data_from_simulator_block(&Simulator, height) -> TweakData`; the binding facade wraps it with a method on the existing `Simulator` binding. Adds one entry to `bindings/simulator.json` (or a small chip-0057 section of it).

- **D-03:** `chia-sdk-bindings/Cargo.toml` adds `"chip-0057"` to the existing `features = [...]` array on its `chia-sdk-test` dep declaration. Mirrors the Phase 5 D-01 wiring (chip-0057 also unconditional on chia-sdk-driver, chia-sdk-utils, chia-sdk-types). The binding facade method is **unconditional** — no `#[cfg(feature = "chip-0057")]` gate inside `chia-sdk-bindings`.

### Labeled E2E + m=0 change-detection

- **D-04:** m=0 sub-test = **sender sends to their OWN SP address**; the sender's send-side code internally emits the output at `m=0` for self-change; the recipient's `scan_from_tweaks` detects it with `label: Some(0)`. Test shape: `alice.send_to(alice.unlabeled_address(), N) → farm → alice.scan(...) → expect detected coin with label = Some(0)`.

  **Open assumption (researcher MUST verify):** This presumes the SDK's send-side code currently emits an `m=0` labeled output when sender == recipient. If it doesn't — i.e., self-send produces an unlabeled detection — the test redesigns to demonstrate the actual change-detection mechanism (whatever it is). The researcher MUST surface this in RESEARCH.md before the planner writes the test task. Cross-reference ADDR-06 (`labeled_address(0)` is rejected at the API boundary) — m=0 is reserved for internal change use only.

- **D-05:** Three separate Rust `#[test]` fns: `test_simulator_e2e_unlabeled`, `test_simulator_e2e_labeled`, `test_simulator_e2e_m0_self_change`. Shared simulator + key-setup helper to keep boilerplate out of each test. Maps 1:1 to SC2 (unlabeled) + SC3 (labeled + m=0). Each failure points to a specific scenario.

- **D-06:** Tests live in `crates/chia-sdk-driver/src/silent_payments/` — exact file name (`tests.rs`, `e2e.rs`, or similar) is the planner's call. This mirrors the existing cat_e2e/nft_e2e pattern where integration tests live next to the primitive code (`chia-sdk-driver/src/primitives/cat/`, etc.). `chia-sdk-test::Simulator` is used as a dev-dep (already wired for other primitives).

### Cross-language E2E parity

- **D-07:** **Full parity** — napi (AVA), pyo3 (pytest), wasm (AVA) all run the same unlabeled send→farm→extract→scan→detect→spend flow. Labeled coverage stays Rust-only (the crypto is identical; only the scanner branch differs and is fully covered by Phase 3 unit tests + Phase 6 Rust E2E). This closes BIND-03 strongly without doubling test surface.

- **D-08:** pyo3 gets a single `pyo3/tests/test_silent_payments.py` (the project's first non-trivial pytest test). **No conftest.py yet** — fixtures defer until a second test consumer exists. Matches the lightweight `test_pyo3.py` precedent (27 lines).

### Example scope

- **D-09:** `examples/silent_payment.rs` demonstrates **unlabeled + one labeled payment in a single tx**. Flow: mnemonic → `SilentPaymentKeys` → unlabeled address + `labeled_address(m=1)` → `Action::send` twice (one per destination) → farm → `tweak_data_from_simulator_block` → scan (detects both, one with `label: None`, one with `label: Some(1)`) → spend the detected coins. Target ~80–120 lines (between `cat_spends.rs` at 42 lines and `custom_p2_puzzle.rs`); structurally mirrors `cat_spends.rs` and `spend_simulator.rs`. Must `cargo build --examples --all-features` cleanly in CI.

### Claude's Discretion

- **Rust test module file name and internal helper signatures** — `tests.rs` vs `e2e.rs` vs `simulator_tests.rs`; exact shape of the shared setup helper (`fn setup_e2e() -> (Simulator, SpContext, SilentPaymentKeys, SilentPaymentKeys)` or similar). Planner picks.
- **bindings/simulator.json entry shape for `tweakDataFromBlock`** — exact JSON descriptor wording, including whether it's a fresh top-level method or a separate chip-0057 sub-block. Planner picks per existing bindings patterns.
- **TypeScript fixture mnemonic** — reuse the BIP-39 TV1 abandon×11+about used in Phase 5's AVA round-trip, or pick a different deterministic mnemonic per test. Planner picks (probably reuse for consistency).
- **AVA/pytest file naming** — `silent_payments_e2e.spec.ts` vs `silent_payments_simulator.spec.ts` vs extending the existing `silent_payments.spec.ts`. Planner picks.
- **Example: exact recipient amount, fee handling, multi-block farm timing** — anything not crypto-load-bearing.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap + State
- `.planning/ROADMAP.md` §"Phase 6" — phase goal + 5 success criteria + dependency on Phases 4, 5
- `.planning/REQUIREMENTS.md` SIM-01, SIM-02, SIM-03, BIND-03, EX-01 — full requirement text
- `.planning/PROJECT.md` — validated requirements catalog (Phases 1–5 closed); OOS section enumerates what to NOT add (CAT2/NFT/CHIP-0058/etc.); Cryptographic subtleties section

### CHIP spec
- `~/silent-payments/chip-silent-payments.md` §425 (k-termination), §446 (`K_MAX_DEFAULT = 2400`), §459 (identity-element tweak point skip rule), §10 (hardware-wallet custody) — referenced by Phase 3 receive logic; Phase 6's E2E exercises §425 and §459 in practice

### Prior-phase context (carries forward to Phase 6)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-CONTEXT.md` — D-01 (chip-0057 unconditional), D-02 (`SilentPayments` namespace), D-03 (`ScalarField` as bindy class, `from_bytes_unsigned` only), D-04 (`SendDestination` opaque-handle pattern)
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-CONTEXT.md` — `Action::send(id, destination, amount, memos)` unified surface
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md` — final Phase 5 surface (11 SP types + 4 statics + `SendDestination` + `Spends.with_silent_payment_keys`)

### Codebase entry points (file:line citations from scout report)
- `crates/chia-sdk-driver/src/silent_payments/types.rs:31` — `TweakData { tweak_points: Vec<PublicKey>, outputs: Vec<OutputMeta> }`
- `crates/chia-sdk-driver/src/silent_payments/types.rs:42` — `OutputMeta`
- `crates/chia-sdk-driver/src/silent_payments/types.rs:52` — `DetectedSpCoin`
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs:73` — `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels, k_max)`
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs:175` — `SilentPaymentScan::scan(&self, tweak_data, labels, k_max)` trait method
- `crates/chia-sdk-driver/src/silent_payments/one_time.rs:60` — `derive_one_time_puzzle_hash`
- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs:53` — `compute_input_hash`
- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs:47` — `aggregate_sender_sks`
- `crates/chia-sdk-driver/src/actions/send.rs:630` — `Action::send` SP routing branch (Phase 4.2 unification point)
- `crates/chia-sdk-test/src/simulator.rs` — `Simulator` + `SimulatorConfig` (entry point for SIM-01 helper)
- `bindings/simulator.json` — existing Simulator binding surface (add `tweakDataFromBlock` here per D-02)
- `crates/chia-sdk-bindings/src/simulator.rs:11` — `Arc<Mutex<Simulator>>` pattern; chip-0057 method lives here

### Existing examples to mirror
- `examples/spend_simulator.rs` (35 lines) — simulator setup + single XCH spend skeleton
- `examples/cat_spends.rs` (42 lines) — issue + spend pattern; closer to multi-output examples like Phase 6 wants
- `examples/custom_p2_puzzle.rs` (130 lines) — biggest existing example; upper-bound shape reference

### Cross-language test runners
- `napi/__test__/silent_payments.spec.ts` (Phase 5) — existing 4 AVA tests; Phase 6 adds an E2E test, likely in a sibling file `silent_payments_e2e.spec.ts`
- `napi/__test__/action_system.spec.ts` (8957 bytes) — existing Simulator-driven AVA tests; reuse harness
- `wasm/__test__/wasm.spec.ts` — AVA framework with `setPanicHook()`; Phase 6 sibling test
- `pyo3/tests/test_pyo3.py` (27 lines) — minimal pytest precedent; Phase 6 adds `test_silent_payments.py`

### Cross-cutting concerns (from ROADMAP §"Cross-Cutting Concerns")
- **#5 CHIP-0058 forward-compat:** Phase 6's example MUST use only `tweak_data_from_simulator_block`, never a CHIP-0058 WS client. `TweakData` has no transport fields — confirmed at `crates/chia-sdk-driver/src/silent_payments/types.rs:1-29`.
- **#7 Workspace lints:** `deny clippy::all`, `warn pedantic`, `deny unsafe_code`, `deny dead_code`, `cargo machete` — applies to every new file (helper, tests, example).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`chia-sdk-test::Simulator`** — provides `new()`, `bls(N)`, `spend_coins(coin_spends, sks)`, `create_block()`, `coin_state(coin_id)`, `children(coin_id)`, `lookup_puzzle_hashes(...)`. Already wired through bindings (`bindings/simulator.json` + `crates/chia-sdk-bindings/src/simulator.rs:11` with `Arc<Mutex<>>` pattern). Phase 6's `Simulator.tweakDataFromBlock(height)` slots in alongside the existing methods.
- **`SilentPaymentKeys::from_mnemonic`** + **`unlabeled_address(network)`** + **`labeled_address(network, m)`** (Phase 2) — example sender + recipient key derivation.
- **`Action::send(id: Id, destination: SendDestination, amount: u64, memos: Memos)`** (Phase 4.2 unified) — the send-side entry; recipient address goes via `SendDestination::silent_payment(addr)` (or `From<SilentPaymentAddress>` impl).
- **`Spends::with_silent_payment_keys(sp_keys_map)`** (Phase 4) — registers SP keys on the spends builder before `Spends::prepare`. Already exposed through bindings (Phase 5).
- **`scan_from_tweaks` / `SilentPaymentScan::scan`** (Phase 3) — receive-side entry; idempotent, no I/O.
- **`StandardLayer::new(pk).spend(ctx, coin, conditions)`** — the existing pattern for spending a detected SP coin (use `detected.onetime_sk.public_key().derive_synthetic()` as the pk).

### Established Patterns
- **Integration-test placement:** `crates/chia-sdk-driver/src/primitives/cat/` co-locates the CAT primitive and its simulator-driven tests; Phase 6's SP E2E follows this pattern in `crates/chia-sdk-driver/src/silent_payments/`.
- **Example structure:** Simulator → `sim.bls(1_000)` for key+coin → context → `Action`s → `spend_coins` → `create_block`. Phase 6's example follows the same rhythm with two SP outputs.
- **Bindings binding shape:** Sync methods, `Arc<Mutex<>>` for shared state, `Result<T, bindy::Error>` return type. `Simulator.tweakDataFromBlock(height) -> Result<TweakData>` slots in naturally.
- **AVA test harness:** `import test from "ava"` + `test("name", t => { ... })`; assertion style is `t.deepEqual` / `t.is`. Phase 6 follows.
- **WASM test harness:** Same as AVA + `setPanicHook()` for Rust panic tracing.

### Integration Points
- **`bindings/simulator.json`** — extend with chip-0057-gated `tweakDataFromBlock` method (mirrors how Phase 5 D-04 extended `bindings/action_system.json` with `SendDestination`).
- **`crates/chia-sdk-bindings/src/simulator.rs`** — add a method on the existing `Simulator` facade that delegates to `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`.
- **`crates/chia-sdk-test/Cargo.toml`** — verify `chip-0057` feature exists (Phase 1 should have wired it); if not, add. The dependency tree from `chia-sdk-bindings → chia-sdk-test` already exists since Simulator is exposed.
- **`crates/chia-sdk-bindings/Cargo.toml`** — add `"chip-0057"` to the `chia-sdk-test` dep's `features = [...]` array (D-03).
- **`pyo3/tests/`** — create `test_silent_payments.py` as the first non-trivial pytest test.

### Known Risks (surface in research/planning)
- **Open assumption D-04:** Does the SDK currently auto-emit `m=0` for self-change? Researcher MUST verify before planner writes the m=0 sub-test.
- **Vec<chia_bls::PublicKey> marshaling:** Phase 5 D-03 introduced `SilentPaymentRegisteredKey`/`SilentPaymentRegisteredSecretKey` wrappers because bindy doesn't marshal tuple types cleanly. `TweakData.tweak_points: Vec<PublicKey>` already builds (Phase 5 Q1 PASSED), but Phase 6's cross-language tests are the *first* tests that actually consume this vector across the FFI — watch for runtime issues that compile-time didn't catch.
- **Simulator timestamp/seed reproducibility:** AVA + pytest + AVA-wasm must produce deterministic test outcomes. Use a fixed seed via `SimulatorConfig` (planner picks the exact seed value).

</code_context>

<specifics>
## Specific Ideas

- Test mnemonic across all 3 cross-language tests: the BIP-39 TV1 **abandon×11+about** mnemonic used in Phase 5's AVA round-trip — keeps fixture seed identical across languages.
- The Phase 5 AVA round-trip lives at `napi/__test__/silent_payments.spec.ts` (4 tests). Phase 6 likely adds a sibling `silent_payments_e2e.spec.ts` rather than extending it — keeps unit-shape tests separated from E2E.
- Verify `chia-sdk-bindings/Cargo.toml`'s `chia-sdk-test` dep currently doesn't enable `chip-0057` — Phase 5 wired chip-0057 onto chia-sdk-driver/utils/types but probably not chia-sdk-test. This is the one-line addition for D-03.

</specifics>

<deferred>
## Deferred Ideas

- **Multi-input SP send in the example** — SEND-06 covers it via unit tests; the example stays focused on the single-input case to fit the 80–120 line budget. Multi-input would be a great teaching addition for a v1.1 follow-up example.
- **Labeled coverage across napi+pyo3+wasm** — considered (option 3 in Area 3). Deferred because Phase 3 unit + Phase 6 Rust E2E already prove labeled detection; cross-language labeled would just re-verify FFI marshaling. Defer to v1.1 if BIND-03 verification finds any gaps.
- **pytest conftest.py with shared simulator/keys fixtures** — defer until a second pyo3 test arrives.
- **Comprehensive tutorial example with privacy-warning callouts** — option 3 in Area 4. Could be a separate doc page (`docs/silent_payments.md`) rather than a code example. Note for v1.1 docs work.
- **CHIP-0058 transport client + GCS filter + bulk-scanning** — already in PROJECT.md OOS; reaffirmed here. v2.

</deferred>

---

*Phase: 06-simulator-round-trip-bindings-e2e-example*
*Context gathered: 2026-05-18*
