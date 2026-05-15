//! Transport-agnostic silent-payment scanner.
//!
//! Implements CHIP-0057's wallet-side detection: given a [`TweakData`] (pre-computed
//! tweak points + candidate output metadata), iterate `k = 0, 1, 2, ...` per spend
//! group, derive the candidate one-time puzzle hash, and emit a [`DetectedSpCoin`]
//! whenever it matches one of the candidate outputs.
//!
//! Two CHIP-mandated guards the reference impl is missing:
//!
//! - **CHIP §459 identity-element skip:** if `tweak_point.is_inf()`, skip silently.
//!   Without this, an adversarial indexer can produce a predictable shared secret
//!   and force false positives. See `03-RESEARCH.md` §9.
//! - **CHIP §416 `K_max` cap:** bounded `for k in 0..k_max` (not `loop { ... }`)
//!   prevents DOS by forged matches. Default `K_MAX_DEFAULT = 2400` per CHIP §446
//!   (the Chia mempool maximum number of silent-payment outputs per spend bundle).
//!
//! The labeled-detection branch is added in Plan 03-04 (CHIP §RECV-04 labeled
//! k-termination rule).

use std::collections::HashSet;

use chia_bls::{PublicKey, SecretKey};
use chia_protocol::Bytes32;
use chia_sdk_utils::silent_payments::LabelRegistry;

use super::protocol::{
    compute_shared_secret_from_tweak, derive_onetime_pk, derive_onetime_sk, derive_output_tweak,
    puzzle_hash_for_pk,
};
use super::types::{DetectedSpCoin, TweakData};

/// Default per-spend-group iteration cap per CHIP-0057 §446.
///
/// Derived from Chia's mempool 5.5 B spend-bundle cost limit: 2,400 is the
/// theoretical maximum number of silent-payment outputs a single spend bundle
/// can fit at standard mempool policy. Callers may pass a smaller value to
/// [`scan_from_tweaks`] (e.g., 32 for a fast pre-scan in resource-constrained
/// environments) but should not exceed this in production scans.
pub const K_MAX_DEFAULT: usize = 2400;

/// Scan a block's worth of tweak points + candidate outputs for silent
/// payments addressed to this wallet.
///
/// For each `tweak_point` in `data.tweak_points`, performs one ECDH operation
/// (`scan_sk * tweak_point`, hashed to a 32-byte shared secret) and iterates
/// `k = 0, 1, 2, ...` up to `k_max`, deriving the candidate one-time puzzle
/// hash and checking against `data.outputs`. Labeled detection (per
/// `labels`) is interleaved per CHIP §RECV-04 in Plan 03-04.
///
/// **CHIP §459 guard:** identity-element tweak points are skipped silently —
/// they produce a predictable shared secret that would otherwise enable
/// false-positive detections.
///
/// **CHIP §416 `K_max` cap:** the `k_max` parameter bounds the per-spend-group
/// iteration count so an adversarial tweak source cannot force unbounded
/// scanning. Default value: [`K_MAX_DEFAULT`].
///
/// **Termination:** the `k` loop stops at the first miss (`if !found { break; }`)
/// — except labeled detections continue if either an unlabeled OR any labeled
/// candidate matches at the current `k` (the labeled rule lands in Plan 03-04).
//
// `clippy::similar_names` on the `spend_sk` / `spend_pk` parameter pair is
// genuinely unavoidable here: Plan 03-03 hard-locks the function signature
// (`pub fn scan_from_tweaks(scan_sk: &SecretKey, spend_sk: &SecretKey, spend_pk:
// &PublicKey, ...)`) and Plan 03-04 references both names directly in the
// labeled-detection branch it appends. The single-byte difference (`sk` vs `pk`)
// is below clippy's similarity threshold but rebinding the params would either
// break the locked signature or break Plan 03-04's structural expectations.
// Allowed at function scope only, not at module scope.
#[allow(clippy::similar_names)]
#[must_use]
pub fn scan_from_tweaks(
    scan_sk: &SecretKey,
    spend_sk: &SecretKey,
    spend_pk: &PublicKey,
    data: &TweakData,
    labels: Option<&LabelRegistry>,
    k_max: usize,
) -> Vec<DetectedSpCoin> {
    // Plan 03-04 will consume `labels`; suppress the unused-parameter lint
    // for Plan 03-03 by binding to `_`. Using `let _ = labels;` keeps the
    // public signature stable across plans.
    let _ = labels;

    let output_phs: HashSet<Bytes32> = data.outputs.iter().map(|o| o.puzzle_hash).collect();
    let mut detected = Vec::new();

    let k_bound = u32::try_from(k_max).unwrap_or(u32::MAX);

    for tweak_point in &data.tweak_points {
        // CHIP §459 guard: skip identity-element tweak points.
        if tweak_point.is_inf() {
            continue;
        }

        let shared_secret = compute_shared_secret_from_tweak(scan_sk, tweak_point);

        for k in 0..k_bound {
            let output_tweak = derive_output_tweak(&shared_secret, k);
            let candidate_pk = derive_onetime_pk(spend_pk, &output_tweak);
            let candidate_hash = puzzle_hash_for_pk(&candidate_pk);

            let mut found = false;

            if output_phs.contains(&candidate_hash)
                && let Some(out) = data
                    .outputs
                    .iter()
                    .find(|o| o.puzzle_hash == candidate_hash)
            {
                let onetime_sk = derive_onetime_sk(spend_sk, &output_tweak);
                detected.push(DetectedSpCoin {
                    coin_id: out.coin_id,
                    puzzle_hash: out.puzzle_hash,
                    amount: out.amount,
                    parent_coin_id: out.parent_coin_id,
                    onetime_sk,
                    k,
                    label: None,
                });
                found = true;
            }

            // PLAN 03-04 APPEND POINT: labeled-detection branch goes here.
            // The labeled branch sets `found = true` if any registered
            // label_pk matches at this k. The termination rule below is
            // already correct for both unlabeled-only AND labeled-OR-unlabeled
            // detection — Plan 03-04 only adds the labeled `if !found { ... }`
            // block immediately above this comment.

            if !found {
                break;
            }
        }
    }

    detected
}

#[cfg(test)]
mod tests {
    use super::super::types::OutputMeta;
    use super::*;
    use hex_literal::hex;

    // ─── TV1 pinned bytes (RESEARCH §10a) ──────────────────────────────────

    const TV1_SCAN_SK: [u8; 32] =
        hex!("132567e4dec19a4f50d9e9a549f16283dfb5aa4ad1ffdb6a505fcfcc56a690f6");
    const TV1_SPEND_SK: [u8; 32] =
        hex!("53d140b312a0e16316314274eb6398e15706d100fe8a754990540febd931b087");
    const TV1_SPEND_PK: [u8; 48] = hex!(
        "8afc580192f44fab624f613369f792eff3220ea3ca822eb839ab2c9309e527db"
        "f6f31e22e0831ba5088c952625a75c74"
    );
    const TV1_A_SUM: [u8; 48] = hex!(
        "8d9a5ed9c9b1a58476b07262007c636d775f2a33f0533737f3b3b0eaf99a8c0c"
        "51b3f2d87dc03a657e07f1828ab760fa"
    );
    const TV1_INPUT_HASH: [u8; 32] =
        hex!("38a1c8379cceb0fbebfdf3016707e54a1c7e9d21afb9489b9cc58f6055cc9411");
    const TV1_COIN_ID: [u8; 32] =
        hex!("5d759d2d97c03b1f6fe0657e91d25f6b7dd1311d6023271a1bcd35978a94a175");
    const TV1_PUZZLE_HASH: [u8; 32] =
        hex!("23adba149dd9000d65e0f8e21b6975364cbe89a63caf56533df4b7664c21fbf5");
    const TV1_ONETIME_SK: [u8; 32] =
        hex!("3c399c61ae130724903b3b650e936ff042b7646764289a33519e17100a89db37");

    // ─── TV4 pinned bytes (RESEARCH §10d) ──────────────────────────────────

    const TV4_A_SUM: [u8; 48] = hex!(
        "a223ab27f801044cd98c8314014b8073347b0e5aae43c69b78b5ca2a562ee9f7"
        "99b8efad179b34da1b306ca4d62bad40"
    );
    const TV4_INPUT_HASH: [u8; 32] =
        hex!("3f1071552b7f2f5e49b68166cb204f0a1b6a23b0c30a28bcba59a9c3f766e166");
    const TV4_COIN_ID: [u8; 32] =
        hex!("209bb03a4cd165785e6149bc6dcb27e35829006f02ec927ab5a20521fd27d21a");
    const TV4_PUZZLE_HASH: [u8; 32] =
        hex!("5d7fc7d7447c746cfb400e801a169fc7bfd1c13e03bc7866e6b743860a53ac6b");
    const TV4_ONETIME_SK: [u8; 32] =
        hex!("6ccc3e13145fd561e438d1bb82954cebb63cfa9577ea15404987aa8e0f309399");

    // ─── Helpers ───────────────────────────────────────────────────────────

    fn sk(bytes: [u8; 32]) -> SecretKey {
        SecretKey::from_bytes(&bytes).expect("test vector secret key")
    }

    fn pk(bytes: [u8; 48]) -> PublicKey {
        PublicKey::from_bytes(&bytes).expect("test vector public key")
    }

    fn tweak_point_from(a_sum: [u8; 48], input_hash: [u8; 32]) -> PublicKey {
        let mut point = pk(a_sum);
        point.scalar_multiply(&input_hash);
        point
    }

    // ─── Tests ─────────────────────────────────────────────────────────────

    /// RECV-02 + CRYPTO-03 (TV1): unlabeled detection at k=0 with TV1's
    /// pinned `shared_secret` → `t_0` → `onetime_pk` → `puzzle_hash` →
    /// `onetime_sk` chain.
    #[test]
    fn tv1_scan_detects_unlabeled_k0() {
        let data = TweakData {
            tweak_points: vec![tweak_point_from(TV1_A_SUM, TV1_INPUT_HASH)],
            outputs: vec![OutputMeta {
                puzzle_hash: TV1_PUZZLE_HASH.into(),
                coin_id: TV1_COIN_ID.into(),
                amount: 1000,
                parent_coin_id: [0u8; 32].into(),
            }],
        };

        let detections = scan_from_tweaks(
            &sk(TV1_SCAN_SK),
            &sk(TV1_SPEND_SK),
            &pk(TV1_SPEND_PK),
            &data,
            None,
            K_MAX_DEFAULT,
        );

        assert_eq!(detections.len(), 1, "expected exactly 1 TV1 detection");
        let detection = &detections[0];
        assert_eq!(detection.k, 0);
        assert!(detection.label.is_none(), "TV1 is unlabeled");
        assert_eq!(detection.puzzle_hash, Bytes32::from(TV1_PUZZLE_HASH));
        assert_eq!(detection.coin_id, Bytes32::from(TV1_COIN_ID));
        assert_eq!(detection.amount, 1000);
        assert_eq!(
            detection.onetime_sk.to_bytes(),
            TV1_ONETIME_SK,
            "TV1 onetime_sk mismatch"
        );
    }

    /// RECV-02 + CRYPTO-03 (TV4): multi-input aggregation. From the
    /// scanner's perspective the only difference vs TV1 is that the
    /// `A_sum` and `input_hash` are aggregated on the sender/indexer side
    /// — the scanner just gets a `tweak_point`. Tests that the scanner
    /// works for any `A_sum` + `input_hash` combination, not just the TV1
    /// single-input one.
    #[test]
    fn tv4_scan_detects_multi_input_aggregation() {
        // TV4 uses the same mnemonic / scan_sk / spend_sk / spend_pk as TV1.
        let data = TweakData {
            tweak_points: vec![tweak_point_from(TV4_A_SUM, TV4_INPUT_HASH)],
            outputs: vec![OutputMeta {
                puzzle_hash: TV4_PUZZLE_HASH.into(),
                coin_id: TV4_COIN_ID.into(),
                amount: 2000,
                parent_coin_id: [0u8; 32].into(),
            }],
        };

        let detections = scan_from_tweaks(
            &sk(TV1_SCAN_SK),
            &sk(TV1_SPEND_SK),
            &pk(TV1_SPEND_PK),
            &data,
            None,
            K_MAX_DEFAULT,
        );

        assert_eq!(detections.len(), 1, "expected exactly 1 TV4 detection");
        let detection = &detections[0];
        assert_eq!(detection.k, 0);
        assert!(detection.label.is_none());
        assert_eq!(detection.puzzle_hash, Bytes32::from(TV4_PUZZLE_HASH));
        assert_eq!(
            detection.onetime_sk.to_bytes(),
            TV4_ONETIME_SK,
            "TV4 onetime_sk mismatch"
        );
    }

    /// RECV-02 (CHIP §459 identity-element guard): a `TweakData` containing
    /// `PublicKey::default()` (identity element) is skipped silently — no
    /// panic, no detections. Without this guard, the predictable shared
    /// secret derived from the identity element would enable false-positive
    /// detections at attacker-supplied puzzle hashes.
    #[test]
    fn identity_tweak_point_skipped() {
        // PublicKey::default() is the BLS12-381 G1 identity element.
        let identity = PublicKey::default();
        assert!(
            identity.is_inf(),
            "PublicKey::default() must be inf — sanity"
        );

        let data = TweakData {
            tweak_points: vec![identity],
            outputs: vec![OutputMeta {
                puzzle_hash: TV1_PUZZLE_HASH.into(),
                coin_id: TV1_COIN_ID.into(),
                amount: 1,
                parent_coin_id: [0u8; 32].into(),
            }],
        };

        let detections = scan_from_tweaks(
            &sk(TV1_SCAN_SK),
            &sk(TV1_SPEND_SK),
            &pk(TV1_SPEND_PK),
            &data,
            None,
            K_MAX_DEFAULT,
        );

        assert!(
            detections.is_empty(),
            "identity-element tweak_point must produce no detections"
        );
    }
}
