# Feature Research

**Domain:** Wallet-side silent-payments support in a Rust SDK (consumer: wallet apps like Sage, third-party Chia wallets)
**Researched:** 2026-05-15
**Confidence:** HIGH for protocol-derived features (CHIP spec, BIP-352, prototype); MEDIUM for Bitcoin-prior-art UX patterns (WebSearch-only, no Context7 entries for these wallets)

## Scope Reminder

v1 of this milestone is intentionally narrow. The SDK ships:
- Address generation (plain + labeled)
- Send-side XCH primitives + a `SilentPaymentSend` Spends action
- A transport-agnostic receive primitive (`scan_from_tweaks(TweakData)`)
- Bindings (napi/pyo3/wasm)
- Simulator round-trip tests

It does NOT ship: a transport client, GCS filters, CAT2 sends, NFT sends, hardware-wallet UX, or any wallet-side persistence (label DB, scan cursor, etc.). The SDK is a library; persistence belongs to the wallet.

This document categorizes features in two layers:

1. **SDK-layer features** — the API surface this milestone is locked to build.
2. **Wallet-layer features** — what downstream wallets (Sage etc.) typically build on top of an SDK like this. Useful to know because it shapes which SDK surfaces must exist for the wallet to be usable.

## Feature Landscape

### Table Stakes (Users Expect These)

Features that, if missing, make the SDK or any consuming wallet feel broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Bech32m-encoded address** with HRP `spxch`/`tspxch` | Every Bitcoin SP wallet uses bech32m (`sp1q...`/`tsp1q...`). Hex pubkey pairs are not a thing in any production wallet. The prototype's hex output is a debug artifact, not a UX. | LOW — `chia-sdk-utils` already has a bech32m `Address` helper for `xch`/`txch`; this is a second HRP and 96-byte payload variant. | Locked as ADDR-02. HRP choice (`spxch`/`tspxch`) is documented in PROJECT.md key decisions. |
| **Address parsing/validation** with explicit HRP check | Wallets paste addresses from clipboards, QR codes, payment URIs. Decoding must reject wrong-HRP (e.g. an `xch1` address pasted into an SP-send field), wrong-checksum, and wrong-length payloads with distinct errors. | LOW — standard bech32m decode + length + HRP checks. | Belongs in `SilentPaymentAddress::decode`. Error variants should be expressive enough for wallets to render "wrong network" vs "malformed" vs "not a silent payment address". |
| **Mnemonic-based key derivation** (BIP-39 → `m/12381/8444/12/0` + `m/12381/8444/13/0`) | This is how every Chia wallet today derives keys. Anything that requires hex-loading defeats the wallet UX. | LOW — `bip39` and BLS derivation are already workspace deps; the CHIP fixes the paths. | Locked as ADDR-01. The prototype's hex-key-file path is a CLI ergonomic, not a wallet expectation. |
| **Unlabeled (base) address generation** | The headline use case: "publish one address, receive payments forever." | LOW — just `B_scan || B_spend` encoded as bech32m. | Locked as ADDR-02/ADDR-03 (`unlabeled_address()`). |
| **Send to a silent-payment address** producing an indistinguishable standard p2 coin | Core value prop. If the SDK can't compute the one-time puzzle hash, nothing else matters. | MEDIUM — ECDH (`compute_shared_secret`), tagged-hash tweaks, `input_hash`, multi-input aggregation, `StandardArgs::curry_tree_hash(synthetic_pk)`. All locked as SEND-01..04. | The hard parts are (a) the `ScalarField` unsigned-vs-signed reduction split called out in PROJECT.md CRYPTO-01 and (b) ensuring multi-input aggregation aborts when the wallet does not control all inputs. |
| **Transport-agnostic receive primitive** (`scan_from_tweaks(TweakData)`) | The wallet needs SOME way to detect incoming payments. Even before CHIP-0058 exists, wallets can adapt `sp-service` or run a local indexer and feed `TweakData` in. | MEDIUM — RECV-01..04. Single ECDH per spend group; loop output indices `k` until a gap; try registered labels on miss. | Locked. The transport-agnostic shape is what makes v1 forward-compatible with CHIP-0058. |
| **Multi-input aggregation correctness** | Wallets routinely spend multiple coins per transaction. If aggregation is wrong, the recipient can't detect the payment — and the failure is silent. | MEDIUM — SEND-03 (`aggregate_sender_sks`). | **The non-negotiable correctness gate:** if the wallet only partially controls inputs (e.g. offers, multi-party bundles), aggregation MUST fail loudly rather than silently fall back. PROJECT.md CONCERNS calls this out. |
| **Identity-element / zero-sum guard** on both send and scan paths | The CHIP §"Identity Element" specifies a defense-in-depth check. Without it, a malicious sender can construct a transaction whose shared secret is predictable. | LOW — single equality check against the BLS12-381 G1 identity (`0xc000...0`). | Edge case, but the CHIP explicitly mandates it. Belongs in the same module as the ECDH primitive. |
| **CHIP test vectors pass as unit tests** | Cross-implementation interop is the whole point of the CHIP. If TV1/TV3 don't pass, the wallet is silently incompatible with every other implementation. | LOW — test wiring, not new code. | Locked as CRYPTO-03. |
| **Bindings (napi / pyo3 / wasm)** for address, send, scan | Sage is TypeScript; many Chia wallets are Python or JS. A Rust-only v1 would gate adoption on a separate bindings phase. | MEDIUM — descriptor JSON + bindy macro. The send action's interaction with `Spends` is the trickiest binding surface. | Locked as BIND-01..03. The PROJECT.md "Bindings ship in v1" key decision pins this. |

### Differentiators (Competitive Advantage)

Features beyond the minimum. Each item is judged on a different axis (how common across Bitcoin BIP-352 wallets, complexity to add to the SDK, marginal user value).

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Labeled sub-addresses** (`labeled_address(m)`) | Recipients can distinguish payment streams ("donations" vs "invoices") from a single mnemonic. **Universal in production BIP-352 wallets** — Bitcoin Core, Cake, Sparrow, Silentium all support labels. Skipping creates an asymmetric API. | LOW — adds 32 bytes of state per registered label and one G1 add per `k` candidate during scan. | **Locked in v1.** PROJECT.md's "Labels fully supported in v1" key decision is consistent with prior art — this is closer to table-stakes than differentiator. Treating as a differentiator only because the *minimum* protocol works without it. |
| **Labeled detection during scan** | A wallet that hands out labeled addresses but can't detect payments to them is broken. Asymmetric APIs are footguns. | LOW — for each `k` candidate that misses unlabeled, try `P_k + label_pk` for each registered label. | Locked as RECV-04. |
| **Multi-output sends to the same recipient** (multiple `k` values per send) | Splits change/payment, lets one transaction satisfy multiple invoices, reduces transaction count. BIP-352 universal feature. | LOW — `k` is just an integer increment in the existing send loop. No new crypto. | Already implicit in the protocol; the action API just needs to accept `Vec<Recipient>` rather than a single recipient. **Recommend extending `SilentPaymentSend` to take multiple recipients at construction time** so multi-output is a first-class path, not an afterthought. |
| **Multi-input sends across the same derivation index** | Common case: wallet spends two coins at the same index. Pass 2a scanner grouping handles this natively. | LOW — falls out of `aggregate_sender_sks` + the existing signer. | Locked as SEND-03. |
| **Multi-input sends across different derivation indices** (with announcement linkage) | Less common, but real: wallet sweeps dust from multiple indices into one send. Requires opcode 60/61 announcement binding so the scanner's Pass 2b can reconstruct the group. | MEDIUM — the SDK's `SilentPaymentSend` must emit `CREATE_COIN_ANNOUNCEMENT` + `ASSERT_COIN_ANNOUNCEMENT` conditions binding all primary inputs together when there are multiple non-grouped inputs. | **Recommend including this in v1** because (a) it's a correctness property — without it, the receiver can't detect the payment — and (b) the alternative is documenting "don't use silent payments with cross-index sends," which is a much worse UX. |
| **Watch-only (scan-key-only) wallet support** | A core BIP-352 use case: the recipient's scan key can run on an always-online machine while the spend key stays in cold storage. Cake, Silentium, Bitcoin Core all support this. | LOW for the SDK — `scan_from_tweaks` already accepts `scan_sk` separately from `spend_sk`; the SDK just shouldn't require both. | **Recommend explicit support** — verify that `scan_from_tweaks` returns enough metadata (the tweak `t_k` and label index) that a *later* online operation with `spend_sk` can sign without re-deriving anything scan-side. Locked SDK surface already does this (`DetectedSpCoin.onetime_sk` requires `spend_sk`, but `t_k` + label are recorded). |
| **Change-output convention** (label `m=0`) | BIP-352 reserves `m=0` for the recipient's own change. Lets a wallet recover its change outputs from mnemonic alone, without needing a separate change-index registry. | LOW — same code path as a labeled address; just a reserved index. | **Worth surfacing as an explicit `change_address()` or `Label::Change` constant** so wallets don't accidentally hand out `m=0` to senders. The CHIP §"Labels for change" mandates that wallets never publish `m=0`. Add a doc-comment-level warning on `labeled_address` if `m=0` is passed. |
| **`DetectedSpCoin` carries enough metadata to re-derive the spend key** | A scanner is useful only if its output composes with the signer. Bitcoin Core's `listsilentpaymentaddresses` returns scan-time metadata; the same shape is needed here. | LOW — locked as RECV-02 (`onetime_sk`, `k`, optional `label`, coin metadata). | Verify that the returned struct also carries the parent coin info / output coin shape needed for the next-hop `CoinSpend`. |
| **Mnemonic OR raw-SK import** (split between high-level and low-level constructors) | Power users (hardware-wallet integration tests, restored wallets, key-import flows) want to construct `SilentPaymentKeys` from raw `scan_sk`/`spend_sk`, not just from mnemonic. | LOW — both `SilentPaymentKeys::from_mnemonic` and `SilentPaymentKeys::from_secret_keys` make sense. | **Recommend adding `from_secret_keys` to the v1 surface** even though it's not in the ADDR-01 requirement list — it's a 5-line constructor and unblocks scan-key-only watch wallets. |
| **Memos on `SilentPaymentSend`** | Chia memos are a routine feature of standard sends. The locked `SilentPaymentSend { recipient, amount, memos }` already accepts a memos field. | LOW for plumbing; see anti-features below for the design constraint. | **The CRITICAL constraint:** the SDK must NOT auto-add a hint memo (the inner puzzle hash) the way the standard wallet does for CATs. The whole privacy point of silent payments is that the one-time puzzle hash is computable only by the recipient — and a hint memo would let any indexer see "payment to puzzle hash X" without ECDH, which is the same as a public address. See anti-feature row "Auto-hint memos." User-supplied memos are fine but should carry a doc warning that on-chain memos are visible to all observers and partially defeat the privacy gain. |
| **Labels are scan-time inputs (no SDK-side persistence)** | Wallets, not the SDK, own the label registry. The SDK accepts `labels: &[LabelEntry]` at scan time. | LOW — already the locked shape. | The SDK is correctly **stateless**; this is a feature, not a missing one. Sage etc. persist label index → user name mapping. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem reasonable but are footguns. The v1 surface should deliberately not ship these.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **CAT2 silent-payment sends** | "We already support CAT2 normal sends. Why not silent-payment CAT2 sends?" The sender side is trivially possible — the CAT layer's `morph_condition` wraps an inner puzzle hash. | **The receiver can't detect them.** CAT2 outputs hide the inner puzzle hash inside the morphed CAT puzzle; extracting it requires layer-aware indexing that doesn't exist until CHIP-0058. Shipping CAT2 sends in v1 produces undetectable, unrecoverable payments. This is a maximum-severity footgun. | Locked as out-of-scope in PROJECT.md. Surface as `SilentPaymentSend` with an XCH-only marker; CAT2 send variant comes in v2 after CHIP-0058 indexer support exists. |
| **NFT silent-payment sends** | Symmetric request. | Singletons have lineage and metadata constraints that make sender construction high-complexity AND the privacy benefit is weaker (the `launcher_id` is permanent and trackable across spends, so unlinkability is partial at best). | Locked as out-of-scope. No clean alternative — deferred indefinitely. |
| **Hex-encoded silent-payment "addresses"** | The prototype prints them this way (see `crates/sp-common/src/keys.rs`, lines 95/107 — `bytes(scan_pk).hex()` etc). | (a) Doesn't follow CHIP §"Silent Payment Address" — the spec mandates bech32m with `spxch`/`tspxch` HRP. (b) Doesn't validate (no checksum). (c) Doesn't carry network info (mainnet vs testnet). (d) Misinterprets concatenated hex pubkeys as "address-like" when they're actually two separate pubkeys. Every Bitcoin BIP-352 implementation uses bech32m. | The CHIP-spec'd bech32m address is the only address format. Hex is fine for debug printing inside test vectors but not in the wallet API. |
| **Custom HRPs** (e.g. wallet-specific `sage_sp1...`) | Some wallets want to "brand" their addresses. | Breaks cross-wallet interop. The CHIP fixes `spxch`/`tspxch`. | None — refuse. Address parsing should reject any HRP outside the CHIP set. |
| **Auto-hint memos on silent-payment `CREATE_COIN`** | "Hints make coin discovery faster. The wallet always adds them on CATs and standard sends." | **Hints publish the inner puzzle hash to every indexer.** Silent payments' entire privacy model depends on the one-time puzzle hash being computable only by the scanning recipient. A hint memo turns the silent payment into a public payment with extra steps. | The recipient detects the coin via scan, not via puzzle-hash hint. **`SilentPaymentSend` must NOT auto-add a hint memo, and user-supplied memos must not silently get promoted to hints.** A hint = the first memo entry that's exactly 32 bytes; the action's serializer must either reject 32-byte memos or strip them from the hint-position with a warning. |
| **Recipient-facing memos** without privacy warning | Wallets that already support memos on standard sends will assume parity. | Memos are visible to every block observer, not just the recipient. Putting a clear-text invoice number or contact identifier on a silent payment is a strict downgrade from the privacy guarantee the user thinks they're getting. | Allow memos (parity is real), but: (a) document the privacy implication in the type's doc-comment, (b) consider a separate `private_memo` variant that does encrypted memos in a follow-up CHIP (out of scope here — flag for v2). |
| **Exposing `scan_sk` without a "this is high-value" affordance** | The scan key is technically lower-stakes than the spend key (compromise reveals detection capability, not funds). Easy to treat as a "watch-only" credential the user can casually share. | Compromising `scan_sk` exposes **every past and future payment to that address**. CHIP §"Scan Key as High-Value Secret" + the privacy-analysis.md doc both classify it as high-value. Wallets that label it "watch-only" alongside an `xch1` watch-only key (which only reveals one address's payments) will mislead users. | The SDK type names already separate `scan_sk` and `spend_sk`. **Recommend:** in bindings docs and any future serialization method, label scan-key exports with privacy implications. Don't add a `to_hex()` or `serialize()` method that would tempt CLI tools to print it without ceremony. |
| **Bulk-scanning the chain inside the SDK** | "Why does the consumer have to run an indexer? Can't the SDK iterate blocks?" | That's `sp-service`'s job. The SDK has no concept of a transport or a block reader. Embedding one couples the SDK to a specific (pre-CHIP-0058) wire format and forces every consumer to ship an indexer. | Locked as out-of-scope. The `TweakData` input type is the seam. |
| **`SilentPaymentTweakSource` async trait in v1** | "Wouldn't an async source be cleaner than a `&TweakData` borrow?" | Locks in an async runtime (`tokio` / `futures-util`) shape that CHIP-0058 may not match. With no real consumer in this milestone, the trait would be designed in a vacuum. | Locked as out-of-scope. Add when CHIP-0058 has a real transport client to design against. |
| **GCS block-filter prefilter** | A real bandwidth win for light clients per CHIP §"Light Client Support" Appendix A. | Not yet specified in any CHIP — the prototype's `sp-service` ships one but the format is implementation-defined. Shipping it now risks lock-in. | Defer until CHIP-0058 specifies the filter format. |
| **`puzzle_hash_for_pk` helper re-shipped in v1** | Mirrors the prototype's API. | Duplicates `chia_puzzle_types::standard::StandardArgs::curry_tree_hash(pk.derive_synthetic())` already in the SDK. Drift risk. | Locked as a key decision in PROJECT.md — drop the helper, reuse the existing one. |
| **Hardware-wallet scan/spend custody split as enforced workflow** | The CHIP §10 notes that `b_spend` cold + `b_scan` online is a recommended hardware-wallet pattern, and BIP-352 wallets like Coldcard support it. | Designing the *workflow* (how the SDK signs the next-hop spend when `spend_sk` is on a HW device) is a large feature in its own right and not tied to silent-payments cryptography per se. | The SDK already exposes `scan_sk` and `spend_sk` independently — that's all that's strictly needed. Hardware-wallet workflow is a Sage-level concern, not an SDK feature. Defer. |
| **Mempool scanning** | Detects incoming payments before block confirmation. Cake Wallet has this. | Same concern as bulk-scanning: requires a transport. Out of SDK scope. | Wallet-layer feature, fed by a CHIP-0058 client. |

## Feature Dependencies

```
[Bech32m address]
    └── enables ──> [Send to address]
                        ├── requires ──> [ScalarField unsigned mod-r]
                        ├── requires ──> [tagged_hash (BIP-340 style)]
                        ├── requires ──> [aggregate_sender_sks]
                        │                    └── requires ──> [single-party control assertion]
                        ├── requires ──> [input_hash]
                        ├── requires ──> [Identity-element guard]
                        └── requires ──> [SilentPaymentSend Spends action]
                                             └── enables ──> [Multi-output sends (k > 0)]

[Mnemonic key derivation]
    └── enables ──> [Unlabeled address]
                        └── enables ──> [Labeled address (m >= 1)]
                                             └── enables ──> [Labeled change (m = 0)]

[TweakData type]
    └── enables ──> [scan_from_tweaks]
                        ├── requires ──> [compute_shared_secret_from_tweak]
                        ├── requires ──> [derive_onetime_sk]
                        └── enables ──> [Labeled detection]
                                             └── requires ──> [label_pk → m registry (caller-provided)]

[Multi-input across different derivation indices]
    └── requires ──> [CREATE_COIN_ANNOUNCEMENT/ASSERT_COIN_ANNOUNCEMENT emission]
    └── requires ──> [Pass 2b scanner support (caller's indexer, not SDK)]

[Watch-only (scan-key-only) wallet]
    └── requires ──> [from_secret_keys constructor (or split key-pair construction)]
    └── enables ──> [Separate online-scan / cold-spend workflow at wallet layer]
```

### Dependency Notes

- **`SilentPaymentSend` requires `aggregate_sender_sks`** because the protocol's `input_hash` and shared secret are computed from the *aggregate* synthetic key, not per-input. A wallet that calls `aggregate_sender_sks` on a subset of the bundle's inputs produces undetectable payments — hence the single-party-control assertion (CONCERNS.md, PROJECT.md key decision).
- **Labeled detection requires the caller to maintain a label registry.** The SDK is intentionally stateless; the wallet passes `labels: &[LabelEntry { label_pk, m }]` at scan time. ADDR-04 calls out that wallets maintain this mapping.
- **Multi-input across different derivation indices enhances multi-input across the same index** — it's a strict superset, but more complex on both send (announcement emission) and scan (Pass 2b grouping). Send-side belongs in v1. Scan-side belongs in the wallet's indexer, not the SDK.
- **The `m = 0` change convention conflicts with `labeled_address(m)` if no guard exists.** A wallet author could accidentally hand out `m = 0` to senders, which lets anyone create payments that look like change. Treat `m = 0` as reserved at the API level — either a separate `change_address()` method or a doc-asserted warning + runtime check.

## MVP Definition

### Launch With (v1)

This matches the PROJECT.md "Active" requirements. Numbered for cross-reference.

- [x] ADDR-01..04 — `SilentPaymentKeys` (mnemonic-based), `SilentPaymentAddress` (bech32m), `unlabeled_address`, `labeled_address(m)`, label registry helper
- [x] SEND-01..04 — `derive_one_time_puzzle_hash`, `compute_input_hash`, `aggregate_sender_sks`, `SilentPaymentSend` Spends action
- [x] RECV-01..04 — `TweakData`, `scan_from_tweaks`, `compute_shared_secret_from_tweak`, labeled detection
- [x] CRYPTO-01..03 — `ScalarField` (unsigned mod-r), `tagged_hash` with `Chia_SP/*` tags, CHIP TV1+TV3 pass
- [x] BIND-01..03 — napi/pyo3/wasm bindings for address gen, send, scan
- [x] WS-01..03 + SIM-01..03 + EX-01 — feature flag, CI, simulator round-trip, example

**Recommended additions to v1 surface** (gaps the research surfaced):

- [ ] **`SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk)` constructor** — enables watch-only wallets and key-import flows. ~5 lines, no new crypto.
- [ ] **`Label::CHANGE` constant (= 0) + guard against handing it out** — either a `change_address()` accessor or a runtime check in `labeled_address(m)` that warns on `m=0`. Prevents the "wallet accidentally publishes change label" footgun called out in the CHIP.
- [ ] **`SilentPaymentSend` accepts `Vec<Recipient>` for multi-output sends** — multi-output is a first-class BIP-352 feature; making it implicit (one action per recipient) loses the property that all outputs share one `input_hash`. **Verify** the locked surface already covers this in the action shape; if not, extend.
- [ ] **Multi-input cross-index announcement binding** in `SilentPaymentSend` — emits opcode 60/61 conditions when the sender's inputs are at different derivation indices, so the recipient's Pass 2b scanner can reconstruct the group. Without this, cross-index sends are undetectable.
- [ ] **Doc-comment privacy warning on user-supplied memos** in `SilentPaymentSend` — memos are visible to all observers; users may not realize this defeats the privacy gain. No API change required, just docstring.
- [ ] **Memo-position guard** — if a user passes a 32-byte memo, it will be interpreted as a hint by the standard wallet. Either reject 32-byte memos in `SilentPaymentSend`, or insert a non-hint memo at position 0 to neutralize the hint slot. Without this guard, silent payments accidentally become public payments.

### Add After Validation (v1.x)

- [ ] **`SilentPaymentTweakSource` async trait** — once CHIP-0058's transport client lands and there's a real shape to design against.
- [ ] **CHIP-0058 transport client** — separate follow-up crate (likely `chia-sdk-silent-payments-client`), thin adapter from the WS protocol to `TweakData`.
- [ ] **GCS block-filter helpers** — once CHIP-0058 specifies the filter format.
- [ ] **CAT2 silent-payment sends** — only after CHIP-0058's indexer extracts inner puzzle hashes from CAT-wrapped outputs.
- [ ] **`SilentPaymentAddress::is_labeled()` or equivalent introspection** — the address bytes don't tell you if it's labeled (the `B_spend` byte position holds `B_spend + label_pk` for labeled addresses, indistinguishable from a plain `B_spend`). Skip in v1 unless a wallet asks for it; the recipient knows from their own label registry.

### Future Consideration (v2+)

- [ ] **NFT silent payments** — singleton constraints + permanent `launcher_id` make this both high-complexity and lower-privacy. May never make sense.
- [ ] **Encrypted recipient-facing memos** — out-of-band notification channel (CHIP §"Out-of-Band Notifications"). Requires its own CHIP.
- [ ] **Hardware-wallet split-custody workflow** — scan-online / spend-cold UX. Wallet-layer feature, possibly with SDK helpers later.
- [ ] **Mempool scanning** — wallet-layer + transport feature.
- [ ] **PayJoin / CoinJoin compatibility** — see `docs/coinjoin-analysis.md`: not practically feasible on Chia today due to `parent_coin_info` linkage. Would require a new puzzle.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Bech32m address (encode/decode/validate) | HIGH | LOW | P1 |
| Mnemonic-based key derivation | HIGH | LOW | P1 |
| Unlabeled address generation | HIGH | LOW | P1 |
| Send to address (single-input, single-output) | HIGH | MEDIUM | P1 |
| Multi-input aggregation (same index) | HIGH | LOW | P1 |
| Transport-agnostic `scan_from_tweaks` | HIGH | MEDIUM | P1 |
| Identity-element zero-sum guard | HIGH (correctness) | LOW | P1 |
| CHIP test vectors pass | HIGH (correctness/interop) | LOW | P1 |
| Bindings (napi/pyo3/wasm) | HIGH | MEDIUM | P1 |
| Labeled addresses + detection | HIGH | LOW | P1 |
| Multi-output sends (k > 0) | MEDIUM | LOW | P1 |
| Multi-input cross-index (announcement binding) | MEDIUM | MEDIUM | P1 |
| Change label (m=0) reserved guard | MEDIUM (footgun prevention) | LOW | P1 |
| `from_secret_keys` constructor | MEDIUM | LOW | P1 |
| Memo privacy warning + hint guard | MEDIUM (footgun prevention) | LOW | P1 |
| Watch-only (scan-key-only) explicit support | MEDIUM | LOW | P1 (free — falls out of correct API) |
| `SilentPaymentTweakSource` async trait | LOW (premature) | MEDIUM | P3 |
| GCS block-filter helpers | LOW (no CHIP yet) | MEDIUM | P3 |
| CAT2 silent-payment sends | HIGH (when possible) | HIGH (blocked by CHIP-0058) | P3 / v2 |
| NFT silent-payment sends | LOW (weak privacy) | HIGH | P3 / never |
| Hardware-wallet split workflow | MEDIUM | HIGH | P3 (wallet-layer) |
| Mempool scanning | MEDIUM | HIGH | P3 (wallet-layer) |
| Hex address format | NEGATIVE | LOW | NEVER |
| Custom HRPs | NEGATIVE | LOW | NEVER |
| Auto-hint memos on SP sends | NEGATIVE (breaks privacy) | LOW | NEVER |

**Priority key:**
- P1: Must have for v1 launch
- P2: Should have, add when possible (none identified in this milestone — everything is either P1 or deferred)
- P3: Defer to v1.x or v2+

## Competitor Feature Analysis

Comparing how Bitcoin BIP-352 wallets expose silent payments. The Chia SDK should match prior art where it makes sense and diverge only with reason.

| Feature | Bitcoin Core 28+ | Cake Wallet | Sparrow | Silentium | This SDK (v1) |
|---------|------------------|-------------|---------|-----------|---------------|
| Address format | bech32m `sp1q...` | bech32m | bech32m | bech32m | bech32m `spxch1...` (mainnet) / `tspxch1...` (testnet) |
| Sending support | YES | NO (receive-only) | YES | NO (receive-only) | YES (XCH; CAT2/NFT deferred) |
| Receiving support | YES | YES | YES | YES | YES (via TweakData) |
| Labeled addresses | YES (descriptors) | YES | YES | YES | YES |
| Change label (m=0) | YES (auto) | YES | YES | YES | YES (recommend: reserved + explicit) |
| Watch-only (scan-only) | YES | YES | YES (planned) | YES | YES (via `from_secret_keys`) |
| Scan from height | YES | YES (auto from wallet creation height) | YES | YES (server-fed) | Caller responsibility — SDK is stateless |
| Mnemonic import | YES (descriptor) | YES (BIP-39) | YES (BIP-39 + descriptor) | YES (BIP-39) | YES (BIP-39) |
| Raw-SK import | YES (descriptor private keys) | NO | YES (xprv) | LIMITED | YES (`from_secret_keys`) |
| Send memos | N/A (Bitcoin has no general-purpose memo field) | N/A | N/A | N/A | YES (with privacy warning + hint-position guard) |
| Light-client scanning | YES (BIP-158 filters) | YES (server-assisted) | YES (server-assisted) | YES (silentiumd server) | NO in v1 (wallet brings its own indexer / `TweakData` source) |
| Hardware-wallet support | BitBox / Coldcard / SeedSigner | YES (some) | YES (planned via BIP-375) | NO | NO in v1 (keys exposed separately so a wallet can build this) |
| Mempool scanning | NO | YES | NO | NO | NO in v1 (wallet-layer + transport) |
| Multi-output | YES | YES | YES | YES | YES |
| Multi-input | YES | YES | YES | YES | YES (same index in v1; cross-index requires announcement binding) |

**Patterns the SDK should match:**
1. **Bech32m address with chain-specific HRP.** Universal across Bitcoin BIP-352 wallets. The CHIP already specifies this. (Locked.)
2. **Label index as a small integer with `m=0` reserved for change.** Universal. (Locked.)
3. **Scan and spend keys exposed as separate fields/types.** Enables watch-only and HW-split workflows. (Locked.)
4. **Stateless scan primitive that accepts label-registry input** rather than persisting labels in the SDK. The locked `scan_from_tweaks(scan_sk, spend_sk, spend_pk, &TweakData, labels)` shape matches this.
5. **Mnemonic-first key import, with raw-SK escape hatch.** Universal among Bitcoin wallets that support BIP-352. **Recommend adding `from_secret_keys`** to match.

**Patterns the SDK should NOT match (because of Chia-specific constraints):**
1. **Light-client scanning helpers in v1.** Bitcoin's BIP-158 filters are standardized; Chia's are not (CHIP-0058 hasn't shipped). Premature here.
2. **Bitcoin Core's descriptor-based label persistence.** Chia wallets don't use descriptors. Stateless API is correct.

**Patterns where Chia has features Bitcoin doesn't, requiring careful handling:**
1. **Memos on `CREATE_COIN`.** Bitcoin has no equivalent. The privacy implication (memos are public) needs explicit documentation. The hint mechanism (first 32-byte memo = puzzle-hash index) needs an explicit guard so silent-payment sends don't accidentally publish a puzzle-hash hint.

## Sources

- [PROJECT.md](file:///home/kdc/chia-wallet-sdk/.planning/PROJECT.md) — locked v1 scope (HIGH confidence — this is authoritative)
- [chip-silent-payments.md](file:///home/kdc/silent-payments/chip-silent-payments.md) — full CHIP spec (HIGH — authoritative protocol source)
- [silent-payments/README.md](file:///home/kdc/silent-payments/README.md) — prototype features and CLI shape (HIGH — reference implementation)
- [docs/privacy-analysis.md](file:///home/kdc/silent-payments/docs/privacy-analysis.md) — scan-key sensitivity, memo implications (HIGH — internal analysis)
- [docs/coinjoin-analysis.md](file:///home/kdc/silent-payments/docs/coinjoin-analysis.md) — why CoinJoin/PayJoin doesn't help on Chia (HIGH — internal analysis)
- [BIP-352 spec](https://github.com/bitcoin/bips/blob/master/bip-0352.mediawiki) — protocol source for cross-checking adaptation choices (HIGH)
- [Silent Payments Wallet Support](https://silentpayments.xyz/docs/wallets/) — wallet implementation matrix (MEDIUM — community-maintained, may lag actual status)
- [Cake Wallet docs — Bitcoin](https://docs.cakewallet.com/cryptos/bitcoin) — Cake's silent-payments UX (MEDIUM — vendor docs)
- [Cake Wallet blog — silent payments fixes](https://blog.cakewallet.com/cake-wallet-launches-sleek-ui-improvements-bitcoin-silent-payments-fixes-and-more/) — scan-height behavior (MEDIUM — vendor blog)
- [Silentium overview (nobsbitcoin.com)](https://www.nobsbitcoin.com/silentium-silent-payments/) — light-wallet PoC features (MEDIUM — secondary source)
- [BIP-352 Wallet Implementation Recommendations gist (macgyver13)](https://gist.github.com/macgyver13/d78cf7c5496a60ffbf6562fec09f9bd0) — community recommendations (LOW-MEDIUM — gist, single author)
- [How Silent Payments Work — Medium (otto)](https://medium.com/@ottosch/how-silent-payments-work-41bea907d6b0) — protocol walkthrough including address parsing (MEDIUM)
- [Bitcoin Optech — Silent Payments topic](https://bitcoinops.org/en/topics/silent-payments/) — implementation status tracking (MEDIUM — community-curated)
- [BIP-352 Light Client Spec (setavenger)](https://github.com/setavenger/BIP0352-light-client-specification) — GCS-filter-based scanning (MEDIUM — draft community spec)
- [Chia docs — Wallet Protocol](https://docs.chia.net/wallet-protocol/) — hint/memo mechanics (HIGH — official docs)
- [The power of memos in Chia (Medium)](https://medium.com/@osielquevedo/the-power-of-memos-in-the-coins-model-of-chia-blockchain-english-traslated-9963c6cc5697) — hint-position semantics (MEDIUM)

---
*Feature research for: CHIP-0057 silent-payments wallet SDK integration*
*Researched: 2026-05-15*
