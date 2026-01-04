//! Topological Ordering - linearizes DAG for transaction execution
//! Produces deterministic order: blue blocks first, then red, by blue_work + hash

use sc_client_api::AuxStore;
use sp_core::H256;
use std::collections::{HashSet, VecDeque, BinaryHeap};
use std::cmp::Ordering;

use crate::store::GhostdagStore;
use crate::ghostdag::GhostdagManager;

/// Block for ordering (sortable by blue_work, then hash)
#[derive(Eq, PartialEq, Clone)]
struct OrderedBlock {
    hash: H256,
    blue_work: u128,
    is_blue: bool,
}

impl Ord for OrderedBlock {
    fn cmp(&self, other: &Self) -> Ordering {
        // Blues before reds
        match (self.is_blue, other.is_blue) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }
        // Higher blue_work first
        match self.blue_work.cmp(&other.blue_work) {
            Ordering::Equal => other.hash.cmp(&self.hash), // Lower hash wins
            other => other,
        }
    }
}

impl PartialOrd for OrderedBlock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Ordering Manager - produces linear order from DAG
pub struct OrderingManager<C> {
    store: GhostdagStore<C>,
    ghostdag: GhostdagManager<C>,
}

impl<C> OrderingManager<C> {
    pub fn new(store: GhostdagStore<C>, ghostdag: GhostdagManager<C>) -> Self {
        Self { store, ghostdag }
    }
}

impl<C: sc_client_api::AuxStore> OrderingManager<C> {
    /// Get topological order of blocks from genesis to tip
    /// This is the canonical order for transaction execution
    pub fn get_topological_order(&self, tip: &H256) -> Vec<H256> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        
        self.topological_sort_recursive(tip, &mut order, &mut visited);
        
        order
    }

    /// Recursive topological sort (DFS post-order)
    fn topological_sort_recursive(
        &self,
        block: &H256,
        order: &mut Vec<H256>,
        visited: &mut HashSet<H256>,
    ) {
        if visited.contains(block) {
            return;
        }
        visited.insert(*block);

        // First visit all parents (in deterministic order)
        if let Some(data) = self.store.get_ghostdag_data(block) {
            // Visit selected parent first
            if data.selected_parent != H256::zero() {
                self.topological_sort_recursive(&data.selected_parent, order, visited);
            }

            // Then visit mergeset_blues (sorted)
            let mut blues = data.mergeset_blues.clone();
            blues.sort();
            for blue in blues {
                self.topological_sort_recursive(&blue, order, visited);
            }

            // Then visit mergeset_reds (sorted)
            let mut reds = data.mergeset_reds.clone();
            reds.sort();
            for red in reds {
                self.topological_sort_recursive(&red, order, visited);
            }
        }

        // Add this block after all dependencies
        order.push(*block);
    }

    /// Get ordered blocks for execution from a starting point
    /// Returns blocks in order they should be executed
    pub fn get_execution_order(&self, from: &H256, to: &H256) -> Vec<H256> {
        let full_order = self.get_topological_order(to);
        
        // Find starting point
        let start_idx = full_order.iter().position(|h| h == from).unwrap_or(0);
        
        // Return blocks from start to end
        full_order[start_idx..].to_vec()
    }

    /// Get the canonical chain (selected parent path)
    pub fn get_canonical_chain(&self, tip: &H256) -> Vec<H256> {
        let mut chain = Vec::new();
        let mut current = *tip;

        loop {
            chain.push(current);
            
            if let Some(data) = self.store.get_ghostdag_data(&current) {
                if data.selected_parent == H256::zero() {
                    break; // Genesis
                }
                current = data.selected_parent;
            } else {
                break;
            }
        }

        chain.reverse();
        chain
    }

    /// Get blocks to execute when a new tip is added
    /// Returns (blocks_to_revert, blocks_to_apply)
    pub fn get_reorg_path(
        &self,
        old_tip: &H256,
        new_tip: &H256,
    ) -> (Vec<H256>, Vec<H256>) {
        let old_chain = self.get_canonical_chain(old_tip);
        let new_chain = self.get_canonical_chain(new_tip);

        // Find common ancestor
        let common_ancestor = self.find_common_ancestor(&old_chain, &new_chain);

        // Blocks to revert (old chain from tip to common ancestor)
        let revert: Vec<H256> = old_chain
            .iter()
            .rev()
            .take_while(|h| **h != common_ancestor)
            .cloned()
            .collect();

        // Blocks to apply (new chain from common ancestor to tip)
        let apply: Vec<H256> = new_chain
            .iter()
            .skip_while(|h| **h != common_ancestor)
            .skip(1) // Skip common ancestor itself
            .cloned()
            .collect();

        (revert, apply)
    }

    /// Find common ancestor of two chains
    fn find_common_ancestor(&self, chain1: &[H256], chain2: &[H256]) -> H256 {
        let set1: HashSet<_> = chain1.iter().collect();
        
        for block in chain2.iter().rev() {
            if set1.contains(block) {
                return *block;
            }
        }

        // Should not happen if both chains share genesis
        H256::zero()
    }

    /// Get merge set in execution order for a block
    pub fn get_ordered_mergeset(&self, block: &H256) -> Vec<H256> {
        let Some(data) = self.store.get_ghostdag_data(block) else {
            return vec![];
        };

        let mut ordered = Vec::new();

        // Blues first (sorted by blue_work desc, then hash asc)
        let mut blues: Vec<_> = data.mergeset_blues
            .iter()
            .filter_map(|h| {
                self.store.get_ghostdag_data(h).map(|d| (*h, d.blue_work))
            })
            .collect();
        blues.sort_by(|a, b| {
            match b.1.cmp(&a.1) {
                Ordering::Equal => a.0.cmp(&b.0),
                other => other,
            }
        });
        ordered.extend(blues.into_iter().map(|(h, _)| h));

        // Then reds (sorted same way)
        let mut reds: Vec<_> = data.mergeset_reds
            .iter()
            .filter_map(|h| {
                self.store.get_ghostdag_data(h).map(|d| (*h, d.blue_work))
            })
            .collect();
        reds.sort_by(|a, b| {
            match b.1.cmp(&a.1) {
                Ordering::Equal => a.0.cmp(&b.0),
                other => other,
            }
        });
        ordered.extend(reds.into_iter().map(|(h, _)| h));

        ordered
    }

    /// Validate ordering - check that all dependencies come before dependents
    pub fn validate_order(&self, order: &[H256]) -> bool {
        let position: std::collections::HashMap<_, _> = order
            .iter()
            .enumerate()
            .map(|(i, h)| (*h, i))
            .collect();

        for (i, block) in order.iter().enumerate() {
            if let Some(parents) = self.store.get_parents(block) {
                for parent in parents {
                    if let Some(&parent_pos) = position.get(&parent) {
                        if parent_pos >= i {
                            // Parent comes after child - invalid!
                            return false;
                        }
                    }
                }
            }
        }

        true
    }
}

impl<C> Clone for OrderingManager<C> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            ghostdag: self.ghostdag.clone(),
        }
    }
}
