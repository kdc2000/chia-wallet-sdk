//! Silent-payment bech32m address (HRP `spxch` mainnet / `tspxch` testnet)
//! and the network discriminant.

use super::SilentPaymentError;

/// Network discriminator for silent-payment addresses.
///
/// Each network maps to a fixed HRP per CHIP-0057 §153, §206:
/// - `Mainnet` → `"spxch"`
/// - `Testnet` → `"tspxch"`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilentPaymentNetwork {
    Mainnet,
    Testnet,
}

impl SilentPaymentNetwork {
    /// The bech32m human-readable part for this network.
    #[must_use]
    pub fn hrp(self) -> &'static str {
        match self {
            Self::Mainnet => "spxch",
            Self::Testnet => "tspxch",
        }
    }

    /// Parse a bech32m HRP string into a network discriminator. Returns
    /// `Err(SilentPaymentError::WrongHrp(_))` for any value other than
    /// `"spxch"` or `"tspxch"`.
    pub fn from_hrp(hrp: &str) -> Result<Self, SilentPaymentError> {
        match hrp {
            "spxch" => Ok(Self::Mainnet),
            "tspxch" => Ok(Self::Testnet),
            other => Err(SilentPaymentError::WrongHrp(other.to_string())),
        }
    }
}

// SilentPaymentAddress struct + encode/decode + tests are appended to this
// file in Plan 02-03. Keeping them in the same file matches the natural
// type grouping (per RESEARCH §1 / §3 — network is a field of the address).
