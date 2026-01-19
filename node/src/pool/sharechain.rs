//! Sharechain - DAG of pool shares
//!
//! Tracks shares from all miners, maintains best tip, provides window for PPLNS.

use super::types::PoolShare;
use sp_core::H256;
use std::collections::HashMap;

/// Share chain state - a simple DAG of shares
pub struct Sharechain {
    /// All known shares by ID
    shares: HashMap<H256, PoolShare>,
    /// Best tip (highest cumulative work)
    best_tip: Option<H256>,
    /// Cumulative work at each share
    cumulative_work: HashMap<H256, u128>,
}

impl Sharechain {
    pub fn new() -> Self {
        Self {
            shares: HashMap::new(),
            best_tip: None,
            cumulative_work: HashMap::new(),
        }
    }

    /// Insert a share, returns true if new and valid
    pub fn insert(&mut self, share: PoolShare) -> bool {
        // Already known?
        if self.shares.contains_key(&share.id) {
            return false;
        }

        // Validate ID
        if !share.validate_id() {
            log::warn!("🏊 Share ID validation failed: {:?}", share.id);
            return false;
        }

        // Calculate cumulative work
        let prev_work = if share.prev == H256::zero() {
            0u128
        } else {
            self.cumulative_work.get(&share.prev).copied().unwrap_or(0)
        };

        // Work = share_difficulty (higher difficulty = more work)
        let work = prev_work.saturating_add(share.share_difficulty);

        let share_id = share.id;
        self.shares.insert(share_id, share);
        self.cumulative_work.insert(share_id, work);

        // Update best tip if this has more work
        let update_tip = match self.best_tip {
            None => true,
            Some(current_tip) => {
                let current_work = self.cumulative_work.get(&current_tip).copied().unwrap_or(0);
                work > current_work
            }
        };

        if update_tip {
            self.best_tip = Some(share_id);
        }

        true
    }

    /// Get the best tip share ID
    pub fn best_tip(&self) -> Option<H256> {
        self.best_tip
    }

    /// Get a share by ID
    pub fn get(&self, id: &H256) -> Option<&PoolShare> {
        self.shares.get(id)
    }

    /// Walk back from a tip, collecting up to `count` shares
    /// Returns shares in reverse order (newest first)
    pub fn walk_back(&self, tip: H256, count: usize) -> Vec<PoolShare> {
        let mut result = Vec::with_capacity(count);
        let mut current = tip;

        while result.len() < count {
            if current == H256::zero() {
                break;
            }

            match self.shares.get(&current) {
                Some(share) => {
                    result.push(share.clone());
                    current = share.prev;
                }
                None => break,
            }
        }

        result
    }

    /// Total number of shares in the chain
    pub fn len(&self) -> usize {
        self.shares.len()
    }

    /// Is the sharechain empty?
    pub fn is_empty(&self) -> bool {
        self.shares.is_empty()
    }
}

impl Default for Sharechain {
    fn default() -> Self {
        Self::new()
    }
}
