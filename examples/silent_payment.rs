//! CHIP-0057 silent-payments end-to-end demo (EX-01).
//!
//! Mirrors `examples/cat_spends.rs`: simulator setup → sender BLS pair →
//! recipient SP keys (BIP-39 mnemonic) → two SP sends in one tx (unlabeled +
//! labeled m=1) → farm → extract `TweakData` via `tweak_data_from_simulator_block`
//! → scan → detect both outputs → spend each via `StandardLayer` after
//! `.derive_synthetic()`.
//!
//! Tweak-data extraction uses only the test-crate helper — no transport
//! client is referenced (forward-compat). The labeled address uses m=1;
//! `labeled_address(0)` errors with `ReservedChangeLabel` (ADDR-06).
//!
//! Run: `cargo run --example silent_payment --all-features`

use anyhow::Result;
use bip39::Mnemonic;
use chia_puzzle_types::DeriveSynthetic;
use chia_wallet_sdk::prelude::*;
use indexmap::indexmap;

/// BIP-39 TV1 mnemonic — stable, well-known, deterministic across runs.
/// Matches the fixture used by the CHIP-0057 test vectors and the SDK's
/// silent-payments unit + binding tests.
const TV1_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn main() -> Result<()> {
    // 1. Setup: simulator, spend context, sender BLS pair, recipient SP keys.
    let mut sim = Simulator::new();
    let ctx = &mut SpendContext::new();
    let sender = sim.bls(1_000);
    let mnemonic = Mnemonic::parse(TV1_MNEMONIC)?;
    let recipient = SilentPaymentKeys::from_mnemonic(&mnemonic);

    // Derive both an unlabeled and a labeled (m=1) address from the same keys.
    let unlabeled_addr = recipient.unlabeled_address(SilentPaymentNetwork::Mainnet);
    let labeled_addr = recipient.labeled_address(SilentPaymentNetwork::Mainnet, 1)?;

    println!("Stage 1/5 — Addresses:");
    println!("  unlabeled:  {}", unlabeled_addr.encode()?);
    println!("  labeled(1): {}", labeled_addr.encode()?);

    // 2. Send 100 mojos to the unlabeled address and 200 mojos to labeled(m=1)
    //    in one tx via the unified Action::send + SendDestination surface.
    let height_before = sim.height();
    let mut spends = Spends::new(sender.puzzle_hash);
    spends.add(sender.coin);
    let deltas = spends.apply(
        ctx,
        &[
            Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(unlabeled_addr)),
                100,
                Memos::None,
            ),
            Action::send(
                Id::Xch,
                SendDestination::SilentPayment(Box::new(labeled_addr)),
                200,
                Memos::None,
            ),
        ],
    )?;
    let pks = indexmap! { sender.puzzle_hash => sender.pk };
    spends.with_silent_payment_keys(
        pks.clone(),
        indexmap! { sender.puzzle_hash => sender.sk.clone() },
    );
    spends.finish_with_keys(ctx, &deltas, Relation::None, &pks)?;
    sim.spend_coins(ctx.take(), std::slice::from_ref(&sender.sk))?;
    println!("Stage 2/5 — Sent 100 mojos unlabeled + 200 mojos labeled(m=1) in one tx.");

    // 3. Extract: walk the just-farmed block to build TweakData via the test-crate
    //    helper directly. Fully-qualified path because the umbrella prelude does
    //    not re-export it (forward-compat: no transport client is referenced).
    let tweak_data =
        chia_sdk_test::silent_payments::tweak_data_from_simulator_block(&sim, height_before);
    println!(
        "Stage 3/5 — Extracted TweakData: {} tweak_point(s), {} output(s).",
        tweak_data.tweak_points.len(),
        tweak_data.outputs.len(),
    );

    // 4. Scan: register m=1 in a fresh LabelRegistry so the scanner can attribute
    //    the labeled output. Unlabeled detection always works regardless of registry.
    let mut labels = LabelRegistry::new();
    labels.register(recipient.scan_sk(), 1);
    let detections = recipient.scan(&tweak_data, Some(&labels), K_MAX_DEFAULT);
    println!(
        "Stage 4/5 — Scanned: detected {} output(s).",
        detections.len()
    );
    for d in &detections {
        println!(
            "  coin_id={} amount={} label={:?} k={}",
            d.coin_id, d.amount, d.label, d.k,
        );
    }

    // 5. Spend: for each detected coin, derive the synthetic secret key from
    //    onetime_sk (mandatory — the puzzle currys StandardArgs(synthetic_key),
    //    so signing with the raw onetime_sk would produce an invalid signature),
    //    then spend via StandardLayer leaving (amount - 1) and a 1-mojo fee.
    for d in &detections {
        let synthetic_secret = d.onetime_sk.derive_synthetic();
        let conditions = Conditions::new()
            .create_coin(sender.puzzle_hash, d.amount - 1, Memos::None)
            .reserve_fee(1);
        let coin = Coin::new(d.parent_coin_id, d.puzzle_hash, d.amount);
        StandardLayer::new(synthetic_secret.public_key()).spend(ctx, coin, conditions)?;
        sim.spend_coins(ctx.take(), std::slice::from_ref(&synthetic_secret))?;
        println!(
            "Stage 5/5 — Spent detected coin {} (label={:?}).",
            d.coin_id, d.label
        );
    }

    Ok(())
}
