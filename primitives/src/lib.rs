//! LUMO Core Primitives
//!
//! Decentralized cryptocurrency with fixed supply.
//! 21M fixed supply, no team, no foundation. Just code.
//!
//! Hard fork v2.3.0 at block 440,000:
//!   - Decimals 12 → 18 (standard EVM)
//!   - Ticker LUMENYX → LUMO
//!   - All balances multiplied ×10^6 on-chain

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

// ============================================
// HARD FORK v2.3.0 CONSTANTS
// ============================================

/// Block height at which v2.3.0 fork activates
pub const FORK_HEIGHT_V2: u32 = 440_000;

/// Multiplier for balance migration (10^6 to go from 12 to 18 decimals)
pub const DECIMAL_MIGRATION_FACTOR: u128 = 1_000_000;

// ============================================
// PRE-FORK CONSTANTS (block < 440,000)
// ============================================

/// Pre-fork: 12 decimals
pub const DECIMALS_PRE: u8 = 12;

/// Pre-fork: token symbol
pub const SYMBOL_PRE: &str = "LUMENYX";

/// Pre-fork: 1 LUMENYX = 10^12 planck
pub const LUMENYX_PRE: u128 = 1_000_000_000_000;

/// Pre-fork: block reward ~0.208 LUMENYX
pub const BLOCK_REWARD_PRE: u128 = 207_953_080_000;

/// Pre-fork: base fee for simple transfer
pub const BASE_TRANSFER_FEE_PRE: u128 = 1_000_000;

/// Pre-fork: fee for smart contract execution
pub const BASE_CONTRACT_FEE_PRE: u128 = 10_000_000;

// ============================================
// POST-FORK CONSTANTS (block >= 440,000)
// ============================================

/// Post-fork: 18 decimals (standard EVM)
pub const DECIMALS: u8 = 18;

/// Post-fork: token symbol
pub const SYMBOL: &str = "LUMO";

/// Post-fork: 1 LUMO = 10^18 planck
pub const LUMENYX: u128 = 1_000_000_000_000_000_000;

/// Post-fork: block reward (same human amount, more planck)
pub const BLOCK_REWARD: u128 = 207_953_080_000_000_000;

/// Post-fork: base fee for simple transfer (×10^6)
pub const BASE_TRANSFER_FEE: u128 = 1_000_000_000_000;

/// Post-fork: fee for smart contract execution (×10^6)
pub const BASE_CONTRACT_FEE: u128 = 10_000_000_000_000;

// ============================================
// SHARED CONSTANTS (unchanged across fork)
// ============================================

/// Total supply: 21,000,000 LUMO (fixed, NEVER MORE)
pub const TOTAL_SUPPLY: u128 = 21_000_000 * LUMENYX;

/// Block time in milliseconds: 2.5 seconds
pub const BLOCK_TIME_MS: u64 = 2_500;

/// Blocks per day (86400 / 2.5 = 34560)
pub const BLOCKS_PER_DAY: u32 = 34_560;

/// Blocks per year (34560 * 365.25 = 12,623,040)
pub const BLOCKS_PER_YEAR: u32 = 12_623_040;

/// Blocks per halving: 4 years exactly (12,623,040 * 4)
pub const BLOCKS_PER_HALVING: u32 = 50_492_160;

/// Minimum block reward before stopping emission (Bitcoin-like: can be 0)
pub const MINIMUM_BLOCK_REWARD: u128 = 0;

pub type BlockNumber = u32;
pub type Balance = u128;
pub type Nonce = u32;
pub type Index = u32;
pub type Hash = sp_core::H256;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

const MAX_HALVINGS: u32 = 128;

/// Get the correct block reward for a given block number (fork-aware)
fn base_block_reward(block_number: BlockNumber) -> Balance {
    if block_number < FORK_HEIGHT_V2 {
        BLOCK_REWARD_PRE
    } else {
        BLOCK_REWARD
    }
}

/// Bitcoin-like block reward:
/// - reward halves every BLOCKS_PER_HALVING blocks via right shift
/// - when the shift produces 0 (in smallest unit), emission stops (reward=0)
/// - fork-aware: uses pre/post fork base reward
pub fn calculate_block_reward(block_number: BlockNumber) -> Balance {
    let halvings = block_number / BLOCKS_PER_HALVING;

    // Prevent shifting by >= 128 (would panic for u128 shifts).
    if halvings >= MAX_HALVINGS {
        return 0;
    }

    let base = base_block_reward(block_number);
    let reward = base >> halvings;

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
    let mut halving: u32 = 0;

    while remaining_blocks > 0 && halving < MAX_HALVINGS {
        let blocks_in_era = remaining_blocks.min(BLOCKS_PER_HALVING);
        let current_reward = base_block_reward(block_number) >> halving;

        if current_reward == 0 {
            break;
        }

        total = total.saturating_add(current_reward.saturating_mul(blocks_in_era as u128));

        remaining_blocks = remaining_blocks.saturating_sub(BLOCKS_PER_HALVING);
        halving = halving.saturating_add(1);
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

/// Returns the correct decimals for a given block number
pub fn decimals_at_block(block_number: BlockNumber) -> u8 {
    if block_number < FORK_HEIGHT_V2 {
        DECIMALS_PRE
    } else {
        DECIMALS
    }
}

/// Returns the correct symbol for a given block number
pub fn symbol_at_block(block_number: BlockNumber) -> &'static str {
    if block_number < FORK_HEIGHT_V2 {
        SYMBOL_PRE
    } else {
        SYMBOL
    }
}

pub const GENESIS_MESSAGE: &str =
    "Bitcoin started with a headline. Ethereum started with a premine. LUMO starts with you.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_supply() {
        assert_eq!(TOTAL_SUPPLY, 21_000_000 * LUMENYX);
    }

    #[test]
    fn test_block_reward_pre_fork() {
        // Pre-fork: 12 decimal reward
        assert_eq!(calculate_block_reward(0), BLOCK_REWARD_PRE);
        assert_eq!(calculate_block_reward(FORK_HEIGHT_V2 - 1), BLOCK_REWARD_PRE);
    }

    #[test]
    fn test_block_reward_post_fork() {
        // Post-fork: 18 decimal reward
        assert_eq!(calculate_block_reward(FORK_HEIGHT_V2), BLOCK_REWARD);
        assert_eq!(calculate_block_reward(FORK_HEIGHT_V2 + 1), BLOCK_REWARD);
    }

    #[test]
    fn test_block_reward_halving() {
        assert_eq!(calculate_block_reward(BLOCKS_PER_HALVING), BLOCK_REWARD / 2);
        assert_eq!(
            calculate_block_reward(BLOCKS_PER_HALVING * 2),
            BLOCK_REWARD / 4
        );
    }

    #[test]
    fn test_halving_period() {
        // 4 years = 50,492,160 blocks
        assert_eq!(BLOCKS_PER_HALVING, 50_492_160);
    }

    #[test]
    fn test_daily_emission() {
        // Post-fork: ~7,187 LUMO per day
        let daily = daily_emission(FORK_HEIGHT_V2);
        let daily_lumo = daily / LUMENYX;
        assert!(daily_lumo >= 7000 && daily_lumo <= 7500);
    }

    #[test]
    fn test_reward_reaches_zero_eventually() {
        let very_late = BLOCKS_PER_HALVING.saturating_mul(200);
        assert_eq!(calculate_block_reward(very_late), 0);
    }

    #[test]
    fn test_decimal_migration_factor() {
        // 10^18 / 10^12 = 10^6
        assert_eq!(LUMENYX / LUMENYX_PRE, DECIMAL_MIGRATION_FACTOR);
    }

    #[test]
    fn test_fork_constants() {
        assert_eq!(FORK_HEIGHT_V2, 440_000);
        assert_eq!(DECIMALS_PRE, 12);
        assert_eq!(DECIMALS, 18);
    }
}
