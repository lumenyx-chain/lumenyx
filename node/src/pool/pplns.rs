//! PPLNS (Pay Per Last N Shares) reward calculation
//!
//! Distributes block reward proportionally based on share difficulty in window.

use super::types::{PoolAccountId, PoolPayoutEntry, PoolShare};
use std::collections::BTreeMap;

/// Compute PPLNS payouts from a window of shares
///
/// # Arguments
/// * `shares` - Shares in the PPLNS window (newest first)
/// * `block_reward` - Total reward to distribute (in planck)
/// * `max_payouts` - Maximum number of payouts (to limit digest size)
///
/// # Returns
/// Vector of payout entries, sorted by amount descending
pub fn compute_pplns_payouts(
    shares: &[PoolShare],
    block_reward: u128,
    max_payouts: usize,
) -> Vec<PoolPayoutEntry> {
    if block_reward == 0 || shares.is_empty() {
        return vec![];
    }

    // Aggregate weight by miner
    // Weight = sum of share_difficulty for each miner
    let mut weights: BTreeMap<PoolAccountId, u128> = BTreeMap::new();
    let mut total_w: u128 = 0;

    for s in shares {
        let w = s.share_difficulty.max(1);
        *weights.entry(s.miner).or_insert(0) = weights
            .get(&s.miner)
            .cloned()
            .unwrap_or(0)
            .saturating_add(w);
        total_w = total_w.saturating_add(w);
    }

    if total_w == 0 {
        return vec![];
    }

    // Calculate payouts proportionally
    let mut payouts: Vec<PoolPayoutEntry> = weights
        .into_iter()
        .map(|(acct, w)| {
            // payout = (block_reward * weight) / total_weight
            let amt = (block_reward.saturating_mul(w)) / total_w;
            PoolPayoutEntry {
                account: acct,
                amount: amt,
            }
        })
        .filter(|p| p.amount > 0)
        .collect();

    // Sort by amount descending (biggest miners first)
    payouts.sort_by(|a, b| b.amount.cmp(&a.amount));

    // Truncate to max_payouts (small miners lose out - incentive to get more shares)
    payouts.truncate(max_payouts);

    // Fix rounding remainder - give to biggest miner
    let sum: u128 = payouts.iter().map(|p| p.amount).sum();
    if !payouts.is_empty() && sum < block_reward {
        payouts[0].amount = payouts[0].amount.saturating_add(block_reward - sum);
    }

    // Safety: if sum > reward due to truncation edge cases, reduce first payout
    let sum2: u128 = payouts.iter().map(|p| p.amount).sum();
    if !payouts.is_empty() && sum2 > block_reward {
        payouts[0].amount = payouts[0].amount.saturating_sub(sum2 - block_reward);
    }

    payouts
}
