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

/// Minimum block reward before stopping emission
pub const MINIMUM_BLOCK_REWARD: u128 = 1;

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

pub fn calculate_block_reward(block_number: BlockNumber) -> Balance {
    let halvings = block_number / BLOCKS_PER_HALVING;
    if halvings >= 64 {
        return MINIMUM_BLOCK_REWARD;
    }
    let reward = BLOCK_REWARD >> halvings;
    if reward < MINIMUM_BLOCK_REWARD {
        MINIMUM_BLOCK_REWARD
    } else {
        reward
    }
}

pub fn calculate_supply_at_block(block_number: BlockNumber) -> Balance {
    let mut total = 0u128;
    let mut remaining_blocks = block_number;
    let mut current_reward = BLOCK_REWARD;
    let mut halving = 0u32;
    while remaining_blocks > 0 && current_reward >= MINIMUM_BLOCK_REWARD && halving < 64 {
        let blocks_in_era = remaining_blocks.min(BLOCKS_PER_HALVING);
        total += current_reward * blocks_in_era as u128;
        remaining_blocks = remaining_blocks.saturating_sub(BLOCKS_PER_HALVING);
        halving += 1;
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

pub const GENESIS_MESSAGE: &str = "Banks ended up in the headlines. Today control over digital money sits in a few hands.";

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
}
