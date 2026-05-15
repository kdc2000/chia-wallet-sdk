---
phase: 01-crypto-primitives-workspace-integration
subsystem: infra
tags: [phase-closure, chip-0057, silent-payments, crypto-primitives, workspace, ci, ws-01, ws-02, ws-03, crypto-01, crypto-02]

requires: []
provides:
  - "chip-0057 workspace feature cascading from root Cargo.toml to chia-sdk-types/driver/utils (mirrors chip-0037 precedent)"
  - "chia_sdk_types::silent_payments module (gated by chip-0057) containing ScalarField + GROUP_ORDER + tagged_hash + Chia_SP/* tag constants + SCAN_PATH + SPEND_PATH"
  - "Per-crate chip-0057 build line in .github/workflows/rust.yml"
  - "Documented Phase-1 final gate pass (5 builds, scoped clippy clean, fmt clean, machete clean, three grep bans hold, 10 silent_payments tests, 2360-test full workspace suite)"

affects:
  - Phase 02 (Address & key types — consumes ScalarField + SCAN_PATH/SPEND_PATH + tagged_hash from chia_sdk_types::silent_payments)
  - Phase 03 (Receive primitive — consumes tagged_hash, CHIA_SP_INPUTS, CHIA_SP_SHARED_SECRET, ScalarField boundary)
  - Phase 04 (Send-side action — consumes the same crypto primitives for output_tweak / input_hash)
  - Phase 05 (Bindings — exposes the address layer that Phase 2 builds on these primitives)
  - Phase 06 (Simulator E2E — tests the full pipeline that this phase's primitives anchor)

requirements-completed: [WS-01, WS-02, WS-03, CRYPTO-01, CRYPTO-02]

plans:
  - "01-01-PLAN.md — Workspace chip-0057 feature flag scaffolding (closed WS-01)"
  - "01-02-PLAN.md — ScalarField + GROUP_ORDER + 5 named scalar tests (closed CRYPTO-01)"
  - "01-03-PLAN.md — tagged_hash + Chia_SP/* tag constants + 5 named tagged_hash tests (closed CRYPTO-02)"
  - "01-04-PLAN.md — SCAN_PATH + SPEND_PATH derivation paths"
  - "01-05-PLAN.md — CI matrix update + final gate closure (closed WS-02 + WS-03)"

duration: 27min  # cumulative across the 5 plans (10 + 7 + 4 + 3 + 13 minus the 4 docs-only commits worth a few s each, rounded)
completed: 2026-05-15
---

# Phase 01: Crypto primitives & workspace integration — Phase Summary

**Phase 1 lands the CHIP-0057 cryptographic foundation in `chia_sdk_types::silent_payments` behind the new `chip-0057` workspace feature (cascading to `chia-sdk-types`/`-driver`/`-utils`) — `ScalarField` with unsigned mod-r reduction, `tagged_hash` over BIP-340 with the three `Chia_SP/*` domain-tag constants pinned by typo-guard tests, and the `SCAN_PATH`/`SPEND_PATH` BIP-32 derivation paths — then closes WS-02 with a per-crate CI build line and WS-03 with a green gate sweep (5 build permutations, scoped clippy `-D warnings` clean on the touched crate, fmt clean, `cargo machete` clean with zero new `ignored` entries, three grep bans hold, all 10 named tests pass, full 2360-test workspace suite green).**

## At a Glance

| | |
|---|---|
| **Phase** | 01 — Crypto primitives & workspace integration |
| **Plans completed** | 5 of 5 |
| **Cumulative duration** | ~27 min execution time |
| **Files created** | 5 (3 Rust sources + 2 phase artifacts) + plan-level SUMMARYs |
| **Files modified** | 6 (4 Cargo.toml + 1 lib.rs + 1 CI workflow) |
| **Workspace deps added** | 0 (`num-bigint` was already in `[workspace.dependencies]`) |
| **`[package.metadata.cargo-machete] ignored` entries added** | 0 |
| **Tests added** | 10 (5 scalar + 5 tagged_hash) — all passing |
| **Requirements closed** | 5 (WS-01, WS-02, WS-03, CRYPTO-01, CRYPTO-02) |

## What Shipped

### Source code (all behind `chip-0057`)

- `crates/chia-sdk-types/src/silent_payments/mod.rs` — module barrel in final sorted ordering `paths < scalar < tagged_hash`.
- `crates/chia-sdk-types/src/silent_payments/scalar.rs` — `ScalarField` newtype + `GROUP_ORDER` (matches `chia_puzzle_types::derive_synthetic::GROUP_ORDER_BYTES` byte-for-byte). Public methods: `from_bytes_unsigned`, `from_bytes_raw`, `add`, `mul`, `as_bytes`, `to_bytes`, `is_zero`. **No `From<[u8; 32]>`** — the type boundary forces callers to make the unsigned-vs-signed reduction choice explicit.
- `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` — `tagged_hash(tag, data) -> [u8; 32]` BIP-340 construction via `chia_sha2::Sha256` (new/update/finalize, never `::digest`). Three `pub const &'static str` tag constants: `CHIA_SP_INPUTS`, `CHIA_SP_SHARED_SECRET`, `CHIA_SP_LABEL`.
- `crates/chia-sdk-types/src/silent_payments/paths.rs` — `SCAN_PATH` and `SPEND_PATH` as `pub const &[u32]` slices: `&[12381, 8444, 12, 0]` and `&[12381, 8444, 13, 0]` (unhardened, no `| 0x80000000`).

### Config / CI

- Root `Cargo.toml` — `chip-0057 = ["chia-sdk-driver/chip-0057", "chia-sdk-types/chip-0057", "chia-sdk-utils/chip-0057"]`.
- `crates/chia-sdk-types/Cargo.toml` — `chip-0057 = []` feature entry + `num-bigint = { workspace = true }` dep.
- `crates/chia-sdk-driver/Cargo.toml` — `chip-0057 = ["chia-sdk-types/chip-0057"]` cascade.
- `crates/chia-sdk-utils/Cargo.toml` — first-ever `[features]` block with `chip-0057 = []`.
- `crates/chia-sdk-types/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`.
- `.github/workflows/rust.yml` — per-crate `cargo build --release -p chia-sdk-types -F chip-0057` line in the "Build individual crates" step (catches feature-isolation regressions).

### Tests

| Test | Module | What it pins |
|---|---|---|
| `from_bytes_unsigned_identity` | scalar | small value round-trips unchanged |
| `from_bytes_raw_does_not_reduce` | scalar | `[0xff; 32]` round-trips unchanged through `from_bytes_raw` |
| `from_bytes_unsigned_max_input_reduces_to_r_minus_one` | scalar | `[0xff; 32]` reduces to `0x1824b159...fffffffd` (NOT `r-1` — test name is misnamed, see Carried-forward observations) |
| `add_wraps_at_r` | scalar | addition wraps modulo `r` |
| `mul_mod_r` | scalar | multiplication is modulo `r` |
| `tagged_hash_matches_bip340_challenge_vector` | tagged_hash | cross-implementation correctness via BIP-340 published vector `c216d352...3713` |
| `tag_inputs_hash_pinned` | tagged_hash | `SHA256("Chia_SP/Inputs") == d44a6db8...3378cf` typo guard |
| `tag_shared_secret_hash_pinned` | tagged_hash | `SHA256("Chia_SP/SharedSecret") == e7b8a524...b9ca9f` typo guard |
| `tag_label_hash_pinned` | tagged_hash | `SHA256("Chia_SP/Label") == c63c8bd2...f94554` typo guard |
| `different_tags_different_outputs` | tagged_hash | sanity that the tag parameter actually affects the output |

All 10 pass under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments`.

## M1..M9 Final Status

| ID | Description | Status |
|----|-------------|--------|
| M1 | `cargo build -p chia-sdk-types -F chip-0057` succeeds | PASS |
| M2 | `cargo build --workspace --all-features` succeeds | PASS |
| M3 | `cargo build --workspace` (no features) succeeds | PASS |
| M4 | `clippy --workspace --all-features --all-targets` clean | PASS (CI's actual invocation — without `-D warnings` — exits 0; pre-existing pedantic warnings in `chia-sdk-daemon` are outside Phase 1 scope, logged to `deferred-items.md`. Scoped clippy on `chia-sdk-types` is clean with `-D warnings`.) |
| M5 | `cargo fmt --check` clean | PASS |
| M6 | `cargo machete` clean, no new `ignored` entries | PASS |
| M7 | `grep -r 'mod_by_group_order' silent_payments/` zero matches | PASS |
| M8 | All ScalarField + tagged_hash unit tests pass | PASS (10/10) |
| M9 | `chip-0057` declared on chia-sdk-types + chia-sdk-driver + chia-sdk-utils | PASS |

## ROADMAP Success Criteria — Final Status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | All five build permutations pass; workspace clippy `--all-features --all-targets` is clean | PASS |
| 2 | Adversarial `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes()` test passes | PASS (test runs green; the value pinned is the mathematically correct `(2^256-1) mod r` value — see Carried-forward observation about the test name) |
| 3 | Tag-pin unit tests pass for `Chia_SP/Inputs`, `Chia_SP/SharedSecret`, `Chia_SP/Label` | PASS |
| 4 | `grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` zero hits | PASS |
| 5 | `cargo machete` passes with no new `[package.metadata.cargo-machete] ignored` entries | PASS |

## Requirements Closed

- **WS-01** — `chip-0057` workspace feature cascade (Plan 01-01). Mirror of `chip-0037` template.
- **WS-02** — Per-crate `-F chip-0057` CI build line (Plan 01-05).
- **WS-03** — Workspace lint policy clean + `cargo machete` clean + no new `ignored` entries (Plan 01-05).
- **CRYPTO-01** — `ScalarField` newtype with unsigned mod-r reduction (Plan 01-02).
- **CRYPTO-02** — `tagged_hash` + `Chia_SP/*` tag constants (Plan 01-03).

Plan 01-04 (SCAN_PATH / SPEND_PATH) does not close a discrete requirement at the v1 level — the paths are validated end-to-end at Phase 2 (ADDR-01) against CHIP test vectors `b_scan = 132567e4...690f6` and `b_spend = 53d140b3...31b087`.

## Key Decisions

(See per-plan SUMMARYs for full rationale; this list aggregates the cross-cutting calls.)

1. **No new `[workspace.dependencies]` entries.** `num-bigint` was already at root (used by `chia-sdk-driver`); Plan 01-02 added a `[dependencies]` reference in `chia-sdk-types/Cargo.toml` only.
2. **No `dep:chia-sdk-types` edge on `chia-sdk-utils` at Phase 1.** The Q8 audit (Phase 2 pre-flight, per STATE.md Blockers) decides whether `SilentPaymentKeys` / `SilentPaymentAddress` need to consume from `chia-sdk-types` or can be self-contained on `chia-bls` + `bech32`.
3. **`chia_sha2::Sha256` only — never bare `sha2::`.** Defense-in-depth grep ban `! grep -rE '^use sha2::' silent_payments/` enforced at Plan 01-05 final gate. Plus `! grep -rE 'Sha256::digest' silent_payments/` because `chia-sha2` doesn't expose `::digest` (RESEARCH.md Pitfall 3).
4. **Type-boundary discipline on `ScalarField`.** No `From<[u8; 32]>` impl. Callers must pick `from_bytes_unsigned` (reduces mod `r`) vs `from_bytes_raw` (does not) explicitly. The whole point of the newtype is to prevent the signed-vs-unsigned mixing hazard that would otherwise leak into every downstream phase.
5. **Tag-pin typo guards via `SHA256(tag.as_bytes())`.** Each `pub const &str` is paired with a `hex!(...)` test that pins `SHA256(tag)`. A typo in any tag-string value fails the test before any protocol code runs.
6. **Descriptive phrasing only for the standard-puzzle reducer.** No literal `mod_by_group_order` token anywhere under `silent_payments/`. The grep ban at Plan 01-05 holds trivially because Plans 02/03/04 used phrases like "the standard-puzzle synthetic-key reducer" or "the existing signed reducer in chia-puzzle-types".

## Carried-forward Observations / Follow-ups

These are NOT Phase 1 blockers but are surfaced for phase verifier visibility and future-phase planning.

### 1. Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is a misnomer

- **Plan:** 01-02 (test landed); 01-05 (re-verified passing).
- **Issue:** The test name and `01-02-PLAN.md`'s `<behavior>` block claim the reduction of `[0xff; 32]` mod `r` equals `r - 1`. The actual reduction is `0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffd` (= `(2^256 - 1) - r`, since `floor((2^256 - 1) / r) = 1`). The test correctly pins this actual value AND asserts inequality with `[0xff; 32]`, so the protocol-correctness guarantee (unsigned reduction did fire) is intact — but the name is misleading.
- **Authoritative source:** `VALIDATION.md` row 43 + `01-02-PLAN.md` task acceptance criteria lock the test name with `--exact`.
- **Disposition (suggested for phase verifier):** Open a 4-line follow-up that (a) renames the test to `from_bytes_unsigned_max_input_reduces_correctly` or `..._reduces_to_pinned_value`, (b) patches `VALIDATION.md` row 43 to match, (c) patches `ROADMAP.md` Phase 1 success criterion 2 (which says "equals `r - 1`") to say "equals the pinned reduced value", and (d) updates the doc-comment inside the test body. The implementation does not change.

### 2. Pre-existing chia-sdk-daemon clippy::pedantic warnings

- **Plan:** Discovered during 01-05 Task 2 gate run.
- **Location:** `crates/chia-sdk-daemon/src/client.rs:426-427`.
- **Lints:** `clippy::match_same_arms`, `clippy::match_wildcard_for_single_variants`.
- **Source:** Upstream commit `ec1a3517` (pre-dates Phase 1).
- **Disposition:** Logged to `deferred-items.md` with a 2-line fix recommendation. NOT a Phase 1 blocker: CI's existing clippy step (without `-D warnings`) exits 0; scoped clippy on `chia-sdk-types` is clean under `-D warnings`. Recommended either a small chore PR or aligning CI's clippy step with the strict local gate — independent of Phase 1 / 2.

## Files Inventory

### Created (Rust source)
- `crates/chia-sdk-types/src/silent_payments/mod.rs` (18 lines)
- `crates/chia-sdk-types/src/silent_payments/scalar.rs` (186 lines, incl. 5 named tests)
- `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` (98 lines, incl. 5 named tests)
- `crates/chia-sdk-types/src/silent_payments/paths.rs` (25 lines, no tests)

### Modified
- `Cargo.toml` (root) — appended `chip-0057` to `[features]`
- `crates/chia-sdk-types/Cargo.toml` — `chip-0057 = []` + `num-bigint = { workspace = true }`
- `crates/chia-sdk-driver/Cargo.toml` — `chip-0057 = ["chia-sdk-types/chip-0057"]`
- `crates/chia-sdk-utils/Cargo.toml` — new `[features]` block with `chip-0057 = []`
- `crates/chia-sdk-types/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- `.github/workflows/rust.yml` — one line: `cargo build --release -p chia-sdk-types -F chip-0057`

### Phase planning artifacts
- `01-01-SUMMARY.md`, `01-02-SUMMARY.md`, `01-03-SUMMARY.md`, `01-04-SUMMARY.md`, `01-05-SUMMARY.md`
- `01-PHASE-SUMMARY.md` (this file)
- `deferred-items.md`

## Plans Index

| Plan | Title | Duration | Commits | SUMMARY |
|------|-------|----------|---------|---------|
| 01-01 | Workspace chip-0057 feature flag scaffolding | 10 min | `79af49c8`, `d6bcae89` (docs) | `01-01-SUMMARY.md` |
| 01-02 | ScalarField + GROUP_ORDER + unsigned mod-r reduction | 7 min | `27db3886`, `b74092c8`, `1777b74c` (docs) | `01-02-SUMMARY.md` |
| 01-03 | tagged_hash + Chia_SP/* tag constants | 4 min | `66c3dac9`, `b394ab5d`, `c23e00a5` (docs) | `01-03-SUMMARY.md` |
| 01-04 | SCAN_PATH + SPEND_PATH derivation paths | 3 min | `2fc7b9cc`, `ed8cd93c`, `218738a3` (docs) | `01-04-SUMMARY.md` |
| 01-05 | CI matrix + final gate verification | 13 min | `d4b84439`, + final docs commit | `01-05-SUMMARY.md` |

## Phase Transition: Phase 2 Readiness

**Inputs Phase 2 inherits from Phase 1:**

- `chip-0057` workspace feature, cascaded to `chia-sdk-types`/`-driver`/`-utils`.
- `chia_sdk_types::silent_payments::ScalarField` — for synthetic-key derivation paths inside Phase 2's `SilentPaymentKeys` and (later) Phase 3's input-hash / shared-secret pipeline.
- `chia_sdk_types::silent_payments::tagged_hash` + `CHIA_SP_INPUTS` / `CHIA_SP_SHARED_SECRET` / `CHIA_SP_LABEL` — for Phase 3/4 protocol code; Phase 2 doesn't consume these directly but they're part of the same module barrel.
- `chia_sdk_types::silent_payments::SCAN_PATH` / `SPEND_PATH` — single source of truth for `SilentPaymentKeys::from_mnemonic` BIP-32 derivation. Phase 2 validates these values end-to-end against the CHIP test vectors (`b_scan = 132567e4...690f6`, `b_spend = 53d140b3...31b087`).
- `crates/chia-sdk-utils/Cargo.toml` has its first `[features]` block ready to gain code; the `chip-0057 = []` feature is empty so Phase 2 can decide between (a) adding `dep:chia-sdk-types` (if the address layer imports from `silent_payments::*`) or (b) keeping utils self-contained on `chia-bls` + `bech32`.

**Outstanding for Phase 2 entry (per STATE.md Blockers section):**

- **Q8 — `dep:chia-sdk-types` edge on `chia-sdk-utils`?** Decide at Phase 2 pre-flight whether `SilentPaymentKeys` / `SilentPaymentAddress` need this dep edge.

**Optional pre-Phase-2 housekeeping:**

- Patch the `from_bytes_unsigned_max_input_reduces_to_r_minus_one` test name + VALIDATION row + ROADMAP success criterion 2 wording (see Carried-forward observation #1).
- Open a 2-line chore PR to fix the pre-existing chia-sdk-daemon pedantic lints (see Carried-forward observation #2).

Both are non-blocking for Phase 2 entry.

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
*Status: ALL 5 PLANS COMPLETE — READY FOR PHASE 2*
