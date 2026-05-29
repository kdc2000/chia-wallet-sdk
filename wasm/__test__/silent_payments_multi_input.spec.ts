// wasm/__test__/silent_payments_multi_input.spec.ts
//
// BRIDGE-06 wasm multi-input SP send + scan-from-tweaks E2E.
//
// Closes the wasm half of BRIDGE-06. Mirrors
// napi/__test__/silent_payments_multi_input.spec.ts structurally; the only
// differences are imports from `../pkg` and the setPanicHook() call at module
// load per wasm-pack convention.
//
// TweakData is constructed by SilentPayments.tweakDataFromBlockSpends (the
// BRIDGE-05 helper) over sim.blockSpends(h) + sim.blockOutputs(h) (the
// BRIDGE-06 facade additions), driven by a 2-coin SP send whose driver-side
// gate is satisfied by spends.prepare(deltas, Relation.assertConcurrent())
// (BRIDGE-04 signature + BRIDGE-03 opaque-handle binding).

import test from "ava";
import {
  Action,
  Clvm,
  Id,
  LabelRegistry,
  Mnemonic,
  Relation,
  SendDestination,
  setPanicHook,
  SilentPaymentKeys,
  SilentPaymentNetwork,
  SilentPaymentRegisteredKey,
  SilentPaymentRegisteredSecretKey,
  SilentPayments,
  Simulator,
  Spends,
} from "../pkg";

setPanicHook();

const TV1_MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const K_MAX_DEFAULT = 2400;

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

test("BRIDGE-06 wasm: multi-input SP send -> tweak_data_from_block_spends -> scan", (t) => {
  const sim = new Simulator();
  const clvm = new Clvm();

  const recipient = SilentPaymentKeys.fromMnemonic(new Mnemonic(TV1_MNEMONIC));
  const recipientAddress = recipient.unlabeledAddress(
    SilentPaymentNetwork.Testnet,
  );

  // Two non-ephemeral XCH coins with different BLS pairs. The Relation
  // cycle binding ties them together so the receiver scanner can re-group
  // them via Pass 2b SCC over opcode-64 AssertConcurrentSpend edges.
  const sender1 = sim.bls(500n);
  const sender2 = sim.bls(500n);
  const heightBefore = sim.height();

  const spends = new Spends(clvm, sender1.puzzleHash);
  spends.addXch(sender1.coin);
  spends.addXch(sender2.coin);

  const actions = [
    Action.send(
      Id.xch(),
      SendDestination.silentPayment(recipientAddress),
      700n,
      undefined,
    ),
  ];

  spends.withSilentPaymentKeys(
    [
      new SilentPaymentRegisteredKey(sender1.puzzleHash, sender1.pk),
      new SilentPaymentRegisteredKey(sender2.puzzleHash, sender2.pk),
    ],
    [
      new SilentPaymentRegisteredSecretKey(sender1.puzzleHash, sender1.sk),
      new SilentPaymentRegisteredSecretKey(sender2.puzzleHash, sender2.sk),
    ],
  );

  const deltas = spends.apply(actions);

  // Pass Relation.assertConcurrent() so the driver-side gate
  // (non_ephemeral_xch_count >= 2) is satisfied. Without it,
  // DriverError::SilentPaymentRequiresInputBinding fires inside prepare().
  const finished = spends.prepare(deltas, Relation.assertConcurrent());

  for (const pending of finished.pendingSpends()) {
    const pendingPh = pending.coin().puzzleHash;
    const isS1 = bytesEqual(pendingPh, sender1.puzzleHash);
    const pk = isS1 ? sender1.pk : sender2.pk;
    finished.insert(
      pending.coin().coinId(),
      clvm.standardSpend(pk, clvm.delegatedSpend(pending.conditions())),
    );
  }
  finished.spend();

  sim.spendCoins(clvm.coinSpends(), [sender1.sk, sender2.sk]);

  // BRIDGE-05 entry point: drive TweakData construction through the new
  // helper, not the older Simulator.tweakDataFromBlock path. The
  // Simulator facade exposes block_spends / block_outputs per the
  // BRIDGE-06 Task 1 additions.
  const blockSpends = sim.blockSpends(heightBefore);
  const blockAdditions = sim.blockOutputs(heightBefore);
  const tweakData = SilentPayments.tweakDataFromBlockSpends(
    blockSpends,
    blockAdditions,
  );
  t.is(
    tweakData.tweakPoints.length,
    1,
    "one SP transaction group -> one tweak_point",
  );

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
  t.is(detections[0].k, 0, "first output at this scan_pk -> k=0");
  // wasm bindy emits `Option<u32>` as `number | undefined` (not null).
  t.is(detections[0].label, undefined, "unlabeled detection -> label is undefined");
  t.is(detections[0].amount, 700n, "multi-input SP output amount round-trips");
});
