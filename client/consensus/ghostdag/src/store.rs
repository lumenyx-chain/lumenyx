//! GHOSTDAG Storage - DAG data persistence in AuxStore

use parity_scale_codec::{Decode, Encode};
use sp_core::H256;
use sc_client_api::AuxStore;
use std::sync::Arc;

pub const GHOSTDAG_PREFIX: &[u8] = b"ghostdag:";

/// Key builder for AuxStore
fn key(tag: &str, h: &H256) -> Vec<u8> {
    let mut k = Vec::with_capacity(GHOSTDAG_PREFIX.len() + tag.len() + 32);
    k.extend_from_slice(GHOSTDAG_PREFIX);
    k.extend_from_slice(tag.as_bytes());
    k.extend_from_slice(h.as_ref());
    k
}

/// GHOSTDAG data for each block - matches Kaspa's GhostdagData
#[derive(Clone, Encode, Decode, Debug, Default, PartialEq, Eq)]
pub struct GhostdagData {
    /// Blue score (number of blue blocks in past)
    pub blue_score: u64,
    /// Blue work (cumulative PoW of blue blocks)
    pub blue_work: u128,
    /// Selected parent (heaviest blue parent)
    pub selected_parent: H256,
    /// Blue blocks in mergeset
    pub mergeset_blues: Vec<H256>,
    /// Red blocks in mergeset
    pub mergeset_reds: Vec<H256>,
    /// Blues anticone sizes (for k-cluster validation)
    pub blues_anticone_sizes: Vec<(H256, u64)>,
}

/// Block relations in DAG
#[derive(Clone, Encode, Decode, Debug, Default)]
pub struct BlockRelations {
    /// All parents of this block
    pub parents: Vec<H256>,
    /// All children of this block (blocks that reference this as parent)
    pub children: Vec<H256>,
}

/// Reachability data for efficient ancestor queries
#[derive(Clone, Encode, Decode, Debug, Default)]
pub struct ReachabilityData {
    /// Interval for tree-based reachability
    pub interval_start: u64,
    pub interval_end: u64,
    /// Parent in reachability tree
    pub tree_parent: Option<H256>,
}

/// GHOSTDAG Store - persists DAG data to AuxStore
pub struct GhostdagStore<C> {
    client: Arc<C>,
}

impl<C> GhostdagStore<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C: AuxStore> GhostdagStore<C> {
    // ============ Parents/Relations ============
    
    pub fn get_parents(&self, block: &H256) -> Option<Vec<H256>> {
        let k = key("parents:", block);
        self.client.get_aux(&k).ok()?.map(|bytes| {
            Vec::<H256>::decode(&mut &bytes[..]).ok()
        }).flatten()
    }

    pub fn store_parents(&self, block: &H256, parents: &[H256]) -> Result<(), String> {
        let k = key("parents:", block);
        let v = parents.encode();
        self.client.insert_aux(&[(&k[..], &v[..])], &[])
            .map_err(|e| format!("Failed to store parents: {:?}", e))
    }

    pub fn get_children(&self, block: &H256) -> Option<Vec<H256>> {
        let k = key("children:", block);
        self.client.get_aux(&k).ok()?.map(|bytes| {
            Vec::<H256>::decode(&mut &bytes[..]).ok()
        }).flatten()
    }

    pub fn add_child(&self, parent: &H256, child: &H256) -> Result<(), String> {
        let mut children = self.get_children(parent).unwrap_or_default();
        if !children.contains(child) {
            children.push(*child);
            let k = key("children:", parent);
            let v = children.encode();
            self.client.insert_aux(&[(&k[..], &v[..])], &[])
                .map_err(|e| format!("Failed to add child: {:?}", e))?;
        }
        Ok(())
    }

    // ============ GHOSTDAG Data ============
    
    pub fn get_ghostdag_data(&self, block: &H256) -> Option<GhostdagData> {
        let k = key("ghostdag:", block);
        self.client.get_aux(&k).ok()?.map(|bytes| {
            GhostdagData::decode(&mut &bytes[..]).ok()
        }).flatten()
    }

    pub fn store_ghostdag_data(&self, block: &H256, data: &GhostdagData) -> Result<(), String> {
        let k = key("ghostdag:", block);
        let v = data.encode();
        self.client.insert_aux(&[(&k[..], &v[..])], &[])
            .map_err(|e| format!("Failed to store ghostdag data: {:?}", e))
    }

    // ============ Blue Score (quick access) ============
    
    pub fn get_blue_score(&self, block: &H256) -> Option<u64> {
        self.get_ghostdag_data(block).map(|d| d.blue_score)
    }

    pub fn get_blue_work(&self, block: &H256) -> Option<u128> {
        self.get_ghostdag_data(block).map(|d| d.blue_work)
    }

    // ============ Reachability ============
    
    pub fn get_reachability(&self, block: &H256) -> Option<ReachabilityData> {
        let k = key("reach:", block);
        self.client.get_aux(&k).ok()?.map(|bytes| {
            ReachabilityData::decode(&mut &bytes[..]).ok()
        }).flatten()
    }

    pub fn store_reachability(&self, block: &H256, data: &ReachabilityData) -> Result<(), String> {
        let k = key("reach:", block);
        let v = data.encode();
        self.client.insert_aux(&[(&k[..], &v[..])], &[])
            .map_err(|e| format!("Failed to store reachability: {:?}", e))
    }

    // ============ Tips ============
    
    pub fn get_tips(&self) -> Vec<H256> {
        let k = key("tips:", &H256::zero());
        self.client.get_aux(&k).ok()
            .flatten()
            .and_then(|bytes| Vec::<H256>::decode(&mut &bytes[..]).ok())
            .unwrap_or_default()
    }

    pub fn store_tips(&self, tips: &[H256]) -> Result<(), String> {
        let k = key("tips:", &H256::zero());
        let v = tips.encode();
        self.client.insert_aux(&[(&k[..], &v[..])], &[])
            .map_err(|e| format!("Failed to store tips: {:?}", e))
    }

    pub fn update_tips(&self, new_block: &H256, parents: &[H256]) -> Result<(), String> {
        let mut tips = self.get_tips();
        // Remove parents from tips (they're no longer tips)
        tips.retain(|t| !parents.contains(t));
        // Add new block as tip
        if !tips.contains(new_block) {
            tips.push(*new_block);
        }
        self.store_tips(&tips)
    }

    // ============ Genesis ============
    
    pub fn init_genesis(&self, genesis_hash: H256) -> Result<(), String> {
        // Genesis has no parents
        self.store_parents(&genesis_hash, &[])?;
        
        // Genesis GHOSTDAG data
        let genesis_data = GhostdagData {
            blue_score: 0,
            blue_work: 0,
            selected_parent: H256::zero(),
            mergeset_blues: vec![],
            mergeset_reds: vec![],
            blues_anticone_sizes: vec![],
        };
        self.store_ghostdag_data(&genesis_hash, &genesis_data)?;
        
        // Genesis is the only tip initially
        self.store_tips(&[genesis_hash])?;
        
        log::info!("🔷 GHOSTDAG genesis initialized: {:?}", genesis_hash);
        Ok(())
    }
}

impl<C> Clone for GhostdagStore<C> {
    fn clone(&self) -> Self {
        Self { client: self.client.clone() }
    }
}
