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

use chia_bls::PublicKey;
use chia_protocol::Bytes;

use crate::Bech32;

/// A CHIP-0057 silent-payment address: bech32m-encoded
/// `serialize(B_scan) || serialize(B_spend)` (96 bytes) under HRP
/// `spxch` (mainnet) / `tspxch` (testnet).
///
/// The address carries no labeled-vs-unlabeled discriminant: per CHIP §375,
/// a labeled address has its `spend_pk` field set to `B_m = B_spend + label_pk`,
/// but the wire form is identical to an unlabeled address with the same
/// underlying point. Senders and scanners cannot distinguish; recipients
/// disambiguate via a registered [`super::LabelRegistry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SilentPaymentAddress {
    pub scan_pk: PublicKey,
    pub spend_pk: PublicKey,
    pub network: SilentPaymentNetwork,
}

impl SilentPaymentAddress {
    /// Construct a `SilentPaymentAddress` from raw pubkeys + network.
    ///
    /// Does NOT validate the pubkeys against the identity element — that
    /// check is performed on `decode` (where untrusted bytes enter the system).
    /// `new` is the trusted constructor used by `SilentPaymentKeys::unlabeled_address`
    /// and `SilentPaymentKeys::labeled_address`, where the pubkeys come from
    /// `SecretKey::public_key()` and are therefore non-identity by construction.
    #[must_use]
    pub fn new(scan_pk: PublicKey, spend_pk: PublicKey, network: SilentPaymentNetwork) -> Self {
        Self {
            scan_pk,
            spend_pk,
            network,
        }
    }

    /// Encode as bech32m: HRP `||` "1" `||` base32(scan_pk `||` spend_pk `||` checksum).
    ///
    /// The payload is the 96-byte concatenation `serialize(B_scan) || serialize(B_spend)`.
    pub fn encode(&self) -> Result<String, SilentPaymentError> {
        let mut payload = Vec::with_capacity(96);
        payload.extend_from_slice(&self.scan_pk.to_bytes());
        payload.extend_from_slice(&self.spend_pk.to_bytes());
        debug_assert_eq!(payload.len(), 96);

        let bech = Bech32::new(Bytes::new(payload), self.network.hrp().to_string());
        Ok(bech.encode()?)
    }

    /// Decode a bech32m silent-payment address.
    ///
    /// Rejects:
    ///   - HRP not in `{"spxch", "tspxch"}` → `SilentPaymentError::WrongHrp`.
    ///   - bech32 / bech32m parse failure → `SilentPaymentError::Bech32(_)`.
    ///   - non-bech32m variant (plain bech32) → `SilentPaymentError::Bech32(Bech32Error::InvalidFormat)`.
    ///   - payload length != 96 → `SilentPaymentError::PayloadLength(N)`.
    ///   - either pubkey half is invalid bytes → `SilentPaymentError::InvalidPublicKey`.
    ///   - either pubkey half is the identity element → `SilentPaymentError::IdentityPublicKey`.
    pub fn decode(s: &str) -> Result<Self, SilentPaymentError> {
        let bech = Bech32::decode(s)?;
        let network = SilentPaymentNetwork::from_hrp(&bech.prefix)?;
        let payload: Vec<u8> = bech.data.to_vec();
        if payload.len() != 96 {
            return Err(SilentPaymentError::PayloadLength(payload.len()));
        }
        let scan_bytes: [u8; 48] = payload[..48].try_into().expect("96 == 48 + 48");
        let spend_bytes: [u8; 48] = payload[48..].try_into().expect("96 == 48 + 48");
        let scan_pk = PublicKey::from_bytes(&scan_bytes)
            .map_err(|_| SilentPaymentError::InvalidPublicKey)?;
        let spend_pk = PublicKey::from_bytes(&spend_bytes)
            .map_err(|_| SilentPaymentError::InvalidPublicKey)?;
        if scan_pk.is_inf() || spend_pk.is_inf() {
            return Err(SilentPaymentError::IdentityPublicKey);
        }
        Ok(Self {
            scan_pk,
            spend_pk,
            network,
        })
    }
}
