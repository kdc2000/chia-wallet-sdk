//! Real-block CHIP-0057 [`TweakData`] builder for any post-decompression caller.
//!
//! Given a block's `Vec<CoinSpend>` (removals with puzzle reveals + solutions)
//! and `Vec<Coin>` (additions list), groups standard-puzzle spends into the
//! CHIP-0057 transaction-group shape that scanners detect against, then emits
//! one `tweak_point = input_hash * A_sum` per group plus paired
//! [`OutputMeta`] per addition. Generator decompression is the caller's
//! responsibility — the helper accepts the natural post-decompression shape so
//! it stays a pure function with no `chia-consensus` dependency.
//!
//! ## Grouping algorithm
//!
//! - **Stage 1 — defensive standard-puzzle filter.** Each [`CoinSpend`] is
//!   parsed via [`StandardLayer::parse_puzzle`]; non-standard puzzles (CAT,
//!   NFT, arbitrary mod hashes) skip silently.
//! - **Stage 2a — same-puzzle-hash bucketing.** Standard-puzzle spends with
//!   identical `coin.puzzle_hash` are bucketed into a group (the canonical
//!   shape for "all inputs at the same wallet derivation index").
//! - **Stage 2b — `AssertConcurrentSpend` SCC.** For every standard-puzzle
//!   spend not already in a Stage 2a group, its puzzle+solution is executed
//!   via [`chia_sdk_types::run_puzzle`] to extract conditions; opcode-64
//!   `AssertConcurrentSpend` targets become directed edges in a graph over
//!   the surviving standard-puzzle spends; iterative Tarjan SCC then groups
//!   spends that form a closed cycle. The cycle pattern is what
//!   `Spends::prepare` emits for multi-input SP sends via
//!   `Relation::AssertConcurrent`. Strongly-connected (not weakly-connected)
//!   grouping is what defends against third-party "pollution" assertions
//!   pointing at a legitimate-send coin: a polluter has a forward edge into
//!   the cycle but no return edge, so it stays in its own trivial SCC and
//!   does not corrupt the legitimate group's `A_sum`.
//! - **Stage 3 — per-group aggregation + tweak emission.** Each group computes
//!   `A_sum = Σ synthetic_key`,
//!   `input_hash = compute_input_hash(coin_ids, A_sum)`,
//!   `tweak_point = A_sum.scalar_multiply(input_hash)`. BLS12-381
//!   identity-element results are suppressed (CHIP §459).
//! - **Stage 4 — outputs.** Each addition becomes one [`OutputMeta`] (no
//!   grouping — outputs land flat in `TweakData.outputs`).
//!
//! ## Group emission order (load-bearing for byte-equality tests)
//!
//! 1. Stage 2a groups in puzzle-hash insertion order (driven by an
//!    `IndexMap` over `coin_spends` input order).
//! 2. Stage 2b SCCs of size ≥ 2 in Tarjan finishing order.
//! 3. Standalone single-spend "groups" in `coin_spends` input order.
//!
//! Cross-call regression tests (including the simulator-helper round-trip
//! oracle) depend on this ordering being stable across runs.

use chia_bls::PublicKey;
use chia_protocol::{Bytes32, Coin, CoinSpend};
use chia_sdk_types::{Condition, run_puzzle};
use clvm_traits::{FromClvm, ToClvm};
use clvmr::{Allocator, NodePtr};
use indexmap::IndexMap;

use crate::silent_payments::{OutputMeta, TweakData, compute_input_hash};
use crate::{DriverError, Layer, Puzzle, StandardLayer};

/// Parsed-standard-puzzle row carried between stages.
struct StandardSpend {
    coin_id: Bytes32,
    synthetic_pk: PublicKey,
    puzzle_hash: Bytes32,
    puzzle: NodePtr,
    solution: NodePtr,
}

/// Build a [`TweakData`] from a real (or simulator) block's coin spends and
/// additions.
///
/// Returns `Err(DriverError)` only on protocol-level corruption (e.g., a
/// downstream invariant in [`compute_input_hash`] failing); non-standard
/// puzzle reveals skip silently, never panicking or erroring.
///
/// See module-level docs for the grouping algorithm and ordering contract.
///
/// # Errors
///
/// Returns [`DriverError`] from downstream protocol primitives. The current
/// implementation never produces an error in practice; the `Result` return
/// shape reserves room for future protocol-level validation without an
/// API break.
pub fn tweak_data_from_block_spends(
    coin_spends: &[CoinSpend],
    additions: &[Coin],
) -> Result<TweakData, DriverError> {
    let mut allocator = Allocator::new();

    // Stage 1 — defensive standard-puzzle filter.
    let mut standard_spends: Vec<StandardSpend> = Vec::new();
    for spend in coin_spends {
        let Ok(puzzle_ptr) = spend.puzzle_reveal.to_clvm(&mut allocator) else {
            continue;
        };
        let parsed = Puzzle::parse(&allocator, puzzle_ptr);
        let Ok(Some(layer)) = StandardLayer::parse_puzzle(&allocator, parsed) else {
            continue;
        };
        let Ok(solution_ptr) = spend.solution.to_clvm(&mut allocator) else {
            continue;
        };
        standard_spends.push(StandardSpend {
            coin_id: spend.coin.coin_id(),
            synthetic_pk: layer.synthetic_key,
            puzzle_hash: spend.coin.puzzle_hash,
            puzzle: puzzle_ptr,
            solution: solution_ptr,
        });
    }

    // Stage 2a — same-puzzle-hash bucketing (insertion-ordered for deterministic emission).
    let mut ph_buckets: IndexMap<Bytes32, Vec<usize>> = IndexMap::new();
    for (i, ss) in standard_spends.iter().enumerate() {
        ph_buckets.entry(ss.puzzle_hash).or_default().push(i);
    }

    let mut grouped: Vec<bool> = vec![false; standard_spends.len()];
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (_ph, indices) in ph_buckets {
        if indices.len() >= 2 {
            for &i in &indices {
                grouped[i] = true;
            }
            groups.push(indices);
        }
    }

    // Stage 2b — `AssertConcurrentSpend` SCC over ungrouped standard spends.
    let surviving: Vec<usize> = (0..standard_spends.len())
        .filter(|i| !grouped[*i])
        .collect();
    if !surviving.is_empty() {
        // Index-to-position map for graph node identity.
        let coin_id_to_pos: IndexMap<Bytes32, usize> = surviving
            .iter()
            .enumerate()
            .map(|(pos, &i)| (standard_spends[i].coin_id, pos))
            .collect();

        // Adjacency list keyed by graph position (0..surviving.len()).
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); surviving.len()];
        for (pos, &i) in surviving.iter().enumerate() {
            let Ok(output) = run_puzzle(
                &mut allocator,
                standard_spends[i].puzzle,
                standard_spends[i].solution,
            ) else {
                continue;
            };
            let Ok(conditions) = Vec::<Condition>::from_clvm(&allocator, output) else {
                continue;
            };
            for cond in conditions {
                if let Condition::AssertConcurrentSpend(a) = cond
                    && let Some(&target_pos) = coin_id_to_pos.get(&a.coin_id)
                {
                    adj[pos].push(target_pos);
                }
            }
        }

        let sccs = iterative_tarjan_scc(&adj);
        for scc in sccs {
            if scc.len() >= 2 {
                let group: Vec<usize> = scc.into_iter().map(|pos| surviving[pos]).collect();
                for &i in &group {
                    grouped[i] = true;
                }
                groups.push(group);
            }
        }
    }

    // Standalone single-spend "groups" (single-input SP sends fall here).
    for (i, &is_grouped) in grouped.iter().enumerate() {
        if !is_grouped {
            groups.push(vec![i]);
        }
    }

    // Stage 3 — per-group aggregation + tweak_point emission.
    let mut tweak_points: Vec<PublicKey> = Vec::new();
    for group in groups {
        let coin_ids: Vec<Bytes32> = group.iter().map(|&i| standard_spends[i].coin_id).collect();
        let mut a_sum = standard_spends[group[0]].synthetic_pk;
        for &i in &group[1..] {
            a_sum += &standard_spends[i].synthetic_pk;
        }
        let input_hash = compute_input_hash(&coin_ids, &a_sum);
        let mut tweak_point = a_sum;
        tweak_point.scalar_multiply(&input_hash.to_bytes());
        if !tweak_point.is_inf() {
            tweak_points.push(tweak_point);
        }
    }

    // Stage 4 — pair additions with OutputMeta (no grouping; flat Vec).
    let outputs: Vec<OutputMeta> = additions
        .iter()
        .map(|coin| OutputMeta {
            puzzle_hash: coin.puzzle_hash,
            coin_id: coin.coin_id(),
            amount: coin.amount,
            parent_coin_id: coin.parent_coin_info,
        })
        .collect();

    Ok(TweakData {
        tweak_points,
        outputs,
    })
}

/// Iterative Tarjan strongly-connected-components over a directed graph
/// represented as adjacency lists.
///
/// Returns SCCs in Tarjan finishing order. Iterative (explicit stack) to
/// avoid stack overflow on adversarial deep graphs that a recursive
/// implementation would not survive.
fn iterative_tarjan_scc(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut index_counter: usize = 0;
    let mut stack: Vec<usize> = Vec::new();
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut indices: Vec<Option<usize>> = vec![None; n];
    let mut lowlinks: Vec<usize> = vec![0; n];
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for start in 0..n {
        if indices[start].is_some() {
            continue;
        }

        // Initialize root frame.
        indices[start] = Some(index_counter);
        lowlinks[start] = index_counter;
        index_counter += 1;
        stack.push(start);
        on_stack[start] = true;

        // Explicit call-stack frames: (node, neighbor iterator position).
        let mut work: Vec<(usize, usize)> = vec![(start, 0)];

        while let Some(&(v, next_neighbor)) = work.last() {
            if next_neighbor < adj[v].len() {
                let w = adj[v][next_neighbor];
                // Advance the iterator on the current frame before recursing.
                if let Some(frame) = work.last_mut() {
                    frame.1 += 1;
                }
                if indices[w].is_none() {
                    indices[w] = Some(index_counter);
                    lowlinks[w] = index_counter;
                    index_counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    work.push((w, 0));
                } else if on_stack[w] {
                    let w_index = indices[w].expect("on-stack node has an index");
                    if w_index < lowlinks[v] {
                        lowlinks[v] = w_index;
                    }
                }
            } else {
                // All neighbors exhausted — finish this node.
                let v_index = indices[v].expect("visited node has an index");
                if lowlinks[v] == v_index {
                    let mut scc: Vec<usize> = Vec::new();
                    loop {
                        let w = stack.pop().expect("tarjan stack invariant");
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
                work.pop();
                if let Some(&(parent, _)) = work.last()
                    && lowlinks[v] < lowlinks[parent]
                {
                    lowlinks[parent] = lowlinks[v];
                }
            }
        }
    }

    sccs
}

#[cfg(test)]
mod tests {
    use super::*;

    use chia_protocol::{Coin, Program};
    use chia_puzzle_types::standard::StandardArgs;
    use chia_sdk_test::Simulator;
    use chia_sdk_test::silent_payments::tweak_data_from_simulator_block;
    use chia_sdk_types::Conditions;

    use crate::SpendContext;
    use crate::StandardLayer;

    /// Build a `(CoinSpend, Coin)` pair for a synthetic-key-controlled coin
    /// whose solution outputs `conditions`. The caller is responsible for the
    /// parent coin info; the puzzle hash is derived from `synthetic_pk` so the
    /// coin shape is internally consistent.
    fn build_standard_coin_spend(
        synthetic_pk: PublicKey,
        parent_coin_info: Bytes32,
        amount: u64,
        conditions: Conditions,
    ) -> CoinSpend {
        let puzzle_hash: Bytes32 = StandardArgs::curry_tree_hash(synthetic_pk).into();
        let coin = Coin::new(parent_coin_info, puzzle_hash, amount);

        let mut ctx = SpendContext::new();
        let layer = StandardLayer::new(synthetic_pk);
        layer
            .spend(&mut ctx, coin, conditions)
            .expect("layer spend");
        ctx.take().pop().expect("one coin spend")
    }

    /// An empty block produces no tweak points and no outputs without panic.
    /// Adding additions with no spends produces zero `tweak_points` and one
    /// `OutputMeta` per addition.
    #[test]
    fn test_empty_block() {
        let td = tweak_data_from_block_spends(&[], &[]).expect("empty block ok");
        assert!(td.tweak_points.is_empty());
        assert!(td.outputs.is_empty());

        let parent: Bytes32 = [0xAAu8; 32].into();
        let puzzle_hash: Bytes32 = [0xBBu8; 32].into();
        let coin = Coin::new(parent, puzzle_hash, 100);
        let td = tweak_data_from_block_spends(&[], &[coin]).expect("additions-only ok");
        assert!(td.tweak_points.is_empty());
        assert_eq!(td.outputs.len(), 1);
        assert_eq!(td.outputs[0].puzzle_hash, puzzle_hash);
    }

    /// A `CoinSpend` whose puzzle reveal is not the standard p2 puzzle skips
    /// silently at Stage 1; no tweak point is emitted and the helper does not
    /// error.
    #[test]
    fn test_non_standard_puzzle_skip() {
        // `Program::default()` deserializes to NIL — not a curried standard puzzle.
        let parent: Bytes32 = [0x11u8; 32].into();
        let puzzle_hash: Bytes32 = [0x22u8; 32].into();
        let coin = Coin::new(parent, puzzle_hash, 1);
        let spend = CoinSpend::new(coin, Program::default(), Program::default());

        let td = tweak_data_from_block_spends(&[spend], &[]).expect("non-standard skip ok");
        assert!(td.tweak_points.is_empty(), "non-standard puzzle must skip");
        assert!(td.outputs.is_empty());
    }

    /// A standard-puzzle spend whose synthetic key is the BLS12-381 identity
    /// element yields `A_sum = identity` and therefore `tweak_point = identity`;
    /// the CHIP §459 guard suppresses emission so `tweak_points` stays empty.
    #[test]
    fn test_identity_element_guard() {
        let identity_pk = PublicKey::default();
        assert!(identity_pk.is_inf(), "PublicKey::default must be identity");

        let parent: Bytes32 = [0x33u8; 32].into();
        let spend = build_standard_coin_spend(identity_pk, parent, 1, Conditions::new());

        let td = tweak_data_from_block_spends(&[spend], &[]).expect("identity guard ok");
        assert!(
            td.tweak_points.is_empty(),
            "identity-element tweak_point must be suppressed",
        );
    }

    /// A single submitted standard-puzzle spend in the simulator produces the
    /// same `tweak_points` whether the data flows through the existing
    /// simulator helper or the new block-shape helper. Locks the standalone
    /// single-spend branch against drift versus the simulator-helper oracle.
    #[test]
    fn test_pass_2a_round_trip_matches_simulator_helper() {
        let mut sim = Simulator::new();
        let mut ctx = SpendContext::new();
        let alice = sim.bls(10);

        let layer = StandardLayer::new(alice.pk);
        layer
            .spend(&mut ctx, alice.coin, Conditions::new())
            .expect("alice spend");
        let coin_spends = ctx.take();

        sim.spend_coins(coin_spends, &[alice.sk]).expect("submit");
        let height = sim.height();

        let block_spends = sim.block_spends(height);
        let block_outputs = sim.block_outputs(height);

        let new_td =
            tweak_data_from_block_spends(&block_spends, &block_outputs).expect("new helper ok");
        let sim_td = tweak_data_from_simulator_block(&sim, height);

        assert_eq!(
            new_td.tweak_points.len(),
            sim_td.tweak_points.len(),
            "tweak_point count must match simulator helper",
        );
        for (a, b) in new_td.tweak_points.iter().zip(sim_td.tweak_points.iter()) {
            assert_eq!(a.to_bytes(), b.to_bytes(), "tweak_point bytes must match");
        }
    }

    /// Two standard-puzzle spends sharing the same `puzzle_hash` fall into a
    /// single Stage 2a group and emit exactly one aggregated `tweak_point`.
    /// Verifies the multi-input bucketing path independent of Pass 2b.
    #[test]
    fn test_multi_input_round_trip() {
        let alice = chia_bls::SecretKey::from_seed(&[0x07u8; 32]);
        let alice_public = alice.public_key();

        let parent_a: Bytes32 = [0x55u8; 32].into();
        let parent_b: Bytes32 = [0x66u8; 32].into();
        let spend_a = build_standard_coin_spend(alice_public, parent_a, 100, Conditions::new());
        let spend_b = build_standard_coin_spend(alice_public, parent_b, 200, Conditions::new());

        assert_eq!(
            spend_a.coin.puzzle_hash, spend_b.coin.puzzle_hash,
            "same synthetic_pk must curry to the same puzzle_hash",
        );

        let td = tweak_data_from_block_spends(&[spend_a, spend_b], &[]).expect("multi-input ok");
        assert_eq!(
            td.tweak_points.len(),
            1,
            "two spends at the same puzzle_hash form one Stage 2a group",
        );
    }

    /// Pass 2b "pollution attack" oracle: a legitimate 2-coin SP cycle
    /// (`a -> b`, `b -> a`) coexists in the same block with a polluter coin
    /// whose solution emits `AssertConcurrentSpend(a)`. SCC grouping must
    /// place `{a, b}` in one group and the polluter alone in its own trivial
    /// group; `A_sum` for the legitimate pair must NOT be contaminated by the
    /// polluter's synthetic key.
    ///
    /// Concretely, the tweak point emitted for `{a, b}` in the polluted block
    /// must equal the tweak point emitted when only `{a, b}` are passed to
    /// the helper in isolation.
    #[test]
    fn test_pass_2b_pollution_resistance() {
        let sk_a = chia_bls::SecretKey::from_seed(&[0x01u8; 32]);
        let sk_b = chia_bls::SecretKey::from_seed(&[0x02u8; 32]);
        let sk_polluter = chia_bls::SecretKey::from_seed(&[0x03u8; 32]);
        let pk_a = sk_a.public_key();
        let pk_b = sk_b.public_key();
        let pk_polluter = sk_polluter.public_key();

        // Distinct parent_coin_infos so coin_ids differ from puzzle_hash and
        // from each other; required for the AssertConcurrentSpend edges to
        // resolve to the right targets.
        let parent_a: Bytes32 = [0xA0u8; 32].into();
        let parent_b: Bytes32 = [0xB0u8; 32].into();
        let parent_polluter: Bytes32 = [0xC0u8; 32].into();

        // Pre-compute coin_ids so each spend can reference the other.
        let puzzle_hash_a: Bytes32 = StandardArgs::curry_tree_hash(pk_a).into();
        let puzzle_hash_b: Bytes32 = StandardArgs::curry_tree_hash(pk_b).into();
        let puzzle_hash_polluter: Bytes32 = StandardArgs::curry_tree_hash(pk_polluter).into();
        let coin_a = Coin::new(parent_a, puzzle_hash_a, 100);
        let coin_b = Coin::new(parent_b, puzzle_hash_b, 200);
        let coin_polluter = Coin::new(parent_polluter, puzzle_hash_polluter, 300);
        let id_a = coin_a.coin_id();
        let id_b = coin_b.coin_id();

        // Closed cycle: a asserts b, b asserts a.
        let spend_a = build_standard_coin_spend(
            pk_a,
            parent_a,
            100,
            Conditions::new().assert_concurrent_spend(id_b),
        );
        let spend_b = build_standard_coin_spend(
            pk_b,
            parent_b,
            200,
            Conditions::new().assert_concurrent_spend(id_a),
        );
        // Polluter: forward edge into the cycle, no return edge.
        let spend_polluter = build_standard_coin_spend(
            pk_polluter,
            parent_polluter,
            300,
            Conditions::new().assert_concurrent_spend(id_a),
        );

        assert_eq!(spend_a.coin, coin_a);
        assert_eq!(spend_b.coin, coin_b);
        assert_eq!(spend_polluter.coin, coin_polluter);

        let polluted =
            tweak_data_from_block_spends(&[spend_a.clone(), spend_b.clone(), spend_polluter], &[])
                .expect("polluted block ok");
        let clean = tweak_data_from_block_spends(&[spend_a, spend_b], &[]).expect("clean block ok");

        assert_eq!(
            polluted.tweak_points.len(),
            2,
            "polluted block must emit two tweak_points (legit SCC + polluter standalone)",
        );
        assert_eq!(
            clean.tweak_points.len(),
            1,
            "clean block must emit one tweak_point (the legit SCC)",
        );

        // The legit SCC's tweak point must match between polluted and clean
        // runs — proving the polluter's synthetic key did NOT leak into
        // A_sum. The polluted block emits SCC groups before standalone
        // groups, so the legit pair lands at index 0.
        assert_eq!(
            polluted.tweak_points[0].to_bytes(),
            clean.tweak_points[0].to_bytes(),
            "legit SCC tweak_point must be invariant under polluter presence",
        );
    }
}
