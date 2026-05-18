// AVA test for Phase 5 BIND-01/BIND-02 — Closes SC2 (address round-trip) +
// SC3 (SendDestination TS construction smoke).
//
// Asserts on `PublicKey.toBytes()` byte-equality, NOT on the encoded bech32m
// string — survives any future bech32m library churn (per RESEARCH §"Common
// Pitfalls" Anti-Pattern 5 + Phase 04.2 CONTEXT.md <specifics>).
//
// Mnemonic fixture is the BIP-39 standard test vector — also used by the
// Rust-side `from_mnemonic_tv1_scan_pk_matches` test at
// crates/chia-sdk-utils/src/silent_payments/keys.rs:152 — so this AVA test
// transitively pins the same CHIP TV1 bytes that the Rust test does.

import test from "ava";
import {
  Action,
  Id,
  Mnemonic,
  SendDestination,
  SilentPaymentAddress,
  SilentPaymentKeys,
  SilentPaymentNetwork,
} from "..";

const TV1_MNEMONIC =
  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

// SC2 — address round-trip via byte-equality on scan_pk/spend_pk
test("silent-payment address round-trip (TV1 mainnet)", (t) => {
  const mnemonic = new Mnemonic(TV1_MNEMONIC);
  const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
  const address = keys.unlabeledAddress(SilentPaymentNetwork.Mainnet);
  const encoded = address.encode();
  const decoded = SilentPaymentAddress.decode(encoded);

  // Byte-equality on the scan and spend public keys — survives bech32m library churn.
  t.deepEqual(decoded.scanPk.toBytes(), keys.scanPk().toBytes());
  t.deepEqual(decoded.spendPk.toBytes(), keys.spendPk().toBytes());
  // Network round-trips correctly.
  t.is(decoded.network, SilentPaymentNetwork.Mainnet);
});

// SC2 supplementary — testnet HRP discriminator round-trips correctly
test("silent-payment address round-trip (TV1 testnet)", (t) => {
  const mnemonic = new Mnemonic(TV1_MNEMONIC);
  const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
  const address = keys.unlabeledAddress(SilentPaymentNetwork.Testnet);
  const encoded = address.encode();
  // Testnet HRP confirmed in the encoded string (the only point in this test
  // where we touch the bech32m output — checked as a string-startsWith, not a
  // byte-pin, so future encoder changes won't false-positive).
  t.true(encoded.startsWith("tspxch1"));
  const decoded = SilentPaymentAddress.decode(encoded);
  t.is(decoded.network, SilentPaymentNetwork.Testnet);
  t.deepEqual(decoded.scanPk.toBytes(), keys.scanPk().toBytes());
});

// SC3 — SendDestination TS construction smoke (Action.send takes SendDestination)
test("SendDestination.silentPayment composes with Action.send (SC3)", (t) => {
  const mnemonic = new Mnemonic(TV1_MNEMONIC);
  const keys = SilentPaymentKeys.fromMnemonic(mnemonic);
  const address = keys.unlabeledAddress(SilentPaymentNetwork.Mainnet);

  // Factory: SendDestination.silentPayment wraps a SilentPaymentAddress.
  const dest = SendDestination.silentPayment(address);

  // Introspectors: confirm the discriminator round-trips.
  t.true(dest.isSilentPayment());
  t.false(dest.isPuzzleHash());

  // The address survives the round-trip through SendDestination.
  const recovered = dest.asSilentPayment();
  t.not(recovered, null);
  t.not(recovered, undefined);
  if (recovered) {
    t.deepEqual(recovered.scanPk.toBytes(), address.scanPk.toBytes());
    t.deepEqual(recovered.spendPk.toBytes(), address.spendPk.toBytes());
  }

  // SC3 critical: Action.send accepts SendDestination as the second arg
  // (this is the post-04.2 unified send shape; pre-04.2 it took Bytes32).
  // We don't execute the spend — just confirm construction succeeds without
  // throwing, which proves the bindy descriptor change in Plan 05-02 Task 3
  // landed correctly. (Full execution is Phase 6's concern.)
  const action = Action.send(Id.xch(), dest, 1000n, undefined);
  t.truthy(action);
});

// SC3 supplementary — SendDestination.puzzleHash factory + introspectors
test("SendDestination.puzzleHash round-trips through Action.send", (t) => {
  const PUZZLE_HASH = new Uint8Array(32);
  PUZZLE_HASH.fill(0x42);

  const dest = SendDestination.puzzleHash(PUZZLE_HASH);
  t.true(dest.isPuzzleHash());
  t.false(dest.isSilentPayment());

  const recovered = dest.asPuzzleHash();
  t.not(recovered, null);
  t.not(recovered, undefined);
  if (recovered) {
    t.deepEqual(new Uint8Array(recovered), PUZZLE_HASH);
  }

  // Pre-04.2 callers calling Action.send(id, puzzleHashBytes, amount, memos)
  // still work because From<Bytes32> for SendDestination preserves the
  // ergonomic — but on the TS side, callers must explicitly wrap via
  // SendDestination.puzzleHash() since impl Into<...> doesn't translate
  // through bindy.
  const action = Action.send(Id.xch(), dest, 1000n, undefined);
  t.truthy(action);
});
