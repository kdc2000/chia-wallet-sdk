//! Silent payments (CHIP-0057) — wallet-side receive primitive and protocol helpers.
//!
//! This module implements the transport-agnostic scanner described in CHIP-0057
//! §RECV-01..05. A wallet that receives [`TweakData`] from any source (a CHIP-0058
//! transport client, the `chia_sdk_test` simulator helper from Phase 6, or a
//! handcrafted fixture) can detect payments addressed to its scan/spend key pair
//! via `scan_from_tweaks` (Plan 03-03).
//!
//! The protocol primitives — `compute_shared_secret_from_tweak`,
//! `derive_output_tweak`, `derive_onetime_pk`, `derive_onetime_sk`,
//! `puzzle_hash_for_pk` (Plan 03-02) — are exposed publicly so the Phase 4
//! send-side action and any caller that needs to compute shared secrets manually
//! can reuse them without round-tripping through the scanner.
//!
//! Forward compatibility with CHIP-0058: [`TweakData`] has no transport fields
//! (no `height`, no `block_hash`, no JSON envelope). A future transport client
//! constructs `TweakData` from its wire messages without breaking this module's
//! shape.
//!
//! All scalar reduction in this module flows through
//! `chia_sdk_types::silent_payments::ScalarField` (Phase 1) so the unsigned-vs-signed
//! reduction choice is type-system-enforced. The signed mod-r reducer that the
//! standard-puzzle synthetic-key offset uses (in `chia_puzzle_types::derive_synthetic`)
//! takes a different sign interpretation and silently disagrees with
//! `from_bytes_unsigned` on inputs whose high bit is set — keep the two routes
//! separate.
//!
//! Hash routines in this module use `chia_sha2::Sha256` exclusively; the workspace
//! grep ban forbids `use sha2::` imports under `silent_payments/`.

mod protocol;
pub use protocol::*;
mod scanner;
pub use scanner::*;
mod types;
pub use types::*;
