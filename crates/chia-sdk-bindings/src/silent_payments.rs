//! Silent-payments (chip-0057) binding facade.
//!
//! Per CONTEXT.md D-01, chip-0057 is unconditional on this crate's deps, so
//! the facade has no feature-gating attributes at all. Per D-02, the
//! four free functions (`scan_from_tweaks`, `derive_one_time_puzzle_hash`,
//! `compute_input_hash`, `aggregate_sender_sks`) are exposed as static methods
//! on the zero-field `SilentPayments` namespace class. Per D-03, `ScalarField`
//! is exposed as its own class (NOT type-grouped to `{bytes}`) so the unsigned
//! mod-r invariant is preserved at the FFI boundary.
//!
//! Privacy warning: silent-payment memos and scan keys carry chip-0057 hazards
//! per Phase 4 SEND-08 — wallets holding a scan key see every payment to its
//! address, and memos land on chain in plaintext.

use bindy::Result;
use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;

use crate::Mnemonic;

// ─── SilentPaymentNetwork (unit-variant enum per RESEARCH Pattern 4) ─────

/// Network discriminator for silent-payment addresses (mainnet `spxch` /
/// testnet `tspxch`).
#[derive(Clone, Copy, Debug)]
pub enum SilentPaymentNetwork {
    Mainnet,
    Testnet,
}

impl From<chia_sdk_utils::silent_payments::SilentPaymentNetwork> for SilentPaymentNetwork {
    fn from(value: chia_sdk_utils::silent_payments::SilentPaymentNetwork) -> Self {
        match value {
            chia_sdk_utils::silent_payments::SilentPaymentNetwork::Mainnet => Self::Mainnet,
            chia_sdk_utils::silent_payments::SilentPaymentNetwork::Testnet => Self::Testnet,
        }
    }
}

impl From<SilentPaymentNetwork> for chia_sdk_utils::silent_payments::SilentPaymentNetwork {
    fn from(value: SilentPaymentNetwork) -> Self {
        match value {
            SilentPaymentNetwork::Mainnet => Self::Mainnet,
            SilentPaymentNetwork::Testnet => Self::Testnet,
        }
    }
}

// ─── SilentPaymentAddress (3-field class + encode/decode) ────────────────

/// CHIP-0057 silent-payment bech32m address.
#[derive(Clone)]
pub struct SilentPaymentAddress {
    pub scan_pk: PublicKey,
    pub spend_pk: PublicKey,
    pub network: SilentPaymentNetwork,
}

impl SilentPaymentAddress {
    pub fn encode(&self) -> Result<String> {
        let inner = chia_sdk_utils::silent_payments::SilentPaymentAddress::new(
            self.scan_pk,
            self.spend_pk,
            self.network.into(),
        );
        Ok(inner.encode()?)
    }

    pub fn decode(address: String) -> Result<Self> {
        let inner = chia_sdk_utils::silent_payments::SilentPaymentAddress::decode(&address)?;
        Ok(Self {
            scan_pk: inner.scan_pk,
            spend_pk: inner.spend_pk,
            network: inner.network.into(),
        })
    }
}

impl From<chia_sdk_utils::silent_payments::SilentPaymentAddress> for SilentPaymentAddress {
    fn from(value: chia_sdk_utils::silent_payments::SilentPaymentAddress) -> Self {
        Self {
            scan_pk: value.scan_pk,
            spend_pk: value.spend_pk,
            network: value.network.into(),
        }
    }
}

impl From<SilentPaymentAddress> for chia_sdk_utils::silent_payments::SilentPaymentAddress {
    fn from(value: SilentPaymentAddress) -> Self {
        Self::new(value.scan_pk, value.spend_pk, value.network.into())
    }
}

// ─── SilentPaymentKeys (opaque wrapper with getters + factories) ─────────

/// CHIP-0057 scan + spend key bundle.
///
/// Privacy warning: `scan_sk` lets the holder see every payment to the
/// associated address. Treat as the more sensitive key for at-rest storage.
#[derive(Clone)]
pub struct SilentPaymentKeys(chia_sdk_utils::silent_payments::SilentPaymentKeys);

impl SilentPaymentKeys {
    pub fn from_mnemonic(mnemonic: Mnemonic) -> Result<Self> {
        Ok(Self(
            chia_sdk_utils::silent_payments::SilentPaymentKeys::from_mnemonic(mnemonic.inner()),
        ))
    }

    pub fn from_secret_keys(scan_sk: SecretKey, spend_sk: SecretKey) -> Result<Self> {
        Ok(Self(
            chia_sdk_utils::silent_payments::SilentPaymentKeys::from_secret_keys(scan_sk, spend_sk),
        ))
    }

    pub fn scan_sk(&self) -> Result<SecretKey> {
        Ok(self.0.scan_sk().clone())
    }

    pub fn spend_sk(&self) -> Result<SecretKey> {
        Ok(self.0.spend_sk().clone())
    }

    pub fn scan_pk(&self) -> Result<PublicKey> {
        Ok(*self.0.scan_pk())
    }

    pub fn spend_pk(&self) -> Result<PublicKey> {
        Ok(*self.0.spend_pk())
    }

    pub fn unlabeled_address(
        &self,
        network: SilentPaymentNetwork,
    ) -> Result<SilentPaymentAddress> {
        Ok(self.0.unlabeled_address(network.into()).into())
    }

    pub fn labeled_address(
        &self,
        network: SilentPaymentNetwork,
        m: u32,
    ) -> Result<SilentPaymentAddress> {
        Ok(self.0.labeled_address(network.into(), m)?.into())
    }
}

// ─── LabelRegistry (full register/forward/lookup/len/is_empty API) ───────

#[derive(Clone)]
pub struct LabelRegistry(chia_sdk_utils::silent_payments::LabelRegistry);

impl LabelRegistry {
    pub fn new() -> Result<Self> {
        Ok(Self(chia_sdk_utils::silent_payments::LabelRegistry::new()))
    }

    /// Register label index `m` against scan secret key `scan_sk`.
    pub fn register(&mut self, scan_sk: SecretKey, m: u32) -> Result<()> {
        self.0.register(&scan_sk, m);
        Ok(())
    }

    pub fn forward(&self, m: u32) -> Result<Option<PublicKey>> {
        Ok(self.0.forward(m).copied())
    }

    pub fn lookup(&self, label_pk: PublicKey) -> Result<Option<u32>> {
        Ok(self.0.lookup(&label_pk))
    }

    pub fn len(&self) -> Result<u32> {
        u32::try_from(self.0.len())
            .map_err(|_| bindy::Error::Custom("LabelRegistry length overflows u32".into()))
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.0.is_empty())
    }
}

impl From<chia_sdk_utils::silent_payments::LabelRegistry> for LabelRegistry {
    fn from(value: chia_sdk_utils::silent_payments::LabelRegistry) -> Self {
        Self(value)
    }
}

impl From<LabelRegistry> for chia_sdk_utils::silent_payments::LabelRegistry {
    fn from(value: LabelRegistry) -> Self {
        value.0
    }
}

// ─── OutputMeta (4-field class with auto-generated new) ──────────────────

#[derive(Clone)]
pub struct OutputMeta {
    pub puzzle_hash: Bytes32,
    pub coin_id: Bytes32,
    pub amount: u64,
    pub parent_coin_id: Bytes32,
}

impl From<chia_sdk_driver::OutputMeta> for OutputMeta {
    fn from(value: chia_sdk_driver::OutputMeta) -> Self {
        Self {
            puzzle_hash: value.puzzle_hash,
            coin_id: value.coin_id,
            amount: value.amount,
            parent_coin_id: value.parent_coin_id,
        }
    }
}

impl From<OutputMeta> for chia_sdk_driver::OutputMeta {
    fn from(value: OutputMeta) -> Self {
        Self {
            puzzle_hash: value.puzzle_hash,
            coin_id: value.coin_id,
            amount: value.amount,
            parent_coin_id: value.parent_coin_id,
        }
    }
}

// ─── TweakData (2-field class with auto-generated new) ───────────────────

#[derive(Clone)]
pub struct TweakData {
    pub tweak_points: Vec<PublicKey>,
    pub outputs: Vec<OutputMeta>,
}

impl From<chia_sdk_driver::TweakData> for TweakData {
    fn from(value: chia_sdk_driver::TweakData) -> Self {
        Self {
            tweak_points: value.tweak_points,
            outputs: value.outputs.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<TweakData> for chia_sdk_driver::TweakData {
    fn from(value: TweakData) -> Self {
        Self {
            tweak_points: value.tweak_points,
            outputs: value.outputs.into_iter().map(Into::into).collect(),
        }
    }
}

// ─── DetectedSpCoin (7-field class — return type of scan_from_tweaks) ────

#[derive(Clone)]
pub struct DetectedSpCoin {
    pub coin_id: Bytes32,
    pub puzzle_hash: Bytes32,
    pub amount: u64,
    pub parent_coin_id: Bytes32,
    pub onetime_sk: SecretKey,
    pub k: u32,
    pub label: Option<u32>,
}

impl From<chia_sdk_driver::DetectedSpCoin> for DetectedSpCoin {
    fn from(value: chia_sdk_driver::DetectedSpCoin) -> Self {
        Self {
            coin_id: value.coin_id,
            puzzle_hash: value.puzzle_hash,
            amount: value.amount,
            parent_coin_id: value.parent_coin_id,
            onetime_sk: value.onetime_sk,
            k: value.k,
            label: value.label,
        }
    }
}

// ─── ScalarField (D-03 — class, NOT type-group) ──────────────────────────

/// CHIP-0057 mod-r scalar with unsigned reduction at construction (D-03).
///
/// Use `ScalarField.fromBytes(bytes)` (TS) / `ScalarField.from_bytes(bytes)`
/// (py) to construct from any 32-byte input — the factory reduces mod r so
/// wallet authors cannot accidentally pass an unreduced value into
/// `SilentPayments.deriveOneTimePuzzleHash`.
#[derive(Clone)]
pub struct ScalarField(chia_sdk_types::silent_payments::ScalarField);

impl ScalarField {
    pub fn from_bytes(bytes: Bytes32) -> Result<Self> {
        // D-03 / Pitfall 4: MUST use the unsigned-reducing factory. The
        // unchecked / no-reduction sibling on `ScalarField` is deliberately
        // not surfaced through this facade.
        Ok(Self(
            chia_sdk_types::silent_payments::ScalarField::from_bytes_unsigned(bytes.into()),
        ))
    }

    pub fn to_bytes(&self) -> Result<Bytes32> {
        Ok(Bytes32::new(self.0.to_bytes()))
    }
}

impl From<chia_sdk_types::silent_payments::ScalarField> for ScalarField {
    fn from(value: chia_sdk_types::silent_payments::ScalarField) -> Self {
        Self(value)
    }
}

impl From<ScalarField> for chia_sdk_types::silent_payments::ScalarField {
    fn from(value: ScalarField) -> Self {
        value.0
    }
}

// ─── SilentPayments (zero-field namespace with 4 statics per D-02) ───────

/// Static-functions namespace per D-02. Hosts the four free-fn primitives.
#[derive(Clone)]
pub struct SilentPayments;

impl SilentPayments {
    /// Detect silent-payment outputs in a `TweakData` blob.
    ///
    /// Privacy warning: requires the scan secret key — anyone with this key
    /// sees every payment to the wallet.
    //
    // Facade param names follow the `b_scan` / `b_spend` / `b_spend_pub`
    // shorthand established by `chia_sdk_driver::silent_payments::scanner.rs`
    // tests to avoid clippy::similar_names without an `#[allow]` attribute.
    // The bindy JSON declares the public arg names (`scan_sk`, `spend_sk`,
    // `spend_pk`) which become the call-site names; bindy passes them
    // positionally so the facade is free to rename internally.
    pub fn scan_from_tweaks(
        b_scan: SecretKey,
        b_spend: SecretKey,
        b_spend_pub: PublicKey,
        data: TweakData,
        labels: LabelRegistry,
        k_max: u32,
    ) -> Result<Vec<DetectedSpCoin>> {
        let driver_data: chia_sdk_driver::TweakData = data.into();
        let driver_labels: chia_sdk_utils::silent_payments::LabelRegistry = labels.into();
        let detections = chia_sdk_driver::scan_from_tweaks(
            &b_scan,
            &b_spend,
            &b_spend_pub,
            &driver_data,
            Some(&driver_labels),
            k_max as usize,
        );
        Ok(detections.into_iter().map(Into::into).collect())
    }

    pub fn derive_one_time_puzzle_hash(
        b_scan_pub: PublicKey,
        b_spend_pub: PublicKey,
        aggregated_sender_sk: ScalarField,
        input_hash: ScalarField,
        k: u32,
    ) -> Result<Bytes32> {
        let agg: chia_sdk_types::silent_payments::ScalarField = aggregated_sender_sk.into();
        let ih: chia_sdk_types::silent_payments::ScalarField = input_hash.into();
        Ok(chia_sdk_driver::derive_one_time_puzzle_hash(
            &b_scan_pub,
            &b_spend_pub,
            &agg,
            &ih,
            k,
        ))
    }

    pub fn compute_input_hash(
        coin_ids: Vec<Bytes32>,
        aggregated_sender_pk: PublicKey,
    ) -> Result<ScalarField> {
        Ok(chia_sdk_driver::compute_input_hash(&coin_ids, &aggregated_sender_pk).into())
    }

    pub fn aggregate_sender_sks(sks: Vec<SecretKey>) -> Result<ScalarField> {
        Ok(chia_sdk_driver::aggregate_sender_sks(&sks).into())
    }
}
