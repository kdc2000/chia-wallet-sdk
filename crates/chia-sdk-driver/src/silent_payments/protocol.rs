//! Silent-payments protocol primitives — the building blocks the scanner and
//! the (Phase 4) send-side action share.
//!
//! Five public functions:
//!
//! - [`compute_shared_secret_from_tweak`] — wallet-side ECDH: `SHA256(scan_sk * tweak_point)`.
//! - [`derive_output_tweak`] — per-output tweak scalar: `tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret ‖ ser32(k)) mod r`.
//! - [`derive_onetime_pk`] — `spend_pk + tweak * G`.
//! - [`derive_onetime_sk`] — `(spend_sk + tweak) mod r`.
//! - [`puzzle_hash_for_pk`] — `StandardArgs::curry_tree_hash(pk.derive_synthetic())`.
//!
//! Every scalar that comes out of `tagged_hash` flows through
//! [`chia_sdk_types::silent_payments::ScalarField::from_bytes_unsigned`] — the
//! Phase 1 type-system boundary that prevents the signed-vs-unsigned mixing
//! hazard.

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;
use chia_puzzle_types::DeriveSynthetic;
use chia_puzzle_types::standard::StandardArgs;
use chia_sdk_types::silent_payments::{CHIA_SP_SHARED_SECRET, ScalarField, tagged_hash};
use chia_sha2::Sha256;

/// Compute the wallet-side ECDH shared secret for a pre-computed tweak point.
///
/// `shared_secret = SHA256(serialize(scan_sk * tweak_point))`.
///
/// The 48-byte compressed BLS12-381 G1 serialization is fed to SHA-256
/// directly per CHIP-0057 §169 and `~/silent-payments/crates/sp-common/src/ecdh.rs`.
///
/// `tweak_point` must not be the identity element (the scanner guards this
/// upstream — `PublicKey::is_inf()`); this function is the cheap inner
/// primitive and does not re-check.
#[must_use]
pub fn compute_shared_secret_from_tweak(scan_sk: &SecretKey, tweak_point: &PublicKey) -> [u8; 32] {
    let mut point = *tweak_point;
    point.scalar_multiply(&scan_sk.to_bytes());
    let mut h = Sha256::new();
    h.update(point.to_bytes());
    h.finalize()
}

/// Derive the per-output tweak scalar `t_k` for output index `k` within a
/// spend group.
///
/// `t_k = ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_SHARED_SECRET, shared_secret ‖ ser32(k)))`
///
/// `ser32(k)` is BIG-endian per CHIP-0057 §169. The `to_be_bytes` choice is
/// invisible from TV1/TV3/TV4 (all `k = 0`); the bespoke `k = 1` test in Plan
/// 03-04 catches a `to_le_bytes` regression.
#[must_use]
pub fn derive_output_tweak(shared_secret: &[u8; 32], k: u32) -> ScalarField {
    let mut data = [0u8; 36];
    data[..32].copy_from_slice(shared_secret);
    data[32..].copy_from_slice(&k.to_be_bytes());
    let hash = tagged_hash(CHIA_SP_SHARED_SECRET, &data);
    ScalarField::from_bytes_unsigned(hash)
}

/// Derive the one-time public key for an output: `onetime_pk = spend_pk + tweak * G`.
///
/// The `tweak` is treated as a 32-byte BLS-style secret key by
/// `chia_bls::SecretKey::from_bytes` — this is exactly the canonical encoding
/// `ScalarField::as_bytes` produces, so the round-trip is byte-clean. The
/// `ScalarField::from_bytes_unsigned` boundary guarantees `tweak.as_bytes() < r`.
#[must_use]
pub fn derive_onetime_pk(spend_pk: &PublicKey, tweak: &ScalarField) -> PublicKey {
    let tweak_secret = SecretKey::from_bytes(tweak.as_bytes())
        .expect("ScalarField::from_bytes_unsigned guarantees value < r");
    spend_pk + &tweak_secret.public_key()
}

/// Derive the one-time secret key for an output: `onetime_sk = (spend_sk + tweak) mod r`.
///
/// The `spend_sk` bytes are fed through `ScalarField::from_bytes_raw` (NOT
/// `from_bytes_unsigned`) — `chia_bls::SecretKey` is already constrained to
/// `< r` by construction, so no reduction is needed and we want to preserve
/// the byte pattern exactly. The addition then reduces mod r.
#[must_use]
pub fn derive_onetime_sk(spend_sk: &SecretKey, tweak: &ScalarField) -> SecretKey {
    let sk_scalar = ScalarField::from_bytes_raw(spend_sk.to_bytes());
    let result = sk_scalar.add(tweak);
    SecretKey::from_bytes(result.as_bytes()).expect("ScalarField add stays < r by construction")
}

/// Compute the standard p2 puzzle hash for a one-time public key.
///
/// Reuses `chia_puzzle_types::DeriveSynthetic` + `StandardArgs::curry_tree_hash`
/// — the SDK precedent at `crates/chia-sdk-driver/src/layers/standard_layer.rs:118`.
/// Hand-rolling this puzzle-hash construction is permanently out of scope per
/// REQUIREMENTS.md "Out of Scope (permanently)".
///
/// Note: `pk.derive_synthetic()` routes through `chia_puzzle_types::derive_synthetic`'s
/// SIGNED reducer. That is correct here — the `derive_synthetic` offset is a
/// Chia-consensus-pinned signed reduction, NOT a silent-payments construction.
/// The silent-payments `ScalarField` boundary applies only to the silent-payments
/// output tweak, not the standard-puzzle synthetic offset.
#[must_use]
pub fn puzzle_hash_for_pk(pk: &PublicKey) -> Bytes32 {
    let synthetic = pk.derive_synthetic();
    StandardArgs::curry_tree_hash(synthetic).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    // ─── TV1 pinned bytes (RESEARCH §10a) ──────────────────────────────────

    const TV1_SCAN_SK: [u8; 32] =
        hex!("132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6");
    const TV1_A_SUM: [u8; 48] = hex!(
        "8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c"
        "51b3f2d87dc03a657e07f1828ab760fa"
    );
    const TV1_INPUT_HASH: [u8; 32] =
        hex!("38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411");
    const TV1_SHARED_SECRET: [u8; 32] =
        hex!("d3ac1e8f651a73d2e20b43cb73fd6997de5504afbc04a2d4546a92d0020ba2c6");

    /// Helper: construct TV1's `tweak_point = input_hash * A_sum`.
    fn tv1_tweak_point() -> PublicKey {
        let mut a_sum = PublicKey::from_bytes(&TV1_A_SUM).expect("TV1 A_sum");
        a_sum.scalar_multiply(&TV1_INPUT_HASH);
        a_sum
    }

    fn scan_sk() -> SecretKey {
        SecretKey::from_bytes(&TV1_SCAN_SK).expect("TV1 scan_sk")
    }

    /// RECV-03 + CRYPTO-03: `compute_shared_secret_from_tweak` matches TV1's
    /// pinned value `d3ac1e8f...0ba2c6` — verifies the ECDH primitive byte
    /// for byte.
    #[test]
    fn tv1_shared_secret_matches() {
        let tweak_point = tv1_tweak_point();
        let secret = compute_shared_secret_from_tweak(&scan_sk(), &tweak_point);
        assert_eq!(secret, TV1_SHARED_SECRET);
    }

    /// CRYPTO-03 success criterion 3: an adversarial scalar whose first byte
    /// has the high bit set reduces under UNSIGNED interpretation
    /// (`BigUint::from_bytes_be(&bytes) % r`), NOT signed interpretation.
    ///
    /// Construction: feed `derive_output_tweak` a `shared_secret` of `[0xff; 32]`
    /// and `k = 0`. The resulting `tagged_hash` bytes are deterministic. The
    /// `ScalarField` output has been reduced mod r, so its first byte must be
    /// `< 0x80` (BLS12-381 subgroup order `r` begins with `0x73`, so any value
    /// `< r` has a first byte `<= 0x73 < 0x80`).
    ///
    /// The signed-reduction path (the reducer that `chia_puzzle_types::derive_synthetic`
    /// uses internally for the standard-puzzle synthetic offset) interprets the
    /// leading byte as the sign and produces a different result on inputs whose
    /// first byte has the high bit set. Phase 1's `ScalarField` type boundary is
    /// the prevention mechanism.
    /// This test verifies the boundary fires end-to-end through
    /// `derive_output_tweak`: any future refactor that swaps
    /// `from_bytes_unsigned` for a signed reducer would either (a) produce a
    /// value with high-bit set (since the underlying hash bytes for this input
    /// are likely high-bit-set, and signed routes interpret them as a negative
    /// integer offset), failing the first assertion below, or (b) compile-error
    /// because `ScalarField` has no signed constructor.
    #[test]
    fn adversarial_ff32_scalar_reduces_unsigned() {
        let shared_secret = [0xffu8; 32];
        let tweak = derive_output_tweak(&shared_secret, 0);

        // Sanity: the result IS a ScalarField (< r), so the first byte must
        // be < 0x80 (since r < 2^255 + 2^33 for BLS12-381 — the order is
        // 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001,
        // first byte 0x73). Any output with first byte >= 0x80 would be a
        // signed-reduction failure.
        let bytes = tweak.to_bytes();
        assert!(
            bytes[0] < 0x80,
            "ScalarField output first byte must be < 0x80 (got 0x{:02x}); signed reduction would have produced a different value",
            bytes[0]
        );

        // Stronger pin: re-derive via the same protocol path and assert
        // determinism (catches an accidental rand-injection regression).
        let tweak2 = derive_output_tweak(&shared_secret, 0);
        assert_eq!(tweak.to_bytes(), tweak2.to_bytes());

        // Cross-check against the direct ScalarField::from_bytes_unsigned path:
        // the tweak derived through `derive_output_tweak` must equal
        // `ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_SHARED_SECRET,
        // shared_secret ‖ ser32(0)))`. Any future regression that swapped the
        // unsigned reducer for a signed one would produce a different value.
        let mut data = [0u8; 36];
        data[..32].copy_from_slice(&shared_secret);
        // ser32(0) is four zero bytes; included for clarity.
        data[32..].copy_from_slice(&0u32.to_be_bytes());
        let expected = ScalarField::from_bytes_unsigned(tagged_hash(CHIA_SP_SHARED_SECRET, &data));
        assert_eq!(tweak.to_bytes(), expected.to_bytes());
    }
}
