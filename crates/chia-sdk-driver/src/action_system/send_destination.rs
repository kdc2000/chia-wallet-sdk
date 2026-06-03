//! `SendDestination` — discriminator for the unified `Action::send` constructor.
//!
//! Every send-action target — regular puzzle-hash output OR chip-0057
//! silent-payment output — flows through `Action::send(id, destination,
//! amount, memos)`. The discriminator is this enum. Existing
//! `Action::send(id, puzzle_hash: Bytes32, ...)` call sites continue to
//! compile unchanged via [`From<Bytes32> for SendDestination`].

use chia_protocol::Bytes32;

#[cfg(feature = "chip-0057")]
use chia_sdk_utils::silent_payments::SilentPaymentAddress;

/// Where a `SendAction` addresses its output: either a literal puzzle hash
/// (the standard case) or a silent-payment address (chip-0057; the recipient
/// publishes one static address and every payment lands at a fresh,
/// unlinkable one-time puzzle hash derived via ECDH).
///
/// `impl From<Bytes32>` lets every `Action::send(id, ph, amount, memos)`
/// caller compile unchanged, since `Action::send`'s second parameter is
/// `impl Into<SendDestination>`.
///
/// Privacy warning: the `SilentPayment` variant carries a [`SilentPaymentAddress`]
/// — any memos attached to the resulting `Action::send` land on chain in
/// plaintext and are visible to anyone holding the recipient's scan key. A
/// 32-byte first memo is rejected at apply time by the relocated
/// memo-hint guard (`DriverError::SilentPaymentMemoHintForbidden`).
// With chip-0057 off this collapses to a single `PuzzleHash(Bytes32)` variant
// whose fields are all Copy, so `missing_copy_implementations` fires. An
// unconditional `Copy` derive cannot be used because the chip-0057-on
// `SilentPayment(Box<_>)` variant is not Copy.
#[cfg_attr(not(feature = "chip-0057"), allow(missing_copy_implementations))]
#[derive(Debug, Clone)]
pub enum SendDestination {
    PuzzleHash(Bytes32),
    /// Boxed because `SilentPaymentAddress` is ~296 bytes (two BLS pubkeys) and
    /// would dominate the enum's size otherwise (`clippy::large_enum_variant`).
    /// Boxing preserves the type's `Clone` semantics.
    ///
    /// Privacy warning: memos attached to an `Action::send` with this destination
    /// land on chain in plaintext via the deferred `CreateCoin` emission. They
    /// are visible to anyone holding the recipient's scan key. A 32-byte first
    /// memo is rejected at apply time by `SendAction::spend`'s chip-0057 SP arm
    /// (`DriverError::SilentPaymentMemoHintForbidden`).
    #[cfg(feature = "chip-0057")]
    SilentPayment(Box<SilentPaymentAddress>),
}

impl From<Bytes32> for SendDestination {
    fn from(puzzle_hash: Bytes32) -> Self {
        Self::PuzzleHash(puzzle_hash)
    }
}
