//! LUMENYX RPC Extensions
//!
//! This module provides both Substrate and Ethereum RPC endpoints.
//! MetaMask and all Ethereum tools connect via eth_* methods.

use std::{collections::BTreeMap, sync::Arc};
use crate::pool_mode_handle::{SharedPoolMode, write_persisted_pool_mode};

use jsonrpsee::RpcModule;
use lumenyx_runtime::{AccountId, Balance, Hash, Nonce};
use sc_client_api::{
    backend::{Backend, StorageProvider},
    client::BlockchainEvents,
    AuxStore, UsageProvider,
};
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_blockchain::{Error as BlockchainError, HeaderBackend, HeaderMetadata};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::Block as BlockT;

// Frontier imports
use fc_rpc::{
    Eth, EthApiServer, EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net,
    NetApiServer, TxPool, TxPoolApiServer, Web3, Web3ApiServer,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fc_storage::StorageOverride;
use fp_rpc::{ConvertTransaction, ConvertTransactionRuntimeApi, EthereumRuntimeRPCApi};

pub use sc_rpc_api::DenyUnsafe;
pub type Block = lumenyx_runtime::opaque::Block;

/// Extra dependencies for Ethereum RPC
pub struct EthDeps<C, P, A: ChainApi, CT, CIDP> {
    /// The client instance to use
    pub client: Arc<C>,
    /// Transaction pool instance
    pub pool: Arc<P>,
    /// Graph pool instance
    pub graph: Arc<Pool<A>>,
    /// Converter for Ethereum transactions
    pub converter: Option<CT>,
    /// Whether to deny unsafe calls
    pub is_authority: bool,
    /// Whether to enable dev signer
    pub enable_dev_signer: bool,
    /// Network service
    pub network: Arc<dyn sc_network::service::traits::NetworkService>,
    /// Chain syncing service
    pub sync: Arc<SyncingService<Block>>,
    /// Frontier backend
    pub frontier_backend: Arc<dyn fc_api::Backend<Block>>,
    /// Storage override
    pub storage_override: Arc<dyn StorageOverride<Block>>,
    /// Block data cache
    pub block_data_cache: Arc<fc_rpc::EthBlockDataCacheTask<Block>>,
    /// Filter pool
    pub filter_pool: Option<FilterPool>,
    /// Maximum number of logs in a query
    pub max_past_logs: u32,
    /// Fee history cache
    pub fee_history_cache: FeeHistoryCache,
    /// Fee history cache limit
    pub fee_history_cache_limit: u64,
    /// Execute gas limit multiplier
    pub execute_gas_limit_multiplier: u64,
    /// Mandated parent hashes for a given block hash
    pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
    /// Pending inherent data providers
    pub pending_create_inherent_data_providers: CIDP,
}

/// Full dependencies for LUMENYX node RPC
pub struct FullDeps<C, P, A: ChainApi, CT, CIDP> {
    /// The client instance
    pub client: Arc<C>,
    /// Transaction pool instance
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// Pool mode handle for runtime toggle
    pub pool_mode: SharedPoolMode,
    /// Ethereum dependencies
    pub eth: EthDeps<C, P, A, CT, CIDP>,
}

/// Default Ethereum RPC configuration
pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for DefaultEthConfig<C, BE>
where
    C: StorageProvider<Block, BE> + Sync + Send + 'static,
    BE: Backend<Block> + 'static,
{
    type EstimateGasAdapter = ();
    type RuntimeStorageOverride =
        fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Create the full RPC extensions for LUMENYX
pub fn create_full<C, P, BE, A, CT, CIDP>(
    deps: FullDeps<C, P, A, CT, CIDP>,
    subscription_task_executor: SubscriptionTaskExecutor,
    pubsub_notification_sinks: Arc<
        fc_mapping_sync::EthereumBlockNotificationSinks<
            fc_mapping_sync::EthereumBlockNotification<Block>,
        >,
    >,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: sp_block_builder::BlockBuilder<Block>,
    C::Api: EthereumRuntimeRPCApi<Block>,
    C::Api: ConvertTransactionRuntimeApi<Block>,
    C: BlockchainEvents<Block> + 'static,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockchainError>,
    C: CallApiAt<Block> + AuxStore + UsageProvider<Block> + StorageProvider<Block, BE>,
    C: Send + Sync + 'static,
    BE: Backend<Block> + 'static,
    P: TransactionPool<Block = Block> + 'static,
    A: ChainApi<Block = Block> + 'static,
    CT: ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
    CIDP: CreateInherentDataProviders<Block, ()> + Send + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        pool_mode,
        eth,
    } = deps;

    // Substrate standard RPCs
    io.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    // Ethereum RPCs
    let EthDeps {
        client,
        pool,
        graph,
        converter,
        is_authority,
        enable_dev_signer,
        network,
        sync,
        frontier_backend,
        storage_override,
        block_data_cache,
        filter_pool,
        max_past_logs,
        fee_history_cache,
        fee_history_cache_limit,
        execute_gas_limit_multiplier,
        forced_parent_hashes,
        pending_create_inherent_data_providers,
    } = eth;

    // Signature methods disabled (enable_dev_signer = false for production)
    let signers: Vec<Box<dyn fc_rpc::EthSigner>> = vec![];

    // Eth RPC - Core Ethereum API
    io.merge(
        Eth::<_, _, _, _, _, _, _, DefaultEthConfig<C, BE>>::new(
            client.clone(),
            pool.clone(),
            graph.clone(),
            converter,
            sync.clone(),
            signers,
            storage_override.clone(),
            frontier_backend.clone(),
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_cache_limit,
            execute_gas_limit_multiplier,
            forced_parent_hashes,
            pending_create_inherent_data_providers,
            None, // pending_consensus_data_provider
        )
        .replace_config::<DefaultEthConfig<C, BE>>()
        .into_rpc(),
    )?;

    // EthFilter RPC - eth_newFilter, eth_getLogs, etc.
    if let Some(filter_pool) = filter_pool {
        io.merge(
            EthFilter::new(
                client.clone(),
                frontier_backend.clone(),
                graph.clone(),
                filter_pool,
                500_usize, // max stored filters
                max_past_logs,
                block_data_cache.clone(),
            )
            .into_rpc(),
        )?;
    }

    // EthPubSub RPC - WebSocket subscriptions
    io.merge(
        EthPubSub::new(
            pool,
            client.clone(),
            sync,
            subscription_task_executor,
            storage_override,
            pubsub_notification_sinks,
        )
        .into_rpc(),
    )?;

    // Net RPC - net_version, net_listening, net_peerCount
    io.merge(
        Net::new(
            client.clone(),
            network,
            true, // enable peer count
        )
        .into_rpc(),
    )?;

    // Web3 RPC - web3_clientVersion, web3_sha3
    io.merge(Web3::new(client.clone()).into_rpc())?;

    // TxPool RPC - txpool_status, txpool_inspect, txpool_content
    io.merge(TxPool::new(client, graph).into_rpc())?;


    // LUMENYX Pool Mode RPC - lumenyx_setPoolMode, lumenyx_getPoolMode
    let pool_mode_handle = pool_mode.clone();
    let pool_mode_handle2 = pool_mode.clone();
    io.register_method("lumenyx_getPoolMode", move |_, _, _| {
        Ok::<bool, jsonrpsee::types::ErrorObjectOwned>(pool_mode_handle.get())
    })?;
    io.register_method("lumenyx_setPoolMode", move |params, _, _| {
        let enabled: bool = params.one()?;
        pool_mode_handle2.set(enabled);
        if let Err(e) = write_persisted_pool_mode(enabled) {
            return Err(jsonrpsee::types::ErrorObjectOwned::owned(
                -32000,
                format!("failed to persist pool mode: {e}"),
                None::<()>,
            ));
        }
        Ok::<bool, jsonrpsee::types::ErrorObjectOwned>(pool_mode_handle2.get())
    })?;
    Ok(io)
}
