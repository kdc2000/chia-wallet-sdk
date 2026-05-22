//! Build a [`TweakData`] from a single simulator block.
//!
//! The defensive standard-puzzle parse goes through
//! [`StandardLayer::parse_puzzle`], which internally calls
//! `StandardArgs::from_clvm` on the curried args and returns
//! `Ok(None)` for any puzzle whose mod hash does not match the standard p2.

use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_sdk_driver::silent_payments::{OutputMeta, TweakData, compute_input_hash};
use chia_sdk_driver::{Layer, Puzzle, StandardLayer};
use chia_sdk_types::silent_payments::ScalarField;
use clvm_traits::ToClvm;
use clvmr::Allocator;

use crate::Simulator;

/// Construct a [`TweakData`] from one block of the simulator's history.
///
/// Walks every coin spend that resolved at `height`, parses each
/// standard-puzzle spend's `synthetic_key`, aggregates them into `A_sum`,
/// computes `input_hash = tagged_hash("Chia_SP/Inputs", lex_min(coin_ids) || A_sum)`,
/// and returns `tweak_point = input_hash * A_sum` paired with every output
/// created in the block.
///
/// Returns an empty `TweakData` when the block contains no standard-puzzle
/// spends.
///
/// **CHIP §459 guard:** if the computed `tweak_point` is the BLS12-381
/// identity element, no tweak point is emitted — matches the scanner's
/// identity-element skip rule.
///
/// **Non-standard puzzles** (CAT, NFT, etc.) are silently skipped via the
/// defensive [`StandardLayer::parse_puzzle`] path (which calls
/// `StandardArgs::from_clvm` on the curried args) — the helper never panics
/// on unexpected puzzle reveals.
///
/// This helper is test-side only — production receive callers go through a
/// future CHIP-0058 transport client (out of v1 scope).
#[must_use]
pub fn tweak_data_from_simulator_block(sim: &Simulator, height: u32) -> TweakData {
    let block_spends = sim.block_spends(height);

    let mut allocator = Allocator::new();
    let mut synthetic_pks: Vec<PublicKey> = Vec::new();
    let mut spent_coin_ids: Vec<Bytes32> = Vec::new();

    for spend in &block_spends {
        // Defensive parse: skip any spend whose puzzle reveal does NOT match
        // the standard-puzzle shape (CAT / NFT / genesis / etc.).
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
        // A_sum = sum of every standard-puzzle synthetic_key in the block.
        let mut agg = synthetic_pks[0];
        for pk in &synthetic_pks[1..] {
            agg += pk;
        }

        // input_hash = tagged_hash("Chia_SP/Inputs", lex_min(coin_ids) || agg).
        let input_hash: ScalarField = compute_input_hash(&spent_coin_ids, &agg);

        // tweak_point = input_hash * A_sum (in-place scalar multiplication).
        let mut tweak_point = agg;
        tweak_point.scalar_multiply(&input_hash.to_bytes());

        // CHIP §459: identity-element results are not emitted.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SIM-01 defensive guard: empty block → empty `TweakData`, no panic.
    #[test]
    fn tweak_data_empty_block_returns_empty_tweak_data() {
        let sim = Simulator::new();
        let td = tweak_data_from_simulator_block(&sim, 0);
        assert!(td.tweak_points.is_empty(), "no spends → no tweak_points");
        assert!(td.outputs.is_empty(), "no outputs → empty outputs");
    }

    /// SIM-01 defensive guard: out-of-range height → empty `TweakData`, no panic.
    #[test]
    fn tweak_data_genesis_height_is_safe() {
        let sim = Simulator::new();
        let td = tweak_data_from_simulator_block(&sim, 9999);
        assert!(td.tweak_points.is_empty());
        assert!(td.outputs.is_empty());
    }
}
