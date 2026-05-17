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
