//! `SilentPaymentSend` action — composes with the [`Spends`] action system to
//! emit a silent-payment output (CHIP-0057).
//!
//! The action records deterministic per-output data at apply time (parent
//! reservation + per-recipient k-counter + memos) and defers the ECDH math +
//! `CreateCoin` emission to [`Spends::finish_with_silent_payment_keys`]
//! (Plan 04-03). The deferred-finish split is `04-RESEARCH.md §1e`.
//!
//! Privacy warning: memos passed to this action land on chain in plaintext
//! and are visible to anyone holding the recipient's scan key. Do not include
//! sensitive data. A 32-byte first memo would be promoted to a `puzzle_hash`
//! hint by the standard wallet, defeating silent-payment privacy entirely;
//! Plan 04-05 adds the guard that rejects this shape with
//! `DriverError::SilentPaymentMemoHintForbidden`.

use chia_puzzle_types::Memos;
use chia_sdk_utils::silent_payments::SilentPaymentAddress;
use clvmr::NodePtr;

use crate::{
    Asset, BURN_PUZZLE_HASH, Deltas, DriverError, Id, Output, SpendAction, SpendContext, Spends,
    silent_payments::SilentPaymentPending,
};

/// A silent-payment send: emits one on-chain output addressed to `recipient`
/// at an unlinkable one-time puzzle hash.
///
/// Privacy warning: memos are stored on-chain in plaintext and are visible to
/// anyone holding the recipient's scan key. Do not include sensitive data. A
/// 32-byte first memo would be promoted to a `puzzle_hash` hint by the
/// standard wallet, defeating silent-payment privacy entirely;
/// `SilentPaymentSend::spend` rejects this shape with
/// `DriverError::SilentPaymentMemoHintForbidden` (Plan 04-05).
#[derive(Debug, Clone)]
pub struct SilentPaymentSend {
    /// The recipient's silent-payment address (already-parsed `scan_pk` +
    /// `spend_pk` + network). Use `SilentPaymentAddress::decode` at the
    /// caller's edge to parse a bech32m `spxch1...` / `tspxch1...` string.
    pub recipient: SilentPaymentAddress,

    /// The XCH amount in mojos.
    pub amount: u64,

    /// Memos attached to the output's `CreateCoin` condition.
    ///
    /// Privacy warning: memos are on-chain plaintext, visible to anyone with
    /// the recipient's scan key. A 32-byte first memo is rejected by Plan
    /// 04-05's memo-hint guard.
    pub memos: Memos<NodePtr>,
}

impl SilentPaymentSend {
    /// Privacy warning: `memos` is on-chain plaintext, visible to anyone with
    /// the recipient's scan key. A 32-byte first memo is rejected by Plan
    /// 04-05's memo-hint guard.
    #[must_use]
    pub fn new(recipient: SilentPaymentAddress, amount: u64, memos: Memos<NodePtr>) -> Self {
        Self {
            recipient,
            amount,
            memos,
        }
    }
}

impl SpendAction for SilentPaymentSend {
    fn calculate_delta(&self, deltas: &mut Deltas, _index: usize) {
        deltas.update(Id::Xch).output += self.amount;
        deltas.set_needed(Id::Xch);
    }

    /// Privacy warning: this method records the deterministic pieces of a
    /// silent-payment output (parent reservation, k-counter, memos) for
    /// later ECDH-based puzzle-hash derivation at finish time. The
    /// `CreateCoin` condition itself is NOT emitted here — it lands in
    /// `Spends::finish_with_silent_payment_keys` (Plan 04-03).
    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        _index: usize,
    ) -> Result<(), DriverError> {
        // 1. Reserve an XCH parent. `BURN_PUZZLE_HASH` is used as the
        //    placeholder puzzle hash because `output_source` only uses the
        //    `amount` field for source-selection arithmetic
        //    (`fungible_spends.rs:37-51`); the `puzzle_hash` is used by
        //    `is_allowed` which for `ConditionsSpend` is permissive. The
        //    real puzzle hash arrives at finish time. See `04-RESEARCH.md`
        //    §11 Pitfall C for why we avoid `Bytes32::default()` (a
        //    plausible-looking all-zeros puzzle hash that could collide).
        let output = Output::new(BURN_PUZZLE_HASH, self.amount);
        let source = spends.xch.output_source(ctx, &output)?;
        let parent = &spends.xch.items[source];
        let parent_coin_id = parent.asset.coin_id();
        let parent_puzzle_hash = parent.asset.full_puzzle_hash();

        // 2. Increment the per-recipient k-counter on `Spends`. The counter
        //    is keyed by 48-byte compressed `scan_pk` — distinct sub-addresses
        //    (labeled vs unlabeled, m=1 vs m=2, ...) of the same recipient
        //    share the same scan key and increment the same counter. See
        //    `04-RESEARCH.md` §6b for the keying rationale.
        let scan_pk_bytes: [u8; 48] = self.recipient.scan_pk.to_bytes();
        let next_k = spends
            .silent_payment_counters
            .entry(scan_pk_bytes)
            .or_insert(0);
        let k = *next_k;
        *next_k += 1;

        // 3. Record the pending entry. ECDH math, puzzle-hash derivation,
        //    `CreateCoin` emission, and `outputs.xch.push(...)` are all
        //    deferred to `Spends::finish_with_silent_payment_keys` (Plan
        //    04-03).
        spends.silent_payments_pending.push(SilentPaymentPending {
            scan_pk: self.recipient.scan_pk,
            spend_pk: self.recipient.spend_pk,
            parent_xch_index: source,
            parent_coin_id,
            parent_puzzle_hash,
            k,
            amount: self.amount,
            memos: self.memos,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chia_bls::SecretKey;
    use chia_puzzle_types::Memos;
    use chia_sdk_test::Simulator;
    use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentNetwork};

    use crate::{Action, SpendContext, Spends};

    /// SEND-04 (apply-time portion): after `spends.apply(&[Action::silent_payment_send(...)])`,
    /// the action records one entry on `spends.silent_payments_pending` with
    /// the matching fields. ECDH and `CreateCoin` emission are NOT inspected
    /// here — those land in Plan 04-03's tests.
    #[test]
    fn action_state_machine() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        // Build a recipient address from arbitrary BLS keys (the action does
        // not perform ECDH at apply time, so the keys' relationship to any
        // real wallet is irrelevant for this state-machine test).
        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let expected_scan_pk = recipient_scan_sk.public_key();
        let expected_spend_pk = recipient_spend_sk.public_key();
        let recipient = SilentPaymentAddress::new(
            expected_scan_pk,
            expected_spend_pk,
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let _deltas = spends.apply(
            &mut ctx,
            &[Action::silent_payment_send(recipient, 1, Memos::None)],
        )?;

        assert_eq!(
            spends.silent_payments_pending.len(),
            1,
            "exactly one pending entry after one Action::silent_payment_send"
        );

        let pending = &spends.silent_payments_pending[0];
        assert_eq!(pending.scan_pk, expected_scan_pk);
        assert_eq!(pending.spend_pk, expected_spend_pk);
        assert_eq!(pending.amount, 1);
        assert_eq!(pending.k, 0, "k-counter starts at 0");
        assert!(
            pending.parent_xch_index < spends.xch.items.len(),
            "parent_xch_index is a valid index into spends.xch.items"
        );

        // Counter recorded for this scan_pk:
        let scan_pk_bytes: [u8; 48] = expected_scan_pk.to_bytes();
        assert_eq!(
            spends.silent_payment_counters.get(&scan_pk_bytes).copied(),
            Some(1),
            "counter was incremented past the recorded k value"
        );

        Ok(())
    }

    /// SEND-04 (finish-time) + ROADMAP Phase 4 success criterion #1:
    /// the apply+finish flow produces an XCH output whose `puzzle_hash`
    /// matches what Plan-04-01's `derive_one_time_puzzle_hash` independently
    /// computes for the same `(scan_pk, spend_pk, aggregated_sender_sk,
    /// input_hash, k=0)` tuple. This closes the in-driver round-trip; the
    /// full simulator round-trip is Phase 6's concern.
    #[test]
    fn round_trip_matches_derive_one_time_puzzle_hash() -> Result<()> {
        use indexmap::indexmap;

        use crate::{
            Relation,
            silent_payments::{
                aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash,
            },
        };

        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        // Recipient address from arbitrary BLS keys (Pitfall D: in the
        // BlsPair fixture the "synthetic" SK == raw SK; the math closes
        // because both sides of the round-trip use the same interpretation).
        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let recipient = SilentPaymentAddress::new(
            recipient_scan_sk.public_key(),
            recipient_spend_sk.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        // Capture the recipient's keys before the action moves recipient.
        let scan_pk = recipient.scan_pk;
        let spend_pk = recipient.spend_pk;

        // Apply + finish.
        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::silent_payment_send(recipient, 1, Memos::None)],
        )?;

        let outputs = spends.finish_with_silent_payment_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
            &indexmap! { alice.puzzle_hash => alice.sk.clone() },
        )?;

        // Independently compute the expected one-time puzzle hash via the
        // Plan-04-01 free functions (same composition the implementation
        // uses internally; if the implementation diverges, this assertion
        // fires byte-for-byte).
        // Independent aggregation: vec! ensures the slice is freshly owned
        // (avoids clippy::cloned_ref_to_slice_refs that &[alice.sk.clone()]
        // would trigger; functionally identical to passing one SK through
        // aggregate_sender_sks).
        let alice_sks = vec![alice.sk.clone()];
        let aggregated_sender_sk = aggregate_sender_sks(&alice_sks);
        let agg_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes())
            .expect("aggregated SK < r")
            .public_key();
        let input_hash = compute_input_hash(&[alice.coin.coin_id()], &agg_pk);
        let expected_ph =
            derive_one_time_puzzle_hash(&scan_pk, &spend_pk, &aggregated_sender_sk, &input_hash, 0);

        // Assert: at least one xch output matches expected_ph + amount 1.
        // outputs.xch may also include change (alice.coin amount > 1).
        let found = outputs
            .xch
            .iter()
            .any(|c| c.puzzle_hash == expected_ph && c.amount == 1);
        assert!(
            found,
            "expected an output at puzzle_hash {} amount 1; got outputs.xch = {:?}",
            hex::encode(expected_ph),
            outputs.xch
        );

        Ok(())
    }
}
