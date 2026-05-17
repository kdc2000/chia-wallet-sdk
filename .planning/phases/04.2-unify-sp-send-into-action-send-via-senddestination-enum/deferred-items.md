# Phase 04.2 Deferred Items

Out-of-scope discoveries logged per the GSD executor "SCOPE BOUNDARY" rule. These were observed during Plan 04.2-03 execution but predate Phase 04.2; not fixed under this plan.

## Pre-existing rustdoc warning in chia-sdk-utils

**Location:** `crates/chia-sdk-utils/src/silent_payments/mod.rs:40`
**Error (when running `cargo doc -p chia-sdk-utils --all-features --no-deps`):**

```
error: public documentation for `generate_label` links to private item `labels::generate_label`
  --> crates/chia-sdk-utils/src/silent_payments/mod.rs:40:33
   |
40 | /// Public reach-through over [`labels::generate_label`] (which is `pub(crate)`
```

**Origin:** Introduced by Plan 03-04 (Phase 3 reach-through promotion of `generate_label` from `pub(super)` to `pub(crate)` + `pub fn generate_label` wrapper in `mod.rs`).

**Why deferred:** Pre-existing on `main` (verified via `git stash` + repeated `cargo doc` invocation against the unmodified tree). Phase 04.2 touched no chia-sdk-utils source — the warning is not caused by Phase 04.2's changes.

**Severity:** `-D rustdoc::private-intra-doc-links` is a deny-level docs gate but it does not appear in CI (CI runs lints + clippy + tests, not `cargo doc`). Workspace builds, clippy, fmt, machete, and full test suite all pass.

**Suggested fix path:** Downgrade the intra-doc link to plain code (`` `labels::generate_label` `` -> drop the brackets) or add `#[allow(rustdoc::private_intra_doc_links)]` on the wrapper. Pick a future Phase 5 (Bindings) plan that touches chia-sdk-utils to roll this fix in.
