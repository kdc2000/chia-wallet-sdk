//! Silent-payments (chip-0057) binding facade.
//!
//! Wave-0 stub: contains only the `SilentPayments` namespace class with a
//! `probe_noop` static method, used to empirically verify that bindy-macro
//! supports `"type": "static"` on a zero-field class across all three
//! binding targets (napi, wasm, pyo3). Plan 05-02 (Wave 1) expands this
//! file with the full Phase 5 facade: `SilentPaymentAddress`,
//! `SilentPaymentKeys`, `SilentPaymentNetwork`, `LabelRegistry`,
//! `TweakData`, `OutputMeta`, `DetectedSpCoin`, `ScalarField`, and the
//! four real static methods on `SilentPayments`.

use bindy::Result;

/// Wave-0 stub for the static-functions namespace class (per D-02). Expanded
/// in Plan 05-02 (Wave 1) with `scan_from_tweaks` / `derive_one_time_puzzle_hash`
/// / `compute_input_hash` / `aggregate_sender_sks`.
#[derive(Clone)]
pub struct SilentPayments;

impl SilentPayments {
    /// Pre-flight probe — confirms bindy-macro emits a zero-field-class +
    /// static-method without compile errors. Delete in Plan 05-02 when
    /// the four real static methods land.
    pub fn probe_noop() -> Result<u32> {
        Ok(0)
    }
}
