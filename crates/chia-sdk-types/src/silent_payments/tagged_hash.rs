//! BIP-340-style tagged-hash for CHIP-0057 silent payments.
//!
//! `tagged_hash(tag, data) = SHA256(SHA256(tag) || SHA256(tag) || data)`.
//!
//! Domain-tag constants for the three CHIP-0057 scalars are pinned here.
//! Each tag is hashed at test time by the SHA-256 routine in `chia-sha2`
//! (the SDK-standard hasher — never `sha2::Sha256` in this module tree);
//! the test module pins each `SHA256(tag)` to a `hex!(...)` literal so any
//! typo in a tag-string constant fails the corresponding test before any
//! protocol code runs.

use chia_sha2::Sha256;

/// Tag for the `input_hash` scalar (CHIP-0057 §"Inputs hash").
pub const CHIA_SP_INPUTS: &str = "Chia_SP/Inputs";

/// Tag for the `t_k` output-tweak scalar (CHIP-0057 §"Output tweak").
pub const CHIA_SP_SHARED_SECRET: &str = "Chia_SP/SharedSecret";

/// Tag for the `label_scalar` (CHIP-0057 §"Labels").
pub const CHIA_SP_LABEL: &str = "Chia_SP/Label";

/// Compute `SHA256(SHA256(tag) || SHA256(tag) || data)` using `chia-sha2`.
///
/// This is the BIP-340 tagged-hash construction. `chia-sha2` exposes only
/// `Sha256::{new, update, finalize}` — there is no `::digest(...)` static
/// method. The double-`update(tag_hash)` is intentional per BIP-340.
#[must_use]
pub fn tagged_hash(tag: &str, data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag.as_bytes());
    let tag_hash: [u8; 32] = h.finalize();

    let mut h = Sha256::new();
    h.update(tag_hash);
    h.update(tag_hash);
    h.update(data);
    h.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    /// Single-shot SHA-256 over a byte slice using `chia-sha2`.
    fn sha256(bytes: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize()
    }

    #[test]
    fn tagged_hash_matches_bip340_challenge_vector() {
        // BIP-340 published value:
        //   SHA256(SHA256("BIP0340/challenge") || SHA256("BIP0340/challenge") || "")
        //   = c216d352f5818b7b4beacd4ae0a26fe888080823d2a598856661bcd54f1b3713
        // This cross-implementation vector proves the construction itself is
        // BIP-340-compatible — independent of any Chia-specific tags.
        const EXPECTED: [u8; 32] =
            hex!("c216d352f5818b7b4beacd4ae0a26fe888080823d2a598856661bcd54f1b3713");
        assert_eq!(tagged_hash("BIP0340/challenge", b""), EXPECTED);
    }

    #[test]
    fn tag_inputs_hash_pinned() {
        // Placeholder — Task 2 replaces with actual SHA256(b"Chia_SP/Inputs").
        const EXPECTED: [u8; 32] =
            hex!("0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(sha256(CHIA_SP_INPUTS.as_bytes()), EXPECTED);
    }

    #[test]
    fn tag_shared_secret_hash_pinned() {
        // Placeholder — Task 2 replaces with actual SHA256(b"Chia_SP/SharedSecret").
        const EXPECTED: [u8; 32] =
            hex!("0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(sha256(CHIA_SP_SHARED_SECRET.as_bytes()), EXPECTED);
    }

    #[test]
    fn tag_label_hash_pinned() {
        // Placeholder — Task 2 replaces with actual SHA256(b"Chia_SP/Label").
        const EXPECTED: [u8; 32] =
            hex!("0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(sha256(CHIA_SP_LABEL.as_bytes()), EXPECTED);
    }

    #[test]
    fn different_tags_different_outputs() {
        assert_ne!(tagged_hash("tag1", b"data"), tagged_hash("tag2", b"data"));
    }
}
