---
phase: 1
slug: crypto-primitives-workspace-integration
status: passed
date: 2026-05-15
score: 9/9 must-haves verified; 5/5 ROADMAP success criteria met (with 2 ROADMAP-text imprecisions documented as recommended follow-ups, not gaps)
re_verification: false
verifier_notes:
  - "Implementation is correct. Two ROADMAP success-criterion wording inaccuracies surface during verification but are imprecisions in the ROADMAP/VALIDATION TEXT, not the code. Recommend a docs-only follow-up."
follow_ups_recommended:
  - id: roadmap-sc2-rewording
    target: ".planning/ROADMAP.md Phase 1 success criterion 2"
    issue: "Literal wording says `(2^256-1) mod r == r - 1` which is mathematically false for BLS12-381 r. The actual reduction is `(2^256-1) - r = 0x1824b159...fffffffd` (test pins this value)."
    suggested_fix: "Replace `equals \"r - 1\"` with `equals the pinned reduced value `0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffd`, NOT `[0xff; 32]`` — or just `produces the unsigned mod-r reduction, NOT `[0xff; 32]``."
    blocking: false
  - id: roadmap-sc3-rewording
    target: ".planning/ROADMAP.md Phase 1 success criterion 3"
    issue: "Wording references `chia_sha2::Sha256::digest(\"Chia_SP/Inputs\")`, but chia-sha2 0.36.1 exposes only `new/update/finalize` (no `::digest` static method — RESEARCH.md Pitfall 3). Implementation correctly uses `new/update/finalize` and pins the same SHA-256 output via a `sha256()` helper in tests."
    suggested_fix: "Replace `chia_sha2::Sha256::digest(\"X\")` with `SHA-256 of \"X\"`. The output values are unchanged."
    blocking: false
  - id: validation-row-43-test-name
    target: ".planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md row 43 + test source"
    issue: "Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is misnamed (same root cause as SC2)."
    suggested_fix: "Rename to `from_bytes_unsigned_max_input_reduces_to_pinned_value` or `..._reduces_correctly` in a coordinated rename across VALIDATION.md, the test source, and any acceptance criteria that lock the literal name with `--exact`."
    blocking: false
  - id: daemon-pedantic-lints
    target: "crates/chia-sdk-daemon/src/client.rs:426-427"
    issue: "Two `clippy::pedantic` warnings (`match_same_arms`, `match_wildcard_for_single_variants`) introduced by upstream commit `ec1a3517`, pre-dating Phase 1. CI's clippy step does not pass `-D warnings`, so CI still exits 0; only a strict local invocation surfaces them."
    suggested_fix: "Independent small PR to either (a) collapse the match arms, or (b) align CI's clippy step with `-D warnings`. Documented in `deferred-items.md`."
    blocking: false
---

# Phase 1: Crypto primitives & workspace integration — Verification Report

**Phase Goal (from ROADMAP):** The cryptographic foundation — `ScalarField` type boundary, tagged-hash, derivation paths — is in place under `chip-0057` and every CI permutation builds cleanly.

**Verified:** 2026-05-15
**Status:** PASSED
**Re-verification:** No — initial verification.

---

## Executive Summary

All 9 must-haves (M1..M9) verified GREEN on the live codebase. All 5 ROADMAP success criteria substantively met. All 5 declared requirements (CRYPTO-01, CRYPTO-02, WS-01, WS-02, WS-03) satisfied with concrete implementation evidence.

Two ROADMAP text imprecisions discovered during verification (SC2 says `r - 1`; SC3 references a `::digest` method that does not exist on `chia-sha2`). Both are wording bugs in the planning artifacts, not implementation bugs — the implementation correctly handles both cases (SC2 pins the actual reduced value; SC3 uses `new/update/finalize` to compute the same SHA-256 output). The phase verifier accepts the substantive behavior as correct and surfaces both as **non-blocking, docs-only follow-up recommendations**. See `follow_ups_recommended` in frontmatter.

The known carried observations (executor-documented in `deferred-items.md` and `01-PHASE-SUMMARY.md`) are accepted:
1. **`r - 1` misnomer in test name + VALIDATION row 43 + SC2 wording** — accept. The test passes; the math is correct; the name is misleading. Recommend coordinated rename.
2. **Pre-existing `chia-sdk-daemon` pedantic warnings** — accept. M4 is phrased "as CI runs it" → PASS. The strict `-D warnings` form is blocked by an upstream pre-Phase-1 commit (`ec1a3517`), not Phase 1.

---

## Must-Haves (M1..M9) — Final Status

| ID | Description | Status | Evidence |
|----|-------------|--------|----------|
| M1 | `cargo build --release -p chia-sdk-types -F chip-0057` succeeds | PASS | Verifier ran: `cargo build --release -p chia-sdk-types -F chip-0057` → exit 0 (finished in 0.37s; incremental cache hit) |
| M2 | `cargo build --release --workspace --all-features` succeeds | PASS | Verifier ran: `cargo build --release --workspace --all-features` → exit 0 (3m 01s, full build) |
| M3 | `cargo build --release --workspace` (no features) succeeds | PASS | Verifier ran: `cargo build --release --workspace` → exit 0 (2m 47s, full build); confirms chip-0057 code is properly gated |
| M4 | `cargo clippy --workspace --all-features --all-targets` (as CI runs it) is clean | PASS | Verifier ran: `cargo clippy --workspace --all-features --all-targets` → exit 0 (matches CI workflow line 75). Two pre-existing `chia-sdk-daemon` pedantic warnings surface but CI does not pass `-D warnings`, so the step exits 0. Scoped `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` is also clean (the only crate Phase 1 touched). |
| M5 | `cargo fmt --all -- --files-with-diff --check` clean | PASS | Verifier ran: `cargo fmt --all -- --files-with-diff --check` → exit 0 |
| M6 | `cargo machete` clean with no new `[package.metadata.cargo-machete] ignored` entries | PASS | Verifier ran: `cargo machete` → exit 0 ("didn't find any unused dependencies"). `git diff 87705945..HEAD -- '**/Cargo.toml' 'Cargo.toml' \| grep -E '(package.metadata.cargo-machete\|^\+.*ignored)'` returned zero matches. Pre-existing entries in 6 files (chia-sdk-daemon, chia-sdk-test, chia-sdk-client, wasm, pyo3, napi) are unchanged. |
| M7 | `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` returns zero matches | PASS | Verifier ran: `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` → exit 1 (no matches across paths.rs, scalar.rs, tagged_hash.rs, mod.rs) |
| M8 | All 10 ScalarField + tagged_hash unit tests pass (5 scalar + 5 tagged_hash) | PASS | Verifier ran: `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` → 10 passed; 0 failed; 0 ignored. Individual test names confirmed: `from_bytes_raw_does_not_reduce`, `from_bytes_unsigned_max_input_reduces_to_r_minus_one`, `mul_mod_r`, `from_bytes_unsigned_identity`, `add_wraps_at_r`, `different_tags_different_outputs`, `tag_inputs_hash_pinned`, `tag_label_hash_pinned`, `tag_shared_secret_hash_pinned`, `tagged_hash_matches_bip340_challenge_vector`. |
| M9 | `chip-0057` feature declared on chia-sdk-types, chia-sdk-driver, AND chia-sdk-utils | PASS | Verifier confirmed: <ul><li>`Cargo.toml:75` — `chip-0057 = ["chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]`</li><li>`crates/chia-sdk-types/Cargo.toml:20` — `chip-0057 = []`</li><li>`crates/chia-sdk-driver/Cargo.toml:23` — `chip-0057 = ["chia-sdk-types/chip-0057"]`</li><li>`crates/chia-sdk-utils/Cargo.toml:18` — `chip-0057 = []`</li></ul> |

**Score: 9/9 must-haves PASS.**

---

## Requirements Coverage (CRYPTO-01, CRYPTO-02, WS-01, WS-02, WS-03)

| Requirement | Description (abridged from REQUIREMENTS.md) | Source Plan(s) | Status | Evidence |
|-------------|--------------------------------------------|----------------|--------|----------|
| **CRYPTO-01** | `ScalarField` newtype performs unsigned mod-r reduction over BLS12-381 subgroup order using `num-bigint`. No public `From<[u8;32]>` constructor. | 01-02-PLAN | PASS | `crates/chia-sdk-types/src/silent_payments/scalar.rs:36-103` defines `ScalarField([u8; 32])` with `from_bytes_unsigned` (uses `BigUint::from_bytes_be(&bytes) % BigUint::from_bytes_be(&GROUP_ORDER)`, lines 47-53) and `from_bytes_raw` (no reduction, lines 61-64). Grep for `impl From<\[u8; 32\]> for ScalarField` returns zero matches (verified). 5 unit tests pin the protocol semantics. REQUIREMENTS.md row marked `[x]`. |
| **CRYPTO-02** | `tagged_hash(tag, data)` BIP-340 via `chia-sha2`; three `Chia_SP/*` `&'static str` consts. | 01-03-PLAN | PASS | `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs:28-39` implements `tagged_hash(tag: &str, data: &[u8]) -> [u8; 32]` via `chia_sha2::Sha256::{new, update, finalize}`. Three `pub const &str`: `CHIA_SP_INPUTS = "Chia_SP/Inputs"`, `CHIA_SP_SHARED_SECRET = "Chia_SP/SharedSecret"`, `CHIA_SP_LABEL = "Chia_SP/Label"` (lines 15, 18, 21). Grep `^use sha2::` in silent_payments/ → zero matches. Grep `Sha256::digest` in silent_payments/ → zero matches. 5 unit tests pass. REQUIREMENTS.md row marked `[x]`. |
| **WS-01** | New `chip-0057` workspace feature cascades to types/driver/utils. | 01-01-PLAN | PASS | Cascade verified by grep: root `Cargo.toml:75` declares the umbrella; per-crate features declared at the three target Cargo.tomls. All five build permutations exit 0. REQUIREMENTS.md row marked `[x]`. |
| **WS-02** | `.github/workflows/rust.yml` adds per-crate builds with `-F chip-0057`. | 01-05-PLAN | PASS | `.github/workflows/rust.yml:65` contains `cargo build --release -p chia-sdk-types -F chip-0057` (10-space indent matching surrounding lines). Verified via `grep -nE 'cargo build --release -p chia-sdk-types' .github/workflows/rust.yml` showing 3 lines (no-features at line 63, --all-features at line 64, -F chip-0057 at line 65). REQUIREMENTS.md row marked `[x]`. |
| **WS-03** | All chip-0057 code passes deny `clippy::all` + warn `pedantic` + deny `unsafe_code` + deny `dead_code` + `cargo machete`. No new `ignored` entries. | 01-04-PLAN, 01-05-PLAN | PASS | Scoped `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` exit 0; `cargo machete` exit 0 with no diff in `[package.metadata.cargo-machete]` blocks vs phase-start commit `87705945`. `grep unsafe crates/chia-sdk-types/src/silent_payments/` returns no matches (no unsafe blocks anywhere in the gated module). REQUIREMENTS.md row marked `[x]`. |

**No ORPHANED requirements.** REQUIREMENTS.md Traceability table assigns exactly 5 IDs to Phase 1 (`CRYPTO-01`, `CRYPTO-02`, `WS-01`, `WS-02`, `WS-03`) and all 5 are claimed by the 5 plans in this phase.

---

## ROADMAP Success Criteria

| # | Criterion (verbatim from ROADMAP) | Status | Evidence |
|---|-----------------------------------|--------|----------|
| 1 | `cargo build -p chia-sdk-types -F chip-0057` succeeds; `cargo build --workspace --all-features` succeeds; `cargo build --workspace` (no features) succeeds; `cargo clippy --workspace --all-features --all-targets` is clean. | PASS | M1, M2, M3, M4 all verified. CI clippy (no `-D warnings`) exits 0. |
| 2 | Adversarial unit test passes: `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes()` equals `r - 1` (big-endian), NOT `[0xff; 32]`. (Verifies unsigned reduction over BLS12-381 subgroup order.) | PASS (with documented imprecision) | Test `from_bytes_unsigned_max_input_reduces_to_r_minus_one` passes. The mathematics: `(2^256 - 1) mod r` is NOT `r - 1` for BLS12-381's r — the actual value is `0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffd` (= `(2^256-1) - r`, since `floor((2^256-1)/r) = 1`). The test correctly pins this value AND asserts `!= [0xff; 32]`. The spirit of the criterion ("unsigned reduction did fire on `[0xff; 32]`") is met. The literal wording `equals r - 1` is mathematically wrong but the test does NOT claim to equal `r - 1`. **Disposition: ACCEPT** — implementation is correct; ROADMAP text needs a docs-only patch (see `follow_ups_recommended.roadmap-sc2-rewording`). |
| 3 | Tag-pin unit test passes: SHA-256 of `"Chia_SP/Inputs"`, `"Chia_SP/SharedSecret"`, `"Chia_SP/Label"` each produces a pinned 32-byte value. | PASS | All three pinned tests pass: `tag_inputs_hash_pinned` (pins `d44a6db8...3378cf`), `tag_shared_secret_hash_pinned` (pins `e7b8a524...b9ca9f`), `tag_label_hash_pinned` (pins `c63c8bd2...f94554`). Implementation uses `chia_sha2::Sha256::{new, update, finalize}` because `chia-sha2` does not expose a `::digest` method (RESEARCH.md Pitfall 3); the ROADMAP wording's reference to `chia_sha2::Sha256::digest(...)` is an inaccuracy in the criterion text, not a defect in the test. **Disposition: ACCEPT** — implementation is correct; ROADMAP text needs a docs-only patch (see `follow_ups_recommended.roadmap-sc3-rewording`). The note in the verifier prompt's `<roadmap_success_criteria>` block re-states this criterion as `SHA-256 of "..."` (without `::digest`), which is the intent. |
| 4 | `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` returns zero hits. | PASS | M7 verified — verifier-rerun: zero matches. |
| 5 | `cargo machete` passes with no new `[package.metadata.cargo-machete] ignored` entries. | PASS | M6 verified — `cargo machete` exits 0 and `git diff 87705945..HEAD` shows zero new `ignored`-block additions in any Cargo.toml. |

**All 5 success criteria substantively met.** Two criteria carry documented wording imprecisions (#2 and #3) — the implementation is correct against the spirit of the criteria; the criterion text needs minor patching in a docs-only follow-up.

---

## Required Artifacts — Three-Level Verification

| Artifact | Expected | Exists | Substantive | Wired | Status |
|----------|----------|--------|-------------|-------|--------|
| `crates/chia-sdk-types/src/silent_payments/mod.rs` | Module barrel re-exporting paths/scalar/tagged_hash, doc-comment | YES (18 lines) | YES (descriptive doc-comment about signed-vs-unsigned asymmetry; 3 `mod X; pub use X::*;` pairs in sorted order `paths < scalar < tagged_hash`) | YES (declared by `crates/chia-sdk-types/src/lib.rs:3-4` behind `#[cfg(feature = "chip-0057")]`) | VERIFIED |
| `crates/chia-sdk-types/src/silent_payments/scalar.rs` | `ScalarField` + `GROUP_ORDER` + 5 named tests, ≥80 lines | YES (186 lines) | YES (newtype + GROUP_ORDER + from_bytes_unsigned/from_bytes_raw/add/mul/as_bytes/to_bytes/is_zero + biguint_to_be_bytes_32 helper + 5 named tests + Copy/Clone/Debug/PartialEq/Eq derives + `#[must_use]` on all public functions; NO `From<[u8;32]>` impl) | YES (re-exported via `mod.rs:13-14`) | VERIFIED |
| `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` | tagged_hash + 3 tag consts + 5 named tests, ≥60 lines | YES (98 lines) | YES (BIP-340 construction via `chia_sha2::Sha256::{new, update, finalize}`; 3 `pub const &str` constants matching CHIP-0057 draft; 5 named tests with cross-implementation BIP-340 vector + 3 pinned tag-SHA-256 values + sanity test) | YES (re-exported via `mod.rs:15-16`) | VERIFIED |
| `crates/chia-sdk-types/src/silent_payments/paths.rs` | SCAN_PATH + SPEND_PATH `pub const &[u32]`, ≥12 lines | YES (25 lines) | YES (`pub const SCAN_PATH: &[u32] = &[12381, 8444, 12, 0]`; `pub const SPEND_PATH: &[u32] = &[12381, 8444, 13, 0]`; descriptive-phrasing doc-comment with no `mod_by_group_order` literal) | YES (re-exported via `mod.rs:11-12`) | VERIFIED |
| `crates/chia-sdk-types/src/lib.rs` | `#[cfg(feature = "chip-0057")] pub mod silent_payments;` declaration | YES (line 3-4) | YES (correctly gated and `pub mod` so the module is reachable as `chia_sdk_types::silent_payments`) | YES | VERIFIED |
| `Cargo.toml` (root) | `chip-0057 = [...]` cascade | YES (line 75) | YES (cascades to driver/types/utils) | YES (drives the workspace `--all-features` build) | VERIFIED |
| `crates/chia-sdk-types/Cargo.toml` | `chip-0057 = []` + `num-bigint = { workspace = true }` | YES | YES (feature at line 20; num-bigint at line 36) | YES (consumed by `silent_payments/scalar.rs`) | VERIFIED |
| `crates/chia-sdk-driver/Cargo.toml` | `chip-0057 = ["chia-sdk-types/chip-0057"]` cascade | YES (line 23) | YES (pure cascade, no `dep:*` activations — by design, no chip-0057 code in driver yet) | YES (cascade verified by `cargo build --workspace --all-features`) | VERIFIED |
| `crates/chia-sdk-utils/Cargo.toml` | New `[features]` block with `chip-0057 = []` | YES (lines 17-18) | YES (empty feature, anticipating Phase 2 address code) | YES (referenced by root cascade) | VERIFIED |
| `.github/workflows/rust.yml` | Per-crate `cargo build --release -p chia-sdk-types -F chip-0057` line | YES (line 65) | YES (10-space indent matches surrounding lines; YAML still parses) | YES (will execute in CI on every push/PR) | VERIFIED |

---

## Key Link Verification

| From | To | Via | Status | Detail |
|------|----|----|--------|--------|
| `crates/chia-sdk-types/src/lib.rs` | `crates/chia-sdk-types/src/silent_payments/mod.rs` | `#[cfg(feature = "chip-0057")] pub mod silent_payments;` | WIRED | Verified at lib.rs:3-4 |
| `silent_payments/mod.rs` | `silent_payments/scalar.rs` | `mod scalar; pub use scalar::*;` | WIRED | mod.rs:13-14 |
| `silent_payments/mod.rs` | `silent_payments/tagged_hash.rs` | `mod tagged_hash; pub use tagged_hash::*;` | WIRED | mod.rs:15-16 |
| `silent_payments/mod.rs` | `silent_payments/paths.rs` | `mod paths; pub use paths::*;` | WIRED | mod.rs:11-12 |
| `silent_payments/scalar.rs` | `num_bigint::BigUint` | `use num_bigint::BigUint;` + `BigUint::from_bytes_be` calls | WIRED | scalar.rs:17 (import) + 4 call sites (lines 49, 50, 69, 70, 79, 80) |
| `silent_payments/tagged_hash.rs` | `chia_sha2::Sha256` | `use chia_sha2::Sha256;` + `Sha256::new()/update/finalize` calls | WIRED | tagged_hash.rs:12 (import) + call sites in `tagged_hash` (lines 30-39) and `sha256` test helper (lines 47-51) |
| `Cargo.toml` (root features) | `chia-sdk-types/Cargo.toml` | feature string `chia-sdk-types/chip-0057` | WIRED | Verified at root Cargo.toml line 75 |
| `Cargo.toml` (root features) | `chia-sdk-driver/Cargo.toml` | feature string `chia-sdk-driver/chip-0057` | WIRED | Verified at root Cargo.toml line 75 |
| `Cargo.toml` (root features) | `chia-sdk-utils/Cargo.toml` | feature string `chia-sdk-utils/chip-0057` | WIRED | Verified at root Cargo.toml line 75 |
| `chia-sdk-driver/Cargo.toml` | `chia-sdk-types/Cargo.toml` | feature cascade `chip-0057 = ["chia-sdk-types/chip-0057"]` | WIRED | Verified at driver Cargo.toml line 23 |
| `.github/workflows/rust.yml` | `crates/chia-sdk-types/Cargo.toml` | CI runs `cargo build --release -p chia-sdk-types -F chip-0057` | WIRED | rust.yml line 65 + verifier ran the same command locally, exit 0 |

---

## Data-Flow Trace (Level 4)

Phase 1 produces pure-library primitives (no rendering, no API responses, no UI). Level 4 is not applicable in the user-data-flow sense (no dynamic data sources). However, the equivalent check — does the implementation actually produce the bytes the protocol expects? — IS exercised by the 10 unit tests:

| Truth | Data Source | Produces Real Value | Status |
|-------|-------------|--------------------|--------|
| `ScalarField::from_bytes_unsigned` actually reduces mod r | `BigUint::from_bytes_be(&bytes) % BigUint::from_bytes_be(&GROUP_ORDER)` (scalar.rs:49-51) | Yes — `from_bytes_unsigned_max_input_reduces_to_r_minus_one` pins the actual reduced value `0x1824b159...fffffffd`; `mul_mod_r` and `add_wraps_at_r` exercise multiplicative and additive paths; `from_bytes_unsigned_identity` confirms in-range values round-trip; `from_bytes_raw_does_not_reduce` confirms the unchecked path does NOT reduce. | FLOWING |
| `tagged_hash` produces BIP-340-compatible output | `chia_sha2::Sha256::{new, update, finalize}` triple-update construction (tagged_hash.rs:30-39) | Yes — `tagged_hash_matches_bip340_challenge_vector` pins the published `c216d352...3713` value; `different_tags_different_outputs` confirms tag actually affects output. | FLOWING |
| Tag-string constants hash to pinned values | `CHIA_SP_INPUTS`/`CHIA_SP_SHARED_SECRET`/`CHIA_SP_LABEL` as `&'static str` + test-only `sha256` helper | Yes — three pinned tests pass against cross-verified SHA-256 values (Python `hashlib.sha256` + `sha256sum` CLI). | FLOWING |
| `SCAN_PATH` / `SPEND_PATH` carry the CHIP-0057 indices | `pub const &[u32] = &[12381, 8444, 12, 0]` / `&[..., 13, 0]` | Implementation matches CHIP-0057 §172-173 literally. Empty test surface here is intentional (Phase 2 validates these end-to-end against the published `b_scan = 132567e4...690f6` and `b_spend = 53d140b3...31b087` vectors). | STATIC (by design — validated downstream) |

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| chip-0057 feature gates module under no-features | `cargo build --release -p chia-sdk-types` (no features) | exit 0 (incremental); confirms `#[cfg(feature = "chip-0057")]` gate honored | PASS |
| chip-0057 feature compiles in isolation | `cargo build --release -p chia-sdk-types -F chip-0057` | exit 0 | PASS |
| Cumulative feature activation works | `cargo build --release -p chia-sdk-types --all-features` | exit 0 | PASS |
| Workspace builds without features | `cargo build --release --workspace` | exit 0 (2m 47s, full build) | PASS |
| Workspace builds with all features | `cargo build --release --workspace --all-features` | exit 0 (3m 01s, full build) | PASS |
| All 10 silent_payments unit tests pass | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` | 10 passed; 0 failed; 0 ignored | PASS |
| Formatting clean | `cargo fmt --all -- --files-with-diff --check` | exit 0 | PASS |
| Workspace clippy (as CI runs it) | `cargo clippy --workspace --all-features --all-targets` | exit 0 (2 pre-existing pedantic warnings in `chia-sdk-daemon` are warnings only; CI step does not pass `-D warnings`) | PASS |
| Scoped clippy strict | `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` | exit 0 | PASS |
| cargo-machete clean | `cargo machete` | exit 0 ("didn't find any unused dependencies") | PASS |

---

## Anti-Pattern Scan

Scanned all files modified in Phase 1 commits `79af49c8`, `27db3886`, `b74092c8`, `66c3dac9`, `b394ab5d`, `2fc7b9cc`, `ed8cd93c`, `d4b84439`.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | No TODO/FIXME/placeholder/stub/`return null`/empty-handler/hardcoded-empty/console.log-only patterns found in any Phase 1 source file. |

Also explicitly verified ABSENT (these would each be blocking if present):
- `impl From<[u8; 32]> for ScalarField` — ABSENT (verified by grep — type boundary intact)
- `use sha2::` in `silent_payments/` — ABSENT (verified by grep — chia-sha2-only convention intact)
- `Sha256::digest` in `silent_payments/` — ABSENT (verified by grep — uses `new/update/finalize` per chia-sha2 API)
- `mod_by_group_order` in `silent_payments/` — ABSENT (verified by grep — descriptive phrasing only)
- `unsafe` blocks in `silent_payments/` — ABSENT (workspace `unsafe_code = "deny"` enforced)
- Empty `chip-0057` cascade (cascade missing one of the three target crates) — NEGATIVE — all three are present in root `Cargo.toml:75`

---

## Decision on Known Carried Observations

### Observation 1: `r - 1` misnomer (test name + VALIDATION row 43 + ROADMAP SC2)

**Phase verifier disposition: ACCEPT (Option a) with a non-blocking docs-only follow-up recommendation.**

Rationale:
- The implementation is mathematically correct. The test pins the actual `(2^256 - 1) mod r` value AND asserts inequality with the raw input — both protocol-correctness invariants hold.
- The wording inaccuracy is confined to (a) the test function name, (b) the VALIDATION.md row 43, (c) the ROADMAP success criterion 2 prose, and (d) the comment header inside the test body that documents the discrepancy.
- The misnomer is documented prominently in the codebase: scalar.rs:122-129 carries a multi-line comment explaining why the name is wrong and what the test actually asserts. A reader cannot miss it.
- Renaming the test to `from_bytes_unsigned_max_input_reduces_to_pinned_value` (or similar) is a coordinated edit across at least 4 artifacts (VALIDATION.md, PLAN.md acceptance criteria, the test source, and ROADMAP success criterion 2). It is low-effort but spans multiple files, and the implementation does not change. Best handled as a small follow-up commit before Phase 2 entry.
- Gap-vs-pass call: This is a **documentation defect, not an implementation defect**. The protocol semantics the criterion targets are correctly enforced. PASS.

Recorded in `follow_ups_recommended.roadmap-sc2-rewording` and `follow_ups_recommended.validation-row-43-test-name`.

### Observation 2: Pre-existing `chia-sdk-daemon` pedantic warnings

**Phase verifier disposition: ACCEPT.**

Rationale:
- The warnings originate in upstream commit `ec1a3517` (Chia daemon websocket client), which lands before Phase 1 and is unrelated to silent payments.
- M4 in the verifier prompt is phrased "as CI runs it". The CI invocation at `.github/workflows/rust.yml:75` is `cargo clippy --workspace --all-features --all-targets` WITHOUT `-D warnings`. This invocation exits 0 (verifier confirmed via local rerun).
- The strict `-D warnings` form (specified in 01-05-PLAN.md as a plan-internal gate) blocks on these two warnings. The plan's executor correctly classified this as out-of-scope per the GSD scope-boundary rule and logged it to `deferred-items.md`.
- Scoped clippy on `chia-sdk-types` (the only crate Phase 1 touched) is clean under `-D warnings` (verifier confirmed).
- Gap-vs-pass call: Phase 1 is NOT responsible for upstream pre-existing lint warnings. M4 phrased "as CI runs it" PASSES. PASS.

Recorded in `follow_ups_recommended.daemon-pedantic-lints` for a follow-up small chore PR.

---

## New Findings Not Previously Surfaced

None. The phase executor's `01-PHASE-SUMMARY.md`, `01-05-SUMMARY.md`, and `deferred-items.md` accurately and comprehensively documented every observable defect, deviation, and out-of-scope discovery. The verifier's independent gate-command runs reproduced every claimed exit status. No undocumented issues surface.

The only verifier addition is the formal disposition of the two carried observations (above) — both ACCEPT, both with documented follow-up recommendations carrying explicit suggested fixes.

---

## Verification Summary

- **All 9 must-haves (M1..M9) PASS** with verifier-run command outputs.
- **All 5 ROADMAP success criteria PASS** substantively. SC2 and SC3 carry documented wording imprecisions in the ROADMAP text — the implementation is correct against the spirit of both criteria.
- **All 5 declared requirements PASS** with concrete code evidence. No orphaned requirements.
- **All 10 artifacts VERIFIED at levels 1-3** (exists, substantive, wired).
- **All 11 key links VERIFIED** as WIRED.
- **All 10 unit tests pass** (5 scalar + 5 tagged_hash).
- **No anti-patterns detected.** All defense-in-depth grep bans hold.
- **2 carried observations dispositioned as ACCEPT** with non-blocking docs-only follow-up recommendations.
- **0 new findings** that the executor didn't already surface.

**Phase 1 is structurally complete and ready for Phase 2 entry.**

---

*Verified: 2026-05-15*
*Verifier: Claude (gsd-verifier, Opus 4.7 1M)*
*Status: passed*
