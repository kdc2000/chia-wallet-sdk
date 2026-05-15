---
phase: 01-crypto-primitives-workspace-integration
plan: 02
subsystem: infra
tags: [crypto, chip-0057, silent-payments, scalar-field, bls12-381, num-bigint]

requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "chip-0057 feature flag at workspace + chia-sdk-types (Plan 01-01)"

provides:
  - "chia_sdk_types::silent_payments module (gated by chip-0057)"
  - "ScalarField newtype with unsigned mod-r arithmetic over BLS12-381's r"
  - "GROUP_ORDER pub const [u8; 32] matching chia_puzzle_types::derive_synthetic byte-for-byte"
  - "from_bytes_unsigned / from_bytes_raw type-boundary distinction (no From<[u8;32]>)"
  - "Five named unit tests pinning the unsigned-reduction protocol semantics"

affects:
  - 01-03 (tagged_hash — sibling submodule under silent_payments; appends mod entry after `scalar` in mod.rs)
  - 01-04 (paths — sibling submodule under silent_payments; prepends mod entry before `scalar` in mod.rs)
  - 01-05 (machete + clippy + grep-ban gate closure across the cumulative module)
  - 02-XX (Address & key types — consumes ScalarField for synthetic key derivation paths)
  - 03-XX (Receive primitive — input_hash / shared_secret pipeline flows through ScalarField)
  - 04-XX (Send-side action — output_tweak computation flows through ScalarField)

tech-stack:
  added: []
  patterns:
    - "Newtype-as-type-boundary: ScalarField forbids From<[u8;32]> so callers must explicitly pick from_bytes_unsigned vs from_bytes_raw"
    - "Workspace-dep wiring without new [workspace.dependencies]: num-bigint already declared at root, only the [dependencies] inclusion in chia-sdk-types is new"
    - "Module barrel uses descriptive phrasing (not the literal forbidden token) so the Plan 05 grep ban (`! grep -r 'mod_by_group_order' silent_payments/`) holds trivially"

key-files:
  created:
    - "crates/chia-sdk-types/src/silent_payments/mod.rs"
    - "crates/chia-sdk-types/src/silent_payments/scalar.rs"
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-02-SUMMARY.md"
  modified:
    - "crates/chia-sdk-types/Cargo.toml"
    - "crates/chia-sdk-types/src/lib.rs"

key-decisions:
  - "Test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is preserved as locked by VALIDATION.md but is a misnomer; the actual reduction is `(2^256 - 1) - r`, not `r - 1`. Test pins the actual reduced value and additionally asserts inequality with the raw input."
  - "No `From<[u8;32]>` impl on ScalarField — callers must explicitly choose reducing vs non-reducing constructor; this is the type-boundary that prevents the signed/unsigned reducer hazard from leaking into the protocol path."
  - "Derived Copy in addition to Clone (workspace `missing_copy_implementations = warn` nudges this for a 32-byte newtype); does not affect API surface but allows ergonomic use in tests and downstream call sites."
  - "Module barrel doc-comment describes the signed reducer asymmetry without ever naming it; Plan 05's grep ban becomes trivially satisfied."
  - "Private helper `biguint_to_be_bytes_32` factored out of the three reduction sites (from_bytes_unsigned, add, mul) — DRY without changing public surface."

patterns-established:
  - "silent_payments/ submodules use chia-sha2 only (no `use sha2::` anywhere in the module tree). Plan 03's tagged_hash inherits this precedent."
  - "Public methods carry `#[must_use]` (matches workspace `pedantic = warn` posture); `must_use_candidate = allow` in workspace lints means it's not required, but applied consistently here."
  - "mod.rs barrel uses `mod X; pub use X::*;` pairs in fixed sorted positions; Plan 03 inserts after `scalar`, Plan 04 prepends before `scalar`."

requirements-completed: [CRYPTO-01]

duration: 7min
completed: 2026-05-15
---

# Phase 01 Plan 02: ScalarField + GROUP_ORDER for CHIP-0057 Summary

**`chia_sdk_types::silent_payments::ScalarField` newtype lands behind `chip-0057`, enforcing unsigned mod-r reduction over BLS12-381's subgroup order via `num_bigint::BigUint` and pinning the unsigned-vs-signed reducer asymmetry through a deliberate absence of `From<[u8; 32]>`.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-05-15T16:00:12Z
- **Completed:** 2026-05-15T16:08:09Z
- **Tasks:** 2
- **Files modified:** 4 (2 new + 2 edited)

## Accomplishments

- New `chia_sdk_types::silent_payments` module (gated by `chip-0057`) — first code lands behind the gate scaffolded in Plan 01-01.
- `ScalarField` newtype with `from_bytes_unsigned` (reduces mod `r`) and `from_bytes_raw` (does not), plus `add` / `mul` / `as_bytes` / `to_bytes` / `is_zero`. No `From<[u8; 32]>` — the type boundary forces callers to make the unsigned-vs-signed choice explicit.
- `GROUP_ORDER` `pub const [u8; 32]` matches `chia_puzzle_types::derive_synthetic::GROUP_ORDER_BYTES` byte-for-byte.
- Five named unit tests passing under `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests` — names locked by VALIDATION.md rows 43-47.
- All five plan-level gates green: per-crate `-F chip-0057` build, no-features build (gate honored), `cargo clippy --all-features --all-targets -- -D warnings`, `cargo fmt --check`, full-workspace `cargo build --release --workspace`.
- Module-level doc-comment in `mod.rs` and per-function doc-comments in `scalar.rs` describe the signed-reducer asymmetry using descriptive phrasing only; Plan 05's grep ban (`! grep -r 'mod_by_group_order' silent_payments/`) holds trivially because the literal token appears nowhere in the module tree.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add `num-bigint` to `chia-sdk-types/[dependencies]`** — `27db3886` (chore)
2. **Task 2: Create `silent_payments/{mod,scalar}.rs` + wire `lib.rs` gate** — `b74092c8` (feat)

**Plan metadata commit:** appended after this SUMMARY.md (includes SUMMARY.md, STATE.md, ROADMAP.md, REQUIREMENTS.md updates).

## Files Created/Modified

- `crates/chia-sdk-types/Cargo.toml` — added `num-bigint = { workspace = true }` to `[dependencies]` (alphabetical, between `hex-literal` and `thiserror`). No new `[workspace.dependencies]` entry — `num-bigint = "0.4.6"` already declared at root.
- `crates/chia-sdk-types/src/lib.rs` — inserted `#[cfg(feature = "chip-0057")] pub mod silent_payments;` after the existing `pub mod puzzles;` line.
- `crates/chia-sdk-types/src/silent_payments/mod.rs` — NEW (13 lines). Module barrel with `mod scalar;` / `pub use scalar::*;` and a doc-comment describing the unsigned-vs-signed reducer asymmetry using descriptive phrasing only.
- `crates/chia-sdk-types/src/silent_payments/scalar.rs` — NEW (186 lines). `ScalarField` + `GROUP_ORDER` + five named tests; ports `~/silent-payments/crates/sp-common/src/scalar.rs` with SDK-conformance changes (added `Copy` derive, added `#[must_use]` on every public function, tightened doc-comments, factored private helper `biguint_to_be_bytes_32`, renamed tests to match VALIDATION.md, and the deliberate non-port of `From<[u8;32]>`).

## Decisions Made

- **Test name preserved despite math discrepancy.** The locked test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` is mathematically inaccurate — see "Deviations" below. The name is kept (VALIDATION.md is the source of truth for test names) and the assertion is updated to pin the correct value plus an inequality with the raw input.
- **No `From<[u8;32]>` impl.** The whole purpose of the newtype is to force the unsigned-vs-signed choice. Adding `From` would silently swallow that choice and reintroduce the protocol hazard documented in RESEARCH.md Pitfall 1.
- **`Copy` derive added.** Workspace `missing_copy_implementations = warn` nudges this for a 32-byte newtype, and downstream call sites (Plans 03/04 onward) will appreciate the ergonomic use.
- **`#[must_use]` on every public function.** Matches workspace `pedantic = warn` posture; not strictly required (`must_use_candidate = allow`), applied consistently here as the precedent for the rest of `silent_payments/`.
- **Private helper `biguint_to_be_bytes_32`.** Three callers (`from_bytes_unsigned`, `add`, `mul`) all need to left-pad a `BigUint` into 32 big-endian bytes; factored out to keep the body of each public method narrow.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Test-expectation math mismatch (`from_bytes_unsigned_max_input_reduces_to_r_minus_one`)**

- **Found during:** Task 2 (writing the adversarial test).
- **Issue:** The plan, prompt, and PLAN.md's `<behavior>` block all claim that `ScalarField::from_bytes_unsigned([0xff; 32]).to_bytes() == r - 1` big-endian. The mathematics says otherwise. With `r = 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001` and `max = 2^256 - 1 = 0xff..ff`, `floor(max / r) = 1`, so `max mod r = max - r * 1 = 0x1824b159acc5056f998c4fefecbc4ff55884b7fa0003480200000001fffffffd`. That is neither `r - 1` (= `0x73eda7...ff00000000`) nor `[0xff; 32]`. Pinning to `r - 1` would have produced a test that always failed against any correct implementation.
- **Fix:** Kept the locked test name `from_bytes_unsigned_max_input_reduces_to_r_minus_one` (VALIDATION.md is authoritative for test names; PLAN.md row 43 commits to this exact name with `--exact`). Replaced the assertion with two assertions:
  1. `s.to_bytes() == 0x1824b159...fffffffd` (the actual reduced value, hard-coded as a 32-byte array literal).
  2. `s.to_bytes() != [0xff; 32]` (captures the spirit of the success criterion: "unsigned reduction did fire").
  Documented the discrepancy inline in the test body with a multi-line comment explaining why the name is a misnomer and what the test actually asserts.
- **Files modified:** `crates/chia-sdk-types/src/silent_payments/scalar.rs` (test body + inline comment).
- **Verification:** Test passes; output of `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_unsigned_max_input_reduces_to_r_minus_one -- --exact` exits 0. Math cross-checked via Python (`(2**256 - 1) % r`).
- **Committed in:** `b74092c8` (Task 2 commit).

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug in test expectation).

**Impact on plan:** No scope change. The five test names remain exactly as VALIDATION.md mandates. The semantic guarantee the test enforces is stronger than the plan asked for — it pins the exact reduced value (so any drift in the reduction implementation is caught immediately) AND asserts inequality with the raw input. Downstream plans (03, 04) are not affected.

**Recommendation for Plan 05 / Phase 1 closure:** Consider patching VALIDATION.md row 43 (and PLAN.md's `<behavior>` line for this test) to read `..._reduces_correctly` or `..._reduces_to_pinned_value` instead of `..._reduces_to_r_minus_one`. The misnomer is harmless but a hazard for future readers who trust the test name. Left to Plan 05 since renaming the test would require a coordinated edit across VALIDATION.md, PLAN.md, and the test source; this plan is scoped to landing the code, not editing predecessor planning artifacts.

## Issues Encountered

- **Initially considered:** whether to implement `std::ops::Mul`/`std::ops::Add` traits in addition to the inherent `mul`/`add` methods. Decided against: the reference impl uses methods, clippy doesn't flag this (signature differs from the trait — by-reference + non-generic), and adding trait impls is downstream-API drift that belongs in a later plan if it's wanted at all.
- **Initially considered:** whether to make `from_bytes_raw` return `Result<Self, OutOfRangeError>` or just wrap unchecked. Stuck with unchecked wrap because the reference impl does, the validation contract for `from_bytes_raw_does_not_reduce` mandates that raw `[0xff; 32]` round-trips unchanged, and a fallible signature would change the API surface for downstream plans.
- No build failures, no clippy warnings, no fmt diffs. All five named tests passed on first run after the implementation landed.

## Self-Check: PASSED

Verified all artifacts and commits exist:

- `crates/chia-sdk-types/src/silent_payments/mod.rs` — FOUND (13 lines)
- `crates/chia-sdk-types/src/silent_payments/scalar.rs` — FOUND (186 lines, ≥ 80 per `min_lines` constraint)
- `crates/chia-sdk-types/Cargo.toml` line containing `^num-bigint = { workspace = true }$` — FOUND (exit 0)
- `crates/chia-sdk-types/src/lib.rs` contains both `#[cfg(feature = "chip-0057")]` and `pub mod silent_payments;` — FOUND
- Commit `27db3886` (Task 1, `chore(01-02): add num-bigint`) — FOUND in `git log`
- Commit `b74092c8` (Task 2, `feat(01-02): add ScalarField + GROUP_ORDER`) — FOUND in `git log`
- Forbidden patterns: `grep -E '^impl From<\[u8; 32\]> for ScalarField' scalar.rs` → no match (exit 1) — VERIFIED
- Forbidden patterns: `grep -r 'mod_by_group_order' silent_payments/` → no match (exit 1) — VERIFIED
- Forbidden patterns: `grep -rE '^use sha2::' silent_payments/` → no match (exit 1) — VERIFIED
- Required patterns: `pub struct ScalarField`, `pub const GROUP_ORDER: [u8; 32]`, `#[derive(Clone, Copy, Debug, PartialEq, Eq)]` all FOUND in `scalar.rs`
- All five named tests pass under `cargo test ... -- --exact` (verified individually)
- `cargo build --release --workspace` (no features) — FINISHED OK
- `cargo build --release -p chia-sdk-types -F chip-0057` — FINISHED OK
- `cargo clippy -p chia-sdk-types --all-features --all-targets -- -D warnings` — FINISHED OK
- `cargo fmt --all -- --files-with-diff --check` — FINISHED OK (exit 0)

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-03 (`tagged_hash` + test vectors)** can append `mod tagged_hash; pub use tagged_hash::*;` after the existing `mod scalar; pub use scalar::*;` block in `silent_payments/mod.rs`. The precedent for `chia-sha2`-only hashing (no `use sha2::`) is established by the absence of any sha2 import in this plan's files.
- **Plan 01-04 (`paths` constants)** can prepend `mod paths; pub use paths::*;` before the existing `mod scalar; pub use scalar::*;` block — same barrel pattern, fixed sorted position.
- **Plan 01-05 (machete + clippy + grep-ban closure)** inherits a clean state: no new `[workspace.dependencies]` entries, no unused deps in `chia-sdk-types`, no `mod_by_group_order` literal anywhere under `silent_payments/`, no `use sha2::` anywhere under `silent_payments/`.
- **Phase 2 / 3 / 4 (downstream)** can consume `ScalarField` for synthetic-key derivation paths (Phase 2), input-hash / shared-secret pipelines (Phase 3), and output-tweak computation (Phase 4) — all via the type-boundary-enforced unsigned reduction.
- **Open recommendation:** Plan 05 should consider patching the misnomer in the test name (see Deviations section). Left as a soft recommendation, not a blocker.

---
*Phase: 01-crypto-primitives-workspace-integration*
*Completed: 2026-05-15*
