//! LUMENYX Service Configuration - GHOSTDAG PoW

use std::{collections::BTreeMap, sync::Arc, time::Duration};
use std::path::PathBuf;
use std::fs;
use sp_core::{sr25519, Pair, crypto::Ss58Codec};
use sp_runtime::generic::DigestItem;
use codec::Encode;

use futures::prelude::*;
use lumenyx_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::{BlockBackend, BlockchainEvents, AuxStore, HeaderBackend};
use sc_consensus::{BlockImportParams, import_queue::Verifier, BlockImport};
use sc_executor::WasmExecutor;
use sc_service::{
    error::Error as ServiceError, Configuration, TaskManager, TFullBackend, TFullClient,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus::{BlockOrigin, Proposer};
use sp_core::H256;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_api::ProvideRuntimeApi;

// GHOSTDAG imports
use sc_consensus_ghostdag::GhostdagStore;

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
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
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

/// Load or generate miner keypair
fn get_or_create_miner_key(base_path: &std::path::Path) -> sr25519::Pair {
    let key_file = base_path.join("miner-key");
    
    if key_file.exists() {
        // Load existing key
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
    
    // Generate new key
    let (pair, phrase, seed) = sr25519::Pair::generate_with_phrase(None);
    let seed_hex = hex::encode(seed);
    
    // Save seed to file
    if let Err(e) = fs::write(&key_file, &seed_hex) {
        log::warn!("Failed to save miner key: {:?}", e);
    }
    
    // Log the important info
    log::info!("==========================================");
    log::info!("🔑 NEW MINER WALLET GENERATED!");
    log::info!("==========================================");
    log::info!("📝 Seed phrase: {}", phrase);
    log::info!("📫 Address: {}", pair.public().to_ss58check());
    log::info!("==========================================");
    log::info!("⚠️  SAVE YOUR SEED PHRASE! This is the ONLY way to recover your funds!");
    log::info!("==========================================");
    
    pair
}

pub struct GhostdagVerifier;

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

#[async_trait::async_trait]
impl Verifier<Block> for GhostdagVerifier {
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

    let import_queue = sc_consensus::BasicQueue::new(
        GhostdagVerifier,
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
    // Save base path before config is moved
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

    let ghostdag_store = GhostdagStore::new(client.clone());
    let genesis_hash = client.info().genesis_hash;
    let _ = ghostdag_store.init_genesis(H256::from_slice(genesis_hash.as_ref()));

    log::info!("🔷 GHOSTDAG: K={}, target={}ms, difficulty={}", GHOSTDAG_K, TARGET_BLOCK_TIME_MS, INITIAL_DIFFICULTY);

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
    // GHOSTDAG MINING WITH REAL BLOCK PRODUCTION
    // ============================================
    if role.is_authority() || force_authoring {
        use sp_consensus::Environment;
        use sc_basic_authorship::ProposerFactory;
        use sp_inherents::InherentDataProvider;
        

        // Load or create miner wallet
        let miner_key_path = miner_base_path.clone();
        let miner_pair = get_or_create_miner_key(&miner_key_path);
        let miner_address: [u8; 32] = miner_pair.public().0;
        log::info!("💰 Mining rewards will go to: {}", miner_pair.public().to_ss58check());
        log::info!("⛏️  Starting GHOSTDAG block production...");
        
        let mut proposer_factory = ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
        );
        
        let mining_client = client.clone();
        let block_import = client.clone();
        
        task_manager.spawn_handle().spawn_blocking(
            "ghostdag-miner",
            Some("mining"),
            async move {
                let mut interval = tokio::time::interval(Duration::from_millis(TARGET_BLOCK_TIME_MS));
                let difficulty = INITIAL_DIFFICULTY;
                let target = difficulty_to_target(difficulty);
                
                loop {
                    interval.tick().await;
                    
                    let info = mining_client.info();
                    let parent_hash = info.best_hash;
                    let parent_number = info.best_number;
                    
                    // Get parent header
                    let parent_header = match mining_client.header(parent_hash) {
                        Ok(Some(h)) => h,
                        _ => continue,
                    };
                    
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
                    let proposer = match proposer_factory.init(&parent_header).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("Failed to create proposer: {:?}", e);
                            continue;
                        }
                    };
                    
                    // Create block proposal
                    let proposal = match proposer.propose(
                        inherent_data,
                        {
                        // Create digest with miner address
                        let miner_digest = MinerAddressDigest { miner: miner_address };
                        let pre_runtime = DigestItem::PreRuntime(GHOSTDAG_ENGINE_ID, miner_digest.encode());
                        sp_runtime::generic::Digest { logs: vec![pre_runtime] }
                    },
                        Duration::from_millis(500),
                        None,
                    ).await {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!("Failed to propose block: {:?}", e);
                            continue;
                        }
                    };
                    
                    let (block, storage_changes) = (proposal.block, proposal.storage_changes);
                    let (header, body) = block.deconstruct();
                    
                    // Mine: find valid nonce for this block
                    let header_hash = header.hash();
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
                            // Import the block
                            let mut import_params = BlockImportParams::new(BlockOrigin::Own, header.clone());
                            import_params.body = Some(body.clone());
                            import_params.state_action = sc_consensus::StateAction::ApplyChanges(
                                sc_consensus::StorageChanges::Changes(storage_changes)
                            );
                            import_params.fork_choice = Some(sc_consensus::ForkChoiceStrategy::LongestChain);
                            
                            match block_import.import_block(import_params).await {
                                Ok(result) => {
                                    log::info!(
                                        "✅ Block #{} imported! hash={:?}, pow={:?}, attempts={}",
                                        parent_number + 1,
                                        header_hash,
                                        pow_hash,
                                        attempt + 1
                                    );
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
        log::info!("📡 Sync-only mode");
    }

    log::info!("✅ LUMENYX GHOSTDAG PoW running!");
    network_starter.start_network();
    Ok(task_manager)
}
