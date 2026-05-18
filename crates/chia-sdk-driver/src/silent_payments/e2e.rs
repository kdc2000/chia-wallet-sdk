//! Phase 6 end-to-end simulator tests for CHIP-0057 silent payments.
//!
//! These tests run the full send→farm→extract→scan→detect→spend flow against
//! [`chia_sdk_test::Simulator`], exercising every Phase 1-5 primitive in
//! concert.
//!
//! Closes SIM-02 (unlabeled), SIM-03 labeled half, and SIM-03 m=0 half
//! (redesigned per 06-RESEARCH §3b — m=0 self-change is NOT auto-emitted by
//! the SDK; the test demonstrates [`LabelRegistry`] consistency instead).
//!
//! **Why we re-implement `tweak_data_from_simulator_block` locally:**
//!
//! Plan 06-02 ships `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`
//! as the canonical helper, and this is what production-style code (and the
//! Phase 6 example in Plan 06-05 + the binding round-trips in Plan 06-04) will
//! call. However, calling that cross-crate helper from chia-sdk-driver's own
//! test crate triggers Cargo's well-known cyclic-dev-dep type-confusion: the
//! lib build of chia-sdk-driver (reached via the `chia-sdk-driver → chia-sdk-test
//! → chia-sdk-driver` cycle) is a structurally distinct compile unit from the
//! lib-test build that hosts this module. [`TweakData`] values returned by
//! the chia-sdk-test helper therefore appear as a "different type" to the
//! chia-sdk-driver scanner inside this file.
//!
//! Resolution: this module inlines the same `block_spends`-based extraction
//! algorithm using the chia-sdk-driver primitives directly. The cross-crate
//! `tweak_data_from_simulator_block` helper remains the canonical entry point
//! for non-cyclic callers (the example + binding tests).

use anyhow::Result;
use bip39::Mnemonic;
use chia_bls::PublicKey;
use chia_protocol::{Bytes32, Coin};
use chia_puzzle_types::{DeriveSynthetic, Memos};
use chia_sdk_test::{BlsPairWithCoin, Simulator};
use chia_sdk_types::Conditions;
use chia_sdk_types::silent_payments::ScalarField;
use chia_sdk_utils::silent_payments::{
    LabelRegistry, SilentPaymentKeys, SilentPaymentNetwork,
};
use clvm_traits::ToClvm;
use clvmr::Allocator;
use indexmap::indexmap;

use crate::silent_payments::{
    K_MAX_DEFAULT, OutputMeta, TweakData, compute_input_hash, scan_from_tweaks,
};
use crate::{
    Action, Id, Layer, Puzzle, Relation, SendDestination, SpendContext, Spends, StandardLayer,
};

/// Stable BIP-39 test-vector mnemonic — matches Phase 5 AVA fixture for
/// cross-language consistency.
const TV1_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// Local inlined equivalent of `tweak_data_from_simulator_block` — see the
/// module-level rustdoc for why this exists rather than calling the
/// canonical chia-sdk-test helper directly.
///
/// The algorithm matches `chia_sdk_test::silent_payments::tweak_data_from_simulator_block`
/// byte-for-byte: walk `sim.block_spends(height)`, defensively parse each
/// puzzle reveal as a standard puzzle, aggregate the surviving synthetic
/// keys, compute `tweak_point = input_hash * A_sum`, and pair with
/// `sim.block_outputs(height)`. CHIP §459 identity-element skip rule honored.
fn build_tweak_data(sim: &Simulator, height: u32) -> TweakData {
    let block_spends = sim.block_spends(height);

    let mut allocator = Allocator::new();
    let mut synthetic_pks: Vec<PublicKey> = Vec::new();
    let mut spent_coin_ids: Vec<Bytes32> = Vec::new();

    for spend in &block_spends {
        let Ok(ptr) = spend.puzzle_reveal.to_clvm(&mut allocator) else {
            continue;
        };
        let puzzle = Puzzle::parse(&allocator, ptr);
        let Ok(Some(layer)) = StandardLayer::parse_puzzle(&allocator, puzzle) else {
            continue;
        };
        synthetic_pks.push(layer.synthetic_key);
        spent_coin_ids.push(spend.coin.coin_id());
    }

    let mut tweak_points: Vec<PublicKey> = Vec::new();
    if !synthetic_pks.is_empty() {
        let mut agg = synthetic_pks[0];
        for pk in &synthetic_pks[1..] {
            agg += pk;
        }

        let input_hash: ScalarField = compute_input_hash(&spent_coin_ids, &agg);

        let mut tweak_point = agg;
        tweak_point.scalar_multiply(&input_hash.to_bytes());

        if !tweak_point.is_inf() {
            tweak_points.push(tweak_point);
        }
    }

    let outputs: Vec<OutputMeta> = sim
        .block_outputs(height)
        .into_iter()
        .map(|coin| OutputMeta {
            puzzle_hash: coin.puzzle_hash,
            coin_id: coin.coin_id(),
            amount: coin.amount,
            parent_coin_id: coin.parent_coin_info,
        })
        .collect();

    TweakData {
        tweak_points,
        outputs,
    }
}

/// Shared setup: fresh [`Simulator`] + [`SpendContext`] + sender BLS pair (with
/// a `1_000`-mojo XCH coin) + a recipient [`SilentPaymentKeys`] derived from a
/// deterministic mnemonic.
///
/// Each test calls this fresh — no shared state, deterministic seeds, every
/// run produces identical `coin_id`s.
fn setup_e2e() -> Result<(Simulator, SpendContext, BlsPairWithCoin, SilentPaymentKeys)> {
    let mut sim = Simulator::new();
    let ctx = SpendContext::new();
    let sender = sim.bls(1_000);
    let mnemonic = Mnemonic::parse(TV1_MNEMONIC)?;
    let recipient = SilentPaymentKeys::from_mnemonic(&mnemonic);
    Ok((sim, ctx, sender, recipient))
}

/// SIM-02: unlabeled SP send round-trip against the simulator.
///
/// Asserts:
/// 1. Exactly one detection.
/// 2. `label: None` (unlabeled).
/// 3. `k == 0` (first output to this `scan_pk` in the tx).
/// 4. `amount == 100`.
/// 5. The detected coin spends successfully via [`StandardLayer`] after
///    applying [`DeriveSynthetic::derive_synthetic`] to the `onetime_sk`.
#[test]
fn test_simulator_e2e_unlabeled() -> Result<()> {
    let (mut sim, mut ctx, sender, recipient) = setup_e2e()?;
    let recipient_address = recipient.unlabeled_address(SilentPaymentNetwork::Testnet);
    let height_before = sim.height();

    // Build the SP send via the Phase 4.2 unified Action::send + SendDestination path.
    let mut spends = Spends::new(sender.puzzle_hash);
    spends.add(sender.coin);
    let deltas = spends.apply(
        &mut ctx,
        &[Action::send(
            Id::Xch,
            SendDestination::SilentPayment(Box::new(recipient_address)),
            100,
            Memos::None,
        )],
    )?;
    let pk_map = indexmap! { sender.puzzle_hash => sender.pk };
    let sk_map = indexmap! { sender.puzzle_hash => sender.sk.clone() };
    spends.with_silent_payment_keys(pk_map.clone(), sk_map);
    spends.finish_with_keys(&mut ctx, &deltas, Relation::None, &pk_map)?;

    // Farm: spend_coins farms a block internally.
    sim.spend_coins(ctx.take(), std::slice::from_ref(&sender.sk))?;

    // Extract: build TweakData from the freshly-farmed block (local inline
    // equivalent of `tweak_data_from_simulator_block`).
    let tweak_data = build_tweak_data(&sim, height_before);
    assert!(
        !tweak_data.tweak_points.is_empty(),
        "expected one tweak_point"
    );
    assert!(
        !tweak_data.outputs.is_empty(),
        "expected at least the recipient's output"
    );

    // Scan: recipient detects the coin via `scan_from_tweaks` (free-fn form).
    let detections = scan_from_tweaks(
        recipient.scan_sk(),
        recipient.spend_sk(),
        recipient.spend_pk(),
        &tweak_data,
        None,
        K_MAX_DEFAULT,
    );
    assert_eq!(detections.len(), 1, "expected exactly 1 detection");
    let detected = &detections[0];
    assert!(detected.label.is_none(), "unlabeled → label must be None");
    assert_eq!(detected.k, 0, "first output → k=0");
    assert_eq!(detected.amount, 100);

    // Spend the detected coin: derive_synthetic() then StandardLayer.
    let synthetic_secret = detected.onetime_sk.derive_synthetic();
    let conditions = Conditions::new()
        .create_coin(sender.puzzle_hash, detected.amount - 1, Memos::None)
        .reserve_fee(1);
    let coin = Coin::new(
        detected.parent_coin_id,
        detected.puzzle_hash,
        detected.amount,
    );
    StandardLayer::new(synthetic_secret.public_key()).spend(&mut ctx, coin, conditions)?;
    sim.spend_coins(ctx.take(), std::slice::from_ref(&synthetic_secret))?;

    // Verify: the detected coin is now spent.
    let post_state = sim
        .coin_state(detected.coin_id)
        .expect("detected coin in state");
    assert!(
        post_state.spent_height.is_some(),
        "detected coin must be spent after follow-on spend"
    );

    // Reference `LabelRegistry` here so the unused-import warning does not fire
    // — Task 2's labeled tests register labels via `LabelRegistry::new()`.
    let _registry_canary: LabelRegistry = LabelRegistry::new();

    Ok(())
}
