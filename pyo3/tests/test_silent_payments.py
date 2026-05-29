"""BIND-03 pyo3 unlabeled SP send + scan-from-tweaks E2E.

Closes the Python half of BIND-03. Mirrors napi/__test__/silent_payments_e2e.spec.ts
in snake_case. This is the first non-trivial pytest in pyo3/tests/; no
conftest.py is introduced until a second consumer needs shared fixtures.

Cross-language coverage is scoped to the unlabeled flow; labeled detection is
exercised by the Rust-side E2E tests in
crates/chia-sdk-driver/src/silent_payments/e2e.rs, where the labeled path is
already byte-pinned against the CHIP-0057 test vectors.

`TweakData` is constructed on the Rust side (via
`Simulator.tweak_data_from_block`) and crossed the FFI boundary unchanged.
This is the first runtime test of `Vec<chia_bls::PublicKey>` marshaling on
`TweakData.tweak_points` across the pyo3 FFI.
"""

from chia_wallet_sdk import (
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
)

# BIP-39 TV1 — matches Phase 5 AVA + Rust e2e fixtures so cross-language
# test outputs are byte-identical.
TV1_MNEMONIC = (
    "abandon abandon abandon abandon abandon abandon "
    "abandon abandon abandon abandon abandon about"
)
K_MAX_DEFAULT = 2400


def test_unlabeled_e2e():
    sim = Simulator()
    clvm = Clvm()

    # Recipient: deterministic mnemonic.
    recipient = SilentPaymentKeys.from_mnemonic(Mnemonic(TV1_MNEMONIC))
    recipient_address = recipient.unlabeled_address(SilentPaymentNetwork.Testnet)

    # Sender: fresh BLS pair from simulator.
    sender = sim.bls(1_000)
    height_before = sim.height()

    # Build the SP send via the unified Action.send + SendDestination +
    # with_silent_payment_keys path.
    spends = Spends(clvm, sender.puzzle_hash)
    spends.add_xch(sender.coin)

    actions = [
        Action.send(
            Id.xch(),
            SendDestination.silent_payment(recipient_address),
            100,
            None,
        )
    ]

    # Register SP keys (Phase 5 BIND-02 wrapper-class form — bindy doesn't
    # marshal Vec<(K,V)> directly).
    spends.with_silent_payment_keys(
        [SilentPaymentRegisteredKey(sender.puzzle_hash, sender.pk)],
        [SilentPaymentRegisteredSecretKey(sender.puzzle_hash, sender.sk)],
    )

    deltas = spends.apply(actions)
    finished = spends.prepare(deltas)

    # Standard-puzzle-spend the sender's XCH input.
    for pending in finished.pending_spends():
        finished.insert(
            pending.coin().coin_id(),
            clvm.standard_spend(sender.pk, clvm.delegated_spend(pending.conditions())),
        )
    finished.spend()

    # Farm the block.
    sim.spend_coins(clvm.coin_spends(), [sender.sk])

    # Extract TweakData via the Phase-6 bindings helper.
    # THIS IS THE NEW FFI SURFACE.
    tweak_data = sim.tweak_data_from_block(height_before)
    assert len(tweak_data.tweak_points) == 1, "one SP transaction -> one tweak_point"
    assert len(tweak_data.outputs) >= 1, "at least the recipient's output is present"

    # Vec<PublicKey> runtime marshaling proof — the first time this Vec
    # crosses the FFI boundary in any test. Each tweak point must be a
    # valid PublicKey we can serialize back to bytes.
    for tp in tweak_data.tweak_points:
        assert len(tp.to_bytes()) == 48, "each tweak_point round-trips as 48 bytes"

    # Scan.
    labels = LabelRegistry()
    detections = SilentPayments.scan_from_tweaks(
        recipient.scan_sk(),
        recipient.spend_sk(),
        recipient.spend_pk(),
        tweak_data,
        labels,
        K_MAX_DEFAULT,
    )

    assert len(detections) == 1, "scanner finds exactly one SP output"
    assert detections[0].k == 0, "first output at this scan_pk -> k=0"
    assert detections[0].label is None, "unlabeled detection -> label is None"
    assert detections[0].amount == 100, "amount round-trips"

    # BIND-03 closure: derive_synthetic + standard-puzzle-spend the
    # detected coin from Python. Strongest reading of "Vec<PublicKey>
    # marshals correctly" — the cross-language client not only reads
    # tweak_points but can complete the full send -> farm -> extract ->
    # scan -> SPEND round-trip.
    onetime_sk = detections[0].onetime_sk
    synthetic_sk = onetime_sk.derive_synthetic()
    synthetic_pk = synthetic_sk.public_key()

    detected_coin_id = detections[0].coin_id
    detected_amount = detections[0].amount
    detected_coin_state = sim.coin_state(detected_coin_id)
    assert detected_coin_state is not None, "detected coin in simulator state"
    assert (
        detected_coin_state.spent_height is None
    ), "detected coin is unspent before follow-on spend"

    follow_clvm = Clvm()
    conditions = [
        follow_clvm.create_coin(sender.puzzle_hash, detected_amount - 1, None),
        follow_clvm.reserve_fee(1),
    ]
    delegated_spend = follow_clvm.delegated_spend(conditions)
    standard_spend = follow_clvm.standard_spend(synthetic_pk, delegated_spend)
    follow_clvm.spend_coin(detected_coin_state.coin, standard_spend)

    sim.spend_coins(follow_clvm.coin_spends(), [synthetic_sk])

    after_spend = sim.coin_state(detected_coin_id)
    assert after_spend is not None, "coin state present after follow-on spend"
    assert (
        after_spend.spent_height is not None
    ), "detected SP coin successfully spent"


def test_multi_input_e2e():
    """BRIDGE-06 pyo3 multi-input SP send + scan-from-tweaks E2E.

    Mirrors `test_unlabeled_e2e` but with 2 sender coins,
    `Relation.assert_concurrent()` on `prepare`, and TweakData built via the
    new `SilentPayments.tweak_data_from_block_spends` helper over
    `sim.block_spends(h) + sim.block_outputs(h)`.

    Exercises the full Phase 9 binding surface end-to-end: the `Relation`
    opaque-handle (BRIDGE-03), the extended `Spends.prepare(deltas, relation)`
    signature (BRIDGE-04), the `SilentPayments.tweak_data_from_block_spends`
    static method (BRIDGE-05), and the `Simulator.block_spends` /
    `Simulator.block_outputs` facade additions (BRIDGE-06 prereq).
    """
    from chia_wallet_sdk import Relation

    sim = Simulator()
    clvm = Clvm()

    recipient = SilentPaymentKeys.from_mnemonic(Mnemonic(TV1_MNEMONIC))
    recipient_address = recipient.unlabeled_address(SilentPaymentNetwork.Testnet)

    # Two non-ephemeral XCH coins with different BLS pairs. The Relation
    # cycle binding ties them together so the receiver scanner can re-group
    # them via Pass 2b SCC over opcode-64 AssertConcurrentSpend edges.
    sender1 = sim.bls(500)
    sender2 = sim.bls(500)
    height_before = sim.height()

    spends = Spends(clvm, sender1.puzzle_hash)
    spends.add_xch(sender1.coin)
    spends.add_xch(sender2.coin)

    actions = [
        Action.send(
            Id.xch(),
            SendDestination.silent_payment(recipient_address),
            700,
            None,
        )
    ]

    spends.with_silent_payment_keys(
        [
            SilentPaymentRegisteredKey(sender1.puzzle_hash, sender1.pk),
            SilentPaymentRegisteredKey(sender2.puzzle_hash, sender2.pk),
        ],
        [
            SilentPaymentRegisteredSecretKey(sender1.puzzle_hash, sender1.sk),
            SilentPaymentRegisteredSecretKey(sender2.puzzle_hash, sender2.sk),
        ],
    )

    deltas = spends.apply(actions)

    # Pass Relation.assert_concurrent() so the driver-side gate
    # (non_ephemeral_xch_count >= 2) is satisfied. Without it,
    # DriverError::SilentPaymentRequiresInputBinding fires inside prepare().
    finished = spends.prepare(deltas, Relation.assert_concurrent())

    for pending in finished.pending_spends():
        is_s1 = pending.coin().puzzle_hash == sender1.puzzle_hash
        pk = sender1.pk if is_s1 else sender2.pk
        finished.insert(
            pending.coin().coin_id(),
            clvm.standard_spend(pk, clvm.delegated_spend(pending.conditions())),
        )
    finished.spend()

    sim.spend_coins(clvm.coin_spends(), [sender1.sk, sender2.sk])

    # BRIDGE-05 entry point: drive TweakData construction through the new
    # helper, not the older Simulator.tweak_data_from_block path. The
    # Simulator facade exposes block_spends / block_outputs per the
    # BRIDGE-06 Task 1 additions.
    block_spends = sim.block_spends(height_before)
    block_outputs = sim.block_outputs(height_before)
    tweak_data = SilentPayments.tweak_data_from_block_spends(
        block_spends, block_outputs
    )
    assert (
        len(tweak_data.tweak_points) == 1
    ), "one SP transaction group -> one tweak_point"

    labels = LabelRegistry()
    detections = SilentPayments.scan_from_tweaks(
        recipient.scan_sk(),
        recipient.spend_sk(),
        recipient.spend_pk(),
        tweak_data,
        labels,
        K_MAX_DEFAULT,
    )

    assert len(detections) == 1, "scanner finds exactly one SP output"
    assert detections[0].k == 0, "first output at this scan_pk -> k=0"
    assert detections[0].label is None, "unlabeled detection -> label is None"
    assert detections[0].amount == 700, "multi-input SP output amount round-trips"
