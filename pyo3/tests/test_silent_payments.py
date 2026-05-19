"""BIND-03 pyo3 unlabeled SP send + scan-from-tweaks E2E.

Closes the Python half of BIND-03. Mirrors napi/__test__/silent_payments_e2e.spec.ts
in snake_case. Per D-08, this is the first non-trivial pytest in pyo3/tests/; no
conftest.py until a second consumer exists.

Per D-07: labeled coverage stays Rust-only (Plan 06-03's e2e.rs); cross-language
tests only exercise the unlabeled flow.

Per D-01: TweakData is constructed on the Rust side (via
`Simulator.tweak_data_from_block`) and crossed the FFI boundary. First runtime
test of `Vec<chia_bls::PublicKey>` marshaling on `TweakData.tweak_points` across
the pyo3 FFI.
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
    assert (
        after_spend.spent_height is not None
    ), "detected SP coin successfully spent"
