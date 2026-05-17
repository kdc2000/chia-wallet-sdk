use std::{array::TryFromSliceError, num::TryFromIntError};

use chia_sdk_signer::SignerError;
use clvm_traits::{FromClvmError, ToClvmError};
use clvmr::error::EvalErr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriverError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("try from int error")]
    TryFromInt(#[from] TryFromIntError),

    #[error("try from slice error: {0}")]
    TryFromSlice(#[from] TryFromSliceError),

    #[error("failed to serialize clvm value: {0}")]
    ToClvm(#[from] ToClvmError),

    #[error("failed to deserialize clvm value: {0}")]
    FromClvm(#[from] FromClvmError),

    #[error("clvm eval error: {0}")]
    Eval(#[from] EvalErr),

    #[error("invalid mod hash")]
    InvalidModHash,

    #[error("non-standard inner puzzle layer")]
    NonStandardLayer,

    #[error("missing child")]
    MissingChild,

    #[error("missing hint")]
    MissingHint,

    #[error("missing memo")]
    MissingMemo,

    #[error("invalid memo")]
    InvalidMemo,

    #[error("invalid singleton struct")]
    InvalidSingletonStruct,

    #[error("expected even oracle fee, but it was odd")]
    OddOracleFee,

    #[error("custom driver error: {0}")]
    Custom(String),

    #[error("invalid merkle proof")]
    InvalidMerkleProof,

    #[error("unknown puzzle")]
    UnknownPuzzle,

    #[error("invalid spend count for vault subpath")]
    InvalidSubpathSpendCount,

    #[error("missing spend for vault subpath")]
    MissingSubpathSpend,

    #[error("delegated puzzle wrapper conflict")]
    DelegatedPuzzleWrapperConflict,

    #[error("cannot emit conditions from spend")]
    CannotEmitConditions,

    #[error("cannot settle from spend")]
    CannotSettleFromSpend,

    #[error("singleton spend already finalized")]
    AlreadyFinalized,

    #[error("there is no spendable source coin that can create the output without a conflict")]
    NoSourceForOutput,

    #[error("invalid asset id")]
    InvalidAssetId,

    #[error("missing key")]
    MissingKey,

    #[error("missing spend")]
    MissingSpend,

    #[cfg(feature = "offer-compression")]
    #[error("missing compression version prefix")]
    MissingVersionPrefix,

    #[cfg(feature = "offer-compression")]
    #[error("unsupported compression version")]
    UnsupportedVersion,

    #[cfg(feature = "offer-compression")]
    #[error("streamable error: {0}")]
    Streamable(#[from] chia_traits::Error),

    #[cfg(feature = "offer-compression")]
    #[error("cannot decompress uncompressed input")]
    NotCompressed,

    #[cfg(feature = "offer-compression")]
    #[error("decompressed output exceeds maximum allowed size")]
    DecompressionTooLarge,

    #[cfg(feature = "offer-compression")]
    #[error("flate2 error: {0}")]
    Flate2(#[from] flate2::DecompressError),

    #[cfg(feature = "offer-compression")]
    #[error("error when decoding address: {0}")]
    Decode(#[from] chia_sdk_utils::Bech32Error),

    #[error("incompatible asset info")]
    IncompatibleAssetInfo,

    #[error("missing required singleton asset info")]
    MissingAssetInfo,

    #[error("conflicting inputs in offers")]
    ConflictingOfferInputs,

    #[error("signer error: {0}")]
    Signer(#[from] SignerError),

    #[error("missing vault coin spend in transaction reveal")]
    MissingVaultCoinSpend,

    #[cfg(feature = "chip-0057")]
    #[error("silent payment error: {0}")]
    SilentPayment(#[from] chia_sdk_utils::silent_payments::SilentPaymentError),

    #[cfg(feature = "chip-0057")]
    #[error(
        "silent payment requires aggregating synthetic SKs for every input; multi-party flows are unsupported in v1"
    )]
    SilentPaymentMultiPartyUnsupported,

    #[cfg(feature = "chip-0057")]
    #[error("silent payment requires at least one wallet-controlled XCH input")]
    SilentPaymentNoXchInputs,

    #[cfg(feature = "chip-0057")]
    #[error(
        "a 32-byte first memo would be promoted to a puzzle_hash hint by the standard wallet, defeating silent-payment privacy"
    )]
    SilentPaymentMemoHintForbidden,

    #[cfg(feature = "chip-0057")]
    #[error(
        "silent payment multi-input send requires Relation::AssertConcurrent for CHIP-0057 Pass 2b scanner detection; single-input SP sends accept any Relation"
    )]
    SilentPaymentRequiresInputBinding,
}
