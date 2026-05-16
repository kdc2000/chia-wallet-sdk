//! Send-side `Spends` extension: pending state + finish-time entry point.
//!
//! - [`SilentPaymentPending`] holds the deterministic per-output data the
//!   action records at apply time (parent reservation + k + memos).
//! - [`Spends::finish_with_silent_payment_keys`] is the finish-time entry
//!   point that aggregates the sender SKs, computes the `input_hash`, derives
//!   each output's one-time puzzle hash, emits the matching `CreateCoin`
//!   conditions, and (Plan 04-04) emits the opcode 60/61 announcement
//!   binding for >=2-input scenarios.
//!
//! The struct + a stub impl land in Plan 04-02; the real implementation
//! lands in Plan 04-03; the announcement-binding extension lands in
//! Plan 04-04.

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;
use chia_puzzle_types::Memos;
use clvmr::NodePtr;
use indexmap::IndexMap;

use crate::{Deltas, DriverError, Outputs, Relation, SpendContext, Spends};

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
    /// Aggregates the sender synthetic SKs across all wallet-controlled XCH
    /// inputs, computes the per-spend-group `input_hash`, derives each pending
    /// recipient's one-time puzzle hash, emits the matching `CreateCoin`
    /// conditions, and (Plan 04-04 extension) emits opcode 60/61 announcement
    /// bindings for >=2-input scenarios. Hard-errors with
    /// `DriverError::SilentPaymentMultiPartyUnsupported` (Plan 04-03) if any
    /// XCH input's `p2_puzzle_hash` is missing from `synthetic_sks` (the
    /// caller does not hold every input's SK — multi-party flows are out of
    /// scope for v1).
    ///
    /// `synthetic_pks` is the same map [`Spends::finish_with_keys`] takes;
    /// `synthetic_sks` carries the SK material the silent-payment ECDH
    /// requires. The maps are kept distinct so wallets that split
    /// scan-online/spend-offline can pass an empty SK map when no
    /// `SilentPaymentSend` actions are in the batch (in which case this
    /// method delegates to `finish_with_keys` — Plan 04-03).
    ///
    /// Privacy warning: any memos attached to `SilentPaymentSend` actions in
    /// this batch are visible on chain in plaintext and to anyone holding the
    /// recipient's scan key. The 32-byte first-memo hint guard fires at
    /// apply time (`DriverError::SilentPaymentMemoHintForbidden` — Plan
    /// 04-05).
    pub fn finish_with_silent_payment_keys(
        self,
        _ctx: &mut SpendContext,
        _deltas: &Deltas,
        _relation: Relation,
        _synthetic_pks: &IndexMap<Bytes32, PublicKey>,
        _synthetic_sks: &IndexMap<Bytes32, SecretKey>,
    ) -> Result<Outputs, DriverError> {
        // STUB — Plan 04-03 replaces this body with the real implementation.
        // The stub ships in Plan 04-02 so the type is reachable for Plan
        // 04-02's apply-time integration test and for Plan 04-03's
        // multi-party hard-error test scaffolding.
        //
        // The destructure-and-discard below names each field so that
        // workspace-level `dead_code = "deny"` recognizes the pending state
        // as "read" without requiring `#[allow(dead_code)]`. Plan 04-03 will
        // replace the loop with a real ECDH + `CreateCoin` derivation.
        for SilentPaymentPending {
            scan_pk,
            spend_pk,
            parent_xch_index,
            parent_coin_id,
            parent_puzzle_hash,
            k,
            amount,
            memos,
        } in &self.silent_payments_pending
        {
            let _ = (
                scan_pk,
                spend_pk,
                parent_xch_index,
                parent_coin_id,
                parent_puzzle_hash,
                k,
                amount,
                memos,
            );
        }
        Err(DriverError::Custom(
            "finish_with_silent_payment_keys not yet implemented (Plan 04-03 fills in the body)"
                .into(),
        ))
    }
}
