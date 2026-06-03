# `with_silent_payment_keys` silently accepts raw (un-synthesized) keys → unspendable coins; no type/runtime guard

**Component:** `chia-sdk-driver` silent payments (CHIP-0057)
**Version:** chia-wallet-sdk 0.33.0 (git `392ee2e7c05886b4cb18ce126d0e0ed8170f8455`), reproduced via the Python bindings.

## Summary

`Spends::with_silent_payment_keys` requires the caller to pass **synthetic** keys, but its
parameters are plain `IndexMap<Bytes32, PublicKey>` / `IndexMap<Bytes32, SecretKey>`, and
nothing — neither the type system nor a runtime check — enforces it. Passing the **raw**
wallet keys compiles, runs, and produces a fully valid-looking signed `SpendBundle`. The only
thing wrong is the recipient's one-time puzzle hash, which is derived from the raw sender
scalar in the ECDH. The payment broadcasts and confirms, but the resulting coin is
**undetectable and unspendable** by any BIP-352 / CHIP-0057-compliant scanner. The error
surfaces only off-chain, after funds have moved — i.e. it is silent and effectively
unrecoverable.

```rust
// crates/chia-sdk-driver/src/action_system/spends.rs:113-121
#[cfg(feature = "chip-0057")]
pub fn with_silent_payment_keys(
    &mut self,
    synthetic_pks: IndexMap<Bytes32, PublicKey>,
    secret_keys: IndexMap<Bytes32, SecretKey>,   // <- MUST already be synthetic; not enforced
) -> &mut Self { ... }
```

## Why it's easy to hit

Two adjacent APIs use **opposite** synthesis conventions:

- **Standard-spend path synthesizes _for_ the caller** —
  `puzzle_hash_for_pk` calls `pk.derive_synthetic()`:
  ```rust
  // crates/chia-sdk-driver/src/silent_payments/protocol.rs:109-112
  pub fn puzzle_hash_for_pk(pk: &PublicKey) -> Bytes32 {
      let synthetic = pk.derive_synthetic();          // <- synthesized internally
      StandardArgs::curry_tree_hash(synthetic).into()
  }
  ```
- **Silent-payment send path uses the registered key _verbatim_** — no `.derive_synthetic()`:
  ```rust
  // crates/chia-sdk-driver/src/silent_payments/protocol.rs:227-253 (derive_one_time_puzzle_hash)
  let tweak_scalar = aggregated_sender_sk.mul(input_hash);
  let mut point = *scan_pk;
  point.scalar_multiply(tweak_scalar.as_bytes());     // <- registered SK used as-is
  // ... and aggregate_sender_sks (protocol.rs:148-155) sums the SKs verbatim, no synthesis
  ```

A wallet that correctly registers raw keys everywhere else, and reads the standard-spend path
as the precedent, will register raw keys here too and get silently-wrong output. The
requirement lives only in a doc comment:

```rust
// crates/chia-sdk-driver/src/silent_payments/protocol.rs:123-126
// Synthetic-vs-raw key boundary: callers MUST pass synthetic SKs (the ones
// whose PKs are curried into `StandardArgs::synthetic_key`). The Spends-level
// multi-party hard-error check in `Spends::finish_with_keys` is the prevention
// mechanism.
```

Through the Python bindings even the `silent_payment_synthetic_sks` field-name hint is gone —
the caller just hands over a `SecretKey`.

## The existing guard does not cover this

The comment above claims `finish_with_keys`'s multi-party hard-error is the prevention
mechanism, but the gates only check key **presence / coverage**, not synthetic-ness:

```rust
// crates/chia-sdk-driver/src/action_system/spends.rs (sp_finish_branch)
// GATE 2 — keys registered at all:
let Some(secret_keys) = spends.silent_payment_synthetic_sks.as_ref() else {
    return Err(DriverError::SilentPaymentKeysNotRegistered);
};
// GATE 3 — every input has a matching key (presence only):
let Some(sk) = secret_keys.get(&ph) else {
    return Err(DriverError::SilentPaymentMultiPartyUnsupported);
};
```

A key that is **present but raw** passes both GATE 2 and GATE 3.

## Reproduction

Single input, one recipient. Register raw `wallet_pk` / `wallet_sk` vs. their
`.derive_synthetic()` equivalents; everything else identical (same `input_hash`, same coin-id
set, same `Chia_SP/` tagged-hash prefix, `k = 0`):

| Registered key | Recipient one-time puzzle hash | Outcome |
|----------------|--------------------------------|---------|
| **RAW**        | `45c3736f…`                    | compiles, signs, broadcasts — **WRONG** (no error raised) |
| **SYNTHETIC**  | `cf4a72ce…`                    | matches the BIP-352 / reference derivation |

(These values are from our project's pinned fixture; an independent run with throwaway keys
reproduces the same mechanism — `34df32eb…` raw vs `bd0e04dc…` synthetic. The mechanism is the
point, not the specific bytes. A minimal standalone Rust/Python repro can be attached.)

## Suggested fixes (in order of bang-for-buck)

1. **Runtime invariant (cheap, non-breaking, covers all language bindings).**
   The `IndexMap` key is already the coin's `p2_puzzle_hash`. For a correctly-synthetic
   registered pk, `StandardArgs::curry_tree_hash(registered_pk) == p2_puzzle_hash` by
   construction; a raw pk fails this. Assert it (and `registered_sk.public_key() ==
   registered_pk`) at registration/finish and fail with a new
   `DriverError::SilentPaymentKeyNotSynthetic`. Deterministic, catches single-input too, and
   fires **before** the bundle is signed. This alone would have caught our bug.

2. **Type-level guard (Rust).**
   A `SyntheticSecretKey` / `SyntheticPublicKey` newtype whose only safe constructor is
   `derive_synthetic()` (plus an explicit `_unchecked` escape hatch); make
   `with_silent_payment_keys` take it. Makes the raw-key mistake a compile error. Note the
   root of the unsafety: `DeriveSynthetic::derive_synthetic()` maps `SecretKey → SecretKey`
   (same type), so today the compiler cannot tell raw from synthetic. (Rust only — the Python
   bindings still need fix #1.)

3. **Consistency.**
   Align the silent-payment send path with the standard-spend convention so the two adjacent
   APIs don't synthesize-or-not differently — or, at minimum, document the boundary loudly at
   the public call site (`with_silent_payment_keys`) rather than only internally.

## Context

We hit this in a BIP-352 silent-payments port. Fixing our side was a one-liner (register
`.derive_synthetic()` keys), but the silent, irreversible failure mode seems worth a guard
upstream. Happy to send a minimal reproduction or a PR for option 1.
