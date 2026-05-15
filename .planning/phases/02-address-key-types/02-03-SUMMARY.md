---
phase: 02-address-key-types
plan: 03
subsystem: infra
tags: [chip-0057, silent-payments, bech32m, address-encoding, chia-sdk-utils, test-vectors, chia-bls]

# Dependency graph
requires:
  - phase: 02-address-key-types
    provides: "Plan 02-02: SilentPaymentNetwork enum (Mainnet=spxch, Testnet=tspxch) + SilentPaymentError six-variant enum (#[from] Bech32Error) in chia_sdk_utils::silent_payments, plus address.rs stub already imports `super::SilentPaymentError` and exposes the network discriminant"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "Workspace-level chip-0057 feature flag; chia_sdk_utils::Bech32 wrapper (new/encode/decode) already enforces Variant::Bech32m and propagates bech32::Error through #[from] Bech32Error"
provides:
  - "chia_sdk_utils::silent_payments::SilentPaymentAddress { scan_pk: chia_bls::PublicKey, spend_pk: chia_bls::PublicKey, network: SilentPaymentNetwork } with #[derive(Clone, Debug, PartialEq, Eq)] (no Copy — PublicKey is not Copy)"
  - "SilentPaymentAddress::new(scan_pk, spend_pk, network) trusted constructor; #[must_use]"
  - "SilentPaymentAddress::encode(&self) -> Result<String, SilentPaymentError> — bech32m over 96-byte scan_pk||spend_pk payload, HRP from network.hrp()"
  - "SilentPaymentAddress::decode(s: &str) -> Result<Self, SilentPaymentError> — parses bech32m via Bech32::decode (variant enforcement reused), validates HRP via SilentPaymentNetwork::from_hrp, validates payload length == 96, parses both 48-byte PublicKey halves, rejects identity-element via is_inf()"
  - "12 named #[test] functions in silent_payments::address::tests locked verbatim to 02-VALIDATION.md rows 47-56 + 60-61: 4 TV1 positive (round-trip mainnet/testnet, encode-pinned mainnet/testnet), 2 TV3 positive (labeled encode-pinned mainnet/testnet), 6 negative (wrong HRP, invalid checksum, bech32-not-bech32m, short payload, identity scan_pk, identity spend_pk)"
affects: [02-04-keys-and-labels, 02-05-prelude-and-gate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Bech32m via existing wrapper, never direct: SilentPaymentAddress::encode/decode delegate to chia_sdk_utils::Bech32::{new, encode, decode} rather than importing the `bech32` crate. Pitfall 1 grep-checked at acceptance time (`! grep -E '^use bech32::' address.rs`). Reusing the wrapper inherits its bech32m-variant enforcement and the #[from] propagation of bech32::Error through Bech32Error → SilentPaymentError automatically."
    - "Identity-element rejection ordering: parse → is_inf() → reject, NOT relying on PublicKey::from_bytes to fail on infinity (RESEARCH §13 Pitfall 5). Empirically confirmed at Task 2 execution: chia-bls 0.36.1 PublicKey::from_bytes(&[0xc0, 0, 0, ..., 0]) succeeds and pk.is_inf() returns true. The defensive ordering catches it."
    - "Permissive rejection-variant matches!: identity-pubkey tests assert matches!(result, Err(IdentityPublicKey | InvalidPublicKey)) — both satisfy CHIP §215 (\"implementations MUST reject the point at infinity\"). The test name `decode_identity_*_rejected` covers both meanings, so the API contract is the same regardless of which internal variant chia-bls picks for the encoding. Future chia-bls versions can flip this without breaking the test suite."
    - "Test-vector pinning via doubled-string hex_literal!: 48-byte and 96-byte hex constants are split across two adjacent string literals (no concatenation operator) inside `hex!(...)`. Matches Phase 1's pattern in chia-sdk-types/src/silent_payments/tagged_hash.rs:44 and avoids hand-counting 96 nibbles on one line."
    - "Real-world negative test sourcing: decode_xch_hrp_rejected uses an actual xch1... address copied from crates/chia-sdk-utils/src/bech32.rs:126 (a tested string in the same workspace), and decode_bech32_not_bech32m_rejected uses the Bitcoin SegWit V0 address from bech32.rs:139. Both are proven bech32/bech32m strings — no hand-crafting risk that the negative test fails for the wrong reason (typo'd HRP, etc.)."

key-files:
  created: []
  modified:
    - "crates/chia-sdk-utils/src/silent_payments/address.rs (was 41 lines stub from Plan 02-02 — now 336 lines: existing SilentPaymentNetwork enum unchanged on lines 12-41, new SilentPaymentAddress struct + new/encode/decode on lines 47-126, new #[cfg(test)] mod tests with 12 #[test] functions + 3 hex constants + 4 address-string constants on lines 128-339)"

key-decisions:
  - "Permissive identity-element rejection match (RESEARCH §13 Pitfall 5): The two `decode_identity_*_rejected` tests use `matches!(result, Err(IdentityPublicKey | InvalidPublicKey))` rather than tightening to one variant after observing chia-bls 0.36.1's behavior. Rationale: both variants satisfy CHIP §215 rejection equally; the test asserts the boundary (rejection), not the internal labeling. If chia-bls 0.36.2 ever shifts which path fires for `[0xc0, 0, ...]`, this test does not break."
  - "Sources for negative test inputs (bech32 strings) are recycled from `crates/chia-sdk-utils/src/bech32.rs`'s own test suite (lines 126, 139): the `xch1...` address and the Bitcoin SegWit V0 address. This guarantees the strings are genuinely valid bech32m (for the HRP test) and genuinely valid-but-not-bech32m (for the variant test) — eliminates the risk of a typo making a negative test fail for the wrong reason."
  - "Imports consolidated at top of file (rustfmt fix): The initial Edit placed `use chia_bls::PublicKey; use chia_protocol::Bytes; use crate::Bech32;` immediately above the new struct (mid-file). rustfmt left them there, but that splits the file's use block. Consolidated to a single top-of-file use block: stdlib-style ordering keeps `chia_bls`, `chia_protocol`, `crate`, `super` together as one entry point. This is consistent with the rest of the silent_payments/ tree (error.rs and Plan 02-02's address.rs stub both have a single top-of-file use)."
  - "Clippy `unnested_or_patterns` fix is structural, not via `#[allow]`: Replaced `Err(A) | Err(B)` with `Err(A | B)` per the suggested help. Inside `matches!()`, both forms are semantically identical; the clippy-pedantic preference is the nested form. Maintains the workspace-wide \"no clippy-bypass attributes\" invariant established in Phase 1 (Pitfall 9)."
  - "Clippy `doc_markdown` fix on `encode()` doc-comment: Wrapped `scan_pk`, `spend_pk`, and `\"1\"` in backticks inside the bech32m formula notation. Pure cosmetic; no behavior change. Documents that those tokens are code identifiers / string literals, not free-form English."

patterns-established:
  - "Append-to-stub-file pattern with TDD ordering: Plan 02-02 shipped address.rs as a 41-line stub holding only SilentPaymentNetwork. Plan 02-03 (this) appended SilentPaymentAddress + 12 tests to the SAME file without renaming, without splitting into a separate file, and without touching the mod.rs barrel (the existing `pub use address::*;` already re-exports the new struct). Same file evolves through three plans (02-02 → 02-03 → potentially 02-05 for prelude wiring) without intermediate refactors. Reduces churn on the barrel and keeps the type group co-located."
  - "Test-first acceptance with verbatim-locked names: 02-VALIDATION.md pins 12 test names via `--exact` invocations in the per-task verification map. The plan's Task 2 then references those exact names. The summary's frontmatter `provides:` block also lists them. This three-way pinning (validation → plan → summary) means any future refactor that renames a test fails the gate matrix automatically. Phase 2's Plan 02-04 will repeat the pattern for the keys/labels test names."

requirements-completed: [ADDR-02]

# Metrics
duration: 8min
completed: 2026-05-15
---

# Phase 02 Plan 03: SilentPaymentAddress encode/decode + 14 address tests Summary

**SilentPaymentAddress struct + bech32m encode/decode (96-byte scan_pk||spend_pk payload, HRP spxch/tspxch) + 12 named tests pinning the CHIP-0057 TV1/TV3 test vectors and 6 negative-case rejections — ADDR-02 closed end-to-end with all tests passing on first run.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-15T19:45:56Z
- **Completed:** 2026-05-15T19:53:56Z
- **Tasks:** 2 (1 append production code + 1 append test module)
- **Files modified:** 1 (`address.rs` grew from 41 lines to 336 lines)

## Accomplishments

- `SilentPaymentAddress` struct (scan_pk, spend_pk, network) with `#[derive(Clone, Debug, PartialEq, Eq)]` appended to `crates/chia-sdk-utils/src/silent_payments/address.rs` after the existing `SilentPaymentNetwork` enum, sharing the same file per Plan 02-02's "stub-as-final-file-location" pattern.
- `SilentPaymentAddress::encode()` produces bech32m via `chia_sdk_utils::Bech32::new(...).encode()` — no direct `bech32::` imports. Payload is 96 bytes: `scan_pk.to_bytes() || spend_pk.to_bytes()`.
- `SilentPaymentAddress::decode(s)` parses bech32m via `Bech32::decode(s)` (which enforces `Variant::Bech32m` and propagates `bech32::Error` through `Bech32Error` via the existing `#[from]` chain), validates HRP via `SilentPaymentNetwork::from_hrp`, validates payload length == 96, parses both 48-byte `PublicKey` halves, and rejects identity-element pubkeys via `is_inf()` after successful parse (NOT relying on `from_bytes` failing on identity — RESEARCH §13 Pitfall 5).
- All 12 named tests pass on first run under `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::<name> -- --exact`:
  - **TV1 round-trip:** `tv1_mainnet_round_trip`, `tv1_testnet_round_trip` — decode then re-encode equals original.
  - **TV1 encode-pinned:** `tv1_mainnet_encode_pinned`, `tv1_testnet_encode_pinned` — from raw TV1 hex pubkeys, encode equals the recomputed Python-reference string.
  - **TV3 labeled encode-pinned:** `tv3_mainnet_labeled_pinned`, `tv3_testnet_labeled_pinned` — same scan_pk as TV1, B_m as spend_pk, encode equals the TV3 labeled recomputed string.
  - **6 negative:** `decode_xch_hrp_rejected`, `decode_invalid_checksum_rejected`, `decode_bech32_not_bech32m_rejected`, `decode_short_payload_rejected`, `decode_identity_scan_pk_rejected`, `decode_identity_spend_pk_rejected` — each asserts the correct error variant.
- All plan-level gates green: `cargo build --release -p chia-sdk-utils` (no features) + `-F chip-0057` + `--all-features` + workspace `--all-features`; `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests` (12/12 pass); `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; both grep bans (`mod_by_group_order`, `^use sha2::`) hold under `crates/chia-sdk-utils/src/silent_payments/`; no `#[allow(...)]` attributes in `address.rs`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Append SilentPaymentAddress struct + encode/decode to address.rs** — `9eb57817` (feat)
2. **Task 2: Add 12 named #[test] functions to address.rs (TV1 round-trip + TV1/TV3 encode-pinned + 6 negative)** — `ffb97dee` (test, with inline clippy fixes for `doc_markdown` and `unnested_or_patterns`)

**Plan metadata commit:** to follow this SUMMARY.

## Files Created/Modified

- `crates/chia-sdk-utils/src/silent_payments/address.rs` (modified; was 41 lines after Plan 02-02, now 336 lines):
  - Lines 1-10: module doc + consolidated top-of-file `use` block (`chia_bls::PublicKey`, `chia_protocol::Bytes`, `crate::Bech32`, `super::SilentPaymentError`).
  - Lines 12-41 (unchanged from Plan 02-02): `SilentPaymentNetwork` enum + `hrp()` + `from_hrp()`.
  - Lines 43-126 (new): `SilentPaymentAddress` struct + `new()` + `encode()` + `decode()`.
  - Lines 128-339 (new): `#[cfg(test)] mod tests` with TV1/TV3 hex constants, the address-string constants, a `pk()` helper, and 12 named `#[test]` functions.

## Decisions Made

- **Permissive identity-pubkey assertions:** Both `decode_identity_*_rejected` tests assert `matches!(result, Err(IdentityPublicKey | InvalidPublicKey))`. Empirical observation at Task 2: chia-bls 0.36.1's `PublicKey::from_bytes(&[0xc0, 0, ..., 0])` succeeds and the resulting `pk.is_inf()` returns true — so the IdentityPublicKey branch fires in the current toolchain. The permissive matcher remains as the documented contract since the test asserts the boundary (rejection), not the internal labeling. Plan instruction explicitly allowed leaving the matcher permissive — tightening was optional, not required.
- **No `#[allow(...)]` attributes anywhere in `address.rs`:** The two clippy issues encountered (`doc_markdown` complaining about `scan_pk`/`spend_pk` in a doc-comment, `unnested_or_patterns` complaining about `Err(A) | Err(B)` inside `matches!`) were both fixed structurally — backticks on identifiers, and `Err(A | B)` nesting. Workspace pedantic clippy gate enforces this discipline per Phase 1 Pitfall 9.
- **Consolidated top-of-file `use` block:** After the initial Edit placed the new `use chia_bls::PublicKey;` etc. mid-file (immediately above the struct), `cargo fmt` did not auto-merge them. Manually consolidated to the file head to match the convention in the existing `silent_payments/error.rs` and the rest of `chia-sdk-utils`. The four imports (`chia_bls::PublicKey`, `chia_protocol::Bytes`, `crate::Bech32`, `super::SilentPaymentError`) are now grouped at lines 4-10.
- **Test-vector hex constants use the doubled-string `hex!(...)` form:** Each 48-byte / 96-byte hex constant is split across two adjacent string literals inside `hex_literal::hex!(...)`. No `+` concatenation. Matches Phase 1's pattern in `chia-sdk-types/src/silent_payments/tagged_hash.rs:44` and is the cleanest way to type out 96 hex digits without line wrap.

## Deviations from Plan

**1. [Rule 1 - Bug] Inline clippy fixes (doc_markdown + unnested_or_patterns) during Task 2**

- **Found during:** Task 2 — after running `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings`.
- **Issue 1 (doc_markdown):** The doc-comment for `encode()` mentioned `scan_pk` and `spend_pk` as plain identifiers without backticks; pedantic clippy flagged them as `item in documentation is missing backticks`. The doc-comment was added in Task 1 but only triggered at Task 2 because `cargo build` (Task 1's verify) is more permissive than `clippy -D warnings`.
- **Issue 2 (unnested_or_patterns):** The two identity-pubkey test `assert!(matches!(...))` calls used `Err(A) | Err(B)` rather than `Err(A | B)`. Pedantic clippy preferred the nested form.
- **Fix:** Wrapped `scan_pk`, `spend_pk`, and `"1"` in backticks in the doc-comment (lines 70). Replaced `Err(IdentityPublicKey) | Err(InvalidPublicKey)` with `Err(IdentityPublicKey | InvalidPublicKey)` in both identity-pubkey tests.
- **Files modified:** `crates/chia-sdk-utils/src/silent_payments/address.rs` (3 small edits inside Task 2's overall change).
- **Commit:** `ffb97dee` (rolled into the same Task 2 test commit, since these were the bring-the-suite-green fixes for Task 2's clippy verify acceptance).

**2. [Rule 1 - Bug] Imports consolidation after rustfmt**

- **Found during:** Task 1 → Task 2 transition — `cargo fmt --all` left `use chia_bls::PublicKey;`, `use chia_protocol::Bytes;`, `use crate::Bech32;` in their as-appended mid-file position rather than merging them with the existing `use super::SilentPaymentError;` block at the top.
- **Issue:** Two `use`-statement blocks separated by other items (`SilentPaymentNetwork` enum + impl) violate the file-level convention. Risked triggering pedantic clippy in a future plan and was inconsistent with `error.rs` (single top-of-file `use` block).
- **Fix:** Moved the three new `use` statements to the top of the file, grouping with the existing `use super::SilentPaymentError;`.
- **Files modified:** `crates/chia-sdk-utils/src/silent_payments/address.rs`.
- **Commit:** `ffb97dee` (rolled into Task 2 commit, since the consolidation was discovered when running clippy at Task 2 time).

Neither deviation changed the Plan's intent — both were Rule 1 cleanups to bring the file to the clippy-clean state the plan's verify gate demanded.

## Issues Encountered

None substantial. All test vectors passed on first run (12/12), confirming Task 1's production code was correct end-to-end (HRP wiring, payload byte order, bech32m round-trip, identity rejection via is_inf). The two clippy fixes above were anticipated style adjustments to satisfy `-D warnings`.

## User Setup Required

None — no external service or environment configuration required.

## Next Phase Readiness

**Plan 02-04 (keys + labels) is unblocked:**
- `SilentPaymentAddress::new(scan_pk, spend_pk, network)` is the public constructor that `SilentPaymentKeys::unlabeled_address` and `SilentPaymentKeys::labeled_address` will return.
- `super::SilentPaymentAddress` is the path Plan 02-04's `keys.rs` and `labels.rs` will import via `use super::SilentPaymentAddress;` — the `pub use address::*;` in `silent_payments/mod.rs` makes it visible.
- `super::SilentPaymentNetwork` from Plan 02-02 plus `super::SilentPaymentAddress` from this plan are both available under one `use super::{SilentPaymentAddress, SilentPaymentNetwork};` import.
- TV1 mainnet/testnet address strings are now embedded in `address.rs` test constants; Plan 02-04's keys tests will use a different consts module (in `keys.rs`) to pin `b_scan`, `b_spend`, `scan_pk`, `spend_pk` byte sequences from RESEARCH §8 — no constant reuse needed across files.

**Plan 02-05 (prelude + final gate) is unblocked but waits for 02-04:**
- The deferred `prelude.rs` re-export from Plan 02-01 can now include `SilentPaymentAddress` once `SilentPaymentKeys` (Plan 02-04) is also defined.

**Blockers/concerns:** None. ADDR-02 is closed; ROADMAP Phase 2 success criterion 1 (TV1 round-trip) and criterion 3 (negative-case coverage) verifiable mechanically.

## Self-Check

- [x] `crates/chia-sdk-utils/src/silent_payments/address.rs` exists — verified via Read
- [x] Commit `9eb57817` (Task 1, feat) exists — verified via `git log --oneline`
- [x] Commit `ffb97dee` (Task 2, test) exists — verified via `git log --oneline`
- [x] All 12 named tests pass under `--exact` invocations (one per row of 02-VALIDATION.md rows 47-56 + 60-61) — verified via per-test cargo test loop
- [x] `cargo build --release -p chia-sdk-utils` (no features) green
- [x] `cargo build --release -p chia-sdk-utils -F chip-0057` green
- [x] `cargo build --release -p chia-sdk-utils --all-features` green
- [x] `cargo build --release --workspace --all-features` green
- [x] `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean
- [x] `cargo fmt --all -- --files-with-diff --check` clean
- [x] `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` holds
- [x] `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` holds
- [x] `! grep -E '#\[allow\(' crates/chia-sdk-utils/src/silent_payments/address.rs` holds (no clippy-bypass attributes)
- [x] `! grep -E '^use bech32::' crates/chia-sdk-utils/src/silent_payments/address.rs` holds (no direct bech32 imports — Pitfall 1)
- [x] `pub struct SilentPaymentAddress` exists with the three pub fields and the four-derive attribute (verified via grep against acceptance criteria)
- [x] `encode()` and `decode()` signatures match the locked acceptance grep patterns
- [x] `address.rs` line count ≥ 250 (acceptance min_lines): 336 lines

## Self-Check: PASSED

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
