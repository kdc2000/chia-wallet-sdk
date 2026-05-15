---
phase: 02-address-key-types
plan: 02
subsystem: infra
tags: [chip-0057, silent-payments, error-types, network-enum, bech32m-hrp, thiserror, chia-sdk-utils]

# Dependency graph
requires:
  - phase: 02-address-key-types
    provides: "Plan 02-01: chip-0057 feature wired on chia-sdk-utils with optional deps (chia-sdk-types, bip39, chia-bls); silent_payments module barrel exists as cfg-gated public empty barrel ready for sorted-order submodule appends"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "Workspace-level chip-0057 feature flag; chia_sdk_utils::Bech32Error type re-exported at crate root from existing bech32.rs (lines 5-18)"
provides:
  - "chia_sdk_utils::silent_payments::SilentPaymentError enum with six pinned variants: WrongHrp(String), PayloadLength(usize), InvalidPublicKey, IdentityPublicKey, ReservedChangeLabel, Bech32(#[from] crate::Bech32Error)"
  - "chia_sdk_utils::silent_payments::SilentPaymentNetwork enum (Mainnet, Testnet) with hrp(self) -> &'static str and from_hrp(&str) -> Result<Self, SilentPaymentError>"
  - "Address HRPs pinned: 'spxch' (mainnet, 5 chars) and 'tspxch' (testnet, 6 chars)"
  - "address.rs stub form: SilentPaymentAddress struct intentionally NOT yet present — Plan 02-03 appends it (same-file co-location matches the natural type grouping)"
  - "silent_payments/mod.rs declares mod address; mod error; in sorted order with pub use re-exports"
affects: [02-03-address-encoding, 02-04-keys-and-labels, 02-05-prelude-and-gate]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Typed error enum first, before any logic depends on it: lets every downstream task `Result<_, SilentPaymentError>` from the first line of code, no refactor when the next negative-case test needs a new variant"
    - "Network discriminator co-located with future address struct in `address.rs` rather than its own file: per CHIP-0057 §1/§3 the network is a field of the address, so the type group is one file (matches `Address` + `Bech32` in chia-sdk-utils/src/bech32.rs)"
    - "Same-file APPEND pattern across plans: Plan 02-03 appends SilentPaymentAddress + encode/decode + 8 tests to the same `address.rs`; avoids creating a `network.rs` that would later need to be merged into `address.rs`"
    - "Six-variant error pinning via grep: each `#[error(\"...\")]` string is locked verbatim and checked by `grep -F` in acceptance criteria, so wave-2 parallel tests (02-03 and 02-04) can pattern-match without coordinating wording"

key-files:
  created:
    - "crates/chia-sdk-utils/src/silent_payments/error.rs (45 lines, six-variant SilentPaymentError enum + From<Bech32Error> via #[from])"
    - "crates/chia-sdk-utils/src/silent_payments/address.rs (41 lines, stub: SilentPaymentNetwork enum + hrp() + from_hrp(); SilentPaymentAddress struct comes in Plan 02-03)"
  modified:
    - "crates/chia-sdk-utils/src/silent_payments/mod.rs (replaced sentinel comment with `mod address; pub use address::*; mod error; pub use error::*;` in sorted order)"

key-decisions:
  - "SilentPaymentError does NOT derive Clone or PartialEq (RESEARCH §7) — neither is needed for the API surface, and Bech32Error already supports Clone/PartialEq if a consumer needs pattern depth"
  - "Network is `Mainnet`/`Testnet` only (not `Simnet`/`Devnet`/etc.) — matches the CHIP-0057 §153/§206 HRP table; future regtests can use Testnet"
  - "address.rs ships in STUB form per the plan's design: declares SilentPaymentNetwork only, with a comment marking that Plan 02-03 will APPEND the SilentPaymentAddress struct + encode/decode + tests to the same file. Avoids reshuffling files between waves."
  - "Six error variants pinned verbatim from RESEARCH §7 (no paraphrasing in #[error(\"...\")] strings) so that wave-2 plans (02-03, 02-04) running in parallel can both pattern-match on the exact variant constructors without communicating about message wording"
  - "from_hrp uses `other.to_string()` (not `hrp.to_string()`) inside the catch-all arm — preserves the original bound-but-unmatched value so the error message shows what the user actually supplied (e.g., 'xch' from a standard-address paste)"

patterns-established:
  - "Phase 2 wave-2 split pattern: small foundational types (errors + network discriminant) land in a single quick plan, unblocking subsequent waves that can run in parallel because they share no files"
  - "Stub-as-final-file-location pattern: rather than creating `network.rs` and later merging into `address.rs`, ship `address.rs` with only the network discriminant; downstream plan appends to the same file. Reduces churn on the mod.rs barrel and keeps the type group co-located from day one."

requirements-completed: []  # Plan 02-02 frontmatter lists ADDR-02 and ADDR-06 as eventual coverage, but `requirements_addressed: []` is explicit. ADDR-02 closes in Plan 02-03 (SilentPaymentAddress + encode/decode + round-trip tests). ADDR-06 closes in Plan 02-04 (labeled_address(0) -> Err(ReservedChangeLabel)). Plan 02-02 lays the foundation (HRP enum + error variant) for both.

# Metrics
duration: 8min
completed: 2026-05-15
---

# Phase 02 Plan 02: SilentPaymentError + SilentPaymentNetwork foundational types Summary

**Six-variant `SilentPaymentError` enum (with `#[from] Bech32Error` for ergonomic conversion) plus `SilentPaymentNetwork` discriminant (`spxch`/`tspxch` HRPs) — the two foundational types Plans 02-03 and 02-04 both depend on, now public via `chia_sdk_utils::silent_payments::{...}`.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-15T19:34:10Z
- **Completed:** 2026-05-15T19:42:04Z
- **Tasks:** 3 (2 file-creation + 1 barrel edit)
- **Files modified:** 3 (2 new + 1 edited)

## Accomplishments
- `SilentPaymentError` enum lives in `crates/chia-sdk-utils/src/silent_payments/error.rs` with the six pinned variants from RESEARCH §7 in the exact declaration order: `WrongHrp(String)`, `PayloadLength(usize)`, `InvalidPublicKey`, `IdentityPublicKey`, `ReservedChangeLabel`, `Bech32(#[from] crate::Bech32Error)`. Each `#[error("...")]` string is verbatim from RESEARCH §7.
- `SilentPaymentNetwork` enum lives in `crates/chia-sdk-utils/src/silent_payments/address.rs` (stub form) with `Mainnet`/`Testnet` variants, `hrp(self) -> &'static str` returning `"spxch"`/`"tspxch"`, and `from_hrp(&str) -> Result<Self, SilentPaymentError>` parsing the inverse direction with `WrongHrp(other.to_string())` on unknown HRPs.
- `silent_payments/mod.rs` now declares both submodules in sorted order (`address` < `error`) with `pub use ...::*;` re-exports. Plan 02-04 will append `keys` and `labels` after `error`.
- All 5 build permutations green: `chia-sdk-utils` ± `chip-0057`, `chia-sdk-utils --all-features`, workspace ± `--all-features`. `cargo fmt --check` clean. Both grep bans (`mod_by_group_order`, `^use sha2::`) still hold across `silent_payments/`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create silent_payments/error.rs with the six-variant SilentPaymentError enum** — `7f920eb7` (feat)
2. **Task 2: Create silent_payments/address.rs stub with SilentPaymentNetwork only** — `94c74267` (feat)
3. **Task 3: Wire address.rs + error.rs into silent_payments/mod.rs barrel** — `d3f6e2f8` (feat)

**Plan metadata commit:** to follow this SUMMARY.

_Note: Plan-level frontmatter declares the tasks as `tdd="true"` but the plan body does not specify behavioral tests for these foundational types — the negative-case tests that exercise each error variant are scheduled in Plan 02-03 (8 address tests) and Plan 02-04 (keys/labels tests). The two TDD-flagged tasks here are pure declarative-type creation, so no `test` commits were produced; RED/GREEN happens at the wave-2 boundary._

## Files Created/Modified

- `crates/chia-sdk-utils/src/silent_payments/error.rs` (NEW, 45 lines) — Six-variant `SilentPaymentError` enum with `#[from] Bech32Error` conversion; `#[derive(Debug, Error)]`; six `#[error("...")]` strings pinned verbatim from RESEARCH §7.
- `crates/chia-sdk-utils/src/silent_payments/address.rs` (NEW, 41 lines, STUB) — Two-variant `SilentPaymentNetwork` enum + `hrp()` + `from_hrp()`; intentionally does NOT yet contain `SilentPaymentAddress` (Plan 02-03 appends it).
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (modified, 32 lines after edit) — Replaced sentinel-comment block at bottom of file with `mod address; pub use address::*; mod error; pub use error::*;` in sorted order. Doc-comment block (lines 1-27) untouched.

## Decisions Made

- **No `Clone` / `PartialEq` derives on `SilentPaymentError`** — RESEARCH §7 explicitly omits them. The `Bech32Error` inner type retains its own `Clone+PartialEq+Eq` for callers that need to match against bech32 wire errors; the outer `SilentPaymentError` is just `Debug+Error`. Adding `Clone` would force `Bech32Error`'s `bech32::Error` source-error wrapper to expose its own Clone, which isn't guaranteed across versions.
- **Network is a two-variant enum, not a string** — `SilentPaymentNetwork::Mainnet`/`Testnet` with `hrp(self)` is a stronger contract than a free-form `String` HRP: the compiler enforces that only valid networks compile, and the HRP-string mapping lives in one place (the `match` in `hrp()`) so the inverse (`from_hrp`) is mechanically the same map flipped.
- **`from_hrp` uses bound-variable `other`** — `Err(SilentPaymentError::WrongHrp(other.to_string()))` rather than `Err(SilentPaymentError::WrongHrp(hrp.to_string()))`. Both compile; using `other` (the match-arm binding) signals that the original `hrp` parameter is being passed through unchanged.
- **Stub `address.rs` ships in its final filename, not a temporary `network.rs`** — keeps the type group (network + future address struct) in one file from the moment the network discriminant exists. Plan 02-03 appends the `SilentPaymentAddress` struct + tests without any rename or file move.

## Deviations from Plan

None — plan executed exactly as written. All three tasks completed in their declared order; all acceptance criteria green on first attempt; no Rule 1/2/3 auto-fixes triggered.

## Issues Encountered

None. All builds and grep checks passed on first attempt with no compilation friction.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready for Plan 02-03 (address encoding) and Plan 02-04 (keys + labels) — these can now run in parallel:**
- Both plans can `use super::{SilentPaymentError, SilentPaymentNetwork};` and reference the six error variants by name.
- Plan 02-03 will APPEND `SilentPaymentAddress` struct + `encode()`/`decode()` methods + 8 tests to the same `address.rs` file — no new file needed, no `mod.rs` edit needed (the `pub use address::*;` already re-exports anything added).
- Plan 02-04 will create `keys.rs` and `labels.rs` and APPEND `mod keys; pub use keys::*; mod labels; pub use labels::*;` AFTER `pub use error::*;` in `mod.rs` (sorted order continues: `address < error < keys < labels`).
- Plan 02-05 (final gate) takes over the `prelude.rs` re-export work that Plan 02-01 deferred and the clippy/machete sweep that Plan 02-02 did not run (deferred per the plan's verification section).

**Blockers/concerns:** None for Plans 02-03 and 02-04. The wave-2 parallelism is now unblocked.

## Self-Check

- [x] `crates/chia-sdk-utils/src/silent_payments/error.rs` exists — verified via `test -f`
- [x] `crates/chia-sdk-utils/src/silent_payments/address.rs` exists — verified via `test -f`
- [x] `crates/chia-sdk-utils/src/silent_payments/mod.rs` updated — verified `mod address;` line at line 29, `mod error;` line at line 31, sorted
- [x] Commit `7f920eb7` exists — verified via `git log --oneline -6`
- [x] Commit `94c74267` exists — verified via `git log --oneline -6`
- [x] Commit `d3f6e2f8` exists — verified via `git log --oneline -6`
- [x] All Task 1 acceptance criteria (17 checks) pass — verified
- [x] All Task 2 acceptance criteria (14 checks) pass — verified
- [x] All Task 3 acceptance criteria pass: 5 build permutations + fmt + 2 grep bans + sort-order check
- [x] `error.rs` is 45 lines (≥ 40 min_lines)
- [x] `address.rs` is 41 lines (≥ 30 min_lines)
- [x] `SilentPaymentAddress` struct NOT in `address.rs` yet (stub form preserved for Plan 02-03 append)

## Self-Check: PASSED

---
*Phase: 02-address-key-types*
*Completed: 2026-05-15*
