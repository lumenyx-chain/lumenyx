//! DAG Sync Module for LUMENYX
//! 
//! Handles:
//! - Orphan pool for blocks waiting for parents
//! - Topological processing (process only when all parents ready)
//! - Request missing parents via network
//! - Cascade trigger when parent arrives

use sp_core::H256;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Block data stored in orphan pool
#[derive(Clone)]
pub struct OrphanBlock {
    pub hash: H256,
    pub header_parent: H256,
    pub all_parents: Vec<H256>,
    pub block_data: Vec<u8>,
}

/// DAG Orphan Pool - stores blocks waiting for missing parents
pub struct DagOrphanPool {
    /// Orphan blocks by their hash
    orphans: HashMap<H256, OrphanBlock>,
    /// Map from missing parent hash -> list of children waiting for it
    waiting_for_parent: HashMap<H256, Vec<H256>>,
    /// Count of missing parents for each orphan
    missing_count: HashMap<H256, usize>,
    /// Parents we've already requested (to avoid duplicate requests)
    requested_parents: HashSet<H256>,
    /// Parents that need to be requested
    pending_requests: Vec<H256>,
}

impl DagOrphanPool {
    pub fn new() -> Self {
        Self {
            orphans: HashMap::new(),
            waiting_for_parent: HashMap::new(),
            missing_count: HashMap::new(),
            requested_parents: HashSet::new(),
            pending_requests: Vec::new(),
        }
    }

    /// Add a block to orphan pool
    /// Returns list of missing parent hashes that need to be requested
    pub fn add_orphan(&mut self, block: OrphanBlock, missing_parents: Vec<H256>) -> Vec<H256> {
        let hash = block.hash;
        let missing_count = missing_parents.len();
        
        if missing_count == 0 {
            return vec![];
        }

        log::info!(
            "🔶 Orphan {:?}: {} missing parents",
            hash, missing_count
        );

        // Store the orphan
        self.orphans.insert(hash, block);
        self.missing_count.insert(hash, missing_count);

        // Track which parents this block is waiting for
        let mut to_request = Vec::new();
        for parent in missing_parents {
            self.waiting_for_parent
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(hash);
            
            // Only request if we haven't already
            if !self.requested_parents.contains(&parent) {
                self.requested_parents.insert(parent);
                to_request.push(parent);
                self.pending_requests.push(parent);
            }
        }

        to_request
    }

    /// Called when a parent block arrives
    /// Returns list of orphans that are now ready to process
    pub fn parent_arrived(&mut self, parent_hash: H256) -> Vec<OrphanBlock> {
        let mut ready_blocks = Vec::new();

        // Remove from requested set
        self.requested_parents.remove(&parent_hash);

        // Get all children waiting for this parent
        if let Some(children) = self.waiting_for_parent.remove(&parent_hash) {
            for child_hash in children {
                // Decrement missing count
                if let Some(count) = self.missing_count.get_mut(&child_hash) {
                    *count = count.saturating_sub(1);
                    
                    if *count == 0 {
                        // All parents present! Remove from orphan pool
                        self.missing_count.remove(&child_hash);
                        if let Some(orphan) = self.orphans.remove(&child_hash) {
                            log::info!(
                                "🔷 Orphan {:?} ready (parent {:?} arrived)",
                                child_hash, parent_hash
                            );
                            ready_blocks.push(orphan);
                        }
                    }
                }
            }
        }

        ready_blocks
    }

    /// Get and clear pending requests
    pub fn take_pending_requests(&mut self) -> Vec<H256> {
        std::mem::take(&mut self.pending_requests)
    }

    /// Check if a block is in the orphan pool
    pub fn contains(&self, hash: &H256) -> bool {
        self.orphans.contains_key(hash)
    }

    /// Get number of orphans
    pub fn len(&self) -> usize {
        self.orphans.len()
    }

    /// Check if we've already requested a parent
    pub fn is_requested(&self, hash: &H256) -> bool {
        self.requested_parents.contains(hash)
    }

    /// Get number of pending requests
    pub fn pending_request_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Clear old orphans (cleanup)
    pub fn clear_old(&mut self, max_orphans: usize) {
        if self.orphans.len() > max_orphans {
            log::warn!("🔶 Orphan pool overflow ({}), clearing oldest", self.orphans.len());
            // Simple: just clear everything if too many
            self.orphans.clear();
            self.waiting_for_parent.clear();
            self.missing_count.clear();
            self.requested_parents.clear();
            self.pending_requests.clear();
        }
    }
}

/// Thread-safe wrapper
pub type SharedOrphanPool = Arc<RwLock<DagOrphanPool>>;

pub fn new_shared_orphan_pool() -> SharedOrphanPool {
    Arc::new(RwLock::new(DagOrphanPool::new()))
}
