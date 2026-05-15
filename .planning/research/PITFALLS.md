# Pitfalls Research — CHIP-0057 Silent Payments (Wallet-Side)

**Domain:** BIP-352-style silent payments on BLS12-381 (Chia adaptation)
**Researched:** 2026-05-15
**Confidence:** HIGH for all crypto-correctness pitfalls (verified against `~/silent-payments` prototype, CHIP draft §§ 4-11, and `.planning/codebase/CONCERNS.md`); MEDIUM for the privacy / forward-compat items (verified against CHIP §10-11 and the privacy-analysis doc, but no production deployments exist).

This document is scoped to **what the wallet SDK can get wrong**. Out-of-scope pitfalls (light-client transport, full-node scan privacy, hardware-wallet custody UX, CAT2 receive detection) are listed at the end with a brief "not our problem in v1" disposition so the roadmap doesn't accidentally schedule them.

Each pitfall is categorized:
- **CORRECTNESS** — silently produces wrong output / undetectable payment / invalid signature.
- **PRIVACY** — protocol works, but observers learn more than they should.
- **API DESIGN** — wallet author misuses the SDK; correctness emerges only with the right call shape.
- **FORWARD-COMPAT** — bakes in a decision that CHIP-0058 (or v2) will break.

---

## Critical Pitfalls

### Pitfall 1: Signed-vs-unsigned scalar reduction mixing

**Category:** CORRECTNESS

**What goes wrong:**
The CHIP requires UNSIGNED reduction mod r for three protocol scalars:
- `input_hash = int(tagged_hash("Chia_SP/Inputs", coin_id_L || serialize(A_sum))) mod r`
- `t_k = int(tagged_hash("Chia_SP/SharedSecret", shared_secret || ser32(k))) mod r`
- `label_scalar = int(tagged_hash("Chia_SP/Label", ser256(b_scan) || ser32(m))) mod r`

The standard puzzle's *synthetic offset* uses SIGNED reduction (`int.from_bytes(..., signed=True) mod r`) for backward compatibility with the original Chia wallet (`calculate_synthetic_offset`). If a developer reaches for the existing `mod_by_group_order` helper (signed) inside silent-payments code, the result is silently wrong: the high-bit-set ~50% of digests get reduced to a different scalar than the receiver computes.

Concrete failure: sender computes one one-time puzzle hash `P_k`, receiver computes a different `P_k` from the same shared secret, the on-chain output **does not match** the scanner's candidate, and the payment is **undetectable forever**. No error is raised — the wallet UI just never shows the incoming coin.

**Why it happens:**
1. The existing SDK / `chia-bls` ecosystem already has a `mod_by_group_order` (signed). It's the obvious-looking helper.
2. The CHIP § "Synthetic Key Computation Note" explicitly says the offset uses signed reduction. Developers reading the spec top-to-bottom encounter signed-reduction *first*, then forget the protocol scalars are unsigned.
3. There is no test that *deliberately* uses a high-bit-set digest; the TV1/TV3/TV4 vectors all happen to produce input_hash values whose high bit is 0, so a signed-vs-unsigned bug passes the canonical vectors.

**How to avoid (prevention strategy):**
Make the type system enforce it. The reference prototype already shows the pattern (`crates/sp-common/src/scalar.rs`):

```rust
/// Big-endian 32-byte scalar with UNSIGNED mod-r reduction.
/// Only constructor that reduces is `from_bytes_unsigned`.
/// `from_bytes_raw` is for values already known to be in [0, r) (e.g. SK bytes).
pub struct ScalarField([u8; 32]);

impl ScalarField {
    pub fn from_bytes_unsigned(bytes: [u8; 32]) -> Self { /* BigUint % r */ }
    pub fn from_bytes_raw(bytes: [u8; 32]) -> Self { /* no reduction */ }
}
```

The SDK port must:
1. Define `ScalarField` as the **only** path to mod-r reduction inside `#[cfg(feature = "chip-0057")]` code. No `pub use` of `mod_by_group_order` from `chia-bls` should be visible in silent-payments modules.
2. Make all three protocol scalars (`input_hash`, `t_k`, `label_scalar`) return `ScalarField`, not `[u8; 32]`. The compiler then prevents accidentally substituting a signed-reduced scalar.
3. Write a deliberately-adversarial unit test: feed `tagged_hash` an input whose first byte is `>= 0x80` and verify the reduction matches the prototype's `BigUint::from_bytes_be(&bytes) % r`. The TV1/TV3 vectors do not catch this.

**Warning signs:**
- A PR adds `use chia_bls::mod_by_group_order` inside `chip-0057` code.
- A PR adds `from_bytes_be_signed` or any function with `signed=True` to a silent-payments helper.
- A test vector marked "round-trip" passes but a randomized property test fails.
- `cargo grep -E 'signed.*=.*true' crates/chia-sdk-*/src/silent_payments*` returns hits outside the synthetic-offset call site.

**Phase to address:**
Phase 1 (`CRYPTO-01` requirement). The `ScalarField` newtype must land before any of `input_hash`, `t_k`, `label_scalar` are computed in real code. Sequencing it later means rewriting every protocol function.

**Verification:**
Property test in CI: for 1000 random 32-byte inputs, `ScalarField::from_bytes_unsigned(bytes).as_bytes()` must equal `BigUint::from_bytes_be(&bytes) % r` (big-endian-padded to 32). Additionally, regression test: input `[0xff; 32]` (which has high bit set) must NOT equal `[0xff; 32]` after reduction.

---

### Pitfall 2: Synthetic vs raw key confusion in the send-side API

**Category:** API DESIGN → CORRECTNESS

**What goes wrong:**
BIP-352 says "sender_sk." The CHIP says *sender_sks* are the **synthetic** secret keys — the ones that, when multiplied by G, produce the public key curried into the spent coin's `p2_delegated_puzzle_or_hidden_puzzle`. If the SDK accepts `Vec<SecretKey>` and a wallet author passes the **raw** wallet SKs (from `master_to_wallet_sk(master, index)`), every downstream computation is wrong:

- `a_sum` is the sum of raw SKs, not synthetic SKs.
- The sender's `A_sum = a_sum · G` does **not** match what the receiver extracts from the puzzle reveal (which contains the *synthetic* PK).
- `input_hash = H(coin_id_L || serialize(A_sum))` differs between sender and receiver.
- Payment is undetectable.

Worse, the *signature* over the spend may still validate (because the synthetic SK is computed elsewhere in the signer path), so the bundle goes on-chain successfully. The receiver simply never sees it.

**Why it happens:**
1. `chia-bls::SecretKey` is one type; there's no compile-time distinction between raw and synthetic.
2. Chia developers who haven't internalized `p2_delegated_puzzle_or_hidden_puzzle` think "SecretKey is SecretKey."
3. `StandardLayer::new(synthetic_key)` (line 19, `standard_layer.rs`) already takes a synthetic key, so the SDK's own API is consistent — but a new `aggregate_sender_sks(&[SecretKey])` does not force the caller to thread synthetic SKs through.

**How to avoid:**
Two options, in decreasing order of safety:

**Option A (recommended):** Type-level distinction.
```rust
/// A synthetic secret key — the SK whose PK is curried into a p2 puzzle reveal.
/// Construct via `SyntheticSecretKey::from_raw(raw_sk, hidden_puzzle_hash)`
/// or `SyntheticSecretKey::from_wallet_sk(wallet_sk)` (which derives the offset).
#[derive(Clone)]
pub struct SyntheticSecretKey(SecretKey);

pub fn aggregate_sender_sks(sks: &[SyntheticSecretKey]) -> ScalarField { ... }
```
Wallet author cannot pass a raw `SecretKey` by accident — they get a compile error.

**Option B:** If type pollution is unacceptable, name the parameter `synthetic_sks: &[SecretKey]` and put a `#[doc = "..."]` block-warning, plus a debug-assert that `synthetic_sks[i].public_key()` matches the synthetic PK extracted from the corresponding coin's puzzle reveal (when the coin is available). This catches the bug at test time but not at compile time.

The CHIP itself supports Option A: the spec consistently writes `a_syn` / `A_syn` to mark this.

**Warning signs:**
- `aggregate_sender_sks(&self.wallet.unhardened_sks)` — direct pass of wallet keys.
- A test that does `let sk = master_to_wallet_sk(master, 0); send_silent_payment(sk, ...)` without calling `derive_synthetic()` or going through the signer.
- TV4 (multi-input) passes but a simulator-round-trip test fails.

**Phase to address:**
Phase 2 (`SEND-03` requirement). The `aggregate_sender_sks` signature is the API surface; locking it down before `SilentPaymentSend` action exists is cheaper than retrofitting.

**Verification:**
Simulator test (`SIM-02`) is the canonical check — it exercises the actual signer path that produces synthetic keys. A unit test alone (with handcrafted scalars) can't catch this because the test author can just pass the right thing.

---

### Pitfall 3: Multi-party aggregation silently uses only the wallet's own inputs

**Category:** CORRECTNESS

**What goes wrong:**
`aggregate_sender_sks` sums the keys it's given. If a spend bundle has 3 inputs — 2 from this wallet, 1 from a counterparty (offer settlement, multi-party CoinSet bundle) — and the SDK calls `aggregate_sender_sks(&my_keys_only)`, then:

- The SDK computes `A_my = a_1 + a_2`, derives `input_hash = H(coin_id_L || A_my)`, and constructs the one-time puzzle hash from there.
- The recipient's scanner sees all 3 spends on-chain, extracts all 3 synthetic PKs, sums them to `A_all = A_1 + A_2 + A_3`, computes `H(coin_id_L || A_all)` — which is **different** from `A_my`.
- Shared secret differs. One-time puzzle hash differs. Payment is undetectable.

This is documented in `.planning/codebase/CONCERNS.md` ("Multi-Input Aggregation Risk") and CHIP §"Using all inputs". It's the most likely production failure mode because **offers are common on Chia** and **the SDK's signer path already produces synthetic SKs for the local wallet only**.

**Why it happens:**
1. The SDK's mental model is "I sign for my coins, the counterparty signs for theirs." For BLS signature aggregation that's correct — the aggregated signature is constructed from individual signatures.
2. For silent-payment ECDH, "my a + their a" is **not** something either party can compute alone (each party only knows their own scalar). The aggregation must happen at sign time, with both parties' synthetic public keys (point addition, not scalar addition).
3. The single-wallet path (where `aggregate_sender_sks` does work) is the happy case all tests cover. Multi-party is invisible until offers ship.

**How to avoid:**

1. **Detect external inputs and refuse.** When building a `SilentPaymentSend` action, the SDK already knows which inputs the local signer can sign for (because `Spends` tracks them). If `tx.inputs.len() != local_synthetic_sks.len()`, the SDK must either:
   - Refuse with a hard error: `Error::MultiPartyAggregationRequired { external_input_count: usize }`.
   - Accept an explicit `external_synthetic_pks: &[PublicKey]` parameter so the receiver-side `A_sum` can be computed from points (which both parties can publish), even when scalars aren't shareable.

2. **Document the math.** The aggregated input_hash needs `A_sum = a_local · G + A_external_pks_summed`. The local party can compute `A_local = a_local · G` and add the external PKs (which the offer payload includes). So `input_hash` is computable by either party from public information. The puzzle hash derivation needs `a_local · input_hash · B_scan + A_external_total · input_hash · B_scan ?` — actually no, see next paragraph.

   **Wait — this is harder than offers.** The shared secret `S = (input_hash · a_sum) · B_scan` requires the **scalar** `a_sum = a_local + a_external`. Neither party alone can compute it. The only sound approaches are:
   - Single-party aggregation only (refuse multi-party with hard error). This is what v1 should ship.
   - A multi-party protocol where parties jointly compute the shared secret via a sum-of-ECDH-points trick: `S = input_hash · a_local · B_scan + input_hash · a_external · B_scan`. Each party computes their term and shares the **point** (not scalar) with the sender-of-record. This is feasible but is its own multi-round protocol.
   - Wrap the silent-payment payment inside a "regular" output that one party (the payment originator) signs for fully, and have the multi-party flow just be a normal mixed-input spend bundle where one of the inputs is the payment originator's. This sidesteps the aggregation problem.

3. **For v1: ship a hard error.** A `MultiPartyAggregator` API is a v2 concern.

**Warning signs:**
- `aggregate_sender_sks(&self.local_keys)` called from inside `Spends::sign()` without checking `tx.input_count`.
- An integration test where the spend bundle has inputs from two wallets and the silent-payment recipient detects the coin. (If this passes, something is wrong — either the test is single-party and mislabeled, or aggregation is silently buggy.)
- A user report: "I sent through Sage's offer-settlement flow and the recipient never saw it."

**Phase to address:**
Phase 2 (`SEND-03`). The hard-error path must be in v1; multi-party `MultiPartyAggregator` is explicitly out-of-scope per `PROJECT.md` Key Decisions.

**Verification:**
Unit test: construct a fake `Spends` action system with 2 inputs, register 1 synthetic SK, call the silent-payment driver. Assert it returns `Err(MultiPartyAggregationRequired { external_input_count: 1 })` (or whatever the error variant ends up named). Adversarial test: confirm it does NOT silently aggregate the 1 SK it has.

---

### Pitfall 4: Tagged-hash domain-tag typos

**Category:** CORRECTNESS

**What goes wrong:**
The CHIP defines three domain tags as exact strings:
- `"Chia_SP/Inputs"` — input hash
- `"Chia_SP/SharedSecret"` — output tweak
- `"Chia_SP/Label"` — label scalar

A typo (`Chia-SP/Inputs`, `chia_sp/inputs`, `Chia_SP/Input`, `Chia_SP/SharedSecrets`) silently produces a completely different hash. Sender derives `P_k` with the wrong tag; receiver computes the right `P_k`; no match; undetectable payment.

Worse, the prototype's `tagged_hash` already shows the failure mode: SHA256("Chia_SP/Inputs") vs SHA256("Chia-SP/Inputs") differ in every bit. If one of the two SDK files spells it `Chia-SP/Inputs` and the other spells it `Chia_SP/Inputs`, internal tests (sender + scanner in the same module) pass — but cross-implementation interop with the Python prototype (or any future implementation) fails.

**Why it happens:**
1. Magic strings are inherently typo-prone.
2. Internal-only tests build the same wrong tag for both sender and scanner, so the test passes.
3. Refactoring (renaming `Chia_SP` to `ChiaSP`) without grep-ing all usages.

**How to avoid:**
1. **Single constant per tag.** Define once, reference everywhere:
   ```rust
   pub const TAG_INPUTS:        &str = "Chia_SP/Inputs";
   pub const TAG_SHARED_SECRET: &str = "Chia_SP/SharedSecret";
   pub const TAG_LABEL:         &str = "Chia_SP/Label";
   ```
   No code outside this constants module should pass a string literal to `tagged_hash`. A clippy rule (`disallowed_methods` for `tagged_hash` called with a non-const argument) is overkill but possible; a code-review checklist is sufficient.

2. **Cross-implementation test vectors.** TV1, TV3, TV4 from the CHIP encode the exact tag values implicitly (any change to the tag changes `input_hash`, `t_0`, `label_scalar` — all of which are in the test vectors). Running the CHIP vectors as Rust unit tests catches any tag mismatch instantly.

3. **Pin SHA256(tag) constants in a unit test.** As a defense-in-depth, hard-code the expected `SHA256("Chia_SP/Inputs")` value and assert it. If someone changes the constant, the SHA256 assertion fails at compile-time test, before any cryptographic computation.

**Warning signs:**
- Multiple distinct string literals across the file (`"Chia_SP/Inputs"` vs `"Chia_SP/inputs"`).
- Tag passed as a function parameter from non-constant source.
- TV1 fails after a "harmless rename" PR.

**Phase to address:**
Phase 1 (`CRYPTO-02` requirement). The constants module must land alongside `tagged_hash` itself.

**Verification:**
The CHIP test vectors (TV1, TV3, TV4) are sufficient. Adding the `SHA256(tag)` pin is belt-and-suspenders.

---

### Pitfall 5: Coin-ID lexicographic ordering — endianness / byte-string semantics

**Category:** CORRECTNESS

**What goes wrong:**
The CHIP specifies `coin_id_L = min(coin_ids)` where `min` is "lexicographic comparison as raw bytes." The coin ID is a 32-byte SHA-256 digest. Lexicographic byte-string min is well-defined and is what the prototype implements (`coin_ids.iter().min()` in Rust gives `[u8; 32]` byte-wise ordering, identical to Python's `min(bytes_list)`).

Two ways this can go subtly wrong:

1. **Hex-string comparison.** If anyone converts coin IDs to hex strings (`hex::encode(coin_id)`) before comparison, the order is **the same** (hex preserves lex order), but if one converter uppercases and another lowercases, the comparison is broken.

2. **Integer-with-endianness comparison.** If a developer reads "lexicographic" as "treat as big-endian integer and pick the smallest," that happens to match byte-string lex order — but if they read it as "treat as little-endian integer," it differs. The CHIP is explicit: byte-string lex. Reference implementation: `coin_ids.iter().min()` (Rust `[u8; 32]` derives `Ord` byte-wise from MSB).

3. **Sorting then `[0]` vs `min`.** Both are correct; only matters if a future refactor breaks the equivalence.

Sender computes one `coin_id_L`, receiver computes another → different `input_hash` → undetectable.

**Why it happens:**
1. "Lexicographic" is ambiguous in conversational English. The CHIP says "as raw bytes" but that qualifier can be dropped during translation.
2. Cross-language ports: Python's `min(bytes_list)` and Rust's `iter().min()` on `&[u8; 32]` are both byte-wise; JavaScript's `Math.min` on hex strings (`Math.min("0x01...", "0x02...")` returns `NaN`!) is not. Anyone porting to JS without using `Buffer.compare` will break it.
3. Multi-input is rare in unit tests; the bug hides until production multi-input flows.

**How to avoid:**
1. **Single helper.** A `fn smallest_coin_id(coin_ids: &[Bytes32]) -> Bytes32` that does `coin_ids.iter().min().copied().expect("coin_ids must not be empty")`. No code outside this helper does the min.
2. **TV4 test vector** explicitly exercises this: coin_id_0 = `2b9857e0...`, coin_id_1 = `209bb03a...`, expected `coin_id_L = coin_id_1` (`209bb03a... < 2b9857e0...` byte-wise). Running TV4 catches a backwards comparator.
3. **Property test:** Generate 100 random pairs of coin IDs, assert `smallest_coin_id(&[a, b]) == smallest_coin_id(&[b, a])` (order independence) and that the result is one of the two inputs.
4. **JavaScript bindings:** When the bindings layer surfaces this, ensure napi/wasm receive `Bytes32` (Uint8Array) and not hex strings.

**Warning signs:**
- Hex-string conversion inside the input-hash path.
- `coin_ids.sort_by(...)` with a custom comparator.
- A bindings test that calls `compute_input_hash` with hex strings.

**Phase to address:**
Phase 2 (`SEND-02` requirement: `compute_input_hash`).

**Verification:**
TV4 + the property test above.

---

### Pitfall 6: Endianness of `ser32(k)` and `ser256(b_scan)`

**Category:** CORRECTNESS

**What goes wrong:**
The CHIP defines:
- `ser32(k)` = 4-byte **big-endian** of a 32-bit integer.
- `ser256(x)` = 32-byte **big-endian** of a 256-bit scalar.

Both feed into tagged hashes (`t_k` uses `ser32(k)`; `label_scalar` uses `ser256(b_scan)`). Little-endian encoding gives a different hash, different scalar, different one-time PK. Undetectable.

The Rust prototype uses `k.to_be_bytes()` (correct). The Python prototype uses `int.to_bytes(4, "big")` (correct). `chia-protocol` types serialize integers little-endian by default in many places (clvm encoding uses CLVM-int convention). A developer accustomed to CLVM int encoding might call `int_to_bytes(k)` or similar and silently produce LE bytes.

**Why it happens:**
1. CLVM's `int_to_bytes` uses a *minimal-byte signed-int* encoding (`int_to_bytes(0) == b""`, `int_to_bytes(1) == b"\x01"`, etc.) — neither little-endian nor fixed-width big-endian. A developer reaching for `int_to_bytes` instead of `to_be_bytes` produces yet a third encoding.
2. The compiler doesn't distinguish `[u8; 4]` LE-of-k from `[u8; 4]` BE-of-k.
3. `chia-bls::SecretKey::to_bytes` is documented as big-endian, which is correct — but a developer might convert through `BigUint::to_bytes_le` for some intermediate operation.

**How to avoid:**
1. **Helper functions:** `fn ser32(k: u32) -> [u8; 4] { k.to_be_bytes() }` and `fn ser256(scan_sk: &SecretKey) -> [u8; 32] { scan_sk.to_bytes() /* documented big-endian */ }`. All call sites go through these.
2. **TV3 test vector** locks in `ser256(b_scan)` encoding (the label_scalar hex is sensitive to it).
3. **TV1 / TV3 / TV4 all have `t_0` values that depend on `ser32(0)` being `[0, 0, 0, 0]` (which is endianness-agnostic — it's all zeros!).** This is a **gotcha** in the test vectors: `k = 0` does not distinguish BE from LE. We need at least one test that uses `k = 1` or higher to catch endianness errors. A multi-output simulator test (sending to the same scan key twice) exercises `k = 1` naturally.

**Warning signs:**
- `int_to_bytes` (CLVM helper) used inside silent-payments code.
- `k.to_le_bytes()` anywhere.
- All test vectors use `k = 0`.

**Phase to address:**
Phase 2 (output tweak derivation) and Phase 3 (labels).

**Verification:**
A custom test vector with `k = 1`: compute `t_1` for TV1 keys, lock the expected value, and verify. Without this, TV1/TV3/TV4 don't catch ser32 endianness.

---

### Pitfall 7: `k` iteration termination — the labeled-output corner case

**Category:** CORRECTNESS

**What goes wrong:**
BIP-352 says: iterate `k = 0, 1, 2, ...` until no match, then stop. The CHIP follows this. The naive implementation:

```rust
let mut k = 0;
loop {
    let candidate = derive_onetime_pk(spend_pk, &derive_output_tweak(&shared_secret, k));
    if !output_phs.contains(&puzzle_hash_for_pk(&candidate)) { break; }
    detected.push(...);
    k += 1;
}
```

is **wrong** when labels are involved. At `k = 3`, the unlabeled puzzle hash might not match, but the **labeled** version (`labeled_pk = candidate + label_pk_m`) at `k = 3` might. The correct stop condition is: "neither unlabeled NOR any labeled candidate matches at this `k`." The prototype `scanner.rs` gets this right:

```rust
// At each k:
//   1. Try unlabeled. If match, record + continue to k+1.
//   2. Else try each labeled. If any match, record + continue to k+1.
//   3. Else break.
```

A wallet that stops at the first unlabeled miss will silently skip labeled payments at higher `k`. The most common failure: a wallet sends to recipient's *labeled* address, then sends to the same recipient's *unlabeled* address in a future block — scanner detects neither because it broke at `k=0` on the unlabeled miss.

The **opposite** failure: never breaking (off-by-one in the other direction) means the scanner iterates to `K_max = 2400` for every spend group in every block, blowing up scan time by 100-1000x.

**Why it happens:**
1. BIP-352's pseudocode is unlabeled-only; the labeled extension is a separate section. Developers implementing unlabeled-first and then bolting on labels often forget to re-think the termination rule.
2. Unit tests for labels typically use `k = 0` only, so the off-by-one is invisible.
3. The "break on no match" rule is easy to read as "break after the loop body" rather than "break when nothing at all matched."

**How to avoid:**
1. **Mirror the prototype's structure** (`sp-client/src/scanner.rs:74-135`): a single `loop` with an explicit `found` flag, only breaking when the flag is false after both unlabeled and labeled paths.
2. **K_max guard:** The CHIP § "K_max" recommends a hard cap. Implement `K_max = 2400` (matching BIP-352's analogous bound and the CHIP's derivation). Without this guard, a malicious or buggy tweak-data producer can force unbounded iteration.
3. **Test vector for labeled-at-k=1:** Construct a synthetic test where the recipient's labeled-address payment lands at `k=1` (preceded by an unlabeled payment at `k=0` to the same scan key). The scanner must detect both. Without this test, every labeled implementation has the termination bug.

**Warning signs:**
- `if let Some(match) = output_phs.iter().find(...) { ... } else { break }` — breaks before trying labels.
- No `K_max` constant in the scanner.
- The labeled detection test only uses `k=0`.

**Phase to address:**
Phase 3 (`RECV-04` — labeled detection). The unlabeled-only scanner from Phase 2 is structurally fine; the labeled extension is where this bug bites.

**Verification:**
1. Synthetic test: TV1-like setup with two outputs in one spend group, second output labeled at `k=1`. Scanner must detect both.
2. `K_max` upper bound: malicious tweak-data with a fabricated shared secret that produces 10000 consecutive "matches" (forged outputs) — scanner must stop at 2400.

---

### Pitfall 8: Bech32m HRP / checksum mistakes

**Category:** CORRECTNESS (parsing failure, not silent corruption)

**What goes wrong:**
Three failure modes for the `spxch` / `tspxch` address encoding:

1. **Wrong HRP.** Decoder accepts `xch` (standard wallet address) as if it were a silent-payment address, attempts to parse the 32-byte payload as a 96-byte `scan_pk || spend_pk`, fails or worse — returns garbage. Symmetric: encoder uses `xch` instead of `spxch` and the address is unrecognizable.
2. **Wrong checksum variant.** Bech32 (BIP-173) and bech32m (BIP-350) differ in the checksum polynomial constant. A library defaulting to bech32 silently produces invalid-looking-but-recoverable addresses; users paste them into wallets and lose funds (well, in this case, they paste them and the wallet refuses, which is *better* than the silent case — but still a UX disaster).
3. **HRP case sensitivity.** Bech32m HRPs are lowercase by convention. Mixed-case input (`SPXCH1...`) decodes to the same address — but if the address is *displayed* in mixed case, the QR-code-scanner-decoded version may differ.

This is a low-severity correctness pitfall because failures are noisy (parse errors), not silent (the payment-undetectable failures elsewhere are worse). But it blocks all downstream features.

**Why it happens:**
1. The default `bech32` Rust crate offers both `bech32` and `bech32m` variants. Picking the wrong one is one constant.
2. HRP is a string literal — see Pitfall 4 (typos).
3. The 96-byte payload is unusual (standard Chia addresses are 32 bytes); 5-bit-group encoding produces a non-obvious length, and length-mismatch failures may be misdiagnosed.

**How to avoid:**
1. **CHIP test vectors include encoded addresses** (TV1: `txch1ywkm59yamyqq6e0qlr3pk6t4xextazdx8jh4v5ea7jmkvnppl06swd5m0t` — but this is the *one-time* address, not the silent-payment address). The CHIP doesn't include explicit `spxch1...` test vectors. **Generate fresh ones from the reference implementation and pin them.**
2. **Use `chia-sdk-utils::Bech32`** if it already supports bech32m. (Verify: the standard `xch` address uses bech32m per BIP-350.) Reuse the same code path with HRP `spxch` / `tspxch`.
3. **Round-trip property test:** For 1000 random `(B_scan, B_spend)` pairs, `decode(encode(addr)) == addr` must hold. Additionally, `encode` must produce a string starting with `spxch1` (mainnet) or `tspxch1` (testnet).
4. **Negative tests:** Decoding `xch1...` (wrong HRP), decoding a bech32-not-bech32m string, decoding mixed-case input, decoding truncated input — all must return `Err`, not garbage.

**Warning signs:**
- HRP constant in two places.
- `Bech32::Encoding::Bech32` instead of `Bech32m`.
- No round-trip test.

**Phase to address:**
Phase 1 (`ADDR-02` requirement).

**Verification:**
Round-trip + negative tests above.

---

### Pitfall 9: Generic Rust / chia-bls API gotchas

**Category:** CORRECTNESS

**What goes wrong:**
Several `chia-bls` API shapes are easy to misuse:

1. **`PublicKey::scalar_multiply` mutates in place.** It does not return a new point. Code that does:
   ```rust
   let result = pk.scalar_multiply(&scalar_bytes);
   ```
   binds `result` to `()` and continues using the now-mutated `pk` — usually wrong. The correct pattern is:
   ```rust
   let mut tweak_point = *pk;
   tweak_point.scalar_multiply(&scalar);
   // use tweak_point
   ```
   The prototype's `compute_shared_secret_from_tweak` does this correctly. Any new code must mirror it.

2. **`PublicKey::from_bytes` returns `Result`.** Invalid 48-byte sequences (not on the curve, wrong subgroup) return `Err`. Code that uses `.expect()` panics on attacker-controlled input. Scanner inputs come from `TweakData`, which may be attacker-controlled (a malicious tweak-data producer can submit invalid points). Use `?` or skip with a log.

3. **`SecretKey::from_bytes` rejects zero and values `>= r`.** It returns `Result`. A reduced scalar from `ScalarField::from_bytes_unsigned` is guaranteed `< r`, but **may be zero** (negligible probability, but defense-in-depth matters). Constructing a `SecretKey` from a zero scalar will fail. The protocol functions like `derive_onetime_sk` should handle this — if `(b_spend + t_k) mod r == 0`, the wallet has detected a payment to the identity element, which is an invalid one-time key. The recipient cannot spend it. The probability is `1/r`, but a malicious sender could try to construct one to embarrass a wallet.

4. **`PublicKey::default()` is the identity element.** Using it as a sentinel ("uninitialized") is wrong because it's a *valid* G1 point (the point at infinity) and may silently match. The CHIP requires explicit identity-element checks (`A_sum != O`) before scanning.

5. **`scalar_multiply` does not clear the high bits / interpret signed.** It takes raw 32 big-endian bytes. Passing a SHA-256 output directly (without mod-r reduction) is **unsigned reduction by truncation** — wrong for protocol scalars. Always reduce mod r first via `ScalarField::from_bytes_unsigned`, then pass `.as_bytes()`.

**Why it happens:**
1. `chia-bls` mirrors `blst` which mirrors C-style mutate-in-place APIs.
2. `Result` boilerplate is annoying; developers reach for `.unwrap()` / `.expect()`.
3. Identity-element handling is rarely tested.

**How to avoid:**
1. **Wrap the dangerous APIs.** A helper `fn scalar_mul(pk: &PublicKey, scalar: &ScalarField) -> PublicKey { let mut p = *pk; p.scalar_multiply(scalar.as_bytes()); p }` makes the in-place mutation invisible and forces the `ScalarField` type. All code uses the wrapper.
2. **No `.expect()` on attacker-controlled bytes.** The scanner's `PublicKey::from_bytes(&tweak_point_bytes)` must use `?`. Internal-only constructions (from a `ScalarField` known to be in range) may use `.expect()`.
3. **Identity-element check at scan entry.** Reject any `tweak_point == PublicKey::default()` before the scalar-mul, per CHIP § "Identity Element (Zero-Sum Prevention)."
4. **Zero-aggregate check at send entry.** Reject `a_sum.is_zero()` per CHIP § "Identity Element."

**Warning signs:**
- `pk.scalar_multiply(...)` followed by use of `pk` (not the mutated result).
- `PublicKey::from_bytes(...).unwrap()` in any code path that ingests `TweakData`.
- No identity-element guard at the top of `scan_block`.

**Phase to address:**
Phase 1 (`CRYPTO-01` / `CRYPTO-02` — the primitives layer is where the wrappers live).

**Verification:**
1. Adversarial scanner test: feed `TweakData` with a malformed 48-byte tweak point (all-zero except for a high bit), assert the scanner returns `Ok(no_detections)` (or logs and skips), not a panic.
2. Identity-element test: feed `TweakData` with `tweak_point = PublicKey::default()`, assert no detections (and no panic).

---

### Pitfall 10: Cross-chain test-vector confusion (BIP-352 vs CHIP-0057)

**Category:** CORRECTNESS (silent — your tests pass but you're testing the wrong protocol)

**What goes wrong:**
BIP-352 uses secp256k1. The CHIP uses BLS12-381 G1. The test vectors are **not interchangeable**. They use:
- Different group orders.
- Different tag strings (`BIP0352/Inputs` vs `Chia_SP/Inputs`).
- Different key sizes (32-byte secp pubkeys vs 48-byte BLS G1 pubkeys).
- Different curves entirely.

A developer who lifts BIP-352 test vectors into the SDK's test suite will produce a test that **passes** (because both sender and scanner use the wrong curve) but doesn't validate the CHIP at all. Worse, if the test vector data accidentally works with the BLS implementation (which it shouldn't, but BLS hash-to-curve operations might tolerate it), the test could give false confidence.

**Why it happens:**
1. BIP-352 has more documentation, blog posts, and reference implementations than the CHIP. Copy-paste is tempting.
2. The CHIP's structure mirrors BIP-352's, so the test-vector format is similar.
3. Reviewers may not notice the curve mismatch.

**How to avoid:**
1. **Use the CHIP test vectors verbatim.** `chip-silent-payments.md` § "Test Cases" provides TV1, TV2, TV3, TV4 with full intermediate values. The prototype's `tests/test_vectors.py` (and `sp-common`'s Rust unit tests) check these. Copy them as-is into the SDK.
2. **Sanity check on key length.** Any test vector with a `33-byte` or `32-byte` sender PK is a BIP-352 vector; BLS G1 compressed is 48 bytes.
3. **Tag string check.** Any test vector that mentions `BIP0352/` (anywhere) is wrong.

**Warning signs:**
- Test data with `02` or `03` prefix (secp compressed key prefix) on a 33-byte hex string.
- `BIP0352/` in comments or constants.
- Test asserts a 32-byte or 33-byte public key.

**Phase to address:**
Phase 1 (`CRYPTO-03` — vectors as unit tests).

**Verification:**
The four CHIP TVs already lock the curve, tags, and key sizes.

---

### Pitfall 11: Reusing labeled addresses without warning users

**Category:** PRIVACY (linkability leak)

**What goes wrong:**
A labeled address `(B_scan, B_m)` shares `B_scan` with all other labeled addresses from the same wallet (and with the unlabeled address). An on-chain observer who collects multiple labeled addresses published by the same recipient (e.g., one labeled "donations", another labeled "invoices") can trivially link them: the first 48 bytes of the bech32m payload are identical.

This is documented in CHIP § "Label Linkability Note" and `docs/privacy-analysis.md`. It's not a protocol bug; it's a **UX hazard**. Wallets that present labeled addresses as if they were unrelated identities mislead users.

**Why it happens:**
1. "Labels" sound like "separate accounts" but cryptographically are not.
2. The convenience of one scan key for all labels is the whole point — but the privacy implication is non-obvious.
3. Users expect "different address = different identity" from prior wallet conventions.

**How to avoid:**
1. **SDK-side:** Provide a `SilentPaymentAddress::scan_pk()` accessor and document that two addresses with the same `scan_pk` are publicly linkable. This is informational, not enforced — the SDK can't prevent users from publishing linkable addresses.
2. **Wallet-author guidance:** The `BIND-02` documentation (or the `examples/silent_payment.rs`) should call this out. Wallet authors who present labels as "unlinkable" mislead users.
3. **Hard error case:** None. This is fundamental to the protocol.

**Warning signs:**
- Wallet UI presents labeled addresses with no indication of shared origin.
- API documentation describes labels as providing "privacy" rather than "payment-stream distinction."

**Phase to address:**
Phase 3 (`BIND-02` docs) + `examples/silent_payment.rs` (`EX-01`).

**Verification:**
Documentation review at Phase 5 (examples / bindings polish). No code-level verification possible.

---

### Pitfall 12: CAT2 silent-payment send producing undetectable payments

**Category:** API DESIGN → CORRECTNESS

**What goes wrong:**
CAT2 (and other layered primitives) wrap a `p2_delegated_puzzle_or_hidden_puzzle` as their inner puzzle. The sender CAN construct a CAT2 silent payment today — the CAT layer's `morph_condition` produces an output that, when unwrapped, has a synthetic key matching the silent-payment one-time PK.

But the **receiver cannot detect it** until CHIP-0058 (light-wallet indexer) supports CAT-aware tweak extraction: the on-chain output's puzzle hash is the CAT-wrapped hash, not the inner hash. The scanner derives the inner hash (from ECDH); the CAT wrapping converts it to the outer hash; the scanner never sees the inner hash on-chain.

If the SDK exposes a generic `derive_one_time_puzzle_hash` helper without flagging CAT, a wallet author might:
1. Call it.
2. Wrap the result in a CAT layer.
3. Send the bundle.
4. Recipient never sees it. Funds locked, but at a one-time PK the recipient *can* derive — the recipient just doesn't know to look.

The CAT2 send is **deferred to v2** per `PROJECT.md` Key Decisions. But the **risk** is that the v1 API surface accidentally enables it without warning.

**Why it happens:**
1. The send-side primitive `derive_one_time_puzzle_hash` is pure crypto — it takes keys and returns a puzzle hash. It doesn't know whether the caller will use it for XCH or for a CAT layer.
2. The CAT layer in the SDK (`Cat::issue_with_coin`) takes an inner puzzle hash and wraps it. Composing the two is mechanical for a wallet author who knows both APIs.

**How to avoid:**

1. **API shape:** Don't expose a bare `derive_one_time_puzzle_hash` that lets callers compose freely. Instead, ship the `SilentPaymentSend` *action* (per `SEND-04`), which composes through `Spends` and the standard layer — and **does not** plug into the CAT layer in v1. A wallet author would have to write custom non-action code to compose CAT + silent-payment, which is a clear signal they're going off the supported path.

2. **Documentation:** Anywhere a public helper produces a one-time PK or puzzle hash, add a doc-comment block warning: "This puzzle hash is intended for XCH (standard p2 puzzle). Wrapping in CAT/NFT layers produces outputs that recipients CANNOT DETECT until CHIP-0058 indexer support exists. Do not use for CAT silent payments in v1."

3. **Type-level discouragement:** If `SilentPaymentSend` returns a structured `SpendOutput` that already pairs with the standard p2 layer, it's hard to accidentally feed into the CAT path.

**Warning signs:**
- A public `derive_one_time_puzzle_hash` function without the warning comment.
- An example or test that composes `Cat::*` with `SilentPaymentSend`.

**Phase to address:**
Phase 2 (`SEND-01`, `SEND-04`). The action-system shape decision is here.

**Verification:**
Manual API review at Phase 5 (examples). Confirm there's no example showing CAT + silent-payment composition.

---

### Pitfall 13: Privacy leak — querying full node for candidate puzzle hashes

**Category:** PRIVACY (out-of-scope for SDK, but document)

**What goes wrong:**
A light client wanting to verify whether a candidate one-time puzzle hash exists on-chain can call `get_coin_records_by_puzzle_hash(candidate_ph)`. This is fast and uses no extra bandwidth. **But it reveals the candidate to the queried node.** If the node operator is the adversary, they learn that some client is interested in this puzzle hash. With enough queries from the same connection, the operator can build a candidate set that, combined with side channels (timing, IP), partially deanonymizes the recipient.

The CHIP § 11 ("Light Client Support") and the privacy-analysis doc both flag this. Mitigations are:
- BIP-158 compact filters (private, expensive).
- Full-node scanning (best, but only available to users with personal full nodes).
- The "tweak data" approach: server pre-computes per-spend tweak points and serves them; client does ECDH locally; this doesn't leak candidate PHs but does leak that the client is doing some kind of scanning.

For the **SDK** specifically: this is **not our problem in v1**. The SDK exposes `scan_from_tweaks(scan_sk, spend_sk, &TweakData, ...)` — a pure function. It doesn't fetch tweak data, it consumes it. Where the `TweakData` comes from is the wallet's choice. The SDK should:

1. Not provide a "fetch and scan" convenience that uses `get_coin_records_by_puzzle_hash`. Tempting (it'd be a one-liner for Sage), but it bakes in the privacy-leaking transport.
2. Document in the `TweakData` docstring that "the transport that produces `TweakData` should preserve scan privacy; querying candidate puzzle hashes directly leaks the candidate set."

**Why it happens:**
1. `get_coin_records_by_puzzle_hash` is the obvious, simplest API call.
2. Privacy concerns are abstract until a user-facing breach occurs.

**How to avoid:**
- The v1 design (transport-agnostic `TweakData` consumer) already does this correctly.
- Resist scope creep that adds a "convenience" `scan_from_full_node(rpc_client, ...)` helper.

**Warning signs:**
- A PR adds an RPC-backed scan helper to `chia-sdk-coinset` or `chia-sdk-client`.
- An example uses `get_coin_records_by_puzzle_hash` for scanning.

**Phase to address:**
Phase 3 docs (`RECV-01` / `RECV-02`). Reaffirm at Phase 5 (examples / bindings polish).

**Verification:**
Phase 5 review.

---

### Pitfall 14: Scan-key compromise reveals all past + future payments

**Category:** PRIVACY (custody-UX, out-of-scope for SDK)

**What goes wrong:**
The scan key `b_scan` is the **detection capability** for every past and future payment to this address. If compromised, the attacker can:
1. Scan the entire blockchain history and identify every silent payment received.
2. Continue to identify every future silent payment until the user rotates to a new address (and convinces all senders to use the new one — impractical for published addresses).

The spend key `b_spend` is needed to actually spend, so compromise of just `b_scan` doesn't directly enable theft. But the privacy violation is total.

The CHIP § 10 explicitly recommends `b_spend` cold-storage with `b_scan` online — but the *online* `b_scan` is still a high-value secret. Wallets that store `b_scan` next to `b_spend` (typical cold-wallet design) accidentally bring `b_spend` online for scanning, defeating the cold-storage benefit.

**Why it happens:**
1. Wallet authors assume "private keys are for spending" and put scan keys in the same secure-element as spend keys, then expose them together for any operation.
2. Mnemonic-based recovery derives both keys from the same seed; if the seed is compromised, both are compromised. This is intrinsic to the design.

**How to avoid (SDK-side, scoped):**
1. **Expose scan and spend keys separately.** `SilentPaymentKeys::scan_sk()` and `SilentPaymentKeys::spend_sk()` as distinct accessors, so wallet authors can architect split-custody. This is `ADDR-01` and is already the plan.
2. **Document the recommendation:** in the `SilentPaymentKeys` module docstring, link to CHIP § 10 and call out the asymmetric trust model.
3. **Don't enforce split custody.** Per `PROJECT.md` Out of Scope: "the SDK exposes both keys separately so wallet authors can split them, but doesn't enforce or design that workflow."

**Warning signs:**
- A combined `SilentPaymentKeys::all_keys() -> (SecretKey, SecretKey)` helper that encourages bundling.

**Phase to address:**
Phase 1 (`ADDR-01`).

**Verification:**
API review.

---

### Pitfall 15: Forward-compat — baking in `sp-service` JSON wire format

**Category:** FORWARD-COMPAT

**What goes wrong:**
Today's `sp-service` ships `ServerMessage::BlockData { height: u32, tweaks: Vec<PublicKey>, outputs: Vec<OutputMeta> }` over WebSocket JSON. This is a prototype, not a CHIP. CHIP-0058 (when defined) will specify a different wire format — possibly:
- A different message envelope (binary instead of JSON).
- Per-tweak coin-ID and amount metadata.
- Range-server semantics (request blocks N..M).
- Cut-through aggregation (skip already-spent coins).

If the SDK's `TweakData` type is literally the deserialized `ServerMessage::BlockData`, every wallet is locked to today's wire format. Migrating to CHIP-0058 becomes a breaking change for all consumers.

`PROJECT.md` Key Decisions explicitly anticipates this: "v1 = send-side + transport-agnostic receive primitive (no WS client)." The plan is to ship `TweakData` as a wire-format-agnostic input.

**Why it happens:**
1. Convenience: the easiest implementation is "deserialize the WS message, hand it to the scanner."
2. The prototype works today, so it's tempting to take the shortcut.

**How to avoid:**
1. **`TweakData` does not derive `Serialize`/`Deserialize` from the `sp-service` wire schema.** It's a plain Rust struct (`Vec<PublicKey>`, `Vec<OutputMeta>`) with no transport-specific fields (`height`, message envelope, etc.).
2. **The wallet author writes an adapter:** `fn from_sp_service_message(msg: ServerMessage) -> TweakData`. The adapter lives outside the SDK. When CHIP-0058 ships, only the adapter changes.
3. **No `serde` derives on `TweakData` matching `sp-service` field names.** If `TweakData` needs `serde` for bindings (`napi`, `pyo3`, `wasm`), define an explicit `#[derive(Serialize, Deserialize)]` with field names that match the SDK's idiomatic conventions, not `sp-service`'s.

**Warning signs:**
- `TweakData` has a `height: u32` field. (That's transport metadata, not scanning data.)
- `TweakData` is constructed via `serde_json::from_str` in the SDK.
- An `sp-service` dependency in `chia-sdk-*` Cargo.toml. (Should not exist.)

**Phase to address:**
Phase 3 (`RECV-01`).

**Verification:**
`cargo machete` will catch any accidental `sp-service` dependency. Manual review of `TweakData` field list.

---

### Pitfall 16: Bindings type-mapping for 48-byte BLS PublicKey

**Category:** API DESIGN

**What goes wrong:**
The bindy macro and `bindings/*.json` descriptors must map `chia_bls::PublicKey` to the appropriate target-language type:
- **napi (Node.js):** `Uint8Array` (48 bytes).
- **pyo3 (Python):** `bytes` (48 bytes).
- **wasm (browser):** `Uint8Array` (48 bytes).

If the descriptor maps `PublicKey` to a string (hex) or to a 32-byte array (Bytes32), the bindings produce wrong-shaped data at runtime. Worse, if it maps to `Vec<u8>` without a length constraint, callers can pass 32-byte or 96-byte buffers and the failure happens deep inside the BLS crate.

The existing bindings (per `concerns.md` § "Generated Binding Artifacts") use `bindy` to map types. The standard Chia wallet PK is also 48-byte BLS G1, so the mapping must already exist — silent payments should reuse it. If a new mapping is invented for `SilentPaymentAddress` or `TweakData`, that's a smell.

**Why it happens:**
1. Bindy descriptors are JSON; typos in type names are silent.
2. The 96-byte silent-payment address payload (scan + spend) is unusual; binding authors may try to invent a new type instead of reusing two `PublicKey` fields.
3. The `Vec<PublicKey>` in `TweakData::tweak_points` requires a list-of-fixed-bytes mapping; the existing bindings may handle this for `Vec<G1Element>` or may not.

**How to avoid:**
1. **Reuse existing `PublicKey` mappings.** `chia-sdk-bindings` already exposes `PublicKey` for the standard puzzle path. The silent-payments descriptor should reference the same type, not redefine it.
2. **Confirm `Vec<PublicKey>` handling.** Check that bindy generates a `PublicKey[]` (TS), `list[bytes]` (Python), `Array<Uint8Array>` (wasm) for the `TweakData::tweak_points` field. If not, this is a bindy bug to fix in `bindy-macro`, not a per-CHIP fix.
3. **`SilentPaymentAddress` exposes `(scan_pk: PublicKey, spend_pk: PublicKey)` as two fields, plus an `encode()` returning a string.** The 96-byte payload is an internal detail of the encoding, never exposed at the bindings boundary.
4. **`BIND-03` tests** (AVA / pytest / wasm) round-trip a `PublicKey` from each language through encoding and back. This catches mapping bugs immediately.

**Warning signs:**
- `bindings/silent_payments.json` defines a new `PublicKey48` type instead of referencing the existing `PublicKey`.
- A binding test passes a 32-byte array as a scan key and the SDK accepts it.
- TS `index.d.ts` shows `scan_pk: string` or `scan_pk: number[]` rather than `Uint8Array`.

**Phase to address:**
Phase 4 (`BIND-01`, `BIND-02`, `BIND-03`).

**Verification:**
`BIND-03` round-trip tests in each language are sufficient.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use existing signed `mod_by_group_order` for `input_hash` reduction | One less newtype to write | Undetectable payments forever; rewrite of all protocol primitives | **Never** — pitfall 1 |
| Accept `Vec<SecretKey>` for sender keys (raw or synthetic) | Smaller type surface | Wallet authors silently pass raw keys; payments undetectable | **Never** — pitfall 2 |
| Silently aggregate only wallet-controlled keys when external inputs present | "Works" in single-party tests | Multi-party offers silently break; users blame Sage | **Never** — pitfall 3 |
| Embed `sp-service` JSON deserialization into `TweakData` | Faster Sage integration today | Breaking change for every consumer when CHIP-0058 lands | Only if `sp-service` becomes the CHIP-0058 ref (TBD) — pitfall 15 |
| Ship `derive_one_time_puzzle_hash` as a public helper without action-system gating | Composability for power users | CAT2 silent-payment footgun (undetectable payments) | Only if every public helper carries the warning comment — pitfall 12 |
| Ship without `K_max` cap on scanner `k` iteration | Simpler scanner code | DoS via malicious tweak data | **Never** — pitfall 7 |
| Use `.unwrap()` on `PublicKey::from_bytes` for scanner inputs | Cleaner code | Panic on malformed `TweakData` | **Never** — pitfall 9 |
| Test only with `k = 0` test vectors | TV1/TV3/TV4 pass | Endianness bug in `ser32(k)` invisible | Only with a separate `k = 1` test — pitfall 6 |
| Test only at compile-time-known SHA tags | Easy assertions | Tag typo in a future PR breaks interop | Only with at least one external-corpus interop test — pitfall 4 |
| Use `bech32` instead of `bech32m` because the crate defaults to it | Less config | Wallet refuses your addresses | **Never** — pitfall 8 |

---

## Integration Gotchas

Common mistakes when connecting to external services / surfaces.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `chia-bls::PublicKey::scalar_multiply` | Treating it as functional (`let x = pk.scalar_multiply(...)`) | Mutate in place: `let mut p = *pk; p.scalar_multiply(...); use p` (pitfall 9.1) |
| `chia-bls::PublicKey::from_bytes` | `.unwrap()` on attacker-controlled bytes | Use `?` or skip with log when ingesting `TweakData` (pitfall 9.2) |
| `chia-bls::SecretKey::from_bytes` | Assume always succeeds for in-range scalars | Handle zero / out-of-range error case (pitfall 9.3) |
| `chia_puzzle_types::standard::StandardArgs::curry_tree_hash` | Pass raw wallet PK (not synthetic) | Call `pk.derive_synthetic()` first (pitfall 2) |
| `bip39::Mnemonic::to_seed` | Pass passphrase as the salt directly | Use `to_seed("")` — `bip39` prepends `"mnemonic"` automatically; matches Python PBKDF2 (verified in prototype `keys.rs`) |
| `chia-sdk-utils::Bech32` (assumed bech32m) | Use bech32 (non-m) variant | Use bech32m; HRP `spxch` / `tspxch` (pitfall 8) |
| `sp-service` WS messages | Deserialize directly into `TweakData` | Write an adapter; `TweakData` is transport-agnostic (pitfall 15) |
| Full-node RPC for scanning | `get_coin_records_by_puzzle_hash(candidate_ph)` per candidate | Out-of-scope for SDK; let consumers choose private vs fast (pitfall 13) |
| `Spends` action system with mixed-party inputs | `aggregate_sender_sks(&self.local_keys)` unconditionally | Detect external inputs, return `MultiPartyAggregationRequired` (pitfall 3) |
| CAT layer + silent-payment composition | `Cat::issue_with_coin(inner_ph = silent_payment_ph)` | Document as undetectable in v1; defer to v2 (pitfall 12) |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| No `K_max` cap on scanner `k` iteration | Scanner CPU spikes on certain blocks | Hard cap at `K_max = 2400` per spend group | First malicious or buggy tweak-data provider — could be day 1 (pitfall 7) |
| Per-iteration `BigUint` allocation in `ScalarField` arithmetic | Scanner slower than expected | Acceptable for v1; revisit if scan-throughput tests show this is the bottleneck | At >10k tweak points/second |
| Manual `scalar_multiply` loop (per CHIP § Appendix C) | If we end up doing what the Python prototype does | `chia-bls::PublicKey::scalar_multiply` already wraps `blst_p1_mult` — use it | The Python prototype's problem, not ours |
| `output_phs: Vec<[u8; 32]>` linear scan inside `k` loop | O(N×K_max×P) per block | Build `HashSet<[u8; 32]>` once per block; the prototype `scan_block` already does this | At >100 outputs per spend group |
| Re-derive `label_pk` from `label_scalar` inside `k` loop | O(N×K_max×L) point operations | Precompute `Vec<(label_pk, m)>` outside the loop; prototype already does this | At >5 registered labels |

---

## Security Mistakes

Domain-specific issues beyond general Rust safety.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Identity-element shared secret (`A_sum == O`) | Catastrophic: all recipients get the same predictable shared secret | Sender refuses `a_sum == 0`; scanner skips `A_sum == O` (pitfall 9.4, CHIP § Identity Element) |
| Scan key in same module/file as spend key with no access-control | `b_scan` compromise reveals all past + future payments | Expose `scan_sk()` / `spend_sk()` as separate accessors; document asymmetric trust (pitfall 14) |
| Reusing the *same* one-time puzzle hash for two payments | Double-payment to same address (won't happen in spec, but a bug could cause it) | Already prevented by `input_hash` (coin_id varies per spend); just don't disable it |
| Logging `shared_secret`, `t_k`, or `b_scan` | Privacy + theft (b_scan: privacy; b_spend + t_k: theft of detected coins) | No `Debug` derive on key types; secret-key types should `Debug` as `SecretKey([REDACTED])` |
| Storing `TweakData` containing user's scan key | Adversary with disk access decrypts payment history | `TweakData` does not contain the scan key (it's just tweak_points + outputs); but the *derived* shared secrets and onetime SKs do — protect them |
| Constant-time concerns in `ScalarField::mul` / `add` | Side-channel leak of scan key | The ECDH path uses scan_sk through `chia_bls::scalar_multiply` (blst, constant-time). `ScalarField` ops are used for `input_hash`, `t_k`, `label_scalar` — none of which are secrets. Acceptable for v1; document. |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Wallet shows `spxch1...` and `xch1...` addresses without explanation | User confusion; pasting silent-payment address into a non-supporting wallet → funds lost (well, address rejected) | UI labels each type clearly; documentation explicitly names them |
| Wallet treats labeled addresses as separate identities | Users believe they have privacy between payment streams; observer can link them via shared `B_scan` | Wallet UI calls out shared origin; SDK docs (pitfall 11) |
| Wallet auto-rotates labels without telling sender | Sender's saved address keeps working (correct), but no notification → confusing audit trail | Out-of-scope for SDK; wallet design problem |
| "Send" succeeds but recipient doesn't see the payment | If pitfalls 1/2/3/4/5/6 land in production: total trust loss | Comprehensive test coverage, simulator round-trip (pitfall 1-9) |
| Recovery from mnemonic doesn't scan for change label (m=0) | Change appears as "missing" or unspendable until separately re-scanned | Per CHIP: every recipient wallet should scan for `m=0` change label; `RECV-02` should include change-scan in default behavior |

---

## "Looks Done But Isn't" Checklist

- [ ] **`ScalarField`:** Has a `from_bytes_unsigned` constructor; has an adversarial test with `[0xff; 32]` input that verifies reduction (not just identity). (Pitfall 1)
- [ ] **`compute_input_hash`:** Uses lex-smallest coin ID via `iter().min()`; has TV4 multi-input test passing. (Pitfall 5)
- [ ] **`derive_output_tweak`:** Has at least one test with `k = 1` (not just `k = 0`). (Pitfall 6)
- [ ] **`derive_onetime_pk`:** When labels are involved, scanner does NOT break before checking labels at this `k`. (Pitfall 7)
- [ ] **`aggregate_sender_sks`:** Refuses (or hard-errors) when called with fewer keys than spend bundle inputs. (Pitfall 3)
- [ ] **`SilentPaymentSend`:** Composes through `Spends` and standard layer only; cannot be composed with CAT layer in v1. (Pitfall 12)
- [ ] **`SilentPaymentAddress::decode`:** Rejects bech32 (non-m), wrong HRP, mixed-case, truncated. Round-trip property test passes. (Pitfall 8)
- [ ] **Scanner:** Has `K_max = 2400` cap; identity-element guard at entry; uses `?` (not `.unwrap()`) on `PublicKey::from_bytes` for tweak points. (Pitfalls 7, 9)
- [ ] **`TweakData`:** No `height` field; no `sp-service` dependency in Cargo.toml; transport-agnostic. (Pitfall 15)
- [ ] **Tagged-hash tags:** Defined once as `const &str`; CHIP test vectors (TV1/TV3/TV4) pass. (Pitfall 4)
- [ ] **Test suite:** Uses CHIP test vectors (BLS, 48-byte keys, `Chia_SP/` tags), NOT BIP-352 vectors. (Pitfall 10)
- [ ] **Bindings:** `PublicKey` maps to `Uint8Array(48)` / `bytes(48)`; round-trip test passes in each target language. (Pitfall 16)
- [ ] **Docs:** `SilentPaymentKeys` warns about scan-key high-value-secret; `derive_*` helpers warn about CAT undetectability; `TweakData` docstring flags candidate-PH query as privacy-leaking. (Pitfalls 12, 13, 14)

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Signed/unsigned scalar mix-up shipped (pitfall 1) | HIGH | Every silent payment sent during the bug window is undetectable. Recipients with the (correct, unsigned) scan logic will never find them. Senders must be notified to resend after the fix; existing one-time PHs are unspendable by the intended recipient. Sender's wallet retains the offset secret (technically can derive the one-time SK and recover the coin to a new address). |
| Synthetic vs raw key mix-up (pitfall 2) | HIGH | Same as pitfall 1: undetectable payments. Recovery: sender's wallet derived the wrong synthetic key, so the one-time SK is unknown to anyone — funds are likely lost unless the sender can recompute from logs. |
| Multi-party aggregation skipped (pitfall 3) | HIGH | Same. Sender's wallet has the local-only `a_sum`, could compute the one-time SK and recover, but only if it logged the right intermediate. |
| Tagged-hash tag typo (pitfall 4) | HIGH | Same. Network-wide bug if many wallets ship the same typo. |
| Coin-ID ordering bug (pitfall 5) | HIGH | Same. |
| `ser32` / `ser256` endianness bug (pitfall 6) | HIGH | Same. |
| `k` iteration termination bug (pitfall 7) | MEDIUM | Detection fails for some payments (labeled at k≥1 after unlabeled miss). Re-scan with fixed code recovers all detections — no on-chain action needed. |
| Bech32m HRP / checksum bug (pitfall 8) | LOW | Addresses are unparseable; users get clear errors. Fix and republish. No funds at risk. |
| Generic Rust / chia-bls misuse (pitfall 9) | LOW-MEDIUM | Panics in scanner: re-run after fix. Identity-element bypass: defense-in-depth, probability is negligible. |
| BIP-352 vs CHIP test vector mix-up (pitfall 10) | LOW (caught in CI) | Tests fail at compile/test time, not in production. |
| Labeled-address linkability disclosure (pitfall 11) | LOW (documentation) | Update docs; advise users to use separate wallets for unlinkable identities. |
| CAT2 silent-payment footgun (pitfall 12) | HIGH | Same as pitfall 1 for CAT silent payments. Mitigation is to prevent the footgun in API, not recover. |
| Full-node query privacy leak (pitfall 13) | MEDIUM (privacy, not funds) | Affected users' scanning patterns are known to node operators. Cannot retroactively redact. Future use should switch transport. |
| Scan-key compromise (pitfall 14) | HIGH (privacy) | User must publish a new address (different scan key derivation index or new mnemonic). Past payments remain linkable. |
| `sp-service` wire format baked in (pitfall 15) | MEDIUM | Breaking SDK version bump for all consumers. Adapter-layer design (recommended) reduces this to "internal-only" breakage. |
| Bindings type-mapping bug (pitfall 16) | LOW | Caught at first cross-language test; fix and regenerate bindings. |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls. (Phase numbering tentative — orchestrator will finalize.)

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Signed/unsigned scalar reduction | **Phase 1** (Crypto primitives — `CRYPTO-01`) | `ScalarField` newtype is the only mod-r path; adversarial `[0xff; 32]` test passes; TV1/TV3/TV4 pass. |
| 2. Synthetic vs raw key confusion | **Phase 2** (Send-side — `SEND-01`/`SEND-03`) | `SyntheticSecretKey` newtype OR documented + simulator round-trip (`SIM-02`) catches it. |
| 3. Multi-party aggregation silent fallback | **Phase 2** (`SEND-03`) | Unit test with mismatched input count returns `Err(MultiPartyAggregationRequired)`. |
| 4. Tagged-hash domain-tag typo | **Phase 1** (`CRYPTO-02`) | Constants module; CHIP TVs pass. |
| 5. Coin-ID lexicographic ordering | **Phase 2** (`SEND-02`) | TV4 multi-input test passes; order-independence property test. |
| 6. `ser32`/`ser256` endianness | **Phase 2** (`SEND-01`) + **Phase 3** (labels — `RECV-04`) | Custom `k=1` test vector; TV3 label_scalar pin. |
| 7. `k` iteration termination | **Phase 3** (`RECV-04` — labeled detection) | Synthetic test: labeled at `k=1` after unlabeled `k=0`; `K_max` cap test. |
| 8. Bech32m HRP / checksum | **Phase 1** (`ADDR-02`) | Round-trip test; negative tests for wrong HRP / non-m. |
| 9. Generic Rust / chia-bls API misuse | **Phase 1** (`CRYPTO-01`/`CRYPTO-02`) + **Phase 3** (`RECV-02`) | Wrappers; adversarial scanner test with malformed `TweakData`. |
| 10. BIP-352 vs CHIP test vectors | **Phase 1** (`CRYPTO-03`) | CHIP TVs only; review checklist. |
| 11. Labeled-address linkability | **Phase 5** (Examples / bindings polish — `EX-01`, `BIND-02`) | Documentation review. |
| 12. CAT2 silent-payment footgun | **Phase 2** (`SEND-04` API shape) + **Phase 5** (docs) | Action-system composition only; doc warnings on public helpers. |
| 13. Full-node query privacy | **Phase 3** docs (`RECV-01`) | No RPC-backed scan helper in SDK. |
| 14. Scan-key compromise | **Phase 1** (`ADDR-01` API shape) | Separate accessors; docstring with CHIP § 10 link. |
| 15. `sp-service` wire format lock-in | **Phase 3** (`RECV-01`) | `TweakData` is plain Rust struct, no `sp-service` dep. |
| 16. Bindings type-mapping | **Phase 4** (`BIND-01`/`BIND-02`/`BIND-03`) | Round-trip tests in napi / pyo3 / wasm. |

---

## Out-of-Scope Pitfalls (Documented, Not Addressed)

These appear in the privacy analysis / CHIP § 11 but are explicitly out-of-scope per `PROJECT.md`. Listed here so the roadmap doesn't accidentally schedule them.

- **Light-client transport privacy** (BIP-158 filters, compact-block-filter scanning). Out of scope; wallet authors choose their transport. CHIP-0058 will specify.
- **Hardware-wallet scan-key custody UX.** Out of scope; SDK exposes keys, wallets design custody.
- **Bulk-scanning the chain.** `sp-service`'s job, not the SDK's.
- **GCS block-filter prefilter.** `sp-service`'s job; not yet a CHIP.
- **Timing / amount correlation attacks.** Inherent to transparent blockchain; not addressable in this protocol layer.
- **Mempool observability** (sender's bundle is visible pre-block). Inherent to Chia's mempool gossip; out of scope.
- **CoinJoin / mixer privacy.** Not a substitute; documented in `privacy-analysis.md`. Not what silent payments provide.

---

## Sources

- `~/silent-payments/chip-silent-payments.md` §§ 4 (Overview), 5 (Specification), 10 (Security), 11 (Light Client Support, Appendix A), Appendix C (ECDH Scalar Multiplication). Test vectors TV1, TV2, TV3, TV4.
- `~/silent-payments/docs/privacy-analysis.md` — Privacy properties, threat model, CoinJoin compatibility analysis.
- `~/silent-payments/crates/sp-common/src/scalar.rs` — Reference `ScalarField` implementation with explicit unsigned-vs-signed documentation.
- `~/silent-payments/crates/sp-common/src/keys.rs` — BIP-39 → scan/spend SK derivation paths (`m/12381/8444/12/0`, `m/12381/8444/13/0`).
- `~/silent-payments/crates/sp-common/src/protocol.rs` — `aggregate_sender_sks`, `create_silent_payment_outputs`, `scan_for_silent_payments`.
- `~/silent-payments/crates/sp-common/src/ecdh.rs` — Sender / scanner ECDH, `compute_input_hash`.
- `~/silent-payments/crates/sp-common/src/tagged_hash.rs` — BIP-340 tagged-hash implementation.
- `~/silent-payments/crates/sp-client/src/scanner.rs` — Reference scanner termination rule (unlabeled-or-labeled match → continue; both miss → break).
- `~/silent-payments/CLAUDE.md` — Key design details, especially `calculate_synthetic_offset` signed-bytes note.
- `/home/kdc/chia-wallet-sdk/.planning/PROJECT.md` — Requirements, key decisions, out-of-scope items.
- `/home/kdc/chia-wallet-sdk/.planning/codebase/CONCERNS.md` — Signed-vs-unsigned hazard, synthetic-vs-raw key hazard, multi-input aggregation hazard.
- BIP-352 (Silent Payments, Bitcoin) — Josie Baker, Ruben Somsen, Andrew Toth. Referenced for protocol structure; **NOT** for test vectors (curve mismatch).
- BIP-340 (Schnorr Signatures for secp256k1) — tagged-hash construction.
- BIP-350 (Bech32m). HRP encoding for the silent-payment address.

---
*Pitfalls research for: CHIP-0057 silent payments — wallet-side integration into chia-wallet-sdk*
*Researched: 2026-05-15*
