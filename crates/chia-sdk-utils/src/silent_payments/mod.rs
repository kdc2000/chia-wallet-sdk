//! Silent payments (CHIP-0057) — wallet-facing key derivation and bech32m address encoding.
//!
//! This module contains the public types a wallet author uses to:
//!   - derive `(scan_sk, spend_sk)` from a BIP-39 mnemonic at the CHIP-0057
//!     paths `m/12381/8444/12/0` and `m/12381/8444/13/0` ([`SilentPaymentKeys::from_mnemonic`]),
//!   - build a watch-only / key-import setup from raw secret keys
//!     ([`SilentPaymentKeys::from_secret_keys`]),
//!   - encode and decode silent-payment addresses as bech32m with HRP
//!     `spxch` (mainnet) / `tspxch` (testnet) over the 96-byte
//!     `serialize(B_scan) || serialize(B_spend)` payload
//!     ([`SilentPaymentAddress::encode`], [`SilentPaymentAddress::decode`]),
//!   - generate labeled sub-addresses ([`SilentPaymentKeys::labeled_address`])
//!     and maintain a `label_pk → label_index` registry ([`LabelRegistry`])
//!     for the scanner (Phase 3, RECV-04) to attribute labeled detections.
//!
//! All scalar-field reduction in this module flows through
//! `chia_sdk_types::silent_payments::ScalarField` (Phase 1) so the choice of
//! unsigned vs signed byte interpretation is type-system-enforced. Do NOT
//! introduce alternate reducers here. In particular, the signed mod-r reducer
//! that the standard-puzzle synthetic-key offset uses (in
//! `chia_puzzle_types::derive_synthetic`) takes a different sign interpretation
//! and silently disagrees with `from_bytes_unsigned` on inputs whose high bit
//! is set — keep the two routes separate.
//!
//! Hash routines in this module tree use `chia_sha2::Sha256` exclusively (the
//! SDK-standard hasher); the workspace defense-in-depth grep ban forbids
//! `use sha2::` imports under `silent_payments/`.

mod address;
pub use address::*;
mod error;
pub use error::*;
mod keys;
pub use keys::*;
mod labels;
pub use labels::*;
