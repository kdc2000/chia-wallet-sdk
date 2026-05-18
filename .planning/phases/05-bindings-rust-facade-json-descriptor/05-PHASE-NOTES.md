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

---

## Plan 05-03 Wave 2 — environment setup notes

**Date:** 2026-05-18

### pyo3 venv

`maturin` is not installed system-wide on this host. The plan's documented fallback path was applied:

```bash
cd pyo3
python3 -m venv .venv
source .venv/bin/activate
pip install --upgrade pip maturin
maturin develop
```

The venv lives at `pyo3/.venv/` (already in `pyo3/.gitignore:13`). Re-runs require `source pyo3/.venv/bin/activate` first. `maturin 1.13.3` is installed in this venv; the build installs `chia_wallet_sdk-0.33.0` as an editable wheel into the venv.

### wasm-pack + wasm32 target

Both were missing on the host. Installed inline during Task 3:

- `wasm-pack`: installed via `cargo install wasm-pack --version 0.13.1 --locked` (lands in `~/.cargo/bin/`). Pinned to 0.13.1 because the current latest (0.15.0) depends on `cargo-platform@0.3.3` which requires rustc 1.91, and the workspace is pinned to rustc 1.90.0 via `rust-toolchain.toml`. wasm-pack 0.13.1 has no such constraint and builds the same nodejs-target output.
- `wasm32-unknown-unknown` target: installed via `rustup target add wasm32-unknown-unknown`

### Wave 2 Vec<PublicKey> marshaling: PASSED without fallback (auto-detected by bindy-macro)

The RESEARCH.md Open Question Q1 predicted that wasm-pack might fail on the remaining `Vec<chia_bls::PublicKey>` in `TweakData.tweak_points` (after Plan 05-02's wrapper-struct mitigation for `IndexMap<Bytes32, _>`). The actual build outcome: **wasm-pack build --target nodejs exits 0 cleanly** — no `Vec<PublicKey>` marshaling errors. The generated `wasm/pkg/chia_wallet_sdk_wasm.d.ts` declares `TweakData.tweakPoints: PublicKey[]` at line 2603 and the constructor takes `PublicKey[]` directly. No inline `bindings.json` `wasm` / `wasm_stubs` Vec<PublicKey> entries were needed; the bindy-macro auto-handles `Vec<T>` where `T` is a bindy class (including `remote: true` types like `PublicKey` from `bindings/bls.json`).

The fallback paths documented in the plan (Option A — explicit type-group entry; Option B — `TweakPoints(Vec<PublicKey>)` newtype) remain unapplied. If a future bindings change introduces a `Vec<chia_bls::PublicKey>` shape on a different surface and it fails to marshal, the precedent for the inline fix is `Vec<Bytes32>` at `bindings.json:33-34` + line 46-47.
