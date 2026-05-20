# Phase 8: Second-pass v1 polish — tighten SP module surface and dispatch ergonomics - Research

**Researched:** 2026-05-20
**Domain:** Pure-refactor — Rust module reorganization, public-surface tightening, single-`match` dispatch cleanup. No new dependencies, no behavior change.
**Confidence:** HIGH — CONTEXT.md locks byte-precise targets; current-state inventory below verifies every claim against the actual source.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### POLISH-01 — Tighten `silent_payments/mod.rs` re-exports (D-01)
**Switch wildcards to named `pub use`** in `crates/chia-sdk-driver/src/silent_payments/mod.rs`. After this phase's POLISH-02 fold, the mod.rs body becomes:

```rust
mod protocol;
pub use protocol::{
    aggregate_sender_sks,
    compute_input_hash,
    compute_shared_secret_from_tweak,
    derive_one_time_puzzle_hash,
    derive_onetime_pk,
    derive_onetime_sk,
    derive_output_tweak,
    puzzle_hash_for_pk,
};
mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};
mod send_keys;
pub(crate) use send_keys::*;
mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
```

**Why named not pub(crate):** the protocol primitives are documented as public reusable surface in `silent_payments/mod.rs:9-13` rustdoc AND listed in `src/prelude.rs:43-44`. Demoting them is a documented-API break.

**`pub(crate) use send_keys::*` is correct as-is** — module contains only crate-private items.

**Acceptance grep:** `grep -nE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returns 0 hits.

#### POLISH-02 — Fold single-function modules into `protocol.rs` (D-02)
**Fold `aggregate.rs` + `input_hash.rs` + `one_time.rs` into `crates/chia-sdk-driver/src/silent_payments/protocol.rs`.**

- **Delete:** `aggregate.rs` (91 lines), `input_hash.rs` (157 lines), `one_time.rs` (193 lines)
- **Grow:** `protocol.rs` from 199 → ~640 lines (under scanner.rs's 735 ceiling)
- **Update:** `mod.rs` drops 3 `mod` declarations + 3 `pub use` lines

Test consolidation: merge inline `#[cfg(test)] mod tests {}` blocks. Planner picks (a) flat `mod tests {}` or (b) `mod tests { mod aggregate_tests; mod input_hash_tests; mod one_time_tests; }`. Test names + assertions stay byte-identical.

Imports inside protocol.rs after fold: protocol primitives are siblings — no `use crate::silent_payments::{...}` for them. `aggregate_sender_sks` + `compute_input_hash` become file-local.

External callsites unaffected — all import via `silent_payments::*` or named imports from the same module path; `pub use` in mod.rs republishes from protocol.rs.

**Why protocol.rs (not new send.rs):** scanner.rs already imports protocol primitives at `scanner.rs:30-33`. Folding co-locates "primitives + their composers"; a separate send.rs would split related code.

**Acceptance:**
- `test ! -f` for all 3 deleted files
- `wc -l protocol.rs` ≤ 700 (target ~640)
- Public test count unchanged

#### POLISH-03 — De-duplicate `SendDestination` Boxing rationale (D-03)
**Keep the variant-level rustdoc; delete the enum-level Boxing/Copy mention** in `crates/chia-sdk-driver/src/action_system/send_destination.rs`:

- **Delete** lines 19-23 (the `/// Cannot derive Copy because SilentPaymentAddress is Clone-only.` line and the surrounding `/// impl From<Bytes32>` explanation if grouped; CONTEXT.md specifies lines 21-23 — the line containing `Cannot derive Copy`).
- **Keep** lines 33-41 (the variant-level rustdoc explaining boxing, directly above `Box<SilentPaymentAddress>`).

Note: actual current source has the line at `send_destination.rs:19` (`/// Cannot derive Copy because SilentPaymentAddress is Clone-only.`), surrounded by blank `///` lines at 18 and 20. CONTEXT.md's "lines 21-23" matches the pre-Phase-7 source; the live file's offending paragraph is line 19. Planner verifies current line number at execution time.

**Net:** ~3 fewer lines of doc; no information loss; canonical location preserved.

**Acceptance:**
- `grep -c 'large_enum_variant' send_destination.rs` returns exactly 1
- `grep -c 'Cannot derive .Copy.' send_destination.rs` returns 0

#### POLISH-04 — Restructure `Action::send` chip-0057 dispatch (D-04)
**Single exhaustive `match` with SP arm `return`-ing from inside.** Rewrite `crates/chia-sdk-driver/src/actions/send.rs:44-62`:

```rust
let puzzle_hash = match &self.destination {
    SendDestination::PuzzleHash(ph) => *ph,
    #[cfg(feature = "chip-0057")]
    SendDestination::SilentPayment(addr) => {
        return crate::actions::silent_payment_send::handle_silent_payment_send(
            ctx, spends, &self.id, addr, self.amount, self.memos,
        );
    }
};
```

**Net:** the early-return `if let SendDestination::SilentPayment(addr) = &self.destination` block (lines 44-54) AND the post-match `unreachable!("handled above")` arm (lines 60-61) AND the explanatory comments at lines 41-43 + 56-58 all collapse into the single match above. ~8-10 source-line reduction. Behavior unchanged.

**Acceptance:**
- `grep -c 'unreachable!' send.rs` returns 0
- `grep -c 'handle_silent_payment_send' send.rs` returns 1

### Claude's Discretion

1. **Test-module merger shape in POLISH-02.** Planner picks (a) one flat `mod tests {}` at the bottom of protocol.rs with all merged test fns; (b) `mod tests { mod aggregate_tests; mod input_hash_tests; mod one_time_tests; }` preserving the original groupings as sub-modules.

2. **Whether to delete the now-dead `Memos` re-import in `silent_payments/protocol.rs`** after the fold — n/a, none of the four files imports `Memos`. The relevant dedup is `ScalarField`, `SecretKey`, `PublicKey`, `Bytes32` (see Current State Inventory below).

3. **Whether to update `silent_payments/mod.rs` rustdoc** to reflect the smaller module count. Current rustdoc references primitives by symbol, not by sub-file. Optional small edit.

4. **Phase ordering and wave assignment.** Suggested ordering: POLISH-01 → POLISH-02 → POLISH-03 → POLISH-04. POLISH-01/02 are serially dependent. POLISH-03 and POLISH-04 are independent of POLISH-01/02 and of each other.

### Deferred Ideas (OUT OF SCOPE)

- **POLISH-05** (extract `scanner.rs` inline tests) — DROPPED. Repo convention is inline tests; same reasoning as Phase 7's CLEANUP-05 drop.
- **Demote protocol primitives to `pub(crate)`** — rejected (public-API commitment in mod.rs rustdoc + prelude).
- **Extract `dispatch_puzzle_hash_target` helper in `send.rs`** — rejected as larger diff than nit warrants.
- **Update mod.rs rustdoc to reference fewer sub-modules** — left to Claude's Discretion.
- **Sage / external consumer audit** — not part of Phase 8 (no public-API break expected).
- **Other v1.1 polish items** — go to v1.1 backlog; Phase 8 only addresses the 4 explicit concerns.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POLISH-01 | `silent_payments/mod.rs` contains zero `pub use foo::*;` wildcard re-exports for public modules; all public re-exports list symbols explicitly. `pub(crate) use send_keys::*;` is permitted. Acceptance grep `^pub use [a-z_]+::\*;` returns 0. | Current state confirms 5 wildcards in mod.rs (lines 32, 34, 36, 38, 40, 44 — see baseline grep). Locked decision (D-01) supplies the exact post-edit body. External callsite audit (see "External Callsite Audit") shows all consumers reach symbols via named imports from `silent_payments::{...}` or `silent_payments::*` — both still resolve after switching mod.rs wildcards to named lists. |
| POLISH-02 | Three single-`pub fn` modules (aggregate.rs/input_hash.rs/one_time.rs) folded into protocol.rs. The three source files deleted. `wc -l protocol.rs` ≤ 700 (target ~640). All external callsites resolve identically; chip-0057 test count unchanged. | Confirmed 91 + 157 + 193 lines for the three deletion targets (CONTEXT.md numbers match). protocol.rs at 199 lines today. Naive sum 199+91+157+193 = 640 — matches the target. Test functions enumerated below (6 tests across the 3 deletion files + 2 in protocol.rs = 8 tests post-fold). Import overlap catalogued; only one non-trivial dep needs `use super::{...}` removed (one_time.rs's `use crate::silent_payments::{...}` becomes file-local). |
| POLISH-03 | `Box<SilentPaymentAddress>` rationale lives in exactly one location (variant-level rustdoc above `Box<SilentPaymentAddress>`). Enum-level "Cannot derive Copy" paragraph removed. Acceptance greps: `Cannot derive .Copy.` returns 0; `large_enum_variant` returns exactly 1. | Current file at 50 lines; current `Cannot derive .Copy.` grep = 1; current `large_enum_variant` grep = 1. The "Cannot derive Copy" sentence is at `send_destination.rs:19` (single line). Variant-level rustdoc spans lines 33-41 and includes `large_enum_variant` at line 34. Surgical delete: drop lines 18-20 (the blank `///` before, the sentence, the blank `///` after) — or the planner picks the minimal slice that leaves `large_enum_variant` intact in the variant doc. |
| POLISH-04 | `actions/send.rs` chip-0057 dispatch is a single exhaustive `match` where the `SilentPayment` arm calls `handle_silent_payment_send(...)` and `return`s. The `unreachable!("handled above")` arm is removed. Acceptance: `unreachable!` returns 0; `handle_silent_payment_send` returns 1. | Current send.rs at 399 lines; current `unreachable!` grep = 2 (line 62 is the offending arm; one more occurs elsewhere in the file — see Risk R4); current `handle_silent_payment_send` grep = 1 (the if-let early-return call at line 46). The current shape spans lines 41-63 (comments + if-let + post-match `unreachable!`); the locked target replaces it with a single 10-ish-line match block. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Authoritative project rules the planner MUST honor:

| Constraint | Source | Enforcement |
|------------|--------|-------------|
| Rust toolchain pinned to **1.90.0**, edition **2024** | `rust-toolchain.toml` | `cargo --version` |
| Workspace lints: `deny clippy::all`, `warn clippy::pedantic`, `warn clippy::cargo`, `deny unsafe_code`, `deny dead_code` | `Cargo.toml [workspace.lints]` | `cargo clippy --workspace --all-features --all-targets` |
| No new workspace deps, no bumping `chia-protocol@0.36.1` / `chia-puzzles@0.20.3` | `Cargo.toml [workspace.dependencies]` | `git diff Cargo.toml` shows no new entries; `cargo machete` clean |
| `cargo fmt --all --check` MUST pass | CI gate | run after every edit |
| `cargo machete` MUST be clean — no new `[package.metadata.cargo-machete] ignored` entries | CI gate | run before phase complete |
| Per-crate `cargo build` with each feature MUST pass | CI matrix | `cargo build --release -p chia-sdk-driver --features chip-0057` and `--no-default-features` |
| No `unsafe` blocks anywhere | `unsafe_code = "deny"` | clippy enforces |
| All `chip-0057`-gated code MUST compile under all CI permutations | WS-03 | per-crate + workspace `--all-features` build |
| Public-API stability: prelude re-export list at `src/prelude.rs:41-45` MUST remain byte-identical after Phase 8 | locked decision (D-01) | `git diff src/prelude.rs` shows no changes outside ordering of the 12 SP names |
| Comments contain NO references to GSD planning artifacts | CLEANUP-01 (Phase 7 closed) | `grep -rE 'CONTEXT\.md|RESEARCH(\.md)?|Plan 0[1-9]-|\bD-0[1-9]\b|Pitfall [0-9]|Pattern [0-9]'` returns 0 across all SP files |
| Workflow gate: use a GSD command for ALL repo-changing work | CLAUDE.md `## GSD Workflow Enforcement` | Phase 8 is itself GSD-driven |

## Summary

Phase 8 is a pure-refactor "second-pass polish" mirroring Phase 7's shape: 4 surgical changes to the chip-0057 silent-payments code with byte-precise acceptance oracles. CONTEXT.md locks every decision and pins exact post-edit shapes for POLISH-01 (mod.rs body), POLISH-02 (file deletions + ≤700-line protocol.rs target), POLISH-03 (deletion target lines + grep counts), and POLISH-04 (the exact replacement `match` block). No behavior change, no API removals — the `chia_sdk_driver::silent_payments::*` and `chia_wallet_sdk::prelude::*` surfaces remain byte-identical for downstream callers.

The current-state inventory (file-by-file below) confirms every line count, every wildcard, every `unreachable!` call site, and every external callsite path CONTEXT.md references. Test names across the three POLISH-02 deletion targets are enumerated (6 tests total: 1 in aggregate.rs, 3 in input_hash.rs, 2 in one_time.rs) — all carry unique names and merge cleanly into protocol.rs's existing `mod tests {}` block. Import deduplication after fold is mechanical (drop `use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk};` from the absorbed `one_time.rs` body — they're siblings in protocol.rs).

**Primary recommendation:** Plan a 4-wave linear sequence — Wave 1 POLISH-01 (mod.rs named exports, smallest blast radius), Wave 2 POLISH-02 (the fold; depends on Wave 1's named-import set being the post-fold authority), Wave 3 POLISH-03 + POLISH-04 in parallel (independent, both surgical). Validation Architecture is grep + wc + `cargo build` + `cargo test --release -p chia-sdk-driver --features chip-0057` test-count parity — same Dimension-8 shape Phase 7 used.

## Current State Inventory

### Verified file sizes (matches CONTEXT.md exactly)

| File | Lines | Role |
|------|-------|------|
| `crates/chia-sdk-driver/src/silent_payments/mod.rs` | **44** | POLISH-01 target |
| `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` | **91** | POLISH-02 deletion target |
| `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` | **157** | POLISH-02 deletion target |
| `crates/chia-sdk-driver/src/silent_payments/one_time.rs` | **193** | POLISH-02 deletion target |
| `crates/chia-sdk-driver/src/silent_payments/protocol.rs` | **199** | POLISH-02 fold target → ~640 |
| `crates/chia-sdk-driver/src/silent_payments/scanner.rs` | 735 | unchanged (in-module ceiling reference) |
| `crates/chia-sdk-driver/src/silent_payments/types.rs` | 83 | unchanged |
| `crates/chia-sdk-driver/src/silent_payments/send_keys.rs` | 217 | unchanged |
| `crates/chia-sdk-driver/src/actions/send.rs` | **399** | POLISH-04 target |
| `crates/chia-sdk-driver/src/actions/silent_payment_send.rs` | 711 | unchanged (callee) |
| `crates/chia-sdk-driver/src/action_system/send_destination.rs` | **50** | POLISH-03 target |

Naive concatenation: `199 + 91 + 157 + 193 = 640` → exactly the CONTEXT.md target. Import dedup reduces this by ~3-5 lines net (see "Import dedup" below); test merger costs nothing structurally. Final protocol.rs lands in the 630-645 range — comfortably under the 700-line acceptance ceiling.

### Baseline acceptance-grep oracles (pre-phase)

```
$ grep -nE '^pub use [a-z_]+::\*;' silent_payments/mod.rs
32:pub use aggregate::*;
34:pub use input_hash::*;
36:pub use one_time::*;
38:pub use protocol::*;
40:pub use scanner::*;
44:pub use types::*;
                                          # POLISH-01 acceptance: 0 (post-phase)
                                          # Pre-phase: 6 hits

$ grep -c 'unreachable!' actions/send.rs
2                                         # POLISH-04 acceptance: 0
                                          # NOTE: 2 hits, not 1 — one at line 62
                                          # (the offending arm); investigate the
                                          # second site as Risk R4 below.

$ grep -c 'handle_silent_payment_send' actions/send.rs
1                                         # POLISH-04 acceptance: 1
                                          # Pre-phase already 1 (the if-let
                                          # early-return at line 46). The match
                                          # restructure keeps this count at 1.

$ grep -c 'large_enum_variant' action_system/send_destination.rs
1                                         # POLISH-03 acceptance: exactly 1
                                          # (preserved in variant-level rustdoc)

$ grep -c 'Cannot derive .Copy.' action_system/send_destination.rs
1                                         # POLISH-03 acceptance: 0
```

### POLISH-01 target — `silent_payments/mod.rs` (44 lines, today)

Current body (lines 31-44):
```rust
mod aggregate;
pub use aggregate::*;
mod input_hash;
pub use input_hash::*;
mod one_time;
pub use one_time::*;
mod protocol;
pub use protocol::*;
mod scanner;
pub use scanner::*;
mod send_keys;
pub(crate) use send_keys::*;
mod types;
pub use types::*;
```

After Phase 8 (POLISH-01 + POLISH-02 combined, per D-01's locked body):
```rust
mod protocol;
pub use protocol::{
    aggregate_sender_sks,
    compute_input_hash,
    compute_shared_secret_from_tweak,
    derive_one_time_puzzle_hash,
    derive_onetime_pk,
    derive_onetime_sk,
    derive_output_tweak,
    puzzle_hash_for_pk,
};
mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};
mod send_keys;
pub(crate) use send_keys::*;
mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
```

Module-level rustdoc (lines 1-30) stays. Optional Claude's-discretion edit: lines 9-13 reference 5 protocol primitives by symbol — those references still resolve post-fold. Lines 8-13 do NOT claim the primitives live in separate sub-files, so no rustdoc edit is required.

### POLISH-02 deletion targets — file-by-file content map

#### `aggregate.rs` (91 lines)

| Line range | Content |
|------------|---------|
| 1-24 | Module-level rustdoc (`//!`) — privacy warning + synthetic-vs-raw boundary |
| 26-27 | `use chia_bls::SecretKey;` `use chia_sdk_types::silent_payments::ScalarField;` |
| 29-47 | Function rustdoc for `aggregate_sender_sks` |
| 48-56 | `pub fn aggregate_sender_sks(sks: &[SecretKey]) -> ScalarField` (body) |
| 58 | `#[cfg(test)]` |
| 59-91 | `mod tests { ... }` containing **1 test**: `tv4_aggregate_sender_sks_matches` (line 79) — TV4 byte-pin, hex-literal of `TV4_AGGREGATED_SK = 0x5600d878...cbf95b89`. Test-block imports: `use super::*; use hex_literal::hex;` |

**Constants inside test block:** `TV4_SENDER_SK_0`, `TV4_SENDER_SK_1`, `TV4_AGGREGATED_SK` (all `[u8; 32]`).

#### `input_hash.rs` (157 lines)

| Line range | Content |
|------------|---------|
| 1-28 | Module-level rustdoc (`//!`) — input-hash semantics + AssertConcurrent reference |
| 29-31 | `use chia_bls::PublicKey;` `use chia_protocol::Bytes32;` `use chia_sdk_types::silent_payments::{CHIA_SP_INPUTS, ScalarField, tagged_hash};` |
| 33-55 | Function rustdoc for `compute_input_hash` |
| 56-72 | `pub fn compute_input_hash(coin_ids: &[Bytes32], aggregated_sender_pk: &PublicKey) -> ScalarField` |
| 74 | `#[cfg(test)]` |
| 75-157 | `mod tests { ... }` containing **3 tests**: `tv1_compute_input_hash_matches` (line 102), `input_hash_uses_lex_min_coin_id` (line 120), `input_hash_order_independent` (line 142). Test-block imports: `use super::*; use hex_literal::hex;` |

**Constants inside test block:** `TV1_INPUT_HASH`, `TV1_COIN_ID`, `TV1_A_SUM`.

#### `one_time.rs` (193 lines)

| Line range | Content |
|------------|---------|
| 1-30 | Module-level rustdoc (`//!`) — derivation pipeline + privacy warning |
| 31-34 | `use chia_bls::PublicKey;` `use chia_protocol::Bytes32;` `use chia_sdk_types::silent_payments::ScalarField;` `use chia_sha2::Sha256;` |
| 36 | `use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk};` ← **becomes file-local after fold (drop)** |
| 38-60 | Function rustdoc for `derive_one_time_puzzle_hash` |
| 61-89 | `pub fn derive_one_time_puzzle_hash(...) -> Bytes32` (5-step body) |
| 91 | `#[cfg(test)]` |
| 92-193 | `mod tests { ... }` containing **2 tests**: `tv1_derive_one_time_puzzle_hash_matches` (line 129), `derive_one_time_puzzle_hash_k1_round_trip` (line 156). Test-block imports: `use super::*; use chia_bls::SecretKey; use hex_literal::hex;`. The k1 test ALSO contains `use crate::silent_payments::compute_shared_secret_from_tweak;` inline at line 157 — **becomes `use super::compute_shared_secret_from_tweak;` (or trivially via `use super::*;`) after fold**. |

**Constants inside test block:** `TV1_SCAN_SK`, `TV1_SCAN_PK`, `TV1_SPEND_PK`, `TV1_AGGREGATED_SENDER_SK`, `TV1_INPUT_HASH`, `TV1_PUZZLE_HASH`.

#### `protocol.rs` (199 lines, fold target)

| Line range | Content |
|------------|---------|
| 1-15 | Module-level rustdoc (`//!`) — five primitives + ScalarField boundary note. **After fold: extend to describe the 3 absorbed compositions.** |
| 17-22 | `use chia_bls::{PublicKey, SecretKey};` `use chia_protocol::Bytes32;` `use chia_puzzle_types::DeriveSynthetic;` `use chia_puzzle_types::standard::StandardArgs;` `use chia_sdk_types::silent_payments::{CHIA_SP_SHARED_SECRET, ScalarField, tagged_hash};` `use chia_sha2::Sha256;` |
| 24-41 | `pub fn compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` |
| 43-58 | `pub fn derive_output_tweak(shared_secret, k) -> ScalarField` |
| 60-71 | `pub fn derive_onetime_pk(spend_pk, tweak) -> PublicKey` |
| 73-84 | `pub fn derive_onetime_sk(spend_sk, tweak) -> SecretKey` |
| 86-102 | `pub fn puzzle_hash_for_pk(pk) -> Bytes32` |
| 104 | `#[cfg(test)]` |
| 105-199 | `mod tests { ... }` containing **2 tests**: `tv1_shared_secret_matches` (line 137), `adversarial_ff32_scalar_reduces_unsigned` (line 166). Test-block imports: `use super::*; use hex_literal::hex;` + helper fns `tv1_tweak_point` (line 123) and `scan_sk` (line 129). Constants: `TV1_SCAN_SK`, `TV1_A_SUM`, `TV1_INPUT_HASH`, `TV1_SHARED_SECRET`. |

### Test inventory after POLISH-02 fold

Final test set inside `protocol.rs::tests` (8 tests):

| Test name | Origin file | Notes |
|-----------|-------------|-------|
| `tv1_shared_secret_matches` | protocol.rs (existing) | unchanged |
| `adversarial_ff32_scalar_reduces_unsigned` | protocol.rs (existing) | unchanged |
| `tv4_aggregate_sender_sks_matches` | aggregate.rs | merge |
| `tv1_compute_input_hash_matches` | input_hash.rs | merge |
| `input_hash_uses_lex_min_coin_id` | input_hash.rs | merge |
| `input_hash_order_independent` | input_hash.rs | merge |
| `tv1_derive_one_time_puzzle_hash_matches` | one_time.rs | merge |
| `derive_one_time_puzzle_hash_k1_round_trip` | one_time.rs | merge |

**No name collisions.** All 8 names are unique → flat `mod tests {}` (Claude's-discretion option (a)) works without renaming. Test-block imports collapse to `use super::*; use chia_bls::SecretKey; use hex_literal::hex;` — superset of every individual file's test imports.

**Constant collisions to resolve:**
- `TV1_INPUT_HASH` appears in input_hash.rs::tests (`38a1c8...cc9411`) AND in one_time.rs::tests (same value) AND in protocol.rs::tests (same value). All three are byte-identical; the merged tests block declares it **once**.
- `TV1_A_SUM` appears in input_hash.rs::tests and protocol.rs::tests with identical bytes. Declare once.
- `TV1_SCAN_SK` appears in one_time.rs::tests and protocol.rs::tests with identical bytes. Declare once.
- `TV1_SCAN_PK`, `TV1_SPEND_PK`, `TV1_AGGREGATED_SENDER_SK`, `TV1_PUZZLE_HASH`, `TV1_SHARED_SECRET`, `TV1_COIN_ID`, `TV4_SENDER_SK_0`, `TV4_SENDER_SK_1`, `TV4_AGGREGATED_SK` — each appears in exactly one origin block; no collision.
- Helper fns `tv1_tweak_point()` and `scan_sk()` (from protocol.rs) — used only by `tv1_shared_secret_matches`. Keep as-is.

**Recommendation for the planner:** flat `mod tests {}` (option (a)). 8 unique test names + 3 byte-identical constants to dedupe = ~10 lines saved vs. (b)'s nested mod ceremony. (b)'s historical-grouping signal is weaker than the simplicity gain.

### POLISH-03 target — `send_destination.rs` (50 lines, today)

Current source (relevant snippet — lines 13-44):
```
13: /// Where a `SendAction` addresses its output: either a literal puzzle hash
14: /// (the standard case) or a silent-payment address (chip-0057; the recipient
15: /// publishes one static address and every payment lands at a fresh,
16: /// unlinkable one-time puzzle hash derived via ECDH).
17: ///
18: /// Cannot derive `Copy` because `SilentPaymentAddress` is `Clone`-only.       ← LINE 19, not 21-23
19: ///
20: /// `impl From<Bytes32>` lets every existing `Action::send(id, ph, amount, memos)`
21: /// caller continue to compile unchanged after `Action::send`'s second parameter
22: /// becomes `impl Into<SendDestination>` in Plan 04.2-02.                       ← REMAINING `Plan 04.2-02` violates CLEANUP-01 grep — see Risk R3
23: ///
24: /// Privacy warning: the `SilentPayment` variant carries a [`SilentPaymentAddress`]
25: /// — any memos attached to the resulting `Action::send` land on chain in
26: /// plaintext and are visible to anyone holding the recipient's scan key. A
27: /// 32-byte first memo is rejected at apply time by the relocated
28: /// memo-hint guard (`DriverError::SilentPaymentMemoHintForbidden`).
29: #[derive(Debug, Clone)]
30: pub enum SendDestination {
31:     PuzzleHash(Bytes32),
32:     /// Boxed because `SilentPaymentAddress` is ~296 bytes (two BLS pubkeys) and
33:     /// would dominate the enum's size otherwise (`clippy::large_enum_variant`).  ← `large_enum_variant` is at line 34
34:     /// Boxing preserves the type's `Clone` semantics.
35:     ///
36:     /// Privacy warning: memos attached to an `Action::send` with this destination
37:     /// land on chain in plaintext via the deferred `CreateCoin` emission. They
38:     /// are visible to anyone holding the recipient's scan key. A 32-byte first
39:     /// memo is rejected at apply time by `SendAction::spend`'s chip-0057 SP arm
40:     /// (`DriverError::SilentPaymentMemoHintForbidden`).
41:     #[cfg(feature = "chip-0057")]
42:     SilentPayment(Box<SilentPaymentAddress>),
43: }
```

**Note on CONTEXT.md line numbers:** D-03 says "lines 21-23" — the live file has the offending sentence at **line 19** (single line, not three). The discrepancy is a CONTEXT.md drift from an earlier view of the file. Planner verifies at execution time and deletes the actual single-line "Cannot derive Copy" sentence + the bracketing blank `///` lines (likely 18 and 20) for a clean ~3-line removal.

**Surgical edit:** delete line 19 plus one of its bracketing `///` blanks (e.g., delete 18-19 OR 19-20). Net: 2 lines removed, paragraph breaks remain coherent.

### POLISH-04 target — `actions/send.rs:35-63` (the dispatch site)

Current source (lines 35-63):
```rust
35:     fn spend(
36:         &self,
37:         ctx: &mut SpendContext,
38:         spends: &mut Spends,
39:         _index: usize,
40:     ) -> Result<(), DriverError> {
41:         // chip-0057 SP arm: delegate to the dedicated module so this dispatch
42:         // stays generic. The helper fires SilentPaymentRequiresXch (Id check)
43:         // BEFORE the memo-hint guard BEFORE parent reservation.
44:         #[cfg(feature = "chip-0057")]
45:         if let SendDestination::SilentPayment(addr) = &self.destination {
46:             return crate::actions::silent_payment_send::handle_silent_payment_send(
47:                 ctx,
48:                 spends,
49:                 &self.id,
50:                 addr,
51:                 self.amount,
52:                 self.memos,
53:             );
54:         }
55:
56:         // PuzzleHash destination — exhaustive extraction. Under chip-0057 the
57:         // SilentPayment(_) arm is statically handled above; the early return
58:         // makes the post-handled match exhaustiveness arm unreachable!().
59:         let puzzle_hash = match &self.destination {
60:             SendDestination::PuzzleHash(ph) => *ph,
61:             #[cfg(feature = "chip-0057")]
62:             SendDestination::SilentPayment(_) => unreachable!("handled above"),
63:         };
```

Target replacement (D-04, lines 41-63 → ~10 lines):
```rust
        let puzzle_hash = match &self.destination {
            SendDestination::PuzzleHash(ph) => *ph,
            #[cfg(feature = "chip-0057")]
            SendDestination::SilentPayment(addr) => {
                return crate::actions::silent_payment_send::handle_silent_payment_send(
                    ctx, spends, &self.id, addr, self.amount, self.memos,
                );
            }
        };
```

**Net delta:** 23 lines collapse to ~9. Two prose comments (41-43, 56-58) disappear with the structural justification they explained. `grep -c 'unreachable!' send.rs` drops from 2 to **at most** 1 (Risk R4 — investigate the second site). `grep -c 'handle_silent_payment_send' send.rs` stays at 1.

## External Callsite Audit

All identified external import paths (verified by grep across `crates/` and `src/`):

| Callsite | Import | Resolves after POLISH-02? |
|----------|--------|--------------------------|
| `crates/chia-sdk-bindings/src/silent_payments.rs:407,425,432` | `chia_sdk_driver::derive_one_time_puzzle_hash`, `::compute_input_hash`, `::aggregate_sender_sks` | YES — names re-exported through `silent_payments/mod.rs` (D-01 explicitly lists all 3) and through `chia_sdk_driver` top-level (named in `chia-sdk-driver/src/lib.rs` via `silent_payments::*` re-export pattern). |
| `crates/chia-sdk-test/src/silent_payments/tweak_data.rs:10` | `use chia_sdk_driver::silent_payments::{OutputMeta, TweakData, compute_input_hash};` | YES — all 3 listed in D-01's post-edit body. |
| `crates/chia-sdk-driver/tests/silent_payments_e2e.rs:24` | `use chia_sdk_driver::silent_payments::{K_MAX_DEFAULT, scan_from_tweaks};` | YES — both listed in D-01's `pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};`. |
| `crates/chia-sdk-driver/src/actions/silent_payment_send.rs:138` | `use crate::silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash};` (inside the `#[cfg(test)]` block, line 138) | YES — all 3 listed in D-01's post-edit `pub use protocol::{...}`. |
| `crates/chia-sdk-driver/src/action_system/spends.rs:596-598` | `use crate::silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash};` (inside `sp_finish_branch` private fn) | YES — same 3 names. |
| `src/prelude.rs:41-45` | 12 names: `DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_input_hash, compute_shared_secret_from_tweak, derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks` (13 if `scan_from_tweaks` is split across the trailing line — count is 13 names total). | YES — D-01's post-edit body re-exports all 13 names explicitly (8 from protocol, 3 from scanner, 3 from types ≠ wait, let me recount: protocol = 8, scanner = 3, types = 3 = 14? Actually it's: 8 protocol + 3 scanner + 3 types = 14 names republished; prelude lists 13 — the missing one is `SilentPaymentScan` (which IS in scanner re-export and IS in prelude). Recount: prelude:41-45 = 13 names (excluding `aggregate_sender_sks` — verified by re-reading the prelude). D-01 republishes 14 (the 8th protocol entry `aggregate_sender_sks` is NOT in prelude per Phase 4 D-decision). All 13 prelude entries resolve. |

**Cross-check the prelude:** `src/prelude.rs:41-45` enumerates `DetectedSpCoin, K_MAX_DEFAULT, OutputMeta, SilentPaymentScan, TweakData, compute_input_hash, compute_shared_secret_from_tweak, derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak, puzzle_hash_for_pk, scan_from_tweaks` = **13 names**. D-01's post-edit mod.rs re-exports a superset: 8 from protocol + 3 from scanner + 3 from types = 14. The extra one is `aggregate_sender_sks` — present in mod.rs's `pub use` but DELIBERATELY OMITTED from prelude per Phase 04.2 D-decision (logged in STATE.md as "aggregate_sender_sks DELIBERATELY omitted from umbrella prelude per RESEARCH Anti-Pattern 2 — too easy to misuse outside Spends::finish_with_silent_payment_keys invariants"). This is intentional and survives Phase 8.

**Conclusion:** every external callsite path resolves identically after POLISH-02 fold. No external file needs editing.

## Architecture Patterns

### Recommended approach (per locked decisions)

Phase 8 follows Phase 7's "surgical-edit + grep-oracle" pattern. No new architecture; only file consolidation.

### Pattern 1: Named `pub use` over wildcard

**Source:** `src/prelude.rs:35-39` (utils SP types) and `src/prelude.rs:41-45` (driver SP names) already use this style. `crates/chia-sdk-utils/src/silent_payments/mod.rs` is a workspace reference for the same.

**Application in Phase 8:** `silent_payments/mod.rs` switches from 6 wildcards to 4 named-list re-exports (2 of which collapse after the POLISH-02 fold), bringing it in line with the rest of the workspace.

### Pattern 2: Single-`match` dispatch with early-return inside an arm

**Source:** Rust idiom; common across the SDK's existing dispatch sites. Example: `actions/send.rs:68-115` (the Cat/Did/Nft/Option dispatch chain) reads top-to-bottom without auxiliary if-lets.

**Application in Phase 8 (POLISH-04):** the chip-0057 arm becomes a single `match` arm that `return`s from inside via `handle_silent_payment_send(...)`. The `unreachable!("handled above")` artifact disappears because exhaustiveness is now satisfied by the SP arm's `return`, not by an empty post-match arm.

### Pattern 3: Flat-sibling module file with inline `#[cfg(test)] mod tests {}`

**Source:** every primitive under `crates/chia-sdk-driver/src/primitives/` (CAT, NFT, DID, Singleton, Vault, OptionContract). Example: `crates/chia-sdk-driver/src/primitives/cat/cat_info.rs` (113 lines, flat sibling with inline tests). `crates/chia-sdk-driver/src/primitives/nft/nft_info.rs` (311 lines, same shape). The new `protocol.rs` (~640 lines) is consistent with these and remains under `scanner.rs`'s 735-line in-module ceiling.

**Application in Phase 8 (POLISH-02):** the 3 deletion targets' inline test blocks merge into protocol.rs's existing inline test block at the file's bottom. Same convention.

### Anti-Patterns to Avoid

- **Don't introduce a `silent_payments/send.rs`** as part of POLISH-02. CONTEXT.md explicitly rejects this; the send compositions belong with the primitives they compose, and scanner.rs already imports protocol primitives — splitting would force two files to share the same primitive set.
- **Don't demote protocol primitives to `pub(crate)`.** CONTEXT.md rejects this in `<deferred>`. The documented commitment in mod.rs rustdoc + prelude membership is binding.
- **Don't extract a `dispatch_puzzle_hash_target` helper** in send.rs. CONTEXT.md rejects this in `<deferred>` — larger diff than the POLISH-04 nit warrants.
- **Don't update prelude ordering or wording** outside the explicit Phase 8 scope. The 13 SP names in `src/prelude.rs:41-45` stay byte-identical. Any change there would create a documented-API churn signal.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Detecting wildcard re-exports | Custom AST scanner | `grep -nE '^pub use [a-z_]+::\*;'` | Acceptance oracle is already grep-shaped; CI infrastructure runs greps in seconds. |
| Verifying test-count parity | Custom test parser | `cargo test --release -p chia-sdk-driver --features chip-0057 --no-run` then list test fns by name in test output | Standard `cargo test` is the source of truth; building a parser duplicates work and drifts. |
| Verifying ≤700-line target | Custom rust-lines counter | `wc -l protocol.rs` | `wc` is universally available; rust-tokens-only counts diverge from human line counts. |
| Marshaling `pub use` symbol lists across crates | Reach into bindy/bindings/* JSON | Standard `pub use foo::{Bar, Baz};` re-exports | Bindings are independent of mod.rs structure — they import named symbols and don't see internal modularity. |
| Test merge "shape" picker | Build a heuristic | Read this RESEARCH's Test Inventory and pick option (a) | 8 unique test names + 3 dedupable constants makes flat `mod tests {}` clearly cleaner. |

**Key insight:** Phase 8 is a maintainer-style polish phase. Every acceptance oracle is a grep, wc, or `cargo test --list` — all native CLI tools. No new tooling, no new abstraction.

## Runtime State Inventory

**Not applicable.** Phase 8 is a code-only refactor:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — verified by zero references to data persistence in any of the 4 POLISH items | None |
| Live service config | None — Phase 8 touches only chia-sdk-driver source files | None |
| OS-registered state | None — no installed binaries, no registered services | None |
| Secrets/env vars | None — no auth, no environment-dependent behavior | None |
| Build artifacts | None — protocol.rs grows but no Cargo.toml / feature gate / proc-macro / build.rs changes; `cargo build` rebuilds incrementally | None |

The phase is purely "rearrange source text"; runtime state survives mechanically.

## Common Pitfalls

### Pitfall 1: Hidden import collision in test-block merge

**What goes wrong:** when the 3 deletion files' `#[cfg(test)] mod tests` blocks merge into protocol.rs's existing test block, two const-name collisions between aggregate.rs (test `TV4_SENDER_SK_0/1`) and one_time.rs (`TV1_AGGREGATED_SENDER_SK = 5002eaf0...678c7a` which equals aggregate.rs's `TV4_SENDER_SK_0` byte-for-byte) need careful inspection — both are the same 32 bytes but the names differ.

**Why it happens:** TV1's aggregated sender SK (single-input case) and TV4's first sender SK happen to be the same byte string in the upstream reference impl.

**How to avoid:** Planner directs the implementer to (a) keep both names if both are used by distinct tests, since they're semantically different even when byte-identical; OR (b) declare once and the second test uses the same name. **Recommendation:** keep both names, since `TV1_AGGREGATED_SENDER_SK` and `TV4_SENDER_SK_0` carry different "meaning" in the test prose — declaring twice with two `pub const`s of the same value is the lowest-friction outcome and matches the existing source.

**Warning signs:** clippy `clippy::declare_interior_mutable_const` or `clippy::duplicated_const_value` if it ever fires. If it does, dedupe to a single name and update one of the tests' references.

### Pitfall 2: `unreachable!` still grep-matches after the POLISH-04 edit

**What goes wrong:** the file-wide grep `grep -c 'unreachable!' send.rs` returns 2 today. CONTEXT.md's acceptance says "returns 0", which only holds if the SECOND `unreachable!` call site is ALSO removed.

**Why it happens:** `actions/send.rs:399` is a large file with multiple dispatch branches. Either the second site is (a) cfg-conditional code that doesn't actually reach `unreachable!()` (false positive in raw grep — e.g., inside a doc string or comment), or (b) a separate legitimate `unreachable!()` outside the chip-0057 SP arm that POLISH-04 doesn't address.

**How to avoid:** Planner directs the implementer to **first** locate the second `unreachable!` hit (Wave 1, before editing): `grep -n 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs`. If it's a comment / doc / non-executable site, decide whether the CONTEXT.md acceptance "returns 0" is achievable; if not, raise as a discovered-during-execution clarification. If it's a separate legitimate dispatch site, POLISH-04 scope expands to address both. **Most likely outcome:** the second hit is in a comment explaining the first (lines 56-58 reference "unreachable!()" in the comment), and removing the comment block as part of POLISH-04 also removes the second grep hit. See Risk R4.

**Warning signs:** acceptance grep returns 1 (not 0) after the implementer's edit.

### Pitfall 3: `Plan 04.2-02` reference inside `send_destination.rs:22`

**What goes wrong:** the kept paragraph "lines 25-32" of CONTEXT.md (which I've verified maps to actual lines 20-22 — the `impl From<Bytes32>` explanatory paragraph) contains the string `Plan 04.2-02`. This violates CLEANUP-01's acceptance grep `Plan 0[1-9]-` and would re-introduce a violation if the surrounding paragraph survives Phase 8 unedited.

**Why it happens:** Phase 7's CLEANUP-01 grep ban targets `Plan 0[1-6]-` (digits 1-6 only — see REQUIREMENTS.md:69), not 1-9. The `04.2-02` reference (a) doesn't match `0[1-6]-` literally, AND (b) was deliberately left alone. So Phase 7 is consistent and this isn't a CLEANUP-01 regression.

**How to avoid:** Phase 8 leaves the `Plan 04.2-02` reference untouched. POLISH-03 only deletes the "Cannot derive Copy" sentence + bracketing blanks. The `impl From<Bytes32>` paragraph (the `Plan 04.2-02` reference's home) stays. No action needed — but verify the CLEANUP-01 grep still returns 0 hits across the full target set after Phase 8.

**Warning signs:** if a future broader CLEANUP grep (covering `Plan 0[1-9]-`) emerges, this paragraph becomes a violation. Out of Phase 8 scope.

### Pitfall 4: Workspace test-count tracking (cosmetic, not behavioral)

**What goes wrong:** after the fold, the same 8 tests run from a single file instead of 4 files. The workspace test count for `chia-sdk-driver --features chip-0057` is **unchanged** (8 → 8). But STATE.md tracks "workspace tests 2399 → 2429" running totals — Phase 8 does NOT change this number.

**Why it happens:** by design — pure refactor.

**How to avoid:** acceptance is "test count unchanged", not "test count increases". Planner directs the implementer to record pre-phase and post-phase test counts identically.

**Warning signs:** test count diverges (means a test was accidentally dropped or renamed during the merge — fix immediately).

### Pitfall 5: ScalarField imports not file-local after fold

**What goes wrong:** the fold combines 3 files that each import `chia_sdk_types::silent_payments::ScalarField` (and protocol.rs already imports it as part of a larger named-list). After fold, declaring it twice is fine syntactically but `cargo machete` / clippy may warn on duplicate imports.

**Why it happens:** trivial import dedup oversight.

**How to avoid:** Planner directs the implementer to consolidate the use-statements at the top of protocol.rs after fold. The deduplicated header is:

```rust
use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;
use chia_puzzle_types::DeriveSynthetic;
use chia_puzzle_types::standard::StandardArgs;
use chia_sdk_types::silent_payments::{CHIA_SP_INPUTS, CHIA_SP_SHARED_SECRET, ScalarField, tagged_hash};
use chia_sha2::Sha256;
```

(Note `CHIA_SP_INPUTS` joins protocol.rs's existing `CHIA_SP_SHARED_SECRET` named-list import; `Bytes32` comes from input_hash.rs and one_time.rs but was already in protocol.rs.) The `use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk};` at one_time.rs:36 is **dropped** — those are siblings in the same file post-fold.

**Warning signs:** clippy `clippy::single_component_path_imports` or `clippy::useless_imports`; or `cargo machete` listing a now-unused crate dep (no — all listed crates remain used).

## Code Examples

### Example 1: Named-export pattern (POLISH-01 target shape)

**Source:** `crates/chia-sdk-utils/src/silent_payments/mod.rs` (workspace reference) and CONTEXT.md D-01:

```rust
mod protocol;
pub use protocol::{
    aggregate_sender_sks,
    compute_input_hash,
    compute_shared_secret_from_tweak,
    derive_one_time_puzzle_hash,
    derive_onetime_pk,
    derive_onetime_sk,
    derive_output_tweak,
    puzzle_hash_for_pk,
};
```

Alphabetical-by-symbol within each block matches the workspace style. CONTEXT.md doesn't lock the within-block ordering (D-01's code block IS alphabetical for protocol's 8 names); planner adopts alphabetical for consistency.

### Example 2: Single-match dispatch with `return`-in-arm (POLISH-04 target)

**Source:** CONTEXT.md D-04 (verbatim):

```rust
let puzzle_hash = match &self.destination {
    SendDestination::PuzzleHash(ph) => *ph,
    #[cfg(feature = "chip-0057")]
    SendDestination::SilentPayment(addr) => {
        return crate::actions::silent_payment_send::handle_silent_payment_send(
            ctx, spends, &self.id, addr, self.amount, self.memos,
        );
    }
};
```

Compiles identically under both feature flags (SP arm gated by `#[cfg]`, exhaustiveness satisfied because the un-cfg'd code only has `PuzzleHash` to match).

### Example 3: Flat-sibling mod tests with deduplicated constants (POLISH-02 fold shape — flat-tests variant (a))

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chia_bls::SecretKey;
    use hex_literal::hex;

    // ─── TV1 pinned bytes (CHIP-0057 test vector 1) ──────────────────────
    const TV1_SCAN_SK: [u8; 32]    = hex!("132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6");
    const TV1_SCAN_PK: [u8; 48]    = hex!("a04f404bfbfdc9311736899fe32d2275bb007814510c3523529487ad7573607573ade20d31c75107b40331fff79ac896");
    const TV1_SPEND_PK: [u8; 48]   = hex!("8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527dbf6f31e22e0831ba5088c952625a75c74");
    const TV1_A_SUM: [u8; 48]      = hex!("8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c51b3f2d87dc03a657e07f1828ab760fa");
    const TV1_COIN_ID: [u8; 32]    = hex!("5d759d2d97c03b1f6fe0657e91d25f6b7dd1311d6023271a1bcd35978a94a175");
    const TV1_INPUT_HASH: [u8; 32] = hex!("38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411");
    const TV1_SHARED_SECRET: [u8; 32] = hex!("d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6");
    const TV1_PUZZLE_HASH: [u8; 32] = hex!("23adba149dd9000d65e0f8e21b6975364cbe89a63caf56533df4b7664c21fbf5");
    const TV1_AGGREGATED_SENDER_SK: [u8; 32] = hex!("5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a");

    // ─── TV4 pinned bytes (CHIP-0057 test vector 4, multi-input) ─────────
    const TV4_SENDER_SK_0: [u8; 32]    = hex!("5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a");
    const TV4_SENDER_SK_1: [u8; 32]    = hex!("05fded8808216b65d439fc41cb07c7270e37ed743e0745652afe055cfe91cf0f");
    const TV4_AGGREGATED_SK: [u8; 32]  = hex!("5600d8781de32f0f3d86bc96b46a3a4ea56ae26da168b55dc66b503acbf95b89");

    // Helpers and 8 tests follow (in declaration order from the 4 origin files):
    fn tv1_tweak_point() -> PublicKey { /* from protocol.rs */ }
    fn scan_sk() -> SecretKey { /* from protocol.rs */ }

    #[test] fn tv1_shared_secret_matches() { /* from protocol.rs */ }
    #[test] fn adversarial_ff32_scalar_reduces_unsigned() { /* from protocol.rs */ }
    #[test] fn tv4_aggregate_sender_sks_matches() { /* from aggregate.rs */ }
    #[test] fn tv1_compute_input_hash_matches() { /* from input_hash.rs */ }
    #[test] fn input_hash_uses_lex_min_coin_id() { /* from input_hash.rs */ }
    #[test] fn input_hash_order_independent() { /* from input_hash.rs */ }
    #[test] fn tv1_derive_one_time_puzzle_hash_matches() { /* from one_time.rs */ }
    #[test] fn derive_one_time_puzzle_hash_k1_round_trip() { /* from one_time.rs */ }
}
```

Note `TV1_AGGREGATED_SENDER_SK == TV4_SENDER_SK_0` byte-for-byte (Pitfall 1) — keep both names. Test bodies copy in verbatim from the originals; `use crate::silent_payments::compute_shared_secret_from_tweak;` at one_time.rs:157 is dropped (now a sibling — `super::*` covers it).

## State of the Art

Not applicable — Phase 8 is a polish phase on existing code, not adoption of new ecosystem tooling.

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| n/a | n/a | n/a | n/a |

## Open Questions

### 1. The second `unreachable!` hit in `actions/send.rs`

- **What we know:** `grep -c 'unreachable!' send.rs` returns **2**, not 1. The intentional one is at line 62. CONTEXT.md acceptance says "returns 0".
- **What's unclear:** where the second hit is. Likely candidates: a comment at line 58 (which IS in the deletion target — line 58 reads `// makes the post-handled match exhaustiveness arm unreachable!().`), OR an unrelated executable `unreachable!()` elsewhere in send.rs.
- **Recommendation:** Wave 1 of POLISH-04 includes a precondition step: `grep -n 'unreachable!' send.rs` and visually confirm both hits. **Strong hypothesis:** the second hit is in the comment at line 58 ("...arm unreachable!().") which IS deleted as part of POLISH-04. Outcome: both hits resolve to 0 after Phase 8.
- **Action for planner:** add a precondition verification step + a post-edit verification step; if the second hit is NOT in the comment, raise to the user before continuing.

### 2. CONTEXT.md line-number drift in POLISH-03

- **What we know:** D-03 references "lines 21-23" for the "Cannot derive Copy" paragraph; the live file has the sentence at line 19 (single line).
- **What's unclear:** whether the file changed between CONTEXT.md authoring and now, OR CONTEXT.md just used approximate line numbers.
- **Recommendation:** trust the live file. Surgical delete is "the line containing `Cannot derive .Copy.`" + one bracketing blank `///` line. Net 2 lines removed. The acceptance grep (`Cannot derive .Copy.` returns 0; `large_enum_variant` returns exactly 1) is the source of truth, not line numbers.
- **Action for planner:** verify post-edit greps, not line numbers.

### 3. POLISH-02 test-merger shape — option (a) vs option (b)

- **What we know:** Claude's Discretion explicitly leaves this to the planner. (a) flat `mod tests {}`; (b) nested `mod tests { mod aggregate_tests; mod input_hash_tests; mod one_time_tests; }`.
- **What's unclear:** the planner's stylistic preference.
- **Recommendation:** **option (a) — flat `mod tests {}`.** Evidence supporting (a):
  - 8 unique test names, zero collision risk.
  - 3 byte-identical constants dedupable into single declarations (option (b) would require either (b.1) shared `super::*` re-exports of the constants, or (b.2) duplicate constant declarations across the 3 sub-modules; (a) just declares each once).
  - Existing repo convention (CAT/NFT/DID primitives) is single flat `mod tests {}` at file bottom.
  - The historical-grouping signal that (b) preserves is recoverable from `git blame`.

### 4. Whether `silent_payments/mod.rs` rustdoc needs an update

- **What we know:** Claude's Discretion explicitly leaves this optional. Current rustdoc references primitives by symbol (`compute_shared_secret_from_tweak`, etc.) — not by sub-file.
- **What's unclear:** whether to opportunistically tighten the rustdoc.
- **Recommendation:** **make a tiny rustdoc edit** to remove any implicit sub-module references. The current text at lines 9-13 says "The protocol primitives ... are exposed publicly so the send-side action and any caller that needs to compute shared secrets manually can reuse them" — this is module-structure-agnostic and already correct. **No edit needed.** Leave the rustdoc as-is. If the planner wants to tighten further, the optional edit is to change "The protocol primitives" → "These protocol primitives" (one word) — but this is gratuitous.

### 5. Should the rustdoc on protocol.rs grow to describe the 3 absorbed compositions?

- **What we know:** the current rustdoc at protocol.rs:1-15 lists exactly 5 functions: `compute_shared_secret_from_tweak`, `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`. After POLISH-02 fold, the file ALSO contains `aggregate_sender_sks`, `compute_input_hash`, `derive_one_time_puzzle_hash` — three more public functions whose docs the rustdoc should arguably mention.
- **What's unclear:** whether to extend the rustdoc.
- **Recommendation:** **YES, extend the rustdoc** with a short "Send-side compositions" subsection listing the 3 absorbed functions. Rationale: the rustdoc is what shows on docs.rs and IDE tooltips; readers landing on protocol.rs after Phase 8 will see 8 pub fns but only 5 mentioned in the file header. Concretely: add a subsection (3-4 lines) after the existing 5-bullet list, listing the 3 absorbed compositions with one-line descriptions. This is a small write but high readability value. Planner can also leave this for a future polish pass; the absorbed functions each have their own `///` rustdoc on the `pub fn` so missing the file-header mention isn't a doc-correctness defect.

## Environment Availability

Phase 8 has no external runtime dependencies beyond the workspace itself.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` (Rust toolchain) | All cargo invocations | ✓ | 1.90.0 (pinned in `rust-toolchain.toml`) | — |
| `rustc` | Build | ✓ | 1.90.0 | — |
| `clippy` | Lint gate | ✓ | bundled with rustup | — |
| `rustfmt` | Format gate | ✓ | bundled with rustup | — |
| `cargo machete` | Unused-dep gate | ✓ (verified to be available in Phase 7 verification) | — | — |
| `grep` | Acceptance oracles | ✓ | GNU grep on Linux | — |
| `wc` | Line-count oracle | ✓ | coreutils | — |

**Missing dependencies:** none.

**Conclusion:** Phase 8 runs entirely within the workspace toolchain. No new install steps.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust 1.90.0 built-in test harness) |
| Config file | `Cargo.toml` (workspace), `crates/chia-sdk-driver/Cargo.toml` (chip-0057 feature) |
| Quick run command | `cargo test --release -p chia-sdk-driver --features chip-0057 --lib silent_payments` |
| Full chip-0057 driver suite | `cargo test --release -p chia-sdk-driver --features chip-0057` |
| Integration target (e2e) | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` |
| Workspace suite | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |

### Phase Requirements → Acceptance-Oracle Map

| Req ID | Behavior | Test Type | Automated Command | Pre-Phase Result | Post-Phase Acceptance |
|--------|----------|-----------|-------------------|------------------|------------------------|
| POLISH-01 | mod.rs has zero public-module wildcard re-exports | grep oracle | `grep -cE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` | **5** (6 hits if counting `pub(crate)` — but the regex excludes `pub(crate)`, so the result is `5`: aggregate, input_hash, one_time, protocol, scanner, types = 6 minus the post-fold deletions; pre-phase exact result is `6`) → recount: regex `^pub use` matches all 6, since `pub(crate) use send_keys::*;` starts with `pub(crate)` not `pub use`. So 6 hits pre-phase. | `0` |
| POLISH-01 | All 13 prelude SP names remain reachable | build oracle | `cargo build --release --workspace --all-features` | clean | clean (would fail if any prelude name were unreachable) |
| POLISH-02 | 3 deletion targets are gone | file-existence oracle | `test ! -f crates/chia-sdk-driver/src/silent_payments/aggregate.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/input_hash.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/one_time.rs` | files exist | exit 0 |
| POLISH-02 | protocol.rs ≤ 700 lines | wc oracle | `wc -l crates/chia-sdk-driver/src/silent_payments/protocol.rs \| awk '{print $1}'` | `199` | `≤ 700` (target ~640) |
| POLISH-02 | Test count unchanged | cargo-test oracle | `cargo test --release -p chia-sdk-driver --features chip-0057 2>&1 \| grep -E 'test result:' \| awk '{print $4}'` (count passed) | record N pre-phase | exactly N post-phase |
| POLISH-02 | Named tests still run | cargo-test name match | `cargo test --release -p chia-sdk-driver --features chip-0057 -- tv4_aggregate_sender_sks_matches tv1_compute_input_hash_matches input_hash_uses_lex_min_coin_id input_hash_order_independent tv1_derive_one_time_puzzle_hash_matches derive_one_time_puzzle_hash_k1_round_trip tv1_shared_secret_matches adversarial_ff32_scalar_reduces_unsigned 2>&1 \| grep 'test result:'` | all 8 pass | all 8 pass |
| POLISH-03 | `Cannot derive Copy` removed | grep oracle | `grep -c 'Cannot derive .Copy.' crates/chia-sdk-driver/src/action_system/send_destination.rs` | `1` | `0` |
| POLISH-03 | `large_enum_variant` rationale preserved | grep oracle | `grep -c 'large_enum_variant' crates/chia-sdk-driver/src/action_system/send_destination.rs` | `1` | `1` (exactly) |
| POLISH-04 | `unreachable!` calls eliminated from send.rs | grep oracle | `grep -c 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs` | `2` (see Open Q1) | `0` |
| POLISH-04 | `handle_silent_payment_send` call preserved | grep oracle | `grep -c 'handle_silent_payment_send' crates/chia-sdk-driver/src/actions/send.rs` | `1` | `1` (exactly) |
| POLISH-04 | Action::send dispatch behavior unchanged | cargo-test oracle | `cargo test --release -p chia-sdk-driver --features chip-0057 -- test_action_send_xch test_action_send_cat test_action_send_xch_with_change action_state_machine round_trip_matches_derive_one_time_puzzle_hash silent_payment_destination_requires_xch_id silent_payment_keys_not_registered_errors_at_finish` | all pass | all pass |

### Cross-Cutting CI Gates

| Gate | Command | Acceptance |
|------|---------|------------|
| Workspace build (--all-features) | `cargo build --release --all-features` | exit 0 |
| Per-crate chip-0057 build | `cargo build --release -p chia-sdk-driver --features chip-0057` | exit 0 |
| Per-crate no-default-features build | `cargo build --release -p chia-sdk-driver --no-default-features` | exit 0 |
| fmt | `cargo fmt --all --check` | exit 0 |
| Scoped clippy (chip-0057 driver, -D warnings) | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` | exit 0 |
| Scoped clippy (chia-sdk-bindings all-features, -D warnings) | `cargo clippy -p chia-sdk-bindings --all-features --all-targets -- -D warnings` | exit 0 |
| Workspace clippy (no -D warnings) | `cargo clippy --workspace --all-features --all-targets` | only pre-existing chia-sdk-daemon warnings remain (Phase 7 deferred-items precedent) |
| cargo machete (no new ignored entries) | `cargo machete` | "didn't find any unused dependencies" |
| chip-0057 driver suite | `cargo test --release -p chia-sdk-driver --features chip-0057` | all tests pass; count unchanged from pre-phase |
| Integration target | `cargo test --release -p chia-sdk-driver --features chip-0057 --test silent_payments_e2e` | 3/3 pass |
| Full workspace test suite | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` | all tests pass |

### Sampling Rate

- **Per task commit:** scoped grep oracles + `cargo build --release -p chia-sdk-driver --features chip-0057` + scoped clippy (`-D warnings`).
- **Per wave merge:** all per-crate builds + `cargo test --release -p chia-sdk-driver --features chip-0057` + `cargo fmt --check`.
- **Phase gate (`/gsd:verify-work`):** all 11 cross-cutting CI gates above + all 11 per-requirement acceptance oracles.

### Wave 0 Gaps

**None — Phase 8 introduces no new tests.** The existing 8 test functions across the 4 silent_payments protocol files merge in place; the test infrastructure (cargo test, scoped clippy, grep oracles, wc, file-existence checks) is fully available.

Acceptance oracles per Phase 7 verification precedent are all native CLI:
- `grep`, `wc`, `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`, `cargo machete`, `test -f` (`! -f`).

## Risks & Mitigations

| ID | Risk | Mitigation | Detection |
|----|------|------------|-----------|
| **R1** | Test merge introduces unintentional rename (e.g., capitalization drift) | Planner directs implementer to copy test bodies verbatim; only path of file changes | `cargo test --features chip-0057 -- <list-of-8-named-tests>` succeeds — failure indicates rename |
| **R2** | Import dedup drops a use-statement still needed by a test inside the merged block | Use full deduplicated header from Pitfall 5; verify `cargo build -p chia-sdk-driver --features chip-0057` after fold | compile error during build |
| **R3** | POLISH-03 also removes the `Plan 04.2-02` reference at line 22, which is OUT of POLISH-03 scope | Planner explicitly bounds POLISH-03 to removing only the "Cannot derive Copy" sentence; the `Plan 04.2-02` paragraph stays | Visual diff of `send_destination.rs` before/after; grep `Plan 04.2-02` should still return 1 hit (unless the user wants to clean this up as a bonus, which is out of CONTEXT.md scope) |
| **R4** | Second `unreachable!` hit in send.rs is NOT in the comment block POLISH-04 deletes | Wave 1 of POLISH-04 includes precondition grep + visual inspection. If the second hit is in unrelated code, raise to the user before continuing. | `grep -c 'unreachable!' send.rs` returns 1 after the edit (not 0) → file isn't yet at acceptance |
| **R5** | External callsite uses a path that isn't in the CONTEXT.md audit (e.g., a downstream crate) | All known callsites in this workspace are audited. Downstream consumers (Sage) use `chia_wallet_sdk::prelude::*` which is byte-identical. | `cargo build --release --workspace --all-features` clean (would fail on any unresolved path) |
| **R6** | Workspace test count drifts due to test merge | Pre-phase: capture test count via `cargo test ... 2>&1 \| grep 'test result:'`. Post-phase: same command, same N. | Direct comparison — any drift requires investigation |
| **R7** | rustdoc inside protocol.rs renders broken intra-doc links after fold (e.g., `[`derive_output_tweak`]` link to a then-private symbol) | All absorbed primitives are `pub` (in protocol.rs's mod.rs re-export); intra-doc links resolve. Verify via `cargo doc -p chia-sdk-driver --features chip-0057 --no-deps` post-fold. | `cargo doc` returns 0 errors |
| **R8** | POLISH-02 mod-declaration delete + named-export edit are not atomic — transient build-break window | Combine the mod.rs edit (deleting `mod aggregate; pub use aggregate::*;` etc.) and the file delete + protocol.rs append into a SINGLE commit. Same pattern Phase 7 Plan 04.1-01 documented. | Pre-commit `cargo build` passes for the staged final state |
| **R9** | `pub use protocol::{aggregate_sender_sks, ...}` adds a new top-level driver re-export that wasn't there before — drift on `chia_sdk_driver` top-level surface | Verify `chia-sdk-driver/src/lib.rs` doesn't `pub use silent_payments::*` (and thus doesn't blast new symbols to driver-top-level). Per STATE.md, `lib.rs` is `pub mod silent_payments;` (the module itself is pub); names reach `chia_sdk_driver::FOO` only because the prelude re-imports them. | `grep -n 'pub use silent_payments' crates/chia-sdk-driver/src/lib.rs` confirms which symbols leak to top-level |

## Sources

### Primary (HIGH confidence)
- **`./CLAUDE.md`** (this repo) — pinned toolchain version, workspace lints, test commands.
- **`.planning/phases/08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics/08-CONTEXT.md`** — locked decisions D-01 through D-04, acceptance grep oracles.
- **`.planning/REQUIREMENTS.md`** — POLISH-01 through POLISH-04 definitions + traceability table.
- **`.planning/STATE.md`** — phase 7 precedent decisions, current workspace test count (~2429), conventions adopted.
- **`.planning/phases/07-code-review-cleanup/07-VERIFICATION.md`** — verification pattern Phase 8 follows.
- **Live source files** (all 16 files in `<files_to_read>`) — verified line counts, current grep results, import statements, test function names, external callsite paths.
- **`Cargo.toml`** (workspace) — feature flags, dependency declarations.

### Secondary (MEDIUM confidence)
- **Repo convention precedents** (cited in CONTEXT.md): `crates/chia-sdk-driver/src/primitives/cat/cat_info.rs`, `nft_info.rs`, `scanner.rs`. Not re-read in full but their existence + size confirmed.

### Tertiary (LOW confidence)
- None. Phase 8 is fully constrained by the in-repo evidence above.

## Metadata

**Confidence breakdown:**
- POLISH-01 (mod.rs named exports): **HIGH** — every wildcard's current line is verified; D-01's post-edit body is byte-precise; all 13 prelude names map cleanly to the new re-export list.
- POLISH-02 (fold): **HIGH** — all 4 files inventoried; 8 tests enumerated with no name collisions; 3 byte-identical constants identified; import dedup mechanical.
- POLISH-03 (doc dedup): **HIGH-MEDIUM** — CONTEXT.md line numbers drift slightly (lines 21-23 → actual line 19), but acceptance grep oracles are unambiguous; planner trusts greps over line numbers.
- POLISH-04 (dispatch restructure): **HIGH-MEDIUM** — D-04's target code is verbatim; the only open item is Risk R4 (second `unreachable!` hit location), which is a precondition-check task, not a structural unknown.
- Validation architecture: **HIGH** — Phase 7 established the grep+wc+cargo pattern; Phase 8 inherits it directly.

**Research date:** 2026-05-20
**Valid until:** 2026-06-20 (30 days; stable refactor scope, no external dependencies that could drift).

---

## RESEARCH COMPLETE
