//! Wire-protocol types for the silent-payment scanner.
//!
//! These three structs are the transport-agnostic surface a wallet sees:
//!
//! - [`TweakData`] is the scanner's input — pre-computed per-spend-group tweak
//!   points + candidate output metadata. Constructed by an indexer adapter (a
//!   CHIP-0058 transport client, the Phase-6 simulator helper, etc.) but
//!   carries no transport fields.
//! - [`OutputMeta`] is the metadata for one candidate coin — the puzzle hash
//!   the scanner matches against, plus the bookkeeping fields the wallet needs
//!   to act on a detected coin without re-parsing the block.
//! - [`DetectedSpCoin`] is the scanner's output — one entry per detected coin,
//!   carrying enough information for the wallet to compose a follow-on spend.

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;

/// Transport-agnostic input to the silent-payment scanner.
///
/// `tweak_points` are the pre-computed per-spend-group ECDH multipliers
/// `tweak_point[i] = input_hash[i] * A_sum[i]` (the indexer or sender computes
/// these on the send/index side; the scanner consumes them as-is).
/// `outputs` are the candidate coin metadata to scan against — typically all
/// outputs in a single block, but the primitive does not care about block
/// boundaries.
///
/// No transport fields: no `height`, no `block_hash`, no JSON envelope. A
/// future CHIP-0058 transport client constructs `TweakData` from its wire
/// messages without breaking this struct's shape.
#[derive(Clone, Debug)]
pub struct TweakData {
    pub tweak_points: Vec<PublicKey>,
    pub outputs: Vec<OutputMeta>,
}

/// Coin metadata the scanner needs to identify and report a detected
/// silent-payment coin.
///
/// `puzzle_hash` is the field the scanner matches against; the remaining fields
/// flow through to the returned [`DetectedSpCoin`] so the wallet can act on the
/// coin without re-parsing the block.
#[derive(Clone, Copy, Debug)]
pub struct OutputMeta {
    pub puzzle_hash: Bytes32,
    pub coin_id: Bytes32,
    pub amount: u64,
    pub parent_coin_id: Bytes32,
}

/// A silent-payment coin detected by `scan_from_tweaks`, carrying enough
/// information for the wallet to immediately compose a follow-on spend.
#[derive(Clone, Debug)]
pub struct DetectedSpCoin {
    pub coin_id: Bytes32,
    pub puzzle_hash: Bytes32,
    pub amount: u64,
    pub parent_coin_id: Bytes32,
    /// The one-time secret key for this output — `(b_spend + t_k) mod r` for
    /// unlabeled detections; `(b_spend + t_k + label_scalar) mod r` for labeled.
    pub onetime_sk: SecretKey,
    /// The `k` counter at which this output was detected within its spend group.
    pub k: u32,
    /// `None` for unlabeled detections; `Some(m)` for label-index `m`.
    pub label: Option<u32>,
}

#[cfg(test)]
mod tests {
    use chia_bls::PublicKey;

    /// Defensive test: malformed 48-byte pubkey bytes are rejected at
    /// `PublicKey::from_bytes`, not inside the scanner. Documents the
    /// deserialization boundary referenced by `03-RESEARCH.md` §11 test #8.
    #[test]
    fn malformed_pubkey_caught_at_deserialization() {
        let result = PublicKey::from_bytes(&[0xff; 48]);
        assert!(
            result.is_err(),
            "PublicKey::from_bytes(&[0xff; 48]) must reject"
        );
    }
}
