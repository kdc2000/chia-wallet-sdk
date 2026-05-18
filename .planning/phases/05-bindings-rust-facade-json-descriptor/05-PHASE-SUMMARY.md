---
phase: 5
slug: bindings-rust-facade-json-descriptor
status: complete
completed: 2026-05-18
plans: 4
waves: 4
duration_minutes: 35
---

# Phase 5: Bindings (Rust facade + JSON descriptor) — Phase Summary

## Outcome

Phase 5 exposes the chip-0057 silent-payments Rust surface (Phases 1-4.2) through the bindy descriptor pipeline so TypeScript, Python, and WASM consumers can use it through their idiomatic APIs.

**Headline deliverables:**

1. **`bindings/silent_payments.json`** — 11-entry descriptor: SilentPaymentNetwork enum + 7 data classes (SilentPaymentAddress, SilentPaymentKeys, LabelRegistry, OutputMeta, TweakData, DetectedSpCoin, ScalarField) + 2 SP-key registration wrappers (SilentPaymentRegisteredKey, SilentPaymentRegisteredSecretKey) + zero-field SilentPayments namespace with 4 static methods.
2. **`crates/chia-sdk-bindings/src/silent_payments.rs`** — ~430-line Rust facade exposing every type the descriptor references. chip-0057 is unconditional on the three relevant workspace dep declarations (D-01); no `#[cfg(feature = "chip-0057")]` gates inside the facade.
3. **`bindings/action_system.json` SendDestination patch + Action.send signature change** — SP construction in TS now goes through `Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos)`. The Rust 28-caller compatibility from Phase 04.2 (`From<Bytes32> for SendDestination`) lives on the Rust side; TS callers explicitly construct via `SendDestination.puzzleHash(bytes)` or `SendDestination.silentPayment(addr)`.
4. **Cross-target build verification** — napi build, maturin develop, wasm-pack build all succeed against the new descriptor.
5. **`napi/__test__/silent_payments.spec.ts`** — 4 AVA tests: 2 closing SC2 (address round-trip with byte-equality on scan_pk/spend_pk for mainnet + testnet) + 2 closing SC3 (SendDestination factory + introspector round-trip + Action.send composition smoke).
6. **`scripts/sp_descriptor_facade_drift.sh`** — descriptor↔facade drift audit; runs in <5s; zero drift at phase close (22 methods on both sides).

## Plans Completed

| Plan | Wave | Files | Outcome |
|------|------|-------|---------|
| 05-01 | 0 | 7 | Wave 0 pre-flight: chip-0057 unconditional wiring (D-01); zero-field SilentPayments stub probe → SC4 verdict NATIVE SUPPORT CONFIRMED (PASS); VALIDATION.md per-task map populated. |
| 05-02 | 1 | 9 | Full facade (silent_payments.rs ~430 lines, 11 types) + full descriptor (silent_payments.json, 11 entries) + SendDestination patch to action_system.json + Spends.with_silent_payment_keys method + bindy::Error::SilentPayment variant. |
| 05-03 | 2 | 4 | Cross-target build verification: napi build, maturin develop, wasm-pack build — all green; `Vec<PublicKey>` marshaling (RESEARCH Q1) PASSED without fallback — bindy-macro auto-handles Vec<bindy-class-type> across all three targets. |
| 05-04 | 3 | 5 | AVA round-trip test (4 named tests) + drift audit script + REQUIREMENTS.md/STATE.md updates + this PHASE-SUMMARY. Action_system.spec.ts wrapped pre-existing puzzle-hash args in SendDestination.puzzleHash(...) per the post-04.2 binding-side signature change. |

## Phase-Gate Matrix

| Gate | Status | Source |
|------|--------|--------|
| SC1: `cargo build --workspace --all-features` builds chia-sdk-bindings cleanly with chip-0057 | PASS | Plan 05-02 + 05-03 |
| SC1: napi build produces index.d.ts with required symbols | PASS | Plan 05-03 Task 1 |
| SC1: pyo3 maturin develop succeeds | PASS | Plan 05-03 Task 2 |
| SC1: wasm-pack build succeeds | PASS | Plan 05-03 Task 3 |
| SC2: AVA address round-trip test (byte-equality on scan_pk/spend_pk) | PASS | Plan 05-04 Task 1 |
| SC3: `bindings/action_system.json` SendDestination opaque-handle class | PASS | Plan 05-02 Task 3 |
| SC3: TS caller composes Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos) | PASS | Plan 05-04 Task 1 |
| SC4: bindy-macro static-functions schema verified | PASS | Plan 05-01 Wave 0 pre-flight (PHASE-NOTES.md) |
| Descriptor↔facade drift | ZERO | Plan 05-04 Task 2 |
| Workspace clippy clean under -D warnings | PASS | Plan 05-02 Task 1 |
| Zero new `#[allow(...)]` attributes | PASS | Plan 05-02 final clippy run |
| Zero `#[cfg(feature = "chip-0057")]` in facade (D-01 / Pitfall 2) | PASS | Plan 05-02 Task 1 + 3 |
| ScalarField uses from_bytes_unsigned only (D-03 / Pitfall 4) | PASS | Plan 05-02 Task 1 |
| cargo machete clean (no new ignored entries) | PASS | Plan 05-02 Task 1 |

## Requirements Closed

| REQ | Status | Notes |
|-----|--------|-------|
| BIND-01 | closed | `bindings/silent_payments.json` descriptor + `chia-sdk-bindings::silent_payments` facade expose all address/keys types through bindy-macro. |
| BIND-02 | closed | Send-side and receive primitives exposed as static methods on the zero-field SilentPayments namespace per D-02. Bindy-macro static-functions schema is natively supported — no fallback required. |
| BIND-03 | deferred to Phase 6 | Cross-language E2E (TS+Py+WASM full round trip) is Phase 6's concern alongside the simulator round-trip. |

## Lessons Learned

1. **The Wave 0 stub probe is cheap and gives a definitive verdict.** A 1-method zero-field class + 1 unit-struct facade + 4 target-build commands = ~3 minutes total. Always run a Wave 0 pre-flight before committing the full descriptor when an architectural assumption is involved.
2. **The bindings/ directory auto-discovery is a feature.** Just dropping a new JSON file in `bindings/` registers it; the macro reads them all (bindy-macro/src/lib.rs:95-104). No registration step needed.
3. **D-03 ScalarField exposure was the right call.** The cost is a 4-line facade class + 2 JSON entries; the benefit is wallet authors cannot silently pass unreduced bytes into `derive_one_time_puzzle_hash`. The Rust type-system invariant survives the FFI boundary.
4. **The opaque-handle pattern (`Id`, `SendDestination`) is the right escape hatch for tagged enums.** Bindy doesn't support `Class | Enum | Function` tagged-union variants natively (`bindy-macro/src/lib.rs:35-86`). Factory methods + `is_*`/`as_*` introspectors are mechanical and consistent.
5. **Action.send signature change is a breaking change for TS callers.** Pre-04.2 TS callers passing `Action.send(id, puzzleHashBytes, ...)` would have type-errored at the call site, but bindy's pre-04.2 descriptor had `puzzle_hash: Bytes32` (= Uint8Array), so the pre-04.2 TS surface was `Action.send(id, Uint8Array, ...)`. After Phase 5, it's `Action.send(id, SendDestination, ...)` — callers must wrap via `SendDestination.puzzleHash(bytes)`. The Rust 28-caller compatibility (`From<Bytes32> for SendDestination`) does NOT translate to TS. Document for Sage migration. This bit Plan 05-04 itself — `napi/__test__/action_system.spec.ts` had 10 such call sites that needed wrapping; fixed inline as a [Rule 3 - Blocking] deviation.
6. **Vec<(K,V)> tuple types do not marshal through bindy's napi codegen.** Caught at Plan 05-02 Task 3 (`Spends::with_silent_payment_keys`); resolved via 2-field wrapper bindy classes (`SilentPaymentRegisteredKey` + `SilentPaymentRegisteredSecretKey`) that convert to `IndexMap<Bytes32, _>` inside the facade body. Pattern documented for future use.
7. **bindy's `&self.0.method(...)` dispatch contract requires `&self` on all facade methods.** Caught at Plan 05-02 Task 2 (`LabelRegistry::register`); resolved via `Arc<Mutex<_>>` interior mutability, matching the Spends/FinishedSpends precedent in action_system.rs.

## Open Items (Phase 6 work)

- BIND-03: Cross-language E2E test in pyo3/tests/test_silent_payments.py + wasm/__test__/silent_payments.spec.ts.
- SIM-01/02/03: simulator-based send + scan round trip.
- EX-01: examples/silent_payment.rs.
- The pyo3 stub generator (`cargo run -p pyo3-stub-generator`) was not exercised in Phase 5 — defer .pyi shape verification to Phase 6.

## File Manifest

**New files (committed in Phase 5):**
- `bindings/silent_payments.json` (11-entry descriptor)
- `crates/chia-sdk-bindings/src/silent_payments.rs` (~430-line facade)
- `napi/__test__/silent_payments.spec.ts` (4 AVA tests)
- `scripts/sp_descriptor_facade_drift.sh` (drift audit)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-NOTES.md`
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-PHASE-SUMMARY.md`
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-0{1,2,3,4}-SUMMARY.md`

**Modified files:**
- `crates/chia-sdk-bindings/Cargo.toml` (D-01: 3 dep features wired with chip-0057; added indexmap dep)
- `crates/chia-sdk-bindings/src/lib.rs` (added mod silent_payments + re-exports + DriverSendDestination alias re-export)
- `crates/chia-sdk-bindings/src/action_system.rs` (Action.send signature change + SendDestination facade + Spends.with_silent_payment_keys)
- `crates/chia-sdk-bindings/src/mnemonic.rs` (added pub(crate) fn inner() crate-internal accessor)
- `crates/chia-sdk-bindings/bindy/Cargo.toml` (added "chip-0057" feature to chia-sdk-utils dep)
- `crates/chia-sdk-bindings/bindy/src/lib.rs` (added Error::SilentPayment variant via #[from] SilentPaymentError)
- `bindings/action_system.json` (SendDestination entry + Action.send descriptor signature change + Spends.with_silent_payment_keys)
- `napi/index.d.ts` + `napi/index.js` (regenerated by Plan 05-03)
- `napi/__test__/action_system.spec.ts` (Plan 05-04 wrapped 10 raw `Uint8Array` puzzle-hash args in `SendDestination.puzzleHash(...)` per the post-04.2 binding-side signature change)
- `.planning/REQUIREMENTS.md` (BIND-01 + BIND-02 marked [x] with post-04.2 wording)
- `.planning/STATE.md` (Phase 5 complete; positions/decisions updated)
- `.planning/phases/05-bindings-rust-facade-json-descriptor/05-VALIDATION.md` (per-task map fully populated; all rows green)

---
*Phase: 05-bindings-rust-facade-json-descriptor*
*Completed: 2026-05-18*
