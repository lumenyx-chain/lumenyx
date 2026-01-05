//! LUMENYX Service Configuration - GHOSTDAG PoW with proper fork choice
//!
//! Key changes from previous version:
//! 1. Uses GhostdagSelectChain instead of LongestChain
//! 2. Verifier processes blocks through GHOSTDAG (calculates blue_score/blue_work)
//! 3. Fork choice based on blue_work, not block number
//! 4. All blocks (own + received) are added to DAG store

use std::{collections::BTreeMap, sync::Arc, time::Duration};
use std::fs;
use sp_core::{sr25519, Pair, crypto::Ss58Codec, H256};
use sp_runtime::generic::DigestItem;
use codec::{Encode, Decode};

use futures::prelude::*;
use lumenyx_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{BlockBackend, BlockchainEvents, AuxStore, HeaderBackend};
use sc_consensus::{BlockImportParams, import_queue::Verifier, BlockImport};
use sc_executor::WasmExecutor;
use sc_service::{
    error::Error as ServiceError, Configuration, TaskManager, TFullBackend, TFullClient,
};
use sc_transaction_pool_api::TransactionPool;
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus::{BlockOrigin, SelectChain, SyncOracle};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_api::ProvideRuntimeApi;

// GHOSTDAG imports
use sc_consensus_ghostdag::{GhostdagStore, GhostdagData};

// Custom select chain
use crate::ghostdag_select::GhostdagSelectChain;

// Frontier imports
use fc_mapping_sync::{EthereumBlockNotification, EthereumBlockNotificationSinks, SyncStrategy, kv::MappingSyncWorker};
use fc_rpc::EthTask;
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fc_storage::StorageOverrideHandler;

pub fn db_config_dir(config: &Configuration) -> std::path::PathBuf {
    config.base_path.config_dir(config.chain_spec.id())
}

pub type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>;
type FullBackend = TFullBackend<Block>;
type FullSelectChain = GhostdagSelectChain<Block, FullBackend, FullClient>;
pub type FrontierBackend = fc_db::Backend<Block, FullClient>;

pub struct FrontierPartialComponents {
    pub filter_pool: Option<FilterPool>,
    pub fee_history_cache: FeeHistoryCache,
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
}

const GHOSTDAG_K: u64 = 18;
const TARGET_BLOCK_TIME_MS: u64 = 1000;
const INITIAL_DIFFICULTY: u64 = 100;

/// GHOSTDAG Engine ID for digests
const GHOSTDAG_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"GDAG";

/// Miner address digest structure
#[derive(Clone, codec::Encode, codec::Decode)]
struct MinerAddressDigest {
    miner: [u8; 32],
}

/// DAG parents digest - for multi-parent blocks
#[derive(Clone, codec::Encode, codec::Decode)]
struct DagParentsDigest {
    parents: Vec<H256>,
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

fn compute_pow_hash(data: &[u8], nonce: &[u8; 32]) -> H256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(nonce);
    H256::from_slice(&hasher.finalize().as_bytes()[..32])
}

fn difficulty_to_target(difficulty: u64) -> H256 {
    if difficulty == 0 { return H256::repeat_byte(0xff); }
    let mut target = [0u8; 32];
    let divisor = difficulty as u128;
    let mut remainder = 0u128;
    for i in 0..32 {
        let current = (remainder << 8) | 0xff_u128;
        target[i] = (current / divisor) as u8;
        remainder = current % divisor;
    }
    H256::from_slice(&target)
}

fn hash_meets_target(hash: &H256, target: &H256) -> bool {
    hash.as_bytes() <= target.as_bytes()
}

/// GHOSTDAG Verifier - processes blocks through DAG and sets fork choice
pub struct GhostdagVerifier<C> {
    client: Arc<C>,
    ghostdag_store: GhostdagStore<C>,
}

impl<C> GhostdagVerifier<C> {
    pub fn new(client: Arc<C>) -> Self {
        let ghostdag_store = GhostdagStore::new(client.clone());
        Self { client, ghostdag_store }
    }
}

impl<C: AuxStore + HeaderBackend<Block> + Send + Sync> GhostdagVerifier<C> {
    /// Process a block through GHOSTDAG
    fn process_ghostdag(&self, block_hash: H256, parent_hash: H256, extra_parents: Vec<H256>) -> Result<GhostdagData, String> {
        // Collect all parents (header parent + extra parents from digest)
        let mut all_parents = vec![parent_hash];
        for p in extra_parents {
            if p != parent_hash && !all_parents.contains(&p) {
                all_parents.push(p);
            }
        }

        // For genesis, parent is zero
        if parent_hash == H256::zero() || all_parents.iter().all(|p| *p == H256::zero()) {
            let genesis_data = GhostdagData {
                blue_score: 0,
                blue_work: 1, // Genesis has work 1
                selected_parent: H256::zero(),
                mergeset_blues: vec![],
                mergeset_reds: vec![],
                blues_anticone_sizes: vec![],
            };
            self.ghostdag_store.store_ghostdag_data(&block_hash, &genesis_data)?;
            self.ghostdag_store.store_parents(&block_hash, &[])?;
            self.ghostdag_store.update_tips(&block_hash, &[])?;
            return Ok(genesis_data);
        }

        // CRITICAL FIX (as per ChatGPT/Grok advice):
        // Verify the primary parent exists in our GHOSTDAG store
        // If not, return UnknownParent error - Substrate sync will request it
        if self.ghostdag_store.get_ghostdag_data(&parent_hash).is_none() {
            return Err(format!("UnknownParent: {:?} not in GHOSTDAG store", parent_hash));
        }

        // Find selected parent (highest blue_work) - only from parents we HAVE
        let selected_parent = all_parents.iter()
            .filter_map(|p| {
                self.ghostdag_store.get_ghostdag_data(p).map(|d| (*p, d.blue_work))
            })
            .max_by(|a, b| {
                match a.1.cmp(&b.1) {
                    std::cmp::Ordering::Equal => b.0.cmp(&a.0), // Lower hash wins tie
                    other => other,
                }
            })
            .map(|(h, _)| h)
            .ok_or_else(|| format!("UnknownParent: no valid parent found for {:?}", block_hash))?;

        // Get selected parent's data - MUST exist now
        let parent_data = self.ghostdag_store.get_ghostdag_data(&selected_parent)
            .ok_or_else(|| format!("UnknownParent: selected parent {:?} missing", selected_parent))?;

        // Mergeset: other parents that we have in our store become blues
        let mergeset_blues: Vec<H256> = all_parents.iter()
            .filter(|p| **p != selected_parent)
            .filter(|p| self.ghostdag_store.get_ghostdag_data(p).is_some())
            .cloned()
            .collect();

        // Calculate blue_score and blue_work
        let blue_score = parent_data.blue_score + 1 + mergeset_blues.len() as u64;
        let blue_work = parent_data.blue_work + 1 + mergeset_blues.len() as u128;

        let data = GhostdagData {
            blue_score,
            blue_work,
            selected_parent,
            mergeset_blues,
            mergeset_reds: vec![],
            blues_anticone_sizes: vec![],
        };

        // Store in DAG
        self.ghostdag_store.store_ghostdag_data(&block_hash, &data)?;
        self.ghostdag_store.store_parents(&block_hash, &all_parents)?;
        self.ghostdag_store.update_tips(&block_hash, &all_parents)?;

        // Update children for each parent we have
        for parent in &all_parents {
            if self.ghostdag_store.get_ghostdag_data(parent).is_some() {
                let _ = self.ghostdag_store.add_child(parent, &block_hash);
            }
        }

        log::info!(
            "🔷 GHOSTDAG: block {:?} blue_score={} blue_work={} selected_parent={:?}",
            block_hash, data.blue_score, data.blue_work, data.selected_parent
        );

        Ok(data)
    }
}

#[async_trait::async_trait]
impl<C: AuxStore + HeaderBackend<Block> + Send + Sync> Verifier<Block> for GhostdagVerifier<C> {
    async fn verify(
        &self,
        mut block: BlockImportParams<Block>,
    ) -> Result<BlockImportParams<Block>, String> {
        let block_hash = block.header.hash();
        let parent_hash = *block.header.parent_hash();
        
        // Extract extra parents from digest (if any)
        let mut extra_parents = Vec::new();
        for log in block.header.digest().logs() {
            if let DigestItem::Other(data) = log {
                if let Ok(digest) = DagParentsDigest::decode(&mut &data[..]) {
                    extra_parents = digest.parents;
                    break;
                }
            }
        }

        // Process through GHOSTDAG
        let block_h256 = H256::from_slice(block_hash.as_ref());
        let parent_h256 = H256::from_slice(parent_hash.as_ref());
        
        let ghostdag_data = self.process_ghostdag(block_h256, parent_h256, extra_parents)?;

        // CRITICAL: Compare blue_work with current best to decide fork choice
        // Only set Custom(true) if this block has more blue_work than current best
        let current_best_hash = self.client.info().best_hash;
        let current_best_h256 = H256::from_slice(current_best_hash.as_ref());
        let current_best_work = self.ghostdag_store.get_ghostdag_data(&current_best_h256)
            .map(|d| d.blue_work)
            .unwrap_or(0);
        
        // This block becomes best if it has more blue_work, or same work but lower hash (tie-breaker)
        let is_better = ghostdag_data.blue_work > current_best_work || 
            (ghostdag_data.blue_work == current_best_work && block_h256 < current_best_h256);
        
        block.fork_choice = Some(sc_consensus::ForkChoiceStrategy::Custom(is_better));

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
        ),
    >,
    ServiceError,
> {
    // Auto-create network key
    let network_path = config.network.net_config_path.clone()
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

    let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, _>(
        config,
        telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
        executor,
    )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    // Use GHOSTDAG select chain instead of LongestChain
    let select_chain = GhostdagSelectChain::new(backend.clone(), client.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    // Use GHOSTDAG verifier
    let verifier = GhostdagVerifier::new(client.clone());
    
    let import_queue = sc_consensus::BasicQueue::new(
        verifier,
        Box::new(client.clone()),
        None,
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    let frontier_backend = Arc::new(FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
        Arc::clone(&client),
        &config.database,
        &db_config_dir(&config),
    )?)));

    let filter_pool: Option<FilterPool> = Some(Arc::new(std::sync::Mutex::new(BTreeMap::new())));
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache_limit: FeeHistoryCacheLimit = 2048;

    Ok(sc_service::PartialComponents {
        client: client.clone(),
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (client, telemetry, frontier_backend, FrontierPartialComponents {
            filter_pool,
            fee_history_cache,
            fee_history_cache_limit,
        }),
    })
}

pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let miner_base_path = config.base_path.path().to_path_buf();
    
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (_, mut telemetry, frontier_backend, frontier_partial),
    } = new_partial(&config)?;

    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = frontier_partial;

    // Initialize GHOSTDAG with genesis
    let ghostdag_store = GhostdagStore::new(client.clone());
    let genesis_hash = client.info().genesis_hash;
    let genesis_h256 = H256::from_slice(genesis_hash.as_ref());
    
    if ghostdag_store.get_ghostdag_data(&genesis_h256).is_none() {
        let _ = ghostdag_store.init_genesis(genesis_h256);
        log::info!("🔷 GHOSTDAG genesis initialized: {:?}", genesis_h256);
    }

    let net_config = sc_network::config::FullNetworkConfiguration::<
        Block,
        <Block as BlockT>::Hash,
        sc_network::NetworkWorker<Block, <Block as BlockT>::Hash>,
    >::new(&config.network, config.prometheus_registry().cloned());

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

    let role = config.role;
    let force_authoring = config.force_authoring;
    let prometheus_registry = config.prometheus_registry().cloned();

    log::info!("🔷 GHOSTDAG: K={}, target={}ms, initial_difficulty={}", 
        GHOSTDAG_K, TARGET_BLOCK_TIME_MS, INITIAL_DIFFICULTY);

    let pubsub_notification_sinks: Arc<EthereumBlockNotificationSinks<EthereumBlockNotification<Block>>> = Default::default();
    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));

    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        storage_override.clone(),
        50, 50,
        prometheus_registry.clone(),
    ));

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
                    3, 0u32.into(),
                    SyncStrategy::Normal,
                    sync_service.clone(),
                    pubsub_notification_sinks.clone(),
                ).for_each(|()| futures::future::ready(())),
            );
        }
        _ => {}
    }

    task_manager.spawn_essential_handle().spawn(
        "frontier-fee-history", None,
        EthTask::fee_history_task(client.clone(), storage_override.clone(), fee_history_cache.clone(), fee_history_cache_limit),
    );

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();
        let graph = transaction_pool.pool().clone();
        let network = network.clone();
        let sync = sync_service.clone();
        let frontier_backend = frontier_backend.clone();
        let storage_override = storage_override.clone();
        let block_data_cache = block_data_cache.clone();
        let filter_pool = filter_pool.clone();
        let fee_history_cache = fee_history_cache.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();
        let is_authority = config.role.is_authority();

        Box::new(move |subscription_task_executor| {
            let eth_deps = crate::rpc::EthDeps {
                client: client.clone(),
                pool: pool.clone(),
                graph: graph.clone(),
                converter: Some(lumenyx_runtime::TransactionConverter::<Block>::default()),
                is_authority,
                enable_dev_signer: false,
                network: network.clone(),
                sync: sync.clone(),
                frontier_backend: match &*frontier_backend {
                    fc_db::Backend::KeyValue(b) => b.clone(),
                    _ => unreachable!(),
                },
                storage_override: storage_override.clone(),
                block_data_cache: block_data_cache.clone(),
                filter_pool: filter_pool.clone(),
                max_past_logs: 10000,
                fee_history_cache: fee_history_cache.clone(),
                fee_history_cache_limit,
                execute_gas_limit_multiplier: 10,
                forced_parent_hashes: None,
                pending_create_inherent_data_providers: move |_, ()| async move {
                    Ok(sp_timestamp::InherentDataProvider::from_system_time())
                },
            };

            crate::rpc::create_full(
                crate::rpc::FullDeps { client: client.clone(), pool: pool.clone(), deny_unsafe: crate::rpc::DenyUnsafe::No, eth: eth_deps },
                subscription_task_executor,
                pubsub_notification_sinks.clone(),
            ).map_err(Into::into)
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
    // GHOSTDAG MINING
    // ============================================
    if role.is_authority() || force_authoring {
        use sp_consensus::{Environment, Proposer};
        use sc_basic_authorship::ProposerFactory;
        use sp_inherents::InherentDataProvider;

        let miner_pair = get_or_create_miner_key(&miner_base_path);
        let miner_address: [u8; 32] = miner_pair.public().0;
        log::info!("💰 Mining rewards to: {}", miner_pair.public().to_ss58check());
        log::info!("⛏️  Starting GHOSTDAG miner...");

        let mut proposer_factory = ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
        );

        let mining_client = client.clone();
        let mining_store = GhostdagStore::new(client.clone());
        let block_import = client.clone();
        let select_chain_mining = select_chain.clone();
        let mining_sync_service = sync_service.clone();
        let mining_tx_pool = transaction_pool.clone();

        task_manager.spawn_handle().spawn_blocking(
            "ghostdag-miner",
            Some("mining"),
            async move {
                let mut interval = tokio::time::interval(Duration::from_millis(TARGET_BLOCK_TIME_MS));
                let difficulty = INITIAL_DIFFICULTY;
                let target = difficulty_to_target(difficulty);
                let mut consecutive_propose_failures: u32 = 0;
                const MAX_FAILURES_BEFORE_POOL_CLEAR: u32 = 3;

                loop {
                    // KASPA-STYLE: Only pause if no peers (isolated node)
                    // GHOSTDAG handles convergence via blue_work - dont block based on block number
                    if mining_sync_service.num_connected_peers() == 0 {
                        log::debug!("⏸️  No peers - waiting for connection...");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    interval.tick().await;

                    // Get best block from GHOSTDAG select chain
                    let best_header = match select_chain_mining.best_chain().await {
                        Ok(h) => h,
                        Err(e) => {
                            log::warn!("Failed to get best chain: {:?}", e);
                            continue;
                        }
                    };
                    
                    let parent_hash = best_header.hash();
                    let parent_number = *best_header.number();

                    // Get all tips for multi-parent block
                    let tips = match select_chain_mining.leaves().await {
                        Ok(t) => t,
                        Err(_) => vec![parent_hash],
                    };

                    // Select parents: best + other tips (up to MAX_PARENTS)
                    let max_parents = 10usize;
                    let mut parents: Vec<H256> = vec![H256::from_slice(parent_hash.as_ref())];
                    for tip in tips.iter().take(max_parents - 1) {
                        let tip_h256 = H256::from_slice(tip.as_ref());
                        if tip_h256 != parents[0] && !parents.contains(&tip_h256) {
                            parents.push(tip_h256);
                        }
                    }

                    // Create inherent data
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let inherent_data = match timestamp.create_inherent_data().await {
                        Ok(d) => d,
                        Err(e) => {
                            log::warn!("Failed to create inherent data: {:?}", e);
                            continue;
                        }
                    };

                    // Create proposer
                    let proposer = match proposer_factory.init(&best_header).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("Failed to create proposer: {:?}", e);
                            continue;
                        }
                    };

                    // Build digest with miner address and extra parents
                    let miner_digest = MinerAddressDigest { miner: miner_address };
                    let mut digest_logs = vec![
                        DigestItem::PreRuntime(GHOSTDAG_ENGINE_ID, miner_digest.encode()),
                    ];
                    
                    // Add extra parents if any
                    if parents.len() > 1 {
                        let parents_digest = DagParentsDigest { parents: parents[1..].to_vec() };
                        digest_logs.push(DigestItem::Other(parents_digest.encode()));
                    }

                    let proposal = match proposer.propose(
                        inherent_data,
                        sp_runtime::generic::Digest { logs: digest_logs },
                        Duration::from_millis(500),
                        None,
                    ).await {
                        Ok(p) => p,
                        Err(e) => {
                            consecutive_propose_failures += 1;
                            log::warn!("⚠️ Failed to propose block ({}/{} failures): {:?}", consecutive_propose_failures, MAX_FAILURES_BEFORE_POOL_CLEAR, e);
                            if consecutive_propose_failures >= MAX_FAILURES_BEFORE_POOL_CLEAR {
                                log::warn!("🧹 Too many propose failures - clearing transaction pool to recover");
                                let pending: Vec<_> = mining_tx_pool.ready().map(|tx| tx.hash.clone()).collect();
                                for hash in pending {
                                    let _ = mining_tx_pool.remove_invalid(&[hash]);
                                }
                                consecutive_propose_failures = 0;
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                    };

                    let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                    let (header, body) = block.deconstruct();
                    let header_hash = header.hash();

                    // Mine: find valid nonce
                    let mut nonce = [0u8; 32];
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                    nonce[0..8].copy_from_slice(&seed.to_le_bytes());

                    let mut found = false;
                    for attempt in 0..1_000_000u64 {
                        for i in 0..32 {
                            if nonce[i] == 255 { nonce[i] = 0; }
                            else { nonce[i] += 1; break; }
                        }

                        let pow_hash = compute_pow_hash(header_hash.as_ref(), &nonce);

                        if hash_meets_target(&pow_hash, &target) {
                            // Process through GHOSTDAG before import
                            let block_h256 = H256::from_slice(header_hash.as_ref());
                            let parent_h256 = H256::from_slice(header.parent_hash().as_ref());
                            
                            // Calculate GHOSTDAG data
                            let extra_parents: Vec<H256> = if parents.len() > 1 {
                                parents[1..].to_vec()
                            } else {
                                vec![]
                            };
                            
                            let mut all_parents = vec![parent_h256];
                            all_parents.extend(extra_parents.iter().cloned());
                            
                            let selected_parent = all_parents.iter()
                                .filter_map(|p| mining_store.get_ghostdag_data(p).map(|d| (*p, d.blue_work)))
                                .max_by_key(|(_, w)| *w)
                                .map(|(h, _)| h)
                                .unwrap_or(parent_h256);
                            
                            let parent_data = mining_store.get_ghostdag_data(&selected_parent)
                                .unwrap_or_default();
                            
                            let mergeset_blues: Vec<H256> = all_parents.iter()
                                .filter(|p| **p != selected_parent)
                                .filter(|p| mining_store.get_ghostdag_data(p).is_some())
                                .cloned()
                                .collect();
                            
                            let blue_score = parent_data.blue_score + 1 + mergeset_blues.len() as u64;
                            let blue_work = parent_data.blue_work + 1 + mergeset_blues.len() as u128;
                            
                            let ghostdag_data = GhostdagData {
                                blue_score,
                                blue_work,
                                selected_parent,
                                mergeset_blues,
                                mergeset_reds: vec![],
                                blues_anticone_sizes: vec![],
                            };
                            
                            // Store GHOSTDAG data
                            let _ = mining_store.store_ghostdag_data(&block_h256, &ghostdag_data);
                            let _ = mining_store.store_parents(&block_h256, &all_parents);
                            let _ = mining_store.update_tips(&block_h256, &all_parents);
                            for p in &all_parents {
                                let _ = mining_store.add_child(p, &block_h256);
                            }

                            // Import block with Custom fork choice
                            let mut import_params = BlockImportParams::new(BlockOrigin::Own, header.clone());
                            import_params.body = Some(body.clone());
                            import_params.state_action = sc_consensus::StateAction::ApplyChanges(
                                sc_consensus::StorageChanges::Changes(storage_changes)
                            );
                            import_params.fork_choice = Some(sc_consensus::ForkChoiceStrategy::Custom(true));

                            match block_import.import_block(import_params).await {
                                Ok(_) => {
                                    log::info!(
                                        "✅ Block #{} mined! hash={:?} blue_score={} blue_work={} parents={}",
                                        parent_number + 1,
                                        header_hash,
                                        blue_score,
                                        blue_work,
                                        all_parents.len()
                                    );
                                    consecutive_propose_failures = 0; // Reset on success
                                }
                                Err(e) => {
                                    log::error!("❌ Failed to import block: {:?}", e);
                                }
                            }

                            found = true;
                            break;
                        }
                    }

                    if !found {
                        log::debug!("⏳ No valid nonce found for block #{}", parent_number + 1);
                    }
                }
            },
        );

        log::info!("🚀 GHOSTDAG miner started!");
    } else {
        log::info!("📡 Sync-only mode (not mining)");
    }

    log::info!("✅ LUMENYX GHOSTDAG running!");
    network_starter.start_network();
    Ok(task_manager)
}
