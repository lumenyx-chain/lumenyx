//! # LUMENYX Emission Pallet
//!
//! Simple emission like Bitcoin:
//! - ~0.208 LUMENYX per block from genesis
//! - Halving every 4 years (50,492,160 blocks)
//! - Total supply: 21,000,000 LUMENYX (immutable)
//!
//! Daily emission: ~7,187 LUMENYX (34,560 blocks * 0.208)
//! ~50% mined in first 4 years

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::Decode;
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{SaturatedConversion, Saturating};
    use sp_std::vec::Vec;

    use lumenyx_primitives::{
        blocks_until_halving, calculate_block_reward, current_era, TOTAL_SUPPLY,
        FORK_HEIGHT_V2, DECIMAL_MIGRATION_FACTOR,
    };

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Pool payout digest for P2Pool PPLNS
    #[derive(Clone, codec::Encode, codec::Decode, scale_info::TypeInfo)]
    pub struct PoolPayoutDigest {
        pub sharechain_tip: sp_core::H256,
        pub block_reward: u128,
        pub payouts: Vec<([u8; 32], u128)>,
    }

    const POOL_PAYOUT_DIGEST_TAG: &[u8; 4] = b"PPLN";

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Current halving era (0 = first era, 1 = after first halving, etc.)
    #[pallet::storage]
    #[pallet::getter(fn current_halving_era)]
    pub type CurrentHalvingEra<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Total LUMENYX emitted through mining (this pallet only)
    #[pallet::storage]
    #[pallet::getter(fn total_emitted)]
    pub type TotalEmitted<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// True once emission has ended (reward == 0) and EmissionComplete was emitted.
    #[pallet::storage]
    #[pallet::getter(fn emission_finished)]
    pub type EmissionFinished<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// True once the decimal migration (12→18) has been applied at block 440,000
    #[pallet::storage]
    #[pallet::getter(fn decimal_migration_done)]
    pub type DecimalMigrationDone<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Block reward issued to validator
        BlockRewardIssued {
            validator: T::AccountId,
            amount: BalanceOf<T>,
            block_number: BlockNumberFor<T>,
        },
        /// Halving occurred
        HalvingOccurred {
            era: u32,
            new_reward: BalanceOf<T>,
            at_block: BlockNumberFor<T>,
        },
        /// Emission has completed (reward reached 0 or supply cap reached)
        EmissionComplete {
            total_emitted: BalanceOf<T>,
            at_block: BlockNumberFor<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// All coins have been emitted
        EmissionComplete,
        /// Arithmetic overflow
        ArithmeticOverflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_u32: u32 = block_number.try_into().unwrap_or(u32::MAX);

            // ======================================================
            // FORK v2.3.0: Decimal migration at block 440,000
            // Multiply ALL balances by 10^6 (12→18 decimals)
            // Runs exactly once, guarded by DecimalMigrationDone flag
            // ======================================================
            if block_u32 >= FORK_HEIGHT_V2 && !Self::decimal_migration_done() {
                Self::run_decimal_migration();
            }

            // Check for halving
            let expected_era = current_era(block_u32);
            let stored_era = Self::current_halving_era();

            if expected_era > stored_era {
                CurrentHalvingEra::<T>::put(expected_era);

                let new_reward = calculate_block_reward(block_u32);
                let new_reward_balance: BalanceOf<T> = new_reward.try_into().unwrap_or_default();

                Self::deposit_event(Event::HalvingOccurred {
                    era: expected_era,
                    new_reward: new_reward_balance,
                    at_block: block_number,
                });
            }

            Weight::from_parts(5_000, 0)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    impl<T: Config> Pallet<T> {
        /// Execute the decimal migration (12→18) via raw storage.
        /// This multiplies all account balances, TotalIssuance, TotalEmitted,
        /// and BaseFeePerGas by DECIMAL_MIGRATION_FACTOR (10^6).
        /// Runs exactly once at block FORK_HEIGHT_V2 (440,000).
        fn run_decimal_migration() {
            log::info!("🔄 FORK v2.3.0: Starting decimal migration");
            log::info!("🔄 Multiplying all balances by {} (12→18 decimals)", DECIMAL_MIGRATION_FACTOR);

            let factor: u128 = DECIMAL_MIGRATION_FACTOR;

            // 1. Migrate all System::Account balances via raw storage
            // Storage prefix: twox_128("System") ++ twox_128("Account")
            let prefix = frame_support::storage::storage_prefix(b"System", b"Account");
            let mut count: u64 = 0;

            let mut next = frame_support::storage::unhashed::get_raw(&prefix)
                .map(|_| prefix.to_vec())
                .or_else(|| {
                    sp_io::storage::next_key(&prefix)
                        .filter(|k| k.starts_with(&prefix))
                });

            // Iterate all keys under System::Account
            // Each value is AccountInfo { nonce: u32, consumers: u32, providers: u32, sufficients: u32, data: AccountData { free: u128, reserved: u128, frozen: u128 } }
            // We need to decode, multiply balance fields, re-encode
            let mut key = sp_io::storage::next_key(&prefix);
            while let Some(k) = key {
                if !k.starts_with(&prefix) {
                    break;
                }
                if let Some(raw) = sp_io::storage::get(&k) {
                    // AccountInfo layout: nonce(4) + consumers(4) + providers(4) + sufficients(4) + free(16) + reserved(16) + frozen(16)
                    // Total: 64 bytes with compact encoding, but actually SCALE encoded
                    // Use codec::Decode
                    #[derive(codec::Encode, codec::Decode)]
                    struct AccountInfo {
                        nonce: u32,
                        consumers: u32,
                        providers: u32,
                        sufficients: u32,
                        data: AccountData,
                    }
                    #[derive(codec::Encode, codec::Decode)]
                    struct AccountData {
                        free: u128,
                        reserved: u128,
                        frozen: u128,
                    }

                    if let Ok(mut info) = AccountInfo::decode(&mut &raw[..]) {
                        info.data.free = info.data.free.saturating_mul(factor);
                        info.data.reserved = info.data.reserved.saturating_mul(factor);
                        info.data.frozen = info.data.frozen.saturating_mul(factor);
                        sp_io::storage::set(&k, &info.encode());
                        count += 1;
                    }
                }
                key = sp_io::storage::next_key(&k);
            }
            log::info!("🔄 Migrated {} accounts", count);

            // 2. Migrate TotalIssuance via raw storage
            // Key: twox_128("Balances") ++ twox_128("TotalIssuance")
            let ti_key = frame_support::storage::storage_prefix(b"Balances", b"TotalIssuance");
            if let Some(raw) = frame_support::storage::unhashed::get_raw(&ti_key) {
                if let Ok(old_val) = <u128 as codec::Decode>::decode(&mut &raw[..]) {
                    let new_val = old_val.saturating_mul(factor);
                    frame_support::storage::unhashed::put(&ti_key, &new_val);
                    log::info!("🔄 TotalIssuance: {} → {}", old_val, new_val);
                }
            }

            // 3. Migrate TotalEmitted (our own storage)
            TotalEmitted::<T>::mutate(|total| {
                let old: u128 = (*total).saturated_into();
                let new_val = old.saturating_mul(factor);
                *total = new_val.saturated_into();
            });
            log::info!("🔄 TotalEmitted migrated");

            // 4. Migrate BaseFeePerGas via raw storage
            // Key: twox_128("BaseFee") ++ twox_128("BaseFeePerGas")
            let bf_key = frame_support::storage::storage_prefix(b"BaseFee", b"BaseFeePerGas");
            if let Some(raw) = frame_support::storage::unhashed::get_raw(&bf_key) {
                if let Ok(old_val) = <sp_core::U256 as codec::Decode>::decode(&mut &raw[..]) {
                    let new_val = old_val * sp_core::U256::from(factor);
                    frame_support::storage::unhashed::put(&bf_key, &new_val);
                    log::info!("🔄 BaseFeePerGas migrated");
                }
            }

            // Mark done - never run again
            DecimalMigrationDone::<T>::put(true);
            log::info!("✅ FORK v2.3.0: Decimal migration COMPLETE");
        }

        /// Extract pool payout digest from block header
        fn pool_payout_digest_from_header() -> Option<PoolPayoutDigest> {
            for item in frame_system::Pallet::<T>::digest().logs.iter() {
                if let sp_runtime::generic::DigestItem::Other(bytes) = item {
                    if bytes.len() < 4 {
                        continue;
                    }
                    if &bytes[0..4] != POOL_PAYOUT_DIGEST_TAG {
                        continue;
                    }

                    let mut payload = &bytes[4..];
                    if let Ok(d) = PoolPayoutDigest::decode(&mut payload) {
                        return Some(d);
                    }
                }
            }
            None
        }

        /// Issue block reward (validator OR P2Pool PPLNS via digest)
        /// Called by the consensus/block author system
        pub fn issue_block_reward(validator: &T::AccountId) -> DispatchResult {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(u32::MAX);

            // If emission already ended, do nothing (and do not spam events).
            if Self::emission_finished() {
                return Ok(());
            }

            // Calculate scheduled reward
            let reward = calculate_block_reward(block_u32);

            // Bitcoin-like stop condition: when reward == 0, emission is finished.
            if reward == 0 {
                let total_emitted = Self::total_emitted();
                Self::deposit_event(Event::EmissionComplete {
                    total_emitted,
                    at_block: block_number,
                });
                EmissionFinished::<T>::put(true);
                return Ok(());
            }

            // Check supply cap (defensive): if already reached, emit once and stop.
            let total_emitted = Self::total_emitted();
            let total_emitted_u128: u128 = total_emitted.saturated_into::<u128>();

            if total_emitted_u128 >= TOTAL_SUPPLY {
                Self::deposit_event(Event::EmissionComplete {
                    total_emitted,
                    at_block: block_number,
                });
                EmissionFinished::<T>::put(true);
                return Err(Error::<T>::EmissionComplete.into());
            }

            // Cap reward if it would exceed total supply
            let remaining = TOTAL_SUPPLY.saturating_sub(total_emitted_u128);
            let actual_reward_u128: u128 = reward.min(remaining);

            // -------------------- P2Pool digest path --------------------
            // If digest is present + valid -> pay miners according to payouts
            if let Some(d) = Self::pool_payout_digest_from_header() {
                // 1) Reward declared in digest must match scheduled/capped reward
                if d.block_reward == actual_reward_u128 {
                    // 2) Validate sum(payouts) <= block_reward
                    let mut sum: u128 = 0;
                    for (_acc, amt) in &d.payouts {
                        sum = sum.saturating_add(*amt);
                    }

                    if sum <= d.block_reward {
                        // Pay each miner
                        for (acc32, amt_u128) in d.payouts {
                            if amt_u128 == 0 {
                                continue;
                            }

                            // Convert [u8; 32] to AccountId via codec
                            let who: T::AccountId = match T::AccountId::decode(&mut &acc32[..]) {
                                Ok(acc) => acc,
                                Err(_) => continue, // skip invalid account
                            };
                            let amount: BalanceOf<T> = amt_u128.saturated_into();

                            T::Currency::deposit_creating(&who, amount);
                        }

                        // Remainder goes to validator (optional, but avoids "lost emission")
                        let remainder_u128: u128 = d.block_reward.saturating_sub(sum);
                        if remainder_u128 > 0 {
                            let rem: BalanceOf<T> = remainder_u128.saturated_into();
                            T::Currency::deposit_creating(validator, rem);
                        }

                        // Update total emitted by full actual reward (not just sum)
                        let actual_reward_balance: BalanceOf<T> =
                            actual_reward_u128.saturated_into();
                        TotalEmitted::<T>::mutate(|total| {
                            *total = total.saturating_add(actual_reward_balance)
                        });

                        Self::deposit_event(Event::BlockRewardIssued {
                            validator: validator.clone(),
                            amount: actual_reward_balance,
                            block_number,
                        });

                        return Ok(());
                    }
                }
                // else: digest invalid -> fallback to validator-only payout
            }
            // ------------------------------------------------------------

            // Fallback: Issue reward to validator (current behavior)
            let reward_balance: BalanceOf<T> = actual_reward_u128.saturated_into();
            T::Currency::deposit_creating(validator, reward_balance);

            TotalEmitted::<T>::mutate(|total| *total = total.saturating_add(reward_balance));

            Self::deposit_event(Event::BlockRewardIssued {
                validator: validator.clone(),
                amount: reward_balance,
                block_number,
            });

            Ok(())
        }

        /// Get current block reward (for UI/RPC)
        pub fn get_current_reward() -> u128 {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(0);
            calculate_block_reward(block_u32)
        }

        /// Get emission info: (current_reward, blocks_until_halving, current_era)
        pub fn emission_info() -> (u128, u32, u32) {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(0);

            (
                calculate_block_reward(block_u32),
                blocks_until_halving(block_u32),
                current_era(block_u32),
            )
        }
    }
}
