//! BIP-32-style unhardened derivation paths for CHIP-0057 silent-payment
//! scan and spend keys.
//!
//! ```text
//! m/12381/8444/12/0   scan secret key  (b_scan)
//! m/12381/8444/13/0   spend secret key (b_spend)
//! ```
//!
//! Indices `12` (scan) and `13` (spend) are CHIP-0057 reserved values,
//! distinct from index `2` used by the standard Chia wallet.
//!
//! NOTE: All scalar reduction in CHIP-0057 code paths goes through
//! [`super::ScalarField`]. Do NOT introduce alternate reducers in this
//! module — in particular, the signed reducer used by the standard-puzzle
//! synthetic-key offset (in `chia_puzzle_types::derive_synthetic`) takes
//! a different sign interpretation and silently disagrees with
//! `from_bytes_unsigned` on inputs whose top bit is set.

/// Unhardened derivation path for the silent-payment scan secret key:
/// `m/12381/8444/12/0`.
pub const SCAN_PATH: &[u32] = &[12381, 8444, 12, 0];

/// Unhardened derivation path for the silent-payment spend secret key:
/// `m/12381/8444/13/0`.
pub const SPEND_PATH: &[u32] = &[12381, 8444, 13, 0];
