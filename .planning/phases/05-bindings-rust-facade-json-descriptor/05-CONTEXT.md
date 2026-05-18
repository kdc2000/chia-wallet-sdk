# Phase 5: Bindings (Rust facade + JSON descriptor) - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose the chip-0057 silent-payments Rust surface through the bindy descriptor pipeline so TypeScript, Python, and WASM consumers can use it through their idiomatic API. After this phase:

- `bindings/silent_payments.json` exists and declares the SP types: `SilentPaymentAddress`, `SilentPaymentKeys`, `SilentPaymentNetwork`, `LabelRegistry`, `TweakData`, `OutputMeta`, `DetectedSpCoin`, `ScalarField`, and a zero-field `SilentPayments` namespace carrying the 4 free functions.
- `bindings/action_system.json` gains a `SendDestination` opaque-handle class entry (factory methods + `is_*`/`as_*` introspectors) per Phase 04.2 SC12 — supersedes the original SC3 wording.
- `crates/chia-sdk-bindings/src/silent_payments.rs` (or directory) hosts the pure-Rust facade types and conversion glue (planner picks file-vs-directory based on size).
- `chia-sdk-bindings` builds clean with chip-0057 unconditionally enabled via its dep declarations on chia-sdk-driver / chia-sdk-utils / chia-sdk-types. No `chip-0057` feature on chia-sdk-bindings itself.
- `napi/`, `pyo3/`, `wasm/` build clean via `napi build`, `maturin develop`, and `wasm-pack build` respectively.
- Generated `napi/index.d.ts` exposes all SC1 symbols: `SilentPaymentAddress`, `SilentPaymentKeys`, `TweakData`, `DetectedSpCoin`, `LabelRegistry`, and the four static functions as `SilentPayments.scanFromTweaks`/`SilentPayments.deriveOneTimePuzzleHash`/`SilentPayments.computeInputHash`/`SilentPayments.aggregateSenderSks`.
- `napi/__test__/silent_payments.ts` proves the address round-trip: TV1 mnemonic → `SilentPaymentKeys` → unlabeled address → encode/decode → assert `scan_pk`/`spend_pk` bytes match (SC2).

**Not in scope** (these are separate efforts):
- Cross-language send + scan full round-trip from TS/Py/WASM — Phase 6's concern (SIM-02, SIM-03, BIND-03).
- pyo3 and wasm tests — Phase 6 covers those. Phase 5 only requires that pyo3/wasm **build** clean.
- `chia-sdk-test::silent_payments::tweak_data_from_simulator_block` exposure — Phase 6 SC1.
- `examples/silent_payment.rs` — Phase 6 EX-01.
- Sage-repo or any downstream consumer adoption — outside the SDK.

</domain>

<decisions>
## Implementation Decisions

### Feature wiring
- **D-01:** **chip-0057 is unconditional in chia-sdk-bindings's deps.** `chia-sdk-bindings`'s `Cargo.toml` `[dependencies]` declarations on `chia-sdk-driver`, `chia-sdk-utils`, and `chia-sdk-types` add `"chip-0057"` to the `features = [...]` list. No new `chip-0057` cargo feature on chia-sdk-bindings; no `#[cfg(feature = "chip-0057")]` gates inside the binding facade. Matches the existing pattern for `offer-compression` and `action-layer` (both always-on on the driver dep). Justification: simplifies the build matrix, prevents accidental disablement by binding consumers, and matches SC1's "enabled-by-default" wording — there is no consumer asking for an SP-disabled binding.

### Static functions carrier
- **D-02:** **Zero-field `SilentPayments` namespace class** for the four free functions. Add a `SilentPayments` class to `bindings/silent_payments.json` with no fields, no constructor, and 4 static methods:
  - `scan_from_tweaks(scan_sk: SecretKey, spend_sk: SecretKey, spend_pk: PublicKey, data: TweakData, labels: LabelRegistry, k_max: u32) -> Vec<DetectedSpCoin>`
  - `derive_one_time_puzzle_hash(scan_pk: PublicKey, spend_pk: PublicKey, aggregated_sender_sk: ScalarField, input_hash: ScalarField, k: u32) -> Bytes32`
  - `compute_input_hash(coin_ids: Vec<Bytes32>, aggregated_sender_pk: PublicKey) -> ScalarField`
  - `aggregate_sender_sks(sks: Vec<SecretKey>) -> ScalarField`

  TS call shape: `SilentPayments.scanFromTweaks(...)`. Mirrors the discoverable-namespace pattern used by Bitcoin SP libraries. Confirms SC4's primary path (the zero-field-class-with-static-functions approach) — the fallback (distributed statics across carrier types) is not needed; bindy's `"type": "static"` mechanism on a class with no constructor already works (precedents: `Mnemonic.verify`, `PublicKey.aggregate_verify`).

### ScalarField exposure
- **D-03:** **`ScalarField` is exposed as a bindy class** in `silent_payments.json`. Two methods:
  - `from_bytes(bytes: Bytes32) -> ScalarField` (factory; uses `ScalarField::from_bytes_unsigned` semantics — accepts any 32-byte input and reduces mod r)
  - `to_bytes() -> Bytes32` (getter, returns the canonical 32-byte big-endian representation of the reduced scalar)

  Preserves the type-system distinction across the binding boundary: `compute_input_hash` and `aggregate_sender_sks` return `ScalarField` (not raw `Bytes32`), and `derive_one_time_puzzle_hash` takes `&ScalarField` arguments. Without this, wallet authors composing the primitives in TS/Py/WASM would have to pipe raw `Uint8Array`/`bytes` through and silently risk passing unreduced values. NOT mapped to `Bytes32` via type-group — that would erase the invariant the Rust type guards.

  Note: SC1 lists `derive_one_time_puzzle_hash`/`compute_input_hash`/`aggregate_sender_sks` but not `ScalarField` explicitly; D-03 adds `ScalarField` to the exposed surface so the three primitives can compose in any target language without manual byte-shuffling.

### SC3 reconciliation
- **D-04:** **Patched ROADMAP.md Phase 5 SC3** during this discussion to reflect the post-04.2 reality. New SC3 text: a `SendDestination` opaque-handle class entry in `action_system.json` with factory methods (`puzzle_hash(Bytes32)` + `silent_payment(SilentPaymentAddress)`) and introspectors (`is_puzzle_hash`/`as_puzzle_hash`/`is_silent_payment`/`as_silent_payment`). Pre-committed by Phase 04.2 SC12 (and `04.2-CONTEXT.md` <specifics>). TS callers construct an SP send via `Action.send(Id.xch(), SendDestination.silentPayment(addr), amount, memos)`. The original SC3 wording referencing `silent_payment_send` factory was stale — Phase 04.2 deleted that constructor.

### Claude's Discretion
The following are left to the planner to decide based on cohesion / file-size / convention concerns at planning time:

- **`chia-sdk-bindings::silent_payments` module shape** — one `silent_payments.rs` file (matches every existing concept module: `address.rs`, `mnemonic.rs`, `bls.rs`) vs. a `silent_payments/` directory with sub-files (`address.rs`, `keys.rs`, `tweak_data.rs`, `static_fns.rs`). Decide based on resulting file size.
- **`SilentPaymentKeys` constructor shape** — `from_mnemonic(Mnemonic) -> SilentPaymentKeys` as factory (Mnemonic class is already exposed) vs. a `new(scan_sk: SecretKey, spend_sk: SecretKey)` constructor with `from_mnemonic` and `from_seed` as factories. Planner picks based on what reads best in TS — both are valid bindy patterns.
- **`SilentPaymentNetwork` shape** — bindy `Enum { values: ["Mainnet", "Testnet"] }` (unit-variant enum, the bindy-supported pattern for `mainnet`/`testnet` selection) vs. exposed via the HRP string. Enum is the more idiomatic call shape; planner verifies bindy enum codegen works in all three targets.
- **`LabelRegistry` exposed surface** — full `register`/`lookup`/`get` API vs. minimal `new()` + `register(m, label_pk)`. Planner picks based on what the AVA test + future Phase 6 SIM-03 helper needs; over-exposing here risks descriptor churn in Phase 6.
- **`OutputMeta` exposure** — TweakData has `outputs: Vec<OutputMeta>`, so `OutputMeta` is implicitly required. Planner declares it as a bindy class with `new` + 4 public fields (`puzzle_hash`, `coin_id`, `amount`, `parent_coin_id`) following the `Address` 4-field-class precedent.
- **AVA test fixture mnemonic** — reuse the TV1 mnemonic that drives the Rust tests in `crates/chia-sdk-utils/src/silent_payments/keys.rs::tests` so the TS test asserts the same on-chain bytes the Rust test does. Planner pulls the exact 12/24-word string from there.
- **pyo3/wasm smoke tests for Phase 5** — SC2 only requires AVA. Planner may add minimal pyo3 import-and-call tests (no SP logic) just to catch build/descriptor-codegen breakage that would otherwise surface only in Phase 6. Not required; nice-to-have.
- **Privacy-warning rustdoc propagation** — every memo-bearing public surface in the Rust crate carries `/// Privacy warning: memos are stored on-chain in plaintext...` (Phase 4 SEND-08). Planner decides whether the binding facade's memo-bearing surfaces (`Action.send` with `SendDestination::silent_payment`, any SP-related descriptor entry mentioning memos) need a parallel TS/Py docstring or whether the Rust-level warning is enough. Recommendation: mirror in the bindy descriptor where possible (`description` field on the method) so the generated `.d.ts` / `.pyi` carries the warning.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 5 scope and history
- `.planning/ROADMAP.md` §"Phase 5: Bindings (Rust facade + JSON descriptor)" — 4 success criteria + goal + dependencies. SC3 was patched during 05 discuss-phase per D-04 to reflect the post-04.2 API.
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-CONTEXT.md` <specifics> + canonical_refs §"Bindings descriptor pattern" — pre-committed SendDestination opaque-handle pattern (SC12 + ~12-line JSON sketch).
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-PHASE-SUMMARY.md` — final state of the Rust API that Phase 5 exposes.
- `.planning/STATE.md` "Roadmap Evolution" — discovery context.

### bindy descriptor pattern (the authoritative schema)
- `crates/chia-sdk-bindings/bindy-macro/src/lib.rs` — the `Binding` schema: `Class | Enum { values: Vec<String> } | Function`. Only unit-variant enums are expressible; tagged unions use the opaque-handle pattern (factory + `is_*`/`as_*`).
- `bindings.json` (root) — type_groups (`{bytes}`/`{bigint}`/`{number}`/`{usize}`) and per-target conversion overrides (`napi`/`wasm`/`pyo3`/`wasm_stubs`). ScalarField (D-03) does NOT go into type_groups; it's exposed as a class.
- `bindings/bls.json` — `SecretKey`, `PublicKey`, `Signature` shape. `remote: true` pattern for upstream types (chia-bls types live outside the binding facade).
- `bindings/mnemonic.json` — `Mnemonic` class with `new` constructor + `from_entropy`/`generate` factories + `verify` static. Direct precedent for D-02 static-functions pattern.
- `bindings/address.json` — `Address` 4-field class precedent (`puzzle_hash` + `prefix` fields, `encode` getter, `decode` factory). Precedent for `OutputMeta`'s 4-field shape.
- `bindings/action_system.json` — `Spends`, `FinishedSpends`, `PendingSpend`, `Action`, `Deltas`, `Outputs` declarations. The new `SendDestination` class entry slots in here per D-04.
- `bindings/puzzles.json` — large file with many `"type": "static"` precedents (lines 612, 625, 675, 774, 861, 868).

### Rust surface being exposed (read for type contract)
- `crates/chia-sdk-utils/src/silent_payments/address.rs` — `SilentPaymentAddress`, `SilentPaymentNetwork`. The address class shape.
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` — `SilentPaymentKeys` (private fields, getters, `from_mnemonic`/`from_secret_keys`/`unlabeled_address`/`labeled_address`). Constructor-shape gray area noted under Claude's Discretion.
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `LabelRegistry` (`new`, `register`, `lookup`, `get`). API-surface gray area noted under Claude's Discretion.
- `crates/chia-sdk-driver/src/silent_payments/types.rs` — `TweakData`, `OutputMeta`, `DetectedSpCoin`. Public fields directly (simple data structs, not opaque handles).
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — `pub fn scan_from_tweaks(scan_sk, spend_sk, spend_pk, data, labels, k_max) -> Vec<DetectedSpCoin>`. Static under D-02.
- `crates/chia-sdk-driver/src/silent_payments/one_time.rs` — `pub fn derive_one_time_puzzle_hash(...) -> Bytes32`. Static under D-02.
- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` — `pub fn compute_input_hash(...) -> ScalarField`. Static under D-02; return value needs the D-03 ScalarField class.
- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` — `pub fn aggregate_sender_sks(...) -> ScalarField`. Same as above.
- `crates/chia-sdk-types/src/silent_payments/scalar.rs` — `ScalarField` with `from_bytes_raw`/`from_bytes_unsigned`/`as_bytes`/`add`/`mul`. The class being exposed under D-03.
- `crates/chia-sdk-driver/src/action_system/send_destination.rs` — `SendDestination` enum (post-04.2). The class added to `action_system.json` per D-04.

### Existing chia-sdk-bindings facade patterns (precedents to mirror)
- `crates/chia-sdk-bindings/src/lib.rs` — module list + `pub use` re-export pattern + the `chip-0057`-unconditional re-export style for `chia_sdk_driver` types (lines 77-84).
- `crates/chia-sdk-bindings/src/address.rs` — minimal 4-line facade pattern for a simple data type. `SilentPaymentAddress` likely follows this.
- `crates/chia-sdk-bindings/src/mnemonic.rs` — `bindy::Result`-returning method pattern. All new SP facade methods follow.
- `crates/chia-sdk-bindings/src/key_pairs.rs` — multi-key-pair facade pattern (`KeyPair`, `BlsPair`, etc.). Reference for how `SilentPaymentKeys` might shape if planner picks the directory layout.
- `crates/chia-sdk-bindings/Cargo.toml` — current `chia-sdk-driver = { workspace = true, features = ["offer-compression", "action-layer"] }` declaration. Per D-01, `"chip-0057"` is added to that list (and analogous lists for chia-sdk-utils + chia-sdk-types deps).

### Per-target binding crate scaffolding
- `napi/src/lib.rs` — `bindy_macro::bindy_napi!("bindings.json")` invocation + hand-written `#[napi] impl Clvm` shims for features bindy can't express.
- `pyo3/src/lib.rs` — `bindy_macro::bindy_pyo3!()` equivalent (read to confirm same shape).
- `wasm/src/lib.rs` — `bindy_macro::bindy_wasm!()` equivalent.
- `napi/__test__/action_system.spec.ts`, `mips_memos.spec.ts` — AVA test precedents for the SP round-trip test (SC2).
- `pyo3/tests/test_pyo3.py` — single existing pytest. Planner decides whether to add a smoke entry here.
- `wasm/__test__/wasm.spec.ts` — single existing wasm AVA test. Same call-vs-Claude-discretion as above.

### Project-level constraints (apply to all decisions)
- `.planning/PROJECT.md` "Constraints" + "Out of Scope" — Rust 1.90.0 edition 2024, `unsafe_code = "deny"`, no new workspace deps, `chip-0057` is the only feature umbrella. D-01 honors this by avoiding a new feature on chia-sdk-bindings.
- `.planning/REQUIREMENTS.md` — BIND-01, BIND-02 are the active requirements satisfied by Phase 5. BIND-03 stays Active for Phase 6.
- `CLAUDE.md` (project-level) — workspace-deny clippy + warn pedantic, `cargo machete` in CI, LSP-over-grep, run clippy before reporting done. Build target also includes the napi/pyo3/wasm crates that are excluded from `cargo test` per CI's exclude list.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`Mnemonic` class** (`chia-sdk-bindings::Mnemonic`, declared in `bindings/mnemonic.json`) — already exposed; `SilentPaymentKeys::from_mnemonic(&Mnemonic)` plugs into this directly. No new mnemonic surface needed.
- **`SecretKey`, `PublicKey`, `Signature`** (`bindings/bls.json` with `remote: true`) — already exposed via the chia-bls crate; `SilentPaymentKeys` getters that return `&SecretKey`/`&PublicKey` return these directly. No type conversion at the facade boundary.
- **`Address` 4-field class precedent** (`bindings/address.json`, `crates/chia-sdk-bindings/src/address.rs` — 22 lines total) — direct precedent for `OutputMeta` (4 plain fields) and for the simpler shape `SilentPaymentAddress` might take if planner picks the field-based class form.
- **`"type": "static"` method pattern** (15+ precedents across `puzzles.json`, `mnemonic.json`, `bls.json`, `simulator.json`, etc.) — exactly the mechanism D-02 uses. No bindy macro changes required.
- **Bindy `Enum { values: [...] }` for unit-variant enums** — direct support for `SilentPaymentNetwork::{Mainnet, Testnet}` if planner picks that shape.
- **chia-sdk-bindings `pub use chia_sdk_driver::{...}` re-export pattern** (`lib.rs:77-84`) — already lists `Cat`, `CatInfo`, `OptionInfo`, `VaultInfo`, etc. without any feature gate. The new SP types (`TweakData`, `OutputMeta`, `DetectedSpCoin`, `SilentPaymentAddress`, `SilentPaymentKeys`, `LabelRegistry`, `ScalarField`, `SendDestination`) re-export here under D-01's unconditional chip-0057 enablement.

### Established Patterns
- **`bindy::Result` return on all facade methods** — every facade method returns `bindy::Result<T>`. New SP facade methods follow.
- **`#[derive(Clone)]` on facade types** — required for bindy to copy values across the FFI boundary. Applies to `SilentPaymentAddress`, `SilentPaymentKeys` (Clone-derive already on the Rust type), `TweakData`, `OutputMeta`, `DetectedSpCoin`, `LabelRegistry`, `ScalarField`.
- **camelCase generation in TS** — bindy auto-converts `scan_from_tweaks` → `scanFromTweaks` for napi/wasm output. SC1's TS names are bindy's output, not hand-written.
- **snake_case generation in Python** — bindy emits `scan_from_tweaks` as-is for pyo3.
- **Privacy-warning rustdoc on every memo-bearing public surface** (Phase 4 SEND-08; verified by `grep -L 'Privacy warning'` gate) — extends to any new binding facade method that takes or produces memos.

### Integration Points
- **`crates/chia-sdk-bindings/Cargo.toml` `[dependencies]` block** — D-01 modifies the `chia-sdk-driver`, `chia-sdk-utils`, `chia-sdk-types` declarations to add `"chip-0057"` to their `features = [...]` lists.
- **`crates/chia-sdk-bindings/src/lib.rs` module list + re-exports** — new `mod silent_payments;` + `pub use silent_payments::*;` (lines 11-52 area); new `pub use chia_sdk_driver::{TweakData, OutputMeta, DetectedSpCoin, SendDestination, ...}` and `pub use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentKeys, SilentPaymentNetwork, LabelRegistry}` and `pub use chia_sdk_types::silent_payments::ScalarField`.
- **New file:** `crates/chia-sdk-bindings/src/silent_payments.rs` (or directory) hosts the facade types/methods. Planner picks file vs directory layout.
- **New file:** `bindings/silent_payments.json` declares the SP descriptor: `ScalarField`, `SilentPaymentNetwork`, `SilentPaymentAddress`, `SilentPaymentKeys`, `LabelRegistry`, `TweakData`, `OutputMeta`, `DetectedSpCoin`, `SilentPayments` (zero-field namespace with 4 statics).
- **Existing file edited:** `bindings/action_system.json` gains the `SendDestination` opaque-handle class entry per D-04. The `Action.send` method's `destination` arg type changes from `Bytes32` to `SendDestination` (the Rust shape post-04.2 takes `impl Into<SendDestination>`, but the binding descriptor needs the concrete type since `impl Into` isn't expressible).
- **napi/pyo3/wasm `src/lib.rs`** — the existing `bindy_napi!()`/`bindy_pyo3!()`/`bindy_wasm!()` macro invocations automatically pick up `silent_payments.json` and the `action_system.json` patch. No hand-written shims unless bindy fails on a specific type (planner verifies during research).
- **`napi/__test__/silent_payments.ts`** (new file) — AVA test for SC2 address round-trip.

</code_context>

<specifics>
## Specific Ideas

- **The bindy schema is verified, not assumed.** During this discussion I confirmed (`bindy-macro/src/lib.rs:37-59`) that bindy supports `Class | Enum { values: Vec<String> } | Function` declarations and that `"type": "static"` methods on a zero-field class work (precedents at `mnemonic.json:24`, `bls.json:79`, 15+ uses in `puzzles.json`). SC4's pre-flight is **already satisfied** for D-02; no fallback strategy is needed. Planner doesn't have to re-verify.
- **The SendDestination JSON entry is small** — about 12 lines, sketched in `04.2-CONTEXT.md` <specifics>:
  ```json
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
  Planner copies this into `bindings/action_system.json` and writes the corresponding facade impl in `crates/chia-sdk-bindings/src/action_system.rs` (or a new `send_destination.rs`).
- **SilentPayments namespace class JSON sketch** (per D-02):
  ```json
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
      "derive_one_time_puzzle_hash": {
        "type": "static",
        "args": {
          "scan_pk": "PublicKey",
          "spend_pk": "PublicKey",
          "aggregated_sender_sk": "ScalarField",
          "input_hash": "ScalarField",
          "k": "u32"
        },
        "return": "Bytes32"
      },
      "compute_input_hash": {
        "type": "static",
        "args": {
          "coin_ids": "Vec<Bytes32>",
          "aggregated_sender_pk": "PublicKey"
        },
        "return": "ScalarField"
      },
      "aggregate_sender_sks": {
        "type": "static",
        "args": { "sks": "Vec<SecretKey>" },
        "return": "ScalarField"
      }
    }
  }
  ```
- **ScalarField JSON sketch** (per D-03):
  ```json
  "ScalarField": {
    "type": "class",
    "methods": {
      "from_bytes": {
        "type": "factory",
        "args": { "bytes": "Bytes32" }
      },
      "to_bytes": { "return": "Bytes32" }
    }
  }
  ```
- **AVA round-trip test sketch** (planner finalizes from TV1 fixture in `crates/chia-sdk-utils/src/silent_payments/keys.rs::tests`):
  ```typescript
  import test from 'ava';
  import { Mnemonic, SilentPaymentKeys, SilentPaymentNetwork, SilentPaymentAddress } from '../index.js';

  test('silent-payment address round-trip (TV1 mainnet)', t => {
    const mnemonic = new Mnemonic("<TV1 12-word mnemonic from keys.rs::tests>");
    const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
    const address = keys.unlabeledAddress(SilentPaymentNetwork.Mainnet);
    const encoded = address.encode();
    const decoded = SilentPaymentAddress.decode(encoded);
    t.deepEqual(decoded.scanPk.toBytes(), keys.scanPk().toBytes());
    t.deepEqual(decoded.spendPk.toBytes(), keys.spendPk().toBytes());
  });
  ```
  Asserts on `Uint8Array` bytes (from `PublicKey.toBytes()`), not on the encoded string — survives any future bech32m library churn.

</specifics>

<deferred>
## Deferred Ideas

- **pyo3 + wasm SP round-trip tests** — Phase 6's concern (BIND-03 requires the full address-gen + send + scan round trip from each language). Phase 5 only requires AVA + clean builds for pyo3/wasm. A minimal pyo3/wasm smoke test (import-and-call, no SP logic) is noted under Claude's Discretion if the planner wants build-time signal earlier than Phase 6.
- **`chia-sdk-test::silent_payments::tweak_data_from_simulator_block` exposure to bindings** — Phase 6 SC1 + SIM-01. Phase 5's `TweakData` exposure is descriptor-only; the simulator-side helper that constructs `TweakData` from real blocks is Phase 6's deliverable.
- **`examples/silent_payment.rs`** — Phase 6 EX-01.
- **Sage-side adapter for SP destinations** — Sage is a separate repo. The post-04.2 `From<Bytes32>` impl means Sage's existing `Action::send` callers keep working without changes; adding SP is a Sage-timeline concern after Phase 5 ships bindings.
- **CAT2 / NFT silent-payment send-side bindings** — already deferred to v2 per PROJECT.md "Out of Scope".
- **Async/streaming `TweakData` consumer trait** — `SilentPaymentTweakSource` async trait deferred per PROJECT.md "Out of Scope"; binding it is even further out.

</deferred>

---

*Phase: 05-bindings-rust-facade-json-descriptor*
*Context gathered: 2026-05-17*
