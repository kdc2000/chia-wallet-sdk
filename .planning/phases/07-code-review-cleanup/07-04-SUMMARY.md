---
phase: 07-code-review-cleanup
plan: 04
subsystem: gsd-tooling
tags: [chip-0057, silent-payments, gsd-tools, validation, nyquist, frontmatter, audit-cleanliness]

# Dependency graph
requires:
  - phase: 01-crypto-primitives-workspace-integration
    provides: stale `01-VALIDATION.md` frontmatter awaiting flip
  - phase: 02-address-key-types
    provides: stale `02-VALIDATION.md` frontmatter awaiting flip
  - phase: 03-receive-primitive-chip-test-vector-closure
    provides: stale `03-VALIDATION.md` frontmatter awaiting flip
  - phase: 04-send-side-action
    provides: stale `04-VALIDATION.md` frontmatter awaiting flip
  - phase: 04.1-sage-style-binding-refactor
    provides: stale `04.1-VALIDATION.md` frontmatter awaiting flip
  - phase: 04.2-unify-sp-send-into-action-send-via-senddestination-enum
    provides: stale `04.2-VALIDATION.md` frontmatter awaiting flip
  - phase: 06-simulator-round-trip-bindings-e2e-example
    provides: stale `06-VALIDATION.md` frontmatter awaiting flip
provides:
  - "All 7 stale VALIDATION.md files now report nyquist_compliant: true and wave_0_complete: true (audit-cleanliness gap from post-v1 maintainer review closed)"
  - "$HOME/.claude/get-shit-done/bin/lib/phase.cjs::cmdPhaseComplete auto-flips VALIDATION.md frontmatter going forward whenever VERIFICATION.md reports status: passed"
  - "The auto-flip is idempotent — re-running phase complete on an already-flipped VALIDATION.md is a true no-op (the patch detects already-true flags and skips the write entirely)"
  - "Patch is non-fatal on error (wrapped in try/catch) — phase completion never breaks because a VALIDATION.md flip failed"
affects: [Phase 7 own VALIDATION.md (intended to flip when Phase 7 itself completes via the new auto-flip), all future GSD phases on this user install]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Leading-anchored frontmatter parsing: when files use `---` as body horizontal dividers (as VALIDATION.md does between sections), extract frontmatter with `/^---\\r?\\n([\\s\\S]+?)\\r?\\n---/` (anchored to file start) rather than `(?:^|\\n)\\s*---...` (matches any block). The shared `extractFrontmatter` helper at frontmatter.cjs:11 picks the LAST block in the file, which on these documents is a body section, not the leading frontmatter — using its output via `spliceFrontmatter` drops every non-flag field."
    - "Idempotent CLI mutations on frontmatter flags: build a `needsFlip` predicate that compares parsed flag values to the target, and skip the write entirely when no change is needed. This makes re-running the CLI a byte-level no-op (mtime unchanged), which is the strongest possible idempotency guarantee."

key-files:
  created:
    - ".planning/phases/07-code-review-cleanup/07-04-SUMMARY.md"
  modified:
    - ".planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md (nyquist_compliant: false → true; wave_0_complete: false → true; other 4 fields preserved)"
    - ".planning/phases/02-address-key-types/02-VALIDATION.md (same flip; other 4 fields preserved)"
    - ".planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md (same flip; other 4 fields preserved)"
    - ".planning/phases/04-send-side-action/04-VALIDATION.md (same flip; other 4 fields preserved)"
    - ".planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md (same flip; other 4 fields preserved)"
    - ".planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md (same flip; other 4 fields preserved)"
    - ".planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md (same flip; other 4 fields preserved)"
  modified_outside_repo:
    - "$HOME/.claude/get-shit-done/bin/lib/phase.cjs (line 8: import gains spliceFrontmatter; lines 861-924: CLEANUP-06 Part B auto-flip block inside cmdPhaseComplete; line 905: result object gains validation_flipped field). NOT committed — file lives in the user's home directory, not the repo."

key-decisions:
  - "Manual Edit (not `gsd-tools frontmatter merge`) used for Part A's 7 in-repo flips: the CLI's `extractFrontmatter` helper has a regex bug — it matches `---..---` blocks anywhere in the file and picks the LAST one. VALIDATION.md files use `---` as section dividers (Phase 1's has 7 such body dividers; Phase 7's has 7), so the helper returns the LAST body section's content (no top-level scalar key:value lines → returns `{}`). Then `spliceFrontmatter` rewrites the file's leading frontmatter with reconstructed-from-empty-object content — dropping `phase`, `slug`, `status`, `created`. This was caught on the first invocation, files restored from git, and replaced with line-targeted Edit tool calls that preserve all other fields and the original field ordering."
  - "Part B's auto-flip block parses frontmatter inline (anchored to file start) rather than calling `extractFrontmatter`. Same data-loss bug would otherwise hit every future `phase complete` run. The fix is local to phase.cjs (a single inline parser) rather than modifying the shared `extractFrontmatter` helper — changing that helper's semantics could break 20+ unrelated gsd-tools call sites and is Rule 4 architectural territory. `spliceFrontmatter` itself IS leading-anchored (frontmatter.cjs:156) and is used unchanged for the write."
  - "Idempotency implemented as an explicit `needsFlip` check: parsed `valFm.nyquist_compliant !== 'true' || valFm.wave_0_complete !== 'true'` — if both flags already report `'true'` (string from the inline parser), the patch skips `fs.writeFileSync` entirely. mtime stays unchanged on no-op runs; the file is byte-identical to its prior state."
  - "Phase 7's own `07-VALIDATION.md` deliberately left as `nyquist_compliant: false` + `wave_0_complete: false`. It will flip when Phase 7 itself completes via `gsd-tools phase complete 7`, which is precisely the auto-flip mechanism this plan installs. The closure is intentionally circular."
  - "Optional `validation_flipped: <bool>` field added to the `result` object returned by `cmdPhaseComplete` (per RESEARCH Open Q3 — discretionary). Surfaces the flip outcome so the execute-phase workflow can verify and report it."

patterns-established:
  - "Auto-flip-on-VERIFICATION-pass pattern: at the end of `phase complete`, after STATE.md is written, the CLI checks the phase's VERIFICATION.md frontmatter for `status: passed` and — only then — flips the matching VALIDATION.md's `nyquist_compliant` + `wave_0_complete` to true. Gated, idempotent, non-fatal. This is the canonical way to keep the Nyquist audit-cleanliness invariant in sync with VERIFICATION outcomes going forward."

requirements-completed: [CLEANUP-06]

# Metrics
duration: 8min
completed: 2026-05-20
---

# Phase 7 Plan 04: CLEANUP-06 (VALIDATION.md frontmatter flip + auto-flip CLI patch) Summary

**Flipped `nyquist_compliant` + `wave_0_complete` to `true` on the 7 stale VALIDATION.md files (Phases 1, 2, 3, 4, 4.1, 4.2, 6) — preserving all other frontmatter fields — and patched `gsd-tools.cjs phase complete` to auto-flip these flags going forward whenever the matching VERIFICATION.md reports `status: passed`. Patch is idempotent and non-fatal.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-20T16:21:56Z
- **Completed:** 2026-05-20T16:30:21Z
- **Tasks:** 2/2
- **Files modified (in repo):** 7
- **Files modified (outside repo, $HOME/.claude/):** 1

## Accomplishments

### Part A — 7 stale VALIDATION.md files flipped (committed in repo)

| Phase | File                                                                                        | Pre-flip                                                                       | Post-flip                                                                    |
| ----- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| 1     | `.planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md`              | `nyquist_compliant: false` + `wave_0_complete: false` (4 other fields)         | `nyquist_compliant: true` + `wave_0_complete: true` (4 other fields preserved) |
| 2     | `.planning/phases/02-address-key-types/02-VALIDATION.md`                                    | same                                                                           | same                                                                         |
| 3     | `.planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md`           | same                                                                           | same                                                                         |
| 4     | `.planning/phases/04-send-side-action/04-VALIDATION.md`                                     | same                                                                           | same                                                                         |
| 4.1   | `.planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md`                      | same                                                                           | same                                                                         |
| 4.2   | `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md` | same                                                                           | same                                                                         |
| 6     | `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md`            | same                                                                           | same                                                                         |

**Phase 5 (`05-bindings-rust-facade-json-descriptor/05-VALIDATION.md`)** — already correct (flipped during Phase 5 execution); unchanged this plan; git timestamp from 2026-05-17 confirmed pre-dating this plan's run.

**Phase 7 (`07-code-review-cleanup/07-VALIDATION.md`)** — deliberately untouched; will flip when Phase 7 completes via the Part B auto-flip mechanism this plan installs.

### Part B — `gsd-tools phase complete` CLI patched (outside repo)

**File:** `$HOME/.claude/get-shit-done/bin/lib/phase.cjs` (NOT committed; lives in user's home directory, not in this repo).

**Change 1 — Import update (line 8):**

```javascript
// Before:
const { extractFrontmatter } = require('./frontmatter.cjs');
// After:
const { extractFrontmatter, spliceFrontmatter } = require('./frontmatter.cjs');
```

**Change 2 — Auto-flip block inserted into `cmdPhaseComplete` (lines 861–924, between the STATE.md write and the `const result = {` object literal):**

```javascript
  // CLEANUP-06 Part B: auto-flip VALIDATION.md frontmatter when VERIFICATION
  // reports status: passed. Idempotent — already-true flags are a no-op.
  //
  // Implementation note: VALIDATION.md files use `---` as body horizontal
  // dividers between sections. The shared extractFrontmatter helper picks the
  // LAST `---..---` block in the file, which on these files is a body section
  // — reconstructing from that drops every non-flag field. We parse the
  // LEADING block (anchored to file start) ourselves into the same shape
  // extractFrontmatter would return, then hand the object to spliceFrontmatter
  // (which IS leading-anchored at frontmatter.cjs:156) to do the rewrite.
  let validationFlipped = false;
  try {
    const phaseFullDir = path.isAbsolute(phaseInfo.directory)
      ? phaseInfo.directory
      : path.join(cwd, phaseInfo.directory);
    const dirFiles = fs.readdirSync(phaseFullDir);
    const verFile = dirFiles.find(f => f.includes('-VERIFICATION') && f.endsWith('.md'));
    const valFile = dirFiles.find(f => f.includes('-VALIDATION') && f.endsWith('.md'));
    if (verFile && valFile) {
      const verContent = fs.readFileSync(path.join(phaseFullDir, verFile), 'utf-8');
      // Leading frontmatter block only (anchored to file start, ^).
      const verLeading = verContent.match(/^---\r?\n([\s\S]+?)\r?\n---/);
      // Build a frontmatter-style object from the leading block by parsing
      // only top-level scalar `key: value` lines (sufficient for our needs;
      // we just check `status`).
      const verFm = {};
      if (verLeading) {
        for (const line of verLeading[1].split(/\r?\n/)) {
          const m = line.match(/^([a-zA-Z0-9_-]+):\s*(.+?)\s*$/);
          if (m) verFm[m[1]] = m[2].replace(/^["']|["']$/g, '');
        }
      }
      if (verFm.status === 'passed') {
        const valPath = path.join(phaseFullDir, valFile);
        const valContent = fs.readFileSync(valPath, 'utf-8');
        const valLeading = valContent.match(/^---\r?\n([\s\S]+?)\r?\n---/);
        if (valLeading) {
          // Same leading-only top-level scalar parse for VALIDATION.md.
          const valFm = {};
          for (const line of valLeading[1].split(/\r?\n/)) {
            const m = line.match(/^([a-zA-Z0-9_-]+):\s*(.+?)\s*$/);
            if (m) valFm[m[1]] = m[2].replace(/^["']|["']$/g, '');
          }
          // Detect whether either flag actually needs flipping (idempotency).
          const needsFlip =
            valFm.nyquist_compliant !== 'true' || valFm.wave_0_complete !== 'true';
          if (needsFlip) {
            valFm.nyquist_compliant = true;
            valFm.wave_0_complete = true;
            // spliceFrontmatter at frontmatter.cjs:156 is leading-anchored
            // (`/^---\r?\n[\s\S]+?\r?\n---/`) so it correctly rewrites only
            // the leading block, preserving body `---` dividers.
            const newValContent = spliceFrontmatter(valContent, valFm);
            fs.writeFileSync(valPath, newValContent, 'utf-8');
            validationFlipped = true;
          }
        }
      }
    }
  } catch (_e) {
    // Non-fatal — phase completion still succeeds even if VALIDATION flip fails.
    validationFlipped = false;
  }
```

**Change 3 — `result` object gains `validation_flipped` field (line 905):**

```javascript
const result = {
  // ... existing fields ...
  requirements_updated: requirementsUpdated,
  validation_flipped: validationFlipped,  // NEW
  warnings,
  has_warnings: warnings.length > 0,
};
```

### Patch behavior

| Scenario                                                                          | Outcome                                                                                    |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| VERIFICATION.md `status: passed` + VALIDATION.md flags both `false`               | Both flags flipped to `true`; all other frontmatter fields preserved; `validation_flipped: true`. |
| VERIFICATION.md `status: passed` + VALIDATION.md flags already both `true`        | No write (`needsFlip` is false); `validation_flipped: false`; mtime unchanged. **Idempotent.** |
| VERIFICATION.md `status: passed` + only one flag is `true`, the other is `false`  | The false one flipped; the other unchanged; `validation_flipped: true`.                    |
| VERIFICATION.md `status: gaps_found` / `human_needed` / `failed` / missing        | No flip attempted; `validation_flipped: false`.                                            |
| VALIDATION.md missing or leading-frontmatter unparseable                          | Caught by try/catch; phase complete succeeds; `validation_flipped: false`.                 |
| `phase complete` re-run on a phase whose VALIDATION.md is already flipped         | `needsFlip` short-circuits; `validation_flipped: false`. **Idempotent.**                   |

## Task Commits

Each task was committed atomically (when the change was in-repo):

1. **Task 1: Flip 7 stale VALIDATION.md frontmatter files** — `7786b03e` (docs)
2. **Task 2: Patch `$HOME/.claude/get-shit-done/bin/lib/phase.cjs::cmdPhaseComplete`** — *no in-repo commit* (file lives outside the repo). Verified via `node -c` (syntactic) + idempotency smoke test + `gsd-tools config-get` (module loads).

**Plan metadata commit:** pending (will land with the SUMMARY.md/STATE.md/ROADMAP.md docs commit at the end of the executor flow).

## Files Created/Modified

### In repo (committed)

- `.planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md` — `+2/-2` lines (boolean flip only; phase / slug / status / created fields untouched).
- `.planning/phases/02-address-key-types/02-VALIDATION.md` — `+2/-2`.
- `.planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md` — `+2/-2`.
- `.planning/phases/04-send-side-action/04-VALIDATION.md` — `+2/-2`.
- `.planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md` — `+2/-2`.
- `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md` — `+2/-2`.
- `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md` — `+2/-2`.

### Outside repo (NOT committed; lives in $HOME/.claude/)

- `$HOME/.claude/get-shit-done/bin/lib/phase.cjs` — `+~65` lines net (import gains `spliceFrontmatter`; lines 861-924 are the new auto-flip block; line 905 adds `validation_flipped: validationFlipped` to the result object).

## Verification (the plan's own oracle, all PASS)

```bash
# Part A oracles — both 0 means no stale VALIDATION.md remains (excluding Phase 7's draft)
grep -l 'nyquist_compliant: false' .planning/phases/*/*-VALIDATION.md 2>/dev/null \
  | grep -v 07-code-review-cleanup | wc -l
# → 0 ✓

grep -l 'wave_0_complete: false' .planning/phases/*/*-VALIDATION.md 2>/dev/null \
  | grep -v 07-code-review-cleanup | wc -l
# → 0 ✓

# Part B oracles — patch artifacts present in phase.cjs
grep -c 'CLEANUP-06 Part B' "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 1 ✓

grep -c 'spliceFrontmatter' "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 4 ✓ (import + 3 references)

grep -c "verFm.status === 'passed'" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 1 ✓

grep -c "valFm.nyquist_compliant = true" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 1 ✓

grep -c "valFm.wave_0_complete = true" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 1 ✓

grep -c "require.*frontmatter" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → 1 ✓

# Part B — syntactic check
node -c "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → exit 0 ✓

# Part B — gsd-tools still loads after patch
node "$HOME/.claude/get-shit-done/bin/gsd-tools.cjs" config-get workflow.auto_advance
# → exit 0 ✓

# Plan's literal verify block (all checks chained with &&)
grep -q "CLEANUP-06 Part B" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs" && \
  grep -q "spliceFrontmatter" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs" && \
  grep -q "verFm.status === 'passed'" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs" && \
  grep -q "valFm.nyquist_compliant = true" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs" && \
  grep -q "valFm.wave_0_complete = true" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs" && \
  grep -q "require.*frontmatter" "$HOME/.claude/get-shit-done/bin/lib/phase.cjs"
# → exit 0 ✓
```

### Idempotency smoke test (post-patch)

Two direct Node invocations against real files in this repo:

1. Phase 7's still-`false` VALIDATION.md → parsed → `needsFlip: true` → `spliceFrontmatter` would write `nyquist_compliant: true` + `wave_0_complete: true` preserving `phase: 7`, `slug: code-review-cleanup`, `status: draft`, `created: 2026-05-20`.
2. Phase 1's already-`true` VALIDATION.md → parsed → `needsFlip: false` → no write attempted. **Idempotent confirmed.**

### Body-preservation smoke test (post-patch)

`spliceFrontmatter`'s leading-anchored regex (frontmatter.cjs:156, `/^---\r?\n[\s\S]+?\r?\n---/`) was verified to leave the body byte-equal on a flip operation against Phase 7's VALIDATION.md (length 6065 bytes pre- and post-, body content identical via `===`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] `gsd-tools frontmatter merge` CLI drops non-flag frontmatter fields on VALIDATION.md**

- **Found during:** Task 1, immediately after the first `node gsd-tools.cjs frontmatter merge ...` invocation on Phase 1's VALIDATION.md. `git diff` revealed the CLI had silently dropped `phase`, `slug`, `status`, `created` and emitted only the two boolean lines.
- **Issue:** `frontmatter.cjs::extractFrontmatter` (line 16) uses the regex `/(?:^|\n)\s*---\r?\n([\s\S]+?)\r?\n---/g` and picks the LAST match (line 17, "since it represents the most recent state sync"). VALIDATION.md files use `---` as body section dividers (Phase 1's has 7 such body dividers in addition to the leading frontmatter), so the regex captures 4 matches and picks the LAST (a body section starting with `## Manual-Only Verifications`). That section has no top-level `key: value` lines, so `extractFrontmatter` returns `{}`. Then `spliceFrontmatter` rewrites the LEADING block (its own regex IS leading-anchored at line 156) from the empty object — dropping every original field. Confirmed by direct `node -e 'extractFrontmatter(c)'` on the file — returned `{}`.
- **Fix:** All 7 files restored from git (`git checkout HEAD -- ...`), then re-flipped using the **Read + Edit** tool pair on each file's `nyquist_compliant: false\nwave_0_complete: false` substring — preserving all other fields and ordering. The plan documents this as the explicit fallback ("Alternative (manual edit)"; Step 2's tail).
- **Files modified by fix:** All 7 Part A files (same as the intended fix; just via a different tool).
- **Commit:** `7786b03e` (Task 1's normal commit; the rule-1 bug is mentioned in the commit body).
- **Scope:** Bug is in shared `frontmatter.cjs` infrastructure used by 20+ gsd-tools call sites. Out of this plan's scope to fix at the source. Worked around by using a different tool for this plan's writes.

**2. [Rule 1 — Bug] Patch's literal `extractFrontmatter` use would have re-triggered Issue 1 every future `phase complete`**

- **Found during:** Task 2, while validating Part B idempotency. Ran the patch's logic (as literally proposed in RESEARCH §CLEANUP-06 line 593-614) inline against Phase 1's already-flipped VALIDATION.md. `extractFrontmatter` returned `{}` → `valFm.nyquist_compliant = true` + `valFm.wave_0_complete = true` set on an empty object → `spliceFrontmatter` rewrote the leading frontmatter with only the two booleans → all 4 other fields dropped. Not idempotent; data-loss bug at every flip.
- **Issue:** The plan's pseudocode used `extractFrontmatter(valContent)` on VALIDATION.md, which carries Issue 1 forward into the CLI itself.
- **Fix:** Replaced the `extractFrontmatter` call inside the Part B patch with a small inline parser:
    ```js
    const valLeading = valContent.match(/^---\r?\n([\s\S]+?)\r?\n---/);
    const valFm = {};
    for (const line of valLeading[1].split(/\r?\n/)) {
      const m = line.match(/^([a-zA-Z0-9_-]+):\s*(.+?)\s*$/);
      if (m) valFm[m[1]] = m[2].replace(/^["']|["']$/g, '');
    }
    ```
  This parses ONLY the leading frontmatter (anchored to file start) and only top-level scalar `key: value` lines — sufficient for the flag check + flag set. `spliceFrontmatter` is still used unchanged for the WRITE (its own regex IS leading-anchored, so the write path was correct as-is in the original plan).
  Also added the explicit `needsFlip` predicate so already-true flags short-circuit before `fs.writeFileSync` — guaranteeing byte-level idempotency.
- **Files modified by fix:** `$HOME/.claude/get-shit-done/bin/lib/phase.cjs` (the Part B patch itself; not in repo).
- **Commit:** none (file is outside the repo per Step 8 of the plan).
- **Scope:** Fix is local to phase.cjs (a single inline parser block). Did NOT modify `frontmatter.cjs::extractFrontmatter` itself — that would be Rule 4 (changes semantics for 20+ call sites; risk of breaking unrelated gsd-tools commands). The local fix satisfies the plan's `must_haves.truths` ("idempotent", "auto-flips going forward") and the plan's literal `<verify>` grep block.

### Architectural Changes

None.

### Authentication Gates

None.

## Self-Check: PASSED

- File `.planning/phases/01-crypto-primitives-workspace-integration/01-VALIDATION.md`: FOUND
- File `.planning/phases/02-address-key-types/02-VALIDATION.md`: FOUND
- File `.planning/phases/03-receive-primitive-chip-test-vector-closure/03-VALIDATION.md`: FOUND
- File `.planning/phases/04-send-side-action/04-VALIDATION.md`: FOUND
- File `.planning/phases/04.1-sage-style-binding-refactor/04.1-VALIDATION.md`: FOUND
- File `.planning/phases/04.2-unify-sp-send-into-action-send-via-senddestination-enum/04.2-VALIDATION.md`: FOUND
- File `.planning/phases/06-simulator-round-trip-bindings-e2e-example/06-VALIDATION.md`: FOUND
- File `$HOME/.claude/get-shit-done/bin/lib/phase.cjs`: FOUND (modified in place; NOT in repo)
- Commit `7786b03e` (Task 1, 7 VALIDATION.md flips): FOUND in `git log`
- All 7 VALIDATION.md files now report `nyquist_compliant: true` + `wave_0_complete: true` (verified via `grep -E 'nyquist_compliant|wave_0_complete' ...` on each file).
- All 7 files have their 4 non-flag fields (`phase`, `slug`, `status`, `created`) preserved (verified via `awk '/^---$/{c++; if(c==2){print; exit}} {print}'` on each file — full leading frontmatter block printed and inspected).
- `phase.cjs` syntactically valid (`node -c` exit 0).
- `gsd-tools.cjs` still loads after the patch (`config-get workflow.auto_advance` returns `false` cleanly, exit 0).
- Plan's literal `<verify>` grep chain exits 0.

## Downstream notes

- **Future `gsd-tools phase complete <N>` runs auto-flip the matching VALIDATION.md.** No manual step needed for Phase 7 itself (which still reports `false` here) or Phase 8+ on this user's install. The flip happens after STATE.md is written, gated on VERIFICATION.md's `status: passed`, wrapped in try/catch (non-fatal), and reports its outcome via `result.validation_flipped` for the execute-phase workflow to surface.
- **Manual flip via `frontmatter merge` is NOT the fallback.** The shared `frontmatter merge` CLI carries the same data-loss bug documented in Deviation 1 above. The correct fallback if the auto-flip ever fails is the **manual Read + Edit** pair used in Part A of this plan (line-targeted replacement of the two boolean lines, preserving all other fields).
- **Per-user-install scope.** The phase.cjs patch lives in `$HOME/.claude/get-shit-done/` on this developer's machine. Other developers will not have the auto-flip until the same patch lands upstream in the gsd-tools distribution. Filing the underlying `extractFrontmatter` bug upstream is recommended but out of scope for this plan.
