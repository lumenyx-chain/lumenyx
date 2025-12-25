//! LUMENYX Core Primitives
//! 
//! The only blockchain with everything:
//! - Fixed supply (21M)
//! - BNB speed (3 second blocks)
//! - Privacy (ZK optional)
//! - Smart contracts (EVM compatible)
//! - True decentralization (fair launch)
//! 
//! # Key Constants
//! - Total Supply: 21,000,000 LUMENYX (fixed, immutable)
//! - Block Time: 3 seconds
//! - Emission: 3 phases (Bootstrap → Early → Standard)
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
/// BLOCK TIME - 3 SECONDS (FAST LIKE BNB)
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
/// EMISSION SCHEDULE - 3 PHASES
/// ============================================
/// 
/// Phase 0 (Bootstrap): ~12 days pre-launch (350,000 blocks)
///   - Reward: 2.4 LUMENYX per block
///   - Purpose: Network security, initial validators
///   - Documented in whitepaper (transparent)
/// 
/// Phase 1 (Early Adoption): 30 days post-launch
///   - Reward: 0.3 LUMENYX per block
///   - Purpose: Incentivize early adopters
/// 
/// Phase 2 (Standard): Forever
///   - Reward: 0.25 LUMENYX per block
///   - Emission: 7,200 LUMENYX/day (standard emission)
///   - Halving: Every ~4 years

/// Phase 0: Bootstrap - 350,000 blocks (~12 days, 4% founder)
pub const PHASE_0_BLOCKS: u32 = 350_000; // 350,000 blocks (4% pre-mining)
pub const PHASE_0_REWARD: u128 = 2_400_000_000_000; // 2.4 LUMENYX

/// Phase 1: Early Adoption - 30 days after launch
pub const PHASE_1_BLOCKS: u32 = 30 * BLOCKS_PER_DAY; // 864,000 blocks
pub const PHASE_1_REWARD: u128 = 300_000_000_000; // 0.3 LUMENYX

/// Phase 2: Standard - Forever (with halving)
pub const PHASE_2_REWARD: u128 = 250_000_000_000; // 0.25 LUMENYX

/// End of Phase 0 (block number)
pub const PHASE_0_END: u32 = PHASE_0_BLOCKS; // Block 350,000

/// End of Phase 1 (block number)  
pub const PHASE_1_END: u32 = PHASE_0_BLOCKS + PHASE_1_BLOCKS; // Block 1,214,000

/// ============================================
/// HALVING (Phase 2 only)
/// ============================================

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
/// EMISSION PHASE ENUM
/// ============================================

/// Represents the current emission phase
#[derive(Clone, Copy, Debug, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum EmissionPhase {
    /// Phase 0: Bootstrap (~12 days pre-launch, 2.4 LUMENYX/block)
    Bootstrap,
    /// Phase 1: Early Adoption (30 days post-launch, 0.3 LUMENYX/block)
    EarlyAdoption,
    /// Phase 2: Standard (forever, 0.25 LUMENYX/block with halving)
    Standard,
}

impl EmissionPhase {
    /// Determine the phase based on block number
    pub fn from_block(block: BlockNumber) -> Self {
        if block < PHASE_0_END {
            EmissionPhase::Bootstrap
        } else if block < PHASE_1_END {
            EmissionPhase::EarlyAdoption
        } else {
            EmissionPhase::Standard
        }
    }
    
    /// Get the base reward for this phase (before halving)
    pub fn base_reward(&self) -> Balance {
        match self {
            EmissionPhase::Bootstrap => PHASE_0_REWARD,
            EmissionPhase::EarlyAdoption => PHASE_1_REWARD,
            EmissionPhase::Standard => PHASE_2_REWARD,
        }
    }
}

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
pub fn calculate_block_reward(block_number: BlockNumber) -> Balance {
    let phase = EmissionPhase::from_block(block_number);
    
    match phase {
        EmissionPhase::Bootstrap => PHASE_0_REWARD,
        EmissionPhase::EarlyAdoption => PHASE_1_REWARD,
        EmissionPhase::Standard => {
            // Calculate halvings since Phase 2 started
            let blocks_since_phase2 = block_number.saturating_sub(PHASE_1_END);
            let halvings = blocks_since_phase2 / BLOCKS_PER_HALVING;
            
            // After ~50 halvings, reward becomes negligible
            if halvings >= 50 {
                return MINIMUM_BLOCK_REWARD;
            }
            
            // Reward = Phase2Reward / 2^halvings
            let reward = PHASE_2_REWARD >> halvings;
            
            if reward < MINIMUM_BLOCK_REWARD {
                MINIMUM_BLOCK_REWARD
            } else {
                reward
            }
        }
    }
}

/// Calculate total supply emitted by a given block number
pub fn calculate_supply_at_block(block_number: BlockNumber) -> Balance {
    let mut total = 0u128;
    
    // Phase 0 contribution
    if block_number > 0 {
        let phase0_blocks = block_number.min(PHASE_0_END);
        total += PHASE_0_REWARD * phase0_blocks as u128;
    }
    
    // Phase 1 contribution
    if block_number > PHASE_0_END {
        let phase1_blocks = (block_number - PHASE_0_END).min(PHASE_1_BLOCKS);
        total += PHASE_1_REWARD * phase1_blocks as u128;
    }
    
    // Phase 2 contribution (with halving)
    if block_number > PHASE_1_END {
        let mut remaining = block_number - PHASE_1_END;
        let mut current_reward = PHASE_2_REWARD;
        let mut halving = 0u32;
        
        while remaining > 0 && current_reward >= MINIMUM_BLOCK_REWARD {
            let blocks_in_era = remaining.min(BLOCKS_PER_HALVING);
            total += current_reward * blocks_in_era as u128;
            remaining = remaining.saturating_sub(BLOCKS_PER_HALVING);
            halving += 1;
            current_reward = PHASE_2_REWARD >> halving;
        }
    }
    
    total.min(TOTAL_SUPPLY)
}

/// Calculate daily emission for current phase
pub fn daily_emission(block_number: BlockNumber) -> Balance {
    calculate_block_reward(block_number) * BLOCKS_PER_DAY as u128
}

/// ============================================
/// GENESIS CONSTANTS
/// ============================================

/// Genesis timestamp: 25 December 2025, 12:00:00 UTC
pub const GENESIS_TIMESTAMP: u64 = 1766664000; // 25 December 2025, 12:00:00 UTC

/// Genesis message (to be determined at launch)
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
    fn test_phase_detection() {
        // Phase 0
        assert_eq!(EmissionPhase::from_block(0), EmissionPhase::Bootstrap);
        assert_eq!(EmissionPhase::from_block(100_000), EmissionPhase::Bootstrap);
        assert_eq!(EmissionPhase::from_block(PHASE_0_END - 1), EmissionPhase::Bootstrap);
        
        // Phase 1
        assert_eq!(EmissionPhase::from_block(PHASE_0_END), EmissionPhase::EarlyAdoption);
        assert_eq!(EmissionPhase::from_block(PHASE_1_END - 1), EmissionPhase::EarlyAdoption);
        
        // Phase 2
        assert_eq!(EmissionPhase::from_block(PHASE_1_END), EmissionPhase::Standard);
        assert_eq!(EmissionPhase::from_block(PHASE_1_END + 1_000_000), EmissionPhase::Standard);
    }
    
    #[test]
    fn test_phase_rewards() {
        // Phase 0: 2.4 LUMENYX
        assert_eq!(calculate_block_reward(0), PHASE_0_REWARD);
        assert_eq!(calculate_block_reward(100_000), PHASE_0_REWARD);
        
        // Phase 1: 0.3 LUMENYX
        assert_eq!(calculate_block_reward(PHASE_0_END), PHASE_1_REWARD);
        
        // Phase 2: 0.25 LUMENYX
        assert_eq!(calculate_block_reward(PHASE_1_END), PHASE_2_REWARD);
        
        // Phase 2 after 1 halving: 0.125 LUMENYX
        assert_eq!(calculate_block_reward(PHASE_1_END + BLOCKS_PER_HALVING), PHASE_2_REWARD / 2);
    }
    
    #[test]
    fn test_phase0_emission() {
        // Phase 0: 350,000 blocks * 2.4 LUMENYX = 840,000 LUMENYX (4.0%)
        let phase0_total = PHASE_0_BLOCKS as u128 * PHASE_0_REWARD;
        assert_eq!(phase0_total, 840_000 * LUMENYX);
    }
    
    #[test]
    fn test_phase1_emission() {
        // Phase 1: 30 days * 28,800 blocks * 0.3 LUMENYX = 259,200 LUMENYX
        let phase1_total = PHASE_1_BLOCKS as u128 * PHASE_1_REWARD;
        assert_eq!(phase1_total, 259_200 * LUMENYX);
    }
    
    #[test]
    fn test_daily_emission_phase2() {
        // Phase 2: 28,800 blocks * 0.25 LUMENYX = 7,200 LUMENYX/day (standard emission)
        let daily = BLOCKS_PER_DAY as u128 * PHASE_2_REWARD;
        assert_eq!(daily, 7_200 * LUMENYX);
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
