---
phase: 03-receive-primitive-chip-test-vector-closure
plan: 02
subsystem: crypto
tags: [chip-0057, silent-payments, protocol-primitives, ecdh, scalar-field-boundary, chia-sdk-driver, crypto-03, recv-03, tv1]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "ScalarField::from_bytes_unsigned (BLS12-381 subgroup-order unsigned mod-r reducer) and ScalarField::from_bytes_raw (no-reduction escape hatch); tagged_hash + CHIA_SP_SHARED_SECRET (BIP-340 construction via chia_sha2); the three Phase 1 grep bans (no mod_by_group_order, no `^use sha2::`, no Sha256::digest in silent_payments/) — all three still hold after this plan"
  - phase: 02-address-key-types
    provides: "no direct API consumption (Phase 3 is transport-agnostic crypto); Phase 2's chip-0057 cascade pattern is reused but cascade landed in Plan 03-01"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 01
    provides: "crates/chia-sdk-driver/src/silent_payments/{mod.rs,types.rs} scaffold; #[cfg(feature = chip-0057)] mod silent_payments; in lib.rs; chip-0057 driver-crate feature cascade now activates dep:chia-sdk-utils + chia-sdk-utils/chip-0057; DriverError::SilentPayment(#[from] SilentPaymentError) variant; the wire types TweakData/OutputMeta/DetectedSpCoin"
provides:
  - "crates/chia-sdk-driver/src/silent_payments/protocol.rs — five public functions (compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) implementing the CHIP-0057 protocol primitives that both the scanner (Plan 03-03) and the Phase 4 send-side action consume"
  - "TV1 byte-level pin: tv1_shared_secret_matches asserts compute_shared_secret_from_tweak(TV1_scan_sk, TV1_tweak_point) == d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6 (RECV-03 closure)"
  - "CRYPTO-03 success criterion 3 (adversarial scalar boundary): adversarial_ff32_scalar_reduces_unsigned verifies derive_output_tweak([0xff;32], 0) flows through ScalarField::from_bytes_unsigned — the result has first byte < 0x80 (i.e., reduced mod r) and is deterministic + equal to the direct ScalarField construction"
  - "silent_payments/mod.rs barrel now declares mod protocol; pub use protocol::*; mod types; pub use types::*; (sorted protocol < types) — establishes the slot pattern Plans 03-03 / 03-04 will append into"
  - "+2 silent_payments tests on chia-sdk-driver (3 total now; 12 more land in 03-03..05)"
affects:
  - Plan 03-03 (scanner core) — consumes all five protocol primitives via `use super::*;` (or `use super::{compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk};`). The scanner's TV1, TV4, identity-element-guard tests can now compare detected_ph against the unique value returned by puzzle_hash_for_pk(derive_onetime_pk(spend_pk, derive_output_tweak(secret, k))).
  - Plan 03-04 (labeled detection branch) — reuses derive_onetime_pk, derive_onetime_sk, and puzzle_hash_for_pk for the labeled candidate path; the bespoke k=1 test will exercise derive_output_tweak's `to_be_bytes` choice end-to-end.
  - Plan 03-05 (DOS guard + CI matrix + prelude re-export) — will re-export the five new pub fns through src/prelude.rs alongside scan_from_tweaks.
  - Phase 04 (send-side action) — `SilentPaymentSend` will call derive_output_tweak, derive_onetime_pk, derive_onetime_sk directly (the send-side derive_one_time_puzzle_hash composes all three plus puzzle_hash_for_pk).
  - Phase 05 (bindings) — these five functions are bindings-clean by construction (input: &SecretKey/&PublicKey/&[u8;32]/&ScalarField/u32; output: [u8;32]/ScalarField/PublicKey/SecretKey/Bytes32). bindings/silent_payments.json will list them under static_functions in Phase 5.

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "End-to-end ScalarField boundary enforcement: derive_output_tweak's `tagged_hash` output flows through `ScalarField::from_bytes_unsigned`; derive_onetime_pk reads the canonical 32-byte big-endian encoding via `tweak.as_bytes()` and round-trips through chia_bls::SecretKey::from_bytes (which only accepts values < r) with a load-bearing .expect that names the Phase 1 invariant; derive_onetime_sk uses `from_bytes_raw` on the spend_sk bytes (preserves bit pattern; chia_bls::SecretKey already enforces < r) and `add` reduces mod r. The two reducer choices are encoded in the function bodies and documented at the call sites — no signed reducer is reachable from this module."
    - "Big-endian ser32(k) per CHIP-0057 §169: `k.to_be_bytes()` in derive_output_tweak (NOT to_le_bytes). TV1/TV3/TV4 are all k=0 so they don't catch a swap; the bespoke k=1 test in Plan 03-04 is the regression guard."
    - "Pedantic-clippy avoidance pattern: derive_onetime_pk rebinds the tweak SecretKey local as `tweak_secret` (was `tweak_sk` in sp-common reference impl; clippy::similar_names triggered against the existing `tweak: &ScalarField` parameter and the line-local `tweak_pk`). Refactored to one line: `spend_pk + &tweak_secret.public_key()`. No #[allow] attribute (phase rule)."
    - "Documentation-as-fence: every fn doc-comment names the Phase 1 boundary it crosses — derive_output_tweak's body says 'flows through ScalarField::from_bytes_unsigned'; derive_onetime_sk's body says 'from_bytes_raw preserves bit pattern'; puzzle_hash_for_pk's body says 'the silent-payments ScalarField boundary applies only to the silent-payments output tweak, not the standard-puzzle synthetic offset'. The signed-vs-unsigned distinction is documented exactly where a reviewer would look for it."

key-files:
  created:
    - "crates/chia-sdk-driver/src/silent_payments/protocol.rs (200 lines — 5 public fns + 2 named tests + TV1 pinned-byte constants)"
  modified:
    - "crates/chia-sdk-driver/src/silent_payments/mod.rs (+2 lines: `mod protocol;` and `pub use protocol::*;` inserted before `mod types;` per sorted order protocol < types)"

key-decisions:
  - "derive_onetime_pk rebinding from `tweak_sk` (sp-common reference) to `tweak_secret` to avoid clippy::similar_names against the parameter `tweak`. Inlined `tweak_pk` into the addition expression: `spend_pk + &tweak_secret.public_key()`. Total 4-line function (down from 5). No #[allow] (phase rule)."
  - "Adversarial test asserts (a) first byte < 0x80 (proves unsigned reduction fired), (b) determinism via re-derivation, and (c) equality with the direct ScalarField::from_bytes_unsigned(tagged_hash(...)) path. Three assertions together pin the unsigned-reduction route end-to-end through derive_output_tweak; a future regression that swapped from_bytes_unsigned for a signed reducer would either produce a high-bit-set first byte (assertion a fails), or fail assertion c (the direct path uses the unsigned reducer explicitly)."
  - "Single atomic commit (079d9e21) for the protocol primitives + tests. The plan declared one task; both functions and tests cross-reference each other, and the build matrix needed all five fns + both tests to validate the change in one shot."
  - "Doc-comment rewrite to drop the literal string `mod_by_group_order`: Plan 03-02 acceptance criteria require `! grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 grep ban). The original docstring referenced the upstream reducer by its name; rephrased to 'the reducer that chia_puzzle_types::derive_synthetic uses internally for the standard-puzzle synthetic offset'. Documentation intent preserved; grep ban honored."

patterns-established:
  - "Five-function CHIP-0057 protocol surface: compute_shared_secret_from_tweak / derive_output_tweak / derive_onetime_pk / derive_onetime_sk / puzzle_hash_for_pk. This is the exact bottom-of-stack the scanner (Plan 03-03), the labeled detection branch (Plan 03-04), and Phase 4's send-side derive_one_time_puzzle_hash all compose. No re-implementation in any downstream plan."
  - "Module-level barrel insertion in sorted order: mod protocol; pub use protocol::*; lands before mod types; pub use types::*; (alphabetical p < t). Plan 03-03 will insert mod scanner; between (p < s < t)."
  - "TV1 pinned-byte test-constant pattern: const TV1_SCAN_SK / TV1_A_SUM / TV1_INPUT_HASH / TV1_SHARED_SECRET as module-level `hex!(...)` literals at the top of the tests submodule, with a tv1_tweak_point() helper that reconstructs the multiplied point from the published bytes. Plans 03-03 and 03-04 will append TV1_T_0 / TV1_ONETIME_PK / TV1_PUZZLE_HASH / TV1_ONETIME_SK and the TV3 constants in the same shape."

requirements-completed: [CRYPTO-03]

# Metrics
duration: 12min
completed: 2026-05-15
---

# Phase 03 Plan 02: Protocol primitives + adversarial scalar test Summary

**Wave 2 of Phase 3: lands all five CHIP-0057 protocol primitives that the scanner (Plan 03-03), the labeled-detection branch (Plan 03-04), and Phase 4's send-side action all consume — plus the TV1 byte-level shared-secret pin and the CRYPTO-03 adversarial scalar test that closes Phase 3 success criterion 3.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-15T21:50:45Z
- **Completed:** 2026-05-15T22:03:31Z (approx)
- **Tasks:** 1 (one atomic commit per plan; both protocol fns and tests inter-reference)
- **Files modified:** 1 (silent_payments/mod.rs); 1 created (silent_payments/protocol.rs)

## Accomplishments

1. **Five public CHIP-0057 protocol primitives** in `crates/chia-sdk-driver/src/silent_payments/protocol.rs`:
   - `compute_shared_secret_from_tweak(scan_sk, tweak_point) -> [u8; 32]` — wallet-side ECDH; `point = scan_sk * tweak_point`, then `SHA256(point.to_bytes())` via `chia_sha2::Sha256::{new,update,finalize}` (NOT `digest`).
   - `derive_output_tweak(shared_secret, k) -> ScalarField` — `tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret ‖ k.to_be_bytes())` reduced via `ScalarField::from_bytes_unsigned`.
   - `derive_onetime_pk(spend_pk, tweak) -> PublicKey` — `spend_pk + tweak * G`, going through `SecretKey::from_bytes(tweak.as_bytes()).public_key()` (the `ScalarField` boundary guarantees `< r`).
   - `derive_onetime_sk(spend_sk, tweak) -> SecretKey` — `(spend_sk + tweak) mod r` via `ScalarField::from_bytes_raw(spend_sk.to_bytes()).add(tweak)`.
   - `puzzle_hash_for_pk(pk) -> Bytes32` — `StandardArgs::curry_tree_hash(pk.derive_synthetic()).into()` (SDK precedent at `crates/chia-sdk-driver/src/layers/standard_layer.rs:118`).
2. **TV1 byte-level shared-secret pin** (`tv1_shared_secret_matches`). Reconstructs `tweak_point = TV1_input_hash * TV1_A_sum` via `PublicKey::from_bytes` + `scalar_multiply`, then asserts `compute_shared_secret_from_tweak(TV1_scan_sk, tweak_point) == d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6`. Closes RECV-03.
3. **CRYPTO-03 adversarial scalar test** (`adversarial_ff32_scalar_reduces_unsigned`). Asserts `derive_output_tweak([0xff;32], 0)` produces a `ScalarField` whose first byte is `< 0x80` (proves unsigned reduction mod r fired; signed reduction would not constrain the high bit this way), is deterministic across two derivations, and equals the direct `ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret ‖ ser32(0)))` value byte-for-byte. Phase 3 success criterion 3 is closed.
4. **Sorted module barrel** in `silent_payments/mod.rs`: `mod protocol; pub use protocol::*; mod types; pub use types::*;` (alphabetical `protocol < types`). Plans 03-03 and 03-04 will append `mod scanner;` between them.
5. **Workspace test count went from 2388 (post-Plan 03-01) to 2390 (+2 new tests).** Full workspace `cargo test --release --all-features` minus binding crates is green.
6. **All Phase 1 grep bans still hold** under `silent_payments/`: no `mod_by_group_order`, no `^use sha2::`, no `Sha256::digest`.

## Task Commits

One atomic commit per plan; both protocol functions and tests cross-reference each other in the same compilation unit:

1. **Task 1 (atomic):** `feat(03-02): silent_payments protocol primitives + TV1/adversarial tests` — `079d9e21`

## Files Created/Modified

### Created
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` (200 lines) — Five public CHIP-0057 protocol primitives + two named tests (`tv1_shared_secret_matches`, `adversarial_ff32_scalar_reduces_unsigned`). All five fns `#[must_use]`. Module-level doc-comment names the five fns and the Phase 1 `ScalarField` boundary.

### Modified
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` (+2 lines) — Added `mod protocol;` and `pub use protocol::*;` immediately before the existing `mod types;` declaration (sorted order `protocol < types`).

## Decisions Made

1. **Single atomic commit for Task 1.** The plan declared one task; the five protocol fns and the two tests inter-reference each other (the tests call `compute_shared_secret_from_tweak` and `derive_output_tweak` from the same module). Splitting into RED/GREEN commits would have produced an intermediate broken state.
2. **`derive_onetime_pk` local rebinding from `tweak_sk` to `tweak_secret`.** Clippy `pedantic` `similar_names` fires when `tweak_sk` and `tweak_pk` exist as adjacent locals in a function whose parameter is named `tweak`. Refactored to (a) rename the SecretKey local to `tweak_secret`, (b) inline `tweak_pk` directly into the addition expression: `spend_pk + &tweak_secret.public_key()`. Net delta is one fewer local binding and one fewer doc-comment to maintain. No `#[allow]` (phase rule).
3. **Doc-comment phrasing to honor the Phase 1 grep ban.** The adversarial test's doc-comment originally explained the boundary by naming `chia_puzzle_types::derive_synthetic`'s internal reducer literally. The plan's acceptance criteria assert `! grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/`; rephrased to "the reducer that `chia_puzzle_types::derive_synthetic` uses internally for the standard-puzzle synthetic offset". Documentation intent preserved exactly; ban honored.
4. **TV1 pinned-byte constants defined at the top of the `tests` submodule via `hex_literal::hex!`** rather than constructed via test-time computation. `hex-literal` is already a workspace dep (already used in `tagged_hash::tests`). The four constants (`TV1_SCAN_SK`, `TV1_A_SUM`, `TV1_INPUT_HASH`, `TV1_SHARED_SECRET`) match the bytes published in `03-RESEARCH.md` §10a verbatim. Plans 03-03/03-04 will append `TV1_T_0`, `TV1_ONETIME_PK`, `TV1_PUZZLE_HASH`, `TV1_ONETIME_SK` constants in the same shape.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] `derive_onetime_pk` `tweak_sk`/`tweak_pk` rebinding**
- **Found during:** Task 1 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** The plan's `<action>` Step B sample code uses `let tweak_sk = SecretKey::from_bytes(...); let tweak_pk = tweak_sk.public_key();`. Clippy `pedantic`'s `similar_names` flagged `tweak_pk` as too similar to `tweak_sk` (existing binding two lines above). Workspace `clippy --all-features --all-targets -- -D warnings` (the local strict gate) turns this into an error.
- **Fix:** Renamed the SecretKey local to `tweak_secret` and inlined the public_key() call: `spend_pk + &tweak_secret.public_key()`. Net effect: one fewer line, one fewer name to keep distinct from `tweak: &ScalarField` (the parameter).
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs:67-70`
- **Verification:** `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0 after the change. Tests still pass.
- **Committed in:** `079d9e21` (the single Task 1 commit)

**2. [Rule 1 - Lint] Three `doc_markdown` violations on test-module doc-comments**
- **Found during:** Task 1 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** Three doc-comment lines used bare identifiers (`A_sum`, `input_hash`, `tagged_hash`) without backticks. Clippy `pedantic`'s `doc_markdown` lint flagged each. Workspace strict-clippy turns these into errors.
- **Fix:** Wrapped `tweak_point = input_hash * A_sum` and `tagged_hash` in backticks in the affected lines. No semantic change.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs:122,148`
- **Verification:** Strict clippy exits 0 post-fix.
- **Committed in:** `079d9e21`

**3. [Rule 1 - Lint] Phase 1 grep-ban literal in doc-comment**
- **Found during:** Acceptance criteria post-check (`grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/`)
- **Issue:** The adversarial test's doc-comment originally referenced `chia_puzzle_types::derive_synthetic`'s internal reducer by name. The literal string appeared in the doc-comment text. The plan's acceptance criteria require zero hits for that grep.
- **Fix:** Rephrased to "the reducer that `chia_puzzle_types::derive_synthetic` uses internally for the standard-puzzle synthetic offset". Same documentation intent; ban honored.
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs:153-157`
- **Verification:** `grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` returns zero hits post-fix.
- **Committed in:** `079d9e21`

---

**Total deviations:** 3 auto-fixed (3 lint/policy; zero semantic).
**Impact on plan:** Net delta is +1 stronger assertion in the adversarial test (the direct `ScalarField::from_bytes_unsigned(tagged_hash(...))` cross-check became a third assertion), -1 local binding in `derive_onetime_pk`, and three doc-comment polish edits. No API change, no semantic change, no test name change.

## Issues Encountered

None during the planned work. Clippy `pedantic`'s `similar_names` + `doc_markdown` lints are the only friction (consistent with the Phase 2 `clone_on_copy` / `similar_names` polish lints) and were three small inline fixes.

## Phase 3 Plan 02 Gate — Final Status

| ID | Check | Status |
|----|-------|--------|
| G1 | `cargo build --release -p chia-sdk-driver` (no features) clean | PASS |
| G2 | `cargo build --release -p chia-sdk-driver -F chip-0057` clean | PASS |
| G3 | `cargo build --release -p chia-sdk-driver --all-features` clean | PASS |
| G4 | `cargo build --release --workspace --all-features` clean | PASS |
| G5 | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean | PASS |
| G6 | `cargo fmt --all -- --files-with-diff --check` clean | PASS |
| G7 | `cargo machete` clean, no new ignored entries | PASS |
| G8 | `! grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G9 | `! grep -rE '^use sha2::' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G10 | `! grep -rE 'Sha256::digest' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban) | PASS (zero matches) |
| G11 | `! grep -r '#\[allow' crates/chia-sdk-driver/src/silent_payments/` (no allow attrs) | PASS (zero matches) |
| G12 | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::protocol::tests::tv1_shared_secret_matches -- --exact` passes | PASS (1 passed) |
| G13 | `cargo test --release -p chia-sdk-driver --features chip-0057 silent_payments::protocol::tests::adversarial_ff32_scalar_reduces_unsigned -- --exact` passes | PASS (1 passed) |
| G14 | Full CI test suite (workspace --all-features minus binding crates) passes | PASS (2390 passed, 0 failed — was 2388 baseline, +2 new tests) |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Plan 03-03 (scanner core) unblocked.** The five protocol primitives are pub and available via `use super::*;` (or specific names) from the new `scanner.rs`. The scanner's `scan_from_tweaks` composes `compute_shared_secret_from_tweak` (once per `tweak_point`) and a `k`-loop that calls `derive_output_tweak → derive_onetime_pk → puzzle_hash_for_pk`. The TV1 + TV4 unit tests inherit the `TV1_SCAN_SK`/`TV1_A_SUM`/`TV1_INPUT_HASH`/`TV1_SHARED_SECRET` constant style from `protocol.rs::tests` (just append a `TV1_SPEND_SK`, `TV1_T_0`, `TV1_ONETIME_PK`, `TV1_PUZZLE_HASH`).

**Plan 03-04 (labeled detection branch) unblocked.** Reuses `derive_onetime_pk`, `derive_onetime_sk`, and `puzzle_hash_for_pk` for the labeled candidate path. The bespoke `k = 1` test will exercise `derive_output_tweak`'s `to_be_bytes` choice end-to-end (regression guard for the `ser32(k)` endianness bug that TV1/TV3/TV4 — all `k = 0` — cannot catch).

**Plan 03-05 (DOS guard + prelude re-export + final gate).** The five new `pub fn`s will join `scan_from_tweaks` in the `src/prelude.rs` `#[cfg(feature = "chip-0057")]` re-export block.

**Phase 04 (send-side action).** `SilentPaymentSend` will call `derive_output_tweak` + `derive_onetime_pk` + `derive_onetime_sk` directly for send-side construction. The TV1 shared-secret pin gives Phase 4 a byte-level cross-check (sender-side ECDH must produce the same shared secret as the scanner's `compute_shared_secret_from_tweak` on TV1).

**Phase 05 (bindings).** The five fns are bindings-clean: input/output types are all in the bindings-supported type-groups (`Bytes32 = {bytes}`, `[u8; 32] = {bytes}`, `u32 = {number}`, `PublicKey`/`SecretKey`/`ScalarField` already covered). `bindings/silent_payments.json` will declare them under `static_functions` in Phase 5.

**No blockers for Plans 03-03..05.**

## Self-Check: PASSED

Verified all claims:
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` exists (FOUND)
- `git log --oneline --all | grep -q '079d9e21'` (FOUND)
- `grep -c 'pub fn compute_shared_secret_from_tweak\|pub fn derive_output_tweak\|pub fn derive_onetime_pk\|pub fn derive_onetime_sk\|pub fn puzzle_hash_for_pk' crates/chia-sdk-driver/src/silent_payments/protocol.rs` returns 5 (FOUND)
- `grep -c 'k\.to_be_bytes()\|ScalarField::from_bytes_unsigned\|ScalarField::from_bytes_raw\|StandardArgs::curry_tree_hash' crates/chia-sdk-driver/src/silent_payments/protocol.rs` returns 14 (FOUND — multiple references across fns + tests)
- `mod protocol; pub use protocol::*;` present in `silent_payments/mod.rs` (FOUND)
- Phase 1 grep bans all return zero hits under `crates/chia-sdk-driver/src/silent_payments/`
- No `#[allow]` attributes anywhere in `silent_payments/`
- Both named tests pass under `--exact`
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo fmt --all -- --files-with-diff --check` exits 0 post-fmt

---
*Phase: 03-receive-primitive-chip-test-vector-closure*
*Plan: 02*
*Completed: 2026-05-15*
