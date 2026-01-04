//! DAG Graph - manages block relationships and reachability queries

use sp_core::H256;
use std::collections::{HashMap, HashSet, VecDeque};
use parking_lot::RwLock;
use std::sync::Arc;

use crate::store::GhostdagStore;

/// DAG Manager - handles graph operations and reachability
pub struct DagManager<C> {
    store: GhostdagStore<C>,
    /// In-memory cache for fast reachability queries
    reachability_cache: Arc<RwLock<HashMap<(H256, H256), bool>>>,
    /// K parameter for GHOSTDAG (anticone size limit)
    pub k: u64,
}

impl<C> DagManager<C> {
    pub fn new(store: GhostdagStore<C>, k: u64) -> Self {
        Self {
            store,
            reachability_cache: Arc::new(RwLock::new(HashMap::new())),
            k,
        }
    }
}

impl<C: sc_client_api::AuxStore> DagManager<C> {
    /// Check if block A is in the past of block B (A is ancestor of B)
    pub fn is_in_past(&self, ancestor: &H256, descendant: &H256) -> bool {
        if ancestor == descendant {
            return true;
        }

        // Check cache first
        let cache_key = (*ancestor, *descendant);
        if let Some(&result) = self.reachability_cache.read().get(&cache_key) {
            return result;
        }

        // BFS from descendant going backwards through parents
        let result = self.bfs_reachability(ancestor, descendant);
        
        // Cache result
        self.reachability_cache.write().insert(cache_key, result);
        
        result
    }

    /// BFS to check if ancestor is reachable from descendant
    fn bfs_reachability(&self, ancestor: &H256, descendant: &H256) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(*descendant);

        while let Some(current) = queue.pop_front() {
            if current == *ancestor {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Get parents and continue search
            if let Some(parents) = self.store.get_parents(&current) {
                for parent in parents {
                    if !visited.contains(&parent) {
                        queue.push_back(parent);
                    }
                }
            }
        }

        false
    }

    /// Get the past set of a block (all ancestors)
    pub fn get_past(&self, block: &H256) -> HashSet<H256> {
        let mut past = HashSet::new();
        let mut queue = VecDeque::new();
        
        if let Some(parents) = self.store.get_parents(block) {
            for parent in parents {
                queue.push_back(parent);
            }
        }

        while let Some(current) = queue.pop_front() {
            if past.contains(&current) {
                continue;
            }
            past.insert(current);

            if let Some(parents) = self.store.get_parents(&current) {
                for parent in parents {
                    if !past.contains(&parent) {
                        queue.push_back(parent);
                    }
                }
            }
        }

        past
    }

    /// Get the anticone of a block relative to another block
    /// Anticone(B, A) = blocks not in past(A) and not in future(A) from B's perspective
    pub fn get_anticone(&self, block: &H256, reference: &H256) -> HashSet<H256> {
        let past_of_block = self.get_past(block);
        let past_of_reference = self.get_past(reference);
        
        // Anticone = past(block) - past(reference) - {reference}
        let mut anticone: HashSet<H256> = past_of_block
            .difference(&past_of_reference)
            .cloned()
            .collect();
        anticone.remove(reference);
        
        anticone
    }

    /// Get mergeset - blocks to be merged when adding a new block
    /// Mergeset = blocks in past(new_block) but not in past(selected_parent)
    pub fn get_mergeset(&self, block: &H256, selected_parent: &H256) -> HashSet<H256> {
        let past_of_block = self.get_past(block);
        let past_of_parent = self.get_past(selected_parent);
        
        past_of_block
            .difference(&past_of_parent)
            .cloned()
            .collect()
    }

    /// Get all current tips (blocks with no children)
    pub fn get_tips(&self) -> Vec<H256> {
        self.store.get_tips()
    }

    /// Select parents for a new block (up to max_parents tips)
    pub fn select_parents(&self, max_parents: usize) -> Vec<H256> {
        let tips = self.get_tips();
        if tips.is_empty() {
            return vec![];
        }

        // Sort tips by blue_work (highest first)
        let mut sorted_tips: Vec<(H256, u128)> = tips
            .iter()
            .filter_map(|t| {
                self.store.get_blue_work(t).map(|w| (*t, w))
            })
            .collect();
        
        sorted_tips.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top max_parents
        sorted_tips
            .into_iter()
            .take(max_parents)
            .map(|(h, _)| h)
            .collect()
    }

    /// Find selected parent (parent with highest blue_work)
    pub fn find_selected_parent(&self, parents: &[H256]) -> Option<H256> {
        if parents.is_empty() {
            return None;
        }

        parents
            .iter()
            .filter_map(|p| {
                self.store.get_blue_work(p).map(|w| (*p, w))
            })
            .max_by_key(|(_, w)| *w)
            .map(|(h, _)| h)
    }

    /// Add a new block to the DAG
    pub fn add_block(&self, block_hash: H256, parents: Vec<H256>) -> Result<(), String> {
        // Store parents
        self.store.store_parents(&block_hash, &parents)?;

        // Update children for each parent
        for parent in &parents {
            self.store.add_child(parent, &block_hash)?;
        }

        // Update tips
        self.store.update_tips(&block_hash, &parents)?;

        // Clear relevant cache entries
        self.reachability_cache.write().retain(|k, _| {
            k.0 != block_hash && k.1 != block_hash
        });

        Ok(())
    }

    /// Check if adding a block would violate k-cluster property
    pub fn check_k_cluster(&self, block: &H256, parents: &[H256]) -> bool {
        for parent in parents {
            let anticone = self.get_anticone(block, parent);
            if anticone.len() as u64 > self.k {
                return false;
            }
        }
        true
    }

    /// Get virtual selected parent chain (for ordering)
    pub fn get_selected_chain(&self) -> Vec<H256> {
        let tips = self.get_tips();
        if tips.is_empty() {
            return vec![];
        }

        // Find tip with highest blue_work
        let best_tip = tips
            .iter()
            .filter_map(|t| {
                self.store.get_blue_work(t).map(|w| (*t, w))
            })
            .max_by_key(|(_, w)| *w)
            .map(|(h, _)| h);

        let Some(mut current) = best_tip else {
            return vec![];
        };

        // Walk back through selected parents
        let mut chain = vec![current];
        while let Some(data) = self.store.get_ghostdag_data(&current) {
            if data.selected_parent == H256::zero() {
                break; // Genesis
            }
            chain.push(data.selected_parent);
            current = data.selected_parent;
        }

        chain.reverse();
        chain
    }
}

impl<C> Clone for DagManager<C> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            reachability_cache: self.reachability_cache.clone(),
            k: self.k,
        }
    }
}
