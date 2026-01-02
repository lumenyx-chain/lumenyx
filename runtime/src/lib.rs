//! LUMENYX Runtime
//!
//! The only blockchain with everything:
//! - Fixed supply (21M)
//! - BNB speed (3 second blocks)
//! - Privacy (ZK optional)
//! - Smart contracts (EVM compatible)
//! - True decentralization (fair launch, no governance)
//! - Permissionless validation (anyone can become validator!)
//! - Self-healing GRANDPA (auto-removes validators who don't sign)

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Encode, Decode};
extern crate alloc;
use alloc::vec::Vec;

use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
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
    traits::{ConstU32, ConstU64, ConstU8, FindAuthor, OnFinalize, ConstBool},
    weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_transaction_payment::FungibleAdapter;

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

/// Block time: 3 seconds (3000ms) - Fast like BNB!
pub const MILLISECS_PER_BLOCK: u64 = 3000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

/// Minimum balance to keep account alive
pub const EXISTENTIAL_DEPOSIT: Balance = 500;

/// EVM Chain ID - unique identifier for LUMENYX
/// Using 7777 as a memorable chain ID
pub const EVM_CHAIN_ID: u64 = 7777;

/// Number of sessions a validator can be inactive (not producing blocks) before removal
/// 50 sessions * 10 blocks/session * 3 sec/block = 25 minutes
pub const MAX_INACTIVE_SESSIONS: u32 = 50;

/// Maximum allowed gap between best block and finalized block
/// If gap exceeds this, newest validators get removed one by one
/// 100 blocks * 3 sec = 5 minutes of stuck GRANDPA triggers removal
pub const MAX_GRANDPA_LAG: u32 = 100;

/// Sessions to wait before removing a new validator for GRANDPA issues
/// Give them time to sync and start signing
/// 20 sessions * 10 blocks * 3 sec = 10 minutes grace period
pub const GRANDPA_GRACE_SESSIONS: u32 = 20;

#[sp_version::runtime_version]
pub const RUNTIME_VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: sp_runtime::create_runtime_str!("lumenyx"),
    impl_name: sp_runtime::create_runtime_str!("lumenyx-node"),
    authoring_version: 1,
    spec_version: 304,  // Updated version
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
            Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND * 2, u64::MAX)
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

// ============================================
// SESSION KEYS - Define before pallets that use them
// ============================================
impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
        pub grandpa: Grandpa,
    }
}

// ============================================
// SESSION CONFIGURATION - Permissionless Validators!
// ============================================
parameter_types! {
    pub const SessionPeriod: BlockNumber = 10;
    pub const SessionOffset: BlockNumber = 0;
}

// ============================================
// VALIDATOR ACTIVITY TRACKING
// ============================================
use frame_support::storage::StorageMap;
use frame_support::traits::StorageInstance;

/// Storage prefix for validator last active session (block production)
pub struct ValidatorLastActivePrefix;
impl StorageInstance for ValidatorLastActivePrefix {
    fn pallet_prefix() -> &'static str { "Session" }
    const STORAGE_PREFIX: &'static str = "ValidatorLastActive";
}

/// Storage prefix for tracking when each validator joined
pub struct ValidatorJoinedAtPrefix;
impl StorageInstance for ValidatorJoinedAtPrefix {
    fn pallet_prefix() -> &'static str { "Session" }
    const STORAGE_PREFIX: &'static str = "ValidatorJoinedAt";
}

/// Storage prefix for tracking last finalized block seen
pub struct LastFinalizedBlockPrefix;
impl StorageInstance for LastFinalizedBlockPrefix {
    fn pallet_prefix() -> &'static str { "Session" }
    const STORAGE_PREFIX: &'static str = "LastFinalizedBlock";
}

/// Storage prefix for tracking sessions with GRANDPA stuck
pub struct GrandpaStuckSessionsPrefix;
impl StorageInstance for GrandpaStuckSessionsPrefix {
    fn pallet_prefix() -> &'static str { "Session" }
    const STORAGE_PREFIX: &'static str = "GrandpaStuckSessions";
}

/// Maps validator AccountId to the last session they produced a block
pub type ValidatorLastActive = frame_support::storage::types::StorageMap<
    ValidatorLastActivePrefix,
    frame_support::Blake2_128Concat,
    AccountId,
    u32,
    frame_support::storage::types::OptionQuery,
>;

/// Maps validator AccountId to the session they joined
pub type ValidatorJoinedAt = frame_support::storage::types::StorageMap<
    ValidatorJoinedAtPrefix,
    frame_support::Blake2_128Concat,
    AccountId,
    u32,
    frame_support::storage::types::OptionQuery,
>;

/// Stores the last finalized block number we observed
pub type LastFinalizedBlock = frame_support::storage::types::StorageValue<
    LastFinalizedBlockPrefix,
    u32,
    frame_support::storage::types::ValueQuery,
>;

/// Counts consecutive sessions where GRANDPA was stuck
pub type GrandpaStuckSessions = frame_support::storage::types::StorageValue<
    GrandpaStuckSessionsPrefix,
    u32,
    frame_support::storage::types::ValueQuery,
>;

/// Permissionless validator set with GRANDPA self-healing
/// 
/// Anyone can become a validator by calling session.setKeys()
/// Validators are automatically removed if:
/// 1. They don't produce blocks for MAX_INACTIVE_SESSIONS
/// 2. GRANDPA is stuck and they are the newest non-essential validator
/// 
/// The network always maintains at least 2 validators for stability.
pub struct PermissionlessValidatorSet;

impl pallet_session::SessionManager<AccountId> for PermissionlessValidatorSet {
    fn new_session(new_index: u32) -> Option<Vec<AccountId>> {
        log::info!(
            target: "runtime::session",
            "🔄 Session {} - Discovering active validators...",
            new_index
        );

        // Get current block author and mark them as active
        if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
            ValidatorLastActive::insert(&author, new_index);
            log::info!(
                target: "runtime::session",
                "✍️ Block author {:?} marked active in session {}",
                author,
                new_index
            );
        }

        // Check GRANDPA health - compare current block with finalized
        let current_block = frame_system::Pallet::<Runtime>::block_number();
        
        // Get finalized block from storage (updated by GRANDPA)
        let last_finalized = LastFinalizedBlock::get();
        let grandpa_lag = current_block.saturating_sub(last_finalized);
        
        let grandpa_is_stuck = grandpa_lag > MAX_GRANDPA_LAG;
        
        if grandpa_is_stuck {
            let stuck_count = GrandpaStuckSessions::get().saturating_add(1);
            GrandpaStuckSessions::put(stuck_count);
            log::warn!(
                target: "runtime::session",
                "⚠️ GRANDPA stuck! Lag: {} blocks, stuck for {} sessions",
                grandpa_lag,
                stuck_count
            );
        } else {
            // Reset stuck counter if GRANDPA is healthy
            if GrandpaStuckSessions::get() > 0 {
                log::info!(
                    target: "runtime::session",
                    "✅ GRANDPA recovered! Resetting stuck counter."
                );
            }
            GrandpaStuckSessions::put(0);
            // Update last finalized block
            LastFinalizedBlock::put(current_block);
        }

        // Collect all accounts that have registered session keys
        let all_registered: Vec<AccountId> = pallet_session::NextKeys::<Runtime>::iter()
            .map(|(account, _keys)| account)
            .collect();

        if all_registered.is_empty() {
            log::warn!(
                target: "runtime::session",
                "⚠️ No registered validators, keeping current set"
            );
            return None;
        }

        // Build list of validators with their join time
        let mut validators_with_join_time: Vec<(AccountId, u32)> = all_registered
            .into_iter()
            .filter_map(|account| {
                // Check if validator is active (producing blocks)
                match ValidatorLastActive::get(&account) {
                    Some(last_active) => {
                        let sessions_inactive = new_index.saturating_sub(last_active);
                        if sessions_inactive > MAX_INACTIVE_SESSIONS {
                            log::warn!(
                                target: "runtime::session",
                                "🚫 Validator {:?} inactive for {} sessions (no blocks), removing",
                                account,
                                sessions_inactive
                            );
                            ValidatorLastActive::remove(&account);
                            ValidatorJoinedAt::remove(&account);
                            return None;
                        }
                    }
                    None => {
                        // New validator - record join time and mark active
                        ValidatorLastActive::insert(&account, new_index);
                        ValidatorJoinedAt::insert(&account, new_index);
                        log::info!(
                            target: "runtime::session",
                            "🆕 New validator {:?} joined at session {}",
                            account,
                            new_index
                        );
                    }
                }
                
                let joined = ValidatorJoinedAt::get(&account).unwrap_or(0);
                Some((account, joined))
            })
            .collect();

        // Sort by join time (oldest first)
        validators_with_join_time.sort_by(|a, b| a.1.cmp(&b.1));

        // If GRANDPA is stuck for too long, remove newest validators
        let stuck_sessions = GrandpaStuckSessions::get();
        if grandpa_is_stuck && stuck_sessions >= 2 && validators_with_join_time.len() > 2 {
            // Find the newest validator that has passed grace period
            let validators_past_grace: Vec<&(AccountId, u32)> = validators_with_join_time
                .iter()
                .filter(|(_, joined)| new_index.saturating_sub(*joined) > GRANDPA_GRACE_SESSIONS)
                .collect();
            
            if validators_past_grace.len() > 2 {
                // Remove the newest one (last in sorted list that's past grace)
                if let Some((newest_account, joined_at)) = validators_with_join_time
                    .iter()
                    .rev()
                    .find(|(_, joined)| new_index.saturating_sub(*joined) > GRANDPA_GRACE_SESSIONS)
                {
                    log::warn!(
                        target: "runtime::session",
                        "🚫 GRANDPA stuck! Removing newest validator {:?} (joined session {})",
                        newest_account,
                        joined_at
                    );
                    
                    ValidatorLastActive::remove(newest_account);
                    ValidatorJoinedAt::remove(newest_account);
                    
                    // Remove from our list
                    validators_with_join_time.retain(|(acc, _)| acc != newest_account);
                    
                    // Reset stuck counter to give network time to recover
                    GrandpaStuckSessions::put(0);
                }
            }
        }

        // Extract just the account IDs
        let active_validators: Vec<AccountId> = validators_with_join_time
            .into_iter()
            .map(|(account, _)| account)
            .collect();

        if active_validators.is_empty() {
            log::error!(
                target: "runtime::session",
                "❌ No active validators! Keeping current set."
            );
            return None;
        }

        // Safety: always keep at least 2 validators
        if active_validators.len() < 2 {
            log::warn!(
                target: "runtime::session",
                "⚠️ Only {} validator(s), need at least 2. Keeping current set.",
                active_validators.len()
            );
            return None;
        }

        log::info!(
            target: "runtime::session",
            "✅ Session {}: {} active validator(s), GRANDPA lag: {} blocks",
            new_index,
            active_validators.len(),
            grandpa_lag
        );

        Some(active_validators)
    }

    fn end_session(_end_index: u32) {}
    
    fn start_session(start_index: u32) {
        if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
            ValidatorLastActive::insert(&author, start_index);
        }
    }
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = sp_runtime::traits::ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<SessionPeriod, SessionOffset>;
    type SessionManager = PermissionlessValidatorSet;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = Session;
    type MaxAuthorities = ConstU32<100>;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = ConstU32<100>;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
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
    pub const TransactionByteFee: Balance = 100_000_000;
    pub FeeMultiplier: sp_runtime::FixedU128 = sp_runtime::FixedU128::from_u32(1);
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = frame_support::weights::IdentityFee<Balance>;
    type LengthToFee = frame_support::weights::IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

/// Find block author using Aura consensus
pub struct AuraAccountAdapter;
impl FindAuthor<AccountId> for AuraAccountAdapter {
    fn find_author<'a, I>(digests: I) -> Option<AccountId>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        pallet_aura::AuraAuthorId::<Runtime>::find_author(digests)
            .map(|author_id| AccountId::from(sp_core::sr25519::Public::from(author_id).0))
    }
}

/// Handler that issues block rewards AND tracks validator activity
pub struct BlockRewardHandler;
impl pallet_authorship::EventHandler<AccountId, BlockNumber> for BlockRewardHandler {
    fn note_author(author: AccountId) {
        let _ = Halving::issue_block_reward(&author);
        let current_session = pallet_session::Pallet::<Runtime>::current_index();
        ValidatorLastActive::insert(&author, current_session);
    }
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = AuraAccountAdapter;
    type EventHandler = BlockRewardHandler;
}

// ============================================
// LUMENYX CUSTOM PALLETS
// ============================================

impl pallet_halving::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
}

parameter_types! {
    pub const MaxNotes: u32 = 1_048_576;
    pub const TreeDepth: u32 = 20;
}

impl pallet_privacy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxNotes = MaxNotes;
    type TreeDepth = TreeDepth;
}

// ============================================
// VALIDATOR FAUCET - Permissionless Bootstrap
// ============================================
parameter_types! {
    pub const ClaimAmount: u128 = 2_000_000_000_000;
    pub const PowDifficulty: u32 = 18;
    pub const MaxClaimsPerBlock: u32 = 5;
}

impl pallet_validator_faucet::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ClaimAmount = ClaimAmount;
    type PowDifficulty = PowDifficulty;
    type MaxClaimsPerBlock = MaxClaimsPerBlock;
}

// ============================================
// EVM CONFIGURATION - ETHEREUM COMPATIBILITY
// ============================================

pub const GAS_PER_SECOND: u64 = 40_000_000;

pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND * 2,
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
        (U256::from(1_000_000_000u64), Weight::zero())
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
    type FindAuthor = FindAuthorTruncated<AuraAccountAdapter>;
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
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Session: pallet_session,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Authorship: pallet_authorship,
        Halving: pallet_halving,
        Privacy: pallet_privacy,
        ValidatorFaucet: pallet_validator_faucet,
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
// RUNTIME APIS
// ============================================

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion { RUNTIME_VERSION }
        fn execute_block(block: Block) { Executive::execute_block(block) }
        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata { OpaqueMetadata::new(Runtime::metadata().into()) }
        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }
        fn metadata_versions() -> sp_std::vec::Vec<u32> { Runtime::metadata_versions() }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }
        fn finalize_block() -> <Block as BlockT>::Header { Executive::finalize_block() }
        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }
        fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(source: TransactionSource, tx: <Block as BlockT>::Extrinsic, block_hash: <Block as BlockT>::Hash) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) { Executive::offchain_worker(header) }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }
        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().to_vec()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> { SessionKeys::generate(seed) }
        fn decode_session_keys(encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }
        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }
        fn submit_report_equivocation_unsigned_extrinsic(
            _: sp_consensus_grandpa::EquivocationProof<<Block as BlockT>::Hash, NumberFor<Block>>,
            _: sp_consensus_grandpa::OpaqueKeyOwnershipProof
        ) -> Option<()> { None }
        fn generate_key_ownership_proof(
            _: sp_consensus_grandpa::SetId,
            _: GrandpaId
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> { None }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce { System::account_nonce(account) }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> {
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
        fn preset_names() -> Vec<sp_genesis_builder::PresetId> { vec![] }
    }

    // ============================================
    // FRONTIER EVM RUNTIME APIS
    // ============================================

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 { EVM_CHAIN_ID }

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

        fn initialize_pending_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header);
        }

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

pub mod opaque {
    use super::*;
    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;
}

pub const VERSION: RuntimeVersion = RUNTIME_VERSION;
