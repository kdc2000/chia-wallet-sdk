// napi/__test__/silent_payments_e2e.spec.ts
//
// BIND-03 napi unlabeled SP send + scan-from-tweaks E2E.
//
// Closes BIND-03 with full FFI fidelity: TweakData is constructed on the
// Rust side and crossed the FFI boundary unchanged. This is the first runtime
// test of Vec<chia_bls::PublicKey> marshaling on TweakData.tweakPoints across
// napi.
//
// Cross-language coverage is scoped to the unlabeled flow; the labeled
// detection branch is exercised by the Rust-side E2E tests in
// crates/chia-sdk-driver/src/silent_payments/e2e.rs.

import test from "ava";
import {
  Action,
  Clvm,
  Id,
  LabelRegistry,
  Mnemonic,
  SendDestination,
  SilentPaymentKeys,
  SilentPaymentNetwork,
  SilentPaymentRegisteredKey,
  SilentPaymentRegisteredSecretKey,
  SilentPayments,
  Simulator,
  Spends,
} from "..";

const TV1_MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const K_MAX_DEFAULT = 2400;

test("BIND-03 napi: unlabeled SP send + scan-from-tweaks E2E", (t) => {
  const sim = new Simulator();
  const clvm = new Clvm();

  // Recipient: deterministic mnemonic so the test is reproducible
  // (matches the Phase 5 silent_payments.spec.ts fixture).
  const recipient = SilentPaymentKeys.fromMnemonic(new Mnemonic(TV1_MNEMONIC));
  const recipientAddress = recipient.unlabeledAddress(
    SilentPaymentNetwork.Testnet,
  );

  // Sender: fresh BLS pair from simulator.
  const sender = sim.bls(1_000n);
  const heightBefore = sim.height();

  // Build the SP send via the unified Action.send + SendDestination +
  // withSilentPaymentKeys path (Phase 4.2 + Phase 5).
  const spends = new Spends(clvm, sender.puzzleHash);
  spends.addXch(sender.coin);

  const actions = [
    Action.send(
      Id.xch(),
      SendDestination.silentPayment(recipientAddress),
      100n,
      undefined,
    ),
  ];

  // Register SP keys BEFORE apply (with_silent_payment_keys must precede
  // finish-time SP processing inside prepare()). Wrapper-class form per
  // Phase 5 BIND-02 — bindy doesn't marshal Vec<(K,V)> directly.
  spends.withSilentPaymentKeys(
    [new SilentPaymentRegisteredKey(sender.puzzleHash, sender.pk)],
    [new SilentPaymentRegisteredSecretKey(sender.puzzleHash, sender.sk)],
  );

  const deltas = spends.apply(actions);
  const finished = spends.prepare(deltas);

  // Standard-puzzle-spend the sender's XCH input (mirrors
  // napi/__test__/action_system.spec.ts:142-157 Wallet.spend pattern).
  for (const pending of finished.pendingSpends()) {
    finished.insert(
      pending.coin().coinId(),
      clvm.standardSpend(sender.pk, clvm.delegatedSpend(pending.conditions())),
    );
  }
  finished.spend();

  // Farm the block.
  sim.spendCoins(clvm.coinSpends(), [sender.sk]);

  // Extract TweakData via the Phase-6 bindings helper.
  // THIS IS THE NEW FFI SURFACE.
  const tweakData = sim.tweakDataFromBlock(heightBefore);
  t.is(
    tweakData.tweakPoints.length,
    1,
    "one SP transaction → one tweak_point",
  );
  t.true(
    tweakData.outputs.length >= 1,
    "at least the recipient's output is in OutputMeta list",
  );
  // Vec<PublicKey> runtime marshaling proof — the first time this Vec
  // crosses the FFI boundary in any test. Each tweak point must be a
  // valid PublicKey we can serialize back to bytes.
  for (const tp of tweakData.tweakPoints) {
    t.is(tp.toBytes().length, 48, "each tweak_point round-trips as 48 bytes");
  }

  // Scan — Vec<PublicKey> marshaling check fires here on
  // tweakData.tweakPoints access inside the scanner.
  const labels = new LabelRegistry();
  const detections = SilentPayments.scanFromTweaks(
    recipient.scanSk(),
    recipient.spendSk(),
    recipient.spendPk(),
    tweakData,
    labels,
    K_MAX_DEFAULT,
  );

  t.is(detections.length, 1, "scanner finds exactly one SP output");
  t.is(detections[0].k, 0, "first output at this scan_pk → k=0");
  t.is(detections[0].label, null, "unlabeled detection → label is null");
  t.is(detections[0].amount, 100n, "amount round-trips");

  // BIND-03 closure: derive_synthetic + standard-puzzle-spend the
  // detected coin from TypeScript. This is the strongest reading of
  // "Vec<PublicKey> marshals correctly" — the cross-language client
  // not only reads tweak_points but can complete the full
  // send → farm → extract → scan → SPEND round-trip.
  const onetimeSk = detections[0].onetimeSk;
  const syntheticSk = onetimeSk.deriveSynthetic();
  const syntheticPk = syntheticSk.publicKey();

  const detectedCoinId = detections[0].coinId;
  const detectedAmount = detections[0].amount;
  const detectedCoinState = sim.coinState(detectedCoinId);
  t.not(detectedCoinState, null, "detected coin is in simulator state");
  t.is(
    detectedCoinState?.spentHeight,
    null,
    "detected coin is unspent before follow-on spend",
  );

  // Build conditions to spend the detected coin back to the sender, with a
  // 1-mojo fee. Reuse the same Clvm allocator (action_system.spec.ts pattern).
  const followClvm = new Clvm();
  const conditions = [
    followClvm.createCoin(sender.puzzleHash, detectedAmount - 1n, null),
    followClvm.reserveFee(1n),
  ];
  const delegatedSpend = followClvm.delegatedSpend(conditions);
  const standardSpend = followClvm.standardSpend(syntheticPk, delegatedSpend);
  const detectedCoin = detectedCoinState!.coin;
  followClvm.spendCoin(detectedCoin, standardSpend);

  sim.spendCoins(followClvm.coinSpends(), [syntheticSk]);

  const afterSpend = sim.coinState(detectedCoinId);
  t.not(afterSpend?.spentHeight, null, "detected SP coin successfully spent");
});
