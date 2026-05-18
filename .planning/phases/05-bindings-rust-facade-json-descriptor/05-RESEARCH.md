---
phase: 5
slug: bindings-rust-facade-json-descriptor
created: 2026-05-17
---

# Phase 5: Bindings (Rust facade + JSON descriptor) - Research

**Researched:** 2026-05-17
**Domain:** bindy descriptor pipeline + napi/pyo3/wasm cross-language exposure of chip-0057 silent-payments surface
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01: chip-0057 is unconditional in chia-sdk-bindings's deps.** `chia-sdk-bindings`'s `Cargo.toml` `[dependencies]` declarations on `chia-sdk-driver`, `chia-sdk-utils`, and `chia-sdk-types` add `"chip-0057"` to the `features = [...]` list. No new `chip-0057` cargo feature on chia-sdk-bindings; no `#[cfg(feature = "chip-0057")]` gates inside the binding facade. Matches the existing pattern for `offer-compression` and `action-layer` (both always-on on the driver dep). Justification: simplifies the build matrix, prevents accidental disablement by binding consumers, and matches SC1's "enabled-by-default" wording — there is no consumer asking for an SP-disabled binding.

**D-02: Zero-field `SilentPayments` namespace class** for the four free functions. Add a `SilentPayments` class to `bindings/silent_payments.json` with no fields, no constructor, and 4 static methods:
- `scan_from_tweaks(scan_sk: SecretKey, spend_sk: SecretKey, spend_pk: PublicKey, data: TweakData, labels: LabelRegistry, k_max: u32) -> Vec<DetectedSpCoin>`
- `derive_one_time_puzzle_hash(scan_pk: PublicKey, spend_pk: PublicKey, aggregated_sender_sk: ScalarField, input_hash: ScalarField, k: u32) -> Bytes32`
- `compute_input_hash(coin_ids: Vec<Bytes32>, aggregated_sender_pk: PublicKey) -> ScalarField`
- `aggregate_sender_sks(sks: Vec<SecretKey>) -> ScalarField`

TS call shape: `SilentPayments.scanFromTweaks(...)`. Confirms SC4's primary path (the zero-field-class-with-static-functions approach) — the fallback (distributed statics across carrier types) is not needed; bindy's `"type": "static"` mechanism on a class with no constructor already works (precedents: `Mnemonic.verify`, `PublicKey.aggregate_verify`).

**D-03: `ScalarField` is exposed as a bindy class** in `silent_payments.json`. Two methods:
- `from_bytes(bytes: Bytes32) -> ScalarField` (factory; uses `ScalarField::from_bytes_unsigned` semantics — accepts any 32-byte input and reduces mod r)
- `to_bytes() -> Bytes32` (getter, returns the canonical 32-byte big-endian representation of the reduced scalar)

Preserves the type-system distinction across the binding boundary. NOT mapped to `Bytes32` via type-group — that would erase the invariant the Rust type guards.

**D-04: Patched ROADMAP.md Phase 5 SC3** during this discussion to reflect the post-04.2 reality. New SC3 text: a `SendDestination` opaque-handle class entry in `action_system.json` with factory methods (`puzzle_hash(Bytes32)` + `silent_payment(SilentPaymentAddress)`) and introspectors (`is_puzzle_hash`/`as_puzzle_hash`/`is_silent_payment`/`as_silent_payment`). Pre-committed by Phase 04.2 SC12. TS callers construct an SP send via `Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos)`. The original SC3 wording referencing `silent_payment_send` factory was stale — Phase 04.2 deleted that constructor.

### Claude's Discretion

The following are left to the planner to decide based on cohesion / file-size / convention concerns at planning time:

- **`chia-sdk-bindings::silent_payments` module shape** — one `silent_payments.rs` file (matches every existing concept module: `address.rs`, `mnemonic.rs`, `bls.rs`) vs. a `silent_payments/` directory with sub-files (`address.rs`, `keys.rs`, `tweak_data.rs`, `static_fns.rs`). Decide based on resulting file size.
- **`SilentPaymentKeys` constructor shape** — `from_mnemonic(Mnemonic) -> SilentPaymentKeys` as factory (Mnemonic class is already exposed) vs. a `new(scan_sk: SecretKey, spend_sk: SecretKey)` constructor with `from_mnemonic` and `from_seed` as factories. Planner picks based on what reads best in TS — both are valid bindy patterns.
- **`SilentPaymentNetwork` shape** — bindy `Enum { values: ["Mainnet", "Testnet"] }` (unit-variant enum, the bindy-supported pattern for `mainnet`/`testnet` selection) vs. exposed via the HRP string. Enum is the more idiomatic call shape; planner verifies bindy enum codegen works in all three targets.
- **`LabelRegistry` exposed surface** — full `register`/`lookup`/`get` API vs. minimal `new()` + `register(m, label_pk)`. Planner picks based on what the AVA test + future Phase 6 SIM-03 helper needs; over-exposing here risks descriptor churn in Phase 6.
- **`OutputMeta` exposure** — TweakData has `outputs: Vec<OutputMeta>`, so `OutputMeta` is implicitly required. Planner declares it as a bindy class with `new` + 4 public fields (`puzzle_hash`, `coin_id`, `amount`, `parent_coin_id`) following the `Address` 4-field-class precedent.
- **AVA test fixture mnemonic** — reuse the TV1 mnemonic that drives the Rust tests in `crates/chia-sdk-utils/src/silent_payments/keys.rs::tests` so the TS test asserts the same on-chain bytes the Rust test does. The pinned mnemonic is `"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"` (BIP-39 standard test vector; line 152 of keys.rs).
- **pyo3/wasm smoke tests for Phase 5** — SC2 only requires AVA. Planner may add minimal pyo3 import-and-call tests (no SP logic) just to catch build/descriptor-codegen breakage that would otherwise surface only in Phase 6. Not required; nice-to-have.
- **Privacy-warning rustdoc propagation** — every memo-bearing public surface in the Rust crate carries `/// Privacy warning: memos are stored on-chain in plaintext...`. Planner decides whether the binding facade's memo-bearing surfaces (memo-related descriptor entries) need a parallel doc-comment or whether the Rust-level warning is enough. Recommendation: mirror the warning as Rust doc-comments on facade methods (these propagate to `.d.ts` and `.pyi` via the doctest mechanism Rust toolchain manages).

### Deferred Ideas (OUT OF SCOPE)

- **pyo3 + wasm SP round-trip tests** — Phase 6's concern (BIND-03 requires the full address-gen + send + scan round trip from each language). Phase 5 only requires AVA + clean builds for pyo3/wasm.
- **`chia-sdk-test::silent_payments::tweak_data_from_simulator_block` exposure to bindings** — Phase 6 SC1 + SIM-01.
- **`examples/silent_payment.rs`** — Phase 6 EX-01.
- **Sage-side adapter for SP destinations** — Sage is a separate repo. The post-04.2 `From<Bytes32>` impl means Sage's existing `Action::send` callers keep working without changes.
- **CAT2 / NFT silent-payment send-side bindings** — already deferred to v2 per PROJECT.md "Out of Scope".
- **Async/streaming `TweakData` consumer trait** — `SilentPaymentTweakSource` async trait deferred per PROJECT.md "Out of Scope".

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BIND-01 | `bindings/silent_payments.json` descriptor + `chia-sdk-bindings::silent_payments` facade expose `SilentPaymentKeys` (from_mnemonic, from_secret_keys, scan_pk, spend_pk, unlabeled_address, labeled_address) and `SilentPaymentAddress` (encode, decode, fields) through the `bindy-macro`. | §Standard Stack (bindy schema, class pattern), §Code Examples (address.json + mnemonic.json precedents), §Architecture Patterns (3-part contribution: facade Rust + JSON entry + (rare) shim). |
| BIND-02 | Same descriptor exposes the send-side primitives (`derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`) and the receive primitive (`scan_from_tweaks`, `TweakData`, `DetectedSpCoin`, `LabelRegistry`). Verify `bindy-macro`'s support for static-only function classes; fall back to distributing methods onto carrier types if unsupported. | §Pre-Flight Gate (SC4) — confirmed natively supported via `"type": "static"` on a class with no `new`; precedents at puzzles.json:612/625/675/774/861/868 + mnemonic.json:24 + bls.json:79. Fallback not needed. |

</phase_requirements>

## Summary

Phase 5 plugs the chip-0057 silent-payments Rust surface (already shipped in Phases 1–4.2) into the existing `bindy` descriptor pipeline. There is **no new bindy infrastructure** to build — every shape the SP work needs (4-field classes, static methods on zero-field classes, factory/introspector enum-like opaque handles, unit-variant enums) is already exercised by `address.json`, `mnemonic.json`, `bls.json`, `puzzles.json`, and `action_system.json`. The plan is **schema-by-precedent**: copy the matching pattern, swap the type names.

Three deliverables: (1) a new `bindings/silent_payments.json` descriptor listing 8 bindy entries (`ScalarField`, `SilentPaymentNetwork`, `SilentPaymentAddress`, `SilentPaymentKeys`, `LabelRegistry`, `OutputMeta`, `TweakData`, `DetectedSpCoin`, `SilentPayments`); (2) a `SendDestination` entry appended to `bindings/action_system.json` mirroring the existing `Id` opaque-handle pattern verbatim; (3) facade Rust under `crates/chia-sdk-bindings/src/silent_payments.rs` (or directory) that wraps the underlying types and re-exports them through `lib.rs`. Phase 04.2 already locked the wire-up: `Action.send` is changed from `puzzle_hash: Bytes32` to `destination: SendDestination`, and the SP-keys registration moves to a new `Spends::with_silent_payment_keys` method (also exposed in `action_system.json`).

The chip-0057 feature is wired **unconditionally** on the bindings crate's three workspace dep declarations (driver, utils, types), matching how `offer-compression` and `action-layer` already ship always-on. There is **no `chip-0057` feature on chia-sdk-bindings itself** and **no `#[cfg(...)]` gates inside the facade** — this is a deliberate change vs. chip-0035/chip-0037, which are currently not bindings-exposed.

**Primary recommendation:** Adopt Wave 0 as a one-line pre-flight (verify a zero-field class compiles cleanly under `bindy_napi!()` by writing the simplest possible `SilentPayments` stub with one static returning `u32`, then deleting it). After that, the rest is mechanical descriptor-and-facade work in 3 waves: (A) JSON descriptors + facade types in lockstep, (B) napi build + AVA round-trip test, (C) pyo3/wasm build verification + final phase gate.

## Standard Stack

### Core (already in workspace; no version bumps)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `bindy` | `0.33.0` (path) | `FromRust` / `IntoRust` per-target conversion traits | The runtime side of bindy; loaded by all three binding targets via per-target `Cargo.toml` features. |
| `bindy-macro` | `0.33.0` (path) | `bindy_napi!()` / `bindy_pyo3!()` / `bindy_wasm!()` codegen procedural macros | The compile-time codegen that reads `bindings.json` + every `bindings/*.json`. |
| `chia-sdk-bindings` | `0.33.0` (path) | Hosts the pure-Rust facade types that bindy wraps for each target | The single source of truth for the facade — three binding crates (`napi/`, `pyo3/`, `wasm/`) all consume it via target-specific features. |
| `napi` + `napi-derive` | `3.3.0` / `3.2.5` | Node.js native addon bindings | Already used by every existing AVA test (`napi/__test__/*.spec.ts`). |
| `pyo3` + `pyo3-async-runtimes` | `0.23.5` / `0.23` | Python C extension bindings via maturin | Already used by `pyo3/tests/test_pyo3.py`. |
| `wasm-bindgen` + `wasm-bindgen-derive` | `0.2.100` / `0.3.0` | WebAssembly bindings | Already used by `wasm/__test__/wasm.spec.ts`. |
| `ava` | `^7.0.0` | TypeScript test runner | The AVA test framework used by both `napi/__test__/` and `wasm/__test__/`. |

### Supporting (no new deps required for the facade)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `chia-bls` | `0.36.1` | `SecretKey` / `PublicKey` / `Signature` types | Re-used as-is from `bls.json` (which has `"remote": true` for these three types). Facade methods that take/return SP keys reference these types directly. |
| `chia-protocol` | `0.36.1` | `Bytes32` etc. | The 32-byte buffer marshaling target for napi/pyo3/wasm (mapped via root `bindings.json` `type_groups.{bytes}`). |
| `bip39` | `2.2.0` | `bip39::Mnemonic` (re-used through the existing `Mnemonic` bindy class) | Don't reach for it directly — go through the exposed `Mnemonic` class. |

### Alternatives Considered (NONE — D-01..D-04 lock the design)

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| chip-0057 unconditional on chia-sdk-bindings deps (D-01) | chia-sdk-bindings's own `chip-0057` cargo feature, default-on | Three more places to keep in sync (chia-sdk-bindings + napi + pyo3 + wasm Cargo.toml); easier to accidentally disable. Rejected per D-01. |
| Zero-field `SilentPayments` namespace class (D-02) | Distribute statics onto carrier types (`TweakData.scan`, `SilentPaymentAddress.derive`, etc.) | Loses the namespace discoverability; carrier-type choice is arbitrary. Rejected per D-02. |
| `ScalarField` as bindy class (D-03) | Type-group map to `Bytes32` | Erases the unsigned-vs-signed invariant the Rust newtype is designed to enforce; wallet authors could silently pass unreduced values. Rejected per D-03. |
| `SendDestination` opaque-handle class (D-04) | Inline as field on `Action.send(id, puzzle_hash, ...)` per pre-04.2 | The Rust enum is post-04.2 reality; the descriptor must match. Rejected per D-04. |

**Installation:** Nothing new to install. All dependencies are workspace-pinned and already declared.

**Version verification:** Confirmed `napi = "3.3.0"`, `napi-derive = "3.2.5"`, `pyo3 = "0.23.5"`, `wasm-bindgen = "0.2.100"`, `ava = "^7.0.0"`, `bip39 = "2.2.0"`, `chia-bls = "0.36.1"` from `/home/kdc/chia-wallet-sdk/Cargo.toml` lines 124-184 (workspace.dependencies) + `napi/package.json` line 43. No registry lookups required — versions are pinned by the workspace and frozen by PROJECT.md "Constraints" ("no new workspace deps").

## Architecture Patterns

### Recommended Module/File Layout (Claude's Discretion picks file vs directory)

```
crates/chia-sdk-bindings/
├── Cargo.toml                                 # ← MODIFY: add "chip-0057" to 3 dep feature lists
└── src/
    ├── lib.rs                                 # ← MODIFY: mod silent_payments; pub use silent_payments::*;
    │                                          # ← MODIFY: pub use chia_sdk_driver::{TweakData, OutputMeta, DetectedSpCoin, SendDestination, ...}
    │                                          # ← MODIFY: pub use chia_sdk_utils::silent_payments::{...}
    │                                          # ← MODIFY: pub use chia_sdk_types::silent_payments::ScalarField
    ├── action_system.rs                       # ← MODIFY: rebuild Action::send signature to take SendDestination
    │                                          #            + add Spends::with_silent_payment_keys
    │                                          #            + add SendDestination facade class
    └── silent_payments.rs  (or directory)     # ← NEW: SilentPaymentAddress, SilentPaymentKeys, TweakData,
                                               #         OutputMeta, DetectedSpCoin, LabelRegistry, ScalarField,
                                               #         SilentPayments (namespace) facade types

bindings/
├── silent_payments.json                       # ← NEW: 9 entries (ScalarField, SilentPaymentNetwork,
│                                              #         SilentPaymentAddress, SilentPaymentKeys, LabelRegistry,
│                                              #         OutputMeta, TweakData, DetectedSpCoin, SilentPayments)
└── action_system.json                         # ← MODIFY: add SendDestination class + Action.send signature change
                                               #            + Spends.with_silent_payment_keys method

napi/__test__/
└── silent_payments.spec.ts                    # ← NEW: AVA address round-trip test (SC2)
```

**One file vs directory:** The biggest existing single-file facade is `src/action_system.rs` (~486 lines). The Phase 5 SP facade has ~7 types with simple shapes; estimated ~250-350 lines total. **Recommendation: single file** unless `SilentPaymentKeys`'s methods alone push past ~150 lines, in which case split into a directory matching `puzzle/cat.rs` precedent (one file per type-cluster).

### Pattern 1: Three-Part Bindy Contribution

**What:** Every new bindings-exposed type has three parts that must stay in sync.
**When to use:** Any time you add a type to the cross-language surface.

```rust
// Part A: Rust facade in chia-sdk-bindings/src/silent_payments.rs
// Source: precedent at chia-sdk-bindings/src/address.rs (entire file, 22 lines)
use bindy::Result;
use chia_protocol::Bytes32;

#[derive(Clone)]
pub struct SilentPaymentAddress {
    pub scan_pk: chia_bls::PublicKey,
    pub spend_pk: chia_bls::PublicKey,
    pub network: SilentPaymentNetwork,
}

impl SilentPaymentAddress {
    pub fn encode(&self) -> Result<String> {
        Ok(chia_sdk_utils::silent_payments::SilentPaymentAddress::new(
            self.scan_pk, self.spend_pk, self.network,
        ).encode()?)
    }

    pub fn decode(address: String) -> Result<Self> {
        let info = chia_sdk_utils::silent_payments::SilentPaymentAddress::decode(&address)?;
        Ok(Self { scan_pk: info.scan_pk, spend_pk: info.spend_pk, network: info.network })
    }
}
```

```json
// Part B: bindings/silent_payments.json descriptor entry
// Source: precedent at bindings/address.json (entire file)
{
  "SilentPaymentAddress": {
    "type": "class",
    "new": true,
    "fields": {
      "scan_pk": "PublicKey",
      "spend_pk": "PublicKey",
      "network": "SilentPaymentNetwork"
    },
    "methods": {
      "encode": { "return": "String" },
      "decode": { "type": "factory", "args": { "address": "String" } }
    }
  }
}
```

```rust
// Part C: usually empty. The bindy_napi!()/bindy_pyo3!()/bindy_wasm!() macros
//         in napi/src/lib.rs, pyo3/src/lib.rs, wasm/src/lib.rs auto-generate
//         the typed Rust shims AND the .d.ts/.pyi stubs from parts A + B.
//
//         Hand-written shims are ONLY needed when a type's shape exceeds
//         what bindy can describe (e.g., Clvm's polymorphic alloc takes
//         Either9<f64, BigInt, bool, String, Uint8Array, Array, Null,
//         Undefined, Value1>, and the macro can't express that union). The
//         existing hand-shims in napi/src/lib.rs:10-19, pyo3/src/lib.rs:13-18,
//         wasm/src/lib.rs:20-26 are the ONLY ones in the entire repo as of
//         Phase 04.2.
//
//         Phase 5's SP types do NOT need any hand-written shim — every type
//         has a shape bindy can describe natively.
```

### Pattern 2: Zero-Field Class as Static-Function Namespace (D-02)

**What:** Use a bindy class with no `new`, no `fields`, and only `"type": "static"` methods to expose free functions under a namespace.
**When to use:** When the Rust source has free functions that you want grouped under a TypeScript-discoverable namespace.

```json
// Source: precedent at bindings/mnemonic.json line 23-29 (Mnemonic.verify)
// Source: precedent at bindings/bls.json line 78-86 (PublicKey.aggregate_verify)
// Source: precedent at bindings/puzzles.json lines 612, 625, 675, 774, 861, 868
"SilentPayments": {
  "type": "class",
  "methods": {
    "scan_from_tweaks": {
      "type": "static",
      "args": {
        "scan_sk": "SecretKey",
        "spend_sk": "SecretKey",
        "spend_pk": "PublicKey",
        "data": "TweakData",
        "labels": "LabelRegistry",
        "k_max": "u32"
      },
      "return": "Vec<DetectedSpCoin>"
    },
    "derive_one_time_puzzle_hash": { "type": "static", ... },
    "compute_input_hash":          { "type": "static", ... },
    "aggregate_sender_sks":        { "type": "static", ... }
  }
}
```

**How bindy compiles a static method** (verified at `crates/chia-sdk-bindings/bindy-macro/src/lib.rs:302-324` for napi):
- napi: `#[napi]` attribute, no `&self`, no `factory` modifier → emits `pub fn method_name(env: Env, args...) -> napi::Result<Ret>`. In TS, accessible as `ClassName.methodName(...)`.
- wasm: `#[wasm_bindgen(js_name = ...)]` → emits `pub fn method_name(args...) -> Result<Ret, JsError>`. In TS, accessible as `ClassName.methodName(...)`.
- pyo3: `#[staticmethod]` → emits `pub fn method_name(args...) -> PyResult<Ret>`. In Python, accessible as `ClassName.method_name(...)` (snake_case preserved).

The class itself compiles to a generated `pub struct SilentPayments(chia_sdk_bindings::SilentPayments);` even with no fields — this is fine because the facade type can be a unit struct (`pub struct SilentPayments;`).

**Facade-side shape** (Part A for the static namespace):

```rust
// In crates/chia-sdk-bindings/src/silent_payments.rs:
#[derive(Clone)]
pub struct SilentPayments;

impl SilentPayments {
    pub fn scan_from_tweaks(
        scan_sk: chia_bls::SecretKey,
        spend_sk: chia_bls::SecretKey,
        spend_pk: chia_bls::PublicKey,
        data: TweakData,
        labels: LabelRegistry,
        k_max: u32,
    ) -> bindy::Result<Vec<DetectedSpCoin>> {
        let detected = chia_sdk_driver::scan_from_tweaks(
            &scan_sk, &spend_sk, &spend_pk, &data.into(), &labels.into(), k_max,
        );
        Ok(detected.into_iter().map(Into::into).collect())
    }

    pub fn derive_one_time_puzzle_hash(...) -> bindy::Result<Bytes32> { ... }
    pub fn compute_input_hash(...) -> bindy::Result<ScalarField> { ... }
    pub fn aggregate_sender_sks(...) -> bindy::Result<ScalarField> { ... }
}
```

### Pattern 3: Opaque-Handle Enum (factory + introspector — for SendDestination per D-04)

**What:** Use a bindy class with factory methods as variant constructors and `is_*`/`as_*` methods as introspectors to expose a Rust enum.
**When to use:** When the Rust source is a tagged enum and you want to expose it cross-language without inventing a class-per-variant.

```json
// Source: precedent at bindings/action_system.json lines 222-256 — Id { Xch | Existing(Bytes32) | New(usize) }
// SendDestination follows the same shape:
"SendDestination": {
  "type": "class",
  "methods": {
    "puzzle_hash":       { "type": "factory", "args": { "puzzle_hash": "Bytes32" } },
    "silent_payment":    { "type": "factory", "args": { "address": "SilentPaymentAddress" } },
    "is_puzzle_hash":    { "return": "bool" },
    "as_puzzle_hash":    { "return": "Option<Bytes32>" },
    "is_silent_payment": { "return": "bool" },
    "as_silent_payment": { "return": "Option<SilentPaymentAddress>" }
  }
}
```

**Facade-side shape** (in `chia-sdk-bindings/src/action_system.rs`):

```rust
// Source: precedent at chia-sdk-bindings/src/action_system.rs lines 408-445 — Id facade
#[derive(Clone, Debug)]
pub struct SendDestination(chia_sdk_driver::SendDestination);

impl SendDestination {
    pub fn puzzle_hash(puzzle_hash: Bytes32) -> bindy::Result<Self> {
        Ok(Self(chia_sdk_driver::SendDestination::PuzzleHash(puzzle_hash)))
    }

    pub fn silent_payment(address: SilentPaymentAddress) -> bindy::Result<Self> {
        Ok(Self(chia_sdk_driver::SendDestination::SilentPayment(
            Box::new(address.into()),
        )))
    }

    pub fn is_puzzle_hash(&self) -> bindy::Result<bool> {
        Ok(matches!(self.0, chia_sdk_driver::SendDestination::PuzzleHash(_)))
    }

    pub fn as_puzzle_hash(&self) -> bindy::Result<Option<Bytes32>> {
        Ok(match self.0 {
            chia_sdk_driver::SendDestination::PuzzleHash(ph) => Some(ph),
            _ => None,
        })
    }

    pub fn is_silent_payment(&self) -> bindy::Result<bool> {
        Ok(matches!(self.0, chia_sdk_driver::SendDestination::SilentPayment(_)))
    }

    pub fn as_silent_payment(&self) -> bindy::Result<Option<SilentPaymentAddress>> {
        Ok(match &self.0 {
            chia_sdk_driver::SendDestination::SilentPayment(addr) => Some((**addr).clone().into()),
            _ => None,
        })
    }
}
```

Note: the Rust `SendDestination` enum has its `SilentPayment(Box<SilentPaymentAddress>)` variant `#[cfg(feature = "chip-0057")]`-gated (see `crates/chia-sdk-driver/src/action_system/send_destination.rs:42-43`). Since chip-0057 is unconditional on the bindings crate's driver dep (D-01), this cfg-gate evaluates to "always enabled" in chia-sdk-bindings — no facade-side cfg is needed.

### Pattern 4: Unit-Variant Enum (for `SilentPaymentNetwork` if planner picks the enum shape)

**What:** Use bindy's `Enum { values: [...] }` discriminator to expose a Rust unit-variant enum directly.
**When to use:** When the Rust enum has only unit variants and you want the target language to see it as a native enum.

```json
// Source: bindy-macro/src/lib.rs:50-52 (the Enum variant of Binding)
// Source: precedent at bindings/conditions.json (if SoftforkOpcode etc. exist there — verify at plan time)
"SilentPaymentNetwork": {
  "type": "enum",
  "values": ["Mainnet", "Testnet"]
}
```

**How bindy compiles it** (`bindy-macro/src/lib.rs:445-475` napi, `981-1026` wasm, `1361-1392` pyo3):
- napi: `#[napi]` enum with `Mainnet`/`Testnet` variants. TS sees `SilentPaymentNetwork.Mainnet` / `SilentPaymentNetwork.Testnet`.
- wasm: emits both a Rust enum AND a `typescript_custom_section` `export enum SilentPaymentNetwork { Mainnet = 0, Testnet = 1 }`. Mapped to JS-side number enum.
- pyo3: `#[pyo3::pyclass(eq, eq_int)]` IntEnum. Python sees `SilentPaymentNetwork.Mainnet` / `SilentPaymentNetwork.Testnet`.

This pattern requires the underlying Rust type to be a unit-variant enum, which `SilentPaymentNetwork` is (`crates/chia-sdk-utils/src/silent_payments/address.rs:17-20`).

### Pattern 5: 4-Field Class with `new: true` (for `OutputMeta`)

**What:** Bindy class with `"new": true` and a `fields` table; bindy generates the constructor automatically.
**When to use:** When the underlying Rust type has all-public fields and a derive-able new constructor.

```json
// Source: precedent at bindings/address.json (Address: puzzle_hash + prefix fields, new + encode + decode)
"OutputMeta": {
  "type": "class",
  "new": true,
  "fields": {
    "puzzle_hash":    "Bytes32",
    "coin_id":        "Bytes32",
    "amount":         "u64",
    "parent_coin_id": "Bytes32"
  }
}
```

The underlying Rust type at `crates/chia-sdk-driver/src/silent_payments/types.rs:43-48` already has exactly these 4 fields (and `#[derive(Clone, Copy, Debug)]`). For the facade, follow `chia-sdk-bindings/src/address.rs` — declare a struct with the same 4 fields and provide conversion to/from the underlying type via field copy.

### Anti-Patterns to Avoid

- **Hand-editing `napi/index.d.ts` or `napi/index.js`** — these are GENERATED by `napi build`. Re-running the build will overwrite hand edits. Source: CLAUDE.md line 91.
- **Adding `#[cfg(feature = "chip-0057")]` inside the bindings facade** — D-01 says chip-0057 is unconditional on the chia-sdk-bindings deps, so no facade-side gating is needed. The cfg-gate on the underlying enum variant (in chia-sdk-driver) evaluates to "always enabled" once the dep features are wired.
- **Distributing the SilentPayments statics across carrier types** — D-02 rejects this path. Don't add `TweakData.scan(...)` or `SilentPaymentAddress.derive(...)` methods.
- **Returning raw `[u8; 32]` for `ScalarField`** — D-03 rejects this. Always return the `ScalarField` opaque-handle type so wallet authors can't silently feed unreduced scalars into `derive_one_time_puzzle_hash`.
- **Asserting on the encoded bech32m string in the AVA test** — Phase 04.2 CONTEXT.md notes the assertion should be on `PublicKey.toBytes()` byte-equality, NOT the address string. This survives any future bech32m library churn.
- **Using `chia-sdk-utils::silent_payments::SilentPaymentNetwork::hrp()` to switch on a string in the facade** — Use the bindy enum shape (Pattern 4) for `SilentPaymentNetwork` so the cross-language API is idiomatic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| napi/pyo3/wasm class wrapper boilerplate | Don't write `#[napi] pub struct ...` by hand for every SP type | `bindy_napi!()/bindy_pyo3!()/bindy_wasm!()` macros (already invoked from `napi/src/lib.rs:8`, `pyo3/src/lib.rs:11`, `wasm/src/lib.rs:13`) | The macros generate the wrapper struct, FromRust/IntoRust impls, the napi/wasm/pyo3 `#[derive]` attributes, AND the typescript_custom_section stubs. Hand-writing them duplicates ~50 lines per type with subtle target-specific quirks (napi vs wasm trait bounds, pyo3 async runtime wiring). |
| Bytes32 marshaling | Don't hand-convert `Vec<u8>` ↔ `Uint8Array` ↔ `bytes` ↔ `Vec<u8>` | The root `bindings.json` `type_groups.{bytes}` mapping covers `Vec<u8>`, `Bytes32`, `Bytes48`, `Bytes96`, etc. Bindings.json lines 5-16 list them. | Adding a new bytes-shaped type to type_groups also auto-handles `Vec<Bytes32>` mapping via the macro's apply_mappings logic (bindy-macro/src/lib.rs:1631-1681). |
| Enum value extraction in TypeScript | Don't write a switch statement to convert TS enum to a string HRP | Use bindy's `Enum` binding type (D-03 alternative for SilentPaymentNetwork) | Bindy auto-generates the `#[napi] pub enum`, the FromRust/IntoRust round-trip, AND the matching TS/Python enum stubs. |
| Tagged-enum discriminator | Don't expose a TS union type `PuzzleHash \| SilentPayment` | Opaque-handle class with factory methods + `is_*`/`as_*` introspectors (Pattern 3, `Id` precedent) | bindy doesn't support tagged unions natively. The opaque-handle pattern is the standard escape hatch (see `bindy-macro/src/lib.rs:35-59` Binding enum, which is exactly `Class | Enum { values: Vec<String> } | Function` — no tagged-union variant). |
| ScalarField type-system safety | Don't expose ScalarField as a bare `Uint8Array`/`bytes` | Bindy class with `from_bytes` factory + `to_bytes` getter (D-03) | The Rust newtype enforces unsigned mod-r reduction at construction; erasing the type at the binding boundary lets a TS/Python wallet author feed unreduced bytes into `derive_one_time_puzzle_hash` and silently produce undetectable silent payments. |
| TweakData / DetectedSpCoin wire-format | Don't invent a JSON wire format and hand-marshal | Direct bindy class with public fields | Both types have simple `pub` fields in their Rust definitions (types.rs:31-65); bindy handles `Vec<PublicKey>` and nested struct marshaling natively via `clvm_types` (for `PublicKey`) and the per-target Vec handling (`apply_mappings_with_flavor` in bindy-macro/src/lib.rs:1631-1681). |
| Test fixture mnemonics | Don't generate a fresh mnemonic in the AVA test | Re-use the TV1 mnemonic literal from `crates/chia-sdk-utils/src/silent_payments/keys.rs:152` | The Rust `from_mnemonic_tv1_scan_pk_matches` test already asserts the exact `scan_pk` bytes (`TV1_B_SCAN_PK` at keys.rs:159-162); the AVA test asserting byte-equality against the same fixture mnemonic transitively pins to the same CHIP-pinned values. No reason to use a different mnemonic. |

**Key insight:** This phase has *zero* hand-rolled bindings code if you stay on the rails. The entire napi/pyo3/wasm cross-language surface lands automatically once the JSON descriptor + facade Rust are in place. The only "code" the planner has to write is the facade Rust (~250-350 lines following the address.rs/mnemonic.rs precedent) and the AVA test file (~20-30 lines).

## Runtime State Inventory

Not applicable — this phase is purely additive code/config changes (new descriptor file, new facade module, modified Cargo.toml feature list, modified `lib.rs` re-exports, modified `action_system.json`/`action_system.rs`). No databases, services, OS registrations, secrets, or build artifacts hold stale references to anything that would need migration.

**Stored data:** None — verified, this phase touches no databases, no Mem0/Chroma/Redis state, no n8n workflows.
**Live service config:** None — no external services involved.
**OS-registered state:** None — no Windows tasks, launchd plists, pm2 processes, systemd units.
**Secrets/env vars:** None — no SOPS keys, no .env files, no CI env vars affected.
**Build artifacts:** Re-running `pnpm build` in `napi/` regenerates `index.d.ts` / `index.js` from the new descriptor; re-running `maturin develop` in `pyo3/` regenerates the `.so` and `.pyi`; re-running `wasm-pack build` in `wasm/` regenerates `pkg/`. These are expected and intentional — they're the deliverables. Nothing to migrate.

## Common Pitfalls

### Pitfall 1: Type-Group Re-Use of {bytes} Hides Type Distinctions
**What goes wrong:** Adding a new Bytes32-shaped type to `bindings.json`'s `type_groups.{bytes}` makes it indistinguishable from any other Bytes32 across the binding boundary.
**Why it happens:** Bindy's apply_mappings logic recursively maps `Bytes32` → `Uint8Array` (napi) / `bytes` (pyo3) / `Vec<u8>` (wasm) wherever it appears. If `ScalarField` is added to that group, every method returning a `ScalarField` returns a raw byte array on the other side, losing the type-system invariant.
**How to avoid:** Per D-03, `ScalarField` is NOT in `type_groups.{bytes}`. Expose it as its own bindy class. Wallet authors then go `compute_input_hash(...).toBytes()` and `ScalarField.fromBytes(bytes)` instead of passing raw `Uint8Array` directly into `derive_one_time_puzzle_hash`.
**Warning signs:** Any `pub fn` on the facade that returns or accepts a `chia_sdk_types::silent_payments::ScalarField` directly — if you find one in a code review, the binding probably erased the type.

### Pitfall 2: cfg-Gated Enum Variants Behind a Feature That Isn't Always On
**What goes wrong:** The `SendDestination` enum has `SilentPayment(Box<SilentPaymentAddress>)` gated by `#[cfg(feature = "chip-0057")]`. If chia-sdk-bindings's driver dep doesn't enable chip-0057, the descriptor's `silent_payment` factory references a Rust variant that doesn't exist → compile error.
**Why it happens:** D-01 is the prevention — chip-0057 is unconditional on the chia-sdk-bindings deps. But if someone (a future Phase-6 author, say) re-adds a chia-sdk-bindings cargo feature and conditionally pulls in chip-0057, the descriptor + facade combination silently breaks for the chip-0057-off case.
**How to avoid:** Verify at phase-gate time that `cargo build -p chia-sdk-bindings` (no features) succeeds. If it doesn't, chip-0057 isn't unconditional and D-01 was violated somewhere.
**Warning signs:** Any `[features]` table entry in `crates/chia-sdk-bindings/Cargo.toml` mentioning `chip-0057`; any `#[cfg(feature = "chip-0057")]` attribute inside `crates/chia-sdk-bindings/src/*.rs`. Phase 5 should have ZERO of these.

### Pitfall 3: `napi build` Regenerates `index.d.ts` and `index.js`
**What goes wrong:** Editing `napi/index.d.ts` or `napi/index.js` by hand → next `pnpm build` silently overwrites the edits. Worse: the edits work locally but break in CI because CI rebuilds from scratch.
**Why it happens:** These files are generated by napi-build from the `#[napi]` annotations the bindy macro emits. CLAUDE.md line 91 explicitly warns about this.
**How to avoid:** Treat both files as build artifacts. Any change to the TS surface comes from changing the JSON descriptor or the facade Rust. To verify a TS surface change locally, run `pnpm build` and inspect the resulting `index.d.ts`.
**Warning signs:** Diffs in `napi/index.d.ts` that didn't come from a `pnpm build` run; AVA tests that depend on `index.d.ts` shapes not reproducible from the JSON descriptor.

### Pitfall 4: ScalarField's `from_bytes_unsigned` vs `from_bytes_raw` Choice at the Boundary
**What goes wrong:** The Rust newtype has TWO constructors (`from_bytes_unsigned` reduces mod-r; `from_bytes_raw` does not). The bindy descriptor's `from_bytes` factory has to pick one. If it picks `from_bytes_raw`, a TS/Python caller can construct a `ScalarField` from any 32-byte slice including ones `>= r`, silently re-introducing the signed-vs-unsigned hazard.
**Why it happens:** D-03 specifies the bindy `from_bytes` factory uses `from_bytes_unsigned` semantics (reduces). But a careless facade implementer might wire it to `from_bytes_raw` for "performance" reasons.
**How to avoid:** Confirm at code-review time that the `ScalarField::from_bytes` facade method body calls `ScalarField::from_bytes_unsigned`, NOT `from_bytes_raw`. The `from_bytes_raw` constructor should NOT be exposed cross-language at all in Phase 5.
**Warning signs:** A facade method named `from_bytes_raw` or `from_bytes_unchecked`; a doc comment saying "no reduction" anywhere in the SP facade's `ScalarField` methods.

### Pitfall 5: SilentPaymentNetwork as an Enum vs as a String
**What goes wrong:** If the planner picks the "expose via HRP string" shape, the TS API becomes `keys.unlabeledAddress("spxch")` — a string literal that the binding crate can't validate at compile time. A typo in the JS-side string produces a runtime error instead of a TS-compile error.
**Why it happens:** The HRP string IS the network discriminator at the bech32m layer, so wiring through the HRP feels natural.
**How to avoid:** Use the bindy `Enum { values: ["Mainnet", "Testnet"] }` shape. All three target languages get a typed enum, the JS-side typo turns into a TS-compile error, and the facade boundary preserves the Rust type-system invariant.
**Warning signs:** Any `pub fn` on the facade that takes a `String` argument named `hrp` or `network`; any TS-side test that passes a string literal where a network discriminator is expected.

### Pitfall 6: Workspace Lint Policy on Generated Code
**What goes wrong:** The bindy-emitted napi/wasm/pyo3 code may trigger workspace clippy lints like `clippy::similar_names`, `clippy::needless_pass_by_value`, or `clippy::inherent_to_string`. These ARE caught by workspace clippy (which is `deny clippy::all + warn clippy::pedantic`).
**Why it happens:** Generated code is grammar-driven and doesn't follow naming conventions humans would. The chia-sdk-bindings crate already has 9 `#![allow(...)]` attributes at the top of `lib.rs` (lines 1-9) to suppress these for the entire crate.
**How to avoid:** When adding new types to the SP facade, follow the existing pattern — let the crate-level allows cover the macro-emitted code. If a SP-specific lint fires that's NOT covered by the existing allow list, fix it inline in the facade source (rename a parameter, use `&` instead of by-value, etc.). NEVER add a new `#![allow(...)]` to the crate root; NEVER add `#[allow(...)]` to facade source unless absolutely necessary (the entire Phase 4 + 4.1 + 4.2 work added zero new `#[allow]` attributes).
**Warning signs:** Any new `#[allow(...)]` attribute in a Phase 5 commit; any new `#![allow(...)]` in `lib.rs`.

### Pitfall 7: Mnemonic Constructor Style Affects AVA Test Shape
**What goes wrong:** If `SilentPaymentKeys::from_mnemonic` takes `Mnemonic` by value (move), the TS test can't reuse the mnemonic for subsequent assertions. If it takes by reference, bindy auto-marshals to napi `ClassInstance`. Picking the wrong shape forces test gymnastics.
**Why it happens:** Bindy's napi non-async param mapping uses `ClassInstance<'_, ...>` for any class type (bindy-macro/src/lib.rs:210-213), so passing a Mnemonic to `from_mnemonic` works regardless of by-value-vs-by-ref in the facade signature — but the TS test ergonomics differ slightly.
**How to avoid:** Look at the existing `Mnemonic` precedent at `chia-sdk-bindings/src/mnemonic.rs:13` (`pub fn new(mnemonic: String) -> Result<Self>` — by value) and follow that style. The TS test would then construct one Mnemonic and pass it to `SilentPaymentKeys.fromMnemonic(mnemonic)`.
**Warning signs:** TS test that has to clone or recreate a Mnemonic between assertions; facade method signature that takes `&Mnemonic` (by-ref) where the bindy macro expects an owned type.

## Code Examples

Verified patterns from official sources. All file paths are absolute.

### Example 1: Class with Static Method (the SilentPayments namespace pattern)

```json
// File: /home/kdc/chia-wallet-sdk/bindings/mnemonic.json (lines 23-29 — Mnemonic.verify static)
{
  "Mnemonic": {
    "type": "class",
    "methods": {
      "verify": {
        "type": "static",
        "args": { "mnemonic": "String" },
        "return": "bool"
      }
    }
  }
}
```

```rust
// File: /home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/mnemonic.rs (lines 34-36 — facade-side)
impl Mnemonic {
    pub fn verify(mnemonic: String) -> Result<bool> {
        Ok(bip39::Mnemonic::from_str(&mnemonic).is_ok())
    }
}
```

Result: TS sees `Mnemonic.verify("...")`. Python sees `Mnemonic.verify("...")`. WASM sees `Mnemonic.verify("...")`. This is exactly the shape D-02 specifies for `SilentPayments.scanFromTweaks(...)` etc.

### Example 2: 4-Field Class with new + factory (the OutputMeta + SilentPaymentAddress pattern)

```json
// File: /home/kdc/chia-wallet-sdk/bindings/address.json (entire file)
{
  "Address": {
    "type": "class",
    "new": true,
    "fields": {
      "puzzle_hash": "Bytes32",
      "prefix": "String"
    },
    "methods": {
      "encode": { "return": "String" },
      "decode": {
        "type": "factory",
        "args": { "address": "String" }
      }
    }
  }
}
```

```rust
// File: /home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/address.rs (entire file, 22 lines)
use bindy::Result;
use chia_protocol::Bytes32;

#[derive(Clone)]
pub struct Address {
    pub puzzle_hash: Bytes32,
    pub prefix: String,
}

impl Address {
    pub fn encode(&self) -> Result<String> {
        Ok(chia_sdk_utils::Address::new(self.puzzle_hash, self.prefix.clone()).encode()?)
    }

    pub fn decode(address: String) -> Result<Self> {
        let info = chia_sdk_utils::Address::decode(&address)?;
        Ok(Self {
            puzzle_hash: info.puzzle_hash,
            prefix: info.prefix,
        })
    }
}
```

This is the template for both `OutputMeta` (4 plain fields, no methods beyond auto-generated getters/setters and the auto `new` constructor) and `SilentPaymentAddress` (3 fields — `scan_pk`, `spend_pk`, `network` — plus `encode` method and `decode` factory).

### Example 3: Opaque-Handle Enum (the SendDestination pattern)

```json
// File: /home/kdc/chia-wallet-sdk/bindings/action_system.json (lines 222-256 — Id)
{
  "Id": {
    "type": "class",
    "methods": {
      "xch":         { "type": "factory" },
      "existing":    { "type": "factory", "args": { "asset_id": "Bytes32" } },
      "new":         { "type": "factory", "args": { "index": "usize" } },
      "is_xch":      { "return": "bool" },
      "as_existing": { "return": "Option<Bytes32>" },
      "as_new":      { "return": "Option<usize>" },
      "equals":      { "args": { "id": "Id" }, "return": "bool" }
    }
  }
}
```

```rust
// File: /home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/action_system.rs (lines 408-445 — Id facade)
#[derive(Clone, Debug)]
pub struct Id(sdk::Id);

impl Id {
    pub fn xch() -> Result<Self> { Ok(Self(sdk::Id::Xch)) }
    pub fn existing(asset_id: Bytes32) -> Result<Self> { Ok(Self(sdk::Id::Existing(asset_id))) }
    pub fn new(index: usize) -> Result<Self> { Ok(Self(sdk::Id::New(index))) }
    pub fn is_xch(&self) -> Result<bool> { Ok(self.0 == sdk::Id::Xch) }
    pub fn as_existing(&self) -> Result<Option<Bytes32>> {
        Ok(match self.0 { sdk::Id::Existing(asset_id) => Some(asset_id), _ => None })
    }
    pub fn as_new(&self) -> Result<Option<usize>> {
        Ok(match self.0 { sdk::Id::New(index) => Some(index), _ => None })
    }
    pub fn equals(&self, id: Id) -> Result<bool> { Ok(self.0 == id.0) }
}
```

`SendDestination` follows this exact shape — 2 factories (`puzzle_hash`, `silent_payment`) + 4 introspectors (`is_puzzle_hash`/`as_puzzle_hash`/`is_silent_payment`/`as_silent_payment`).

### Example 4: Remote Type (for re-using chia-bls types via bls.json pattern, if needed)

```json
// File: /home/kdc/chia-wallet-sdk/bindings/bls.json (lines 2-64 — SecretKey with "remote": true)
{
  "SecretKey": {
    "type": "class",
    "remote": true,
    "methods": { "from_seed": { ... }, "to_bytes": { "return": "Bytes32" }, ... }
  }
}
```

```rust
// Facade-side: trait extension pattern (no actual struct definition; the underlying chia_bls::SecretKey
// is the wrapped type). See chia-sdk-bindings/src/bls.rs for the SecretKeyExt trait pattern.
// SecretKey lives in chia-bls, so the bindy facade extends it via SecretKeyExt rather than wrapping it.
```

**Note for Phase 5:** All SP types live INSIDE the workspace, not in an external crate, so the SP descriptor does NOT need any `"remote": true` entries. Just plain classes that wrap the workspace types.

### Example 5: AVA Round-Trip Test Pattern (the SC2 test)

```typescript
// File pattern: /home/kdc/chia-wallet-sdk/napi/__test__/silent_payments.spec.ts (NEW for Phase 5)
//
// Following the existing precedents:
//   - napi/__test__/index.spec.ts (BLS PublicKey roundtrip pattern, line 165-171)
//   - napi/__test__/mips_memos.spec.ts (named-import + new-constructor pattern)
//   - napi/__test__/wasm.spec.ts (Buffer-vs-Uint8Array handling)
import test from "ava";
import {
  Mnemonic,
  SilentPaymentKeys,
  SilentPaymentNetwork,
  SilentPaymentAddress,
} from "../index.js";

const TV1_MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

test("silent-payment address round-trip (TV1 mainnet)", (t) => {
  const mnemonic = new Mnemonic(TV1_MNEMONIC);
  const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
  const address = keys.unlabeledAddress(SilentPaymentNetwork.Mainnet);
  const encoded = address.encode();
  const decoded = SilentPaymentAddress.decode(encoded);

  // Assert on PublicKey.toBytes() byte-equality, NOT the encoded string —
  // survives any future bech32m library churn (per Phase 04.2 CONTEXT.md
  // <specifics>).
  t.deepEqual(decoded.scanPk.toBytes(), keys.scanPk().toBytes());
  t.deepEqual(decoded.spendPk.toBytes(), keys.spendPk().toBytes());
});

test("silent-payment address round-trip (TV1 testnet HRP)", (t) => {
  // Optional secondary test: confirm Testnet network discriminator round-trips
  // and that the TV1 secret keys produce a tspxch1... address.
  const mnemonic = new Mnemonic(TV1_MNEMONIC);
  const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
  const address = keys.unlabeledAddress(SilentPaymentNetwork.Testnet);
  const encoded = address.encode();
  t.true(encoded.startsWith("tspxch1"));
});
```

**Test file location:** `/home/kdc/chia-wallet-sdk/napi/__test__/silent_payments.spec.ts` (new).
**Test runner:** AVA, invoked via `pnpm test` from `napi/`. AVA picks up all `*.spec.ts` files in `__test__/` (configured in `napi/package.json` lines 47-57).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Action::silent_payment_send(recipient, amount, memos)` Rust factory | `Action::send(Id, SendDestination, amount, memos)` with `SendDestination::silent_payment(addr)` | Phase 04.2 (2026-05-17) | Phase 5 SC3 patched per D-04; descriptor reflects this. |
| Dedicated `Spends::finish_with_silent_payment_keys(...)` finish method | `Spends::with_silent_payment_keys(pks, sks)` builder + generic `Spends::finish_with_keys` (chip-0057 internal branch) | Phase 04.2 | Bindings expose `Spends::with_silent_payment_keys`; no separate finish method to bind. |
| Opcode 60/61 announcement binding for multi-input SP | `Relation::AssertConcurrent` cycle binding (no SP-specific marker) | Phase 04.1 | Doesn't affect bindings surface — `Relation` is already an internal-only enum on the Rust side and the binding facade doesn't expose `Relation` directly. |
| `Action::SilentPaymentSend` enum variant | Removed; `SendDestination::SilentPayment` discriminator inside the generic `Action::Send(SendAction)` | Phase 04.2 | The descriptor's `Action.send` method changes signature; that's the only Action.json change. |

**Deprecated/outdated:**
- `Spends::emit_silent_payment_announcements` (deleted in Phase 04.1) — never reached bindings.
- The original SC3 wording referencing a `silent_payment_send` factory descriptor entry — D-04 patched the ROADMAP during Phase 5 discuss-phase.

## Open Questions

1. **Vec<PublicKey> marshaling for TweakData.tweak_points across napi/pyo3/wasm**
   - What we know: `chia-bls::PublicKey` is in the root `bindings.json` `clvm_types` array (line 64) and has a `remote: true` entry in `bindings/bls.json`. `Vec<PublicKey>` should marshal correctly to napi `Array<PublicKey>`, pyo3 `List[PublicKey]`, wasm `PublicKey[]`. The bindy macro's `param_mappings` and `return_mappings` recursively handle `Vec<T>` for any class type (bindy-macro/src/lib.rs:1666-1675).
   - What's unclear: BIND-03 in Phase 6 calls out testing `Vec<chia_bls::PublicKey>` as "a new bindings shape introduced by `TweakData::tweak_points`" — that suggests this exact combination has not been exercised yet. Phase 5 builds will be the first time it compiles in the binding crates.
   - Recommendation: Treat as a build-time risk. If `cargo build -p chia-wallet-sdk-napi` or `wasm-pack build` fails on `Vec<PublicKey>` marshaling, the fix is either (a) explicitly map `Vec<PublicKey>` in root `bindings.json` `wasm` / `wasm_stubs` sections (precedent at line 33-34, 47 for `Vec<Bytes32>` and `Vec<Bytes>`) or (b) wrap the inner vec in a newtype on the facade side. Cost is low; surface the question to the planner so a fallback wave-task is queued.

2. **`Box<SilentPaymentAddress>` in the SendDestination variant — does bindy unbox transparently?**
   - What we know: The Rust enum variant is `SilentPayment(Box<SilentPaymentAddress>)` per Phase 04.2 inline fix (clippy::large_enum_variant). The facade's `as_silent_payment` method needs to dereference the box.
   - What's unclear: Whether the Pattern-3 (Id-precedent) facade body shape I showed above correctly handles the `Box` deref. The Id precedent's `as_existing` returns `Option<Bytes32>` from a `Bytes32` variant (no Box); the SP variant has Box.
   - Recommendation: The facade body for `as_silent_payment` should look like `Some((**addr).clone().into())` (deref the Box, deref-clone the SilentPaymentAddress, into the facade type). This is mechanical Rust; surface to planner as a one-line implementation detail, not a design question.

3. **OutputMeta's `Copy` derive vs facade's `Clone`-only assumption**
   - What we know: The Rust `OutputMeta` is `#[derive(Clone, Copy, Debug)]` (types.rs:42). Bindy facade types are `#[derive(Clone)]` per the macro's generated code (bindy-macro/src/lib.rs:423, 917).
   - What's unclear: Whether putting a `Copy`-able type behind a bindy facade changes any behavior. Likely no — bindy clones at the FFI boundary either way.
   - Recommendation: Drop the assumption that OutputMeta needs to be Copy on the facade side. The facade wraps it as `Clone` and that's fine. No action.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (rustc) | All cargo builds | ✓ | 1.90.0 (pinned via `rust-toolchain.toml`) | — |
| cargo | All cargo builds | ✓ | bundled with rustc | — |
| Node.js | napi build + AVA test runner | ✓ (assumed — required by project per `napi/package.json:13` `"node": ">= 14"`) | >=14 (tested with 20, 22, 24 per stack docs) | None — required for SC2 |
| pnpm | napi `pnpm install && pnpm build && pnpm test` (per CLAUDE.md line 41-42) | ✓ (assumed — pinned `9.11.0` in `napi/package.json:11`) | 9.11.0 | None — required for SC2 |
| napi-cli (`@napi-rs/cli`) | `napi build --platform --release` | ✓ (pulled via `pnpm install` from `napi/package.json` devDependencies line 41) | 3.0.0-alpha.91 | — |
| ava | `pnpm test` in `napi/` | ✓ (pulled via `pnpm install` from devDependencies line 43) | ^7.0.0 | — |
| Python | pyo3 `maturin develop` | ✓ (assumed — required per project Python 3.8+) | 3.8+ | None — required for SC1 pyo3 build |
| maturin | pyo3 `maturin develop` | ✓ (assumed — required by project) | latest | None — required for SC1 pyo3 build |
| pip | maturin auto-installs into venv | ✓ (assumed) | bundled with Python | — |
| wasm-pack | wasm `wasm-pack build` | ✓ (assumed — pulled via cargo-install per stack docs) | latest | None — required for SC1 wasm build |
| Rust target wasm32-unknown-unknown | wasm-pack build target | needs verification at phase start | — | `rustup target add wasm32-unknown-unknown` if missing |

**Missing dependencies with no fallback:**
- None known. All toolchain dependencies were used to ship Phases 1-4.2 successfully (commit history shows clean napi/pyo3/wasm builds throughout).

**Missing dependencies with fallback:**
- `wasm32-unknown-unknown` Rust target: if not installed, `rustup target add wasm32-unknown-unknown` adds it in <1 minute. Wave 0 should include a `rustup show` check to verify.

**Pre-flight verification commands:**
```bash
# Verify Rust toolchain
cd /home/kdc/chia-wallet-sdk && rustc --version    # expect 1.90.0
cd /home/kdc/chia-wallet-sdk && rustup show

# Verify node/pnpm
node --version
pnpm --version

# Verify python/maturin
python3 --version
maturin --version || python3 -m pip install maturin

# Verify wasm-pack
wasm-pack --version || cargo install wasm-pack
```

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework (Rust) | cargo test (no separate framework needed; built into rustc) |
| Framework (TS — napi) | AVA `^7.0.0` (configured in `/home/kdc/chia-wallet-sdk/napi/package.json:47-57`) |
| Framework (TS — wasm) | AVA `^7.0.0` (configured in `/home/kdc/chia-wallet-sdk/wasm/`) |
| Framework (Python) | pytest (used by `/home/kdc/chia-wallet-sdk/pyo3/tests/test_pyo3.py`) |
| Config file (napi AVA) | `/home/kdc/chia-wallet-sdk/napi/package.json` lines 47-57 (inline `ava` block) |
| Config file (workspace clippy) | `/home/kdc/chia-wallet-sdk/Cargo.toml` lines 55-67 (workspace.lints.clippy) |
| Quick run command (Rust, scoped to chia-sdk-bindings) | `cargo build -p chia-sdk-bindings --features napi` |
| Full suite command (Rust + bindings) | `cargo build --release --workspace --all-features && cargo clippy --workspace --all-features --all-targets` |
| napi AVA quick run | `cd napi && pnpm test -- --match 'silent-payment*'` (AVA's `--match` flag filters by test title pattern) |
| napi full AVA run | `cd napi && pnpm install && pnpm build && pnpm test` |
| pyo3 build verify | `cd pyo3 && maturin develop` |
| wasm build verify | `cd wasm && pnpm install && pnpm test` (the `pnpm test` script in `wasm/package.json` runs `wasm-pack build --target nodejs` before AVA — confirm at plan time) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BIND-01 | `bindings/silent_payments.json` exposes `SilentPaymentKeys` + `SilentPaymentAddress` through bindy; `chia-sdk-bindings` builds cleanly | unit (compile check) | `cargo build -p chia-sdk-bindings --features napi --features wasm --features pyo3` | ❌ Wave 1 (descriptor + facade lands together; the compile is the test) |
| BIND-01 | Generated `napi/index.d.ts` exposes `SilentPaymentAddress`, `SilentPaymentKeys` etc. as named exports | integration | `cd napi && pnpm install && pnpm build && grep -E 'export (class \|enum) (SilentPaymentAddress\|SilentPaymentKeys\|TweakData\|DetectedSpCoin\|LabelRegistry\|ScalarField\|SilentPayments\|SilentPaymentNetwork\|OutputMeta\|SendDestination)' napi/index.d.ts` | ❌ Wave 2 (after napi build runs) |
| BIND-01 | Address round-trip test asserts byte-equality of scan_pk/spend_pk | integration | `cd napi && pnpm test -- --match 'silent-payment address round-trip*'` | ❌ Wave 2 — `napi/__test__/silent_payments.spec.ts` |
| BIND-02 | Send-side primitives exposed through `SilentPayments.scanFromTweaks` etc. as static functions | integration | `cd napi && pnpm install && pnpm build && grep -E 'static (scanFromTweaks\|deriveOneTimePuzzleHash\|computeInputHash\|aggregateSenderSks)' napi/index.d.ts` | ❌ Wave 2 (after napi build runs) |
| BIND-02 | bindy-macro static-functions schema verified — pre-flight per SC4 | unit (compile check) | `cargo build -p chia-sdk-bindings --features napi` after adding the minimal `SilentPayments` zero-field class entry | ❌ Wave 0 |
| Phase SC1 | `cargo build --workspace --all-features` succeeds | unit (compile check) | `cargo build --release --workspace --all-features` | ✓ (existing CI gate) |
| Phase SC1 | `napi build` succeeds | integration (build check) | `cd napi && pnpm install && pnpm build` | ✓ (existing CI gate) |
| Phase SC1 | `maturin develop` succeeds in pyo3/ | integration (build check) | `cd pyo3 && maturin develop` | ✓ (existing CI gate) |
| Phase SC1 | `wasm-pack build` succeeds in wasm/ | integration (build check) | `cd wasm && pnpm install && pnpm test` (runs wasm-pack build as a preceding script) | ✓ (existing CI gate) |
| Phase SC3 | `bindings/action_system.json` has SendDestination class with factory + introspectors | unit (descriptor verification + compile) | `jq '.SendDestination.methods | keys' bindings/action_system.json && cargo build -p chia-sdk-bindings --features napi` | ❌ Wave 1 (descriptor + facade lands together) |
| Phase SC3 | TS caller can construct an SP send via `Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos)` | integration (TS surface verification — type-check only, not runtime test) | The grep in BIND-01's `napi/index.d.ts` line for `Action.send`'s second parameter type. If the .d.ts shows `destination: Uint8Array` (the old Bytes32 shape), descriptor wasn't updated. If it shows `destination: SendDestination`, descriptor was updated. | ❌ Wave 2 (after napi build) |
| Phase SC4 | bindy-macro static-functions confirmed natively supported | unit (descriptor verification + Wave 0 compile-test) | One-line `SilentPayments` stub with one static `noop -> u32` method; `cargo build -p chia-sdk-bindings --features napi`; if clean, native support confirmed. Stub then expanded to full 4-static surface in Wave 1. | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo build -p chia-sdk-bindings --features napi` (the cheapest gate that exercises the bindy macro expansion). Time budget: ~15-30 seconds incremental.
- **Per wave merge:** `cargo build --release --workspace --all-features && cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings && cargo machete`. Time budget: ~2-3 minutes incremental.
- **Phase gate:** Full suite green:
  ```
  cargo build --release --workspace --all-features
  cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro
  cargo clippy --workspace --all-features --all-targets -- -D warnings
  cargo fmt --all -- --files-with-diff --check
  cargo machete
  cd napi && pnpm install && pnpm build && pnpm test
  cd pyo3 && maturin develop && pytest
  cd wasm && pnpm install && pnpm test
  ```
  before `/gsd:verify-work` for the phase.

### Wave 0 Gaps

- [ ] **bindy static-functions pre-flight stub (SC4):** Land a one-line `SilentPayments` zero-field class entry in `bindings/silent_payments.json` with ONE no-op static method (e.g., `noop -> u32` returning a literal). Land the corresponding `pub struct SilentPayments;` + `impl SilentPayments { pub fn noop() -> bindy::Result<u32> { Ok(0) } }` facade. Run `cargo build -p chia-sdk-bindings --features napi --features wasm --features pyo3`. If clean → SC4 PASS (native support confirmed). Delete the stub before Wave 1 expands the descriptor.
- [ ] **Toolchain probe (`rustup show` to confirm `wasm32-unknown-unknown` target is installed):** If missing, run `rustup target add wasm32-unknown-unknown`.
- [ ] **Confirm pnpm + ava + maturin + wasm-pack are runnable locally** (one-line `--version` calls — see "Pre-flight verification commands" above).
- [ ] **No new test file needed for Rust-side tests** — the existing `cargo test` workspace suite (CI excludes binding crates per CLAUDE.md lines 20-23) already covers all Phase-1..4.2 chip-0057 behavior. Phase 5 adds no new Rust unit tests; it adds the bindings.json descriptor + facade Rust + ONE new AVA test file.

*(If no gaps after Wave 0 closes: "None — Wave 0 confirmed bindy native support for static-only classes; all toolchain dependencies present; the rest of Phase 5 is straightforward descriptor + facade work")*

## Pre-Flight Gate (SC4) Verdict

**Verdict: NATIVE SUPPORT CONFIRMED — fallback strategy NOT required.**

**Evidence:**

1. `bindy-macro/src/lib.rs:36-59` defines `Binding` as `Class | Enum { values: Vec<String> } | Function`. The `Class` variant accepts `methods: IndexMap<String, Method>` where `Method.kind` is `MethodKind`. `MethodKind::Static` is one of the variants (lines 82-83).

2. The napi codegen path for `MethodKind::Static` is at `bindy-macro/src/lib.rs:302-303` (`MethodKind::Static => quote!(#[napi])`) + `lib.rs:311-324` (constructor/static/factory share the same code-gen branch, emitting `pub fn` with no `&self`). The wasm path is at `lib.rs:725-737`. The pyo3 path is at `lib.rs:1210-1212` + `lib.rs:1229-1241`.

3. **Zero-field class is structurally valid:** the codegen at `lib.rs:421-443` (napi), `lib.rs:914-936` (wasm), `lib.rs:1337-1359` (pyo3) emits `pub struct ClassName(rust_struct_ident);` regardless of whether the `fields` table is empty. The bindy struct itself can be a unit struct (`pub struct SilentPayments;`); the macro-generated wrapper struct will be `pub struct SilentPayments(chia_sdk_bindings::SilentPayments);` and the FromRust/IntoRust impls trivially round-trip the unit value.

4. **Existing precedents (all in shipped bindings code):**
   - `Mnemonic.verify` (bindings/mnemonic.json line 23-29) — static method on a non-zero-field class.
   - `PublicKey.aggregate_verify` (bindings/bls.json lines 78-86) — static method on a remote class.
   - `puzzles.json` lines 612, 625, 675, 774, 861, 868 — 6 distinct static methods on various puzzle-type classes.
   - `bindings/action_system.json` Deltas.from_actions (line 190) — static method that takes input and returns the wrapping type.

   Of these, the closest analog to a zero-field-class-with-only-statics is the `Constants` class at the top of `bindings/constants.json` (referenced by `bindy-macro/src/lib.rs:110-136`), which has NO `new`, NO `fields`, and ONLY methods (all populated programmatically at macro expand time, but conceptually identical to the `SilentPayments` shape).

5. **The fallback (distributed statics across carrier types) is NOT needed.** If we wanted to fall back, we'd hang `scan_from_tweaks` on `TweakData` (`TweakData.scan(scan_sk, ...)`), `derive_one_time_puzzle_hash` on `SilentPaymentAddress`, etc. The result would be discoverable but less canonical (which class owns `aggregate_sender_sks`? It takes a `Vec<SecretKey>` and returns a `ScalarField` — neither type is the obvious carrier). D-02 picks the namespace pattern explicitly.

**Wave 0 validation:** Per the Wave 0 entry above, the planner should write a one-line `SilentPayments` stub with one no-op static method and run `cargo build -p chia-sdk-bindings --features napi --features wasm --features pyo3`. This is a 15-minute compile-test; if it passes, the verdict above is confirmed empirically.

**Action item:** The Wave 0 task that lands the stub and the compile-verification should produce an artifact (a commit message + one-line log) recording the verdict. No additional documentation required.

## Project Constraints (from CLAUDE.md)

These are extracted from `/home/kdc/chia-wallet-sdk/CLAUDE.md` and from PROJECT.md Constraints. Plans must honor all of them.

- **Tech stack:** Rust 1.90.0 (pinned via `rust-toolchain.toml`), edition 2024. Cannot use nightly-only features.
- **No new workspace deps.** Phase 5 introduces ZERO new packages — every type used is already pinned (`napi`, `napi-derive`, `pyo3`, `wasm-bindgen`, `bindy`, `bindy-macro`, `chia-bls`, `chia-protocol`, `bip39`, `ava`).
- **`unsafe_code = "deny"`** at workspace level. The bindy-emitted napi/wasm/pyo3 code does NOT use unsafe blocks (verified by surveying the existing chia-sdk-bindings + napi/wasm/pyo3 src trees).
- **Workspace clippy = `deny clippy::all + warn clippy::pedantic + warn clippy::cargo`.** Locally treat pedantic as errors via `-D warnings` per CLAUDE.md line 51.
- **`dead_code = "deny"`** at workspace level. Anything declared in the facade Rust must be referenced (the bindy macro references everything it sees in the JSON, so the JSON descriptor effectively guarantees this).
- **`cargo machete`** runs in CI. Don't add unused `[dependencies]` entries to `crates/chia-sdk-bindings/Cargo.toml`. The 3 deps that need features added (`chia-sdk-driver`, `chia-sdk-utils`, `chia-sdk-types`) are already listed; modifying their `features = [...]` doesn't introduce new entries.
- **`chip-0057` is the ONLY feature umbrella for this work.** No `chip-0057-bindings`, no sub-features, no per-target chip-0057 aliases. Per D-01, the bindings crate doesn't even have its own chip-0057 feature — it consumes chip-0057 transitively via its driver/utils/types deps.
- **CI builds each crate individually with and without `--all-features`.** Verify both modes compile (`cargo build -p chia-sdk-bindings` and `cargo build -p chia-sdk-bindings --all-features`) at the phase gate.
- **Bindings format:** Wire-protocol surfaces (`TweakData`, `DetectedSpCoin`) MUST be expressible in `bindings/silent_payments.json` so napi/pyo3/wasm get them. Bytes32 + lists of (PublicKey, OutputMeta) is the granularity to design around. CONFIRMED MET by Patterns 1 + 5 + the static namespace.
- **Forward compatibility with CHIP-0058:** `TweakData` has no transport fields. The bindings facade preserves this — the descriptor exposes only `tweak_points: Vec<PublicKey>` and `outputs: Vec<OutputMeta>`. A future CHIP-0058 transport client constructs `TweakData` from its wire messages without breaking the API.
- **Re-running clippy before reporting "done"** is a hard requirement per CLAUDE.md line 51. The phase gate includes a `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` line.
- **LSP over grep for code navigation** per CLAUDE.md `### Code Intelligence` section. Phase 5 has heavy work in `chia-sdk-bindings/src/action_system.rs` and `lib.rs`; use LSP `findReferences` to confirm every place `Action::send` is called still compiles after the descriptor + facade signature change.

## Sources

### Primary (HIGH confidence)

- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/bindy-macro/src/lib.rs` (entire file, 1720 lines) — the canonical Binding/Method/MethodKind schema definition (lines 35-86) + the three `bindy_*!` proc macros. Confirms native support for `Class | Enum | Function` with `MethodKind::{Normal, Async, ToString, Static, Factory, AsyncFactory, Constructor}`. Confirms zero-field classes are structurally valid.
- `/home/kdc/chia-wallet-sdk/bindings.json` — root descriptor with type_groups, per-target mappings, clvm_types list. Confirms `Vec<u8>`/`Bytes32`/`Bytes48`/`Bytes96` are in the `{bytes}` group (lines 5-16).
- `/home/kdc/chia-wallet-sdk/bindings/mnemonic.json` — Mnemonic class precedent for D-02 (`verify` static).
- `/home/kdc/chia-wallet-sdk/bindings/bls.json` — `remote: true` class precedent + `PublicKey.aggregate_verify` static precedent (line 78).
- `/home/kdc/chia-wallet-sdk/bindings/address.json` — 4-field class with `new: true` precedent (the OutputMeta template).
- `/home/kdc/chia-wallet-sdk/bindings/action_system.json` — `Id` opaque-handle pattern (lines 222-256), the direct template for `SendDestination` per D-04.
- `/home/kdc/chia-wallet-sdk/bindings/puzzles.json` lines 612, 625, 675, 774, 861, 868 — 6 distinct `"type": "static"` precedents.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/lib.rs` — current module list + chia_sdk_driver re-export pattern (lines 77-84). Modified in Phase 5 to add silent_payments mod + re-exports.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/Cargo.toml` lines 17-30 — current `[features]` + `[dependencies]` shape. Modified in Phase 5 per D-01.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/address.rs` — entire 22-line file is the template for `SilentPaymentAddress` facade.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/mnemonic.rs` — entire 49-line file is the template for `SilentPaymentKeys` facade.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-bindings/src/action_system.rs` lines 408-445 — `Id` facade implementation, the direct template for `SendDestination` facade.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-utils/src/silent_payments/{mod,address,error,keys,labels}.rs` — Phase 2 Rust surface being exposed.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/silent_payments/{mod,types,scanner,protocol,aggregate,input_hash,one_time,send_keys}.rs` — Phase 3/4 Rust surface being exposed.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-types/src/silent_payments/scalar.rs` — `ScalarField` newtype being exposed per D-03.
- `/home/kdc/chia-wallet-sdk/crates/chia-sdk-driver/src/action_system/send_destination.rs` — `SendDestination` enum (the Rust source post-04.2).
- `/home/kdc/chia-wallet-sdk/napi/__test__/index.spec.ts`, `napi/__test__/mips_memos.spec.ts`, `napi/__test__/action_system.spec.ts` — AVA test pattern precedents.
- `/home/kdc/chia-wallet-sdk/napi/src/lib.rs`, `wasm/src/lib.rs`, `pyo3/src/lib.rs` — confirm `bindy_napi!("bindings.json")`/`bindy_wasm!("bindings.json")`/`bindy_pyo3!("bindings.json")` are the sole macro invocations + minimal hand-shim for `Clvm.alloc` only.
- `/home/kdc/chia-wallet-sdk/.planning/REQUIREMENTS.md` — BIND-01, BIND-02 phase-mapped to Phase 5.
- `/home/kdc/chia-wallet-sdk/.planning/ROADMAP.md` Phase 5 entry — 4 success criteria, dependencies on Phases 2/3/4/4.2.
- `/home/kdc/chia-wallet-sdk/.planning/phases/05-bindings-rust-facade-json-descriptor/05-CONTEXT.md` — D-01 through D-04 + Claude's Discretion items + specifics with JSON sketches.
- `/home/kdc/chia-wallet-sdk/.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-PHASE-SUMMARY.md` (referenced in CONTEXT.md canonical_refs) — final state of the Rust API.
- `/home/kdc/chia-wallet-sdk/Cargo.toml` lines 73-80 (workspace [features]) — confirms chip-0057 workspace feature cascade.

### Secondary (MEDIUM confidence)

- (None — Phase 5 research is entirely repo-local; no external sources required.)

### Tertiary (LOW confidence — flagged for validation)

- (None.)

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — every type used is workspace-pinned; no version lookups required; bindy-macro is the in-repo procedural macro whose source I read in full.
- Architecture patterns: **HIGH** — all 5 patterns are direct copies of existing in-repo precedents I read line-by-line (mnemonic.json, bls.json, address.json, action_system.json, puzzles.json + their facade Rust counterparts).
- SC4 pre-flight verdict (native support, no fallback needed): **HIGH** — verified by reading the macro source (`bindy-macro/src/lib.rs:35-86` schema + lines 302-324 napi-static codegen + 725-737 wasm-static codegen + 1210-1241 pyo3-static codegen) AND by listing 6 in-tree precedents. The one-line Wave 0 compile-test will confirm empirically.
- Pitfalls: **HIGH** — derived from observed in-repo facts (workspace lint policy, current state of chia-sdk-bindings facade, existing hand-shim minimization). Pitfall 1 (type-group hides type) and Pitfall 4 (from_bytes_unsigned vs raw) are specifically about D-03 invariant preservation.
- Open questions: **MEDIUM** — Q1 (Vec<PublicKey> marshaling) is a "first-time-it-runs" risk that Wave 1 will resolve at compile time. Q2 (Box deref shape) is a one-line implementation detail. Q3 (Copy vs Clone) is a non-issue.

**Research date:** 2026-05-17
**Valid until:** 2026-06-17 (~30 days; descriptor-pipeline architecture is stable since at least Phase 04.2 and predates the chip-0057 work entirely. The only thing that could invalidate this research is a major bindy refactor, which would invalidate everything else too.)

## RESEARCH COMPLETE

Phase 5 is a pure descriptor + facade phase with no design surface left to explore — D-01..D-04 lock the design, the bindy schema is verified-native-supported for D-02's zero-field-class pattern, and every type being exposed has an existing in-repo precedent to mirror byte-for-byte. The planner now has: (1) line-and-file citations for the 5 binding patterns to use; (2) the JSON sketches for SilentPayments, ScalarField, and SendDestination already locked in CONTEXT.md; (3) a clear Wave 0 pre-flight gate for SC4 (one-line stub + compile-test); (4) the full build/test command matrix for SC1's per-target build verification + SC2's AVA round-trip test; (5) confirmation that no new workspace deps, no `#[cfg(feature = "chip-0057")]` in the facade, and no hand-written napi/pyo3/wasm shims are needed.
