---
phase: 2
slug: address-key-types
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-15
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Distilled from `02-RESEARCH.md §12`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in libtest); `rstest 0.22.0` available as workspace dev-dep for parametric coverage |
| **Config file** | None — `cargo test --release -p chia-sdk-utils --features chip-0057` is the canonical invocation |
| **Quick run command** | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments` |
| **Full suite command** | `cargo test --release --workspace --all-features --exclude chia-wallet-sdk-napi --exclude chia-sdk-derive --exclude chia-wallet-sdk-py --exclude chia-wallet-sdk-wasm --exclude chia-sdk-bindings --exclude bindy --exclude bindy-macro` |
| **Estimated runtime** | Quick: <5s incremental; Full: ~minutes (matches CI) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments`
- **After every plan wave:** Quick run command + 5-build sweep (`workspace --all-features`, `workspace` no features, `-p chia-sdk-utils -F chip-0057`, `-p chia-sdk-utils` no features, plus any new per-crate CI line)
- **Before `/gsd:verify-work`:** Full suite green AND clippy clean AND `cargo machete` clean AND grep-bans hold (no `mod_by_group_order`, no `use sha2::` under `silent_payments/`)
- **Max feedback latency:** <5 seconds incremental on `chia-sdk-utils`

---

## Per-Task Verification Map

> Task IDs are TBD until the planner finalizes plan/wave assignment. The Req-ID → test mapping below is locked. The planner MUST attach every test below to a task and mark it Wave 0 (file creation) or post-Wave-0 (assertion only).

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 0 | ADDR-01 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_mnemonic_tv1_scan_sk_matches -- --exact` | ❌ W0 — `crates/chia-sdk-utils/src/silent_payments/keys.rs` | ⬜ pending |
| TBD | TBD | 0 | ADDR-01 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_mnemonic_tv1_spend_sk_matches -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-01 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_mnemonic_tv1_scan_pk_matches -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-01 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_mnemonic_tv1_spend_pk_matches -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv1_mainnet_round_trip -- --exact` | ❌ W0 — `crates/chia-sdk-utils/src/silent_payments/address.rs` | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv1_testnet_round_trip -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv1_mainnet_encode_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv1_testnet_encode_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_xch_hrp_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_invalid_checksum_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_bech32_not_bech32m_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_short_payload_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_identity_scan_pk_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-02 (neg) | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::decode_identity_spend_pk_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::tv3_label_scalar_matches -- --exact` | ❌ W0 — `crates/chia-sdk-utils/src/silent_payments/labels.rs` (may fold into `keys.rs`) | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::tv3_label_pk_matches -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::tv3_labeled_spend_pk_matches -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv3_mainnet_labeled_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::address::tests::tv3_testnet_labeled_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-03 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::labels_preserve_scan_pk -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-04 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::registry_round_trip -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-04 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::registry_three_labels -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-04 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::registry_lookup_missing -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-04 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::labels::tests::registry_forward_missing -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-05 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_secret_keys_matches_from_mnemonic -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-05 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::from_secret_keys_tv1_mainnet_pinned -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | ADDR-06 | unit | `cargo test --release -p chia-sdk-utils --features chip-0057 silent_payments::keys::tests::labeled_address_zero_rejected -- --exact` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | structural / build | 5-build sweep: `cargo build --release --workspace --all-features` + `cargo build --release --workspace` + `cargo build --release -p chia-sdk-utils -F chip-0057` + `cargo build --release -p chia-sdk-utils` + new per-crate CI line | ❌ W0 — `.github/workflows/rust.yml` | ⬜ pending |
| TBD | TBD | 0 | WS-gate | structural | `grep -E 'chia-sdk-utils.*chip-0057' .github/workflows/rust.yml` (≥1 line) | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | lint | `cargo clippy -p chia-sdk-utils --features chip-0057 --all-targets -- -D warnings` exits 0 | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | lint | `cargo machete` exits 0; no new `[package.metadata.cargo-machete] ignored` entries | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | grep-lint | `! grep -r 'mod_by_group_order' crates/chia-sdk-utils/src/silent_payments/` | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | grep-lint | `! grep -rE '^use sha2::' crates/chia-sdk-utils/src/silent_payments/` (chia-sha2-only invariant) | ❌ W0 | ⬜ pending |
| TBD | TBD | 0 | WS-gate | fmt | `cargo fmt --all -- --files-with-diff --check` exits 0 | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**Property-test candidates (optional, low-cost — see RESEARCH §12):**
- `roundtrip_property` — `decode(encode(addr)) == addr` parameterised over TV1 + TV3 + 6–8 random keypairs via `#[rstest] #[case(...)]`.
- `registry_property` — `lookup(forward(m)) == Some(m)` parameterised over `m ∈ {1, 2, 100, 4_294_967_294}`.

---

## Wave 0 Requirements

- [ ] `crates/chia-sdk-utils/src/silent_payments/mod.rs` — module barrel + doc comment; declared in `lib.rs`
- [ ] `crates/chia-sdk-utils/src/silent_payments/error.rs` — `SilentPaymentError` enum (variants: `WrongHrp(String)`, `Bech32(Bech32Error)`, `PayloadLength(usize)`, `IdentityPublicKey`, `ReservedChangeLabel`, plus any `#[from]` upstream conversions)
- [ ] `crates/chia-sdk-utils/src/silent_payments/address.rs` — `SilentPaymentAddress` + `SilentPaymentNetwork` + all encode/decode tests (positive + 6 negative cases)
- [ ] `crates/chia-sdk-utils/src/silent_payments/keys.rs` — `SilentPaymentKeys` + `from_mnemonic` + `from_secret_keys` + `unlabeled_address` + `labeled_address` (m=0 reject) tests
- [ ] `crates/chia-sdk-utils/src/silent_payments/labels.rs` — `LabelRegistry` + label scalar / label_pk / B_m pinned tests + registry round-trip tests (may fold into `keys.rs` if small)
- [ ] `crates/chia-sdk-utils/Cargo.toml` — `chip-0057 = ["dep:chia-sdk-types", "dep:bip39", "dep:chia-bls"]` with three new `optional = true` deps
- [ ] `crates/chia-sdk-utils/src/lib.rs` — `#[cfg(feature = "chip-0057")] pub mod silent_payments;`
- [ ] `.github/workflows/rust.yml` — add `cargo build --release -p chia-sdk-utils -F chip-0057` line near existing per-crate chip lines
- [ ] (Optional, recommended) `src/prelude.rs` — feature-gated re-export of public Phase 2 types

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| _None_ | — | All ADDR-01..06 behaviors verify automatically via cargo test + grep/build/fmt gates | — |

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all `❌ W0` references in the verification map above
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s on `chia-sdk-utils`
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
