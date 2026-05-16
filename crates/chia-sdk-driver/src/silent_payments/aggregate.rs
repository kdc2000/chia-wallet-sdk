//! Sender-side scalar aggregation for silent-payment sends.
//!
//! Sums a slice of synthetic secret keys (`Σ sk_i mod r`) into a single
//! [`ScalarField`] that the rest of the send-side flow consumes. The output is
//! a scalar — *not* a `SecretKey` — because the next step in the send-side flow
//! (`derive_one_time_puzzle_hash`) needs the scalar form (for ECDH) AND the
//! aggregated public key (for `compute_input_hash`). Recovering the PK is a
//! `chia_bls::SecretKey::from_bytes(scalar.as_bytes()).public_key()` round-trip
//! — see `04-RESEARCH.md` §11 Pitfall A.
//!
//! See `Spends::finish_with_silent_payment_keys` (Plan 04-03) for the
//! Spends-level multi-party hard-error path that this free function does NOT
//! perform. This function is a pure aggregator over the SKs the caller hands
//! it; if the caller passes a partial-control SK slice, the result is silently
//! wrong on chain. The Spends-level check is the prevention mechanism.
//!
//! Synthetic-vs-raw key boundary: the `sks` slice MUST contain synthetic SKs
//! (the ones whose PKs are curried into `StandardArgs::synthetic_key`). The SDK
//! enforces this structurally via the `Spends::finish_with_silent_payment_keys`
//! `synthetic_sks: &IndexMap<Bytes32, SecretKey>` parameter (Plan 04-03) — that
//! map's contract is "every value is a synthetic SK for its keyed
//! `p2_puzzle_hash`" — and pass that slice's values into this function.

use chia_bls::SecretKey;
use chia_sdk_types::silent_payments::ScalarField;

/// Aggregate synthetic sender secret keys via mod-r addition.
///
/// Returns `Σ sk_i mod r` as a [`ScalarField`]. The empty-slice case returns
/// the zero scalar (callers must additionally check `is_empty()` if zero is
/// not a sensible aggregate).
///
/// Each input SK is fed through [`ScalarField::from_bytes_raw`] (NOT
/// `from_bytes_unsigned`) because `chia_bls::SecretKey` is already constrained
/// to `< r` by construction. The addition then reduces mod r. There is a
/// `1/r ≈ 2^-255` chance the sum is zero (cosmic-ray-level probability — see
/// `04-RESEARCH.md` §11 Pitfall H); callers that downstream call
/// `SecretKey::from_bytes(self.as_bytes())` must accept this vanishing risk.
///
/// Privacy warning: this function takes secret-key material. The resulting
/// [`ScalarField`] is sensitive — wallets must treat it like an SK (zeroize on
/// drop, do not log). Consumers further compose this with
/// `derive_one_time_puzzle_hash` which emits an on-chain puzzle hash whose
/// recipient holds the scan key; memos attached at the action layer are
/// visible to anyone holding the recipient's scan key.
#[must_use]
pub fn aggregate_sender_sks(sks: &[SecretKey]) -> ScalarField {
    let mut sum = ScalarField::from_bytes_raw([0u8; 32]);
    for sk in sks {
        let sk_scalar = ScalarField::from_bytes_raw(sk.to_bytes());
        sum = sum.add(&sk_scalar);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // TV4 multi-input aggregation. Two sender synthetic SKs and the pinned
    // aggregated scalar from the `~/silent-payments` reference impl
    // (`crates/sp-common/src/protocol.rs::test_aggregate_sks_tv4`).
    const TV4_SENDER_SK_0: [u8; 32] =
        hex!("5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a");
    const TV4_SENDER_SK_1: [u8; 32] =
        hex!("05fded8808216b65d439fc41cb07c7270e37ed743e0745652afe055cfe91cf0f");
    const TV4_AGGREGATED_SK: [u8; 32] =
        hex!("5600d8781de32f0f3d86bc96b46a3a4ea56ae26da168b55dc66b503acbf95b89");

    /// SEND-03 (free-fn portion) + ROADMAP success criterion #1 (round-trip):
    /// aggregating the two TV4 sender synthetic SKs produces the pinned
    /// `TV4_AGGREGATED_SK` byte-for-byte. Catches `ScalarField::add` regressions
    /// AND iteration-order bugs (aggregation is commutative; if a future
    /// refactor sorts the slice, the result must still match).
    #[test]
    fn tv4_aggregate_sender_sks_matches() {
        let sk0 = SecretKey::from_bytes(&TV4_SENDER_SK_0).expect("TV4 sender SK 0 < r");
        let sk1 = SecretKey::from_bytes(&TV4_SENDER_SK_1).expect("TV4 sender SK 1 < r");

        let aggregated = aggregate_sender_sks(&[sk0, sk1]);

        assert_eq!(
            *aggregated.as_bytes(),
            TV4_AGGREGATED_SK,
            "aggregate_sender_sks(TV4) must match the pinned TV4_AGGREGATED_SK"
        );
    }
}
