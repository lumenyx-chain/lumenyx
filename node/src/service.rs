//! LUMENYX Service Configuration - PoW with Dynamic Difficulty
//!
//! PoW consensus with on-chain difficulty adjustment.

use std::{collections::BTreeMap, sync::Arc, time::Duration};
use std::fs;
use sp_core::{sr25519, Pair, crypto::Ss58Codec, H256, U256, storage::StorageKey};
use sp_runtime::generic::DigestItem;
use codec::{Encode, Decode};

use futures::StreamExt;
use lumenyx_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{BlockBackend, BlockchainEvents, AuxStore, HeaderBackend, StorageProvider};
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

// RX-LX imports
use rx_lx::{Flags, Cache, Vm};

use crate::rx_lx::seed as seed_sched;

// Frontier imports
use fc_mapping_sync::{EthereumBlockNotificationSinks, EthereumBlockNotification, SyncStrategy, kv::MappingSyncWorker};
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

// Fallback difficulty if we can't read from runtime
const FALLBACK_DIFFICULTY: u128 = 1_000_000;

/// LUMENYX Engine ID for digests
const LUMENYX_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"LMNX";

/// Miner address digest structure
#[derive(Clone, codec::Encode, codec::Decode)]
struct MinerAddressDigest {
    miner: [u8; 32],
}

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
        Ok(Some(data)) => {
            // Decode u128 from storage
            match u128::decode(&mut &data.0[..]) {
                Ok(diff) => {
                    log::debug!("📊 Read difficulty from storage: {}", diff);
                    diff
                }
                Err(e) => {
                    log::warn!("Failed to decode difficulty: {:?}, using fallback", e);
                    FALLBACK_DIFFICULTY
                }
            }
        }
        Ok(None) => {
            log::debug!("No difficulty in storage yet, using fallback");
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
        
        log::info!("🔧 Initializing RX-LX cache with seed from block #{}", seed_height);
        cache.init(seed.as_ref());
        
        let vm = Vm::light(flags, &cache).map_err(|e| format!("VM creation failed: {:?}", e))?;
        log::info!("✅ RX-LX initialized successfully");
        
        Ok(Self { flags, cache, vm, seed_height, seed })
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
        log::info!("🔄 RX-LX seed change: block #{} -> #{}", self.seed_height, new_seed_height);
        
        self.cache.init(new_seed.as_ref());
        self.vm = Vm::light(self.flags, &self.cache).map_err(|e| format!("VM recreation failed: {:?}", e))?;
        
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

// Legacy Blake3 function - kept for reference, no longer used
#[allow(dead_code)]
fn compute_pow_hash_blake3(data: &[u8], nonce: &[u8; 32]) -> H256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(nonce);
    H256::from_slice(&hasher.finalize().as_bytes()[..32])
}
// PoW constants
const MIN_DIFFICULTY: u128 = 1;
const MAX_DIFFICULTY: u128 = u128::MAX;
const POW_LIMIT: H256 = H256::repeat_byte(0xff); // 2^256 - 1

fn difficulty_to_target(difficulty: u128) -> H256 {
    // 1) Clamp difficulty in [MIN, MAX] e protezione da 0
    let mut d = difficulty;
    if d < MIN_DIFFICULTY { d = MIN_DIFFICULTY; }
    if d > MAX_DIFFICULTY { d = MAX_DIFFICULTY; }
    if d == 0 { d = MIN_DIFFICULTY; }

    // 2) pow_limit (H256) -> U256 (big-endian)
    let pow_u = U256::from_big_endian(POW_LIMIT.as_fixed_bytes());

    // 3) u128 difficulty -> U256 (zero-extend a 32 byte, big-endian)
    let mut d_be = [0u8; 32];
    d_be[16..].copy_from_slice(&d.to_be_bytes());
    let d_u = U256::from_big_endian(&d_be);

    // 4) target = pow_limit / difficulty
    let mut target_u = pow_u / d_u;

    // 5) Clamp target: minimo 1, massimo pow_limit
    if target_u == U256::from(0u64) {
        target_u = U256::from(1u64);
    }
    if target_u > pow_u {
        target_u = pow_u;
    }

    // 6) U256 -> H256 (big-endian)
    let mut target_be = [0u8; 32];
    target_u.to_big_endian(&mut target_be);
    H256::from_slice(&target_be)
}

fn hash_meets_target(hash: &H256, target: &H256) -> bool {
    // Confronto numerico U256 (big-endian) - più sicuro
    let hash_u = U256::from_big_endian(hash.as_fixed_bytes());
    let target_u = U256::from_big_endian(target.as_fixed_bytes());
    hash_u <= target_u
}

/// Simple verifier - accepts all valid blocks using LongestChain
pub struct SimpleVerifier;

#[async_trait::async_trait]
impl Verifier<Block> for SimpleVerifier {
    async fn verify(
        &self,
        mut block: BlockImportParams<Block>,
    ) -> Result<BlockImportParams<Block>, String> {
        block.fork_choice = Some(sc_consensus::ForkChoiceStrategy::LongestChain);
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

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let verifier = SimpleVerifier;

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
        other: (client, telemetry, frontier_backend, frontier_partial),
    })
}

pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (_, mut telemetry, frontier_backend, frontier_partial),
    } = new_partial(&config)?;

    let net_config = sc_network::config::FullNetworkConfiguration::<Block, <Block as BlockT>::Hash, sc_network::NetworkWorker<Block, <Block as BlockT>::Hash>>::new(&config.network, config.prometheus_registry().cloned());

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

    let pubsub_notification_sinks: Arc<EthereumBlockNotificationSinks<EthereumBlockNotification<Block>>> = Default::default();


    // Spawn Frontier EVM mapping sync worker
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
                ).for_each(|()| futures::future::ready(())),
            );
        }
    }
    // Spawn Frontier EVM mapping sync worker
    // Spawn Frontier fee history task
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
                frontier_backend: match &*frontier_backend { fc_db::Backend::KeyValue(b) => b.clone(), _ => unreachable!(), },
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
    // POW MINING WITH DYNAMIC DIFFICULTY
    // ============================================
    if role.is_authority() || force_authoring {
        use sp_consensus::{Environment, Proposer};
        use sc_basic_authorship::ProposerFactory;
        use sp_inherents::InherentDataProvider;

        let miner_pair = get_or_create_miner_key(&miner_base_path);
        let miner_address: [u8; 32] = miner_pair.public().0;
        log::info!("💰 Mining rewards to: {}", miner_pair.public().to_ss58check());
        log::info!("⛏️  Starting PoW miner with dynamic difficulty...");

        let mut proposer_factory = ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
        );

        let mining_client = client.clone();
        let block_import = client.clone();
        let select_chain_mining = select_chain.clone();
        let mining_sync_service = sync_service.clone();
        let mining_tx_pool = transaction_pool.clone();

        task_manager.spawn_handle().spawn_blocking(
            "pow-miner",
            Some("mining"),
            async move {
                let mut interval = tokio::time::interval(Duration::from_millis(100)); // Check more frequently
                let mut consecutive_propose_failures: u32 = 0;
                const MAX_FAILURES_BEFORE_POOL_CLEAR: u32 = 3;
                let mut last_difficulty: u128 = FALLBACK_DIFFICULTY;
                let mut blocks_since_difficulty_log: u32 = 0;
                
                // RX-LX: Initialize PoW engine
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
                
                loop {
                    if mining_sync_service.is_major_syncing() {
                        log::debug!("⏸️  Syncing in progress...");
                        tokio::time::sleep(Duration::from_millis(2000)).await;
                        continue;
                    }

                    interval.tick().await;

                    let best_header = match select_chain_mining.best_chain().await {
                        Ok(h) => h,
                        Err(e) => {
                            log::warn!("Failed to get best chain: {:?}", e);
                            continue;
                        }
                    };

                    let parent_hash = best_header.hash();
                    let parent_number = *best_header.number();

                    // Read difficulty from runtime storage
                    let difficulty = read_difficulty_from_storage(&*mining_client, parent_hash);
                    let target = difficulty_to_target(difficulty);
                    
                    // Log difficulty changes
                    if difficulty != last_difficulty {
                        log::info!("⚡ Difficulty changed: {} -> {}", last_difficulty, difficulty);
                        last_difficulty = difficulty;
                    }
                    
                    // Log difficulty periodically (every 100 blocks)
                    blocks_since_difficulty_log += 1;
                    if blocks_since_difficulty_log >= 100 {
                        log::info!("📊 Current difficulty: {}", difficulty);
                        blocks_since_difficulty_log = 0;
                    }

                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let inherent_data = match timestamp.create_inherent_data().await {
                        Ok(d) => d,
                        Err(e) => {
                            log::warn!("Failed to create inherent data: {:?}", e);
                            continue;
                        }
                    };

                    let proposer = match proposer_factory.init(&best_header).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("Failed to create proposer: {:?}", e);
                            continue;
                        }
                    };

                    let miner_digest = MinerAddressDigest { miner: miner_address };
                    let digest = sp_runtime::generic::Digest {
                        logs: vec![DigestItem::PreRuntime(LUMENYX_ENGINE_ID, miner_digest.encode())],
                    };

                    let proposal = match proposer.propose(
                        inherent_data,
                        digest,
                        Duration::from_millis(3000),
                        None,
                    ).await {
                        Ok(p) => p,
                        Err(e) => {
                            consecutive_propose_failures += 1;
                            log::warn!("⚠️ Failed to propose block ({}/{}): {:?}",
                                consecutive_propose_failures, MAX_FAILURES_BEFORE_POOL_CLEAR, e);
                            if consecutive_propose_failures >= MAX_FAILURES_BEFORE_POOL_CLEAR {
                                log::warn!("🧹 Clearing transaction pool");
                                let pending: Vec<_> = mining_tx_pool.ready().map(|tx| tx.hash.clone()).collect();
                                for hash in pending {
                                    let _ = mining_tx_pool.remove_invalid(&[hash]);
                                }
                                consecutive_propose_failures = 0;
                            }
                            continue;
                        }
                    };

                    let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                    let (header, body) = block.deconstruct();
                    let header_hash = header.hash();
                    
                    // RX-LX: Check if seed needs to change
                    if let Err(e) = rx_lx_pow.maybe_reseed(parent_number as u64 + 1, &get_block_hash) {
                        log::error!("❌ RX-LX reseed failed: {}", e);
                        continue;
                    }
                    
                    let mut nonce = [0u8; 32];
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64;
                    nonce[0..8].copy_from_slice(&seed.to_le_bytes());

                    let mut found = false;
                    // More iterations for higher difficulty
                    let max_iterations: u64 = 50_000_000;
                    
                    for _ in 0..max_iterations {
                        for i in 0..32 {
                            if nonce[i] == 255 { nonce[i] = 0; }
                            else { nonce[i] += 1; break; }
                        }

                        let pow_hash = rx_lx_pow.hash(&header_hash, &nonce);

                        if hash_meets_target(&pow_hash, &target) {
                            let mut import_params = BlockImportParams::new(BlockOrigin::Own, header.clone());
                            import_params.body = Some(body.clone());
                            import_params.state_action = sc_consensus::StateAction::ApplyChanges(
                                sc_consensus::StorageChanges::Changes(storage_changes)
                            );
                            import_params.fork_choice = Some(sc_consensus::ForkChoiceStrategy::LongestChain);

                            match block_import.import_block(import_params).await {
                                Ok(_) => {
                                    log::info!(
                                        "✅ Block #{} mined! hash={:?} difficulty={}",
                                        parent_number + 1,
                                        header_hash,
                                        difficulty
                                    );
                                    consecutive_propose_failures = 0;
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
                        log::debug!("⏳ No valid nonce found for block #{} (difficulty: {})", 
                            parent_number + 1, difficulty);
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
