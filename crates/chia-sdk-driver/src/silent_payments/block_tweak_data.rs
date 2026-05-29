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
