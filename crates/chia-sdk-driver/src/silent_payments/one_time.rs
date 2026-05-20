//! Sender-side one-time puzzle-hash derivation for silent-payment outputs.
//!
//! Composes Phase 3's [`derive_output_tweak`], [`derive_onetime_pk`], and
//! [`puzzle_hash_for_pk`] over an ECDH shared secret derived from the sender's
//! aggregated synthetic SK and the recipient's scan PK. The result is the
//! standard p2 puzzle hash for the recipient's k-th output in this spend group
//! — a `Bytes32` that the receiver's `scan_from_tweaks` (Phase 3) will match
//! against when scanning the block this transaction lands in.
//!
//! Data flow:
//! ```text
//!   aggregated_sender_sk * input_hash * scan_pk  -->  shared_secret  (ECDH)
//!   shared_secret + k                            -->  t_k            (derive_output_tweak)
//!   spend_pk + t_k * G                           -->  onetime_pk     (derive_onetime_pk)
//!   StandardArgs::curry(onetime_pk.derive_synthetic()) -> puzzle_hash
//! ```
//!
//! The `aggregated_sender_sk` parameter is the result of `aggregate_sender_sks`
//! over the wallet's synthetic SKs for every XCH input of the transaction
//! (`Spends::finish_with_keys` computes this at finish time inside the
//! chip-0057 SP branch). The `input_hash` is from `compute_input_hash` over the
//! same coin-id set and the aggregated synthetic PK.
//!
//! Synthetic-vs-raw key boundary: this function does NOT compute the
//! aggregated PK internally — that's the caller's job. The caller must
//! round-trip the aggregated SK back through
//! `SecretKey::from_bytes(...).public_key()` before calling `compute_input_hash`,
//! otherwise the receiver and sender will disagree on the input hash and
//! detection will silently fail.

use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_sdk_types::silent_payments::ScalarField;
use chia_sha2::Sha256;

use crate::silent_payments::{derive_onetime_pk, derive_output_tweak, puzzle_hash_for_pk};

/// Derive the on-chain standard-p2 puzzle hash for the recipient's k-th output
/// in this spend group.
///
/// `scan_pk` is the recipient's scan public key (from their silent-payment
/// address). `spend_pk` is the recipient's spend public key (for unlabeled
/// sends it equals the address's `spend_pk` field; for labeled sends the
/// caller passes the address's already-tweaked spend PK, which differs from
/// the unlabeled by `+ label_pk(m)`).
///
/// `aggregated_sender_sk` is the sum-mod-r of the sender's synthetic SKs for
/// every XCH input in this transaction (`aggregate_sender_sks`). `input_hash`
/// is the per-spend-group input-hash (`compute_input_hash`). `k` is the
/// per-recipient counter on `Spends` — 0 for the first output to `scan_pk`,
/// 1 for the second, etc.
///
/// Privacy warning: this function emits a puzzle hash that, when used in a
/// `CreateCoin` condition, lands an output at a fresh one-time address on chain.
/// The address is unlinkable to the recipient's published `spxch1...` address
/// without the scan secret key. However, any memos attached to the corresponding
/// `CreateCoin` condition are visible on chain and to anyone with the scan
/// key — see `SilentPaymentSend::memos` for the memo-hint guard that prevents
/// the standard wallet from promoting a 32-byte first memo to a `puzzle_hash`
/// hint and defeating the privacy gain.
#[must_use]
pub fn derive_one_time_puzzle_hash(
    scan_pk: &PublicKey,
    spend_pk: &PublicKey,
    aggregated_sender_sk: &ScalarField,
    input_hash: &ScalarField,
    k: u32,
) -> Bytes32 {
    // Step 1: tweak_scalar = aggregated_sender_sk * input_hash (mod r).
    // This is the sender-side analog of the receiver's tweak_point construction.
    let tweak_scalar = aggregated_sender_sk.mul(input_hash);

    // Step 2: ECDH over scan_pk. shared_secret = SHA256(tweak_scalar * scan_pk).
    // Treats `tweak_scalar` as the scalar in the BLS scalar_multiply call.
    let mut point = *scan_pk;
    point.scalar_multiply(tweak_scalar.as_bytes());
    let mut h = Sha256::new();
    h.update(point.to_bytes());
    let shared_secret: [u8; 32] = h.finalize();

    // Step 3: t_k = derive_output_tweak(shared_secret, k) — reuses Phase 3.
    let t_k = derive_output_tweak(&shared_secret, k);

    // Step 4: onetime_pk = spend_pk + t_k * G — reuses Phase 3.
    let onetime_pk = derive_onetime_pk(spend_pk, &t_k);

    // Step 5: puzzle_hash = curry(onetime_pk.derive_synthetic()) — reuses Phase 3.
    puzzle_hash_for_pk(&onetime_pk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chia_bls::SecretKey;
    use hex_literal::hex;

    // ─── TV1 pinned bytes ──────────────────────────────────────────────────
    // Sourced from:
    //   - Phase 3 `scanner.rs::tests` (TV1_SCAN_SK, TV1_SPEND_SK, TV1_SPEND_PK,
    //     TV1_INPUT_HASH, TV1_PUZZLE_HASH).
    //   - `~/silent-payments/crates/sp-common/src/ecdh.rs::tests::tv1_scan_pk`
    //     (TV1_SCAN_PK).
    //   - `~/silent-payments/crates/sp-common/src/ecdh.rs::tests::tv1_sender_syn_sk`
    //     (TV1_AGGREGATED_SENDER_SK — for single-input TV1 the aggregated
    //     SK is the sole sender synthetic SK).

    const TV1_SCAN_SK: [u8; 32] =
        hex!("132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6");
    const TV1_SCAN_PK: [u8; 48] = hex!(
        "a04f404bfbfdc9311736899fe32d2275bb007814510c3523529487ad75736075"
        "73ade20d31c75107b40331fff79ac896"
    );
    const TV1_SPEND_PK: [u8; 48] = hex!(
        "8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527db"
        "f6f31e22e0831ba5088c952625a75c74"
    );
    // TV1 aggregated sender SK — single-input, so equals the sole sender SK.
    const TV1_AGGREGATED_SENDER_SK: [u8; 32] =
        hex!("5002eaf015c1c3a9694cc054e96273279732f4f963616ff89b6d4addcd678c7a");
    const TV1_INPUT_HASH: [u8; 32] =
        hex!("38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411");
    const TV1_PUZZLE_HASH: [u8; 32] =
        hex!("23adba149dd9000d65e0f8e21b6975364cbe89a63caf56533df4b7664c21fbf5");

    /// SEND-01 + ROADMAP success criterion #1: TV1 round-trip closure.
    /// The sender-side puzzle-hash derivation produces the SAME byte-string
    /// that Phase 3's scanner detected in `tv1_scan_detects_unlabeled_k0`.
    #[test]
    fn tv1_derive_one_time_puzzle_hash_matches() {
        let scan_pk = PublicKey::from_bytes(&TV1_SCAN_PK).expect("TV1 scan_pk");
        let spend_pk = PublicKey::from_bytes(&TV1_SPEND_PK).expect("TV1 spend_pk");
        let aggregated_sender_sk = ScalarField::from_bytes_raw(TV1_AGGREGATED_SENDER_SK);
        let input_hash = ScalarField::from_bytes_unsigned(TV1_INPUT_HASH);

        let result =
            derive_one_time_puzzle_hash(&scan_pk, &spend_pk, &aggregated_sender_sk, &input_hash, 0);

        assert_eq!(
            *result.as_ref(),
            TV1_PUZZLE_HASH,
            "SEND-01 TV1 round-trip: sender-side derive_one_time_puzzle_hash \
             must match Phase 3 scanner's tv1_scan_detects_unlabeled_k0 result"
        );
    }

    /// SEND-01 + ROADMAP success criterion #1 (k=1 leg): at k=1 the derivation
    /// matches Phase 3's `bespoke_k1_detection` in-test computation.
    ///
    /// The test re-derives the expected puzzle hash via the same Phase-3
    /// primitive chain (`compute_shared_secret_from_tweak` +
    /// `derive_output_tweak(.., 1)` + `derive_onetime_pk` + `puzzle_hash_for_pk`)
    /// over the TV1 inputs, then asserts byte-equality against
    /// `derive_one_time_puzzle_hash(.., k=1)`. Catches `ser32(k)` endianness
    /// regressions because TV1/TV3/TV4 are all k=0.
    #[test]
    fn derive_one_time_puzzle_hash_k1_round_trip() {
        use crate::silent_payments::compute_shared_secret_from_tweak;

        // Phase-3 scanner.rs::bespoke_k1_detection name precedent: use
        // `b_*` shorthand to keep clippy::similar_names quiet without
        // suppression attributes (the scan vs spend pair differs by one
        // byte under longer names like `scan_sk` / `spend_pk` — both routes
        // need to be in scope for the round-trip).
        let b_scan = SecretKey::from_bytes(&TV1_SCAN_SK).expect("TV1 scan_sk");
        let b_scan_pub = PublicKey::from_bytes(&TV1_SCAN_PK).expect("TV1 scan_pk");
        let b_spend_pub = PublicKey::from_bytes(&TV1_SPEND_PK).expect("TV1 spend_pk");
        let a_sum_sk = ScalarField::from_bytes_raw(TV1_AGGREGATED_SENDER_SK);
        let input_hash = ScalarField::from_bytes_unsigned(TV1_INPUT_HASH);

        // Receiver-side recomputation: construct the tweak_point the receiver
        // sees (input_hash * A_sum), then compute the shared_secret, then
        // derive the expected k=1 puzzle_hash via the Phase 3 chain.
        let a_sum_pub = SecretKey::from_bytes(a_sum_sk.as_bytes())
            .expect("aggregated SK < r")
            .public_key();
        let mut tweak_point = a_sum_pub;
        tweak_point.scalar_multiply(input_hash.as_bytes());
        let expected_shared_secret = compute_shared_secret_from_tweak(&b_scan, &tweak_point);
        let expected_tweak = derive_output_tweak(&expected_shared_secret, 1);
        let expected_onetime_pk = derive_onetime_pk(&b_spend_pub, &expected_tweak);
        let expected_ph = puzzle_hash_for_pk(&expected_onetime_pk);

        // Sender-side derivation under test:
        let sender_ph =
            derive_one_time_puzzle_hash(&b_scan_pub, &b_spend_pub, &a_sum_sk, &input_hash, 1);

        assert_eq!(
            *sender_ph.as_ref(),
            *expected_ph.as_ref(),
            "SEND-01 k=1 round-trip: sender and receiver derivations must agree byte-for-byte"
        );
    }
}
