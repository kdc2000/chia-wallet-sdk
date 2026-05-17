---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 04.1-01-PLAN.md (FINGERPRINT-01 deletion half + SEND-06 re-validation; Plan 04.1-02 = runtime gate)
last_updated: "2026-05-17T18:42:58.451Z"
last_activity: 2026-05-17
progress:
  total_phases: 7
  completed_phases: 4
  total_plans: 22
  completed_plans: 25
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-15)

**Core value:** A wallet developer can derive a silent-payment address, send XCH to one, and (with a CHIP-0058 tweak-data source) detect incoming silent payments — through the same idiomatic SDK surface the SDK already uses for everything else.
**Current focus:** Phase 04.1 — sage-style-binding-refactor

## Current Position

Phase: 04.1 (sage-style-binding-refactor) — EXECUTING
Plan: 2 of 2
Status: Ready to execute
Last activity: 2026-05-17

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: — min
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Crypto primitives & workspace integration | 0/TBD | — | — |
| 2. Address & key types | 0/TBD | — | — |
| 3. Receive primitive & CHIP test-vector closure | 0/TBD | — | — |
| 4. Send-side action | 0/TBD | — | — |
| 5. Bindings (Rust facade + JSON descriptor) | 0/TBD | — | — |
| 6. Simulator round-trip + bindings E2E + example | 0/TBD | — | — |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01-crypto-primitives-workspace-integration P01 | 10 | 1 tasks | 4 files |
| Phase 01-crypto-primitives-workspace-integration P02 | 7 | 2 tasks | 4 files |
| Phase 01-crypto-primitives-workspace-integration P03 | 4 | 2 tasks | 2 files |
| Phase 01-crypto-primitives-workspace-integration P04 | 3 | 2 tasks | 2 files |
| Phase 01-crypto-primitives-workspace-integration P05 | 13 | 2 tasks | 4 files |
| Phase 02 P01 | 18 | 3 tasks | 3 files |
| Phase 02 P02 | 8 | 3 tasks | 3 files |
| Phase 02 P03 | 8 | 2 tasks | 1 files |
| Phase 02 P04 | 12 | 3 tasks | 4 files |
| Phase 02 P05 | 13 | 2 tasks | 2 files |
| Phase Phase 03 P01 P01 | 14 | 2 tasks | 5 files |
| Phase 03 P02 | 12 | 1 tasks | 2 files |
| Phase 03 P03 | 19 | 1 tasks | 2 files |
| Phase 03 P04 | 13 | 2 tasks | 3 files |
| Phase 03 P05 | 26 | 3 tasks | 4 files |
| Phase 04 P01 | 15 | 4 tasks | 4 files |
| Phase 04 P02 | 13 | 4 tasks | 6 files |
| Phase 04 P03 | 21 | 3 tasks | 3 files |
| Phase 04 P04 | 21min | 2 tasks | 3 files |
| Phase 04 P05 | 23min | 3 tasks | 3 files |
| Phase 04.1 P01 | 8min | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table. Recent decisions affecting current work:

- All Key Decisions in PROJECT.md remain "Pending" until phase transitions validate them.
- Roadmap merged research's 7-phase order into 6 phases: workspace integration (WS-01..03) ships in Phase 1 with crypto primitives (CHIP-0037 precedent commit `bbc7f57f`); example (EX-01) ships in Phase 6 alongside the simulator round-trip rather than as its own phase.
- SEND-02 (`compute_input_hash`) and SEND-03 (`aggregate_sender_sks` with multi-party hard-error) are scoped to Phase 4 (send-side action), not Phase 3, because the hard-error path and announcement binding are correctness gates that only have observable behavior in the context of the `SilentPaymentSend` action.
- [Phase 01-crypto-primitives-workspace-integration]: Phase 1 Plan 01: chip-0057 feature scaffolding mirrors chip-0037 precedent (bbc7f57f) byte-for-byte; chia-sdk-utils gains its first [features] block with empty chip-0057 = []; Q8 dep:chia-sdk-types edge deferred to Phase 2 pre-flight audit.
- [Phase 01-crypto-primitives-workspace-integration]: Plan 01-02: ScalarField newtype lands behind chip-0057 with unsigned mod-r reduction; no From<[u8;32]> impl (type boundary forces unsigned-vs-signed choice). Adversarial test from PLAN '..._reduces_to_r_minus_one' was a misnomer — actual reduction of (2^256-1) mod r is 0x1824b159...fffffffd (not r-1). Test name preserved per VALIDATION lock; assertion pins correct value.
- [Phase 01-crypto-primitives-workspace-integration]: Plan 01-03: tagged_hash + Chia_SP/* tag constants land under chip-0057. BIP-340 construction via chia_sha2::Sha256 (new/update/finalize — never ::digest). Three pub const &'static str tags pinned via SHA256(tag.as_bytes())==hex!() typo guards; values cross-verified via Python hashlib.sha256 AND sha256sum CLI before pinning.
- [Phase 01-crypto-primitives-workspace-integration]: Plan 01-04: SCAN_PATH and SPEND_PATH pinned as pub const &[u32] under chip-0057 — &[12381, 8444, 12, 0] and &[12381, 8444, 13, 0], unhardened (no | 0x80000000) per CHIP-0057 §172-173 and sp-common reference impl. No unit tests at this layer; values validated end-to-end at Phase 2 against b_scan/b_spend CHIP test vectors. silent_payments/mod.rs barrel now in final sorted order paths < scalar < tagged_hash.
- [Phase 01-crypto-primitives-workspace-integration]: Plan 01-05: chia-sdk-daemon clippy::pedantic lints (client.rs:426-427) are pre-existing upstream warnings — out of Phase 1 scope, logged to deferred-items.md, NOT auto-fixed. Scoped clippy on chia-sdk-types is clean with -D warnings; CI's existing clippy step (no -D warnings) exits 0.
- [Phase 01-crypto-primitives-workspace-integration]: Plan 01-05: Phase 1 COMPLETE — CI matrix has chip-0057 per-crate line (WS-02); machete clean with zero new ignored entries; three grep bans (mod_by_group_order, ^use sha2::, Sha256::digest) hold across silent_payments/; all 10 named tests pass; full 2360-test workspace suite green (WS-03). M1-M9 status table: all PASS. Phase 2 readiness: chip-0057 feature + ScalarField/tagged_hash/SCAN_PATH/SPEND_PATH re-exports available; Q8 dep edge audit still open.
- [Phase 02]: Plan 02-01: chia-sdk-utils chip-0057 feature promoted to activate dep:chia-sdk-types, dep:bip39, dep:chia-bls (all optional = true). Q8 (Phase 2 pre-flight audit) resolved: dep edge gated; no-features build unaffected. silent_payments module barrel created as empty cfg-gated public module; Task 3 (prelude.rs re-export) NO-OP'd and deferred to Plan 02-05 to avoid chicken-and-egg compile error.
- [Phase 02]: Plan 02-02: SilentPaymentError (6-variant enum, #[from] Bech32Error) + SilentPaymentNetwork (Mainnet/Testnet, hrp() returns spxch/tspxch) land in chia-sdk-utils/silent_payments. address.rs ships in STUB form holding only SilentPaymentNetwork — Plan 02-03 APPENDS SilentPaymentAddress to the same file. No Clone/PartialEq on SilentPaymentError (RESEARCH §7); from_hrp uses bound 'other' variable to pass through the unmatched HRP literally. Wave-2 parallelism (Plans 02-03 and 02-04) now unblocked.
- [Phase 02]: Plan 02-03: SilentPaymentAddress (struct + new + encode + decode) and 12 named tests append to crates/chia-sdk-utils/src/silent_payments/address.rs. Bech32m via chia_sdk_utils::Bech32 wrapper (no direct bech32:: imports). Identity-pubkey rejection via parse-then-is_inf ordering. ADDR-02 closed end-to-end. Permissive matches!(Err(IdentityPublicKey | InvalidPublicKey)) on the two identity tests since both satisfy CHIP §215; chia-bls 0.36.1 empirically fires IdentityPublicKey. Inline clippy fixes for doc_markdown + unnested_or_patterns (no #[allow]).
- [Phase 02]: Plan 02-04: SilentPaymentKeys (BIP-39 -> scan/spend SKs at m/12381/8444/{12,13}/0; manual redacting Debug; from_secret_keys parity) + LabelRegistry (bidirectional u32 <-> PublicKey via two HashMaps with [u8;48] reverse key) + pub(super) generate_label helper shared between keys.rs::labeled_address and labels.rs::LabelRegistry::register. Closes ADDR-01, ADDR-03, ADDR-04, ADDR-05, ADDR-06. Rule 3 fix: chia-sdk-utils chip-0057 feature now cascades to chia-sdk-types/chip-0057 (was pulling dep but not activating its feature). Inline clippy fixes: clone_on_copy (PublicKey is Copy in chia-bls 0.36.1; drop .clone() in 5 sites + deref via *), similar_names (inline public_key() into struct init to avoid scan_pk/scan_sk local-binding collision). No #[allow] attributes.
- [Phase 02]: Plan 02-05: chia-sdk-utils -F chip-0057 CI build line added to .github/workflows/rust.yml (WS-02 equivalent for Phase 2, 10-space indent, after no-features chia-sdk-utils line). src/prelude.rs gains #[cfg(feature = "chip-0057")] re-export block of the five Phase 2 public types (LabelRegistry, SilentPaymentAddress, SilentPaymentError, SilentPaymentKeys, SilentPaymentNetwork). Full 13-expression phase-gate matrix green: 5 builds, strict + workspace clippy, fmt, machete (zero new ignored entries), both Phase 1 grep bans hold, 27 silent_payments tests, 2387-test full workspace suite. Phase 2 COMPLETE; ADDR-01..06 all mechanically closed.
- [Phase Phase 03]: Plan 03-01: chia-sdk-driver chip-0057 feature cascade extended to activate dep:chia-sdk-utils + chia-sdk-utils/chip-0057 (Plan 02-04 cascade-precedent pattern). silent_payments/{mod,types}.rs scaffold lands under #[cfg(feature = "chip-0057")] mod silent_payments; in lib.rs — first module-level cfg-gate in chia-sdk-driver. Three wire types (TweakData, OutputMeta, DetectedSpCoin) all pub-fielded; OutputMeta gains Copy (Rule 1 inline fix — all fields are Copy, workspace missing_copy_implementations was tripping clippy -D warnings). DriverError::SilentPayment(#[from] SilentPaymentError) variant added once for Phase 4 reuse. Tasks 1+2 shipped as ONE atomic commit (3436f7cb) per plan's done directive. RECV-01 closed; G1..G12 phase-1-gate matrix green; workspace test count 2387 → 2388 (+1 new defensive deserialization test).
- [Phase 03]: Plan 03-02: Five CHIP-0057 protocol primitives land in chia-sdk-driver/src/silent_payments/protocol.rs behind chip-0057. compute_shared_secret_from_tweak (ECDH via chia_sha2), derive_output_tweak (ScalarField::from_bytes_unsigned + tagged_hash with k.to_be_bytes), derive_onetime_pk, derive_onetime_sk (ScalarField::from_bytes_raw on spend_sk preserving bit pattern), puzzle_hash_for_pk (StandardArgs::curry_tree_hash(pk.derive_synthetic())). tv1_shared_secret_matches pins TV1's d3ac1e8f...0ba2c6 byte-for-byte (RECV-03). adversarial_ff32_scalar_reduces_unsigned closes CRYPTO-03 success criterion 3 — three-assertion test verifies the ScalarField boundary fires end-to-end (first byte < 0x80, determinism, direct-path equality). Rule 1 inline lint fixes: similar_names on tweak_sk/tweak_pk (rebinding to tweak_secret + inline public_key), three doc_markdown backtick additions, doc-comment rephrase to honor Phase 1 grep ban on 'mod_by_group_order' string literal. One atomic commit 079d9e21. Workspace tests 2388→2390 (+2).
- [Phase 03]: Plan 03-03: silent_payments scanner.rs lands pub fn scan_from_tweaks (UNLABELED branch only) with both CHIP-spec guards the sp-client reference lacks: PublicKey::is_inf() identity-element skip (CHIP §459) at outer tweak_point loop, bounded  per CHIP §416 with K_MAX_DEFAULT = 2400 (CHIP §446 production cap, NOT 32 which is reserved for the DOS-guard test input in Plan 03-05). PLAN 03-04 APPEND POINT sentinel comment +  placeholder line establish the explicit seam for Plan 03-04's labeled-branch append. Three named tests close RECV-02 (unlabeled) + RECV-03 byte-for-byte against pinned CHIP test-vector outputs: tv1_scan_detects_unlabeled_k0 (TV1 onetime_sk = 3c399c61...0a89db37), tv4_scan_detects_multi_input_aggregation (TV4 onetime_sk = 6ccc3e13...e0f309399), identity_tweak_point_skipped (CHIP §459 guard verified). Workspace test count 2390 → 2393. Rule 1 fixes inline: candidate_ph → candidate_hash (clippy::similar_names vs candidate_pk), nested if-let collapsed via Rust 1.88 if-let-chain (clippy::collapsible_if), three doc_markdown backtick additions. UNAVOIDABLE deviation logged: function-scoped #[allow(clippy::similar_names)] on scan_from_tweaks for the spend_sk/spend_pk parameter pair — must_have #1 locks the exact signature AND Plan 03-04 references both names directly; local rebinding cannot suppress a lint that fires on parameter declaration lines. This is the only #[allow] anywhere under silent_payments/.
- [Phase 03]: Plan 03-04: Option A reach-through (RESEARCH §13): generate_label promoted pub(super)→pub(crate) in labels.rs + pub fn generate_label wrapper in mod.rs (one-line delegate, fully-qualified return types). Cross-crate consumers cannot reach labels::generate_label directly. Future Phase-6 own-change detection (m=0) uses the same reach-through.
- [Phase 03]: Plan 03-04: labeled-detection branch in scan_from_tweaks: if !found && let Some(label_map) = labels { for (m, label_pk) in label_map.iter() { ... break on first labeled match } }. Termination rule placed AFTER labeled branch in source. Closes RECV-04 + CRYPTO-03 success criteria 1 (TV3), 2 (bespoke k=1 via in-test derivation), 6 (labeled k-termination). Workspace tests 2393 → 2397 (+4). No new #[allow] attributes; CHIP-spec test-local names (b_scan, b_spend, b_spend_pub) used to suppress similar_names.
- [Phase 03]: Plan 03-05: SilentPaymentScan trait + impl on SilentPaymentKeys (orphan-rule-compliant cross-crate method add per RESEARCH Open Q3). Both free fn scan_from_tweaks (hardware-split signers) and method keys.scan(...) (ergonomic bundled) coexist; silent_payment_keys_scan_method_matches_free_fn_tv1 pins byte-equality. dos_guard_caps_at_k_max forges 10,000 matches at one tweak point + k_max=32 → asserts detections.len() <= 32 (CHIP §416 cap proof, RECV-05 closure). New CI line cargo build -p chia-sdk-driver -F chip-0057 in rust.yml at 10-space indent (WS-02 equivalent for Phase 3). src/prelude.rs gains SECOND chip-0057 block re-exporting 11 driver-side symbols (5 types + 6 functions). Rule 3 inline fix: mod silent_payments → pub mod silent_payments in chia-sdk-driver/lib.rs (caught at workspace --all-features compile). 4 deviations all inline-fixed: 2x doc_markdown, 2x items_after_statements, 1x rustfmt re-wrap, 1x module visibility. Zero new #[allow]. Workspace tests 2397 → 2399 (+2 new). Phase 3 COMPLETE: 16-gate matrix green, all 6 ROADMAP success criteria PASS, all 6 requirements (RECV-01..05 + CRYPTO-03) closed.
- [Phase 04]: Plan 04-01: 3 send-side free functions (aggregate_sender_sks/compute_input_hash/derive_one_time_puzzle_hash) land in crates/chia-sdk-driver/src/silent_payments/ as composed Phase-3 primitives. TV4 aggregate byte-pinned to 5600d878...cbf95b89 (sp-common reference); TV1 input_hash 38a1c8...cc9411; TV1 puzzle_hash 23adba14...c21fbf5 at k=0; in-test k=1 round-trip catches ser32(k) endianness regressions. compute_input_hash panics on empty slice (action layer guarantees non-empty XCH-input set). b_*-style test naming follows scanner.rs::bespoke_k1_detection precedent to avoid clippy::similar_names without #[allow]. Zero new workspace deps; zero new #[allow] attributes anywhere in silent_payments/.
- [Phase 04]: Plan 04-02: SilentPaymentSend action + apply-time plumbing land in chia-sdk-driver. New pub struct SilentPaymentSend (recipient, amount, memos) + SpendAction impl reserves XCH parent via output_source(BURN_PUZZLE_HASH, amount), increments per-recipient k-counter (keyed by 48-byte compressed scan_pk), pushes SilentPaymentPending; NO CreateCoin emission at apply (deferred per RESEARCH §1e). Action::SilentPaymentSend(SilentPaymentSend) cfg-gated enum variant + Action::silent_payment_send constructor + dispatch arms. Two new pub(crate) chip-0057-gated fields on Spends: silent_payment_counters: HashMap<[u8;48],u32> + silent_payments_pending: Vec<SilentPaymentPending>. Spends::finish_with_silent_payment_keys STUB lands with locked signature ((self, ctx, deltas, relation, &IndexMap<Bytes32,PublicKey>, &IndexMap<Bytes32,SecretKey>) -> Result<Outputs,DriverError>) returning Err(DriverError::Custom) after a dead-code-suppressing destructure-read (no #[allow]). 4 deviations all Rule-1 inline-fixed: destructure-read for dead_code, unused super::*, borrow-after-move via pre-captured expected_scan_pk/expected_spend_pk, pub use → pub(crate) use re-export. 1 new integration test (action_state_machine) → driver test count 1069→1070; silent_payments tests unchanged at 18. 7 Privacy-warning rustdoc mentions across Plan 04-02 surfaces. SEND-04 (apply-time portion) closed; finish-time round-trip blocked on Plan 04-03.
- [Phase 04]: Plan 04-03: Spends::finish_with_silent_payment_keys real 9-step body replaces Plan 04-02's stub. 3 chip-0057-gated DriverError variants land (SilentPaymentMultiPartyUnsupported / SilentPaymentNoXchInputs / SilentPaymentMemoHintForbidden — canonical wording from RESEARCH §4). Inline-fixed clippy: parameter rename synthetic_sks -> secret_keys (clippy::similar_names; locked TYPE preserved); local rename aggregated_sender_pk -> agg_pk; mem::take Pitfall B borrow workaround commented inline. Recorded SilentPaymentPending.parent_puzzle_hash now used (was dead-code-deny). 2 new tests: multi_party_hard_errors (SEND-03 Spends-level) + round_trip_matches_derive_one_time_puzzle_hash (SEND-04 finish-time; byte-equals derive_one_time_puzzle_hash). ROADMAP Phase 4 success criteria #1 + #2 closed at SDK level (Phase 6 closes simulator-level). Driver test count 1070 -> 1072. Zero new #[allow]; only scanner.rs #[allow] from Plan 03-03 remains across silent_payments/. SEND-03 (Spends-level) + SEND-04 (finish-time) requirements complete.
- [Phase 04]: Unit-return signature on emit_silent_payment_announcements (clippy::unnecessary_wraps); no #[allow] added
- [Phase 04]: ConditionsSpend::conditions_ref accessor added for test introspection; pub visibility matches finish/add_conditions
- [Phase 04]: Lex-min coin_id announcer selection (vs Python iteration-order) for order-independent announcement binding
- [Phase 04]: Method-on-Spends shape for emit_silent_payment_announcements (vs free function over FungibleSpends); needed for test introspection independent of finish-time consume
- [Phase 04]: Plan 04-05: memo_hint_guard inlined in actions/silent_payment_send.rs; FIRST-line call in SilentPaymentSend::spend fires DriverError::SilentPaymentMemoHintForbidden on 32-byte first memo BEFORE any side-effects. Defensive CLVM walk; 5 non-hazard cases return Ok (None, non-pair, non-atom head, non-32-byte first atom, malformed). 3 named tests pin contract.
- [Phase 04]: Plan 04-05: Memos<NodePtr> passed by value (clippy::trivially_copy_pass_by_ref); no #[allow] added. Cross-module re-export of SilentPaymentSend goes through crate::actions::* (the private silent_payment_send module's public re-export) instead of crate::actions::silent_payment_send::SilentPaymentSend (E0603).
- [Phase 04]: Plan 04-05: aggregate_sender_sks DELIBERATELY omitted from umbrella prelude per RESEARCH Anti-Pattern 2 — too easy to misuse outside Spends::finish_with_silent_payment_keys invariants. Prelude chip-0057 driver block now 14 symbols (was 11): SilentPaymentSend, derive_one_time_puzzle_hash, compute_input_hash added.
- [Phase 04]: Phase 4 COMPLETE: 18-gate matrix green, all 6 ROADMAP success criteria PASS, all 8 SEND-* requirements (SEND-01..08) closed. Workspace test count 2399 (Phase 3) -> 2416 (+17). 5 plans across 5 waves (~93 min cumulative). Zero new workspace deps; zero new #[allow] anywhere in Phase 4 (only scanner.rs allow from Plan 03-03 remains). Phase 5 (Bindings) unblocked.
- [Phase 04.1]: Plan 04.1-01: Deletion-half of FINGERPRINT-01 closed. emit_silent_payment_announcements + Step 8.5 + 4 unused imports + 2 announcement-specific tests deleted. multi_party_hard_errors and input_hash_round_trip updated to Relation::AssertConcurrent (production-correct value; preempts Plan 04.1-02 runtime gate). send_keys.rs 319->236 (-83 lines); silent_payment_send.rs 778->640 (-138 lines). 9 surviving SP tests green; full workspace --all-features build clean; clippy clean under chip-0057.
- [Phase 04.1]: Plan 04.1-01 deviation (Rule 3 - blocking): Task 1 commit produces transient test-build break until Task 2 lands (Task 2-deleted tests still reference Task 1-deleted method). Committed Task 1 with non-test library build verified + grep gates green; documented in commit. Future plan authors: when deleting a pub(crate) symbol referenced from cfg(test) in another file, those test deletions must commit either first or atomically with the symbol deletion.

### Roadmap Evolution

- 2026-05-17: **Phase 4.1 inserted after Phase 4** (URGENT): Sage-style send-side binding refactor. Drops the SP-specific opcode 60/61 empty-message announcement binding emitted by Phase 4's `Spends::emit_silent_payment_announcements`; switches to relying on the SDK's existing `Relation::AssertConcurrent` cycle (opcode 64 SCC) for multi-input atomicity. Discovered mid-session as a privacy regression: the empty-message announcement is uniquely identifiable as silent-payment traffic, eroding the SP anonymity set. The companion CHIP-0057 reference repo (`~/silent-payments`) already adopted this design (its Phase 2). MUST ship before Phase 5 (Bindings) so the napi/pyo3/wasm surface stabilizes on the post-refactor API. New requirement FINGERPRINT-01 introduced (multi-input SP transactions emit no SP-specific on-chain marker).

### Pending Todos

None yet.

### Blockers/Concerns

Open architectural questions to resolve at the relevant phase entry (from research/SUMMARY.md "Consolidated Open Questions"):

- **Phase 4 (entry):** Option A vs B for deferred ECDH in `SilentPaymentSend` (Q1). Plan-phase should schedule a design spike before full implementation.
- **Phase 4 (entry):** `SyntheticSecretKey` newtype vs documented `&[SecretKey]` parameter (Q2) for `aggregate_sender_sks`.
- **Phase 5 (pre-flight):** Verify `bindy-macro` `"type": "static_functions"` schema support (Q3) before committing the JSON descriptor; fallback strategy documented in research/ARCHITECTURE.md.
- **Phase 2 (pre-flight):** Audit downstream consumers of `chia-sdk-utils` for the new optional `chia-sdk-types -> chia-sdk-utils` dep edge (Q8).

## Session Continuity

Last session: 2026-05-17T18:42:58.444Z
Stopped at: Completed 04.1-01-PLAN.md (FINGERPRINT-01 deletion half + SEND-06 re-validation; Plan 04.1-02 = runtime gate)
Resume file: None
