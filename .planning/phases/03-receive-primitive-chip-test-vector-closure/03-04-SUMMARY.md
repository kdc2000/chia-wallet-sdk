---
phase: 03-receive-primitive-chip-test-vector-closure
plan: 04
subsystem: crypto
tags: [chip-0057, silent-payments, scanner, recv-04, crypto-03, tv3, k1-bespoke, labeled-detection, chia-sdk-driver, chia-sdk-utils]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: "ScalarField boundary (ScalarField::from_bytes_raw + add + as_bytes) + tagged_hash; three Phase 1 grep bans hold"
  - phase: 02-address-key-types
    provides: "Phase 2 generate_label (pub(super)) — promoted here to pub(crate) + exposed via pub fn reach-through (RESEARCH §13 Option A); LabelRegistry::iter API surface (unchanged)"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 01
    provides: "TweakData / OutputMeta / DetectedSpCoin wire types (label: Option<u32> field consumed here)"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 02
    provides: "Five protocol primitives (compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk) — exercised in the in-test k=1 derivation"
  - phase: 03-receive-primitive-chip-test-vector-closure
    plan: 03
    provides: "scan_from_tweaks unlabeled branch + PLAN 03-04 APPEND POINT sentinel + let _ = labels placeholder — both consumed here"
provides:
  - "crates/chia-sdk-utils/src/silent_payments/labels.rs — generate_label visibility promoted from pub(super) to pub(crate)"
  - "crates/chia-sdk-utils/src/silent_payments/mod.rs — pub fn generate_label(scan_sk, m) -> (ScalarField, PublicKey) reach-through over labels::generate_label (Option A from RESEARCH §13)"
  - "crates/chia-sdk-driver/src/silent_payments/scanner.rs — labeled-detection branch: when unlabeled missed, iterate label_map.iter(), compute labeled_pk = candidate_pk + label_pk, match against output_phs, push DetectedSpCoin with onetime_sk = base_onetime_sk + label_scalar"
  - "Four new named tests: tv3_scan_detects_labeled_k0 (TV3 pinned onetime_sk = 58fc6195...b64852dc), bespoke_k1_detection (in-test k=1 derivation catches ser32 LE regressions), labeled_k_termination_rule (unlabeled k=0 + labeled k=1 both detected), unlabeled_preferred_over_labeled_at_same_k (exactly one detection when both branches would match)"
  - "Workspace test count: 2393 → 2397 (+4). chia-sdk-driver silent_payments tests: 6 → 10."
  - "RECV-04 closed (labeled detection branch + correct k-termination rule). CRYPTO-03 success criteria 1 (TV3), 2 (bespoke k=1), 6 (labeled k-termination) closed."
affects:
  - "Plan 03-05 (DOS-guard + CI matrix + prelude) — scan_from_tweaks now feature-complete on the algorithm side; Plan 03-05 adds the K_max = 32 DOS-guard test, the bindings method (if any), the CI matrix line, prelude re-exports, and the final phase gate"
  - "Phase 04 (send-side action) — Phase 4's derive_one_time_puzzle_hash uses the same derive_output_tweak chain that bespoke_k1_detection exercises; the k=1 byte-level pin gives Phase 4 a forward cross-check on the ser32 endianness convention"
  - "Phase 06 (simulator E2E SIM-03 labeled) — labeled detection works against TV3-shaped inputs; SIM-03 will exercise the same labeled branch against simulator-generated on-chain coins"
  - "Phase 06 (own-change detection) — generate_label public reach-through makes m = 0 (the change label) reachable from chia-sdk-driver without exposing the raw labels module; SIM-03 sub-test uses this for own-change detection"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Stable Rust 1.88+ if-let-chain syntax extended to the labeled branch: `if !found && let Some(label_map) = labels { ... }` collapses the outer-if + if-let into one pattern. Inside the branch, `if output_phs.contains(&labeled_hash) && let Some(out) = data.outputs.iter().find(|o| o.puzzle_hash == labeled_hash)` mirrors the unlabeled branch's same Plan 03-03 chain pattern."
    - "Option A reach-through from RESEARCH §13: promote helper to `pub(crate)` in its defining file + add a `pub fn` wrapper in the parent mod.rs. The wrapper uses fully-qualified types (`chia_bls::SecretKey`, `chia_sdk_types::silent_payments::ScalarField`, `chia_bls::PublicKey`) so the imports at the top of mod.rs don't need to change. One-line body; no new logic, no new tests at the chia-sdk-utils boundary (Phase 2's tv3_label_scalar_matches still pins the byte-level behavior)."
    - "Test-side in-test k=1 derivation (RESEARCH § Open Question 2 fallback path): compute the k=1 expected puzzle_hash inside the test BODY using the SDK's protocol primitives (compute_shared_secret_from_tweak → derive_output_tweak(.., 1) → derive_onetime_pk → puzzle_hash_for_pk → derive_onetime_sk), then construct the TweakData carrying that puzzle_hash. The test verifies the round-trip (scanner finds the pre-computed puzzle_hash at k=1) and pins the onetime_sk byte pattern. Weaker than pinning hex literals against an independent reference impl, but catches ser32 endianness via the asymmetry between TV1/TV3/TV4 (all k=0) and the bespoke k=1 case."
    - "CHIP-naming conventions in test locals (b_scan, b_spend, b_spend_pub) to dodge clippy::similar_names without resorting to function-scoped allows. The CHIP-0057 spec uses these names in its pseudocode so the locals stay close to the spec's narrative."

key-files:
  created: []
  modified:
    - "crates/chia-sdk-utils/src/silent_payments/labels.rs (+1 line changed: pub(super) → pub(crate) on generate_label)"
    - "crates/chia-sdk-utils/src/silent_payments/mod.rs (+25 lines: pub fn generate_label reach-through with full doc-comment, must_use, fully-qualified types)"
    - "crates/chia-sdk-driver/src/silent_payments/scanner.rs (+295 lines, -12 lines: removed let _ = labels; placeholder, replaced PLAN 03-04 APPEND POINT block with labeled detection branch + termination rule, appended TV3 pinned constants + 4 new tests; added 2 imports: ScalarField and generate_label)"

key-decisions:
  - "Option A (reach-through) chosen over Option B (publicly re-export labels::*) per RESEARCH §13. Keeps the raw labels module module-private; only the curated reach-through is exposed publicly. Cross-crate consumers in chia-sdk-driver use `chia_sdk_utils::silent_payments::generate_label` — they cannot reach `labels::generate_label` directly. Future Phase-6 own-change detection (m = 0) also goes through the same reach-through; no further visibility widening needed."
  - "In-test k=1 derivation (vs pinned hex literals) per RESEARCH § Open Question 2 fallback. The phase-local rule documents both paths; the executor selected the in-test derivation. Rationale: the Python venv against `~/silent-payments/` was not pre-configured in this environment, and the in-test path still catches ser32 endianness regressions through the asymmetry between TV1/TV3/TV4 (all k=0, where any impl gets ser32 right by accident) and the bespoke k=1 case. Trade-off: a bug in `derive_output_tweak` itself (the primitive both the scanner and the test rely on) would silently propagate to both sides. Mitigation: Plan 03-02's `tv1_shared_secret_matches` test pins shared_secret byte-for-byte; combined with the k=0 TV pins in Plan 03-03 and the k=1 round-trip pin here, the cumulative coverage is `shared_secret → t_k → onetime_pk → puzzle_hash → onetime_sk` at both k=0 (multiple TVs) and k=1 (bespoke test)."
  - "Test-local renames `b_scan`, `b_spend`, `b_spend_pub` (CHIP-spec names) to suppress clippy::similar_names on the test-side pk/sk pairs. The function-scoped `#[allow(clippy::similar_names)]` from Plan 03-03 stays in place on `scan_from_tweaks` (already documented in 03-03-SUMMARY as the only #[allow] in silent_payments/). No new #[allow] attributes were introduced by this plan. `expected_pk_k1` / `expected_sk_k1` similarly renamed to `expected_onetime_pk` / `expected_secret_k1` (different stems) to avoid the sk/pk single-byte trip."
  - "Labeled branch consumes `candidate_pk` by value via Rust's `Copy` impl on `chia_bls::PublicKey` (chia-bls 0.36.1). `Add<&PublicKey> for PublicKey` is the impl chosen by clippy::op_ref over `Add<&PublicKey> for &PublicKey`. Successive iterations of `label_map.iter()` re-use candidate_pk via the implicit Copy. This was inline-fixed during development (initial `&candidate_pk + label_pk` flagged by clippy::op_ref); the simpler `candidate_pk + label_pk` form is what survives."
  - "Termination rule placed AFTER the labeled branch (not before): the `if !found { break; }` at the bottom of each k-iteration body reads `found` AFTER both unlabeled and labeled branches have had a chance to set it. This is the CHIP §RECV-04 rule. Plan 03-03's plan-action sample already arranged the unlabeled branch + termination check correctly; Plan 03-04 inserts the labeled branch BETWEEN them in source order. Order in source matches order in operation: unlabeled first (sets found), then labeled (only if !found), then termination check (only if !found after BOTH)."

patterns-established:
  - "Labeled-detection branch with `break` after first match: the labeled `for (m, label_pk) in label_map.iter() { ... if matched { ... break; } }` stops iterating after the first labeled match wins for the current k. Without this break, multiple labels could double-detect (the same coin attributed to multiple m values). The break is also a perf optimization for the realistic case (one label registered, zero matches: O(1); one label registered, one match: O(1)). For large label registries, the worst case is O(N_labels) per k per unmatched output — still O(K_max × N_labels × N_outputs) overall, bounded by K_max = 2400."
  - "Public reach-through pattern via fully-qualified types: `pub fn generate_label(scan_sk: &chia_bls::SecretKey, m: u32) -> (chia_sdk_types::silent_payments::ScalarField, chia_bls::PublicKey) { labels::generate_label(scan_sk, m) }`. Avoids `use` clauses at the top of the parent mod.rs (which would change the public-namespace surface). Phase 6's own-change detection (m = 0) consumes this same reach-through. Future widenings in this module tree should follow the same pattern."

requirements-completed: [RECV-04]

# Metrics
duration: 13min
completed: 2026-05-15
---

# Phase 03 Plan 04: Labeled Detection Branch + TV3 + Bespoke k=1 + Labeled k-Termination Rule Summary

**Wave 4 of Phase 3: closes RECV-04 + CRYPTO-03 success criteria 1/2/6. Promotes `generate_label` to `pub(crate)` in `chia-sdk-utils/src/silent_payments/labels.rs` and adds a `pub fn generate_label` reach-through in `mod.rs` (RESEARCH §13 Option A). Replaces the `PLAN 03-04 APPEND POINT` sentinel and the `let _ = labels;` placeholder in `chia-sdk-driver/src/silent_payments/scanner.rs` with the labeled-detection branch + correct k-termination rule. Adds four named tests: TV3 labeled detection at k=0 (pinned `onetime_sk = 58fc6195...b64852dc`), bespoke k=1 detection (catches `ser32(k)` endianness bugs), labeled k-termination rule (unlabeled-k=0 + labeled-k=1 both detected), and unlabeled-preferred-over-labeled at the same k.**

## Performance

- **Duration:** ~13 min
- **Started:** 2026-05-15T22:33:17Z
- **Completed:** 2026-05-15T22:46:55Z
- **Tasks:** 2 (Task 1: reach-through; Task 2: scanner branch + 4 tests)
- **Files modified:** 3 (labels.rs, mod.rs, scanner.rs)

## Accomplishments

1. **`generate_label` visibility promoted from `pub(super)` to `pub(crate)`** in `crates/chia-sdk-utils/src/silent_payments/labels.rs`. Body unchanged. Function still callable from sibling modules in `chia-sdk-utils/src/silent_payments/` (the Phase 2 tests still pass).

2. **`pub fn generate_label` reach-through** added at the bottom of `crates/chia-sdk-utils/src/silent_payments/mod.rs`:
   ```rust
   #[must_use]
   pub fn generate_label(
       scan_sk: &chia_bls::SecretKey,
       m: u32,
   ) -> (
       chia_sdk_types::silent_payments::ScalarField,
       chia_bls::PublicKey,
   ) {
       labels::generate_label(scan_sk, m)
   }
   ```
   Fully-qualified return types so the imports at the top of `mod.rs` (which Phase 2 left empty) don't need to change. Doc-comment names the consumer (`chia-sdk-driver` scanner, RECV-04) and the byte-level pin (Phase 2's `tv3_label_scalar_matches`).

3. **Labeled-detection branch in `scan_from_tweaks`** at the now-deleted `PLAN 03-04 APPEND POINT`:
   ```rust
   if !found && let Some(label_map) = labels {
       for (m, label_pk) in label_map.iter() {
           let labeled_pk = candidate_pk + label_pk;
           let labeled_hash = puzzle_hash_for_pk(&labeled_pk);
           if output_phs.contains(&labeled_hash)
               && let Some(out) = data.outputs.iter().find(|o| o.puzzle_hash == labeled_hash)
           {
               let base_sk = derive_onetime_sk(spend_sk, &output_tweak);
               let (label_scalar, _) = generate_label(scan_sk, m);
               let base_scalar = ScalarField::from_bytes_raw(base_sk.to_bytes());
               let labeled_scalar = base_scalar.add(&label_scalar);
               let labeled_sk = SecretKey::from_bytes(labeled_scalar.as_bytes())
                   .expect("labeled scalar < r by ScalarField boundary");
               detected.push(DetectedSpCoin { ..., onetime_sk: labeled_sk, k, label: Some(m) });
               found = true;
               break;
           }
       }
   }
   if !found { break; }   // CHIP §RECV-04 termination rule
   ```
   The `let _ = labels;` placeholder line from Plan 03-03 is REMOVED — `labels` is now consumed inside the labeled branch. The `if !found && let Some(label_map) = labels` chained pattern uses stable Rust 1.88+ if-let-chain syntax (the same pattern the unlabeled branch uses, established in Plan 03-03).

4. **Two new imports in `scanner.rs`:**
   ```rust
   use chia_sdk_types::silent_payments::ScalarField;
   use chia_sdk_utils::silent_payments::{LabelRegistry, generate_label};
   ```
   `ScalarField` is needed for `from_bytes_raw` + `add`; `generate_label` is the public reach-through from Task 1.

5. **TV3 pinned bytes** added to the tests module:
   ```rust
   const TV3_INPUT_HASH: [u8; 32] = hex!("58a1875602949aa6bfaf9cb4837957e7175ffb0b14422dbc8d371799f98e66f5");
   const TV3_COIN_ID: [u8; 32] = hex!("4504f59ea184be18924f95244649287382ec6cdc13f333a8990f648c803a6dac");
   const TV3_PUZZLE_HASH: [u8; 32] = hex!("ba271d218d487e8e5dc994a09a8580e1e8a0559a615bd5805cff11b5a343441c");
   const TV3_LABELED_ONETIME_SK: [u8; 32] = hex!("58fc619583ff32e8e6e5cbe8587f4e1a395a04d538b132e5787d634cb64852dc");
   ```

6. **Four new tests passing under `cargo test ... -- --exact`:**
   - **`tv3_scan_detects_labeled_k0`** — TV3 labeled detection: register `m=1`, scan with `tweak_point = TV1_A_SUM * TV3_INPUT_HASH`, expect 1 detection with `k=0`, `label=Some(1)`, `puzzle_hash=TV3_PUZZLE_HASH`, `onetime_sk=TV3_LABELED_ONETIME_SK` byte-for-byte.
   - **`bespoke_k1_detection`** — in-test derivation of expected k=1 puzzle_hash via `derive_output_tweak(.., 1)` → `derive_onetime_pk` → `puzzle_hash_for_pk`; construct TweakData carrying TV1's k=0 PH AND the derived k=1 PH; scan; expect TWO detections (k=0 unlabeled, k=1 unlabeled); assert k=1's `onetime_sk = (b_spend + t_1) mod r`. Catches `ser32(k)` LE regressions.
   - **`labeled_k_termination_rule`** — register `m=1`; build labeled k=1 puzzle_hash via `(candidate_pk_k1 + label_pk_m1)`; TweakData carries TV1's unlabeled k=0 PH AND the labeled k=1 PH; scan; expect TWO detections (k=0 `label=None`, k=1 `label=Some(1)`) — verifies the k loop continues past unlabeled match.
   - **`unlabeled_preferred_over_labeled_at_same_k`** — register `m=1`; TweakData carries ONLY TV1's unlabeled k=0 PH; scan; expect exactly ONE detection with `label=None`. The labeled branch is `if !found`-guarded so it doesn't double-detect.

7. **Workspace test count: 2393 (post-Plan 03-03) → 2397 (+4 new tests).** chia-sdk-driver silent_payments tests: 6 → 10 (1 types + 2 protocol + 7 scanner).

8. **All Phase 1 grep bans still hold** under `silent_payments/`: zero hits for `mod_by_group_order`, `^use sha2::`, `Sha256::digest`.

9. **Only one `#[allow]` under `silent_payments/`** — the function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03 (already documented in 03-03-SUMMARY). No new `#[allow]` introduced by this plan.

10. **Full CI build matrix passes** for both crates: `cargo build --release -p chia-sdk-{utils,driver}` (no features), `-F chip-0057`, `--all-features`. Workspace `--all-features` clean. `cargo clippy --workspace --all-features --all-targets` clean. `cargo machete` clean (zero new ignored entries). `cargo fmt --check` clean.

## Task Commits

1. **Task 1:** `feat(03-04): promote generate_label to pub(crate) + add public reach-through` — `9cf225f1`
   - `crates/chia-sdk-utils/src/silent_payments/labels.rs` (visibility bump)
   - `crates/chia-sdk-utils/src/silent_payments/mod.rs` (pub fn reach-through)
   - All 27 Phase 2 tests still pass (no regression from the widening).
   - Strict clippy + fmt clean on `chia-sdk-utils`.

2. **Task 2:** `feat(03-04): scanner labeled-detection branch + TV3/k=1/k-termination/preferred tests` — `ebe6c34a`
   - `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (+295 lines, -12 lines: branch + 4 tests + imports + TV3 constants)
   - 10 silent_payments tests pass (was 6 — added 4 new in this commit).
   - Strict clippy + fmt clean on `chia-sdk-driver`.

## Files Created/Modified

### Created
None.

### Modified
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` — 1 line changed (pub(super) → pub(crate) on `generate_label`).
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` — 25 lines added (pub fn reach-through + doc-comment + must_use).
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` — 295 lines added, 12 removed: imports updated (added `ScalarField` + `generate_label`), `let _ = labels;` placeholder removed, `PLAN 03-04 APPEND POINT` block replaced with labeled-detection branch + termination rule, TV3 pinned constants + 4 new tests appended to the tests module.

## Decisions Made

1. **Option A reach-through (RESEARCH §13) over Option B (publicly re-export `labels::*`).** Keeps the raw `labels` module module-private; only the curated `pub fn generate_label` reach-through is exposed. Cross-crate consumers cannot reach `labels::generate_label` directly — they must go through the public function. Future Phase-6 own-change detection (m = 0) uses the same reach-through; no further visibility widening needed.

2. **In-test k=1 derivation (RESEARCH § Open Question 2 fallback) over pinned hex literals** for `bespoke_k1_detection`. The Python reference impl venv against `~/silent-payments/` was not pre-configured in this environment. The in-test path still catches `ser32(k)` endianness regressions through the asymmetry between TV1/TV3/TV4 (all k=0) and the bespoke k=1 case. Phase 1's `tv1_shared_secret_matches` + Plan 03-03's k=0 TV pins + this k=1 round-trip pin give cumulative coverage `shared_secret → t_k → onetime_pk → puzzle_hash → onetime_sk` at both k=0 (multiple TVs) and k=1 (bespoke). The trade-off (a bug in `derive_output_tweak` itself would silently propagate to both scanner and test) is mitigated by Plan 03-02's primitive-level tests.

3. **CHIP-spec test-local names (`b_scan`, `b_spend`, `b_spend_pub`)** instead of `scan_sk_v` / `spend_sk_v` / `spend_pk_v`. The latter pair tripped `clippy::similar_names` (sk vs pk, single-byte difference). Renaming to the CHIP names keeps the locals close to the spec narrative AND avoids a new function-scoped allow. The same logic drove the `expected_pk_k1` / `expected_sk_k1` → `expected_onetime_pk` / `expected_secret_k1` renames (different stems) and `labeled_pk_k1` / `labeled_ph_k1` → `labeled_pk_k1` / `labeled_hash_k1` (avoiding the `ph` / `pk` similar-names trip).

4. **No new `#[allow]` attributes** — the function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03 remains the only `#[allow]` anywhere under `silent_payments/`. All Plan 03-04 lint fixes are inline renames or backtick additions.

5. **Labeled branch consumes `candidate_pk` by value** via `chia_bls::PublicKey`'s `Copy` impl + the `Add<&PublicKey> for PublicKey` trait impl. clippy::op_ref flagged the initial `&candidate_pk + label_pk` form. The simpler `candidate_pk + label_pk` form is what survives; successive iterations of `label_map.iter()` re-use `candidate_pk` via the implicit Copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Lint] `clippy::op_ref` on `&candidate_pk + label_pk`**
- **Found during:** Task 2 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** The plan's `<action>` sample uses `&candidate_pk + label_pk` but clippy flagged the leading `&` as needless — `chia_bls::PublicKey` is `Copy`, so `Add<&PublicKey> for PublicKey` is the right impl (consumes the left value by-value via the implicit Copy, which is what we want for re-use across `label_map.iter()` iterations).
- **Fix:** Rewrote as `candidate_pk + label_pk` (no leading `&` on candidate_pk).
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/scanner.rs:126`
- **Verification:** Strict clippy exits 0 post-fix. Build + tests both work.
- **Committed in:** `ebe6c34a`

**2. [Rule 1 - Lint] Five `clippy::similar_names` violations in test-locals**
- **Found during:** Task 2 (`cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings`)
- **Issue:** The plan's `<action>` sample uses local names like `scan_sk_v` / `spend_sk_v` / `spend_pk_v` and `expected_onetime_pk` / `expected_onetime_sk` and `labeled_pk_k1` / `labeled_ph_k1`. All five pairs (sk_v vs sk_v, sk_v vs pk_v, sk_v vs sk_v, pk_k1 vs sk_k1, pk_k1 vs ph_k1) tripped `clippy::similar_names` (single-byte difference under 8+-char locals). The acceptance criteria require `clippy -- -D warnings` exit 0 with NO new `#[allow]` attributes.
- **Fix:** Renamed test-locals to CHIP-spec names: `scan_sk_v` → `b_scan`, `spend_sk_v` → `b_spend`, `spend_pk_v` → `b_spend_pub`. Renamed `expected_pk_k1` → `expected_onetime_pk`, `expected_sk_k1` → `expected_secret_k1`. Renamed `labeled_ph_k1` → `labeled_hash_k1`. All rename targets have different stems (no single-byte difference).
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/scanner.rs:418-590` (all four new tests)
- **Verification:** Strict clippy exits 0 post-fix. No new `#[allow]` attributes added. Test names + behavior unchanged. All 4 tests still pass.
- **Committed in:** `ebe6c34a`

**3. [Rule 1 - Lint] `clippy::doc_markdown` on a test doc-comment**
- **Found during:** Task 2 (strict clippy)
- **Issue:** The `bespoke_k1_detection` doc-comment said "carrying that `puzzle_hash` plus TV1's `k = 0` puzzle_hash (to keep ..." — the second occurrence of `puzzle_hash` was not in backticks. clippy::doc_markdown flagged it.
- **Fix:** Added backticks: "TV1's `k = 0` `puzzle_hash`".
- **Files modified:** `crates/chia-sdk-driver/src/silent_payments/scanner.rs:407`
- **Verification:** Strict clippy exits 0 post-fix.
- **Committed in:** `ebe6c34a`

**4. [Rule 1 - Fmt] Rustfmt restructured the `pub fn generate_label` signature**
- **Found during:** Task 1 (`cargo fmt --all -- --check`)
- **Issue:** The plan's `<action>` Step B sample writes the return type as a single-line tuple `(chia_sdk_types::silent_payments::ScalarField, chia_bls::PublicKey)`. Rustfmt's default style for long fully-qualified tuple return types splits across lines.
- **Fix:** Ran `cargo fmt --all` to let rustfmt apply its default — the result is functionally identical, just visually multi-line:
  ```rust
  pub fn generate_label(
      scan_sk: &chia_bls::SecretKey,
      m: u32,
  ) -> (
      chia_sdk_types::silent_payments::ScalarField,
      chia_bls::PublicKey,
  ) {
      labels::generate_label(scan_sk, m)
  }
  ```
- **Files modified:** `crates/chia-sdk-utils/src/silent_payments/mod.rs:53-61`
- **Verification:** `cargo fmt --check` clean post-fix. Behavior unchanged.
- **Committed in:** `9cf225f1`

---

**Total deviations:** 4 auto-fixed (4 lint/fmt; zero semantic). No new `#[allow]` attributes (Plan 03-03's function-scoped one remains the only one in `silent_payments/`).
**Impact on plan:** All four deviations are pure inline lint/fmt fixes within the plan's flexibility envelope (local renames in test code, backtick addition, rustfmt-mandated multi-line tuple return). No semantic change to the scanner's behavior, no test name changes, no signature changes. The exact API surface specified in the plan is preserved.

## Issues Encountered

None — the four lint/fmt deviations above are intrinsic to the workspace's strict clippy pedantic policy and rustfmt's default behavior. No blockers, no architectural decisions deferred, no work skipped.

## Phase 3 Plan 04 Gate — Final Status

| ID  | Check                                                                                                        | Status |
|-----|--------------------------------------------------------------------------------------------------------------|--------|
| G1  | `cargo build --release -p chia-sdk-utils` (no features) clean                                                 | PASS   |
| G2  | `cargo build --release -p chia-sdk-utils -F chip-0057` clean                                                  | PASS   |
| G3  | `cargo build --release -p chia-sdk-utils --all-features` clean                                                | PASS   |
| G4  | `cargo build --release -p chia-sdk-driver` (no features) clean                                                | PASS   |
| G5  | `cargo build --release -p chia-sdk-driver -F chip-0057` clean                                                 | PASS   |
| G6  | `cargo build --release -p chia-sdk-driver --all-features` clean                                               | PASS   |
| G7  | `cargo build --release --workspace --all-features` clean                                                      | PASS   |
| G8  | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` clean                      | PASS   |
| G9  | `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` clean                     | PASS   |
| G10 | `cargo clippy --workspace --all-features --all-targets` (CI form) clean                                       | PASS   |
| G11 | `cargo fmt --all -- --files-with-diff --check` clean                                                          | PASS   |
| G12 | `cargo machete` clean, no new ignored entries                                                                 | PASS   |
| G13 | `! grep -r 'mod_by_group_order' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban)                    | PASS (zero matches) |
| G14 | `! grep -rE '^use sha2::' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban)                          | PASS (zero matches) |
| G15 | `! grep -rE 'Sha256::digest' crates/chia-sdk-driver/src/silent_payments/` (Phase 1 ban)                       | PASS (zero matches) |
| G16 | `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` (Phase 1 ban)                     | PASS (zero matches) |
| G17 | `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` (Phase 1 ban)                           | PASS (zero matches) |
| G18 | `grep -c '#\[allow' crates/chia-sdk-driver/src/silent_payments/` = 1 (only the Plan-03-03 documented one)     | PASS (1 hit, justified) |
| G19 | `! grep -E 'let _ = labels;' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                           | PASS (placeholder gone) |
| G20 | `! grep -q 'PLAN 03-04 APPEND POINT' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                   | PASS (sentinel gone) |
| G21 | `grep -q 'label_map.iter()' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                            | PASS |
| G22 | `grep -q 'generate_label(scan_sk, m)' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                  | PASS |
| G23 | `grep -q 'label: Some(m)' crates/chia-sdk-driver/src/silent_payments/scanner.rs`                              | PASS |
| G24 | `grep -E '^pub\(crate\) fn generate_label' crates/chia-sdk-utils/src/silent_payments/labels.rs`               | PASS |
| G25 | `grep -E '^pub fn generate_label' crates/chia-sdk-utils/src/silent_payments/mod.rs`                           | PASS |
| G26 | `grep -q 'labels::generate_label(scan_sk, m)' crates/chia-sdk-utils/src/silent_payments/mod.rs`               | PASS |
| G27 | `cargo test ... silent_payments::scanner::tests::tv3_scan_detects_labeled_k0 -- --exact` passes               | PASS (1 passed) |
| G28 | `cargo test ... silent_payments::scanner::tests::bespoke_k1_detection -- --exact` passes                      | PASS (1 passed) |
| G29 | `cargo test ... silent_payments::scanner::tests::labeled_k_termination_rule -- --exact` passes                | PASS (1 passed) |
| G30 | `cargo test ... silent_payments::scanner::tests::unlabeled_preferred_over_labeled_at_same_k -- --exact` passes| PASS (1 passed) |
| G31 | All 7 scanner tests pass (3 from 03-03 + 4 from 03-04)                                                        | PASS (7 passed) |
| G32 | All 27 Phase 2 silent_payments tests in chia-sdk-utils still pass (no regression from pub(super)→pub(crate))  | PASS (27 passed) |
| G33 | Full workspace test suite passes (2393 → 2397)                                                                | PASS (2397 passed, 0 failed) |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Plan 03-05 (DOS-guard test + bindings method + CI matrix + prelude re-exports + final phase gate) unblocked.** `scan_from_tweaks` is now feature-complete on the algorithm side. Plan 03-05 will:
1. Add the DOS-guard test passing `k_max = 32` (the bounded-iteration test input reserved by Plan 03-03 phase-local rule).
2. Add the per-crate `-F chip-0057` CI line for `chia-sdk-driver` to `.github/workflows/rust.yml` (WS-02 equivalent for Phase 3, mirroring Phase 2's Plan 02-05 pattern).
3. Add prelude re-exports in `src/prelude.rs` for `scan_from_tweaks`, `K_MAX_DEFAULT`, `TweakData`, `OutputMeta`, `DetectedSpCoin`, and the protocol primitives.
4. Close RECV-05 (`K_max` DOS guard) and complete the Phase 3 gate.

**Phase 04 (send-side action) inheritance.** Phase 4's `derive_one_time_puzzle_hash` composes the same `derive_output_tweak → derive_onetime_pk → puzzle_hash_for_pk` chain. The `bespoke_k1_detection` test gives Phase 4 a forward cross-check on the `ser32` endianness convention at k=1 (where TV1/TV3/TV4 cannot catch a LE regression because they're all k=0).

**Phase 06 (simulator E2E SIM-03 labeled + own-change detection) inheritance.** The labeled-detection branch in `scan_from_tweaks` works against TV3-shaped inputs; SIM-03 will exercise the same branch against simulator-generated on-chain coins. The `pub fn generate_label` reach-through makes m=0 (the change label) reachable from `chia-sdk-driver` without exposing the raw `labels` module — Phase 6's own-change detection sub-test consumes this directly.

**No blockers for Plan 03-05 or downstream phases.**

## Self-Check: PASSED

Verified all claims:

**Files exist:**
- `crates/chia-sdk-utils/src/silent_payments/labels.rs` (FOUND)
- `crates/chia-sdk-utils/src/silent_payments/mod.rs` (FOUND)
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (FOUND)

**Commits exist:**
- `9cf225f1` (Task 1): FOUND
- `ebe6c34a` (Task 2): FOUND

**Code claims verified via grep:**
- `pub(crate) fn generate_label` in labels.rs: FOUND
- `pub fn generate_label(` in mod.rs: FOUND
- `labels::generate_label(scan_sk, m)` in mod.rs: FOUND (reach-through delegates correctly)
- `label_map.iter()` in scanner.rs: FOUND (labeled branch present)
- `generate_label(scan_sk, m)` in scanner.rs: FOUND (reach-through used)
- `label: Some(m)` in scanner.rs: FOUND (labeled DetectedSpCoin emission)
- `use chia_sdk_types::silent_payments::ScalarField` in scanner.rs: FOUND
- `use chia_sdk_utils::silent_payments::{LabelRegistry, generate_label}` in scanner.rs: FOUND
- `let _ = labels;` in scanner.rs: NOT FOUND (placeholder removed)
- `PLAN 03-04 APPEND POINT` in scanner.rs: NOT FOUND (sentinel consumed)

**Phase 1 grep bans verified:**
- `mod_by_group_order` under silent_payments/: zero hits in both chia-sdk-driver and chia-sdk-utils
- `^use sha2::` under silent_payments/: zero hits in both crates
- `Sha256::digest` under silent_payments/: zero hits in both crates

**Allow count:**
- Only one `#[allow]` under both `silent_payments/` trees combined: the function-scoped `#[allow(clippy::similar_names)]` on `scan_from_tweaks` documented in 03-03-SUMMARY. NO NEW `#[allow]` introduced by this plan.

**Tests passing under `-- --exact`:**
- `tv3_scan_detects_labeled_k0`: PASS
- `bespoke_k1_detection`: PASS
- `labeled_k_termination_rule`: PASS
- `unlabeled_preferred_over_labeled_at_same_k`: PASS
- All 7 scanner tests + 27 Phase 2 chia-sdk-utils silent_payments tests still pass.

**Strict gates:**
- `cargo clippy -p chia-sdk-driver --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` exits 0
- `cargo clippy --workspace --all-features --all-targets` (CI form) exits 0
- `cargo fmt --all -- --check` exits 0
- `cargo machete` clean

**Workspace test count:** 2393 → 2397 (+4) confirmed via `cargo test --release --workspace --all-features --exclude {binding crates}`.

---
*Phase: 03-receive-primitive-chip-test-vector-closure*
*Plan: 04*
*Completed: 2026-05-15*
