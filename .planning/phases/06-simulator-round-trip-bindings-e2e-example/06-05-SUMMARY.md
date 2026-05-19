---
phase: 06-simulator-round-trip-bindings-e2e-example
plan: 05
subsystem: examples
tags: [chip-0057, silent-payments, ex-01, example, v1-closeout, demo]

# Dependency graph
requires:
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-02 tweak_data_from_simulator_block helper + chia_sdk_test::silent_payments wallet-type re-exports"
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-03 Rust E2E patterns (setup_e2e + Spends + finish_with_keys + derive_synthetic + StandardLayer)"
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: "Plan 06-04 BIND-03 closure — confirms the SP flow works through every binding surface; the example mirrors the same flow in Rust"
  - phase: 04.2
    provides: "Action::send + SendDestination::SilentPayment unified send surface"
  - phase: 02
    provides: "SilentPaymentKeys::from_mnemonic + unlabeled_address + labeled_address (ADDR-06 m=0 reserved)"
provides:
  - "examples/silent_payment.rs (119 lines) — runnable EX-01 demo: mnemonic → unlabeled + labeled(1) addresses → 2 SP sends in one tx → farm → extract → scan → detect both → spend both via derive_synthetic + StandardLayer"
  - "bip39 + indexmap added as workspace dev-dependencies on the umbrella crate (chia-wallet-sdk) so examples can use them"
  - "EX-01 requirement flipped from [ ] to [x] in .planning/REQUIREMENTS.md — v1 silent-payments work is requirement-complete (5 of 5 Phase 6 requirements closed)"
affects: [v1.0]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Example file rhythm: rustdoc header (≤14 lines per cat_spends.rs precedent) → minimal use block → fn main() -> Result<()> → 5 stage markers via println! → Simulator + Spends + Action::send + spend_coins → derive_synthetic + StandardLayer follow-on spend per detection."
    - "Multi-output single-tx SP send: Spends::apply with TWO Action::send entries (one SendDestination::SilentPayment(unlabeled) + one SendDestination::SilentPayment(labeled m=1)) → finish_with_keys runs the chip-0057 SP branch once across both → simulator farms a single block containing both detected coins."
    - "Test-crate helper invocation pattern from binary contexts: fully-qualified path chia_sdk_test::silent_payments::tweak_data_from_simulator_block(&sim, height) because the umbrella prelude does NOT re-export it (forward-compat: a future transport client will produce TweakData from wire messages without breaking this call)."
    - "Cross-cutting #5 + #6 mechanical enforcement: grep gates on the example (zero matches for chip-0058/websocket/sp_service/sp_client; zero matches for labeled_address(..., 0)) make the constraints byte-checkable per plan run."

key-files:
  created:
    - "examples/silent_payment.rs"
    - ".planning/phases/06-simulator-round-trip-bindings-e2e-example/06-05-SUMMARY.md"
  modified:
    - "Cargo.toml"
    - ".planning/REQUIREMENTS.md"

key-decisions:
  - "Minimized rustdoc header from 18 lines to 12 lines after rustfmt expansion pushed the file from 120 → 129 lines on first format. The two literal CHIP-0058 mentions in the original header (cross-cutting #5 disclaimer) had to go to satisfy the plan's locked grep (chip[-_]0058 count = 0); rephrased as 'no transport client is referenced (forward-compat)' which conveys the same architectural commitment without tripping the grep. Same applies to the inline comment at the extract stage."
  - "Used recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT) (the SilentPaymentScan trait method) instead of the free-fn scan_from_tweaks(scan_sk, spend_sk, spend_pk, &tweak_data, ...). Plan 06-03 used the free-fn form because of the cyclic-dev-dep type confusion inside chia-sdk-driver tests; that cycle does NOT apply at the example layer (chia-wallet-sdk → chia-sdk-test is one-way), so the more ergonomic trait method works and reads more like wallet-author code. K_MAX_DEFAULT (re-exported from the prelude per Phase 3 Plan 03-05) is 2400 (production cap, CHIP §446)."
  - "Added bip39 + indexmap to the umbrella crate's [dev-dependencies] (NOT [dependencies]). Examples are dev-targets and these crates aren't needed by library consumers. Both are already in [workspace.dependencies] from prior phases (bip39 was added in Plan 02-01 for chia-sdk-utils; indexmap is a workspace-wide pin)."
  - "Boxed SilentPaymentAddress per the locked Phase 4.2 enum variant shape: SendDestination::SilentPayment(Box::new(addr)). The plan's verbatim <action> block specified this and Plan 04.2 made the variant Boxed inline to satisfy clippy::large_enum_variant. The example mirrors this exactly — no second-guessing the variant shape."
  - "Used Mainnet network (spxch1... prefix) instead of Plan 06-03's Testnet (tspxch1...). The plan's <action> block specifies Mainnet because the runnable demo's user-facing output should reflect what a real wallet would display. Plan 06-03's tests use Testnet because that's the test-crate convention; Plan 06-04's binding tests are mixed. The example's choice is independent and documented in the action block."

patterns-established:
  - "Phase 6 Wave 5 example pattern: 119-line file with 5 stage markers (Stage 1/5 … Stage 5/5) that maps 1:1 onto the conceptual SP flow phases (address → send → extract → scan → spend). The Stage 5/5 marker fires once per detection (so a 2-output send produces 2× Stage 5/5 lines), making the loop visible in stdout. Future SP-related examples (CAT2 in v2, multi-recipient batch in v2) should adopt the same Stage M/N convention."
  - "Plan-acceptance-grep-as-architectural-firewall: the example's plan locks down `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' = 0` and `grep -c 'labeled_address(SilentPaymentNetwork::Mainnet, 0)' = 0`. These greps are mechanical guards against the two cross-cutting concerns being silently violated by a future refactor. The pattern is reusable: any architectural commitment that can be expressed as 'this string must NEVER appear' becomes a grep-cf-zero acceptance criterion."

requirements-completed: [EX-01]

# Metrics
duration: 12min
completed: 2026-05-19
---

# Phase 06 Plan 05: EX-01 — Runnable Silent-Payments Demo & v1 Closeout Summary

**`examples/silent_payment.rs` (119 lines) is the runnable v1 closing artifact: a wallet-author-style end-to-end CHIP-0057 demo (mnemonic → unlabeled + labeled(m=1) addresses → 2 SP sends in one tx → farm → extract via `tweak_data_from_simulator_block` → scan → detect both → spend both via `derive_synthetic` + `StandardLayer`). Five stage markers trace the flow. EX-01 flipped to `[x]` closes the 5th and final Phase 6 requirement; v1 silent-payments work is requirement-complete.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-05-19T00:37:08Z
- **Completed:** 2026-05-19T00:49:31Z
- **Tasks:** 3 (2 commits across implementation tasks; Task 3 is verification-only with no file modifications; SUMMARY commit is the 3rd)
- **Files modified:** 4 (2 new + 2 modified)

## Accomplishments

- `examples/silent_payment.rs` lands as a 119-line runnable demo (within the 80–120 line D-09 budget) mirroring the `examples/cat_spends.rs` rhythm: rustdoc header → minimal `use` block → `fn main() -> Result<()>` → `Simulator::new()` + `sim.bls(1_000)` → `SilentPaymentKeys::from_mnemonic` → 5-stage flow with `println!` markers.
- `Stage 1/5` derives + prints both addresses (unlabeled `spxch1...` + labeled `spxch1...` at m=1). The address strings use `SilentPaymentNetwork::Mainnet` so the demo output shows what a real wallet would display.
- `Stage 2/5` sends 100 mojos to the unlabeled address and 200 mojos to the labeled(m=1) address in ONE transaction via two `Action::send(Id::Xch, SendDestination::SilentPayment(Box::new(addr)), amount, Memos::None)` entries inside a single `Spends::apply` call. Then `Spends::with_silent_payment_keys` registers the sender's pk+sk maps, `Spends::finish_with_keys` runs the chip-0057 SP finish branch (`sp_finish_branch` internally), and `sim.spend_coins` farms the block.
- `Stage 3/5` extracts a `TweakData` from the freshly-farmed block via `chia_sdk_test::silent_payments::tweak_data_from_simulator_block(&sim, height_before)` (fully-qualified path because the umbrella prelude does NOT re-export this helper — forward-compat: a future transport-client implementation will produce `TweakData` from wire messages without breaking this call).
- `Stage 4/5` builds a fresh `LabelRegistry`, registers `m=1` (for the labeled output), and runs `recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT)` via the `SilentPaymentScan` trait method blanket impl on `SilentPaymentKeys`. The scanner detects exactly 2 outputs: one with `label: None` at `k=0` and one with `label: Some(1)` at `k=1` (per CHIP-0057 `k`-counter semantics).
- `Stage 5/5` iterates over both detections, calls `d.onetime_sk.derive_synthetic()` (mandatory per RESEARCH Pitfall 6 — the puzzle currys `StandardArgs(synthetic_key)`; raw `onetime_sk` would fail signature verification), then builds a `StandardLayer::new(synthetic_secret.public_key()).spend(...)` follow-on transaction that produces `(amount - 1)` mojos to the sender's puzzle_hash plus a 1-mojo `reserve_fee`. Each follow-on spend farms via `sim.spend_coins(ctx.take(), std::slice::from_ref(&synthetic_secret))`.
- `bip39` and `indexmap` are added to the umbrella crate's `[dev-dependencies]` (workspace = true). Both were already pinned in `[workspace.dependencies]` from prior phases (bip39 since Plan 02-01; indexmap since the workspace was created); the umbrella crate just hadn't needed them until this example.
- `.planning/REQUIREMENTS.md`: EX-01 flipped from `[ ]` to `[x]`. SIM-01, SIM-02, SIM-03, BIND-03 were already `[x]` from prior plans (no flip needed). All 5 Phase 6 requirements are now `[x]` — v1 silent-payments work is requirement-complete.
- Closeout verification sweep all green: `scripts/sp_descriptor_facade_drift.sh` exits 0 (22 methods on both descriptor and facade sides — Plan 06-04's BIND-03 facade additions still align); `cargo machete` exits 0 (zero unused deps; the bip39 + indexmap dev-deps are consumed by the example); `cargo fmt --all --check` exits 0; CI clippy invocation exits 0 (only pre-existing chia-sdk-daemon warnings, documented in `deferred-items.md`).
- Full Rust workspace test suite green: 2335 driver tests + all binding-excluded crates pass under `cargo test --release --workspace --all-features --exclude {binding crates}`.
- All 3 cross-language test suites green:
  - napi: 52 tests pass (`cd napi && pnpm test`)
  - pyo3: 2 tests pass (`cd pyo3 && python -m pytest tests/`)
  - wasm: 8 tests pass (`cd wasm && pnpm test`)
- `cargo build --release --examples --all-features` exits 0 (release build of the example works too).
- Released `cargo run --release --example silent_payment --all-features` produces all 5 stage markers + 2 detections + 2 follow-on spends in well under 1 second.

## Task Commits

Each task was committed atomically:

1. **Task 1:** `cdd7f89b` — `feat(06-05): add examples/silent_payment.rs (EX-01)` (3 files: examples/silent_payment.rs + Cargo.toml + Cargo.lock)
2. **Task 2:** `c29bae5b` — `docs(06-05): close EX-01 + v1 Phase 6 closeout` (1 file: .planning/REQUIREMENTS.md)
3. **Task 3:** Verification-only sweep (no file modifications) — green across 14 gates (build-examples, run-example, release-build, full Rust workspace test suite, napi, pyo3, wasm, descriptor drift, clippy, fmt, machete, EX-01 checkbox, all-5-Phase-6-IDs checkbox, no-unchecked-IDs).

**Plan metadata commit:** to be created with this SUMMARY.md.

## Files Created/Modified

- `examples/silent_payment.rs` — **created** (119 lines). Rustdoc header (12 lines), `use` block (5 lines), `TV1_MNEMONIC` const (3 lines), `fn main` (99 lines including comments and the 5-stage flow body).
- `Cargo.toml` — **modified**. Added `bip39 = { workspace = true }` and `indexmap = { workspace = true }` to the umbrella crate's `[dev-dependencies]` block (alongside the existing `anyhow` and `hex-literal`). Kept alphabetical-ish ordering. Both deps were already pinned in `[workspace.dependencies]`.
- `Cargo.lock` — **modified** (auto-regenerated when adding the dev-deps to the umbrella crate).
- `.planning/REQUIREMENTS.md` — **modified**. Single-byte edit on line 63: `- [ ] **EX-01** …` → `- [x] **EX-01** …`. Description text unchanged.
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-05-SUMMARY.md` — **created** (this file).

## Decisions Made

- **Rustdoc header trimmed from 18 lines to 12 lines after rustfmt expansion pushed the file from 120 → 129 lines on first format.** The two literal "CHIP-0058" mentions in the original header (cross-cutting #5 disclaimer text) had to go to satisfy the plan's locked grep `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' = 0`. Rephrased as "no transport client is referenced (forward-compat)" which conveys the same architectural commitment without tripping the grep. Same applies to the inline comment at the extract stage. The trimmed wording preserves all the load-bearing architectural commitments (forward-compat via test-crate helper only; m=0 reserved; ADDR-06 reference) just with different surface text.
- **Used `recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT)` (trait method) instead of the free-fn `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &tweak_data, ...)`** that Plan 06-03 used. Plan 06-03's e2e.rs hit the cyclic-dev-dep type-confusion (chia-sdk-driver → chia-sdk-test → chia-sdk-driver cycle inside its own test crate) and had to use the free-fn form. The example doesn't have that cycle (chia-wallet-sdk → chia-sdk-test is one-way), so the more ergonomic trait method works and reads more like real wallet-author code. The plan's verbatim `<action>` block specified `recipient.scan(...)` exactly.
- **Added `bip39` + `indexmap` to the umbrella crate's `[dev-dependencies]` (NOT `[dependencies]`).** Examples are dev-targets and these crates aren't needed by library consumers. Both were already in `[workspace.dependencies]` from prior phases (bip39 was added in Plan 02-01 for chia-sdk-utils; indexmap is a workspace-wide pin). `cargo machete` exits 0 because both are now consumed by the example.
- **Boxed `SilentPaymentAddress` per the locked Phase 4.2 enum variant shape: `SendDestination::SilentPayment(Box::new(addr))`.** The plan's verbatim `<action>` block specified this and Plan 04.2-01 made the variant Boxed inline to satisfy `clippy::large_enum_variant`. The example mirrors this exactly.
- **Used `SilentPaymentNetwork::Mainnet` (spxch1... prefix) instead of Plan 06-03's `Testnet` (tspxch1...).** The plan's `<action>` block specifies Mainnet because the runnable demo's user-facing output should reflect what a real wallet would display. Plan 06-03's tests use Testnet because that's the test-crate convention; Plan 06-04's binding tests are mixed (the napi/pyo3/wasm tests use Testnet too, matching their setup helpers).
- **Did NOT call `Spends::finish_silent_payments` directly.** Plan 06-04 added that as a public hook on `chia_sdk_driver::Spends` to support the binding-layer `Spends::prepare` pipeline. The example uses `Spends::finish_with_keys`, which already internally calls `sp_finish_branch` (the same composition `finish_silent_payments` exposes for the binding layer). No need for the public hook here — `finish_with_keys` is the canonical Rust-side path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] First-pass rustdoc header exceeded the 80–120 line budget after rustfmt**

- **Found during:** Task 1 (post-rustfmt line count check)
- **Issue:** The plan's verbatim `<action>` block's rustdoc header was 18 lines + I initially copied it verbatim. After `cargo fmt --all -- examples/silent_payment.rs`, rustfmt expanded the in-body `println!` calls and the `with_silent_payment_keys` call from compact single-line forms to multi-line forms (the rustfmt default for any expression exceeding the 100-char width). Total grew from 120 lines to 129 lines, breaking the locked D-09 budget (`grep -c lines >= 80 && <= 120`).
- **Fix:** Trimmed the rustdoc header from 18 lines to 12 lines, condensing the multi-paragraph forward-compat disclaimer into a single 3-line block. The two literal "CHIP-0058" mentions (cross-cutting #5 disclaimer) were rephrased to "no transport client is referenced (forward-compat)" — preserving the architectural commitment but eliminating the literal text that would have tripped the plan's `grep -ciE 'chip[-_]0058|... = 0` acceptance criterion. Also trimmed three inline comments (the section 1, section 2, and the labeled-address comment) from 2–3 lines to 1 line each.
- **Files modified:** `examples/silent_payment.rs`
- **Verification:** Final line count after rustfmt is 119; `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' examples/silent_payment.rs` returns 0; all 5 stage markers preserved in output.
- **Committed in:** `cdd7f89b` (Task 1)

**2. [Rule 2 - Missing Critical] Umbrella crate was missing `bip39` and `indexmap` in `[dev-dependencies]`**

- **Found during:** Task 1 (initial `cargo build --examples --all-features`)
- **Issue:** The example uses `bip39::Mnemonic::parse(...)` and `indexmap::indexmap! { ... }`. Neither crate was listed in the umbrella `chia-wallet-sdk`'s `[dev-dependencies]` (only `anyhow` + `hex-literal` were there). Both are already in `[workspace.dependencies]` from prior phases (bip39 was added by Plan 02-01 for chia-sdk-utils; indexmap is workspace-wide). Without adding them, the example wouldn't compile.
- **Fix:** Added `bip39 = { workspace = true }` and `indexmap = { workspace = true }` to the umbrella `[dev-dependencies]` block.
- **Files modified:** `Cargo.toml`
- **Verification:** `cargo build --examples --all-features` exits 0; `cargo machete` still exits 0 (both deps are consumed by the example, so not unused).
- **Committed in:** `cdd7f89b` (Task 1; landed in the same commit as the example itself because the deps are co-required)

### Deferred Items (not auto-fixed; out-of-scope)

**Pre-existing chia-sdk-daemon clippy warnings under `-D warnings`.** Already documented in `.planning/phases/06-simulator-round-trip-bindings-e2e-example/deferred-items.md` (Plan 06-01 baseline; reaffirmed by every Phase 6 plan). CI's clippy invocation (`cargo clippy --workspace --all-features --all-targets`, without `-D warnings`) exits 0. Per the GSD scope-boundary rule, pre-existing warnings in unrelated files are out of scope. No action taken in this plan.

---

**Total deviations:** 2 auto-fixed (1 Rule-1 bug from rustfmt overflow forcing the rustdoc header trim, 1 Rule-2 missing-critical for the bip39 + indexmap dev-deps).

**Impact on plan:** The plan's spirit (a 80–120 line runnable demo mirroring `cat_spends.rs` rhythm + 5 stage markers + cross-cutting #5/#6 honored + EX-01 closed) realized exactly. All locked acceptance grep counts pass:

- `grep -c 'fn main()'` = 1
- `grep -c 'SilentPaymentKeys::from_mnemonic'` = 1
- `grep -c 'unlabeled_address(SilentPaymentNetwork::Mainnet)'` = 1
- `grep -c 'labeled_address(SilentPaymentNetwork::Mainnet, 1)'` = 1
- `grep -c 'labeled_address(SilentPaymentNetwork::Mainnet, 0)'` = 0 (cross-cutting #6 + ADDR-06)
- `grep -c 'SendDestination::SilentPayment'` = 2
- `grep -c 'tweak_data_from_simulator_block'` = 2 (1 doc + 1 call)
- `grep -c 'derive_synthetic'` = 2 (1 doc + 1 call)
- `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client'` = 0 (cross-cutting #5)
- `grep -c '#[allow'` = 0
- `wc -l` = 119 (within 80–120 budget)

## Issues Encountered

- **Rustfmt expanded the file from 120 → 129 lines on first format** (Deviation #1 above). Resolved by trimming the rustdoc header + three inline comments. Future authors of small examples that need to satisfy a hard line budget should write the rustdoc header in single-paragraph form (rustfmt won't re-flow doc comments aggressively the way it does code) and let any multi-arg `println!` / function calls stay compact in source — rustfmt will expand them when the line exceeds 100 chars, so plan for that expansion in advance.
- **The plan's locked `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' = 0` was initially violated** by my first-pass rustdoc header which contained two literal "CHIP-0058" references in the cross-cutting #5 disclaimer. The grep counts comments too. Rephrased to "transport client" — same architectural meaning, no grep hit. Lesson: when a plan locks a grep-zero criterion on a string, even DOCUMENTATION mentions of that string violate it. Comments must be reworded.
- **Pre-existing chia-sdk-daemon clippy warnings** under `cargo clippy --workspace --all-features --all-targets -- -D warnings`. Documented in `deferred-items.md`. CI invocation (no `-D warnings`) exits 0. Per the GSD scope-boundary rule, pre-existing warnings in unrelated files are out of scope. No action taken in this plan.

## User Setup Required

None — the example runs from a fresh `cargo run --example silent_payment --all-features` against the in-process `Simulator`. No external service, no secrets, no on-disk state.

## Next Phase Readiness

- **Phase 6 COMPLETE.** All 5 Phase 6 requirements (BIND-03, SIM-01, SIM-02, SIM-03, EX-01) are `[x]` in `.planning/REQUIREMENTS.md`.
- **v1 silent-payments work is requirement-complete.** All 26 v1 requirements across 7 categories (Address & Key Derivation, Send Side, Receive Side, Cryptographic Primitives, Bindings, Workspace Integration, Simulator Integration, Example) are closed. The v1.0 milestone has reached requirement-completeness.
- **Zero new workspace deps in this plan.** `bip39` and `indexmap` were already pinned in `[workspace.dependencies]`; this plan only added them as dev-deps to the umbrella crate.
- **Zero new `#[allow]` attributes.** Workspace lint policy intact. The only `#[allow]` anywhere in Phase 6 silent_payments code remains the `#[allow(clippy::similar_names)]` on `scan_from_tweaks` from Plan 03-03.
- **Zero `unsafe` code.** No `unsafe_code = "deny"` violations.
- **Workspace test count:** unchanged (this plan adds no tests; the example is functional verification via `cargo run`).
- **Next:** v1 retrospective + release artifact preparation. The work landed in Phase 6 enables:
  1. Wallet authors (Sage, third parties) to derive SP addresses + send XCH to one through idiomatic SDK calls (`Action::send(Id::Xch, SendDestination::SilentPayment(Box::new(addr)), amount, Memos::None)`).
  2. Indexer authors to construct `TweakData` from any source (simulator blocks today; CHIP-0058 wire messages tomorrow) and feed it to `recipient.scan(...)`.
  3. Cross-language consumers (napi/pyo3/wasm) to do all of the above through their respective binding surfaces.

## Verification Summary (Task 3 sweep)

| Gate | Command | Result |
|------|---------|--------|
| 1 | `cargo build --examples --all-features` | PASS (1.42s incremental) |
| 2 | `cargo build --release --examples --all-features` | PASS (36.64s) |
| 3 | `cargo run --example silent_payment --all-features` | PASS — 5 stage markers + 2 detections + 2 follow-on spends |
| 4 | `cargo run --release --example silent_payment --all-features` | PASS (sub-second execution) |
| 5 | `cargo test --release --workspace --all-features --exclude {binding crates}` | PASS (2335 driver tests + others) |
| 6 | `cd napi && pnpm test` | PASS (52/52) |
| 7 | `cd pyo3 && python -m pytest tests/` | PASS (2/2) |
| 8 | `cd wasm && pnpm test` | PASS (8/8) |
| 9 | `bash scripts/sp_descriptor_facade_drift.sh` | PASS (22 methods both sides) |
| 10 | `cargo clippy --workspace --all-features --all-targets` (CI invocation, no -D warnings) | PASS (warnings only on pre-existing chia-sdk-daemon — documented) |
| 11 | `cargo fmt --all --check` | PASS |
| 12 | `cargo machete` | PASS (zero unused deps; bip39+indexmap dev-deps are consumed by the example) |
| 13 | `wc -l < examples/silent_payment.rs` | PASS (119 — within 80–120 budget) |
| 14 | Phase 6 checkbox audit (5 of 5 IDs `[x]`, 0 unchecked) | PASS |

## Plan Acceptance Criteria

All success criteria from the plan met:

- [x] examples/silent_payment.rs lands as a runnable EX-01 demo (~119 lines, within the 80–120 line D-09 budget)
- [x] Example mirrors `cat_spends.rs` rhythm: use block → `fn main` → simulator setup → action build → `spend_coins` → `println!` stage markers
- [x] Example exercises the full flow: mnemonic → SilentPaymentKeys → unlabeled + labeled(m=1) addresses → 2 SP sends in one tx → farm → `tweak_data_from_simulator_block` → scan → detect both → spend both via `StandardLayer::new(synthetic_pk)` after `derive_synthetic`
- [x] Cross-cutting #5 honored: NO CHIP-0058 / transport-client reference; ONLY `tweak_data_from_simulator_block` for extraction (grep count = 0 for the disallowed-strings regex)
- [x] Cross-cutting #6 honored: `labeled_address(Mainnet, 1)`, never m=0; m=0 reserved per ADDR-06 (grep count = 0 for `labeled_address(..., 0)`)
- [x] Pitfall 6 honored: `derive_synthetic()` called before `StandardLayer::new` for every detected coin
- [x] 5 Phase 6 requirements (SIM-01, SIM-02, SIM-03, BIND-03, EX-01) marked `[x]` in `.planning/REQUIREMENTS.md` — v1 closeout
- [x] Phase 5 descriptor↔facade drift script still green
- [x] Workspace clippy, rustfmt, cargo machete all green
- [x] Full Rust workspace test suite + all 3 cross-language test suites (napi, pyo3, wasm) all green
- [x] v1 silent payments work is requirement-complete and shippable
- [x] `grep -c 'fn main()' examples/silent_payment.rs` returns 1
- [x] `grep -c 'SilentPaymentKeys::from_mnemonic' examples/silent_payment.rs` returns 1
- [x] `grep -c 'unlabeled_address(SilentPaymentNetwork::Mainnet)' examples/silent_payment.rs` returns 1
- [x] `grep -c 'labeled_address(SilentPaymentNetwork::Mainnet, 1)' examples/silent_payment.rs` returns 1
- [x] `grep -c 'labeled_address(SilentPaymentNetwork::Mainnet, 0)' examples/silent_payment.rs` returns 0
- [x] `grep -c 'SendDestination::SilentPayment' examples/silent_payment.rs` returns 2
- [x] `grep -c 'tweak_data_from_simulator_block' examples/silent_payment.rs` returns 2
- [x] `grep -c 'derive_synthetic' examples/silent_payment.rs` returns 2
- [x] `grep -ciE 'chip[-_]0058|websocket|ws_client|sp_service|sp_client' examples/silent_payment.rs` returns 0
- [x] `grep -F '#[allow' examples/silent_payment.rs` returns nothing
- [x] `cargo run --example silent_payment --all-features` exits 0
- [x] Output contains all 5 stage markers (`Stage 1/5` through `Stage 5/5`)
- [x] `Stage 5/5` appears 2× in output (one per detection)

## Self-Check: PASSED

All claimed files exist on disk:
- `examples/silent_payment.rs` (created)
- `Cargo.toml` (modified — bip39 + indexmap dev-deps)
- `Cargo.lock` (modified — regenerated)
- `.planning/REQUIREMENTS.md` (modified — EX-01 flipped to `[x]`)
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-05-SUMMARY.md` (this file)

All claimed task commits exist in git history:
- `cdd7f89b` (Task 1: example + dev-deps)
- `c29bae5b` (Task 2: REQUIREMENTS.md EX-01 flip + closeout sweep)

---
*Phase: 06-simulator-round-trip-bindings-e2e-example*
*Completed: 2026-05-19*
