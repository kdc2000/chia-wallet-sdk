---
phase: 01-crypto-primitives-workspace-integration
plan: 04
subsystem: infra
tags: [crypto, chip-0057, silent-payments, bip-32, derivation-paths]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 feature flag at workspace + chia-sdk-types (Plan 01-01)"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "silent_payments/ module barrel (Plan 01-02) — has `mod scalar; pub use scalar::*;` at this point"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "tagged_hash + Chia_SP/* tag constants (Plan 01-03) — appended `mod tagged_hash; pub use tagged_hash::*;` AFTER the scalar block"

provides:
  - "chia_sdk_types::silent_payments::SCAN_PATH — `pub const &[u32] = &[12381, 8444, 12, 0]` (m/12381/8444/12/0, unhardened)"
  - "chia_sdk_types::silent_payments::SPEND_PATH — `pub const &[u32] = &[12381, 8444, 13, 0]` (m/12381/8444/13/0, unhardened)"
  - "silent_payments/mod.rs barrel in final sorted ordering: `paths < scalar < tagged_hash`"

affects:
  - 01-05 (machete + clippy + grep-ban gate closure — inherits clean state; `! grep -r 'mod_by_group_order' silent_payments/` still holds across all three modules)
  - 02-XX (`SilentPaymentKeys::from_mnemonic` in chia-sdk-utils — imports SCAN_PATH and SPEND_PATH as single source of truth for BIP-32 derivation; validated end-to-end against CHIP-0057 test vector b_scan = `132567e4...690f6` and b_spend = `53d140b3...31b087`)
  - 03-XX (Receive primitive — indirectly via Phase 2's key types)
  - 04-XX (Send-side action — indirectly via Phase 2's key types)

tech-stack:
  added: []
  patterns:
    - "Derivation path constants are `pub const &[u32]`, not `[u32; 4]` or `&'static [u32; 4]` — matches what `chia_bls::DerivableKey::derive_unhardened` consumers iterate over in Phase 2"
    - "Unhardened indices encoded as raw u32 values without the hardened bit (no `| 0x80000000`) — CHIP-0057 specifies unhardened derivation explicitly, mirroring the reference impl at `~/silent-payments/crates/sp-common/src/keys.rs` lines 35-47"
    - "Module-level doc-comment continues the descriptive-phrasing convention (no literal `mod_by_group_order` token anywhere under silent_payments/)"

key-files:
  created:
    - "crates/chia-sdk-types/src/silent_payments/paths.rs"
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-04-SUMMARY.md"
  modified:
    - "crates/chia-sdk-types/src/silent_payments/mod.rs"

key-decisions:
  - "Encoded SCAN_PATH and SPEND_PATH as raw u32 values without the BIP-32 hardened bit (`| 0x80000000`). CHIP-0057 §172-173 + reference impl `~/silent-payments/crates/sp-common/src/keys.rs` both specify these as unhardened paths, so the raw integer encoding is correct and matches what `chia_bls::DerivableKey::derive_unhardened(index)` consumes."
  - "Defined the constants as `pub const &[u32]` (slice reference), not `[u32; 4]` (fixed array) or `&'static [u32; 4]` (reference to fixed array). The slice form composes more naturally with `.iter().copied()` patterns Phase 2 will use for `SilentPaymentKeys::from_mnemonic`."
  - "Two-task structure preserved verbatim (Task 1: create file; Task 2: wire into mod.rs). Task 1 leaves `paths.rs` not-yet-referenced, so its compilation is deferred until Task 2 — but that's fine because both task commits land in the same plan execution and the no-features build + clippy gates verify the wired state."
  - "No unit tests in this plan. Path values are validated end-to-end at Phase 2 (ADDR-01) when `SilentPaymentKeys::from_mnemonic` runs against CHIP-0057 test vectors. Adding a `#[test]` here would tautologically assert `SCAN_PATH == [12381, 8444, 12, 0]` against itself."

patterns-established:
  - "silent_payments/ submodules continue the descriptive-phrasing convention for any reference to the standard-puzzle synthetic-key reducer — the literal function-name token never appears under silent_payments/."
  - "mod.rs barrel pairs `mod X; pub use X::*;` in fixed sorted positions: `paths < scalar < tagged_hash`. Final state established by this plan."
  - "No tests at the constants-only file layer; validation happens at the layer that consumes the constants (Phase 2 for paths, similar pattern for any future constant-only files)."

requirements-completed: [WS-03]

duration: 3min
completed: 2026-05-15
---

# Phase 01 Plan 04: SCAN_PATH and SPEND_PATH derivation path constants Summary

**`chia_sdk_types::silent_payments::{SCAN_PATH, SPEND_PATH}` pin the two CHIP-0057 reserved BIP-32 derivation paths (m/12381/8444/12/0 and m/12381/8444/13/0, unhardened) as `pub const &[u32]` slices so Phase 2's `SilentPaymentKeys::from_mnemonic` has a single source of truth — distinct from the standard wallet's index 2, with the module barrel now in its final sorted ordering `paths < scalar < tagged_hash`.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-05-15T16:18:00Z
- **Completed:** 2026-05-15T16:21:31Z
- **Tasks:** 2
- **Files modified:** 2 (1 new + 1 edited)

## Accomplishments

- New `silent_payments/paths.rs` (25 lines) defines two `pub const &[u32]` constants matching CHIP-0057 §172-173 byte-for-byte:
  - `SCAN_PATH = &[12381, 8444, 12, 0]` — unhardened path for b_scan
  - `SPEND_PATH = &[12381, 8444, 13, 0]` — unhardened path for b_spend
- Module-level doc-comment block declares the protocol path values and references `super::ScalarField` as the single reduction route, using descriptive phrasing only (no literal prohibited reducer-function token anywhere in this file).
- `silent_payments/mod.rs` now in final sorted ordering: `mod paths;`, `mod scalar;`, `mod tagged_hash;` (with matching `pub use ::*;` lines). The `paths` block was inserted BEFORE `scalar`, not appended — locked sorted position from the plan.
- All 10 tests from Plans 02 + 03 still pass under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` — no regression.
- All plan-level gates green: `-F chip-0057` build, no-features build (gating preserved), clippy `--all-features --all-targets -- -D warnings`, `cargo fmt --check`, and the grep ban `! grep -r 'mod_by_group_order' silent_payments/`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create `silent_payments/paths.rs` with SCAN_PATH and SPEND_PATH** — `2fc7b9cc` (feat)
2. **Task 2: Wire paths module into silent_payments/mod.rs (insert BEFORE scalar lines)** — `ed8cd93c` (feat)

**Plan metadata commit:** appended after this SUMMARY.md (includes SUMMARY.md, STATE.md, ROADMAP.md, REQUIREMENTS.md updates).

## Files Created/Modified

- `crates/chia-sdk-types/src/silent_payments/paths.rs` — NEW (25 lines). Module-level doc-comment (descriptive phrasing only, no literal reducer token); two `pub const &[u32]` constants with per-constant doc-comments naming the BIP-32 path string. No tests, no code beyond the constants and their docs.
- `crates/chia-sdk-types/src/silent_payments/mod.rs` — MODIFIED (16 → 18 lines). Inserted `mod paths;` and `pub use paths::*;` BEFORE the existing `mod scalar;` block. Final body order: `paths`, `scalar`, `tagged_hash`. Trailing "additional submodules appended in sorted order" comment marker preserved.

## Decisions Made

- **Raw u32 indices, unhardened.** CHIP-0057 §172-173 says explicitly "unhardened" for both scan and spend paths, and the reference impl at `~/silent-payments/crates/sp-common/src/keys.rs` (`derive_scan_sk`, `derive_spend_sk`) calls `.derive_unhardened(12381).derive_unhardened(8444).derive_unhardened(12 or 13).derive_unhardened(0)` — no hardened bit (`| 0x80000000`) involved. The constants encode the raw integers directly so Phase 2 can iterate them through `derive_unhardened` without bit-fiddling. If a later plan needs hardened paths for a different purpose, it should define separate constants (e.g., `SCAN_PATH_HARDENED`) rather than overloading the meaning of these.
- **`pub const &[u32]` (slice), not `[u32; 4]`.** The slice type composes naturally with `for &index in PATH { ... }` patterns and `.iter().copied()`, which is what Phase 2 will use. A `[u32; 4]` would force callers to know the length statically, which adds no safety here (the path length is part of the protocol, not a runtime concern) but loses composability.
- **No tests in this plan.** Path values are validated end-to-end at Phase 2 (ADDR-01) when `SilentPaymentKeys::from_mnemonic` runs against the published CHIP-0057 test vector `b_scan = 132567e4...690f6` and `b_spend = 53d140b3...31b087`. A unit test here would either tautologically assert the value against itself or duplicate Phase 2's vector test — both wasteful. The plan explicitly forbids unit tests at this layer for this reason.
- **Two-task structure preserved.** Task 1 creates the file without wiring it; Task 2 wires it into mod.rs. The plan structures this as two distinct commits because (a) it produces a clean git history showing the file landing and then the module becoming reachable as separate events, and (b) it mirrors Plan 03's two-task pattern. The alternative — folding both into one commit — would be slightly faster but lose audit-trail granularity.

## Deviations from Plan

None — plan executed exactly as written.

Both task actions matched the plan verbatim: the file contents in Task 1 are the verbatim block from the plan's `<action>` field, and the mod.rs edit in Task 2 inserted exactly `mod paths;` and `pub use paths::*;` BEFORE the scalar lines without touching any other lines. All acceptance criteria in both tasks (grep checks for the literal constant declarations, build success on `-F chip-0057`, build success with no features, clippy `-D warnings`, the `! grep -r 'mod_by_group_order'` ban) verified green before each commit.

## Issues Encountered

None. No build failures, no clippy warnings, no fmt diffs, no test regressions. All 10 tests from Plans 02 + 03 (5 scalar tests + 5 tagged_hash tests) passed on first run after Task 2's mod.rs edit landed.

## Self-Check: PASSED

Verified all artifacts and commits exist:

- `crates/chia-sdk-types/src/silent_payments/paths.rs` — FOUND (25 lines, ≥ 12 per `min_lines` constraint)
- `crates/chia-sdk-types/src/silent_payments/mod.rs` — FOUND with both `mod paths;` and `pub use paths::*;` present, BEFORE `mod scalar;` (verified via grep + visual inspection)
- Commit `2fc7b9cc` (Task 1, `feat(01-04): add CHIP-0057 derivation path constants (SCAN_PATH, SPEND_PATH)`) — FOUND in `git log`
- Commit `ed8cd93c` (Task 2, `feat(01-04): wire paths module into silent_payments barrel`) — FOUND in `git log`
- Required patterns: `pub const SCAN_PATH: &[u32] = &[12381, 8444, 12, 0];`, `pub const SPEND_PATH: &[u32] = &[12381, 8444, 13, 0];`, `^mod paths;$`, `^pub use paths::\*;$` — all FOUND
- Forbidden pattern: `! grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` — exits 0 (no matches across paths.rs, scalar.rs, tagged_hash.rs, mod.rs)
- `cargo build --release -p chia-sdk-types -F chip-0057` — FINISHED OK
- `cargo build --release -p chia-sdk-types` (no features) — FINISHED OK (gating preserved)
- `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` — 10 passed; 0 failed; 0 ignored
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` — FINISHED OK
- `cargo fmt -p chia-sdk-types -- --files-with-diff --check` — exit 0

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-05 (machete + clippy + grep-ban gate closure)** inherits the final state of `silent_payments/` with three submodules (`paths`, `scalar`, `tagged_hash`) in sorted ordering. The cumulative grep bans (`! grep -rE '^use sha2::' silent_payments/`, `! grep -rE 'Sha256::digest' silent_payments/`, `! grep -r 'mod_by_group_order' silent_payments/`) are all already green at this point and need only be re-verified.
- **Phase 2 (ADDR-01, key derivation)** can `use chia_sdk_types::silent_payments::{SCAN_PATH, SPEND_PATH};` and pass the slices through a fold over `chia_bls::DerivableKey::derive_unhardened(index)` for `SilentPaymentKeys::from_mnemonic`. The known CHIP-0057 vectors (`b_scan = 132567e4...690f6`, `b_spend = 53d140b3...31b087`) provide the end-to-end correctness test for the path values pinned here.
- **Future plans** that add more constants-only files under `silent_payments/` should continue the same pattern: `pub const`s in their own file, doc-comments describing protocol significance, no unit tests at the constants layer, validation happens where the constants are consumed.

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
