use chia_protocol::Coin;
use chia_puzzle_types::Memos;
use chia_sdk_types::conditions::CreateCoin;

#[cfg(feature = "chip-0057")]
use chia_sdk_utils::silent_payments::SilentPaymentAddress;

use crate::{
    Asset, Deltas, DriverError, Id, Output, SendDestination, SingletonDestination, SpendAction,
    SpendContext, Spends,
};
#[cfg(feature = "chip-0057")]
use crate::{BURN_PUZZLE_HASH, silent_payments::SilentPaymentPending};

#[derive(Debug, Clone)]
pub struct SendAction {
    pub id: Id,
    pub destination: SendDestination,
    pub amount: u64,
    pub memos: Memos,
}

impl SendAction {
    pub fn new(id: Id, destination: SendDestination, amount: u64, memos: Memos) -> Self {
        Self {
            id,
            destination,
            amount,
            memos,
        }
    }
}

impl SpendAction for SendAction {
    fn calculate_delta(&self, deltas: &mut Deltas, _index: usize) {
        deltas.update(self.id).output += self.amount;
        deltas.set_needed(self.id);
    }

    fn spend(
        &self,
        ctx: &mut SpendContext,
        spends: &mut Spends,
        _index: usize,
    ) -> Result<(), DriverError> {
        // chip-0057 SP arm: SilentPayment destination on Xch.
        // Pre-emption order: SilentPaymentRequiresXch (Id check) BEFORE memo-hint
        // guard BEFORE parent reservation. Fires the cheapest guard first.
        #[cfg(feature = "chip-0057")]
        if let SendDestination::SilentPayment(addr) = &self.destination {
            if !matches!(self.id, Id::Xch) {
                return Err(DriverError::SilentPaymentRequiresXch);
            }
            memo_hint_guard(ctx, self.memos)?;
            return spend_silent_payment(ctx, spends, addr, self.amount, self.memos);
        }

        // PuzzleHash destination — exhaustive extraction. Under chip-0057 the
        // SilentPayment(_) arm is statically handled above; the early return
        // makes the post-handled match exhaustiveness arm unreachable!().
        let puzzle_hash = match &self.destination {
            SendDestination::PuzzleHash(ph) => *ph,
            #[cfg(feature = "chip-0057")]
            SendDestination::SilentPayment(_) => unreachable!("handled above"),
        };

        let output = Output::new(puzzle_hash, self.amount);
        let create_coin = CreateCoin::new(puzzle_hash, self.amount, self.memos);

        if matches!(self.id, Id::Xch) {
            let source = spends.xch.output_source(ctx, &output)?;
            let parent = &mut spends.xch.items[source];
            let parent_puzzle_hash = parent.asset.full_puzzle_hash();

            parent.kind.create_coin_with_assertion(
                ctx,
                parent_puzzle_hash,
                &mut spends.xch.payment_assertions,
                create_coin,
            );

            let coin = Coin::new(
                parent.asset.coin_id(),
                create_coin.puzzle_hash,
                create_coin.amount,
            );

            spends.outputs.xch.push(coin);
        } else if let Some(cat) = spends.cats.get_mut(&self.id) {
            let source = cat.output_source(ctx, &output)?;
            let parent = &mut cat.items[source];
            let parent_puzzle_hash = parent.asset.full_puzzle_hash();

            parent.kind.create_coin_with_assertion(
                ctx,
                parent_puzzle_hash,
                &mut cat.payment_assertions,
                create_coin,
            );

            let cat = parent
                .asset
                .child(create_coin.puzzle_hash, create_coin.amount);

            spends.outputs.cats.entry(self.id).or_default().push(cat);
        } else if let Some(did) = spends.dids.get_mut(&self.id) {
            let source = did.last_mut()?;
            source.child_info.destination = Some(SingletonDestination::CreateCoin(create_coin));
        } else if let Some(nft) = spends.nfts.get_mut(&self.id) {
            let source = nft.last_mut()?;
            source.child_info.destination = Some(create_coin);
        } else if let Some(option) = spends.options.get_mut(&self.id) {
            let source = option.last_mut()?;
            source.child_info.destination = Some(SingletonDestination::CreateCoin(create_coin));
        } else {
            return Err(DriverError::InvalidAssetId);
        }

        Ok(())
    }
}

/// Apply-time half of a chip-0057 silent-payment send: reserves an XCH parent,
/// increments the per-`scan_pk` k counter on `Spends`, and pushes a
/// `SilentPaymentPending` entry. NO `CreateCoin` is emitted here — the
/// on-chain output is emitted at finish time by the chip-0057 SP branch of
/// [`Spends::finish_with_keys`], which derives the one-time puzzle hash from
/// the recorded entry.
///
/// Privacy warning: memos passed here land in the on-chain `CreateCoin.memos`
/// field unchanged at finish time and are visible to anyone holding the
/// recipient's scan key. The 32-byte first-memo hint guard
/// (`DriverError::SilentPaymentMemoHintForbidden`) is fired by the caller
/// before this helper runs.
#[cfg(feature = "chip-0057")]
fn spend_silent_payment(
    ctx: &mut SpendContext,
    spends: &mut Spends,
    recipient: &SilentPaymentAddress,
    amount: u64,
    memos: Memos,
) -> Result<(), DriverError> {
    // 1. Reserve XCH parent. BURN_PUZZLE_HASH is the placeholder puzzle hash
    //    because output_source only uses `amount` for source-selection
    //    arithmetic; the real puzzle hash arrives at finish time. Avoiding
    //    Bytes32::default() prevents a plausible-looking all-zeros collision.
    let output = Output::new(BURN_PUZZLE_HASH, amount);
    let source = spends.xch.output_source(ctx, &output)?;
    let parent = &spends.xch.items[source];
    let parent_coin_id = parent.asset.coin_id();
    let parent_puzzle_hash = parent.asset.full_puzzle_hash();

    // 2. Per-scan_pk k counter. Keyed by 48-byte compressed scan_pk so
    //    distinct sub-addresses (labeled vs unlabeled) of the same recipient
    //    share a counter.
    let scan_pk_bytes: [u8; 48] = recipient.scan_pk.to_bytes();
    let next_k = spends
        .silent_payment_counters
        .entry(scan_pk_bytes)
        .or_insert(0);
    let k = *next_k;
    *next_k += 1;

    // 3. Push pending entry. ECDH math + CreateCoin emission + outputs.xch
    //    push are all deferred to the chip-0057 SP branch of
    //    Spends::finish_with_keys.
    spends.silent_payments_pending.push(SilentPaymentPending {
        scan_pk: recipient.scan_pk,
        spend_pk: recipient.spend_pk,
        parent_xch_index: source,
        parent_coin_id,
        parent_puzzle_hash,
        k,
        amount,
        memos,
    });

    Ok(())
}

/// Reject a 32-byte first memo that would be promoted to a `puzzle_hash` hint
/// by the standard Chia wallet, defeating silent-payment privacy.
///
/// Privacy warning: the standard wallet (Sage, the mainnet wallet) treats a
/// 32-byte first memo as a `puzzle_hash` hint and indexes the output by that
/// hash — exposing the one-time puzzle hash to any indexer. For silent
/// payments, this completely defeats the privacy gain.
///
/// Returns `Err(DriverError::SilentPaymentMemoHintForbidden)` if the first
/// memo atom is exactly 32 bytes. All other memo shapes (`Memos::None`, non-pair,
/// non-atom head, first atom != 32 bytes, malformed) return `Ok(())`.
#[cfg(feature = "chip-0057")]
fn memo_hint_guard(ctx: &SpendContext, memos: Memos) -> Result<(), DriverError> {
    use clvmr::SExp;

    let Memos::Some(ptr) = memos else {
        return Ok(());
    };

    let SExp::Pair(head, _tail) = ctx.sexp(ptr) else {
        return Ok(());
    };

    let SExp::Atom = ctx.sexp(head) else {
        return Ok(());
    };

    let atom = ctx.atom(head);
    if atom.as_ref().len() == 32 {
        return Err(DriverError::SilentPaymentMemoHintForbidden);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chia_protocol::{Bytes32, Coin};
    use chia_puzzle_types::standard::StandardArgs;
    use chia_sdk_test::{BlsPair, Simulator};
    use indexmap::indexmap;
    use rstest::rstest;

    use crate::{Action, Cat, Relation};

    use super::*;

    #[test]
    fn test_action_send_xch() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(Id::Xch, alice.puzzle_hash, 1, Memos::None)],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        let coin = outputs.xch[0];
        assert_eq!(outputs.xch.len(), 1);
        assert_ne!(sim.coin_state(coin.coin_id()), None);
        assert_eq!(coin.amount, 1);

        Ok(())
    }

    #[test]
    fn test_action_send_xch_with_change() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(5);
        let bob = BlsPair::new(0);
        let bob_puzzle_hash = StandardArgs::curry_tree_hash(bob.pk).into();

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(Id::Xch, bob_puzzle_hash, 2, Memos::None)],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        assert_eq!(outputs.xch.len(), 2);

        let change = outputs.xch[0];
        assert_ne!(sim.coin_state(change.coin_id()), None);
        assert_eq!(change.amount, 2);
        assert_eq!(change.puzzle_hash, bob_puzzle_hash);

        let coin = outputs.xch[1];
        assert_ne!(sim.coin_state(coin.coin_id()), None);
        assert_eq!(coin.amount, 3);
        assert_eq!(coin.puzzle_hash, alice.puzzle_hash);

        Ok(())
    }

    #[test]
    fn test_action_send_xch_split() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(3);

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[
                Action::send(Id::Xch, alice.puzzle_hash, 1, Memos::None),
                Action::send(Id::Xch, alice.puzzle_hash, 1, Memos::None),
                Action::send(Id::Xch, alice.puzzle_hash, 1, Memos::None),
            ],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        assert_eq!(outputs.xch.len(), 3);

        let coins: Vec<Coin> = outputs
            .xch
            .iter()
            .copied()
            .filter(|coin| {
                sim.coin_state(coin.coin_id())
                    .expect("missing coin")
                    .spent_height
                    .is_none()
            })
            .collect();

        assert_eq!(coins.len(), 3);

        for coin in coins {
            assert_eq!(coin.puzzle_hash, alice.puzzle_hash);
            assert_eq!(coin.amount, 1);
        }

        Ok(())
    }

    #[rstest]
    #[case::normal(None)]
    #[case::revocable(Some(Bytes32::default()))]
    fn test_action_send_cat(#[case] hidden_puzzle_hash: Option<Bytes32>) -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);
        let hint = ctx.hint(alice.puzzle_hash)?;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[
                Action::single_issue_cat(hidden_puzzle_hash, 1),
                Action::send(Id::New(0), alice.puzzle_hash, 1, hint),
            ],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        let cat = outputs.cats[&Id::New(0)][0];
        assert_ne!(sim.coin_state(cat.coin.coin_id()), None);
        assert_eq!(cat.coin.amount, 1);

        Ok(())
    }

    #[rstest]
    #[case::normal(None)]
    #[case::revocable(Some(Bytes32::default()))]
    fn test_action_send_cat_with_change(#[case] hidden_puzzle_hash: Option<Bytes32>) -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(5);
        let bob = BlsPair::new(0);
        let bob_puzzle_hash = StandardArgs::curry_tree_hash(bob.pk).into();
        let bob_hint = ctx.hint(bob_puzzle_hash)?;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[
                Action::single_issue_cat(hidden_puzzle_hash, 5),
                Action::send(Id::New(0), bob_puzzle_hash, 2, bob_hint),
            ],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        let cats = &outputs.cats[&Id::New(0)];
        assert_eq!(cats.len(), 2);

        let change = cats[0];
        assert_ne!(sim.coin_state(change.coin.coin_id()), None);
        assert_eq!(change.coin.amount, 2);
        assert_eq!(change.info.p2_puzzle_hash, bob_puzzle_hash);

        let cat = cats[1];
        assert_ne!(sim.coin_state(cat.coin.coin_id()), None);
        assert_eq!(cat.coin.amount, 3);
        assert_eq!(cat.info.p2_puzzle_hash, alice.puzzle_hash);

        Ok(())
    }

    #[rstest]
    #[case::normal(None)]
    #[case::revocable(Some(Bytes32::default()))]
    fn test_action_send_cat_split(#[case] hidden_puzzle_hash: Option<Bytes32>) -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(3);
        let hint = ctx.hint(alice.puzzle_hash)?;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[
                Action::single_issue_cat(hidden_puzzle_hash, 3),
                Action::send(Id::New(0), alice.puzzle_hash, 1, hint),
                Action::send(Id::New(0), alice.puzzle_hash, 1, hint),
                Action::send(Id::New(0), alice.puzzle_hash, 1, hint),
            ],
        )?;

        let outputs = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None,
            &indexmap! { alice.puzzle_hash => alice.pk },
        )?;

        sim.spend_coins(ctx.take(), &[alice.sk])?;

        let cats = &outputs.cats[&Id::New(0)];
        assert_eq!(cats.len(), 3);

        let cats: Vec<Cat> = cats
            .iter()
            .copied()
            .filter(|cat| {
                sim.coin_state(cat.coin.coin_id())
                    .expect("missing coin")
                    .spent_height
                    .is_none()
            })
            .collect();

        assert_eq!(cats.len(), 3);

        for cat in cats {
            assert_eq!(cat.info.p2_puzzle_hash, alice.puzzle_hash);
            assert_eq!(cat.coin.amount, 1);
        }

        Ok(())
    }
}

#[cfg(all(test, feature = "chip-0057"))]
mod silent_payment_tests {
    use anyhow::Result;
    use chia_bls::SecretKey;
    use chia_puzzle_types::Memos;
    use chia_sdk_test::Simulator;
    use chia_sdk_utils::silent_payments::{SilentPaymentAddress, SilentPaymentNetwork};
    use indexmap::indexmap;

    use crate::{
        Action, DriverError, Id, Relation, SendDestination, SpendContext, Spends,
        silent_payments::{aggregate_sender_sks, compute_input_hash, derive_one_time_puzzle_hash},
    };

    // ====================================================================
    // 8 RELOCATED TESTS — from actions/silent_payment_send.rs::tests (deleted).
    // Each test body is rewritten from the OLD API to the NEW API:
    //   Action::silent_payment_send(recipient, amount, memos)
    //     -> Action::send(Id::Xch, SendDestination::SilentPayment(Box::new(recipient)), amount, memos)
    //   spends.finish_with_silent_payment_keys(ctx, deltas, rel, &pk_map, &sk_map)
    //     -> spends.with_silent_payment_keys(pk_map.clone(), sk_map);
    //        spends.finish_with_keys(ctx, deltas, rel, &pk_map)
    // Test names UNCHANGED — VALIDATION.md grep matchers depend on them.
    // ====================================================================

    /// SEND-04 (apply-time portion): after `spends.apply(&[Action::send(
    /// Id::Xch, SendDestination::SilentPayment(...), ...)])`, the action
    /// records one entry on `spends.silent_payments_pending` with the
    /// matching fields. ECDH and `CreateCoin` emission are NOT inspected
    /// here.
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
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        )?;

        assert_eq!(
            spends.silent_payments_pending.len(),
            1,
            "exactly one pending entry after one Action::send(SilentPayment, ...)"
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
    /// matches what `derive_one_time_puzzle_hash` independently computes for
    /// the same `(scan_pk, spend_pk, aggregated_sender_sk, input_hash, k=0)`
    /// tuple. Re-validated under the unified `Action::send` +
    /// `Spends::finish_with_keys` construction shape (Phase 04.2).
    #[test]
    fn round_trip_matches_derive_one_time_puzzle_hash() -> Result<()> {
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

        // Capture the recipient's keys before the address moves into the action.
        let scan_pk = recipient.scan_pk;
        let spend_pk = recipient.spend_pk;

        // Apply + finish via the new unified API.
        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        )?;

        let pk_map = indexmap! { alice.puzzle_hash => alice.pk };
        let sk_map = indexmap! { alice.puzzle_hash => alice.sk.clone() };
        spends.with_silent_payment_keys(pk_map.clone(), sk_map);

        let outputs = spends.finish_with_keys(&mut ctx, &deltas, Relation::None, &pk_map)?;

        // Independently compute the expected one-time puzzle hash via the
        // free functions (Vec intermediate to satisfy clippy on slice
        // construction; functionally identical to passing one SK through
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

    /// SEND-05 + ROADMAP Phase 4 success criterion #3: two `Action::send`
    /// calls with the same SP recipient in one `Spends` produce outputs at
    /// k=0 and k=1 respectively. The counter on
    /// `spends.silent_payment_counters` increments per `scan_pk`.
    #[test]
    fn multi_output_same_scan_pk_increments_k() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(10);

        let recipient_scan_sk = SecretKey::from_bytes(&[0x42u8; 32])?;
        let recipient_spend_sk = SecretKey::from_bytes(&[0x43u8; 32])?;
        let recipient = SilentPaymentAddress::new(
            recipient_scan_sk.public_key(),
            recipient_spend_sk.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        // Capture scan_pk before recipient is moved into the actions.
        let scan_pk = recipient.scan_pk;

        let _deltas = spends.apply(
            &mut ctx,
            &[
                Action::send(
                    Id::Xch,
                    SendDestination::SilentPayment(Box::new(recipient.clone())),
                    1,
                    Memos::None,
                ),
                Action::send(
                    Id::Xch,
                    SendDestination::SilentPayment(Box::new(recipient)),
                    2,
                    Memos::None,
                ),
            ],
        )?;

        assert_eq!(spends.silent_payments_pending.len(), 2);
        assert_eq!(
            spends.silent_payments_pending[0].k, 0,
            "first output to recipient is k=0"
        );
        assert_eq!(
            spends.silent_payments_pending[1].k, 1,
            "second output to same scan_pk is k=1"
        );
        assert_eq!(spends.silent_payments_pending[0].amount, 1);
        assert_eq!(spends.silent_payments_pending[1].amount, 2);

        let scan_pk_bytes: [u8; 48] = scan_pk.to_bytes();
        assert_eq!(
            spends.silent_payment_counters.get(&scan_pk_bytes).copied(),
            Some(2),
            "counter incremented past k=1"
        );

        Ok(())
    }

    /// SEND-05: two `Action::send` calls with DIFFERENT SP recipients in one
    /// `Spends` produce outputs both at k=0 (per-`scan_pk` counters are
    /// independent).
    #[test]
    fn multi_output_distinct_scan_pks_independent_counters() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(10);

        let recipient_a = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );
        let recipient_b = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x44u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x45u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let _deltas = spends.apply(
            &mut ctx,
            &[
                Action::send(
                    Id::Xch,
                    SendDestination::SilentPayment(Box::new(recipient_a)),
                    1,
                    Memos::None,
                ),
                Action::send(
                    Id::Xch,
                    SendDestination::SilentPayment(Box::new(recipient_b)),
                    2,
                    Memos::None,
                ),
            ],
        )?;

        assert_eq!(spends.silent_payments_pending.len(), 2);
        assert_eq!(
            spends.silent_payments_pending[0].k, 0,
            "recipient_a's first output is k=0"
        );
        assert_eq!(
            spends.silent_payments_pending[1].k, 0,
            "recipient_b's first output is k=0 (independent counter)"
        );

        Ok(())
    }

    /// SEND-06: the receiver's `compute_input_hash` over the on-chain
    /// `coin_id`s plus the aggregated synthetic PK reconstructs the SAME
    /// `input_hash` the sender used. Uses `Relation::AssertConcurrent`
    /// (Phase 04.1 cycle binding) for the 2-input case.
    #[test]
    fn input_hash_round_trip() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(5);
        let bob = sim.bls(7);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let scan_pk = recipient.scan_pk;
        let spend_pk = recipient.spend_pk;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);
        spends.add(bob.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        )?;

        let pk_map = indexmap! {
            alice.puzzle_hash => alice.pk,
            bob.puzzle_hash => bob.pk,
        };
        let sk_map = indexmap! {
            alice.puzzle_hash => alice.sk.clone(),
            bob.puzzle_hash => bob.sk.clone(),
        };
        spends.with_silent_payment_keys(pk_map.clone(), sk_map);

        let outputs =
            spends.finish_with_keys(&mut ctx, &deltas, Relation::AssertConcurrent, &pk_map)?;

        // Independent reconstruction of the expected puzzle hash via the
        // free functions — exactly the path Phase 3's scanner would follow
        // after the cycle binding (opcode-64 SCC) provides the input set.
        //
        // Vec intermediate (clippy::cloned_ref_to_slice_refs precedent):
        // keeps the per-SK `.clone()` byte sequence in the source for grep
        // while satisfying clippy on the slice construction.
        let sender_sks = vec![alice.sk.clone(), bob.sk.clone()];
        let aggregated_sender_sk = aggregate_sender_sks(&sender_sks);
        let agg_pk = SecretKey::from_bytes(aggregated_sender_sk.as_bytes())
            .expect("aggregated SK < r")
            .public_key();
        let coin_ids = vec![alice.coin.coin_id(), bob.coin.coin_id()];
        let input_hash = compute_input_hash(&coin_ids, &agg_pk);
        let expected_ph =
            derive_one_time_puzzle_hash(&scan_pk, &spend_pk, &aggregated_sender_sk, &input_hash, 0);

        let found = outputs
            .xch
            .iter()
            .any(|c| c.puzzle_hash == expected_ph && c.amount == 1);
        assert!(
            found,
            "input_hash round-trip failed: expected puzzle_hash {} amount 1 not in outputs",
            hex::encode(expected_ph)
        );

        Ok(())
    }

    /// SEND-07 + ROADMAP Phase 4 success criterion #5: passing a 32-byte
    /// first memo to `Action::send` with an SP destination errors at apply
    /// time with `DriverError::SilentPaymentMemoHintForbidden`. The
    /// action's side-effects on `Spends` (parent reservation, k-counter
    /// increment, `SilentPaymentPending` push) DO NOT happen because the
    /// guard fires before them.
    #[test]
    fn memo_hint_guard_rejects_32_byte_first_memo() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        // Construct Memos with a 32-byte first atom via the canonical
        // `ctx.hint(...)` helper — it builds a Memos<NodePtr> containing
        // exactly one 32-byte atom (the Bytes32 hint).
        let hint_bytes: chia_protocol::Bytes32 = [0xffu8; 32].into();
        let bad_memos = ctx.hint(hint_bytes)?;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let result = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                bad_memos,
            )],
        );

        assert!(
            matches!(result, Err(DriverError::SilentPaymentMemoHintForbidden)),
            "expected SilentPaymentMemoHintForbidden, got {result:?}"
        );

        // No side effects: pending list still empty (apply failed before
        // the SilentPaymentPending push).
        assert!(
            spends.silent_payments_pending.is_empty(),
            "guard must fire BEFORE pushing SilentPaymentPending"
        );

        Ok(())
    }

    /// SEND-07: a 1-byte sentinel followed by a 32-byte payload passes the
    /// guard — the first memo is 1 byte, not 32. This is the wallet
    /// author's explicit escape hatch if they legitimately need a 32-byte
    /// payload memo: prefix it with a sentinel byte so the first atom is
    /// no longer 32 bytes.
    #[test]
    fn memo_hint_guard_allows_sentinel_prefixed() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        // Build Memos with a 1-byte first atom + a 32-byte second atom.
        let sentinel: chia_protocol::Bytes = chia_protocol::Bytes::new(vec![0x00u8]);
        let payload: chia_protocol::Bytes = chia_protocol::Bytes::new(vec![0xffu8; 32]);
        let safe_memos = ctx.memos(&[sentinel, payload])?;

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let result = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                safe_memos,
            )],
        );

        assert!(
            result.is_ok(),
            "1-byte sentinel + 32-byte payload must pass: {result:?}"
        );
        assert_eq!(spends.silent_payments_pending.len(), 1);

        Ok(())
    }

    /// SEND-07: `Memos::None` passes the guard trivially.
    #[test]
    fn memo_hint_guard_allows_none() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();

        let alice = sim.bls(1);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let result = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        );

        assert!(result.is_ok(), "Memos::None must pass: {result:?}");
        assert_eq!(spends.silent_payments_pending.len(), 1);

        Ok(())
    }

    // ====================================================================
    // 2 NEW WAVE 0 TESTS — per VALIDATION.md
    // ====================================================================

    /// SC9 / ACTION-API-01 acceptance: an SP destination paired with
    /// `Id::Existing(_)` (i.e. NOT `Id::Xch`) returns
    /// `Err(DriverError::SilentPaymentRequiresXch)` at apply time. SP
    /// destinations are XCH-only in v1.
    #[test]
    fn silent_payment_destination_requires_xch_id() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();
        let alice = sim.bls(1);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        // Use Id::Existing(arbitrary 32-byte asset id) — NOT Id::Xch.
        let bogus_asset_id = chia_protocol::Bytes32::from([0x77u8; 32]);
        let id = Id::Existing(bogus_asset_id);

        let result = spends.apply(
            &mut ctx,
            &[Action::send(
                id,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        );

        assert!(
            matches!(result, Err(DriverError::SilentPaymentRequiresXch)),
            "expected SilentPaymentRequiresXch on Id::Existing + SP destination, got: {result:?}"
        );
        Ok(())
    }

    /// SC8 / ACTION-API-01 acceptance: SP destination applied without
    /// a prior `with_silent_payment_keys` call returns
    /// `Err(DriverError::SilentPaymentKeysNotRegistered)` at finish time.
    #[test]
    fn silent_payment_keys_not_registered_errors_at_finish() -> Result<()> {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();
        let alice = sim.bls(1);

        let recipient = SilentPaymentAddress::new(
            SecretKey::from_bytes(&[0x42u8; 32])?.public_key(),
            SecretKey::from_bytes(&[0x43u8; 32])?.public_key(),
            SilentPaymentNetwork::Mainnet,
        );

        let mut spends = Spends::new(alice.puzzle_hash);
        spends.add(alice.coin);

        let deltas = spends.apply(
            &mut ctx,
            &[Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(recipient)),
                1,
                Memos::None,
            )],
        )?;

        // DELIBERATELY DO NOT CALL with_silent_payment_keys.

        let pk_map = indexmap! { alice.puzzle_hash => alice.pk };
        let result = spends.finish_with_keys(
            &mut ctx,
            &deltas,
            Relation::None, // single-input — binding gate short-circuits; KeysNotRegistered fires
            &pk_map,
        );

        assert!(
            matches!(result, Err(DriverError::SilentPaymentKeysNotRegistered)),
            "expected SilentPaymentKeysNotRegistered when finish runs without with_silent_payment_keys, got: {result:?}"
        );
        Ok(())
    }
}
