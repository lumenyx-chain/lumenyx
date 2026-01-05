//! Simple orphan pool for blocks waiting for missing parents
//! Network fetch is handled separately by DagSync

use sp_core::H256;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;

/// Block data stored in orphan pool
#[derive(Clone)]
pub struct OrphanBlock {
    pub hash: H256,
    pub header_parent: H256,
    pub all_parents: Vec<H256>,
    pub block_data: Vec<u8>,
}

/// Simple orphan pool
pub struct OrphanPool {
    orphans: HashMap<H256, OrphanBlock>,
    waiting_for_parent: HashMap<H256, Vec<H256>>,
    missing_count: HashMap<H256, usize>,
}

impl OrphanPool {
    pub fn new() -> Self {
        Self {
            orphans: HashMap::new(),
            waiting_for_parent: HashMap::new(),
            missing_count: HashMap::new(),
        }
    }

    pub fn add_orphan(&mut self, block: OrphanBlock, missing_parents: Vec<H256>) -> Vec<H256> {
        let hash = block.hash;
        let missing_count = missing_parents.len();
        
        if missing_count == 0 {
            return vec![];
        }

        self.orphans.insert(hash, block);
        self.missing_count.insert(hash, missing_count);

        let mut to_request = Vec::new();
        for parent in missing_parents {
            self.waiting_for_parent
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(hash);
            to_request.push(parent);
        }

        to_request
    }

    pub fn parent_arrived(&mut self, parent_hash: H256) -> Vec<OrphanBlock> {
        let mut ready_blocks = Vec::new();

        if let Some(children) = self.waiting_for_parent.remove(&parent_hash) {
            for child_hash in children {
                if let Some(count) = self.missing_count.get_mut(&child_hash) {
                    *count = count.saturating_sub(1);
                    
                    if *count == 0 {
                        self.missing_count.remove(&child_hash);
                        if let Some(orphan) = self.orphans.remove(&child_hash) {
                            ready_blocks.push(orphan);
                        }
                    }
                }
            }
        }

        ready_blocks
    }

    pub fn len(&self) -> usize {
        self.orphans.len()
    }
}

pub type SharedOrphanPool = Arc<RwLock<OrphanPool>>;

pub fn new_shared_orphan_pool() -> SharedOrphanPool {
    Arc::new(RwLock::new(OrphanPool::new()))
}
