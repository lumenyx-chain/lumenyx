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
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    use lumenyx_primitives::{
        blocks_until_halving,
        calculate_block_reward,
        current_era,
        TOTAL_SUPPLY,
    };

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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
        /// Issue block reward to validator
        /// Called by the consensus/block author system
        pub fn issue_block_reward(validator: &T::AccountId) -> DispatchResult {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(u32::MAX);

            // If emission already ended, do nothing (and do not spam events).
            if Self::emission_finished() {
                return Ok(());
            }

            // Calculate reward
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
            let total_emitted_u128: u128 = total_emitted.try_into().unwrap_or(0);

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
            let actual_reward = reward.min(remaining);

            let reward_balance: BalanceOf<T> = actual_reward.try_into().unwrap_or_default();

            // Issue reward to validator
            T::Currency::deposit_creating(validator, reward_balance);

            // Update total emitted
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
