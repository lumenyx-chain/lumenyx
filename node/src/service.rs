//! LUMENYX Service Configuration - PoW with Dynamic Difficulty
//!
//! PoW consensus with on-chain difficulty adjustment.

use sc_network::NetworkBlock;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

use codec::{Decode, Encode};
use sp_core::{crypto::Ss58Codec, sr25519, storage::StorageKey, Pair, H256, U256};
use sp_runtime::generic::DigestItem;

use futures::StreamExt;
use tokio::sync::{mpsc, watch};

use lumenyx_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{AuxStore, BlockBackend, BlockchainEvents, HeaderBackend, StorageProvider};
use sc_consensus::{import_queue::Verifier, BlockImport, BlockImportParams};
use sc_executor::WasmExecutor;
use sc_service::{
    error::Error as ServiceError, Configuration, TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_consensus::{BlockOrigin, SelectChain, SyncOracle};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

// RX-LX imports
use rx_lx::{Cache, Flags, Vm};

use crate::rx_lx::seed as seed_sched;

// PoW BlockImport wrapper with TD tracking
#[path = "pow_import.rs"]
mod pow_import;

use crate::pool::gossip::{spawn_pool_gossip_task, PoolGossip, POOL_PROTO_NAME};
use crate::pool::pplns::compute_pplns_payouts;
use crate::pool::sharechain::Sharechain;
use crate::pool::types::PoolShare;
use crate::pool::{MAX_POOL_PAYOUTS, PPLNS_WINDOW_SHARES, SHARE_DIFFICULTY_DIVISOR};
use pow_import::{LumenyxPowBlockImport, TotalDifficultySelectChain};

// Frontier imports
use fc_mapping_sync::{
    kv::MappingSyncWorker, EthereumBlockNotification, EthereumBlockNotificationSinks, SyncStrategy,
};
use fc_rpc::EthTask;
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fc_storage::StorageOverrideHandler;

pub fn db_config_dir(config: &Configuration) -> std::path::PathBuf {
    config.base_path.config_dir(config.chain_spec.id())
}

pub type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>;
type FullBackend = TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type FrontierBackend = fc_db::Backend<Block, FullClient>;

pub struct FrontierPartialComponents {
    pub filter_pool: Option<FilterPool>,
    pub fee_history_cache: FeeHistoryCache,
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
}

const TARGET_BLOCK_TIME_MS: u64 = 2500;
const SHARE_STALE_DEPTH: u64 = 4; // accept shares up to N ancestors behind best tip

fn is_recent_parent<C>(client: &C, share_parent: H256, best_hash: H256, max_depth: u64) -> bool
where
    C: HeaderBackend<Block>,
{
    if share_parent == best_hash {
        return true;
    }
    let mut cur = best_hash;
    for _ in 0..max_depth {
        let Ok(Some(hdr)) = client.header(cur) else {
            break;
        };
        let p = *hdr.parent_hash();
        if p == share_parent {
            return true;
        }
        if p == H256::zero() {
            break;
        }
        cur = p;
    }
    false
}

// Fallback difficulty if we can't read from runtime
const FALLBACK_DIFFICULTY: u128 = 1; // Backup - genesis should set this

/// LUMENYX Engine ID for digests
const LUMENYX_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"LMNX";

/// Miner address digest structure
#[derive(Clone, codec::Encode, codec::Decode)]
struct MinerAddressDigest {
    miner: [u8; 32],
}

/// Pool payout digest structure
#[derive(Clone, codec::Encode, codec::Decode)]
struct PoolPayoutDigest {
    sharechain_tip: H256,
    block_reward: u128,
    payouts: Vec<([u8; 32], u128)>,
}

const POOL_PAYOUT_DIGEST_TAG: &[u8; 4] = b"PPLN";
/// Generate storage key for Difficulty::CurrentDifficulty
fn difficulty_storage_key() -> StorageKey {
    // twox_128("Difficulty") ++ twox_128("CurrentDifficulty")
    let mut key = sp_core::twox_128(b"Difficulty").to_vec();
    key.extend(sp_core::twox_128(b"CurrentDifficulty"));
    StorageKey(key)
}

/// Read current difficulty from runtime storage
fn read_difficulty_from_storage<C>(client: &C, at: H256) -> u128
where
    C: StorageProvider<Block, FullBackend>,
{
    let key = difficulty_storage_key();

    match client.storage(at, &key) {
        Ok(Some(data)) => match u128::decode(&mut &data.0[..]) {
            Ok(diff) => {
                log::debug!("📊 Read difficulty from storage: {}", diff);
                diff
            }
            Err(e) => {
                log::warn!("Failed to decode difficulty: {:?}, using fallback", e);
                FALLBACK_DIFFICULTY
            }
        },
        Ok(None) => {
            log::error!(
                "❌ CRITICAL: Difficulty storage key NOT FOUND at block {:?}! This usually means storage prefix mismatch or corrupted state.",
                at
            );
            log::warn!(
                "⚠️ No difficulty in storage yet (block {:?}), using fallback: {}",
                at,
                FALLBACK_DIFFICULTY
            );
            FALLBACK_DIFFICULTY
        }
        Err(e) => {
            log::warn!("Failed to read difficulty storage: {:?}, using fallback", e);
            FALLBACK_DIFFICULTY
        }
    }
}

/// Load or generate miner keypair
fn get_or_create_miner_key(base_path: &std::path::Path) -> sr25519::Pair {
    let key_file = base_path.join("miner-key");

    if key_file.exists() {
        if let Ok(seed_hex) = fs::read_to_string(&key_file) {
            if let Ok(seed_bytes) = hex::decode(seed_hex.trim()) {
                if seed_bytes.len() == 32 {
                    let mut seed = [0u8; 32];
                    seed.copy_from_slice(&seed_bytes);
                    return sr25519::Pair::from_seed(&seed);
                }
            }
        }
    }

    let (pair, phrase, seed) = sr25519::Pair::generate_with_phrase(None);
    let seed_hex = hex::encode(seed);

    if let Err(e) = fs::write(&key_file, &seed_hex) {
        log::warn!("Failed to save miner key: {:?}", e);
    }

    log::info!("==========================================");
    log::info!("🔑 NEW MINER WALLET GENERATED!");
    log::info!("==========================================");
    log::info!("📝 Seed phrase: {}", phrase);
    log::info!("📫 Address: {}", pair.public().to_ss58check());
    log::info!("==========================================");
    log::info!("⚠️  SAVE YOUR SEED PHRASE!");
    log::info!("==========================================");

    pair
}

/// RX-LX PoW state - holds cache and VM, reinitializes on seed change
struct RxLxPow {
    flags: Flags,
    cache: Cache,
    vm: Vm,
    seed_height: u64,
    seed: H256,
}

impl RxLxPow {
    /// Initialize RX-LX with seed for given height
    fn new<F>(height: u64, get_block_hash: F) -> Result<Self, String>
    where
        F: Fn(u64) -> H256,
    {
        let flags = Flags::recommended(); // soft AES forced for custom SBOX
        let mut cache = Cache::alloc(flags).map_err(|e| format!("Cache alloc failed: {:?}", e))?;

        let seed_height = seed_sched::seed_height(height);
        let seed = seed_sched::get_seed(height, &get_block_hash);

        log::info!(
            "🔧 Initializing RX-LX cache with seed from block #{}",
            seed_height
        );
        cache.init(seed.as_ref());

        let vm = Vm::light(flags, &cache).map_err(|e| format!("VM creation failed: {:?}", e))?;
        log::info!("✅ RX-LX initialized successfully");

        Ok(Self {
            flags,
            cache,
            vm,
            seed_height,
            seed,
        })
    }

    /// Check if seed changed and reinitialize if needed
    fn maybe_reseed<F>(&mut self, height: u64, get_block_hash: F) -> Result<(), String>
    where
        F: Fn(u64) -> H256,
    {
        let new_seed_height = seed_sched::seed_height(height);
        if new_seed_height == self.seed_height {
            return Ok(());
        }

        let new_seed = seed_sched::get_seed(height, &get_block_hash);
        log::info!(
            "🔄 RX-LX seed change: block #{} -> #{}",
            self.seed_height,
            new_seed_height
        );

        self.cache.init(new_seed.as_ref());
        self.vm = Vm::light(self.flags, &self.cache)
            .map_err(|e| format!("VM recreation failed: {:?}", e))?;

        self.seed_height = new_seed_height;
        self.seed = new_seed;
        log::info!("✅ RX-LX reseeded successfully");
        Ok(())
    }

    /// Compute RX-LX hash: input = header_hash || nonce (64 bytes)
    fn hash(&self, header_hash: &H256, nonce: &[u8; 32]) -> H256 {
        let mut input = [0u8; 64];
        input[..32].copy_from_slice(header_hash.as_ref());
        input[32..].copy_from_slice(nonce);
        H256::from_slice(&self.vm.hash(&input))
    }
}

// ----------------------------
// Mining job messages (workers)
// ----------------------------
#[derive(Clone, Copy, Debug)]
struct MiningJob {
    job_id: u64,
    height: u64,
    parent_hash: H256,
    pre_hash: H256,
    target: H256,
    share_target: H256,
    seed_height: u64,
    seed: H256,
}

#[derive(Clone, Debug)]
struct FoundNonce {
    job_id: u64,
    nonce: [u8; 32],
}

#[derive(Clone, Debug)]
struct FoundShare {
    job_id: u64,
    height: u64,
    parent_hash: H256,
    nonce: [u8; 32],
}

// ----------------------------
// Persistent miner state (+ hashrate counters)
// ----------------------------
struct MinerState {
    job_tx: watch::Sender<MiningJob>,
    found_rx: mpsc::UnboundedReceiver<FoundNonce>,
    found_share_rx: mpsc::UnboundedReceiver<FoundShare>,
    job_id: u64,
    last_parent_hash: Option<H256>,

    // Hashrate counters
    total_hashes: Arc<AtomicU64>,
    per_thread_hashes: Arc<Vec<AtomicU64>>,
}

impl MinerState {
    fn new(num_threads: usize) -> Self {
        let dummy_job = MiningJob {
            job_id: 0,
            height: 0,
            parent_hash: H256::zero(),
            pre_hash: H256::zero(),
            target: H256::repeat_byte(0xff),
            share_target: H256::repeat_byte(0xff),
            seed_height: 0,
            seed: H256::zero(),
        };

        let (job_tx, job_rx) = watch::channel(dummy_job);
        let (found_tx, found_rx) = mpsc::unbounded_channel::<FoundNonce>();
        let (found_share_tx, found_share_rx) = mpsc::unbounded_channel::<FoundShare>();

        let total_hashes = Arc::new(AtomicU64::new(0));
        let per_thread_hashes = Arc::new(
            (0..num_threads)
                .map(|_| AtomicU64::new(0))
                .collect::<Vec<_>>(),
        );

        for thread_id in 0..num_threads {
            let mut job_rx = job_rx.clone();
            let found_tx = found_tx.clone();
            let found_share_tx = found_share_tx.clone();

            let total_hashes = total_hashes.clone();
            let per_thread_hashes = per_thread_hashes.clone();

            std::thread::Builder::new()
                .name(format!("pow-worker-{}", thread_id))
                .spawn(move || {
                    const CHUNK_ITERS: u64 = 20;

                    let flags = rx_lx::Flags::recommended();
                    let mut cache = match rx_lx::Cache::alloc(flags) {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("Worker {}: Cache alloc failed: {:?}", thread_id, e);
                            return;
                        }
                    };

                    let mut vm_opt: Option<rx_lx::Vm> = None;
                    let mut last_seed_height: u64 = u64::MAX;

                    let mut local_nonce = [0u8; 32];
                    let seed_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                    local_nonce[0..8].copy_from_slice(&seed_time.to_le_bytes());
                    local_nonce[8..16].copy_from_slice(&(thread_id as u64).to_le_bytes());

                    loop {
                        if futures::executor::block_on(job_rx.changed()).is_err() {
                            return;
                        }
                        let job = *job_rx.borrow();

                        if job.seed_height != last_seed_height {
                            let seed_bytes: [u8; 32] = job.seed.into();
                            cache.init(&seed_bytes);

                            match rx_lx::Vm::light(flags, &cache) {
                                Ok(vm) => vm_opt = Some(vm),
                                Err(e) => {
                                    log::error!(
                                        "Worker {}: VM creation failed: {:?}",
                                        thread_id,
                                        e
                                    );
                                    vm_opt = None;
                                    continue;
                                }
                            }
                            last_seed_height = job.seed_height;
                        }

                        let Some(vm) = vm_opt.as_ref() else {
                            continue;
                        };

                        loop {
                            let job_now = *job_rx.borrow();
                            if job_now.job_id != job.job_id {
                                break;
                            }

                            let pre_hash = job.pre_hash;
                            let target = job.target;
                            let share_target = job.share_target;

                            for _ in 0..CHUNK_ITERS {
                                // increment nonce
                                for i in 0..32 {
                                    if local_nonce[i] == 255 {
                                        local_nonce[i] = 0;
                                    } else {
                                        local_nonce[i] += 1;
                                        break;
                                    }
                                }

                                // input = header_hash || nonce
                                let mut input = [0u8; 64];
                                input[..32].copy_from_slice(pre_hash.as_ref());
                                input[32..].copy_from_slice(&local_nonce);

                                let pow_hash = sp_core::H256::from_slice(&vm.hash(&input));

                                // hash counters
                                per_thread_hashes[thread_id].fetch_add(1, Ordering::Relaxed);
                                total_hashes.fetch_add(1, Ordering::Relaxed);

                                // Check for share first (easier target)
                                if pow_hash <= share_target {
                                    let _ = found_share_tx.send(FoundShare {
                                        job_id: job.job_id,
                                        height: job.height,
                                        parent_hash: job.parent_hash,
                                        nonce: local_nonce,
                                    });
                                }
                                // Check for block (harder target)
                                if pow_hash <= target {
                                    let _ = found_tx.send(FoundNonce {
                                        job_id: job.job_id,
                                        nonce: local_nonce,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                })
                .expect("failed to spawn pow worker thread");
        }

        Self {
            job_tx,
            found_rx,
            found_share_rx,
            job_id: 0,
            last_parent_hash: None,
            total_hashes,
            per_thread_hashes,
        }
    }

    fn next_job_id_if_parent_changed(&mut self, parent_hash: H256) -> (u64, bool) {
        match self.last_parent_hash {
            Some(prev) if prev == parent_hash => (self.job_id, false),
            _ => {
                self.job_id = self.job_id.wrapping_add(1).max(1);
                self.last_parent_hash = Some(parent_hash);
                (self.job_id, true)
            }
        }
    }

    fn snapshot_hash_counts(&self) -> (u64, Vec<u64>) {
        let total = self.total_hashes.load(Ordering::Relaxed);
        let per = self
            .per_thread_hashes
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect::<Vec<_>>();
        (total, per)
    }
}

// ============================================================
// POW TEMPLATE STRUCTURES (Option A - Reactive Mining)
// ============================================================

// Template congelato associato a job_id
pub(crate) struct PowTemplate {
    pub job_id: u64,
    pub height: u64,
    pub parent_hash: H256,

    pub header_no_seal: <Block as BlockT>::Header,
    pub body: Vec<<Block as BlockT>::Extrinsic>,
    pub storage_changes: sp_state_machine::StorageChanges<<<Block as BlockT>::Header as HeaderT>::Hashing>,

    pub pre_hash: H256,
    pub target: H256,
    pub share_target: H256,
    pub seed_height: u64,
    pub seed: H256,
}

// LRU max 2 templates (active + previous)
pub(crate) struct TemplateLru2 {
    order: std::collections::VecDeque<u64>,
    map: std::collections::HashMap<u64, PowTemplate>,
}

impl TemplateLru2 {
    pub fn new() -> Self {
        Self { order: std::collections::VecDeque::new(), map: std::collections::HashMap::new() }
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.map.clear();
    }

    pub fn insert(&mut self, tpl: PowTemplate) {
        let id = tpl.job_id;

        if self.map.contains_key(&id) {
            self.map.insert(id, tpl);
            self.order.retain(|x| *x != id);
            self.order.push_back(id);
        } else {
            self.map.insert(id, tpl);
            self.order.push_back(id);
        }

        while self.order.len() > 2 {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
    }

    pub fn remove(&mut self, job_id: u64) -> Option<PowTemplate> {
        self.order.retain(|x| *x != job_id);
        self.map.remove(&job_id)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

pub(crate) fn next_job_id(job_id: &mut u64) -> u64 {
    *job_id = job_id.wrapping_add(1).max(1);
    *job_id
}

pub(crate) fn miner_digest_for_template(miner_address: [u8; 32]) -> sp_runtime::generic::Digest {
    let miner_digest_data = MinerAddressDigest { miner: miner_address };
    sp_runtime::generic::Digest {
        logs: vec![sp_runtime::generic::DigestItem::PreRuntime(
            LUMENYX_ENGINE_ID,
            miner_digest_data.encode(),
        )],
    }
}

#[allow(dead_code)]
fn compute_pow_hash_blake3(data: &[u8], nonce: &[u8; 32]) -> H256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(nonce);
    H256::from_slice(&hasher.finalize().as_bytes()[..32])
}

const MIN_DIFFICULTY: u128 = 1;
const MAX_DIFFICULTY: u128 = u128::MAX;
const POW_LIMIT: H256 = H256::repeat_byte(0xff);

fn difficulty_to_target(difficulty: u128) -> H256 {
    let mut d = difficulty;
    if d < MIN_DIFFICULTY {
        d = MIN_DIFFICULTY;
    }
    if d > MAX_DIFFICULTY {
        d = MAX_DIFFICULTY;
    }
    if d == 0 {
        d = MIN_DIFFICULTY;
    }

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

fn hash_meets_target(hash: &H256, target: &H256) -> bool {
    hash <= target
}

/// RxLxVerifier - Verifica blocchi PoW e gestisce correttamente il seal
pub struct RxLxVerifier;

#[async_trait::async_trait]
impl Verifier<Block> for RxLxVerifier {
    async fn verify(
        &self,
        mut block: BlockImportParams<Block>,
    ) -> Result<BlockImportParams<Block>, String> {
        let mut moved: Vec<DigestItem> = Vec::new();

        block.header.digest_mut().logs.retain(|item| {
            let dominated = item
                .as_pre_runtime()
                .map(|(id, _)| id == LUMENYX_ENGINE_ID)
                .unwrap_or(false);
            let is_seal = matches!(item, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID);
            let is_consensus =
                matches!(item, DigestItem::Consensus(id, _) if *id == LUMENYX_ENGINE_ID);

            if dominated {
                return true;
            }
            if is_seal || is_consensus {
                moved.push(item.clone());
                return false;
            }
            true
        });

        let has_seal = moved.iter().any(|d| matches!(d, DigestItem::Seal(_, _)));
        if !has_seal {
            return Err("Missing LUMENYX seal in header digest".into());
        }

        block.post_digests.retain(|d| {
            !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID)
                && !matches!(d, DigestItem::Consensus(id, _) if *id == LUMENYX_ENGINE_ID)
        });
        block.post_digests.extend(moved);

        Ok(block)
    }
}

pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            Arc<FullClient>,
            Option<Telemetry>,
            Arc<FrontierBackend>,
            FrontierPartialComponents,
            LumenyxPowBlockImport<FullClient>,
        ),
    >,
    ServiceError,
> {
    let network_path = config
        .network
        .net_config_path
        .clone()
        .unwrap_or_else(|| config.base_path.config_dir(config.chain_spec.id()))
        .join("network");
    let _ = std::fs::create_dir_all(&network_path);
    let secret_key_path = network_path.join("secret_ed25519");
    if !secret_key_path.exists() {
        use sp_core::Pair;
        let keypair = sp_core::ed25519::Pair::generate().0;
        let _ = std::fs::write(&secret_key_path, keypair.to_raw_vec());
        log::info!("🔑 Generated network key at {:?}", secret_key_path);
    }

    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = sc_service::new_wasm_executor::<sp_io::SubstrateHostFunctions>(&config.executor);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let verifier = RxLxVerifier;
    let pow_block_import = LumenyxPowBlockImport::new(client.clone());

    let import_queue = sc_consensus::BasicQueue::new(
        verifier,
        Box::new(pow_block_import.clone()),
        None,
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    let frontier_backend = Arc::new(FrontierBackend::KeyValue(Arc::new(
        fc_db::kv::Backend::open(
            Arc::clone(&client),
            &config.database,
            &db_config_dir(&config),
        )?,
    )));

    let frontier_partial = FrontierPartialComponents {
        filter_pool: Some(Arc::new(std::sync::Mutex::new(BTreeMap::new()))),
        fee_history_cache: Arc::new(std::sync::Mutex::new(BTreeMap::new())),
        fee_history_cache_limit: 2048,
    };

    Ok(sc_service::PartialComponents {
        client: client.clone(),
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (
            client,
            telemetry,
            frontier_backend,
            frontier_partial,
            pow_block_import,
        ),
    })
}

pub fn new_full(config: Configuration, pool_mode: bool) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (_, mut telemetry, frontier_backend, frontier_partial, pow_block_import),
    } = new_partial(&config)?;

    let mut net_config = sc_network::config::FullNetworkConfiguration::<
        Block,
        <Block as BlockT>::Hash,
        sc_network::NetworkWorker<Block, <Block as BlockT>::Hash>,
    >::new(&config.network, config.prometheus_registry().cloned());

    // P2Pool: Register pool protocol if pool_mode
    let pool_notification_service = if pool_mode {
        let pool_protocol: sc_network::ProtocolName = POOL_PROTO_NAME.into();
        let (pool_config, pool_notif_service) = sc_network::config::NonDefaultSetConfig::new(
            pool_protocol.clone(),
            vec![],
            1024 * 1024,
            None,
            sc_network::config::SetConfig {
                in_peers: 25,
                out_peers: 25,
                reserved_nodes: vec![],
                non_reserved_mode: sc_network::config::NonReservedPeerMode::Accept,
            },
        );
        net_config.add_notification_protocol(pool_config);
        log::info!("🏊 Pool protocol registered: {}", POOL_PROTO_NAME);
        Some(pool_notif_service)
    } else {
        None
    };
    let metrics = sc_network::NotificationMetrics::new(config.prometheus_registry());

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics,
        })?;
    // P2Pool: Spawn gossip task
    let mut pool_gossip = if let Some(notif_service) = pool_notification_service {
        log::info!("🏊 Starting pool gossip task...");
        let gossip = spawn_pool_gossip_task(notif_service, task_manager.spawn_handle());
        log::info!("🏊 Pool gossip task started!");
        Some(gossip)
    } else {
        None
    };

    let role = config.role;
    let force_authoring = config.force_authoring;
    let miner_base_path = config.base_path.path().to_path_buf();
    let prometheus_registry = config.prometheus_registry().cloned();

    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));
    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        storage_override.clone(),
        50,
        50,
        prometheus_registry.clone(),
    ));

    let pubsub_notification_sinks: Arc<
        EthereumBlockNotificationSinks<EthereumBlockNotification<Block>>,
    > = Default::default();

    match &*frontier_backend {
        fc_db::Backend::KeyValue(b) => {
            task_manager.spawn_essential_handle().spawn(
                "frontier-mapping-sync",
                Some("frontier"),
                MappingSyncWorker::new(
                    client.import_notification_stream(),
                    Duration::new(6, 0),
                    client.clone(),
                    backend.clone(),
                    storage_override.clone(),
                    b.clone(),
                    3,
                    0u32.into(),
                    SyncStrategy::Normal,
                    sync_service.clone(),
                    pubsub_notification_sinks.clone(),
                )
                .for_each(|()| futures::future::ready(())),
            );
        }
    }

    task_manager.spawn_essential_handle().spawn(
        "frontier-fee-history",
        None,
        EthTask::fee_history_task(
            client.clone(),
            storage_override.clone(),
            frontier_partial.fee_history_cache.clone(),
            frontier_partial.fee_history_cache_limit,
        ),
    );

    let filter_pool = frontier_partial.filter_pool.clone();
    let fee_history_cache = frontier_partial.fee_history_cache.clone();
    let fee_history_cache_limit = frontier_partial.fee_history_cache_limit;

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();
        let network = network.clone();
        let sync_service = sync_service.clone();
        let frontier_backend = frontier_backend.clone();
        let storage_override = storage_override.clone();
        let block_data_cache = block_data_cache.clone();
        let filter_pool = filter_pool.clone();
        let fee_history_cache = fee_history_cache.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();

        Box::new(move |subscription_task_executor| {
            let eth_deps = crate::rpc::EthDeps {
                client: client.clone(),
                pool: pool.clone(),
                graph: pool.pool().clone(),
                converter: Some(lumenyx_runtime::TransactionConverter::<Block>::default()),
                is_authority: role.is_authority(),
                enable_dev_signer: false,
                network: network.clone(),
                sync: sync_service.clone(),
                frontier_backend: match &*frontier_backend {
                    fc_db::Backend::KeyValue(b) => b.clone(),
                    _ => unreachable!(),
                },
                storage_override: storage_override.clone(),
                block_data_cache: block_data_cache.clone(),
                filter_pool: filter_pool.clone(),
                max_past_logs: 10000,
                fee_history_cache: fee_history_cache.clone(),
                fee_history_cache_limit: fee_history_cache_limit,
                execute_gas_limit_multiplier: 10,
                forced_parent_hashes: None,
                pending_create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    Ok(timestamp)
                },
            };

            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe: sc_rpc::DenyUnsafe::No,
                eth: eth_deps,
            };

            crate::rpc::create_full(
                deps,
                subscription_task_executor,
                pubsub_notification_sinks.clone(),
            )
            .map_err(Into::into)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        rpc_builder: rpc_extensions_builder,
        backend: backend.clone(),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
    })?;

    // ============================================
    // POW MINING WITH DYNAMIC DIFFICULTY (ROBUST MULTI-THREAD + HASHRATE)
    // ============================================
    if role.is_authority() || force_authoring {
        use sc_basic_authorship::ProposerFactory;
        use sp_consensus::{Environment, Proposer};
        use sp_inherents::InherentDataProvider;

        let miner_pair = get_or_create_miner_key(&miner_base_path);
        let miner_address: [u8; 32] = miner_pair.public().0;

        log::info!(
            "💰 Mining rewards to: {}",
            miner_pair.public().to_ss58check()
        );

        let num_threads: usize = std::env::var("LUMENYX_MINING_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(num_cpus::get)
            .max(1);

        log::info!("⛏️  Starting PoW miner with {} threads...", num_threads);

        let mut proposer_factory = ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
        );

        let mining_client = client.clone();
        let block_import = pow_block_import.clone();
        let select_chain_mining =
            TotalDifficultySelectChain::new(select_chain.clone(), client.clone());
        let mining_sync_service = sync_service.clone();
        let mining_tx_pool = transaction_pool.clone();

        task_manager.spawn_handle().spawn_blocking(
            "pow-miner",
            Some("mining"),
            async move {
                let mut consecutive_propose_failures: u32 = 0;
                const MAX_FAILURES_BEFORE_POOL_CLEAR: u32 = 3;
                let mut last_difficulty: u128 = FALLBACK_DIFFICULTY;
                let mut blocks_since_difficulty_log: u32 = 0;

                let get_block_hash = |height: u64| -> H256 {
                    let block_num: u32 = height as u32;
                    match mining_client.hash(block_num) {
                        Ok(Some(hash)) => hash,
                        _ => H256::zero(),
                    }
                };

                let mut rx_lx_pow = match RxLxPow::new(0, &get_block_hash) {
                    Ok(pow) => pow,
                    Err(e) => {
                        log::error!("❌ Failed to initialize RX-LX: {}", e);
                        return;
                    }
                };
                log::info!("⛏️  RX-LX PoW engine initialized!");

                let mut miner_state = MinerState::new(num_threads);
                let sharechain = std::sync::Arc::new(std::sync::Mutex::new(Sharechain::new()));

                // Hashrate reporter
                let mut last_report = tokio::time::Instant::now();
                let mut last_total: u64 = 0;
                let mut last_per_thread: Vec<u64> = vec![0; num_threads];

                let mut import_stream = mining_client.import_notification_stream();
                let mut active_best_hash: Option<H256> = None;
                let mut templates: TemplateLru2 = TemplateLru2::new();

                // Bootstrap: template sul best attuale
                {
                    let best_header = match select_chain_mining.best_chain().await {
                        Ok(h) => h,
                        Err(e) => {
                            log::error!("❌ select_chain_mining.best_chain failed: {:?}", e);
                            return;
                        }
                    };

                    let best_hash: H256 = best_header.hash();
                    active_best_hash = Some(best_hash);

                    let new_job_id = next_job_id(&mut miner_state.job_id);

                    let parent_hash: H256 = best_header.hash();
                    let parent_number: u32 = (*best_header.number()).into();
                    let height: u64 = (parent_number as u64) + 1;

                    let difficulty: u128 = read_difficulty_from_storage(&*mining_client, parent_hash);
                    let target: H256 = difficulty_to_target(difficulty);
                    let share_difficulty = (difficulty / SHARE_DIFFICULTY_DIVISOR).max(1);
                    let share_target: H256 = difficulty_to_target(share_difficulty);

                    let seed_height_val: u64 = seed_sched::seed_height(height);
                    if let Err(e) = rx_lx_pow.maybe_reseed(height, &get_block_hash) {
                        log::error!("❌ RX-LX reseed failed: {}", e);
                        return;
                    }
                    let seed: H256 = rx_lx_pow.seed;

                    use sp_inherents::InherentDataProvider;
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let inherent_data = match timestamp.create_inherent_data().await {
                        Ok(d) => d,
                        Err(e) => {
                            log::error!("❌ Failed to create inherent data: {:?}", e);
                            return;
                        }
                    };

                    let digest = miner_digest_for_template(miner_address);

                    let proposer = match proposer_factory.init(&best_header).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("❌ proposer_factory.init failed: {:?}", e);
                            return;
                        }
                    };

                    let proposal = match proposer.propose(inherent_data, digest, Duration::from_millis(800), None).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::error!("❌ proposer.propose failed: {:?}", e);
                            return;
                        }
                    };

                    let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                    let (header, body) = block.deconstruct();

                    let mut header_no_seal = header.clone();
                    header_no_seal.digest_mut().logs.retain(|d| {
                        !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID)
                    });

                    let pre_hash: H256 = header_no_seal.hash();

                    let tpl = PowTemplate {
                        job_id: new_job_id,
                        height,
                        parent_hash,
                        header_no_seal,
                        body,
                        storage_changes,
                        pre_hash,
                        target,
                        share_target,
                        seed_height: seed_height_val,
                        seed,
                    };

                    let job = MiningJob {
                        job_id: tpl.job_id,
                        height: tpl.height,
                        parent_hash: tpl.parent_hash,
                        pre_hash: tpl.pre_hash,
                        target: tpl.target,
                        share_target: tpl.share_target,
                        seed_height: tpl.seed_height,
                        seed: tpl.seed,
                    };

                    templates.insert(tpl);
                    let _ = miner_state.job_tx.send(job);

                    log::info!(
                        "⛏️ Mining started: #{} parent={:?} job_id={} pre_hash={:?} difficulty={}",
                        job.height, job.parent_hash, job.job_id, job.pre_hash, difficulty
                    );
                }

                loop {
                    if mining_sync_service.is_major_syncing() {
                        tokio::time::sleep(Duration::from_millis(2000)).await;
                        continue;
                    }

                    tokio::select! {
                        maybe_n = import_stream.next() => {
                            let Some(n) = maybe_n else { break; };

                            if !n.is_new_best {
                                continue;
                            }

                            let new_best_hash: H256 = n.hash;

                            if active_best_hash == Some(new_best_hash) {
                                continue;
                            }

                            active_best_hash = Some(new_best_hash);
                            templates.clear();

                            let new_job_id = next_job_id(&mut miner_state.job_id);

                            let parent_hash: H256 = n.header.hash();
                            let parent_number: u32 = (*n.header.number()).into();
                            let height: u64 = (parent_number as u64) + 1;

                            let difficulty: u128 = read_difficulty_from_storage(&*mining_client, parent_hash);
                            let target: H256 = difficulty_to_target(difficulty);
                            let share_difficulty = (difficulty / SHARE_DIFFICULTY_DIVISOR).max(1);
                            let share_target: H256 = difficulty_to_target(share_difficulty);

                            let seed_height_val: u64 = seed_sched::seed_height(height);
                            if let Err(e) = rx_lx_pow.maybe_reseed(height, &get_block_hash) {
                                log::error!("❌ RX-LX reseed failed: {}", e);
                                continue;
                            }
                            let seed: H256 = rx_lx_pow.seed;

                            use sp_inherents::InherentDataProvider;
                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                            let inherent_data = match timestamp.create_inherent_data().await {
                                Ok(d) => d,
                                Err(e) => {
                                    log::warn!("Failed to create inherent data: {:?}", e);
                                    continue;
                                }
                            };

                            let digest = miner_digest_for_template(miner_address);

                            let proposer = match proposer_factory.init(&n.header).await {
                                Ok(p) => p,
                                Err(e) => {
                                    log::warn!("Failed to create proposer: {:?}", e);
                                    continue;
                                }
                            };

                            let proposal = match proposer.propose(inherent_data, digest, Duration::from_millis(800), None).await {
                                Ok(p) => p,
                                Err(e) => {
                                    log::warn!("Failed to propose block: {:?}", e);
                                    continue;
                                }
                            };

                            let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                            let (header, body) = block.deconstruct();

                            let mut header_no_seal = header.clone();
                            header_no_seal.digest_mut().logs.retain(|d| {
                                !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID)
                            });

                            let pre_hash: H256 = header_no_seal.hash();

                            let tpl = PowTemplate {
                                job_id: new_job_id,
                                height,
                                parent_hash,
                                header_no_seal,
                                body,
                                storage_changes,
                                pre_hash,
                                target,
                                share_target,
                                seed_height: seed_height_val,
                                seed,
                            };

                            let job = MiningJob {
                                job_id: tpl.job_id,
                                height: tpl.height,
                                parent_hash: tpl.parent_hash,
                                pre_hash: tpl.pre_hash,
                                target: tpl.target,
                                share_target: tpl.share_target,
                                seed_height: tpl.seed_height,
                                seed: tpl.seed,
                            };

                            templates.insert(tpl);
                            let _ = miner_state.job_tx.send(job);

                            log::info!(
                                "⛏️ New best imported (origin={:?}): mining #{} parent={:?} job_id={} pre_hash={:?} difficulty={}",
                                n.origin, job.height, job.parent_hash, job.job_id, job.pre_hash, difficulty
                            );
                        }

                        maybe_found = miner_state.found_rx.recv() => {
                            let Some(found) = maybe_found else { break; };

                            let Some(tpl) = templates.remove(found.job_id) else {
                                log::debug!("stale nonce dropped job_id={}", found.job_id);
                                continue;
                            };

                            let pow_hash = rx_lx_pow.hash(&tpl.pre_hash, &found.nonce);
                            if pow_hash > tpl.target {
                                log::warn!(
                                    "nonce invalid locally job_id={} height={} pow_hash={:?} target={:?}",
                                    tpl.job_id, tpl.height, pow_hash, tpl.target
                                );
                                continue;
                            }

                            log::info!(
                                "🎯 Nonce found job_id={} height={} parent={:?} pre_hash={:?} pow_hash={:?}",
                                tpl.job_id, tpl.height, tpl.parent_hash, tpl.pre_hash, pow_hash
                            );

                            let mut import_params = BlockImportParams::new(BlockOrigin::Own, tpl.header_no_seal.clone());
                            let seal = DigestItem::Seal(LUMENYX_ENGINE_ID, found.nonce.to_vec());
                            import_params.post_digests.push(seal);

                            import_params.body = Some(tpl.body.clone());
                            import_params.state_action = sc_consensus::StateAction::ApplyChanges(
                                sc_consensus::StorageChanges::Changes(tpl.storage_changes),
                            );

                            match block_import.import_block(import_params).await {
                                Ok(r) => log::info!("✅ mined import_result={:?} job_id={} height={} pre_hash={:?}", r, tpl.job_id, tpl.height, tpl.pre_hash),
                                Err(e) => log::error!("❌ Failed to import mined block job_id={} height={} err={:?}", tpl.job_id, tpl.height, e),
                            }

                            // re-arm if cache empty
                            if templates.len() == 0 {
                                if let Some(best_hash) = active_best_hash {
                                    match mining_client.header(best_hash) {
                                        Ok(Some(best_header)) => {
                                            let new_job_id = next_job_id(&mut miner_state.job_id);

                                            let parent_hash: H256 = best_header.hash();
                                            let parent_number: u32 = (*best_header.number()).into();
                                            let height: u64 = (parent_number as u64) + 1;

                                            let difficulty: u128 = read_difficulty_from_storage(&*mining_client, parent_hash);
                                            let target: H256 = difficulty_to_target(difficulty);
                                            let share_difficulty = (difficulty / SHARE_DIFFICULTY_DIVISOR).max(1);
                                            let share_target: H256 = difficulty_to_target(share_difficulty);

                                            let seed_height_val: u64 = seed_sched::seed_height(height);
                                            if let Err(e) = rx_lx_pow.maybe_reseed(height, &get_block_hash) {
                                                log::error!("❌ RX-LX reseed failed on re-arm: {}", e);
                                                continue;
                                            }
                                            let seed: H256 = rx_lx_pow.seed;

                                            use sp_inherents::InherentDataProvider;
                                            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                                            let inherent_data = match timestamp.create_inherent_data().await {
                                                Ok(d) => d,
                                                Err(e) => {
                                                    log::warn!("Re-arm: failed to create inherent data: {:?}", e);
                                                    continue;
                                                }
                                            };

                                            let digest = miner_digest_for_template(miner_address);

                                            let proposer = match proposer_factory.init(&best_header).await {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::warn!("Re-arm: proposer_factory.init failed: {:?}", e);
                                                    continue;
                                                }
                                            };

                                            let proposal = match proposer.propose(inherent_data, digest, Duration::from_millis(800), None).await {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    log::warn!("Re-arm: proposer.propose failed: {:?}", e);
                                                    continue;
                                                }
                                            };

                                            let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                                            let (header, body) = block.deconstruct();

                                            let mut header_no_seal = header.clone();
                                            header_no_seal.digest_mut().logs.retain(|d| {
                                                !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID)
                                            });

                                            let pre_hash: H256 = header_no_seal.hash();

                                            let tpl = PowTemplate {
                                                job_id: new_job_id,
                                                height,
                                                parent_hash,
                                                header_no_seal,
                                                body,
                                                storage_changes,
                                                pre_hash,
                                                target,
                                                share_target,
                                                seed_height: seed_height_val,
                                                seed,
                                            };

                                            let job = MiningJob {
                                                job_id: tpl.job_id,
                                                height: tpl.height,
                                                parent_hash: tpl.parent_hash,
                                                pre_hash: tpl.pre_hash,
                                                target: tpl.target,
                                                share_target: tpl.share_target,
                                                seed_height: tpl.seed_height,
                                                seed: tpl.seed,
                                            };

                                            templates.insert(tpl);
                                            let _ = miner_state.job_tx.send(job);

                                            log::info!(
                                                "⛏️ Re-armed mining: #{} parent={:?} job_id={} pre_hash={:?} difficulty={}",
                                                job.height, job.parent_hash, job.job_id, job.pre_hash, difficulty
                                            );
                                        }
                                        Ok(None) => log::warn!("re-arm: best_header not found for hash={:?}", best_hash),
                                        Err(e) => log::error!("re-arm: mining_client.header failed: {:?}", e),
                                    }
                                }
                            }
                        }
                    }
                }
            },
        );

        log::info!("🚀 PoW miner started with dynamic difficulty!");
    } else {
        log::info!("📡 Sync-only mode (not mining)");
    }

    log::info!("✅ LUMENYX PoW running!");
    network_starter.start_network();
    Ok(task_manager)
}
