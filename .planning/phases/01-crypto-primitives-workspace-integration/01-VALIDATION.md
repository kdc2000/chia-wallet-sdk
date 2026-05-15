---
phase: 1
slug: crypto-primitives-workspace-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-15
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in Rust harness) + `rstest 0.22.0` (already in `chia-sdk-types/[dev-dependencies]`, optional) |
| **Config file** | None — `chia-sdk-types/Cargo.toml [dev-dependencies]` already has `hex`, `rstest`, `anyhow`, `rand`, `rand_chacha` |
| **Quick run command** | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Estimated runtime** | Quick: <5s; Full: ~minutes (matches CI) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments`
- **After every plan wave:** Run the gate commands (build matrix + clippy + fmt + machete + grep ban) AND `cargo test --release -p chia-sdk-types --all-features`
- **Before `/gsd:verify-work`:** Full suite must be green AND every success criterion from ROADMAP Phase 1 must verify
- **Max feedback latency:** Quick path < 5 seconds

---

## Per-Task Verification Map

> Tasks below will be defined by the planner in PLAN.md files. The Req-ID → test mapping is locked here. The planner MUST assign each test to a task and mark each test as Wave 0 (file creation) or post-Wave-0 (assertion only). Test types: **unit** (Rust `#[test]`), **structural** (build / file-content check), **lint** (clippy / cargo-machete), **grep-lint** (CI `grep` step).

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 0 | CRYPTO-01 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_unsigned_max_input_reduces_to_r_minus_one -- --exact` | ❌ W0 — `crates/chia-sdk-types/src/silent_payments/scalar.rs` | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-01 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_unsigned_identity -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-01 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::mul_mod_r -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-01 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::add_wraps_at_r -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-01 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::scalar::tests::from_bytes_raw_does_not_reduce -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-02 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tagged_hash_matches_bip340_challenge_vector -- --exact` | ❌ W0 — `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-02 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_inputs_hash_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-02 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_shared_secret_hash_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-02 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::tag_label_hash_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | CRYPTO-02 | unit | `cargo test --release -p chia-sdk-types -F chip-0057 silent_payments::tagged_hash::tests::different_tags_different_outputs -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-01 | structural / build | `cargo build --release --workspace --all-features && cargo build --release -p chia-sdk-types -F chip-0057` | ❌ W0 — Cargo.toml edits | ⬜ pending |
| TBD | TBD | 0 | WS-01 | structural | `cargo build --release -p chia-sdk-driver -F chip-0057 && cargo build --release -p chia-sdk-utils -F chip-0057` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-02 | structural | `grep -E 'chia-sdk-types.*chip-0057' .github/workflows/rust.yml` (≥1 line) | ❌ W0 — `.github/workflows/rust.yml` edit | ⬜ pending |
| TBD | TBD | 0 | WS-03 | structural / lint | `cargo clippy --workspace --all-features --all-targets -- -D warnings` exits 0 | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-03 | structural / lint | `cargo machete` exits 0; no new `[package.metadata.cargo-machete] ignored` entries (success criterion 5) | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-03 | grep-lint | `! grep -r 'mod_by_group_order' crates/chia-sdk-types/src/silent_payments/` (success criterion 4) | ❌ W0 — module is being created | ⬜ pending |
| TBD | TBD | 0 | WS-03 | grep-lint | `! grep -rE '^use sha2::' crates/chia-sdk-types/src/silent_payments/` (defense-in-depth: enforce chia-sha2 only) | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-03 | structural | `cargo fmt --all -- --files-with-diff --check` exits 0 | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

All Phase 1 files are net-new. Wave 0 must create:

- [ ] `crates/chia-sdk-types/src/silent_payments/mod.rs` — barrel + module doc-comments
- [ ] `crates/chia-sdk-types/src/silent_payments/scalar.rs` — `ScalarField` + `GROUP_ORDER` + `#[cfg(test)] mod tests`
- [ ] `crates/chia-sdk-types/src/silent_payments/tagged_hash.rs` — `tagged_hash` + tag consts + `#[cfg(test)] mod tests`
- [ ] `crates/chia-sdk-types/src/silent_payments/paths.rs` — `SCAN_PATH` / `SPEND_PATH` constants
- [ ] Edit `crates/chia-sdk-types/src/lib.rs` — add `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- [ ] Edit `Cargo.toml` (root) — append `chip-0057 = [...]` to `[features]`
- [ ] Edit `crates/chia-sdk-types/Cargo.toml` — append `chip-0057 = []` to `[features]`
- [ ] Edit `crates/chia-sdk-driver/Cargo.toml` — append `chip-0057 = ["chia-sdk-types/chip-0057"]` to `[features]`
- [ ] Edit `crates/chia-sdk-utils/Cargo.toml` — add `[features]` block with `chip-0057 = []`
- [ ] Edit `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-types -F chip-0057` line

**Framework install:** Not needed. `cargo test` is built-in. `hex_literal`, `chia-sha2`, `num-bigint` are already in `chia-sdk-types/[dependencies]` or `[dev-dependencies]`.

**Pinned tag-hash byte computation:** Wave 0 must compute the three pinned SHA-256 byte values (one per tag) before the tag-pin tests can pass with real assertions. Approach: write tests with placeholders, run with `-- --nocapture`, copy printed values into the test source.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| (none) | — | — | All phase behaviors have automated verification. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s for quick path
- [ ] `nyquist_compliant: true` set in frontmatter after planner assigns task IDs

**Approval:** pending
