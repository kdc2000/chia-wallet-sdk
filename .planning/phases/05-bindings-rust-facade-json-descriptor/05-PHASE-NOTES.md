# Phase 5 — Phase Notes

## Wave 0 pre-flight verdict: NATIVE SUPPORT CONFIRMED (PASS)

**Date:** 2026-05-17
**Probe:** Plan 05-01 Task 1 added a zero-field `SilentPayments` class with one `probe_noop -> u32` static method to `bindings/silent_payments.json` + a matching unit-struct facade to `crates/chia-sdk-bindings/src/silent_payments.rs`.

**Result:** All four target builds exit 0:
- `cargo build -p chia-sdk-bindings --all-features` → PASS (31.60s)
- `cargo build -p chia-wallet-sdk-napi` → PASS (1m 12s)
- `cargo build -p chia-wallet-sdk-py` → PASS (58.94s)
- `cargo build -p chia-wallet-sdk-wasm` → PASS (1m 01s)

**Verdict:** bindy-macro natively supports `"type": "static"` methods on a zero-field class across all three binding targets. SC4's primary path is confirmed; the fallback strategy (distribute statics across carrier types) is NOT needed.

**Consequence for Plan 05-02:** Expand `bindings/silent_payments.json` from the 1-class stub to the full 9-class descriptor (SilentPayments + ScalarField + SilentPaymentNetwork + SilentPaymentAddress + SilentPaymentKeys + LabelRegistry + OutputMeta + TweakData + DetectedSpCoin); expand `silent_payments.rs` from the stub to the full ~250-line facade per RESEARCH §"Recommended Module/File Layout"; delete `probe_noop` and replace with the 4 real static methods (`scan_from_tweaks`, `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`).

**Evidence references:**
- bindy-macro/src/lib.rs:35-86 — Binding/Method/MethodKind schema definition
- bindy-macro/src/lib.rs:302-324 — napi static codegen
- bindy-macro/src/lib.rs:725-737 — wasm static codegen
- bindy-macro/src/lib.rs:1210-1241 — pyo3 static codegen
- bindings/mnemonic.json:24 — `Mnemonic.verify` static (precedent)
- bindings/bls.json:79 — `PublicKey.aggregate_verify` static (precedent)
- bindings/puzzles.json:612, 625, 675, 774, 861, 868 — 6 distinct static method precedents

**Note on probe command form:** The plan listed `cargo build -p chia-wallet-sdk-py --features pyo3` and `cargo build -p chia-wallet-sdk-wasm --features wasm`, but neither crate declares any features in its `Cargo.toml` — feature activation flows through their dep on `chia-sdk-bindings` which already specifies the target feature unconditionally. The probe was therefore run with plain `cargo build -p <crate>`, which is equivalent in effect and exits 0.
