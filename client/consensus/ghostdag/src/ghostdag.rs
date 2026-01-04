//! GHOSTDAG Algorithm - Blue set selection and mergeset computation
//! Based on Kaspa's GHOSTDAG protocol

use sp_core::H256;
use std::collections::{HashSet, HashMap, BinaryHeap};
use std::cmp::Ordering;

use crate::store::{GhostdagStore, GhostdagData};
use crate::dag::DagManager;

/// Block with blue work for priority queue
#[derive(Eq, PartialEq)]
struct BlockWithWork {
    hash: H256,
    blue_work: u128,
}

impl Ord for BlockWithWork {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher blue_work = higher priority
        // Tie-break by hash (lower hash = higher priority for determinism)
        match self.blue_work.cmp(&other.blue_work) {
            Ordering::Equal => other.hash.cmp(&self.hash), // Lower hash wins
            other => other,
        }
    }
}

impl PartialOrd for BlockWithWork {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// GHOSTDAG Manager - computes blue sets and ordering
pub struct GhostdagManager<C> {
    dag: DagManager<C>,
    store: GhostdagStore<C>,
    /// K parameter (anticone size limit for blue blocks)
    k: u64,
}

impl<C> GhostdagManager<C> {
    pub fn new(dag: DagManager<C>, store: GhostdagStore<C>, k: u64) -> Self {
        Self { dag, store, k }
    }
}

impl<C: sc_client_api::AuxStore> GhostdagManager<C> {
    /// Compute GHOSTDAG data for a new block
    pub fn compute_ghostdag_data(
        &self,
        block_hash: H256,
        parents: &[H256],
        block_work: u128, // PoW work of this block
    ) -> Result<GhostdagData, String> {
        if parents.is_empty() {
            // Genesis block
            return Ok(GhostdagData {
                blue_score: 0,
                blue_work: block_work,
                selected_parent: H256::zero(),
                mergeset_blues: vec![],
                mergeset_reds: vec![],
                blues_anticone_sizes: vec![],
            });
        }

        // 1. Find selected parent (parent with highest blue_work)
        let selected_parent = self.find_selected_parent(parents)?;

        // 2. Get selected parent's GHOSTDAG data
        let parent_data = self.store.get_ghostdag_data(&selected_parent)
            .ok_or_else(|| format!("Missing GHOSTDAG data for parent {:?}", selected_parent))?;

        // 3. Compute mergeset (blocks in past(block) but not in past(selected_parent))
        let mergeset = self.compute_mergeset(&block_hash, parents, &selected_parent);

        // 4. Partition mergeset into blues and reds using k-cluster rule
        let (mergeset_blues, mergeset_reds, blues_anticone_sizes) = 
            self.partition_mergeset(&mergeset, &selected_parent, &parent_data);

        // 5. Compute blue score and blue work
        let blue_score = parent_data.blue_score + 1 + mergeset_blues.len() as u64;
        let blue_work = parent_data.blue_work + block_work + 
            mergeset_blues.iter()
                .filter_map(|b| self.get_block_work(b))
                .sum::<u128>();

        Ok(GhostdagData {
            blue_score,
            blue_work,
            selected_parent,
            mergeset_blues,
            mergeset_reds,
            blues_anticone_sizes,
        })
    }

    /// Find selected parent (highest blue_work, tie-break by hash)
    fn find_selected_parent(&self, parents: &[H256]) -> Result<H256, String> {
        parents
            .iter()
            .filter_map(|p| {
                self.store.get_ghostdag_data(p).map(|d| (*p, d.blue_work))
            })
            .max_by(|a, b| {
                match a.1.cmp(&b.1) {
                    Ordering::Equal => b.0.cmp(&a.0), // Lower hash wins on tie
                    other => other,
                }
            })
            .map(|(h, _)| h)
            .ok_or_else(|| "No valid parent found".to_string())
    }

    /// Compute mergeset - blocks to be ordered when adding this block
    fn compute_mergeset(
        &self,
        _block_hash: &H256,
        parents: &[H256],
        selected_parent: &H256,
    ) -> HashSet<H256> {
        let mut mergeset = HashSet::new();

        // Add all parents except selected_parent
        for parent in parents {
            if parent != selected_parent {
                mergeset.insert(*parent);
                // Also add their past that's not in selected_parent's past
                self.add_past_to_mergeset(parent, selected_parent, &mut mergeset);
            }
        }

        mergeset
    }

    /// Add past blocks to mergeset (blocks not in selected_parent's past)
    fn add_past_to_mergeset(
        &self,
        block: &H256,
        selected_parent: &H256,
        mergeset: &mut HashSet<H256>,
    ) {
        let past = self.dag.get_past(block);
        let selected_past = self.dag.get_past(selected_parent);

        for b in past {
            if !selected_past.contains(&b) && b != *selected_parent {
                mergeset.insert(b);
            }
        }
    }

    /// Partition mergeset into blues and reds using k-cluster rule
    fn partition_mergeset(
        &self,
        mergeset: &HashSet<H256>,
        selected_parent: &H256,
        parent_data: &GhostdagData,
    ) -> (Vec<H256>, Vec<H256>, Vec<(H256, u64)>) {
        let mut blues = Vec::new();
        let mut reds = Vec::new();
        let mut blues_anticone_sizes = Vec::new();

        // Start with selected parent's blue set
        let mut current_blues: HashSet<H256> = HashSet::new();
        current_blues.insert(*selected_parent);
        
        // Add selected parent's mergeset_blues
        for b in &parent_data.mergeset_blues {
            current_blues.insert(*b);
        }

        // Process mergeset blocks in order (by blue_work, then hash)
        let mut heap: BinaryHeap<BlockWithWork> = mergeset
            .iter()
            .filter_map(|h| {
                self.store.get_ghostdag_data(h).map(|d| BlockWithWork {
                    hash: *h,
                    blue_work: d.blue_work,
                })
            })
            .collect();

        while let Some(block) = heap.pop() {
            // Check k-cluster property: anticone size must be <= k
            let anticone_size = self.compute_anticone_size(&block.hash, &current_blues);

            if anticone_size <= self.k {
                // Block is blue
                blues.push(block.hash);
                blues_anticone_sizes.push((block.hash, anticone_size));
                current_blues.insert(block.hash);
            } else {
                // Block is red
                reds.push(block.hash);
            }
        }

        (blues, reds, blues_anticone_sizes)
    }

    /// Compute anticone size of a block relative to current blue set
    fn compute_anticone_size(&self, block: &H256, blues: &HashSet<H256>) -> u64 {
        let mut count = 0u64;

        for blue in blues {
            // Check if blue is in anticone of block
            // (not in past of block and block not in past of blue)
            if !self.dag.is_in_past(blue, block) && !self.dag.is_in_past(block, blue) {
                count += 1;
            }
        }

        count
    }

    /// Get block's PoW work (simplified: use blue_work delta or constant)
    fn get_block_work(&self, block: &H256) -> Option<u128> {
        // For MVP, assume constant work per block
        // In production, derive from difficulty/nonce
        self.store.get_ghostdag_data(block).map(|_| 1u128)
    }

    /// Process a new block - compute and store GHOSTDAG data
    pub fn process_block(
        &self,
        block_hash: H256,
        parents: Vec<H256>,
        block_work: u128,
    ) -> Result<GhostdagData, String> {
        // Add block to DAG
        self.dag.add_block(block_hash, parents.clone())?;

        // Compute GHOSTDAG data
        let data = self.compute_ghostdag_data(block_hash, &parents, block_work)?;

        // Store GHOSTDAG data
        self.store.store_ghostdag_data(&block_hash, &data)?;

        log::debug!(
            "🔷 GHOSTDAG processed block {:?}: blue_score={}, blues={}, reds={}",
            block_hash,
            data.blue_score,
            data.mergeset_blues.len(),
            data.mergeset_reds.len()
        );

        Ok(data)
    }

    /// Get the virtual (best) tip
    pub fn get_virtual_tip(&self) -> Option<H256> {
        let tips = self.dag.get_tips();
        tips.iter()
            .filter_map(|t| {
                self.store.get_ghostdag_data(t).map(|d| (*t, d.blue_work))
            })
            .max_by(|a, b| {
                match a.1.cmp(&b.1) {
                    Ordering::Equal => b.0.cmp(&a.0),
                    other => other,
                }
            })
            .map(|(h, _)| h)
    }

    /// Get blue score of virtual (for difficulty adjustment)
    pub fn get_virtual_blue_score(&self) -> u64 {
        self.get_virtual_tip()
            .and_then(|t| self.store.get_ghostdag_data(&t))
            .map(|d| d.blue_score)
            .unwrap_or(0)
    }
}

impl<C> Clone for GhostdagManager<C> {
    fn clone(&self) -> Self {
        Self {
            dag: self.dag.clone(),
            store: self.store.clone(),
            k: self.k,
        }
    }
}
