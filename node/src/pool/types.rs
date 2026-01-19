//! Pool Types - Share and Payout structures

use codec::{Decode, Encode};
use sp_core::H256;

/// Account ID type for pool (raw 32 bytes)
pub type PoolAccountId = [u8; 32];

/// A share submitted to the pool
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct PoolShare {
    /// Unique share ID (hash of share contents)
    pub id: H256,
    /// Previous share ID in this miner's chain (or zero for first share)
    pub prev: H256,
    /// Main chain parent block this share is built on
    pub main_parent: H256,
    /// Miner address (32 bytes, raw AccountId)
    pub miner: [u8; 32],
    /// Share difficulty (lower than main chain difficulty)
    pub share_difficulty: u128,
    /// Nonce that satisfies share_difficulty
    pub nonce: [u8; 32],
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

impl PoolShare {
    /// Compute share ID from contents
    pub fn compute_id(&self) -> H256 {
        let mut data = Vec::new();
        data.extend_from_slice(self.prev.as_ref());
        data.extend_from_slice(self.main_parent.as_ref());
        data.extend_from_slice(&self.miner);
        data.extend_from_slice(&self.share_difficulty.to_le_bytes());
        data.extend_from_slice(&self.nonce);
        data.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        sp_core::blake2_256(&data).into()
    }

    /// Validate share ID matches computed value
    pub fn validate_id(&self) -> bool {
        self.id == self.compute_id()
    }
}

/// A payout entry for PPLNS distribution
#[derive(Clone, Debug, Encode, Decode)]
pub struct PoolPayoutEntry {
    /// Account to pay (raw 32 bytes)
    pub account: [u8; 32],
    /// Amount in smallest units (planck)
    pub amount: u128,
}
