---
phase: 01-crypto-primitives-workspace-integration
plan: 03
subsystem: infra
tags: [crypto, chip-0057, silent-payments, tagged-hash, bip-340, chia-sha2]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 feature flag at workspace + chia-sdk-types (Plan 01-01)"
  - phase: 01-crypto-primitives-workspace-integration
    provides: "silent_payments/ module barrel (Plan 01-02) — appended `mod tagged_hash; pub use tagged_hash::*;` AFTER the existing `mod scalar; pub use scalar::*;` block"

provides:
  - "chia_sdk_types::silent_payments::tagged_hash(tag, data) — BIP-340-style tagged hash via chia_sha2::Sha256"
  - "Three pub const &'static str tag constants: CHIA_SP_INPUTS, CHIA_SP_SHARED_SECRET, CHIA_SP_LABEL"
  - "Five named tagged_hash unit tests pinning the BIP-340 construction and the three Chia_SP/* tag-string values"

affects:
  - 01-04 (paths — prepends `mod paths;` BEFORE `mod scalar;` in mod.rs; the tagged_hash entries stay in their AFTER-scalar position)
  - 01-05 (machete + clippy + grep-ban gate closure)
  - 03-XX (Receive primitive — consumes tagged_hash for input_hash / shared_secret computation)
  - 04-XX (Send-side action — consumes tagged_hash for output_tweak / input_hash computation)

tech-stack:
  added: []
  patterns:
    - "chia_sha2::Sha256 is the only hashing path: new()/update(bytes)/finalize() — never `::digest(...)` (no such method on chia-sha2)"
    - "Domain-tag typo guard: each pub const &str tag is paired with a hex!(...) test that pins SHA256(tag.as_bytes()); a rename of the tag value breaks the test before any protocol code runs"
    - "Cross-implementation correctness gate via BIP-340 published vector `c216d352...3713` for tagged_hash(\"BIP0340/challenge\", \"\")"

key-files:
  created:
    - "crates/chia-sdk-types/src/silent_payments/tagged_hash.rs"
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-03-SUMMARY.md"
  modified:
    - "crates/chia-sdk-types/src/silent_payments/mod.rs"

key-decisions:
  - "Folded Task 1 (file creation) and the mod.rs wiring into a single commit, mirroring Plan 02's lib.rs-wiring pattern. Avoids any `dead_code = \"deny\"` ordering risk and keeps the new file visible to the build from the moment it lands."
  - "Tag-pin SHA-256 values cross-verified via two independent methods (Python `hashlib.sha256` and the `sha256sum` CLI) before pinning. Both produced byte-identical output for all three tags, so transcription error is ruled out."
  - "Three tag constants are `pub const &'static str` (NOT `&'static [u8]` or hex-encoded byte arrays). This is what makes the typo guard work — the test computes `SHA256(tag.as_bytes())` and pins it, so the constant VALUE (not its hashed bytes) is the source of truth."
  - "Doc-comments call out that the module uses `chia_sha2::Sha256` exclusively — keeps the `! grep -rE '^use sha2::' silent_payments/` ban trivially satisfied."
  - "Private `sha256(bytes)` helper inside `#[cfg(test)] mod tests` factored to keep the three pin-tests narrow; not exposed publicly because the public surface is `tagged_hash`, not a generic SHA-256 helper."

patterns-established:
  - "silent_payments/ submodules continue to use chia-sha2 only (no `use sha2::` anywhere in the module tree). Inherited from Plan 02; reinforced here."
  - "`#[must_use]` on every public function — workspace `pedantic = warn` posture, consistent with Plan 02's precedent."
  - "mod.rs barrel pairs `mod X; pub use X::*;` in fixed sorted positions: `scalar` < `tagged_hash`. Plan 04's `paths` will prepend BEFORE `scalar`."

requirements-completed: [CRYPTO-02]

duration: 4min
completed: 2026-05-15
---

# Phase 01 Plan 03: tagged_hash + Chia_SP/* tag constants Summary

**`chia_sdk_types::silent_payments::tagged_hash` ports the BIP-340 construction into the SDK on top of `chia_sha2::Sha256` (the `new/update/finalize` API — never `::digest`), pins the three CHIP-0057 domain tags as `pub const &'static str`, and locks each tag-string value via a `SHA256(tag.as_bytes()) == hex!(...)` typo-guard test before any downstream protocol code can build on top.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-05-15T16:12:21Z
- **Completed:** 2026-05-15T16:16:15Z
- **Tasks:** 2
- **Files modified:** 2 (1 new + 1 edited)

## Accomplishments

- New `tagged_hash.rs` (98 lines) implements `tagged_hash(tag, data) = SHA256(SHA256(tag) || SHA256(tag) || data)` via `chia_sha2::Sha256::{new, update, finalize}`. No `sha2::` import anywhere; no `::digest(...)` usage.
- Three `pub const &'static str` tag constants — `CHIA_SP_INPUTS`, `CHIA_SP_SHARED_SECRET`, `CHIA_SP_LABEL` — with values exactly matching the CHIP-0057 draft (`"Chia_SP/Inputs"`, `"Chia_SP/SharedSecret"`, `"Chia_SP/Label"`).
- Five named unit tests passing under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests` (names locked by VALIDATION.md rows 48–52):
  - `tagged_hash_matches_bip340_challenge_vector` — cross-implementation correctness via the BIP-340 published value `c216d352...3713`.
  - `tag_inputs_hash_pinned` — typo guard for `CHIA_SP_INPUTS` (SHA-256 = `d44a6db8...3378cf`).
  - `tag_shared_secret_hash_pinned` — typo guard for `CHIA_SP_SHARED_SECRET` (SHA-256 = `e7b8a524...b9ca9f`).
  - `tag_label_hash_pinned` — typo guard for `CHIA_SP_LABEL` (SHA-256 = `c63c8bd2...f94554`).
  - `different_tags_different_outputs` — sanity that the tag parameter matters.
- `silent_payments/mod.rs` now declares `mod tagged_hash; pub use tagged_hash::*;` AFTER the existing `mod scalar; pub use scalar::*;` block — sorted-position invariant preserved for Plan 04 (which prepends `paths` BEFORE `scalar`).
- All plan-level gates green: `-F chip-0057` build, no-features build (gating preserved), clippy `--all-features --all-targets -- -D warnings`, `cargo fmt --check`, plus the three grep bans (`! grep -rE '^use sha2::' silent_payments/`, `! grep -rE 'Sha256::digest' silent_payments/`, `! grep -r 'mod_by_group_order' silent_payments/`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Create `tagged_hash.rs` (scaffold with placeholder pinned bytes) + wire into `mod.rs`** — `66c3dac9` (feat)
2. **Task 2: Replace placeholders with cross-verified SHA-256 bytes for the three tags** — `b394ab5d` (feat)

**Plan metadata commit:** appended after this SUMMARY.md (includes SUMMARY.md, STATE.md, ROADMAP.md, REQUIREMENTS.md updates).

## Files Created/Modified

- `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` — NEW (98 lines). Module-level doc-comment explaining the BIP-340 construction and the `chia_sha2`-only convention; three `pub const &'static str` tag constants; `pub fn tagged_hash(tag, data) -> [u8; 32]`; `#[cfg(test)] mod tests` with the five named tests + a private `sha256` test-only helper.
- `crates/chia-sdk-types/src/silent_payments/mod.rs` — MODIFIED (13 → 15 lines). Appended `mod tagged_hash;` and `pub use tagged_hash::*;` AFTER the existing `mod scalar; pub use scalar::*;` lines, BEFORE the "additional submodules appended in sorted order" comment marker.

## Decisions Made

- **Two-task structure preserved as planned.** Task 1 lands the file (with placeholder hex literals + the mod.rs wiring) in one commit; Task 2 replaces the placeholders in a second commit. This is heavier than a single-commit version, but matches the plan exactly and produces a clean git history showing the typo-guard mechanism going live as a distinct change.
- **Cross-verification before pinning.** Both Python's `hashlib.sha256` and the system `sha256sum` CLI were run on each of the three tag strings; outputs were byte-identical. Only then were the hex literals pasted into the test source. Method B from the plan (cargo-test print) was not needed because Methods A + C already agreed.
- **Doc-comment placement.** Each pinned test carries a comment explaining what the value is and how it was cross-verified. This is the documentation that the test value isn't magic — it's `SHA256(tag.as_bytes())` and any reader can reproduce it in two lines of Python.
- **No `digest`-style helper added.** A naive port from the reference impl (`~/silent-payments/.../tagged_hash.rs`) uses `Sha256::digest(...)` once, but `chia-sha2` doesn't expose that static method (this is RESEARCH.md Pitfall 3 explicitly). The port uses `new/update/finalize` consistently — both inside `tagged_hash` itself AND inside the test-only `sha256` helper.
- **No `pub fn sha256(bytes)` exposed.** The single-shot SHA-256 helper exists only inside `#[cfg(test)] mod tests` because the public surface of this file is `tagged_hash` (the BIP-340 construction), not a generic single-shot hasher. Anyone outside the module who needs `SHA256(bytes)` should use `chia_sha2::Sha256` directly.

## Deviations from Plan

None — plan executed exactly as written.

The plan's two-task structure (scaffold with placeholders → replace placeholders) was followed verbatim. All five tests pass with the names mandated by VALIDATION.md rows 48–52. All acceptance criteria in both tasks (grep checks, build success, individual `-- --exact` test invocations) verified green before each commit.

The plan's Method A (Python one-liner) and Method C (`sha256sum` CLI) were both used as a cross-check before pinning, satisfying the plan's "all three independent methods MUST agree" guidance (Method B was not needed because A + C already agreed byte-for-byte).

## Issues Encountered

- **Initially considered:** whether to fold Task 1 and Task 2 into a single commit (since the SHA-256 values were already known before any code was written). Rejected: the plan explicitly structures them as two separate tasks with two distinct verification gates, and following the plan as written produces a cleaner audit trail showing the typo-guard mechanism going live as a discrete change. The plan also describes the placeholder-then-replace structure as mirroring how a developer would actually do this on day one, which is pedagogically valuable for future maintainers reading the git log.
- No build failures, no clippy warnings, no fmt diffs, no test failures on the real (non-placeholder) bytes. All five named tests passed on first run after Task 2's replacement.

## Self-Check: PASSED

Verified all artifacts and commits exist:

- `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` — FOUND (98 lines, ≥ 60 per `min_lines` constraint)
- `crates/chia-sdk-types/src/silent_payments/mod.rs` — FOUND with both `mod tagged_hash;` and `pub use tagged_hash::*;` present (verified via grep)
- Commit `66c3dac9` (Task 1, `feat(01-03): scaffold tagged_hash + Chia_SP/* tag constants`) — FOUND in `git log`
- Commit `b394ab5d` (Task 2, `feat(01-03): pin SHA-256 bytes for Chia_SP/* tag constants`) — FOUND in `git log`
- Required patterns: `^pub const CHIA_SP_INPUTS: &str = "Chia_SP/Inputs";$`, `^pub const CHIA_SP_SHARED_SECRET: &str = "Chia_SP/SharedSecret";$`, `^pub const CHIA_SP_LABEL: &str = "Chia_SP/Label";$`, `use chia_sha2::Sha256;` — all FOUND
- Forbidden patterns: `! grep -rE '^use sha2::' crates/chia-sdk-types/src/silent_payments/`, `! grep -rE 'Sha256::digest' crates/chia-sdk-types/src/silent_payments/`, `! grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` — all exit 0 (no matches)
- No remaining `hex!("0000...0000")` placeholders in `tagged_hash.rs` (verified)
- 4 `hex!("[0-9a-f]{64}")` literals present (BIP-340 vector + 3 pinned tags) — verified
- All 5 named tests pass under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests` (verified)
- `cargo build --release -p chia-sdk-types -F chip-0057` — FINISHED OK
- `cargo build --release -p chia-sdk-types` (no features) — FINISHED OK (gating preserved)
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` — FINISHED OK
- `cargo fmt -p chia-sdk-types -- --files-with-diff --check` — exit 0

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-04 (`paths` constants)** can prepend `mod paths;` BEFORE `mod scalar;` and `pub use paths::*;` BEFORE `pub use scalar::*;` in `silent_payments/mod.rs` — the existing `mod tagged_hash; pub use tagged_hash::*;` block stays in its AFTER-scalar position. Sorted-position invariant: `paths < scalar < tagged_hash`.
- **Plan 01-05 (machete + clippy + grep-ban closure)** inherits a clean state: no `use sha2::` anywhere under `silent_payments/`, no `Sha256::digest` calls, no `mod_by_group_order` literal. All three grep-ban gates are already green at this point in the phase.
- **Phase 3 (Receive primitive)** can consume `tagged_hash(CHIA_SP_INPUTS, coin_id_l || serialize(A_sum))` for `input_hash` and `tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret || ser32(k))` for `t_k` directly. The tag constants and function are `pub use`-exported through the `silent_payments::*` barrel.
- **Phase 4 (Send-side action)** uses the same primitives for its output-tweak computation. The `#[must_use]` attribute on `tagged_hash` is a soft nudge that callers should bind the return value rather than discard it.
- **CHIP-0058 transport (later)** is unaffected by this plan — `tagged_hash` is a pure function and the tag constants are wire-format-stable (changing them would be a hard fork).

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
