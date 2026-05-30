//! Per-output deterministic state recorded at apply time of a silent-payment send.
//!
//! - [`SilentPaymentPending`] holds the deterministic per-output data the
//!   apply-time chip-0057 arm of `SendAction::spend` records (parent
//!   reservation + k + memos).
//! - The finish-time derivation pipeline (aggregate SKs, compute `input_hash`,
//!   derive one-time puzzle hash, emit `CreateCoin`) lives in the chip-0057 SP
//!   branch of [`crate::Spends::finish_with_keys`] via the private
//!   `sp_finish_branch` helper in `action_system/spends.rs`.
//! - The CHIP-0057 Pass 2b multi-input atomic binding via
//!   [`crate::Relation::AssertConcurrent`] is enforced from the same finish-time
//!   branch.

use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_puzzle_types::Memos;
use clvmr::NodePtr;

/// Per-output deterministic state recorded at apply time, consumed at finish
/// time by the chip-0057 SP branch of [`crate::Spends::finish_with_keys`] to
/// compute the recipient's one-time puzzle hash and emit the on-chain
/// `CreateCoin`.
///
/// The struct is `pub(crate)` — external callers never construct it directly;
/// they go through `Action::send` with a [`crate::SendDestination::SilentPayment`]
/// destination.
#[derive(Debug, Clone)]
pub(crate) struct SilentPaymentPending {
    pub scan_pk: PublicKey,
    pub spend_pk: PublicKey,
    pub parent_xch_index: usize,
    pub parent_coin_id: Bytes32,
    pub parent_puzzle_hash: Bytes32,
    pub k: u32,
    pub amount: u64,
    pub memos: Memos<NodePtr>,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chia_bls::SecretKey;
    use chia_puzzle_types::Memos;
    use chia_sdk_test::Simulator;
    use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentNetwork};
    use indexmap::indexmap;

    use crate::{Action, DriverError, Id, Relation, SendDestination, SpendContext, Spends};

    /// SEND-03 (Spends-level hard-error) + ROADMAP Phase 4 success criterion #2:
    /// a Spends with 2 non-ephemeral XCH inputs but only 1 in the registered SK
    /// map returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)` — NOT
    /// a silent single-input aggregation (which would silently corrupt the
    /// puzzle hash). Multi-party flows are out of scope for v1.
    ///
    /// Uses `Action::send` with `SendDestination::SilentPayment`, registers
    /// keys via `with_silent_payment_keys`, and finishes via `finish_with_keys`.
    #[test]
    fn multi_party_hard_errors() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(2);
        let bob = sim.bls(3);

        // Recipient address from arbitrary BLS keys.
        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let recipient = SilentPaymentAddress::new(
            recipient_scan_sk.public_key(),
            recipient_spend_sk.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let pk_map = indexmap! { alice.puzzle_hash => alice.pk };
        let sk_map = indexmap! { alice.puzzle_hash => alice.sk.clone() };

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin); // wallet-controlled
        spends.add(bob.coin); // counterparty (NOT in sk_map)

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient.clone())),
                1,
                Memos::None,
            )],
        )?;

        // sk_map contains ONLY Alice — Bob is missing.
        spends.with_silent_payment_keys(pk_map.clone(), sk_map);

        let result =
            spends.finish_with_keys(&mut ctx, &deltas, Relation::AssertConcurrent, &pk_map);

        assert!(
            matches!(result, Err(DriverError::SilentPaymentMultiPartyUnsupported)),
            "expected SilentPaymentMultiPartyUnsupported, got {result:?}"
        );

        Ok(())
    }

    /// FINGERPRINT-01 + ROADMAP §04.1 SC3: a `Spends` with 2 wallet-controlled
    /// XCH inputs + 1 SP `Action::send` MUST be passed `Relation::AssertConcurrent`
    /// to `finish_with_keys`. Anything else (including `Relation::None`)
    /// returns `Err(DriverError::SilentPaymentRequiresInputBinding)`.
    ///
    /// The SK-coverage check is NOT triggered: both Alice's and Bob's SKs are
    /// registered, so under `Relation::AssertConcurrent` the call would
    /// succeed. The input-binding gate is sequenced ahead of the SK-coverage
    /// check inside `sp_finish_branch` so a misconfigured multi-input send
    /// fails fast rather than producing detectable-but-unbound output coins.
    #[test]
    fn multi_input_requires_assert_concurrent_relation() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(5);
        let bob = sim.bls(7);

        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let recipient = SilentPaymentAddress::new(
            recipient_scan_sk.public_key(),
            recipient_spend_sk.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let pk_map = indexmap! {
            alice.puzzle_hash => alice.pk,
            bob.puzzle_hash => bob.pk,
        };
        let sk_map = indexmap! {
            alice.puzzle_hash => alice.sk.clone(),
            bob.puzzle_hash => bob.sk.clone(),
        };

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);
        spends.add(bob.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient.clone())),
                1,
                Memos::None,
            )],
        )?;

        spends.with_silent_payment_keys(pk_map.clone(), sk_map);

        let result = spends.finish_with_keys(&mut ctx, &deltas, Relation::None, &pk_map);

        assert!(
            matches!(result, Err(DriverError::SilentPaymentRequiresInputBinding)),
            "expected SilentPaymentRequiresInputBinding, got {result:?}"
        );

        Ok(())
    }

    /// FINGERPRINT-01 + ROADMAP §04.1 SC3: a `Spends` with 1 XCH input + 1 SP
    /// `Action::send` accepts `Relation::None` — the gate short-circuits
    /// because non-ephemeral XCH count < 2. Single-input SP sends do not
    /// require input binding.
    ///
    /// The success path exercises `sp_finish_branch` end-to-end (gates pass;
    /// derivation pipeline runs).
    #[test]
    fn single_input_accepts_relation_none() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(5);

        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let recipient = SilentPaymentAddress::new(
            recipient_scan_sk.public_key(),
            recipient_spend_sk.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let pk_map = indexmap! { alice.puzzle_hash => alice.pk };
        let sk_map = indexmap! { alice.puzzle_hash => alice.sk.clone() };

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient.clone())),
                1,
                Memos::None,
            )],
        )?;

        spends.with_silent_payment_keys(pk_map.clone(), sk_map);

        let result = spends.finish_with_keys(&mut ctx, &deltas, Relation::None, &pk_map);

        assert!(
            result.is_ok(),
            "single-input SP send with Relation::None should succeed: {result:?}"
        );

        Ok(())
    }
}
