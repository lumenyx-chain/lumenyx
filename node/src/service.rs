//! LUMENYX Service Configuration
//!
//! Sets up the full node with both Substrate and Ethereum RPC support.
//! This enables MetaMask and all Ethereum-compatible tools.
//!
//! NO GRANDPA = UNSTOPPABLE like Bitcoin!
//! Finality is probabilistic (6 blocks = ~18 seconds)

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures::prelude::*;
use lumenyx_runtime::{self, opaque::Block, RuntimeApi};
use sc_client_api::BlockchainEvents;
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_executor::WasmExecutor;
use sc_service::{
    error::Error as ServiceError, Configuration, TaskManager, TFullBackend, TFullClient,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;

// Frontier imports
use fc_mapping_sync::{EthereumBlockNotification, EthereumBlockNotificationSinks, SyncStrategy, kv::MappingSyncWorker};
use fc_rpc::EthTask;
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fc_storage::StorageOverrideHandler;

/// Database directory for Frontier
pub fn db_config_dir(config: &Configuration) -> std::path::PathBuf {
    config.base_path.config_dir(config.chain_spec.id())
}

pub type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<sp_io::SubstrateHostFunctions>>;
type FullBackend = TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

// NO GRANDPA types needed!

/// Frontier backend type
pub type FrontierBackend = fc_db::Backend<Block, FullClient>;

/// Extra data for Frontier
pub struct FrontierPartialComponents {
    pub filter_pool: Option<FilterPool>,
    pub fee_history_cache: FeeHistoryCache,
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
}

/// Create partial components for the LUMENYX node
pub fn new_partial(
    config: &Configuration,
) -> Result
    sc_service::PartialComponents
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            Option<Telemetry>,
            Arc<FrontierBackend>,
            FrontierPartialComponents,
        ),
    >,
    ServiceError,
> {
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

    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    // Direct client as block import - no wrapper needed for AURA-only
    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(ImportQueueParams {
        block_import: client.clone(),
        justification_import: None,  // NO GRANDPA justifications!
        client: client.clone(),
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
            Ok((slot, timestamp))
        },
        spawner: &task_manager.spawn_essential_handle(),
        registry: config.prometheus_registry(),
        check_for_equivocation: Default::default(),
        telemetry: telemetry.as_ref().map(|x| x.handle()),
        compatibility_mode: Default::default(),
    })?;

    // ============================================
    // FRONTIER BACKEND SETUP
    // ============================================
    let frontier_backend = Arc::new(FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
        Arc::clone(&client),
        &config.database,
        &db_config_dir(&config),
    )?)));

    // Frontier filter pool
    let filter_pool: Option<FilterPool> = Some(Arc::new(std::sync::Mutex::new(BTreeMap::new())));

    // Fee history cache
    let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let fee_history_cache_limit: FeeHistoryCacheLimit = 2048;

    let frontier_partial = FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (telemetry, frontier_backend, frontier_partial),
    })
}

/// Build the full LUMENYX node with Ethereum RPC support
/// NO GRANDPA = Network NEVER stops!
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (mut telemetry, frontier_backend, frontier_partial),
    } = new_partial(&config)?;

    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = frontier_partial;

    let net_config = sc_network::config::FullNetworkConfiguration::
        Block,
        <Block as sp_runtime::traits::Block>::Hash,
        sc_network::NetworkWorker<Block, <Block as sp_runtime::traits::Block>::Hash>,
    >::new(&config.network, config.prometheus_registry().cloned());

    let metrics = sc_network::NotificationMetrics::new(config.prometheus_registry());

    // NO GRANDPA protocol needed!

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,  // NO warp sync without GRANDPA
            block_relay: None,
            metrics,
        })?;

    let role = config.role;
    let force_authoring = config.force_authoring;
    let prometheus_registry = config.prometheus_registry().cloned();

    // ============================================
    // FRONTIER BACKGROUND TASKS
    // ============================================

    // Ethereum block notification sinks for pub/sub
    let pubsub_notification_sinks: Arc<EthereumBlockNotificationSinks<EthereumBlockNotification<Block>>> =
        Default::default();

    // Storage override for EVM queries
    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));

    // Block data cache for efficient RPC
    let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
        task_manager.spawn_handle(),
        storage_override.clone(),
        50,
        50,
        prometheus_registry.clone(),
    ));

    // Spawn Frontier mapping sync worker
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
        _ => {}
    }

    // Spawn EthTask for fee history
    let fee_history_task = EthTask::fee_history_task(
        client.clone(),
        storage_override.clone(),
        fee_history_cache.clone(),
        fee_history_cache_limit,
    );
    task_manager.spawn_essential_handle().spawn("frontier-fee-history", None, fee_history_task);

    // ============================================
    // RPC EXTENSIONS WITH ETHEREUM SUPPORT
    // ============================================
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
                    _ => panic!("SQL backend not supported"),
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
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    Ok(timestamp)
                },
            };

            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe: crate::rpc::DenyUnsafe::No,
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
        backend,
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
    })?;

    // Start block authoring if validator
    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import: client.clone(),
                proposer_factory,
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        slot_duration,
                    );
                    Ok((slot, timestamp))
                },
                force_authoring,
                backoff_authoring_blocks: None::<()>,
                keystore: keystore_container.keystore(),
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?;

        task_manager.spawn_essential_handle().spawn_blocking("aura", Some("block-authoring"), aura);

        log::info!("🚀 LUMENYX Validator started - UNSTOPPABLE like Bitcoin!");
    }

    // NO GRANDPA voter! Network never stops.
    log::info!("✅ LUMENYX Node running WITHOUT GRANDPA - probabilistic finality (6 blocks = 18 sec)");

    network_starter.start_network();
    Ok(task_manager)
}
