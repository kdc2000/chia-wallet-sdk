---
phase: 02-address-key-types
plan: 04
subsystem: infra
tags: [chip-0057, silent-payments, key-derivation, bip39, labels, label-registry, chia-sdk-utils, test-vectors, chia-bls]

# Dependency graph
requires:
  - phase: 02-address-key-types
    provides: "Plan 02-02: SilentPaymentNetwork enum (Mainnet=spxch, Testnet=tspxch) + SilentPaymentError six-variant enum (#[from] Bech32Error); silent_payments/mod.rs barrel with `mod address; pub use address::*; mod error; pub use error::*;` in sorted order"
  - phase: 02-address-key-types
    provides: "Plan 02-03: SilentPaymentAddress { scan_pk, spend_pk, network } public struct + SilentPaymentAddress::new(scan_pk, spend_pk, network) trusted constructor + bech32m encode/decode (96-byte payload) — consumed by SilentPaymentKeys::{unlabeled_address, labeled_address} as the return type"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chia_sdk_types::silent_payments::{SCAN_PATH, SPEND_PATH, CHIA_SP_LABEL, ScalarField, tagged_hash} (Phase 1) + workspace chip-0057 feature flag"
provides:
  - "chia_sdk_utils::silent_payments::SilentPaymentKeys struct with private fields (scan_sk, spend_sk, scan_pk, spend_pk), #[derive(Clone)] + manual redacting Debug, 7 public methods"
  - "SilentPaymentKeys::from_mnemonic(&Mnemonic) -> Self — empty-passphrase BIP-39 seed -> SecretKey::from_seed -> derive_path for SCAN_PATH (m/12381/8444/12/0) and SPEND_PATH (m/12381/8444/13/0)"
  - "SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk) -> Self — watch-only / key-import construction, caches both pubkeys"
  - "SilentPaymentKeys::{scan_sk, spend_sk, scan_pk, spend_pk}(&self) accessor methods returning &refs"
  - "SilentPaymentKeys::unlabeled_address(network) -> SilentPaymentAddress"
  - "SilentPaymentKeys::labeled_address(network, m) -> Result<SilentPaymentAddress, SilentPaymentError> — m=0 returns Err(ReservedChangeLabel); otherwise B_m = &spend_pk + &label_pk"
  - "chia_sdk_utils::silent_payments::LabelRegistry struct (forward: HashMap<u32, PublicKey>, reverse: HashMap<[u8; 48], u32>) with 6 methods: new/register/forward/lookup/len/is_empty/iter"
  - "chia_sdk_utils::silent_payments::labels::generate_label(scan_sk, m) -> (ScalarField, PublicKey) — pub(super), shared helper consumed by SilentPaymentKeys::labeled_address and LabelRegistry::register"
  - "chia-sdk-utils/Cargo.toml: chip-0057 now cascades to chia-sdk-types/chip-0057 (fixes Rule 3 blocker: -p chia-sdk-utils -F chip-0057 builds previously could not resolve chia_sdk_types::silent_payments because the optional dep was pulled in but its own chip-0057 feature was not activated)"
  - "silent_payments/mod.rs barrel: `mod keys; pub use keys::*; mod labels; pub use labels::*;` appended after `pub use error::*;` (sorted order address < error < keys < labels)"
  - "15 named #[test] functions: 7 in silent_payments::keys::tests (rows 43-46, 67, 68, 69) + 8 in silent_payments::labels::tests (rows 57-66 minus the two address-row entries owned by Plan 02-03's address.rs)"
affects: [02-05-prelude-and-gate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Manual Debug redaction for secret material: SilentPaymentKeys uses an explicit `impl core::fmt::Debug` that prints the public keys as `chia_bls::PublicKey`'s normal Debug but replaces the two secret-key fields with the string literal `\"<redacted>\"`. Auto-deriving Debug would call through to `chia_bls::SecretKey::Debug` which leaks the hex bytes — a privacy regression that pedantic clippy would not catch. Pattern matches what `chia_sdk_bindings::SecretKey` itself does NOT do, so this is the wallet-author-facing safety net."
    - "Stable bidirectional registry via two HashMaps: `LabelRegistry { forward: HashMap<u32, PublicKey>, reverse: HashMap<[u8; 48], u32> }` — uses `label_pk.to_bytes()` as the reverse-map key because chia_bls::PublicKey itself does not implement Hash. Two `HashMap` entries per registered label (forward + reverse). For the realistic upper bound of a few hundred labels per wallet the memory is well under 100 KB."
    - "Same-function generate_label consumed by two callers: keys.rs's labeled_address and labels.rs's LabelRegistry::register both call `generate_label(scan_sk, m)`. The helper is `pub(super)` so it is private to the silent_payments module tree but reachable from both files. This avoids drift: a future change to the label-scalar construction (e.g., a CHIP-0057 spec amendment) lands in one place and both callers update atomically."
    - "Pin-via-grep for the .expect message and test names: each of the 15 test names is grep-checked verbatim by the plan's acceptance criteria, and the `.expect(\"ScalarField::from_bytes_unsigned guarantees value < r\")` string is locked too. This means a refactor that renames or paraphrases breaks the acceptance grep before any test runs. Future Phase 2 plans (02-05) and Phase 3 onward inherit the same discipline."
    - "Empty-string BIP-39 passphrase pinning: SilentPaymentKeys::from_mnemonic calls `mnemonic.to_seed(\"\")` (empty passphrase). The TV1 mnemonic `\"abandon abandon abandon ... about\"` is the canonical 12-word BIP-39 test vector, and the four `from_mnemonic_tv1_*_matches` tests are the structural pin: any future drift in passphrase handling (e.g., accidentally feeding a non-empty string) breaks them immediately. Matches `chia_sdk_test::BlsPair::new` and `chia_sdk_bindings::Mnemonic` conventions."

key-files:
  created:
    - "crates/chia-sdk-utils/src/silent_payments/labels.rs (228 lines, generate_label helper + LabelRegistry struct + 8 named tests covering TV3 label math, labels_preserve_scan_pk cross-file invariant, and four LabelRegistry round-trip patterns)"
    - "crates/chia-sdk-utils/src/silent_payments/keys.rs (250 lines, SilentPaymentKeys struct + 7 public methods + manual redacting Debug + derive_path helper + 7 named tests covering TV1 from_mnemonic round-trip + from_secret_keys parity + m=0 rejection)"
  modified:
    - "crates/chia-sdk-utils/src/silent_payments/mod.rs (32 -> 36 lines: appended `mod keys; pub use keys::*; mod labels; pub use labels::*;` after the existing `pub use error::*;` line in sorted order)"
    - "crates/chia-sdk-utils/Cargo.toml (1 line changed: chip-0057 feature now propagates to chia-sdk-types/chip-0057 in addition to dep:chia-sdk-types — fixes Rule 3 blocker discovered at Task 3 verification)"

key-decisions:
  - "Drop `.clone()` on PublicKey throughout — chia-bls 0.36.1 implements `Copy` for PublicKey. The plan's `<interfaces>` block declared `pub struct PublicKey(/* private */);` without `Copy`, but empirically PublicKey IS Copy in 0.36.1. Clippy's `clippy::clone_on_copy` fired on three sites in keys.rs (`unlabeled_address`, `labeled_address`, the dropped-during-fix struct init) and two sites in labels.rs tests. All five were structurally fixed by removing `.clone()` (and using `*` to dereference the &PublicKey returned by `LabelRegistry::forward` in the two tests). No `#[allow(...)]` introduced."
  - "Inline public_key() calls into struct init to dodge `clippy::similar_names`: `from_secret_keys(scan_sk: SecretKey, spend_sk: SecretKey)` originally bound `let scan_pk = scan_sk.public_key(); let spend_pk = spend_sk.public_key();` then assembled the struct. Pedantic clippy flagged `scan_pk` and `scan_sk` (likewise spend_*) as too-similar binding names. The signature is locked by acceptance grep so renaming the parameters is not an option; instead, the `let` bindings are eliminated by inlining `scan_pk: scan_sk.public_key(),` and `spend_pk: spend_sk.public_key(),` directly into the struct literal. No `#[allow(...)]` introduced."
  - "chip-0057 cascade fix in chia-sdk-utils/Cargo.toml: original feature line `chip-0057 = [\"dep:chia-sdk-types\", \"dep:bip39\", \"dep:chia-bls\"]` pulled in the optional chia-sdk-types dep but did NOT activate its own chip-0057 feature. So `cargo build -p chia-sdk-utils -F chip-0057` had chia_sdk_types in the linker but the silent_payments module configured out — keys.rs and labels.rs failed E0432 on `use chia_sdk_types::silent_payments::*`. Fix: add `\"chia-sdk-types/chip-0057\"` to the feature list. This is a Rule 3 inline cleanup (the missing cascade prevented the plan from compiling at all). The workspace-level chip-0057 in the root Cargo.toml already cascades to both crates separately, so the root path was unaffected — only the targeted per-crate build path was broken."
  - "labels_preserve_scan_pk lives in labels.rs but reaches into keys.rs via `super::super::{SilentPaymentKeys, SilentPaymentNetwork}`: 02-VALIDATION.md row 62 pins this test name in `silent_payments::labels::tests::`, even though it constructs a `SilentPaymentKeys` from keys.rs. The double-`super::super::` reach is intentional and works because the test is `#[cfg(test)] mod tests` inside `silent_payments::labels`, and `SilentPaymentKeys` is re-exported by `silent_payments/mod.rs` via `pub use keys::*;`. Pattern matches the plan's locked test name."
  - "7 keys tests, not 9: the plan's must_haves bullet listed `9 keys tests` but its `<behavior>` block clarified the actual locked count is 7 (rows 43-46 + 67 + 68 + 69). The two `from_mnemonic_tv3_*_pinned` end-to-end tests that the must_haves bullet mentioned are NOT in 02-VALIDATION.md — they were optional. To honor the validation lock, only the 7 locked tests are shipped; the suggested-but-not-locked tv3 round-trip is left to Phase 3 (when scan_from_tweaks closes the receive end of the labeled-address loop)."

patterns-established:
  - "Cross-file test invariant via super::super reach: a test in `silent_payments::labels::tests::` constructs `silent_payments::SilentPaymentKeys` via `super::super::SilentPaymentKeys`. Demonstrates that Phase 2's same-module-tree split (`address.rs`, `error.rs`, `keys.rs`, `labels.rs`) supports cross-file tests without splitting into a top-level `silent_payments/tests/` directory. Phase 3 will reuse this pattern when scan-side tests in a new `scanner.rs` need to construct `SilentPaymentKeys` from this plan."
  - "Per-crate feature cascade gotcha — for an optional dep that itself has feature flags, the parent crate's feature must enable BOTH the optional dep AND the dep's feature: `chip-0057 = [\"dep:chia-sdk-types\", \"chia-sdk-types/chip-0057\", \"dep:bip39\", \"dep:chia-bls\"]`. The workspace-level chip-0057 ALREADY listed both crates separately and was unaffected, but per-crate `-p ... -F ...` builds in CI (and in plan-level gate matrices like this one's) exercise the per-crate feature shape. This pattern lands here for the first time in the silent_payments work — Plan 02-05 inherits."

requirements-completed: [ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06]

# Metrics
duration: 12min
completed: 2026-05-15
---

# Phase 02 Plan 04: SilentPaymentKeys + LabelRegistry + 15 named tests Summary

**SilentPaymentKeys (BIP-39 -> scan/spend SKs at m/12381/8444/{12,13}/0 + watch-only construction + manual redacting Debug + 4 accessors + unlabeled/labeled address builders with m=0 reject) and LabelRegistry (bidirectional u32 <-> PublicKey via two HashMaps) — closes ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06 in one plan with 15 locked tests passing on first run after one Rule 3 fix to the chip-0057 feature cascade.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-15T19:57:33Z
- **Completed:** 2026-05-15T20:09:25Z
- **Tasks:** 3 (2 file-creation + 1 barrel-wire + cascade fix)
- **Files modified:** 4 (2 new + 2 edited)

## Accomplishments

- `crates/chia-sdk-utils/src/silent_payments/labels.rs` (228 lines, NEW) — `pub(super) fn generate_label(scan_sk, m) -> (ScalarField, PublicKey)` computing `ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_LABEL, ser256(b_scan) || ser32(m)))` then `SecretKey::from_bytes(...).public_key()` with the locked `.expect("ScalarField::from_bytes_unsigned guarantees value < r")` message. `LabelRegistry { forward: HashMap<u32, PublicKey>, reverse: HashMap<[u8; 48], u32> }` with `new`, `register`, `forward`, `lookup`, `len`, `is_empty`, `iter` methods. 8 named tests: `tv3_label_scalar_matches`, `tv3_label_pk_matches`, `tv3_labeled_spend_pk_matches`, `labels_preserve_scan_pk` (cross-file via `super::super::{SilentPaymentKeys, SilentPaymentNetwork}`), `registry_round_trip`, `registry_three_labels`, `registry_lookup_missing`, `registry_forward_missing`.
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` (250 lines, NEW) — `SilentPaymentKeys { scan_sk, spend_sk, scan_pk, spend_pk }` private fields with `#[derive(Clone)]` only (no auto-Debug; manual Debug redacts SKs as `"<redacted>"`). `from_mnemonic(&Mnemonic)` uses empty-passphrase BIP-39 seed -> `SecretKey::from_seed` -> `derive_path` for `SCAN_PATH` (`m/12381/8444/12/0`) and `SPEND_PATH` (`m/12381/8444/13/0`). `from_secret_keys(scan_sk, spend_sk)` constructs from raw keys, caching both pubkeys; struct init inlines `.public_key()` calls to dodge `clippy::similar_names`. `unlabeled_address(network)` and `labeled_address(network, m)` return `SilentPaymentAddress` / `Result<SilentPaymentAddress, SilentPaymentError>`; `labeled_address(network, 0)` returns `Err(ReservedChangeLabel)`. 7 named tests: 4 TV1 `from_mnemonic_*_matches` (scan_sk, spend_sk, scan_pk, spend_pk), `from_secret_keys_matches_from_mnemonic` (parity check), `from_secret_keys_tv1_mainnet_pinned` (TV1 mainnet address string), `labeled_address_zero_rejected` (m=0 -> Err).
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (32 -> 36 lines) — appended `mod keys; pub use keys::*; mod labels; pub use labels::*;` after the existing `pub use error::*;` line. Module declarations now in sorted order: `address < error < keys < labels`.
- `crates/chia-sdk-utils/Cargo.toml` (1 line changed) — chip-0057 now propagates to `chia-sdk-types/chip-0057` in addition to `dep:chia-sdk-types`. Without this, `-p chia-sdk-utils -F chip-0057` builds previously could not resolve `chia_sdk_types::silent_payments::*` because the optional dep was pulled in but its own chip-0057 feature was not activated. Rule 3 inline cleanup.
- All 15 named tests pass under `--exact`:
  - **keys.rs (7):** `from_mnemonic_tv1_scan_sk_matches`, `from_mnemonic_tv1_spend_sk_matches`, `from_mnemonic_tv1_scan_pk_matches`, `from_mnemonic_tv1_spend_pk_matches`, `from_secret_keys_matches_from_mnemonic`, `from_secret_keys_tv1_mainnet_pinned`, `labeled_address_zero_rejected`.
  - **labels.rs (8):** `tv3_label_scalar_matches`, `tv3_label_pk_matches`, `tv3_labeled_spend_pk_matches`, `labels_preserve_scan_pk`, `registry_round_trip`, `registry_three_labels`, `registry_lookup_missing`, `registry_forward_missing`.
- Full `silent_payments` test suite: 27 passed; 0 failed (combines Plan 02-03's 12 address tests + this plan's 7 keys + 8 labels).
- All plan-level gates green: `cargo build --release -p chia-sdk-utils` (no features), `-F chip-0057`, `--all-features`; `cargo build --release --workspace` (no features) and `--all-features`; `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean; `cargo fmt --all --files-with-diff --check` clean; `cargo machete` clean; all three grep bans hold (`mod_by_group_order`, `^use sha2::`, `#[allow(` in keys.rs/labels.rs).

## Task Commits

Each task was committed atomically:

1. **Task 1: Add silent_payments/labels.rs with generate_label + LabelRegistry + 8 tests** — `f6826d1c` (feat)
2. **Task 2: Add silent_payments/keys.rs with SilentPaymentKeys + 7 tests** — `46f79c0f` (feat)
3. **Task 3: Wire keys.rs + labels.rs into mod.rs barrel + cascade chip-0057 to chia-sdk-types + clippy fixes** — `4a92ccba` (feat)

**Plan metadata commit:** to follow this SUMMARY.

## Files Created/Modified

- `crates/chia-sdk-utils/src/silent_payments/labels.rs` (NEW, 228 lines):
  - Lines 1-15: module doc-comment (CHIP-0057 §125-§130 label math + m=0 sentinel policy).
  - Lines 17-21: `use` block (`std::collections::HashMap`, `chia_bls::{PublicKey, SecretKey}`, `chia_sdk_types::silent_payments::{CHIA_SP_LABEL, ScalarField, tagged_hash}`).
  - Lines 23-43: `pub(super) fn generate_label` (12 lines impl + 8 lines doc).
  - Lines 45-108: `pub struct LabelRegistry` + `impl LabelRegistry` (new, register, forward, lookup, len, is_empty, iter).
  - Lines 110-228: `#[cfg(test)] mod tests` with 5 hex constants + 2 helper fns + 8 `#[test]` fns.
- `crates/chia-sdk-utils/src/silent_payments/keys.rs` (NEW, 250 lines):
  - Lines 1-3: module doc.
  - Lines 5-11: `use` block (`bip39::Mnemonic`, `chia_bls::{DerivableKey, PublicKey, SecretKey}`, `chia_sdk_types::silent_payments::{SCAN_PATH, SPEND_PATH}`, `super::{SilentPaymentAddress, SilentPaymentError, SilentPaymentNetwork, labels::generate_label}`).
  - Lines 13-35: `pub struct SilentPaymentKeys` with `#[derive(Clone)]` (no auto-Debug).
  - Lines 37-46: manual `impl core::fmt::Debug for SilentPaymentKeys` redacting SKs as `"<redacted>"`.
  - Lines 48-138: `impl SilentPaymentKeys` (from_mnemonic, from_secret_keys, four accessors, unlabeled_address, labeled_address).
  - Lines 140-148: private `fn derive_path` helper.
  - Lines 150-250: `#[cfg(test)] mod tests` with 5 hex constants + 1 mnemonic const + 1 mainnet-address const + 2 helper fns + 7 `#[test]` fns.
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (modified, 32 -> 36 lines): four-line append after `pub use error::*;`.
- `crates/chia-sdk-utils/Cargo.toml` (modified, 1 line changed): `chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057", "dep:bip39", "dep:chia-bls"]`.

## Decisions Made

- **Drop `.clone()` on PublicKey** — chia-bls 0.36.1 implements `Copy` for `PublicKey`. The plan's `<interfaces>` block declared the type without `Copy`, but pedantic clippy fires `clippy::clone_on_copy` empirically. Fix is structural: remove `.clone()` (and use `*` to dereference `&PublicKey` returns from `LabelRegistry::forward` in two test sites). Applies to: `keys.rs::unlabeled_address` (2 sites), `keys.rs::labeled_address` (1 site), `labels.rs` test `registry_round_trip` (1 site), `labels.rs` test `registry_three_labels` (1 site). No `#[allow(...)]` introduced.
- **Inline `public_key()` calls into struct init** — Pedantic clippy's `clippy::similar_names` fired on `from_secret_keys` because the local bindings `scan_pk`/`spend_pk` are too similar to the parameters `scan_sk`/`spend_sk`. The parameter names are locked by acceptance grep (`pub fn from_secret_keys\(scan_sk: SecretKey, spend_sk: SecretKey\) -> Self`), so renaming is not an option. Fix: eliminate the `let` bindings by inlining `scan_pk: scan_sk.public_key(),` and `spend_pk: spend_sk.public_key(),` directly into the struct literal. The compiler's move-checker forces the field order to be pubkeys first (they consume the SKs by reference) and SKs last (they are moved into the struct after their public_key() is read).
- **chip-0057 feature cascade fix** — chia-sdk-utils's `chip-0057` originally had `["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]`. This pulls in chia-sdk-types but does NOT activate `chia-sdk-types/chip-0057`. The plan's gate matrix calls `cargo build -p chia-sdk-utils -F chip-0057` and `cargo test ... -F chip-0057`, both of which fail E0432 on the `use chia_sdk_types::silent_payments::*` lines in keys.rs and labels.rs. Added `"chia-sdk-types/chip-0057"` to the feature list. The workspace-level chip-0057 in the root Cargo.toml already cascades to both crates separately, so the workspace path was unaffected — only the per-crate `-p chia-sdk-utils -F chip-0057` path was broken. Rule 3 inline cleanup.
- **labels_preserve_scan_pk cross-file reach is intentional** — Test lives in `silent_payments::labels::tests::` per 02-VALIDATION.md row 62, but constructs a `SilentPaymentKeys` from keys.rs. Resolution path: `super::super::SilentPaymentKeys` reaches `silent_payments::SilentPaymentKeys` (re-exported by `pub use keys::*;` in mod.rs). Equivalent to `crate::silent_payments::SilentPaymentKeys` but the `super::super::` form is what the plan locked in the action block.
- **7 keys tests honored, 9 was overcounting** — Plan's must_haves bullet said "9 keys tests" but its `<behavior>` block clarified that 02-VALIDATION.md only locks 7 in `keys::tests::*` (rows 43-46, 67, 68, 69). Honored the validation lock; did not invent two more tests.
- **No `#[allow(...)]` attributes anywhere in silent_payments/** — All three clippy issues encountered (`clone_on_copy`, `similar_names`, `clippy::needless_borrow` did NOT fire here as it had been suggested might in the plan) were fixed structurally. Maintains the workspace-wide discipline established in Phase 1 (Pitfall 9) and Plan 02-03.

## Deviations from Plan

**1. [Rule 3 - Blocking issue] chip-0057 feature cascade missing in chia-sdk-utils/Cargo.toml**

- **Found during:** Task 3 — first `cargo build -p chia-sdk-utils -F chip-0057` after wiring mod.rs.
- **Issue:** Build failed with `error[E0432]: unresolved import 'chia_sdk_types::silent_payments'` in both keys.rs:6 and labels.rs:21. The compiler hint pointed to `lib.rs:4 pub mod silent_payments;` being `#[cfg(feature = "chip-0057")]` — chia-sdk-types was pulled in but its own chip-0057 feature was not activated.
- **Root cause:** `chia-sdk-utils/Cargo.toml` line `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]` pulled in the optional dep but did not cascade the feature.
- **Fix:** Added `"chia-sdk-types/chip-0057"` to the feature list. New line: `chip-0057 = ["dep:chia-sdk-types", "chia-sdk-types/chip-0057", "dep:bip39", "dep:chia-bls"]`.
- **Files modified:** `crates/chia-sdk-utils/Cargo.toml` (1 line).
- **Commit:** `4a92ccba` (rolled into Task 3 — the cascade fix is what unblocks the test suite).

**2. [Rule 1 - Bug] clippy::clone_on_copy on PublicKey**

- **Found during:** Task 3 — first `cargo clippy --features chip-0057 --all-targets -- -D warnings` after wiring mod.rs.
- **Issue:** 5 clippy errors total. Plan's `<interfaces>` block declared `PublicKey` without `Copy` so the action block used `.clone()` calls liberally; empirically chia-bls 0.36.1 implements `Copy` and pedantic clippy fires.
- **Sites:** keys.rs lines 109 (twice in `unlabeled_address`), 129 (`labeled_address`); labels.rs lines 190 and 205 (two test sites in `registry_round_trip` and `registry_three_labels`).
- **Fix:** Removed `.clone()` calls; replaced `reg.forward(m).expect("registered").clone()` with `*reg.forward(m).expect("registered")` (dereferencing the `&PublicKey` return — only valid because `PublicKey` is `Copy`).
- **Files modified:** `crates/chia-sdk-utils/src/silent_payments/keys.rs` (3 sites), `crates/chia-sdk-utils/src/silent_payments/labels.rs` (2 sites).
- **Commit:** `4a92ccba` (rolled into Task 3).

**3. [Rule 1 - Bug] clippy::similar_names on from_secret_keys**

- **Found during:** Task 3 — same clippy invocation that surfaced #2 above.
- **Issue:** Pedantic clippy fired `binding's name is too similar to existing binding` on the local `let scan_pk = scan_sk.public_key();` (parameter `scan_sk` vs binding `scan_pk`) and likewise for spend.
- **Constraint:** The parameter names `scan_sk` and `spend_sk` are locked by acceptance grep (`pub fn from_secret_keys\(scan_sk: SecretKey, spend_sk: SecretKey\) -> Self`).
- **Fix:** Eliminated the `let` bindings; inlined `scan_pk: scan_sk.public_key(),` and `spend_pk: spend_sk.public_key(),` directly into the struct literal. Field order in the struct literal is `scan_pk, spend_pk, scan_sk, spend_sk` (pubkeys first because they read the SKs by reference; SKs last because they are then moved into the struct).
- **Files modified:** `crates/chia-sdk-utils/src/silent_payments/keys.rs` (1 site).
- **Commit:** `4a92ccba` (rolled into Task 3).

All three deviations are Rule 1/3 inline cleanups that brought the plan to the green state the gate matrix demanded. None changed the plan's intent or the public API.

## Issues Encountered

The chip-0057 cascade discovery at Task 3 was the only friction. Everything else was clean — all 27 tests passed on the first invocation after the cascade fix landed, no test logic needed adjustment, and the bench-level cross-file invariant (`labels_preserve_scan_pk` reaching into `keys.rs`) worked on first run.

## User Setup Required

None — no external service or environment configuration required.

## Next Phase Readiness

**Plan 02-05 (final gate + prelude) is unblocked:**
- All four files in `silent_payments/` are in place: `address.rs`, `error.rs`, `keys.rs`, `labels.rs`. Plan 02-05 takes over the deferred prelude.rs re-export from Plan 02-01 and the final clippy / machete / CI sweep.
- The public surface to re-export at `src/prelude.rs` is: `SilentPaymentAddress`, `SilentPaymentNetwork`, `SilentPaymentError`, `SilentPaymentKeys`, `LabelRegistry`. All under `chia_sdk_utils::silent_payments::*`.
- Five requirements closed in this plan (ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06) plus ADDR-02 from Plan 02-03 = all six ADDR-* requirements for Phase 2 are now Done. Phase 2 closure depends only on Plan 02-05 (final gate + traceability sweep).

**Blockers/concerns:** None. The chip-0057 cascade fix in chia-sdk-utils/Cargo.toml means CI's `cargo build -p chia-sdk-utils -F chip-0057` line (added in Plan 01-05) now passes — Plan 02-05 will verify this in its final gate matrix.

## Self-Check

- [x] `crates/chia-sdk-utils/src/silent_payments/labels.rs` exists (228 lines) — `wc -l` confirms
- [x] `crates/chia-sdk-utils/src/silent_payments/keys.rs` exists (250 lines) — `wc -l` confirms
- [x] `crates/chia-sdk-utils/src/silent_payments/mod.rs` updated — 4 submodule declarations in sorted order: `address (line 29) < error (line 31) < keys (line 33) < labels (line 35)`
- [x] `crates/chia-sdk-utils/Cargo.toml` updated — `chia-sdk-types/chip-0057` added to chip-0057 cascade
- [x] Commit `f6826d1c` (Task 1, feat) exists — `git log --oneline -5` confirms
- [x] Commit `46f79c0f` (Task 2, feat) exists — `git log --oneline -5` confirms
- [x] Commit `4a92ccba` (Task 3, feat) exists — `git log --oneline -5` confirms
- [x] All 15 named tests pass under `--exact` invocations — each verified individually in the per-test loop
- [x] `cargo build --release -p chia-sdk-utils` (no features) green
- [x] `cargo build --release -p chia-sdk-utils -F chip-0057` green
- [x] `cargo build --release -p chia-sdk-utils --all-features` green
- [x] `cargo build --release --workspace` green
- [x] `cargo build --release --workspace --all-features` green
- [x] `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` runs 27 tests, 0 failures
- [x] `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean (0 errors, 0 warnings)
- [x] `cargo fmt --all -- --files-with-diff --check` clean
- [x] `cargo machete` reports no unused dependencies
- [x] `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` holds
- [x] `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` holds
- [x] `! grep -E '#\[allow\(' crates/chia-sdk-utils/src/silent_payments/keys.rs` holds (no clippy-bypass attributes in keys.rs)
- [x] `! grep -E '#\[allow\(' crates/chia-sdk-utils/src/silent_payments/labels.rs` holds (no clippy-bypass attributes in labels.rs)
- [x] `pub struct SilentPaymentKeys` exists with `#[derive(Clone)]` and manual `impl core::fmt::Debug` redacting `"<redacted>"`
- [x] `pub(super) fn generate_label` exists with locked `.expect("ScalarField::from_bytes_unsigned guarantees value < r")`
- [x] `pub struct LabelRegistry` derives `Clone, Debug, Default` and uses `HashMap` (not `BTreeMap`)
- [x] All 15 named test functions match `02-VALIDATION.md` rows 43-46, 57-69 (minus the two address-row entries owned by Plan 02-03's address.rs)
- [x] keys.rs line count = 250 (>= 250 min_lines)
- [x] labels.rs line count = 228 (>= 200 min_lines)

## Self-Check: PASSED

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
