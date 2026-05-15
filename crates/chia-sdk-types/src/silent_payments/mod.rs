//! Silent payments (CHIP-0057) — wallet-side cryptographic primitives.
//!
//! All scalar-field reduction in this module flows through `ScalarField` so the
//! choice of unsigned vs signed byte interpretation is type-system-enforced.
//! Do NOT introduce alternate reducers here. In particular, the signed mod-r
//! reducer that the standard-puzzle synthetic-key offset uses (in
//! `chia_puzzle_types::derive_synthetic`) takes a different sign interpretation
//! and silently disagrees with `from_bytes_unsigned` on inputs whose high bit
//! is set — keep the two routes separate.

mod scalar;
pub use scalar::*;
mod tagged_hash;
pub use tagged_hash::*;
// (additional submodules appended in sorted order)
