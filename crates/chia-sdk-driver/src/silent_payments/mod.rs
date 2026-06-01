//! Silent payments (CHIP-0057) — wallet-side receive primitive and protocol helpers.
//!
//! This module implements the transport-agnostic scanner described in CHIP-0057.
//! A wallet that receives [`TweakData`] from any source (a CHIP-0058 transport
//! client, the `chia_sdk_test` simulator helper, or a handcrafted fixture) can
//! detect payments addressed to its scan/spend key pair via [`scan_from_tweaks`].
//!
//! The protocol primitives — [`compute_shared_secret_from_tweak`],
//! [`derive_output_tweak`], [`derive_onetime_pk`], [`derive_onetime_sk`],
//! [`puzzle_hash_for_pk`] — are exposed publicly so the send-side action and any
//! caller that needs to compute shared secrets manually can reuse them without
//! round-tripping through the scanner.
//!
//! Forward compatibility with CHIP-0058: [`TweakData`] has no transport fields
//! (no `height`, no `block_hash`, no JSON envelope). A future transport client
//! constructs `TweakData` from its wire messages without breaking this module's
//! shape.
//!
//! All scalar reduction in this module flows through
//! [`chia_sdk_types::silent_payments::ScalarField`], which enforces the
//! unsigned-vs-signed byte-interpretation choice at the type level. See
//! `ScalarField::from_bytes_unsigned` for why unsigned reduction is mandatory
//! for protocol scalars.
//!
//! Hash routines in this module use `chia_sha2::Sha256` exclusively.

mod protocol;
pub use protocol::{
    aggregate_sender_sks, compute_input_hash, compute_shared_secret_from_tweak,
    derive_one_time_puzzle_hash, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,
    puzzle_hash_for_pk,
};
mod block_tweak_data;
pub use block_tweak_data::tweak_data_from_block_spends;
mod scanner;
pub use scanner::{K_MAX_DEFAULT, SilentPaymentScan, scan_from_tweaks};
mod send_keys;
pub(crate) use send_keys::*;
pub use send_keys::{SyntheticPublicKey, SyntheticSecretKey};
mod types;
pub use types::{DetectedSpCoin, OutputMeta, TweakData};
