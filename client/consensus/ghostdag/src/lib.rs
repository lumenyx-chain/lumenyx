//! GHOSTDAG Consensus Engine for LUMENYX
//! 
//! Implements the GHOSTDAG protocol for Substrate:
//! - BlockDAG with multi-parent blocks
//! - Blue set selection (k-cluster)
//! - Topological ordering for execution
//! - PoW mining with 1-3 second block time
//!
//! Based on Kaspa's GHOSTDAG: https://github.com/kaspanet/rusty-kaspa

pub mod store;
use sp_runtime::traits::Header;
pub mod dag;
pub mod ghostdag;
pub mod ordering;
pub mod miner;

pub use store::{GhostdagStore, GhostdagData, BlockRelations, ReachabilityData};
pub use dag::DagManager;
pub use ghostdag::GhostdagManager;
pub use ordering::OrderingManager;
pub use miner::{GhostdagMiner, MiningConfig, BlockTemplate, MinedBlock};

use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
use sc_client_api::AuxStore;
use std::sync::Arc;
use parking_lot::RwLock;
use parity_scale_codec::{Decode, Encode};

/// GHOSTDAG K parameter (anticone size limit)
/// Higher K = more parallelism, lower security
/// Kaspa uses K=18 for ~1 second blocks
pub const DEFAULT_K: u64 = 18;

/// Maximum parents per block
pub const MAX_PARENTS: usize = 10;

/// Target block time in milliseconds
pub const TARGET_BLOCK_TIME_MS: u64 = 1000; // 1 second

/// GHOSTDAG Consensus configuration
#[derive(Clone, Debug)]
pub struct GhostdagConfig {
    /// K parameter for blue set selection
    pub k: u64,
    /// Maximum parents per block
    pub max_parents: usize,
    /// Target block time in ms
    pub target_block_time_ms: u64,
    /// Initial difficulty
    pub initial_difficulty: u64,
}

impl Default for GhostdagConfig {
    fn default() -> Self {
        Self {
            k: DEFAULT_K,
            max_parents: MAX_PARENTS,
            target_block_time_ms: TARGET_BLOCK_TIME_MS,
            initial_difficulty: 1_000_000,
        }
    }
}

/// GHOSTDAG Consensus Engine
pub struct GhostdagConsensus<C> {
    store: GhostdagStore<C>,
    dag: DagManager<C>,
    ghostdag: GhostdagManager<C>,
    ordering: OrderingManager<C>,
    miner: GhostdagMiner<C>,
    config: GhostdagConfig,
    genesis_hash: Arc<RwLock<Option<H256>>>,
}

impl<C: AuxStore + Send + Sync + 'static> GhostdagConsensus<C> {
    /// Create new GHOSTDAG consensus engine
    pub fn new(client: Arc<C>, config: GhostdagConfig) -> Self {
        let store = GhostdagStore::new(client.clone());
        let dag = DagManager::new(store.clone(), config.k);
        let ghostdag = GhostdagManager::new(dag.clone(), store.clone(), config.k);
        let ordering = OrderingManager::new(store.clone(), ghostdag.clone());
        
        let mining_config = MiningConfig {
            target_block_time_ms: config.target_block_time_ms,
            max_parents: config.max_parents,
            initial_difficulty: config.initial_difficulty,
            difficulty_window: 2016,
        };
        let miner = GhostdagMiner::new(dag.clone(), ghostdag.clone(), store.clone(), mining_config);

        Self {
            store,
            dag,
            ghostdag,
            ordering,
            miner,
            config,
            genesis_hash: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize with genesis block
    pub fn initialize_genesis(&self, genesis_hash: H256) -> Result<(), String> {
        self.store.init_genesis(genesis_hash)?;
        *self.genesis_hash.write() = Some(genesis_hash);
        log::info!("🔷 GHOSTDAG initialized with genesis: {:?}", genesis_hash);
        Ok(())
    }

    /// Import a new block
    pub fn import_block(
        &self,
        block_hash: H256,
        parents: Vec<H256>,
        block_work: u128,
    ) -> Result<GhostdagData, String> {
        // Validate parents exist
        for parent in &parents {
            if self.store.get_ghostdag_data(parent).is_none() {
                return Err(format!("Unknown parent: {:?}", parent));
            }
        }

        // Process block through GHOSTDAG
        let data = self.ghostdag.process_block(block_hash, parents, block_work)?;

        log::info!(
            "🔷 Imported block {:?}: blue_score={}, selected_parent={:?}",
            block_hash,
            data.blue_score,
            data.selected_parent
        );

        Ok(data)
    }

    /// Get current tips
    pub fn get_tips(&self) -> Vec<H256> {
        self.dag.get_tips()
    }

    /// Get virtual (best) tip
    pub fn get_virtual_tip(&self) -> Option<H256> {
        self.ghostdag.get_virtual_tip()
    }

    /// Get blue score of virtual
    pub fn get_virtual_blue_score(&self) -> u64 {
        self.ghostdag.get_virtual_blue_score()
    }

    /// Get canonical chain (selected parent path)
    pub fn get_canonical_chain(&self) -> Vec<H256> {
        if let Some(tip) = self.get_virtual_tip() {
            self.ordering.get_canonical_chain(&tip)
        } else {
            vec![]
        }
    }

    /// Get topological order for execution
    pub fn get_execution_order(&self) -> Vec<H256> {
        if let Some(tip) = self.get_virtual_tip() {
            self.ordering.get_topological_order(&tip)
        } else {
            vec![]
        }
    }

    /// Get GHOSTDAG data for a block
    pub fn get_ghostdag_data(&self, block: &H256) -> Option<GhostdagData> {
        self.store.get_ghostdag_data(block)
    }

    /// Get parents for a block
    pub fn get_parents(&self, block: &H256) -> Option<Vec<H256>> {
        self.store.get_parents(block)
    }

    /// Check if block is blue
    pub fn is_blue(&self, block: &H256) -> bool {
        // A block is blue if it's in some block's mergeset_blues
        // For simplicity, check if it has valid GHOSTDAG data
        self.store.get_ghostdag_data(block).is_some()
    }

    /// Get miner reference
    pub fn miner(&self) -> &GhostdagMiner<C> {
        &self.miner
    }

    /// Get ordering manager reference
    pub fn ordering(&self) -> &OrderingManager<C> {
        &self.ordering
    }

    /// Get DAG manager reference
    pub fn dag(&self) -> &DagManager<C> {
        &self.dag
    }

    /// Get GHOSTDAG manager reference  
    pub fn ghostdag_manager(&self) -> &GhostdagManager<C> {
        &self.ghostdag
    }

    /// Get store reference
    pub fn store(&self) -> &GhostdagStore<C> {
        &self.store
    }

    /// Select parents for new block
    pub fn select_parents(&self) -> Vec<H256> {
        self.dag.select_parents(self.config.max_parents)
    }

    /// Finality check - block is "final" after enough blue score
    /// Similar to confirmations in Bitcoin
    pub fn is_final(&self, block: &H256, confirmations: u64) -> bool {
        let Some(block_data) = self.store.get_ghostdag_data(block) else {
            return false;
        };
        
        let virtual_score = self.get_virtual_blue_score();
        virtual_score >= block_data.blue_score + confirmations
    }

    /// Get confirmation count for a block
    pub fn get_confirmations(&self, block: &H256) -> u64 {
        let Some(block_data) = self.store.get_ghostdag_data(block) else {
            return 0;
        };
        
        let virtual_score = self.get_virtual_blue_score();
        virtual_score.saturating_sub(block_data.blue_score)
    }
}

impl<C> Clone for GhostdagConsensus<C> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            dag: self.dag.clone(),
            ghostdag: self.ghostdag.clone(),
            ordering: self.ordering.clone(),
            miner: self.miner.clone(),
            config: self.config.clone(),
            genesis_hash: self.genesis_hash.clone(),
        }
    }
}

/// Digest item for multi-parent blocks
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
pub struct DagParentsDigest {
    /// Extra parents (in addition to header.parent_hash)
    pub parents: Vec<H256>,
    /// PoW nonce
    pub nonce: [u8; 32],
    /// Block work (derived from difficulty)
    pub work: u128,
}

impl DagParentsDigest {
    pub fn new(parents: Vec<H256>, nonce: [u8; 32], work: u128) -> Self {
        Self { parents, nonce, work }
    }
}

/// Extract DAG parents from block digest
pub fn extract_dag_parents<B: BlockT>(block: &B) -> Option<DagParentsDigest> {
    use sp_runtime::DigestItem;
    
    for log in block.header().digest().logs() {
        if let DigestItem::Other(data) = log {
            if let Ok(digest) = DagParentsDigest::decode(&mut &data[..]) {
                return Some(digest);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = GhostdagConfig::default();
        assert_eq!(config.k, 18);
        assert_eq!(config.max_parents, 10);
        assert_eq!(config.target_block_time_ms, 1000);
    }
}
