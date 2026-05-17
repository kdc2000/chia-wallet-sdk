//! `SendDestination` — discriminator for the unified `Action::send` constructor.
//!
//! After Phase 04.2, every send-action target — regular puzzle-hash output OR
//! chip-0057 silent-payment output — flows through `Action::send(id,
//! destination, amount, memos)`. The discriminator is this enum. Sage's
//! pre-existing `Action::send(id, puzzle_hash: Bytes32, ...)` call sites
//! continue to compile unchanged via [`From<Bytes32> for SendDestination`].

use chia_protocol::Bytes32;

#[cfg(feature = "chip-0057")]
use chia_sdk_utils::silent_payments::SilentPaymentAddress;

/// Where a `SendAction` addresses its output: either a literal puzzle hash
/// (the standard case) or a silent-payment address (chip-0057; the recipient
/// publishes one static address and every payment lands at a fresh,
/// unlinkable one-time puzzle hash derived via ECDH).
///
/// Cannot derive `Copy` because `SilentPaymentAddress` is `Clone`-only.
///
/// `impl From<Bytes32>` lets every existing `Action::send(id, ph, amount, memos)`
/// caller continue to compile unchanged after `Action::send`'s second parameter
/// becomes `impl Into<SendDestination>` in Plan 04.2-02.
///
/// Privacy warning: the `SilentPayment` variant carries a [`SilentPaymentAddress`]
/// — any memos attached to the resulting `Action::send` land on chain in
/// plaintext and are visible to anyone holding the recipient's scan key. A
/// 32-byte first memo is rejected at apply time by the relocated
/// memo-hint guard (`DriverError::SilentPaymentMemoHintForbidden`).
#[derive(Debug, Clone)]
pub enum SendDestination {
    PuzzleHash(Bytes32),
    /// Boxed because `SilentPaymentAddress` is ~296 bytes (two BLS pubkeys) and
    /// would dominate the enum's size otherwise (`clippy::large_enum_variant`).
    /// Boxing preserves the type's `Clone` semantics.
    #[cfg(feature = "chip-0057")]
    SilentPayment(Box<SilentPaymentAddress>),
}

impl From<Bytes32> for SendDestination {
    fn from(puzzle_hash: Bytes32) -> Self {
        Self::PuzzleHash(puzzle_hash)
    }
}
