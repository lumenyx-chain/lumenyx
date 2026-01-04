//! GHOSTDAG SelectChain - Fork choice based on blue_work instead of longest chain

use std::sync::Arc;
use sp_blockchain::{Backend as BlockchainBackend, HeaderBackend};
use sp_consensus::{SelectChain, Error as ConsensusError};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use sp_core::H256;
use sc_client_api::{AuxStore, Backend as BackendT};
use sc_consensus_ghostdag::GhostdagStore;

/// GHOSTDAG-based chain selection
pub struct GhostdagSelectChain<Block: BlockT, Backend, Client> {
    backend: Arc<Backend>,
    client: Arc<Client>,
    ghostdag_store: GhostdagStore<Client>,
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block: BlockT, Backend, Client> Clone for GhostdagSelectChain<Block, Backend, Client> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            client: self.client.clone(),
            ghostdag_store: self.ghostdag_store.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Block, Backend, Client> GhostdagSelectChain<Block, Backend, Client>
where
    Block: BlockT,
    Backend: BackendT<Block>,
    Client: HeaderBackend<Block> + AuxStore,
{
    /// Create a new GHOSTDAG select chain
    pub fn new(backend: Arc<Backend>, client: Arc<Client>) -> Self {
        let ghostdag_store = GhostdagStore::new(client.clone());
        Self {
            backend,
            client,
            ghostdag_store,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the block with highest blue_work from all leaves
    fn best_block_by_blue_work(&self) -> Option<Block::Hash> {
        let leaves = self.backend.blockchain().leaves().ok()?;
        
        if leaves.is_empty() {
            return None;
        }

        // Find leaf with highest blue_work
        let mut best_hash: Option<Block::Hash> = None;
        let mut best_work: u128 = 0;

        for leaf in leaves {
            let leaf_h256 = H256::from_slice(leaf.as_ref());
            
            if let Some(data) = self.ghostdag_store.get_ghostdag_data(&leaf_h256) {
                if data.blue_work > best_work || 
                   (data.blue_work == best_work && best_hash.is_none()) {
                    best_work = data.blue_work;
                    best_hash = Some(leaf);
                }
            } else {
                // Block not in GHOSTDAG store - use block number as fallback
                if let Ok(Some(header)) = self.client.header(leaf) {
                    let number: u128 = (*header.number()).try_into().unwrap_or(0);
                    
                    if number > best_work || best_hash.is_none() {
                        best_work = number;
                        best_hash = Some(leaf);
                    }
                }
            }
        }

        best_hash
    }
}

#[async_trait::async_trait]
impl<Block, Backend, Client> SelectChain<Block> for GhostdagSelectChain<Block, Backend, Client>
where
    Block: BlockT,
    Backend: BackendT<Block> + Send + Sync,
    Client: HeaderBackend<Block> + AuxStore + Send + Sync,
{
    async fn leaves(&self) -> Result<Vec<Block::Hash>, ConsensusError> {
        self.backend
            .blockchain()
            .leaves()
            .map_err(|e| ConsensusError::ChainLookup(e.to_string()))
    }

    async fn best_chain(&self) -> Result<Block::Header, ConsensusError> {
        // Get best block by blue_work
        let best_hash = self.best_block_by_blue_work()
            .unwrap_or_else(|| self.client.info().best_hash);

        self.client
            .header(best_hash)
            .map_err(|e| ConsensusError::ChainLookup(e.to_string()))?
            .ok_or_else(|| ConsensusError::ChainLookup("Best header not found".into()))
    }

    async fn finality_target(
        &self,
        target_hash: Block::Hash,
        _maybe_max_number: Option<NumberFor<Block>>,
    ) -> Result<Block::Hash, ConsensusError> {
        Ok(target_hash)
    }
}
