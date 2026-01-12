//! LUMENYX PoW Block Import
//!
//! Wrapper that verifies RX-LX PoW before importing blocks.
//! This ensures all blocks (local and network) pass the same validation.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use codec::{Decode, Encode};
use sc_client_api::{AuxStore, HeaderBackend, StorageProvider};
use sc_consensus::{BlockImport, BlockImportParams, ImportResult};
use sp_core::{H256, U256, storage::StorageKey};
use sp_runtime::{
    generic::DigestItem,
    traits::{Block as BlockT, Header as HeaderT},
};

use lumenyx_runtime::opaque::Block;
use sc_service::TFullBackend;

use rx_lx::{Flags, Cache, Vm};
use crate::rx_lx::seed as seed_sched;

/// LUMENYX Engine ID
const LUMENYX_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"LMNX";

/// Aux key prefix for total difficulty
const AUX_TD_PREFIX: &[u8] = b"lumenyx:td:";

/// PoW constants
const MIN_DIFFICULTY: u128 = 1;
const MAX_DIFFICULTY: u128 = u128::MAX;
const POW_LIMIT: H256 = H256::repeat_byte(0xff);
const FALLBACK_DIFFICULTY: u128 = 1;

/// RX-LX PoW state - holds cache and VM
pub struct RxLxPowState {
    flags: Flags,
    cache: Cache,
    vm: Vm,
    seed_height: u64,
}

impl RxLxPowState {
    /// Initialize RX-LX with seed for given height
    pub fn new<F>(height: u64, get_block_hash: F) -> Result<Self, String>
    where
        F: Fn(u64) -> H256,
    {
        let flags = Flags::recommended();
        let mut cache = Cache::alloc(flags).map_err(|e| format!("Cache alloc failed: {:?}", e))?;
        
        let seed_height = seed_sched::seed_height(height);
        let seed = seed_sched::get_seed(height, &get_block_hash);
        
        log::debug!("🔧 PowImport: Initializing RX-LX cache with seed from block #{}", seed_height);
        cache.init(seed.as_ref());
        
        let vm = Vm::light(flags, &cache).map_err(|e| format!("VM creation failed: {:?}", e))?;
        
        Ok(Self { flags, cache, vm, seed_height })
    }
    
    /// Check if seed changed and reinitialize if needed
    pub fn maybe_reseed<F>(&mut self, height: u64, get_block_hash: F) -> Result<(), String>
    where
        F: Fn(u64) -> H256,
    {
        let new_seed_height = seed_sched::seed_height(height);
        if new_seed_height == self.seed_height {
            return Ok(());
        }
        
        let new_seed = seed_sched::get_seed(height, &get_block_hash);
        log::debug!("🔄 PowImport: RX-LX seed change: block #{} -> #{}", self.seed_height, new_seed_height);
        
        self.cache.init(new_seed.as_ref());
        self.vm = Vm::light(self.flags, &self.cache).map_err(|e| format!("VM recreation failed: {:?}", e))?;
        
        self.seed_height = new_seed_height;
        Ok(())
    }
    
    /// Compute RX-LX hash: input = header_hash || nonce (64 bytes)
    pub fn hash(&self, header_hash: &H256, nonce: &[u8; 32]) -> H256 {
        let mut input = [0u8; 64];
        input[..32].copy_from_slice(header_hash.as_ref());
        input[32..].copy_from_slice(nonce);
        H256::from_slice(&self.vm.hash(&input))
    }
}

/// Generate storage key for Difficulty::CurrentDifficulty
fn difficulty_storage_key() -> StorageKey {
    let mut key = sp_core::twox_128(b"Difficulty").to_vec();
    key.extend(sp_core::twox_128(b"CurrentDifficulty"));
    StorageKey(key)
}

/// Read current difficulty from runtime storage
fn read_difficulty_from_storage<C>(client: &C, at: H256) -> u128
where
    C: StorageProvider<Block, TFullBackend<Block>>,
{
    let key = difficulty_storage_key();
    
    match client.storage(at, &key) {
        Ok(Some(data)) => {
            match u128::decode(&mut &data.0[..]) {
                Ok(diff) => diff,
                Err(_) => FALLBACK_DIFFICULTY,
            }
        }
        _ => FALLBACK_DIFFICULTY,
    }
}

/// Convert difficulty to target
fn difficulty_to_target(difficulty: u128) -> H256 {
    let mut d = difficulty;
    if d < MIN_DIFFICULTY { d = MIN_DIFFICULTY; }
    if d > MAX_DIFFICULTY { d = MAX_DIFFICULTY; }
    if d == 0 { d = MIN_DIFFICULTY; }

    let pow_u = U256::from_big_endian(POW_LIMIT.as_fixed_bytes());

    let mut d_be = [0u8; 32];
    d_be[16..].copy_from_slice(&d.to_be_bytes());
    let d_u = U256::from_big_endian(&d_be);

    let mut target_u = pow_u / d_u;

    if target_u == U256::from(0u64) {
        target_u = U256::from(1u64);
    }
    if target_u > pow_u {
        target_u = pow_u;
    }

    let mut target_be = [0u8; 32];
    target_u.to_big_endian(&mut target_be);
    H256::from_slice(&target_be)
}

/// Check if hash meets target
fn hash_meets_target(hash: &H256, target: &H256) -> bool {
    hash <= target
}

/// Aux key for total difficulty
fn td_key(hash: &H256) -> Vec<u8> {
    let mut k = Vec::with_capacity(AUX_TD_PREFIX.len() + 32);
    k.extend_from_slice(AUX_TD_PREFIX);
    k.extend_from_slice(hash.as_ref());
    k
}

/// PoW Block Import wrapper
/// 
/// Verifies RX-LX PoW seal before importing blocks.
/// Also tracks total difficulty in aux storage.
pub struct RxLxPowBlockImport<C, I> {
    client: Arc<C>,
    inner: I,
    pow_state: Mutex<Option<RxLxPowState>>,
}

impl<C, I> RxLxPowBlockImport<C, I> {
    pub fn new(client: Arc<C>, inner: I) -> Self {
        Self {
            client,
            inner,
            pow_state: Mutex::new(None),
        }
    }
}

impl<C, I> RxLxPowBlockImport<C, I>
where
    C: HeaderBackend<Block> + StorageProvider<Block, TFullBackend<Block>> + AuxStore + Send + Sync,
{
    /// Extract seal from header digest
    fn extract_seal_from_header(header: &<Block as BlockT>::Header) -> Option<Vec<u8>> {
        header.digest().logs.iter().find_map(|d| {
            if let DigestItem::Seal(id, bytes) = d {
                if *id == LUMENYX_ENGINE_ID {
                    return Some(bytes.clone());
                }
            }
            None
        })
    }

    /// Extract seal from post_digests
    fn extract_seal_from_post_digests(post: &[DigestItem]) -> Option<Vec<u8>> {
        post.iter().find_map(|d| {
            if let DigestItem::Seal(id, bytes) = d {
                if *id == LUMENYX_ENGINE_ID {
                    return Some(bytes.clone());
                }
            }
            None
        })
    }

    /// Remove LUMENYX seal from header
    fn remove_seal_from_header(header: &mut <Block as BlockT>::Header) {
        header.digest_mut().logs.retain(|d| {
            !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID)
        });
    }

    /// Normalize post_digests: remove duplicates and add seal
    fn normalize_post_digests(post: &mut Vec<DigestItem>, seal_bytes: Vec<u8>) {
        post.retain(|d| !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID));
        post.push(DigestItem::Seal(LUMENYX_ENGINE_ID, seal_bytes));
    }

    /// Get block hash by number
    fn block_hash_by_number(&self, n: u64) -> H256 {
        self.client
            .hash(n as u32)
            .ok()
            .flatten()
            .unwrap_or_else(H256::zero)
    }

    /// Read total difficulty from aux storage
    fn read_total_difficulty(&self, hash: H256) -> U256 {
        let key = td_key(&hash);
        match self.client.get_aux(&key) {
            Ok(Some(raw)) => U256::decode(&mut &raw[..]).unwrap_or(U256::zero()),
            _ => U256::zero(),
        }
    }

    /// Write total difficulty to aux storage
    fn write_total_difficulty(&self, hash: H256, td: U256) {
        let key = td_key(&hash);
        let val = td.encode();
        let _ = self.client.insert_aux(&[(key.as_slice(), val.as_slice())], &[]);
    }

    /// Verify PoW on header without seal
    fn verify_pow(
        &self,
        header_without_seal: &<Block as BlockT>::Header,
        seal_bytes: &[u8],
    ) -> Result<(), String> {
        // 1) Check nonce length
        if seal_bytes.len() != 32 {
            return Err(format!("Invalid nonce length: expected 32, got {}", seal_bytes.len()));
        }
        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&seal_bytes[..32]);

        // 2) Get height and parent hash
        let height: u64 = (*header_without_seal.number()) as u64;
        let parent_hash: H256 = *header_without_seal.parent_hash();

        // 3) Read difficulty from parent state
        let diff_u128 = read_difficulty_from_storage(&*self.client, parent_hash);
        let target = difficulty_to_target(diff_u128);

        // 4) Compute header hash (without seal)
        let header_hash: H256 = header_without_seal.hash();

        // 5) Initialize or reseed RX-LX
        let mut guard = self.pow_state.lock().map_err(|_| "pow_state poisoned".to_string())?;
        if guard.is_none() {
            *guard = Some(RxLxPowState::new(height, |h| self.block_hash_by_number(h))?);
        }
        let pow = guard.as_mut().unwrap();
        pow.maybe_reseed(height, |h| self.block_hash_by_number(h))?;

        // 6) Compute PoW hash
        let pow_hash = pow.hash(&header_hash, &nonce);

        // 7) Check if hash meets target
        if !hash_meets_target(&pow_hash, &target) {
            return Err(format!(
                "Invalid PoW: hash {:?} does not meet target {:?} (difficulty={})",
                pow_hash, target, diff_u128
            ));
        }

        log::debug!("✅ PoW verified: block #{} hash={:?} difficulty={}", height, pow_hash, diff_u128);
        Ok(())
    }
}

#[async_trait]
impl<C, I> BlockImport<Block> for RxLxPowBlockImport<C, I>
where
    C: HeaderBackend<Block>
        + StorageProvider<Block, TFullBackend<Block>>
        + AuxStore
        + Send
        + Sync
        + 'static,
    I: BlockImport<Block, Error = sp_consensus::Error> + Send + Sync,
{
    type Error = sp_consensus::Error;

    async fn import_block(
        &self,
        mut params: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        // A) Extract seal from header OR post_digests
        let seal = Self::extract_seal_from_header(&params.header)
            .or_else(|| Self::extract_seal_from_post_digests(&params.post_digests));

        let seal_bytes = match seal {
            Some(s) => s,
            None => {
                log::warn!("❌ Missing LUMENYX seal in block");
                return Err(sp_consensus::Error::Other(
                    "Missing LUMENYX seal".into()
                ));
            }
        };

        // B) Create header copy without seal for verification
        let mut header_wo_seal = params.header.clone();
        Self::remove_seal_from_header(&mut header_wo_seal);

        // C) Verify PoW
        if let Err(e) = self.verify_pow(&header_wo_seal, &seal_bytes) {
            log::warn!("❌ PoW verification failed: {}", e);
            return Err(sp_consensus::Error::Other(e.into()));
        }

        // D) Normalize: header without seal + seal in post_digests
        params.header = header_wo_seal;
        Self::normalize_post_digests(&mut params.post_digests, seal_bytes.clone());

        // E) Calculate total difficulty
        let parent_hash = *params.header.parent_hash();
        let parent_td = self.read_total_difficulty(parent_hash);
        let diff_u128 = read_difficulty_from_storage(&*self.client, parent_hash);
        let diff_u256 = U256::from(diff_u128);
        let this_td = parent_td.saturating_add(diff_u256);

        // F) Import block
        let block_hash = params.header.hash();
        let res = self.inner.import_block(params).await?;

        // G) Write total difficulty to aux storage
        self.write_total_difficulty(block_hash, this_td);

        Ok(res)
    }

    async fn check_block(
        &self,
        block: sc_consensus::BlockCheckParams<Block>,
    ) -> Result<sc_consensus::ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }
}
