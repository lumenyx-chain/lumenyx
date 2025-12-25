//! # LUMENYX EVM Configuration
//!
//! Provides EVM configuration constants for LUMENYX blockchain.
//! Full EVM integration will be added in a future update.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::H160;

/// EVM chain ID for LUMENYX mainnet
pub const CHAIN_ID: u64 = 7926;

/// Gas price in wei
pub const GAS_PRICE: u128 = 1_000_000_000;

/// Block gas limit
pub const BLOCK_GAS_LIMIT: u64 = 15_000_000;

/// Maximum gas per transaction
pub const MAX_TX_GAS: u64 = 10_000_000;

/// Check if an address is a precompile
pub fn is_precompile(address: H160) -> bool {
    let addr_bytes = address.as_bytes();
    addr_bytes[0..19] == [0u8; 19] && addr_bytes[19] >= 1 && addr_bytes[19] <= 9
}

/// LUMENYX-specific precompile addresses
pub mod lumenyx_precompiles {
    use sp_core::H160;
    
    pub const ANTI_WHALE: H160 = H160([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0
    ]);
    
    pub const PRIVACY_SHIELD: H160 = H160([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 1
    ]);
    
    pub const QUADRATIC_VOTING: H160 = H160([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 2
    ]);
}

/// Gas costs for LUMENYX operations
pub mod gas_costs {
    pub const TRANSFER: u64 = 21_000;
    pub const CREATE: u64 = 53_000;
    pub const SSTORE: u64 = 20_000;
    pub const SLOAD: u64 = 800;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_id() {
        assert_eq!(CHAIN_ID, 7926);
    }
    
    #[test]
    fn test_gas_costs() {
        assert_eq!(gas_costs::TRANSFER, 21_000);
    }
}
