//! LUMENYX Core Primitives
//!
//! Decentralized cryptocurrency with fixed supply.
//! 21M fixed supply, no team, no foundation. Just code.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

/// LUMENYX token decimals (12 decimals like DOT)
pub const DECIMALS: u8 = 12;

/// Token symbol
pub const SYMBOL: &str = "LUMENYX";

/// 1 LUMENYX = 10^12 planck (smallest unit)
pub const LUMENYX: u128 = 1_000_000_000_000;

/// Total supply: 21,000,000 LUMENYX (fixed, NEVER MORE)
pub const TOTAL_SUPPLY: u128 = 21_000_000 * LUMENYX;

/// Block time in milliseconds: 2.5 seconds
pub const BLOCK_TIME_MS: u64 = 2_500;

/// Blocks per day (86400 / 2.5 = 34560)
pub const BLOCKS_PER_DAY: u32 = 34_560;

/// Blocks per year (34560 * 365.25 = 12,623,040)
pub const BLOCKS_PER_YEAR: u32 = 12_623_040;

/// Block reward: ~0.208 LUMENYX per block
/// Calculated for 50% supply in 4 years with halving
pub const BLOCK_REWARD: u128 = 207_953_080_000;

/// Blocks per halving: 4 years exactly (12,623,040 * 4)
pub const BLOCKS_PER_HALVING: u32 = 50_492_160;

/// Minimum block reward before stopping emission (Bitcoin-like: can be 0)
pub const MINIMUM_BLOCK_REWARD: u128 = 0;

/// Base fee for simple transfer
pub const BASE_TRANSFER_FEE: u128 = 1_000_000;

/// Fee for smart contract execution
pub const BASE_CONTRACT_FEE: u128 = 10_000_000;

pub type BlockNumber = u32;
pub type Balance = u128;
pub type Nonce = u32;
pub type Index = u32;
pub type Hash = sp_core::H256;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

const MAX_HALVINGS: u32 = 128;

/// Bitcoin-like block reward:
/// - reward halves every BLOCKS_PER_HALVING blocks via right shift
/// - when the shift produces 0 (in smallest unit), emission stops (reward=0)
pub fn calculate_block_reward(block_number: BlockNumber) -> Balance {
    let halvings = block_number / BLOCKS_PER_HALVING;

    // Prevent shifting by >= 128 (would panic for u128 shifts).
    if halvings >= MAX_HALVINGS {
        return 0;
    }

    let reward = BLOCK_REWARD >> halvings;

    // With MINIMUM_BLOCK_REWARD = 0, this makes the schedule stop cleanly at 0.
    if reward <= MINIMUM_BLOCK_REWARD {
        0
    } else {
        reward
    }
}

pub fn calculate_supply_at_block(block_number: BlockNumber) -> Balance {
    let mut total: u128 = 0;
    let mut remaining_blocks: u32 = block_number;
    let mut current_reward: u128 = BLOCK_REWARD;
    let mut halving: u32 = 0;

    while remaining_blocks > 0 && current_reward > 0 && halving < MAX_HALVINGS {
        let blocks_in_era = remaining_blocks.min(BLOCKS_PER_HALVING);

        total = total.saturating_add(current_reward.saturating_mul(blocks_in_era as u128));

        remaining_blocks = remaining_blocks.saturating_sub(BLOCKS_PER_HALVING);
        halving = halving.saturating_add(1);
        current_reward = BLOCK_REWARD >> halving;
    }

    total.min(TOTAL_SUPPLY)
}

pub fn daily_emission(block_number: BlockNumber) -> Balance {
    calculate_block_reward(block_number) * BLOCKS_PER_DAY as u128
}

pub fn current_era(block_number: BlockNumber) -> u32 {
    block_number / BLOCKS_PER_HALVING
}

pub fn blocks_until_halving(block_number: BlockNumber) -> u32 {
    BLOCKS_PER_HALVING - (block_number % BLOCKS_PER_HALVING)
}

pub const GENESIS_MESSAGE: &str =
    "Bitcoin started with a headline. Ethereum started with a premine. LUMENYX starts with you.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_supply() {
        assert_eq!(TOTAL_SUPPLY, 21_000_000 * LUMENYX);
    }

    #[test]
    fn test_block_reward() {
        assert_eq!(calculate_block_reward(0), BLOCK_REWARD);
        assert_eq!(calculate_block_reward(BLOCKS_PER_HALVING), BLOCK_REWARD / 2);
        assert_eq!(calculate_block_reward(BLOCKS_PER_HALVING * 2), BLOCK_REWARD / 4);
    }

    #[test]
    fn test_halving_period() {
        // 4 years = 50,492,160 blocks
        assert_eq!(BLOCKS_PER_HALVING, 50_492_160);
    }

    #[test]
    fn test_daily_emission() {
        // ~7,187 LUMENYX per day
        let daily = daily_emission(0);
        let daily_lumenyx = daily / LUMENYX;
        assert!(daily_lumenyx >= 7000 && daily_lumenyx <= 7500);
    }

    #[test]
    fn test_reward_reaches_zero_eventually() {
        // It must be possible for reward to become 0 (Bitcoin-like).
        let very_late = BLOCKS_PER_HALVING.saturating_mul(200);
        assert_eq!(calculate_block_reward(very_late), 0);
    }
}
