//! LUMENYX Runtime
//!
//! Decentralized blockchain:
//! - Fixed supply (21M)
//! - 2.5 second blocks (PoW LongestChain)
//! - Smart contracts (EVM compatible)
//! - True decentralization (fair launch, no governance)

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Encode, Decode};
extern crate alloc;
use alloc::vec::Vec;

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, IdentifyAccount,
             NumberFor, PostDispatchInfoOf, Verify, OpaqueKeys},
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult, MultiSignature, Permill,
};
use sp_std::prelude::*;
use sp_version::RuntimeVersion;

use frame_support::{
    construct_runtime, derive_impl,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{ConstU32, ConstU64, ConstU8, ConstU128, FindAuthor, OnFinalize, ConstBool},
    weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_transaction_payment::FungibleAdapter;

// ============================================
// MINER ADDRESS DIGEST FOR PoW
// ============================================

/// Engine ID for LUMENYX PoW consensus
pub const LUMENYX_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"LMNX";

/// Digest to identify block miner for reward distribution
#[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo)]
pub struct MinerAddressDigest {
    pub miner: [u8; 32],
}

impl MinerAddressDigest {
    pub fn new(miner: [u8; 32]) -> Self {
        Self { miner }
    }
}

// ============================================
// FRONTIER EVM IMPORTS
// ============================================
use fp_rpc::TransactionStatus;
use pallet_ethereum::{Call::transact, PostLogContent, Transaction as EthereumTransaction, TransactionAction, TransactionData};
use pallet_evm::{Precompile,
    Account as EVMAccount, EnsureAddressTruncated, FeeCalculator, HashedAddressMapping, Runner,
};

// Import our primitives
pub use lumenyx_primitives::{BLOCK_TIME_MS, BLOCKS_PER_DAY, BLOCKS_PER_YEAR};

pub type BlockNumber = u32;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Nonce = u32;
pub type Hash = sp_core::H256;

/// Block time: 2.5 seconds (2500ms)
pub const MILLISECS_PER_BLOCK: u64 = 2500;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

/// Minimum balance to keep account alive
pub const EXISTENTIAL_DEPOSIT: Balance = 500;

/// EVM Chain ID - unique identifier for LUMENYX
pub const EVM_CHAIN_ID: u64 = 7777;

#[sp_version::runtime_version]
pub const RUNTIME_VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: sp_runtime::create_runtime_str!("lumenyx"),
    impl_name: sp_runtime::create_runtime_str!("lumenyx-node"),
    authoring_version: 1,
    spec_version: 306,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 2,
    state_version: 1,
};

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = RUNTIME_VERSION;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(
            Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND * 12 / 10, u64::MAX)
        );
    pub BlockLength: frame_system::limits::BlockLength =
        frame_system::limits::BlockLength::max_with_normal_ratio(
            5 * 1024 * 1024, sp_runtime::Perbill::from_percent(75)
        );
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    type AccountId = AccountId;
    type RuntimeCall = RuntimeCall;
    type Lookup = sp_runtime::traits::AccountIdLookup<AccountId, ()>;
    type Nonce = Nonce;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = Version;
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1_000_000;
    pub FeeMultiplier: sp_runtime::FixedU128 = sp_runtime::FixedU128::from_u32(1);
}

pub struct ToAuthor;
impl frame_support::traits::OnUnbalanced<frame_support::traits::fungible::Credit<AccountId, Balances>> for ToAuthor {
    fn on_nonzero_unbalanced(credit: frame_support::traits::fungible::Credit<AccountId, Balances>) {
        use frame_support::traits::fungible::Balanced;
        if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
            let _ = <Balances as Balanced<AccountId>>::resolve(&author, credit);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ToAuthor>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = frame_support::weights::ConstantMultiplier<Balance, ConstU128<1>>;
    type LengthToFee = frame_support::weights::ConstantMultiplier<Balance, ConstU128<1>>;
    type FeeMultiplierUpdate = ();
}

/// Find block author for PoW
pub struct PowAuthorFinder;
impl FindAuthor<AccountId> for PowAuthorFinder {
    fn find_author<'a, I>(digests: I) -> Option<AccountId>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        use codec::Decode;
        // Look for miner address in digest
        for (id, data) in digests {
            if id == LUMENYX_ENGINE_ID {
                if let Ok(miner) = MinerAddressDigest::decode(&mut &data[..]) {
                    return Some(AccountId::from(miner.miner));
                }
            }
        }
        None
    }
}

/// Handler that issues block rewards for PoW miners
pub struct BlockRewardHandler;
impl pallet_authorship::EventHandler<AccountId, BlockNumber> for BlockRewardHandler {
    fn note_author(author: AccountId) {
        let _ = Halving::issue_block_reward(&author);
    }
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = PowAuthorFinder;
    type EventHandler = BlockRewardHandler;
}

// ============================================
// LUMENYX CUSTOM PALLETS
// ============================================

impl pallet_halving::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
}

// ============================================
// DIFFICULTY ADJUSTMENT PALLET
// ============================================

impl pallet_difficulty::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

// ============================================
// EVM CONFIGURATION - ETHEREUM COMPATIBILITY
// ============================================

pub const GAS_PER_SECOND: u64 = 40_000_000;

pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND * 12 / 10,
    u64::MAX,
);

parameter_types! {
    pub BlockGasLimit: U256 = U256::from(
        MAXIMUM_BLOCK_WEIGHT.ref_time() / GAS_PER_SECOND * 1000
    );
    pub PrecompilesValue: LumenyxPrecompiles<Runtime> = LumenyxPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(
        WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND,
        0,
    );
    pub ChainId: u64 = EVM_CHAIN_ID;
}

pub struct LumenyxPrecompiles<R>(sp_std::marker::PhantomData<R>);

impl<R> Default for LumenyxPrecompiles<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> LumenyxPrecompiles<R> {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn used_addresses() -> [H160; 9] {
        [
            hash(1), hash(2), hash(3), hash(4), hash(5),
            hash(6), hash(7), hash(8), hash(9),
        ]
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}

impl<R> pallet_evm::PrecompileSet for LumenyxPrecompiles<R>
where
    R: pallet_evm::Config,
{
    fn execute(&self, handle: &mut impl pallet_evm::PrecompileHandle) -> Option<pallet_evm::PrecompileResult> {
        let address = handle.code_address();
        if address == hash(1) {
            Some(<pallet_evm_precompile_simple::ECRecover as Precompile>::execute(handle))
        } else if address == hash(2) {
            Some(<pallet_evm_precompile_simple::Sha256 as Precompile>::execute(handle))
        } else if address == hash(3) {
            Some(<pallet_evm_precompile_simple::Ripemd160 as Precompile>::execute(handle))
        } else if address == hash(4) {
            Some(<pallet_evm_precompile_simple::Identity as Precompile>::execute(handle))
        } else if address == hash(5) {
            Some(<pallet_evm_precompile_modexp::Modexp as Precompile>::execute(handle))
        } else if address == hash(6) {
            Some(<pallet_evm_precompile_bn128::Bn128Add as Precompile>::execute(handle))
        } else if address == hash(7) {
            Some(<pallet_evm_precompile_bn128::Bn128Mul as Precompile>::execute(handle))
        } else if address == hash(8) {
            Some(<pallet_evm_precompile_bn128::Bn128Pairing as Precompile>::execute(handle))
        } else if address == hash(9) {
            Some(<pallet_evm_precompile_blake2::Blake2F as Precompile>::execute(handle))
        } else {
            None
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> pallet_evm::IsPrecompileResult {
        pallet_evm::IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

pub struct FixedGasPrice;
impl FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        // Min gas price: 45 planck/gas (as per Master document)
        (U256::from(45u64), Weight::zero())
    }
}

pub struct FindAuthorTruncated<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<AccountId>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        F::find_author(digests).map(|author| {
            let bytes: [u8; 32] = author.into();
            H160::from_slice(&bytes[0..20])
        })
    }
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressTruncated;
    type WithdrawOrigin = EnsureAddressTruncated;
    type AddressMapping = HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = LumenyxPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = ();
    type OnCreate = ();
    type FindAuthor = FindAuthorTruncated<PowAuthorFinder>;
    type GasLimitPovSizeRatio = ConstU64<4>;
    type GasLimitStorageGrowthRatio = ConstU64<366>;
    type Timestamp = Timestamp;
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type WeightInfo = ();
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self::Version>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
    pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000u64);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
    fn lower() -> Permill { Permill::zero() }
    fn ideal() -> Permill { Permill::from_parts(500_000) }
    fn upper() -> Permill { Permill::from_parts(1_000_000) }
}

impl pallet_base_fee::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Threshold = BaseFeeThreshold;
    type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    type DefaultElasticity = DefaultElasticity;
}

// ============================================
// RUNTIME CONSTRUCTION
// ============================================

pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic = fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
pub type Executive = frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Authorship: pallet_authorship,
        Halving: pallet_halving,
        Difficulty: pallet_difficulty,
        EVM: pallet_evm,
        Ethereum: pallet_ethereum,
        BaseFee: pallet_base_fee,
    }
);

// ============================================
// ETHEREUM SELF-CONTAINED TRANSACTION SUPPORT
// ============================================

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
                Some(call.dispatch(RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info))))
            }
            _ => None,
        }
    }
}

// ============================================
// RUNTIME API IMPLEMENTATIONS
// ============================================

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            RUNTIME_VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
            Vec::new()
        }

        fn decode_session_keys(_encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            EVM_CHAIN_ID
        }

        fn account_basic(address: H160) -> EVMAccount {
            let (account, _) = pallet_evm::Pallet::<Runtime>::account_basic(&address);
            account
        }

        fn gas_price() -> U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            pallet_evm::AccountCodes::<Runtime>::get(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: U256) -> H256 {
            let mut tmp = [0u8; 32];
            index.to_big_endian(&mut tmp);
            pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let gas_limit = gas_limit.min(U256::from(u64::MAX));
            let transaction_data = TransactionData::new(
                TransactionAction::Call(to),
                data.clone(),
                nonce.unwrap_or_default(),
                gas_limit,
                None,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                value,
                Some(EVM_CHAIN_ID),
                access_list.clone().unwrap_or_default(),
            );
            let (weight_limit, proof_size_base_cost) = pallet_ethereum::Pallet::<Runtime>::transaction_weight(&transaction_data);

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit.low_u64(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                false,
                true,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: U256,
            gas_limit: U256,
            max_fee_per_gas: Option<U256>,
            max_priority_fee_per_gas: Option<U256>,
            nonce: Option<U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };

            let transaction_data = TransactionData::new(
                TransactionAction::Create,
                data.clone(),
                nonce.unwrap_or_default(),
                gas_limit,
                None,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                value,
                Some(EVM_CHAIN_ID),
                access_list.clone().unwrap_or_default(),
            );
            let (weight_limit, proof_size_base_cost) = pallet_ethereum::Pallet::<Runtime>::transaction_weight(&transaction_data);

            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit.low_u64(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                false,
                true,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
            pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            pallet_ethereum::CurrentBlock::<Runtime>::get()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            pallet_ethereum::CurrentReceipts::<Runtime>::get()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<TransactionStatus>>,
        ) {
            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentReceipts::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get(),
            )
        }

        fn extrinsic_filter(xts: Vec<<Block as BlockT>::Extrinsic>) -> Vec<EthereumTransaction> {
            xts.into_iter()
                .filter_map(|xt| match xt.0.function {
                    RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
                    _ => None,
                })
                .collect::<Vec<EthereumTransaction>>()
        }

        fn elasticity() -> Option<Permill> {
            Some(pallet_base_fee::Elasticity::<Runtime>::get())
        }

        fn gas_limit_multiplier_support() {}

        fn pending_block(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> (Option<pallet_ethereum::Block>, Option<Vec<TransactionStatus>>) {
            for ext in xts.into_iter() {
                let _ = Executive::apply_extrinsic(ext);
            }
            Ethereum::on_finalize(System::block_number() + 1);
            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get(),
            )
        }

        fn initialize_pending_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header);
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_unsigned(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }
}

// ============================================
// OPAQUE TYPES
// ============================================

pub mod opaque {
    use super::*;
    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;
}

// ============================================
// TRANSACTION CONVERTER FOR FRONTIER
// ============================================

#[derive(Clone)]
pub struct TransactionConverter<B>(sp_std::marker::PhantomData<B>);

impl<B> Default for TransactionConverter<B> {
    fn default() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}

impl<B: sp_runtime::traits::Block> fp_rpc::ConvertTransaction<<B as sp_runtime::traits::Block>::Extrinsic> for TransactionConverter<B> {
    fn convert_transaction(
        &self,
        transaction: pallet_ethereum::Transaction,
    ) -> <B as sp_runtime::traits::Block>::Extrinsic {
        let extrinsic = UncheckedExtrinsic::new_unsigned(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        );
        let encoded = extrinsic.encode();
        <B as sp_runtime::traits::Block>::Extrinsic::decode(&mut &encoded[..])
            .expect("Encoded extrinsic is always valid")
    }
}

pub const VERSION: RuntimeVersion = RUNTIME_VERSION;
