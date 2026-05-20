# Phase 8: Second-pass v1 polish — tighten SP module surface and dispatch ergonomics - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Tighten 4 structural details in the chip-0057 silent-payments code that a maintainer would flag in PR review — identified by a post-Phase-7 quality survey, distinct from the 5 nits Phase 7 already fixed. Pure refactor — no behavior change, no public API removals (the prelude surface stays intact), no new tests, no new dependencies. Same shape as Phase 7.

**In scope:** 4 polish tasks (POLISH-01, -02, -03, -04). All edits land in `crates/chia-sdk-driver/` source files — no binding or test changes expected.

**Out of scope (per discussion):**
- POLISH-05 (extract `scanner.rs` inline tests) — DROPPED. Repo convention IS inline `#[cfg(test)] mod tests {}` at file bottom across CAT, NFT, Singleton, Vault. Phase 7's CLEANUP-05 was explicitly dropped after the same discovery; Phase 8 honors the precedent. `scanner.rs` at 735 lines stays as-is. If a broader "split large test modules" rule emerges in a future polish phase, scanner.rs comes along then.
- Demoting protocol primitives (`compute_shared_secret_from_tweak`, `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`) from `pub` to `pub(crate)` — rejected. They're documented as public reusable primitives in `silent_payments/mod.rs` rustdoc + listed in `src/prelude.rs:43-44`. Demoting would be a documented-API break for unclear gain (no external consumer reaches them outside the prelude). POLISH-01 only kills the wildcards.
- New features, new requirements, new tests, behavior changes. Belong in v2 or follow-up cleanups.

</domain>

<decisions>
## Implementation Decisions

### POLISH-01 — Tighten `silent_payments/mod.rs` re-exports

- **D-01:** **Switch wildcards to named `pub use`** in `crates/chia-sdk-driver/src/silent_payments/mod.rs`. After this phase's POLISH-02 fold, the mod.rs body becomes:

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

  **Why this and not demote-to-`pub(crate)`:** the protocol primitives are documented as public reusable surface in `silent_payments/mod.rs:9-13` rustdoc ("The protocol primitives ... are exposed publicly so the send-side action and any caller that needs to compute shared secrets manually can reuse them without round-tripping through the scanner") AND listed in `src/prelude.rs:43-44`. Demoting them is a documented-API break for ambiguous gain (no external consumer reaches them outside the prelude today, but the documented intent commits to reusability). Switching wildcards to explicit `pub use foo::{Name, …}` addresses the maintainer's actual concern — making the public surface visible at the module's top — without breaking the documented commitment.

  **`pub(crate) use send_keys::*` is correct as-is** (per Phase 7 audit). Keep the wildcard there since the module only contains crate-private items and `*` is appropriate for crate-private re-exports.

  **Acceptance grep:** `grep -nE '^pub use [a-z_]+::\*;' crates/chia-sdk-driver/src/silent_payments/mod.rs` returns 0 hits.

### POLISH-02 — Fold single-function modules into `protocol.rs`

- **D-02:** **Fold `aggregate.rs` + `input_hash.rs` + `one_time.rs` into `crates/chia-sdk-driver/src/silent_payments/protocol.rs`.** The three modules each have exactly one `pub fn` plus inline tests. Result:

  - **Delete:** `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` (91 lines)
  - **Delete:** `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` (157 lines)
  - **Delete:** `crates/chia-sdk-driver/src/silent_payments/one_time.rs` (193 lines)
  - **Grow:** `crates/chia-sdk-driver/src/silent_payments/protocol.rs` from 199 → ~640 lines (well under `scanner.rs`'s 735; within repo norms).
  - **Update:** `crates/chia-sdk-driver/src/silent_payments/mod.rs` drops 3 `mod` declarations + 3 `pub use` lines.

  **Test consolidation:** each folded module has its own `#[cfg(test)] mod tests {}` block at file bottom. Merge them under a single `#[cfg(test)] mod tests {}` at the bottom of `protocol.rs` (or keep them as named sub-mods like `mod tests { mod aggregate_tests; mod input_hash_tests; mod one_time_tests; }` if there's a name collision). Planner picks the merger shape. Test names + assertions stay byte-identical; only file location changes. Acceptance: `cargo test --release -p chia-sdk-driver --features chip-0057` runs the same test count as pre-phase (same test names visible in the test output).

  **Imports inside `protocol.rs`:** after fold, the protocol primitives that `derive_one_time_puzzle_hash` composes are siblings in the same file — no `use crate::silent_payments::{...}` paths needed for them. `aggregate_sender_sks` and `compute_input_hash` likewise become file-local references.

  **External callsites NOT affected:** all current consumers (`chia-sdk-bindings/src/silent_payments.rs`, `crates/chia-sdk-test/src/silent_payments/tweak_data.rs`, `crates/chia-sdk-driver/src/actions/silent_payment_send.rs`, `crates/chia-sdk-driver/src/action_system/spends.rs`) import via `chia_sdk_driver::silent_payments::*` or named imports from the same module path — those paths still resolve identically after fold (the `pub use` in mod.rs republishes from protocol.rs instead of the deleted modules).

  **Why protocol.rs (not a new `send.rs`):** receive-side scanner.rs already imports the protocol primitives (`use crate::silent_payments::{compute_shared_secret_from_tweak, derive_onetime_pk, ...}` at scanner.rs:31-32). Folding into protocol.rs keeps "the primitive set + the composers that use it" co-located. A separate `send.rs` would force scanner.rs and one_time.rs to live apart even though they share inputs.

  **Acceptance:** `test ! -f crates/chia-sdk-driver/src/silent_payments/aggregate.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/input_hash.rs && test ! -f crates/chia-sdk-driver/src/silent_payments/one_time.rs`. `wc -l crates/chia-sdk-driver/src/silent_payments/protocol.rs` ≤ 700 (target ~640). Public test count unchanged.

### POLISH-03 — De-duplicate `SendDestination` Boxing rationale

- **D-03:** **Keep the variant-level rustdoc; delete the enum-level Boxing/Copy mention** in `crates/chia-sdk-driver/src/action_system/send_destination.rs`. Specifically:

  - **Delete** lines 21-23 of `send_destination.rs` (the `/// Cannot derive `Copy` because `SilentPaymentAddress` is `Clone`-only.` paragraph). The enum-level rustdoc keeps its high-level purpose statement (lines 13-20) and the privacy-warning paragraph (lines 25-32) — only the Boxing/Copy note goes.
  - **Keep** lines 39-43 of `send_destination.rs` (the variant-level rustdoc explaining boxing). That's the canonical location, directly above `Box<SilentPaymentAddress>`.

  **Net:** ~3 fewer lines of doc; no information loss; canonical location preserved.

  **Acceptance:** `grep -c 'large_enum_variant' crates/chia-sdk-driver/src/action_system/send_destination.rs` returns exactly 1 (only the variant-level doc retains the clippy lint reference). `grep -c 'Cannot derive \`Copy\`' crates/chia-sdk-driver/src/action_system/send_destination.rs` returns 0.

### POLISH-04 — Restructure `Action::send` chip-0057 dispatch

- **D-04:** **Single exhaustive `match` with SP arm `return`-ing from inside.** Rewrite `crates/chia-sdk-driver/src/actions/send.rs:44-62` (the chip-0057 dispatch block) from the current if-let + post-match-with-`unreachable!()` shape to:

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

  **Net:** the early-return `if let SendDestination::SilentPayment(addr) = &self.destination` block (lines 44-54) AND the post-match `unreachable!("handled above")` arm (lines 60-61) AND the explanatory comments at lines 41-43 + 56-58 all collapse into the single match above. Estimated ~8-10 source-line reduction with cleaner top-to-bottom flow. Behavior unchanged — both PuzzleHash and SilentPayment dispatches end up in the same control-flow points they do today (PuzzleHash falls through to the Cat/Did/Nft/Option dispatch; SilentPayment short-circuits via `return`).

  **Why not extract a `dispatch_puzzle_hash_target` helper:** Option C (Extract puzzle-hash dispatch into a helper) is a larger diff than the nit warrants. The current Cat/Did/Nft/Option dispatch in `send.rs` (~lines 70-130) reads fine as-is; extracting would invent a helper signature for one caller. POLISH-04 stays surgical.

  **Acceptance:** `grep -c 'unreachable!' crates/chia-sdk-driver/src/actions/send.rs` returns 0. `grep -c 'handle_silent_payment_send' crates/chia-sdk-driver/src/actions/send.rs` returns 1 (down from 1 in the current early-return; the match arm calls it once).

### Claude's Discretion

- **Test-module merger shape in POLISH-02.** Planner picks between (a) one flat `mod tests {}` at the bottom of protocol.rs with all merged test fns; (b) `mod tests { mod aggregate_tests; mod input_hash_tests; mod one_time_tests; }` preserving the original groupings as sub-modules. Both are valid; (a) is simpler, (b) preserves the historical test-organization signal. Pick whichever reads cleanest after the fold.
- **Whether to delete the now-dead `Memos` re-import in `silent_payments/protocol.rs`** after the fold. Some of the folded modules import types that protocol.rs already imports (`ScalarField`, `SecretKey`, `PublicKey`, `Bytes32`). Planner deduplicates the use-statements; clippy will catch unused imports if missed.
- **Whether to update `silent_payments/mod.rs` rustdoc** to reflect the smaller module count. The current rustdoc (lines 1-30) references "compute_shared_secret_from_tweak, derive_output_tweak, derive_onetime_pk, derive_onetime_sk, puzzle_hash_for_pk" by name — those references still work post-fold. Optional small edit: remove any wording that implies these live in separate sub-modules (the mod.rs intro shouldn't claim a module structure that no longer exists).
- **Phase ordering and wave assignment.** Suggested ordering: POLISH-01 (named exports — touches mod.rs only) → POLISH-02 (fold — biggest diff, depends on POLISH-01 having named the symbols) → POLISH-03 (doc dedup — independent) → POLISH-04 (dispatch — independent). POLISH-01/02 are serially dependent; POLISH-03 and POLISH-04 are independent and could parallelize with either or each other. Planner picks waves.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Files Phase 8 modifies
- `crates/chia-sdk-driver/src/silent_payments/mod.rs` — POLISH-01 named exports, POLISH-02 module declarations
- `crates/chia-sdk-driver/src/silent_payments/aggregate.rs` — POLISH-02 deletion target
- `crates/chia-sdk-driver/src/silent_payments/input_hash.rs` — POLISH-02 deletion target
- `crates/chia-sdk-driver/src/silent_payments/one_time.rs` — POLISH-02 deletion target
- `crates/chia-sdk-driver/src/silent_payments/protocol.rs` — POLISH-02 fold target (grows from 199 → ~640 lines)
- `crates/chia-sdk-driver/src/action_system/send_destination.rs` — POLISH-03 doc dedup
- `crates/chia-sdk-driver/src/actions/send.rs` — POLISH-04 dispatch restructure

### External consumers Phase 8 must NOT break (verify after fold)
- `crates/chia-sdk-bindings/src/silent_payments.rs:407-433` — calls `chia_sdk_driver::derive_one_time_puzzle_hash` and `chia_sdk_driver::aggregate_sender_sks` directly. Paths must still resolve after POLISH-02.
- `crates/chia-sdk-test/src/silent_payments/tweak_data.rs:10` — imports `OutputMeta, TweakData, compute_input_hash` from `chia_sdk_driver::silent_payments::*`. Paths must still resolve.
- `crates/chia-sdk-driver/tests/silent_payments_e2e.rs:24` — imports `K_MAX_DEFAULT, scan_from_tweaks` from `chia_sdk_driver::silent_payments::*`. Paths must still resolve.
- `src/prelude.rs:41-45` — re-exports 12 SP names. All names must remain reachable via `chia_sdk_driver::silent_payments::*`.
- `crates/chia-sdk-driver/src/actions/silent_payment_send.rs:138` — imports `silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash}`. Paths still resolve.
- `crates/chia-sdk-driver/src/action_system/spends.rs:597` — imports `silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash}`. Paths still resolve.

### Code review findings (the source of this phase's scope)
- Inline conversation review on 2026-05-20 (post-Phase-7 quality survey) — captured in this CONTEXT.md's decisions above. No external doc.

### Repo convention precedents
- `crates/chia-sdk-driver/src/primitives/cat/cat_info.rs` (113 lines) — flat-sibling primitive convention with inline tests; POLISH-02's protocol.rs fold respects this shape.
- `crates/chia-sdk-driver/src/primitives/nft/nft_info.rs` (311 lines) — same convention; protocol.rs's ~640-line target is consistent with NFT's larger sibling files.
- `crates/chia-sdk-driver/src/silent_payments/scanner.rs` (735 lines) — establishes the in-module ceiling Phase 8 stays under.

### Phase 7 artifacts that established the precedent for this phase
- `.planning/phases/07-code-review-cleanup/07-CONTEXT.md` — Phase 7's CONTEXT pattern (locked decisions + acceptance greps) is the template Phase 8 follows.
- `.planning/phases/07-code-review-cleanup/07-VERIFICATION.md` — Phase 7's verification pattern (grep oracles + wc + test command). Phase 8 expects similar shape.
- `.planning/phases/07-code-review-cleanup/07-04-PLAN.md` Task 2 — the gsd-tools `phase complete` patch installed during Phase 7 will auto-flip Phase 8's VALIDATION.md frontmatter when this phase passes verification.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`pub use foo::{Name, …}` explicit-named re-export style** — already in use elsewhere in the workspace. `src/prelude.rs:35-39` (utils SP types) and `:41-45` (driver SP names) both use named lists, not wildcards. POLISH-01 brings `silent_payments/mod.rs` in line with that style.
- **Single-`mod tests {}` per file convention** — every primitive in `crates/chia-sdk-driver/src/primitives/` (CAT, NFT, DID, Singleton, Vault, OptionContract) has one inline `#[cfg(test)] mod tests {}` block at file bottom. POLISH-02's test merger respects this convention.
- **`#[cfg(feature = "chip-0057")]` at variant + arm level** — `send_destination.rs:42` (variant) and `send.rs:48,60` (arms) demonstrate the cfg-gating pattern POLISH-04 inherits.

### Established Patterns
- **No backwards-compat shims** (per CLAUDE.md) — POLISH-02 deletes 3 files outright with no re-export shims. Same as Phase 7's `Spends::finish_silent_payments` delete (CLEANUP-03) and `e2e.rs` delete (CLEANUP-04).
- **Grep/wc/file-existence acceptance oracles** — Phase 7 established these as the lightweight verification pattern for pure-refactor phases. Phase 8 follows the same.
- **`#[from]` and named-error variants in `DriverError`** — POLISH-04's match restructure does NOT add or remove error variants; the existing `DriverError::SilentPaymentRequiresXch` etc. flow through `handle_silent_payment_send` unchanged.

### Integration Points
- **`silent_payments/mod.rs`** — central registry for re-exports. POLISH-01 + POLISH-02 both modify it.
- **`silent_payments/protocol.rs`** — POLISH-02 fold target. After Phase 8, this file is the single point of truth for protocol primitives + their compositions.
- **`actions/send.rs`** — POLISH-04 dispatch restructure site. The `actions/silent_payment_send.rs` helper that the SP arm calls is unchanged.
- **`src/prelude.rs:41-45`** — must continue to resolve all 12 SP names after POLISH-01/02. Acceptance grep: `cargo check --release --workspace --all-features` clean.
- **`bindings/silent_payments.json` + `chia-sdk-bindings/src/silent_payments.rs`** — untouched by Phase 8. Verify with `git diff --stat` showing no changes under those paths.

</code_context>

<specifics>
## Specific Ideas

- **POLISH-01 mod.rs line-by-line target** — the locked decision above includes the exact post-fold mod.rs body. Planner copies it verbatim (possibly reordering sub-imports alphabetically per workspace style — `crates/chia-sdk-utils/src/silent_payments/mod.rs` for reference).
- **POLISH-02 protocol.rs file order** — suggested order: protocol primitives FIRST (the 5 `compute_shared_secret_from_tweak`, `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`, `puzzle_hash_for_pk`), then the 3 compositions in logical pipeline order (`aggregate_sender_sks` → `compute_input_hash` → `derive_one_time_puzzle_hash`). Matches the data-flow narrative a reader expects.
- **POLISH-03 `large_enum_variant` clippy reference** — the variant-level rustdoc references this lint by name. Keep that reference (it explains WHY boxing is required — clippy would warn otherwise). The enum-level deletion only drops the `/// Cannot derive Copy …` paragraph, not the variant doc.
- **POLISH-04 comment cleanup** — when the if-let + unreachable!() pair goes away, the two existing comments justifying them (lines 41-43 "chip-0057 SP arm: delegate to the dedicated module…" and 56-58 "PuzzleHash destination — exhaustive extraction…") also go. Replace with a single 1-2 line comment if needed, or none.
- **Workspace test suite continuity gate** — every POLISH-* commit must keep the full workspace test suite green. Same invariant as Phase 7. Phase-end verification runs the same full-workspace command.

</specifics>

<deferred>
## Deferred Ideas

- **POLISH-05 (extract `scanner.rs` inline tests)** — dropped. Repo convention is inline tests; same reasoning as Phase 7's CLEANUP-05 drop. If a future phase introduces a workspace-wide "split large test modules" rule, scanner.rs joins then.
- **Demote protocol primitives to `pub(crate)`** — rejected. Public-API commitment in mod.rs rustdoc + `src/prelude.rs`. Not worth the doc/prelude churn for ambiguous gain. Reconsider as part of a v2 API audit if the primitives go genuinely unused for several minor versions.
- **Extract `dispatch_puzzle_hash_target` helper in `send.rs`** — rejected as a larger diff than the nit warrants. The current Cat/Did/Nft/Option dispatch reads fine; extracting would invent a helper signature for one caller. Reconsider if future work touches that dispatch.
- **Update `silent_payments/mod.rs` rustdoc to reference fewer sub-modules** — left to Claude's Discretion in D-02 (planner edits opportunistically; otherwise the existing rustdoc still reads correctly since it names primitives by symbol, not by sub-file location).
- **Sage/Wallet-side consumer audit** — not part of Phase 8. The named-import POLISH-01 change doesn't break any consumer path that uses `chia_sdk_driver::silent_payments::*` or `chia_wallet_sdk::prelude::*`. If Sage uses an unusual import path that the audit in `<canonical_refs>` didn't surface, planner discovers + fixes at execution time.
- **Other v1.1 polish items** — anything else a maintainer might raise in actual upstream review (further mod.rs slimming, additional doc rewrites, etc.) goes to a v1.1 backlog. Phase 8 only addresses the 4 explicit concerns from the post-Phase-7 quality survey.

</deferred>

---

*Phase: 08-second-pass-v1-polish-tighten-sp-module-surface-and-dispatch-ergonomics*
*Context gathered: 2026-05-20*
