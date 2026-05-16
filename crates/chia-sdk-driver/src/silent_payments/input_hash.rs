//! Per-spend-group input-hash binding for silent-payment sends.
//!
//! Computes the scalar `input_hash = tagged_hash(CHIA_SP_INPUTS,
//! coin_id_min || serialize(A_sum)) mod r` where `coin_id_min` is the
//! LEXICOGRAPHICALLY-SMALLEST byte-string among the spent coin ids and `A_sum`
//! is the 48-byte compressed serialization of the aggregated sender synthetic
//! public key.
//!
//! Sender and receiver must both compute the same `input_hash` over the same
//! coin-id-set and the same aggregated PK; the receiver assembles the set via
//! CHIP-0057 Pass 2b announcement grouping (opcodes 60/61 — Plan 04-04).
//!
//! See `04-RESEARCH.md` Section 5 for the announcement-binding rationale and
//! Pitfall A for the synthetic-vs-raw PK round-trip caveat (the caller derives
//! `A_sum` from the aggregated synthetic SK via
//! `SecretKey::from_bytes(aggregated_sk.as_bytes()).public_key()`).
//!
//! Lexicographic minimum: byte-string lex order over the 32-byte coin id
//! representations. CHIP-0057 Pass 2a and
//! `~/silent-payments/crates/sp-common/src/ecdh.rs:37-45` both specify this
//! exact ordering. The function's signature accepts any `&[Bytes32]` slice —
//! sorted or unsorted — and selects the min internally; callers do not need
//! to pre-sort.

use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_sdk_types::silent_payments::{CHIA_SP_INPUTS, ScalarField, tagged_hash};

/// Compute the per-spend-group input-hash scalar.
///
/// `coin_ids` is the slice of spent-coin ids forming the spend group; the
/// lexicographically-smallest 32-byte id is selected internally.
/// `aggregated_sender_pk` is the 48-byte compressed serialization of
/// `Sigma synthetic_sk_i * G`, computed at finish time by
/// `Spends::finish_with_silent_payment_keys` (Plan 04-03).
///
/// Returns a [`ScalarField`] reduced unsigned mod-r. The scalar is used both
/// (a) by the sender to derive each output's per-output tweak (via
/// `derive_output_tweak` downstream of the ECDH path); and (b) by the receiver
/// reconstructing the same group via opcode-60/61 announcement linkage.
///
/// # Panics
/// Panics if `coin_ids` is empty. The action-system caller guarantees a
/// non-empty XCH-input set before calling this function — see
/// `DriverError::SilentPaymentNoXchInputs` (Plan 04-03).
///
/// Privacy warning: the `input_hash` scalar is a deterministic public function of
/// the spent coin ids + aggregated sender PK; both are visible on chain after
/// the send. The scalar itself is not sensitive, but it can be re-derived by
/// anyone observing the transaction. The privacy property of silent payments
/// derives from the recipient's scan key, NOT from this scalar's secrecy.
#[must_use]
pub fn compute_input_hash(coin_ids: &[Bytes32], aggregated_sender_pk: &PublicKey) -> ScalarField {
    assert!(
        !coin_ids.is_empty(),
        "compute_input_hash requires at least one coin id"
    );

    let coin_id_min = coin_ids.iter().min().expect("non-empty checked above");
    let pk_bytes = aggregated_sender_pk.to_bytes();

    let mut data = [0u8; 80];
    data[..32].copy_from_slice(coin_id_min.as_ref());
    data[32..].copy_from_slice(&pk_bytes);

    let hash = tagged_hash(CHIA_SP_INPUTS, &data);
    ScalarField::from_bytes_unsigned(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // TV1 input-hash byte-pin. Sourced from Phase 3 `protocol.rs` lines 117-118
    // and `~/silent-payments/crates/sp-common/src/ecdh.rs::test_compute_input_hash_tv1`.
    const TV1_INPUT_HASH: [u8; 32] =
        hex!("38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411");

    // TV1 single coin id (also pinned in Phase 3 `scanner.rs::TV1_COIN_ID`
    // and `~/silent-payments/crates/sp-common/src/ecdh.rs::tv1_coin_id`,
    // which is SHA256(b"test-vector-1-coin")).
    const TV1_COIN_ID: [u8; 32] =
        hex!("5d759d2d97c03b1f6fe0657e91d25f6b7dd1311d6023271a1bcd35978a94a175");

    // TV1 aggregated sender PK (single-input TV1's `A_sum` equals the sole
    // sender synthetic PK). Pinned in Phase 3 `protocol.rs::TV1_A_SUM` and
    // `scanner.rs::TV1_A_SUM`.
    const TV1_A_SUM: [u8; 48] = hex!(
        "8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c"
        "51b3f2d87dc03a657e07f1828ab760fa"
    );

    /// SEND-02 + ROADMAP success criterion #1: TV1 input-hash byte-pin.
    /// Verifies the lex-min `coin_id` + `serialize(A_sum)` || `tagged_hash`
    /// pipeline matches the byte-for-byte CHIP-pinned `38a1c8...cc9411` value.
    #[test]
    fn tv1_compute_input_hash_matches() {
        let pk = PublicKey::from_bytes(&TV1_A_SUM).expect("TV1 A_sum is a valid BLS PK");
        let coin_ids = vec![Bytes32::new(TV1_COIN_ID)];

        let result = compute_input_hash(&coin_ids, &pk);

        assert_eq!(
            *result.as_bytes(),
            TV1_INPUT_HASH,
            "TV1 compute_input_hash must match pinned 38a1c8...cc9411"
        );
    }

    /// SEND-02 lex-min rule: a two-coin input where the smaller id is in
    /// position [1] returns the SAME scalar as a single-element slice with
    /// just the smaller id. Verifies the function selects `iter().min()`,
    /// not `[0]`.
    #[test]
    fn input_hash_uses_lex_min_coin_id() {
        // Arbitrary BLS PK for this property test — use TV1's A_sum.
        let pk = PublicKey::from_bytes(&TV1_A_SUM).expect("TV1 A_sum");

        let coin_a: Bytes32 = [0x01u8; 32].into();
        let coin_b: Bytes32 = [0x02u8; 32].into();
        // Sanity: coin_a < coin_b lexicographically.
        assert!(coin_a < coin_b);

        let with_a_only = compute_input_hash(&[coin_a], &pk);
        let with_both_b_first = compute_input_hash(&[coin_b, coin_a], &pk);

        assert_eq!(
            with_a_only.to_bytes(),
            with_both_b_first.to_bytes(),
            "lex-min coin_id must be selected from a multi-element slice"
        );
    }

    /// SEND-02 order-independence: swapping the slice order of two coin ids
    /// produces the same scalar.
    #[test]
    fn input_hash_order_independent() {
        let pk = PublicKey::from_bytes(&TV1_A_SUM).expect("TV1 A_sum");

        let coin_a: Bytes32 = [0x01u8; 32].into();
        let coin_b: Bytes32 = [0x02u8; 32].into();

        let ab = compute_input_hash(&[coin_a, coin_b], &pk);
        let ba = compute_input_hash(&[coin_b, coin_a], &pk);

        assert_eq!(
            ab.to_bytes(),
            ba.to_bytes(),
            "compute_input_hash must be order-independent"
        );
    }
}
