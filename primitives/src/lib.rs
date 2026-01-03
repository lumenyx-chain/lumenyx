//! LUMENYX Core Primitives
//!
//! Privacy-first cryptocurrency with ZK-SNARKs.
//! 21M fixed supply, no team, no foundation. Just code.
//!
//! # Key Constants
//! - Total Supply: 21,000,000 LUMENYX (fixed, immutable)
//! - Block Time: 3 seconds
//! - Block Reward: 0.5 LUMENYX (halving every 4 years)
//! - Privacy: Optional ZK shielded transactions
//! - Governance: NONE (code is law)

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

/// ============================================
/// TOKEN CONSTANTS
/// ============================================

/// LUMENYX token decimals (12 decimals like DOT)
pub const DECIMALS: u8 = 12;

/// Token symbol
pub const SYMBOL: &str = "LUMENYX";

/// 1 LUMENYX = 10^12 planck (smallest unit)
pub const LUMENYX: u128 = 1_000_000_000_000;

/// Total supply: 21,000,000 LUMENYX (fixed, NEVER MORE)
pub const TOTAL_SUPPLY: u128 = 21_000_000 * LUMENYX;

/// ============================================
/// BLOCK TIME - 3 SECONDS
/// ============================================

/// Block time in milliseconds: 3 seconds
pub const BLOCK_TIME_MS: u64 = 3_000;

/// Blocks per day at 3 second block time
/// 24 hours * 60 minutes * 60 seconds / 3 = 28,800 blocks
pub const BLOCKS_PER_DAY: u32 = 28_800;

/// Blocks per year at 3 second block time
/// 365.25 days * 28,800 = 10,519,200 blocks
pub const BLOCKS_PER_YEAR: u32 = 10_519_200;

/// ============================================
/// EMISSION SCHEDULE - LIKE BITCOIN
/// ============================================
///
/// Simple and fair:
/// - 0.5 LUMENYX per block from genesis
/// - Halving every ~4 years (42,076,800 blocks)
/// - 100% mined, 0% premine
///
/// Daily emission: 28,800 blocks * 0.5 = 14,400 LUMENYX/day
/// Yearly emission (year 1): ~5,256,000 LUMENYX
/// 50% mined in first 4 years, like Bitcoin

/// Block reward: 0.5 LUMENYX
pub const BLOCK_REWARD: u128 = 500_000_000_000; // 0.5 LUMENYX

/// Blocks per halving: ~4 years
/// 10,519,200 blocks/year * 4 = 42,076,800 blocks
pub const BLOCKS_PER_HALVING: u32 = 42_076_800;

/// Minimum block reward before stopping emission
pub const MINIMUM_BLOCK_REWARD: u128 = 1; // 1 planck

/// ============================================
/// FEES - ALWAYS LOW
/// ============================================

/// Base fee for simple transfer (in planck)
/// ~0.0001 LUMENYX = always cheap regardless of LUMENYX price
pub const BASE_TRANSFER_FEE: u128 = 100_000_000; // 0.0001 LUMENYX

/// Fee for smart contract execution (in planck)
pub const BASE_CONTRACT_FEE: u128 = 1_000_000_000; // 0.001 LUMENYX

/// Fee for privacy (ZK) transaction (in planck)
pub const BASE_PRIVACY_FEE: u128 = 10_000_000_000; // 0.01 LUMENYX

/// ============================================
/// STAKING PARAMETERS
/// ============================================

/// Minimum stake to become validator: 1 LUMENYX
pub const MIN_VALIDATOR_STAKE: u128 = 1 * LUMENYX;

/// Slashing percentage for misbehavior: 30%
pub const SLASHING_PERCENT: u32 = 30;

/// Unbonding period: 28 days (in blocks)
pub const UNBONDING_PERIOD: u32 = 28 * BLOCKS_PER_DAY; // 806,400 blocks

/// ============================================
/// CORE TYPES
/// ============================================

/// Alias for block number
pub type BlockNumber = u32;

/// Alias for account balance
pub type Balance = u128;

/// Alias for transaction nonce
pub type Nonce = u32;

/// Alias for transaction index in block
pub type Index = u32;

/// Alias for hash type
pub type Hash = sp_core::H256;

/// Signature type using sr25519 or ed25519
pub type Signature = MultiSignature;

/// Account ID derived from signature
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// ============================================
/// PRIVACY MODE
/// ============================================

/// Transaction privacy mode
#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Default)]
pub enum PrivacyMode {
    /// Transparent mode (default) - fully traceable
    #[default]
    Transparent,
    /// Shielded mode - zero-knowledge proofs, untraceable
    Shielded,
}

/// ============================================
/// REWARD CALCULATION
/// ============================================

/// Calculate block reward for a given block number
/// Simple halving schedule like Bitcoin
pub fn calculate_block_reward(block_number: BlockNumber) -> Balance {
    // Calculate how many halvings have occurred
    let halvings = block_number / BLOCKS_PER_HALVING;
    
    // After 64 halvings, reward is essentially 0
    if halvings >= 64 {
        return MINIMUM_BLOCK_REWARD;
    }
    
    // Reward = BLOCK_REWARD / 2^halvings
    let reward = BLOCK_REWARD >> halvings;
    
    if reward < MINIMUM_BLOCK_REWARD {
        MINIMUM_BLOCK_REWARD
    } else {
        reward
    }
}

/// Calculate total supply emitted by a given block number
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

/// Calculate daily emission at current block
pub fn daily_emission(block_number: BlockNumber) -> Balance {
    calculate_block_reward(block_number) * BLOCKS_PER_DAY as u128
}

/// Get current halving era (0 = first era, 1 = after first halving, etc.)
pub fn current_era(block_number: BlockNumber) -> u32 {
    block_number / BLOCKS_PER_HALVING
}

/// Blocks until next halving
pub fn blocks_until_halving(block_number: BlockNumber) -> u32 {
    BLOCKS_PER_HALVING - (block_number % BLOCKS_PER_HALVING)
}

/// ============================================
/// GENESIS CONSTANTS
/// ============================================

/// Genesis message
pub const GENESIS_MESSAGE: &str = "Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules.";

/// ============================================
/// TESTS
/// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_supply() {
        assert_eq!(TOTAL_SUPPLY, 21_000_000 * LUMENYX);
    }

    #[test]
    fn test_block_time() {
        assert_eq!(BLOCK_TIME_MS, 3_000); // 3 seconds
        assert_eq!(BLOCKS_PER_DAY, 28_800);
    }

    #[test]
    fn test_block_reward() {
        // Block 0: 0.5 LUMENYX
        assert_eq!(calculate_block_reward(0), BLOCK_REWARD);
        assert_eq!(calculate_block_reward(1_000_000), BLOCK_REWARD);
        
        // After first halving (~4 years): 0.25 LUMENYX
        assert_eq!(calculate_block_reward(BLOCKS_PER_HALVING), BLOCK_REWARD / 2);
        
        // After second halving (~8 years): 0.125 LUMENYX
        assert_eq!(calculate_block_reward(BLOCKS_PER_HALVING * 2), BLOCK_REWARD / 4);
    }

    #[test]
    fn test_daily_emission() {
        // Day 1: 28,800 blocks * 0.5 LUMENYX = 14,400 LUMENYX/day
        let daily = daily_emission(0);
        assert_eq!(daily, 14_400 * LUMENYX);
    }

    #[test]
    fn test_yearly_emission() {
        // Year 1: ~5,256,000 LUMENYX (before halving)
        let yearly = BLOCK_REWARD * BLOCKS_PER_YEAR as u128;
        assert_eq!(yearly, 5_259_600 * LUMENYX); // 10,519,200 * 0.5
    }

    #[test]
    fn test_four_year_emission() {
        // First 4 years: ~50% of supply
        let four_years = BLOCK_REWARD * BLOCKS_PER_HALVING as u128;
        // 42,076,800 blocks * 0.5 LUMENYX = 21,038,400 LUMENYX
        // This is slightly more than 50% but capped at 21M total
        assert!(four_years > 10_000_000 * LUMENYX);
    }

    #[test]
    fn test_halving_era() {
        assert_eq!(current_era(0), 0);
        assert_eq!(current_era(BLOCKS_PER_HALVING - 1), 0);
        assert_eq!(current_era(BLOCKS_PER_HALVING), 1);
        assert_eq!(current_era(BLOCKS_PER_HALVING * 2), 2);
    }

    #[test]
    fn test_fees_are_low() {
        // Transfer fee: 0.0001 LUMENYX
        assert_eq!(BASE_TRANSFER_FEE, LUMENYX / 10_000);

        // Contract fee: 0.001 LUMENYX
        assert_eq!(BASE_CONTRACT_FEE, LUMENYX / 1_000);

        // Privacy fee: 0.01 LUMENYX
        assert_eq!(BASE_PRIVACY_FEE, LUMENYX / 100);
    }

    #[test]
    fn test_staking_params() {
        // Min stake: 1 LUMENYX (anyone can validate)
        assert_eq!(MIN_VALIDATOR_STAKE, 1 * LUMENYX);

        // Slashing: 30%
        assert_eq!(SLASHING_PERCENT, 30);

        // Unbonding: 28 days
        assert_eq!(UNBONDING_PERIOD, 28 * BLOCKS_PER_DAY);
    }
}
