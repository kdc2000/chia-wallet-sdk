//! Send-side `Spends` extension: pending state + finish-time entry point.
//!
//! - [`SilentPaymentPending`] holds the deterministic per-output data the
//!   action records at apply time (parent reservation + k + memos).
//! - [`Spends::finish_with_silent_payment_keys`] is the finish-time entry
//!   point that aggregates the sender SKs, computes the `input_hash`, derives
//!   each output's one-time puzzle hash, emits the matching `CreateCoin`
//!   conditions, and relies on the SDK's `Relation::AssertConcurrent` cycle
//!   binding (Phase 04.1) for multi-input atomicity.

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::{Bytes32, Coin};
use chia_puzzle_types::Memos;
use chia_sdk_types::conditions::CreateCoin;
use clvmr::NodePtr;
use indexmap::IndexMap;

use crate::{
    Asset, Deltas, DriverError, Outputs, Relation, SpendContext, Spends,
    silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash},
};

/// Per-output deterministic state recorded at apply time, consumed at finish
/// time by [`Spends::finish_with_silent_payment_keys`] to compute the
/// recipient's one-time puzzle hash and emit the on-chain `CreateCoin`.
///
/// The struct is `pub(crate)` — external callers never construct it directly;
/// they go through `SilentPaymentSend::spend`.
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

impl Spends {
    /// Finish the spend with synthetic-key maps, completing pending
    /// silent-payment outputs.
    ///
    /// 9-step flow (per `04-RESEARCH.md` §4):
    /// 1. If no pending silent payments, delegate to [`Spends::finish_with_keys`].
    /// 2. Collect non-ephemeral XCH input `coin_id`s.
    /// 3. For each non-ephemeral XCH item, look up `secret_keys[item.p2_puzzle_hash()]`;
    ///    if any missing -> `Err(DriverError::SilentPaymentMultiPartyUnsupported)`.
    /// 4. If the collected SK set is empty -> `Err(DriverError::SilentPaymentNoXchInputs)`.
    /// 5. `aggregated_sender_sk = aggregate_sender_sks(&sender_sks)`.
    /// 6. `agg_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes()).public_key()`.
    /// 7. `input_hash = compute_input_hash(&xch_input_ids, &agg_pk)`.
    /// 8. For each pending: derive one-time puzzle hash, emit `CreateCoin` on the
    ///    recorded parent, push the resulting `Coin` to `outputs.xch`.
    /// 9. Delegate to [`Spends::finish_with_keys`] for the standard finish path.
    ///
    /// `synthetic_pks` is the same map [`Spends::finish_with_keys`] takes;
    /// `secret_keys` carries the SK material the silent-payment ECDH
    /// requires (named distinctly from `synthetic_pks` to satisfy
    /// `clippy::similar_names` without a function-level allow attribute —
    /// the type contract is "synthetic SKs keyed by `p2_puzzle_hash`",
    /// matching the action's apply-time write side). The maps are kept
    /// distinct so wallets that split scan-online/spend-offline can pass an
    /// empty SK map when no `SilentPaymentSend` actions are in the batch
    /// (in which case this method short-circuits to `finish_with_keys`).
    ///
    /// Privacy warning: any memos attached to `SilentPaymentSend` actions in
    /// this batch are visible on chain in plaintext and to anyone holding the
    /// recipient's scan key. The 32-byte first-memo hint guard fires at apply
    /// time (`DriverError::SilentPaymentMemoHintForbidden` — Plan 04-05).
    /// `secret_keys` is sensitive synthetic-secret-key material; wallets must
    /// treat the map like the SKs themselves (zeroize on drop, do not log).
    pub fn finish_with_silent_payment_keys(
        mut self,
        ctx: &mut SpendContext,
        deltas: &Deltas,
        relation: Relation,
        synthetic_pks: &IndexMap<Bytes32, PublicKey>,
        secret_keys: &IndexMap<Bytes32, SecretKey>,
    ) -> Result<Outputs, DriverError> {
        // Step 1: empty-pending short-circuit (no silent-payment outputs to emit).
        if self.silent_payments_pending.is_empty() {
            return self.finish_with_keys(ctx, deltas, relation, synthetic_pks);
        }

        // Phase 04.1 input-binding gate: SP multi-input requires Relation::AssertConcurrent
        // for Pass 2b scanner detection. Single-input SP sends accept any Relation.
        let non_ephemeral_xch_count = self.xch.items.iter().filter(|i| !i.ephemeral).count();
        if non_ephemeral_xch_count >= 2 && !matches!(relation, Relation::AssertConcurrent) {
            return Err(DriverError::SilentPaymentRequiresInputBinding);
        }

        // Step 2 + 3: collect XCH input coin ids + verify SK coverage.
        // Iterating non-ephemeral xch.items only: ephemeral items are
        // intermediate coins created within this spend group and are not
        // wallet-controlled inputs whose SKs the sender holds.
        let mut xch_input_ids: Vec<Bytes32> = Vec::with_capacity(self.xch.items.len());
        let mut sender_sks: Vec<SecretKey> = Vec::with_capacity(self.xch.items.len());
        for item in self.xch.items.iter().filter(|i| !i.ephemeral) {
            let ph = item.asset.p2_puzzle_hash();
            let Some(sk) = secret_keys.get(&ph) else {
                return Err(DriverError::SilentPaymentMultiPartyUnsupported);
            };
            sender_sks.push(sk.clone());
            xch_input_ids.push(item.asset.coin_id());
        }

        // Step 4: no-inputs guard. Only fires if every XCH item is ephemeral
        // (e.g. all inputs are intermediate coins) — pathological but
        // possible if a caller mis-constructs Spends.
        if sender_sks.is_empty() {
            return Err(DriverError::SilentPaymentNoXchInputs);
        }

        // Step 5 + 6: aggregate + recover the aggregated PK.
        // Pitfall A (RESEARCH §11): the aggregated PK is recovered via
        // SecretKey::from_bytes round-trip on the ScalarField bytes, NOT by
        // hand-summing the input PKs (which would diverge from the SK sum on
        // mod-r wraparound).
        // Pitfall H: 1/r ≈ 2^-255 vanishing-probability of zero aggregate.
        // The .expect is acceptable per 04-RESEARCH.md §11.
        // (PK local is named `agg_pk` rather than `aggregated_sender_pk` so
        // it differs enough from `aggregated_sender_sk` to keep
        // clippy::similar_names quiet without a function-level allow.)
        let aggregated_sender_sk = aggregate_sender_sks(&sender_sks);
        let agg_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes())
            .expect("ScalarField guarantees < r; zero aggregate has vanishing probability")
            .public_key();

        // Step 7: input_hash binding over lex-min coin_id + aggregated PK.
        let input_hash = compute_input_hash(&xch_input_ids, &agg_pk);

        // Pitfall B (RESEARCH §11) borrow-checker workaround: take ownership
        // of the pending Vec so the per-pending loop below can iterate it
        // while mutating self.xch.items and self.outputs.xch freely. After
        // this take, self.silent_payments_pending is an empty Vec — Step 9's
        // self.finish_with_keys(...) does not re-read it.
        let pending = std::mem::take(&mut self.silent_payments_pending);

        // Step 8: per-pending derivation + CreateCoin emission.
        for p in &pending {
            let ph = derive_one_time_puzzle_hash(
                &p.scan_pk,
                &p.spend_pk,
                &aggregated_sender_sk,
                &input_hash,
                p.k,
            );

            let create_coin = CreateCoin::new(ph, p.amount, p.memos);

            // Emit the CreateCoin condition on the recorded parent. The
            // p.parent_puzzle_hash was captured at apply time via
            // parent.asset.full_puzzle_hash() (matches the SendAction
            // precedent in actions/send.rs:46) and equals
            // self.xch.items[p.parent_xch_index].asset.full_puzzle_hash() at
            // finish time; using the stored value keeps the borrow scope
            // tight and consumes the dead-code-deny on the recorded field.
            let parent = &mut self.xch.items[p.parent_xch_index];
            parent.kind.create_coin_with_assertion(
                ctx,
                p.parent_puzzle_hash,
                &mut self.xch.payment_assertions,
                create_coin,
            );

            // Record the resulting output coin (parent_coin_id was captured at
            // apply time when the parent was selected, before any intermediate
            // ephemeral coins could shift indices).
            self.outputs
                .xch
                .push(Coin::new(p.parent_coin_id, ph, p.amount));
        }

        // Step 9: delegate to the standard finish path. finish_with_keys
        // handles change creation, conditions emission, relation linking,
        // StandardLayer wrapping, and CoinSpend collection.
        self.finish_with_keys(ctx, deltas, relation, synthetic_pks)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chia_bls::SecretKey;
    use chia_puzzle_types::Memos;
    use chia_sdk_test::Simulator;
    use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentNetwork};
    use indexmap::indexmap;

    use crate::{Action, Relation, SpendContext, Spends};

    use super::*;

    /// SEND-03 (Spends-level hard-error) + ROADMAP Phase 4 success criterion #2:
    /// a Spends with 2 non-ephemeral XCH inputs but only 1 in `secret_keys`
    /// returns `Err(DriverError::SilentPaymentMultiPartyUnsupported)` — NOT a
    /// silent single-input aggregation (which would silently corrupt the
    /// puzzle hash). Multi-party flows are out of scope for v1.
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

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin); // wallet-controlled
        spends.add(bob.coin); // counterparty (NOT in synthetic_sks)

        let deltas = spends.apply(
            &mut ctx,
            &[Action::silent_payment_send(recipient, 1, Memos::None)],
        )?;

        // secret_keys (the SK map) contains ONLY Alice — Bob is missing.
        let result = spends.finish_with_silent_payment_keys(
            &mut ctx,
            &deltas,
            Relation::AssertConcurrent,
            &indexmap! { alice.puzzle_hash => alice.pk },
            &indexmap! { alice.puzzle_hash => alice.sk.clone() },
        );

        assert!(
            matches!(result, Err(DriverError::SilentPaymentMultiPartyUnsupported)),
            "expected SilentPaymentMultiPartyUnsupported, got {result:?}"
        );

        Ok(())
    }

    /// FINGERPRINT-01 + ROADMAP §04.1 SC3: a `Spends` with 2 wallet-controlled
    /// XCH inputs + 1 `SilentPaymentSend` action MUST be passed
    /// `Relation::AssertConcurrent` to `finish_with_silent_payment_keys`.
    /// Anything else (including `Relation::None`) returns
    /// `Err(DriverError::SilentPaymentRequiresInputBinding)`.
    ///
    /// The SK-coverage check is NOT triggered by this test: both Alice's and
    /// Bob's SKs are registered in `secret_keys`, so under
    /// `Relation::AssertConcurrent` the call would succeed. The gate fires
    /// before the SK-coverage check because the gate sits between Step 1 and
    /// Step 2 of `finish_with_silent_payment_keys`.
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

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);
        spends.add(bob.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::silent_payment_send(recipient, 1, Memos::None)],
        )?;

        let result = spends.finish_with_silent_payment_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! {
                alice.puzzle_hash => alice.pk,
                bob.puzzle_hash => bob.pk,
            },
            &indexmap! {
                alice.puzzle_hash => alice.sk.clone(),
                bob.puzzle_hash => bob.sk.clone(),
            },
        );

        assert!(
            matches!(result, Err(DriverError::SilentPaymentRequiresInputBinding)),
            "expected SilentPaymentRequiresInputBinding, got {result:?}"
        );

        Ok(())
    }

    /// FINGERPRINT-01 + ROADMAP §04.1 SC3: a `Spends` with 1 XCH input + 1
    /// `SilentPaymentSend` action accepts `Relation::None` — the gate
    /// short-circuits because non-ephemeral XCH count < 2. Single-input SP
    /// sends do not require input binding (the receiver's Pass 2b only needs
    /// linkage to GROUP multiple inputs; a single-input send has nothing to
    /// group).
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

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::silent_payment_send(recipient, 1, Memos::None)],
        )?;

        let result = spends.finish_with_silent_payment_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
            &indexmap! { alice.puzzle_hash => alice.sk.clone() },
        );

        assert!(
            result.is_ok(),
            "single-input SP send with Relation::None should succeed: {result:?}"
        );

        Ok(())
    }
}
