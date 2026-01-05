//! LUMENYX Core Primitives
//!
//! Privacy-first cryptocurrency with ZK-SNARKs.
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

/// Block time in milliseconds: ~1 second (GHOSTDAG)
pub const BLOCK_TIME_MS: u64 = 1_000;

/// Blocks per day at 1 second block time
pub const BLOCKS_PER_DAY: u32 = 86_400;

/// Blocks per year at 1 second block time
pub const BLOCKS_PER_YEAR: u32 = 31_557_600;

/// Block reward: 0.083 LUMENYX per block
/// ~50% mined in first 4 years like Bitcoin
pub const BLOCK_REWARD: u128 = 83_181_230_512;

/// Blocks per halving: ~4 years at 1 second blocks
pub const BLOCKS_PER_HALVING: u32 = 126_230_400;

/// Minimum block reward before stopping emission
pub const MINIMUM_BLOCK_REWARD: u128 = 1;

/// Base fee for simple transfer: ~0.00000005 LUMENYX
pub const BASE_TRANSFER_FEE: u128 = 1_000_000;

/// Fee for smart contract execution
pub const BASE_CONTRACT_FEE: u128 = 10_000_000;

/// Fee for privacy (ZK) transaction
pub const BASE_PRIVACY_FEE: u128 = 100_000_000;

/// Minimum stake to become validator: 1 LUMENYX
pub const MIN_VALIDATOR_STAKE: u128 = 1 * LUMENYX;

/// Slashing percentage for misbehavior: 30%
pub const SLASHING_PERCENT: u32 = 30;

/// Unbonding period: 28 days (in blocks)
pub const UNBONDING_PERIOD: u32 = 28 * BLOCKS_PER_DAY;

pub type BlockNumber = u32;
pub type Balance = u128;
pub type Nonce = u32;
pub type Index = u32;
pub type Hash = sp_core::H256;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Default)]
pub enum PrivacyMode {
    #[default]
    Transparent,
    Shielded,
}

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
    fn test_daily_emission() {
        let daily = daily_emission(0);
        assert_eq!(daily, 83_000_000_000 * 86_400);
    }
}
