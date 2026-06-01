---
phase: quick-260601-e6z
plan: 01
subsystem: silent-payments (CHIP-0057)
tags: [cleanup, editorial, ci, tests, chip-0057]
requires: []
provides:
  - "Planning-residue-free SP source (shipping + #[cfg(test)])"
  - "Corrected scalar.rs reduction narrative (quotient=2)"
  - "Terse one-line SP DriverError Display strings"
  - "SilentPaymentPending relocated to types.rs"
  - "CI-enforced SP descriptor/facade drift gate"
  - "Multi-input SP e2e test through the real receiver grouping path"
  - "v1 multi-input constraint documented in SP rustdoc"
affects:
  - crates/chia-sdk-types/src/silent_payments/
  - crates/chia-sdk-utils/src/silent_payments/
  - crates/chia-sdk-driver/src/silent_payments/
  - crates/chia-sdk-driver/src/action_system/
  - crates/chia-sdk-driver/src/actions/silent_payment_send.rs
  - crates/chia-sdk-driver/src/driver_error.rs
  - crates/chia-sdk-bindings/src/silent_payments.rs
  - .github/workflows/rust.yml
tech-stack:
  added: []
  patterns:
    - "DriverError Display strings: terse one-line lowercase, remediation in /// rustdoc"
    - "Crate-internal wire/state types live in silent_payments/types.rs"
key-files:
  created: []
  modified:
    - crates/chia-sdk-types/src/silent_payments/scalar.rs
    - crates/chia-sdk-types/src/silent_payments/mod.rs
    - crates/chia-sdk-types/src/silent_payments/paths.rs
    - crates/chia-sdk-utils/src/silent_payments/mod.rs
    - crates/chia-sdk-utils/src/silent_payments/keys.rs
    - crates/chia-sdk-utils/src/silent_payments/labels.rs
    - crates/chia-sdk-driver/src/silent_payments/mod.rs
    - crates/chia-sdk-driver/src/silent_payments/protocol.rs
    - crates/chia-sdk-driver/src/silent_payments/scanner.rs
    - crates/chia-sdk-driver/src/silent_payments/send_keys.rs
    - crates/chia-sdk-driver/src/silent_payments/types.rs
    - crates/chia-sdk-driver/src/action_system/spends.rs
    - crates/chia-sdk-driver/src/action_system/relation.rs
    - crates/chia-sdk-driver/src/actions/silent_payment_send.rs
    - crates/chia-sdk-driver/src/driver_error.rs
    - crates/chia-sdk-bindings/src/silent_payments.rs
    - crates/chia-sdk-driver/tests/silent_payments_e2e.rs
    - .github/workflows/rust.yml
decisions:
  - "Malformed CHIP §RECV-NN citations are requirement IDs, not spec sections — rewritten as plain prose"
  - "SP DriverError variant names/ordering/#[cfg]/#[from] unchanged; only Display strings + rustdoc touched"
  - "SilentPaymentPending move is pure (pub(crate), same fields); consumer paths resolve through mod root unchanged"
metrics:
  duration: ~25 min
  tasks: 3
  files: 18
  completed: 2026-06-01
---

# Phase quick-260601-e6z: CHIP-0057 Silent-Payments Code-Review Cleanup Summary

Editorial + test + CI hygiene pass over the CHIP-0057 silent-payments surface: stripped all GSD planning vocabulary from doc/inline/test comments, fixed the wrong scalar reduction narrative, deduped the signed-vs-unsigned warning to one canonical site, trimmed multi-line SP `DriverError` Display strings to the repo's terse convention, relocated the crate-internal `SilentPaymentPending` struct, wired the descriptor/facade drift guard into CI, and added a multi-input e2e test that drives the real receiver grouping path. Zero behavior change, zero new deps, zero new `#[allow]`, no unsafe.

## What Shipped

### Task 1 — Strip residue, fix scalar narrative, remove scaffolding, fix stale refs (commit `8d264bd4`)
- Removed `Phase N`, requirement IDs (`RECV-/SEND-/ADDR-/CRYPTO-/GUARD-/BIND-/BRIDGE-/POLISH-/CLEANUP-/SIM-` + digits), `Pitfall N`, `Plan NN-MM`, `D-NN`, `SC N`, and `ROADMAP`/`REQUIREMENTS.md` references across every SP file (shipping AND `#[cfg(test)]` inline comments, including box-drawing test section headers). Genuine CHIP §NNN / BIP-352 / BIP-340 citations preserved.
- Fixed two malformed `CHIP §RECV-NN` "citations" (RECV-NN are requirement IDs, not CHIP sections) by rewriting as plain prose.
- Fixed the incorrect `scalar.rs` reduction narrative: the quotient `floor((2^256-1)/r)` is **2** (not 1) and the reduced value is `(2^256-1) - 2r`. The pinned `expected: [u8;32]` bytes were already correct and left unchanged.
- Deduped the signed-vs-unsigned mod-r warning to one canonical statement in `chia-sdk-types/src/silent_payments/scalar.rs`; other sites now terse one-liners deferring to `ScalarField::from_bytes_unsigned`.
- Removed the `// (additional submodules appended in sorted order)` scaffolding comment from `chia-sdk-types/src/silent_payments/mod.rs`.
- Fixed the 3 stale `finish_with_silent_payment_keys` references (now `Spends::finish_with_keys` + the private `sp_finish_branch`) in `action_system/spends.rs` and `action_system/relation.rs`. `grep -rn finish_with_silent_payment_keys crates/chia-sdk-driver/src` returns 0.

### Task 2 — Trim SP DriverError messages + relocate SilentPaymentPending (commit `e910725b`)
- Trimmed all 7 multi-line SP `DriverError` `#[error(...)]` Display strings to terse one-line lowercase form matching the repo convention (`"silent payment key not synthetic"`, `"silent payment requires xch"`, etc.). All remediation prose — including the `StandardArgs::curry_tree_hash` / `from_raw` explanation that was embedded in `SilentPaymentKeyNotSynthetic` — moved into `///` rustdoc. No API names remain in any user-facing Display string. `SilentPayment(#[from] ...)` left as-is. Variant names, ordering, `#[cfg(feature = "chip-0057")]` gates, and `#[from]` unchanged.
- Relocated `pub(crate) struct SilentPaymentPending` (and its imports `Memos`, `NodePtr`) from `send_keys.rs` into `types.rs` alongside the other crate-internal wire/state types. `send_keys.rs` module doc re-pointed to describe the synthetic-key newtypes; its now-unused `Bytes32`/`Memos`/`NodePtr` imports removed.
- Simplified `silent_payments/mod.rs` re-exports: dropped `pub(crate) use send_keys::*;` (so `send_keys` contributes only `pub use {SyntheticPublicKey, SyntheticSecretKey}`), and added `pub(crate) use types::SilentPaymentPending;`. Consumer paths (`action_system/spends.rs`, `actions/silent_payment_send.rs`) resolve through the module root unchanged — pure move.

### Task 3 — Multi-input e2e test + constraint doc + follow-up note + CI drift gate (commit `3aa319b8`)
- Added `test_simulator_e2e_multi_input` to `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`: two distinct-puzzle-hash XCH inputs (`sim.bls(600)` ×2) sent to one unlabeled SP address with `Relation::AssertConcurrent`, signed with both senders' SKs, farmed, extracted via `tweak_data_from_simulator_block`, scanned via `scan_from_tweaks`. Asserts exactly one detection at `k=0`, `label: None`, `amount == 1000`, then spends the detected coin and confirms it is spent. The test PASSES on the existing production grouping path with no production-code changes (TDD: the assertions are meaningful and the real path satisfies them).
- Documented the v1 multi-input constraint in the driver SP module `//!` rustdoc: multi-input SP bundles must be distinct-puzzle-hash, non-ephemeral XCH inputs only (no CAT/DID/NFT/intermediate), so the receiver's same-`AssertConcurrent`-cycle group equals the sender's exact input set; otherwise `input_hash` diverges and the coin is undetectable.
- Added a one-line deferred follow-up code comment at the `AssertConcurrent` gate in `sp_finish_branch` referencing this SUMMARY.
- Wired `scripts/sp_descriptor_facade_drift.sh` into the `test` job of `.github/workflows/rust.yml` (after "Unused dependencies", before publish), as `run: bash scripts/sp_descriptor_facade_drift.sh`. No new install step (ubuntu-latest has jq + bash 4+).

## Deviations from Plan

None — the plan executed exactly as written. Two malformed `CHIP §RECV-NN` references (caught by the residue grep on `mod.rs` line 4 and `scanner.rs`) were not enumerated per-file in the plan but fall squarely under ITEM-1's "strip requirement IDs / keep genuine CHIP §NNN" rule; rewriting them as plain prose is the intended interpretation, not a deviation.

## Deferred Follow-up (recorded per plan ITEM-7)

**OPEN QUESTION (deferred out of this cleanup): filter the `AssertConcurrent` cycle to the SP XCH-input set vs. document the v1 constraint.** This is a privacy/design decision — it interacts with the on-chain-fingerprint no-SP-marker goal (FINGERPRINT-01). v1 ships the documented-constraint approach (distinct-puzzle-hash non-ephemeral XCH-only multi-input bundles); cycle-filtering is a candidate for a future design pass and was NOT changed here. The `AssertConcurrent` cycle-binding logic in `action_system/spends.rs` was left untouched. A one-line comment at the gate in `sp_finish_branch` points back to this SUMMARY.

## Verification (full phase gate, mirrors CI)

- ITEM-1 residue grep over the full SP source set (incl. `#[cfg(test)]`, excl. `.planning/`): **0 hits**.
- `grep -rn finish_with_silent_payment_keys crates/chia-sdk-driver/src`: **0**; scaffolding comment gone.
- `scalar.rs` narrative states quotient=2 / `(2^256-1)-2r`; pinned bytes unchanged.
- SP `DriverError` Display strings terse one-line lowercase, no embedded API names (`grep` for API names inside `#[error` lines: 0).
- `SilentPaymentPending` in `types.rs` (1), not in `send_keys.rs` (0); `pub(crate) use send_keys::*` removed.
- Per-crate builds clean WITH and WITHOUT `--all-features` (`chia-sdk-types`, `chia-sdk-utils`, `chia-sdk-driver`, and each `-F chip-0057`).
- Full CI workspace test suite green (0 failing test-result lines), including the new `test_simulator_e2e_multi_input`.
- `cargo clippy --workspace --all-features --all-targets`: no SP-related warnings. The only warnings are the pre-existing, documented `chia-sdk-daemon` `client.rs:426-427` pedantic warnings (out of scope; logged in Plan 01-05 deferred-items). Scoped `clippy -p {types,utils,driver} --all-features --all-targets -- -D warnings` is clean.
- `cargo fmt --all -- --files-with-diff --check`: clean.
- `cargo machete`: clean (no unused deps).
- `bash scripts/sp_descriptor_facade_drift.sh`: passes (23 methods on both sides).
- Exactly **one** `#[allow]` under `silent_payments/` (the pre-existing `scanner.rs:73` `clippy::similar_names`); zero new `#[allow]`, zero `unsafe`, zero new workspace deps.

## Known Stubs

None.

## Commits

- `8d264bd4` docs(quick-260601-e6z): strip planning residue, dedupe + fix scalar narrative
- `e910725b` refactor(quick-260601-e6z): trim SP DriverError messages + relocate SilentPaymentPending
- `3aa319b8` test(quick-260601-e6z): multi-input SP e2e + constraint doc + CI drift gate

## Self-Check: PASSED

- SUMMARY.md created and present on disk.
- `test_simulator_e2e_multi_input` present in `crates/chia-sdk-driver/tests/silent_payments_e2e.rs`.
- `SilentPaymentPending` present in `crates/chia-sdk-driver/src/silent_payments/types.rs`.
- All three task commits (`8d264bd4`, `e910725b`, `3aa319b8`) exist in git history.
