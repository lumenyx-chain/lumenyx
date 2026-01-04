//! # LUMENYX Emission Pallet
//! 
//! Implements 3-phase emission schedule:
//! - Phase 0 (Bootstrap): ~12 days (350,000 blocks), 2.4 LUMENYX/block
//! - Phase 1 (Early Adoption): 30 days, 0.3 LUMENYX/block  
//! - Phase 2 (Standard): Forever, 0.25 LUMENYX/block with halving every ~4 years
//!
//! Total supply: 21,000,000 LUMENYX (immutable, fixed)

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
    
    // Import primitives for emission calculations
    use lumenyx_primitives::{
        calculate_block_reward,
        EmissionPhase,
        TOTAL_SUPPLY,
        PHASE_0_END,
        PHASE_1_END,
        BLOCKS_PER_HALVING,
    };

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Current emission phase (Bootstrap, EarlyAdoption, Standard)
    #[pallet::storage]
    #[pallet::getter(fn current_phase)]
    pub type CurrentPhase<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Current halving era within Phase 2 (0 = first era, 1 = after first halving, etc.)
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Total LUMENYX emitted through mining
    #[pallet::storage]
    #[pallet::getter(fn total_emitted)]
    pub type TotalEmitted<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Block reward issued to validator
        BlockRewardIssued { 
            validator: T::AccountId, 
            amount: BalanceOf<T>,
            block_number: BlockNumberFor<T>,
        },
        /// Phase transition occurred
        PhaseTransition {
            from_phase: u8,
            to_phase: u8,
            at_block: BlockNumberFor<T>,
        },
        /// Halving occurred in Phase 2
        HalvingOccurred { 
            new_era: u32, 
            new_reward: BalanceOf<T>,
            at_block: BlockNumberFor<T>,
        },
        /// All 21M LUMENYX have been mined
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
            
            // Check for phase transition
            let new_phase = EmissionPhase::from_block(block_u32);
            let current_phase = Self::current_phase();
            let new_phase_u8 = match new_phase {
                EmissionPhase::Bootstrap => 0,
                EmissionPhase::EarlyAdoption => 1,
                EmissionPhase::Standard => 2,
            };
            
            if new_phase_u8 != current_phase {
                CurrentPhase::<T>::put(new_phase_u8);
                Self::deposit_event(Event::PhaseTransition {
                    from_phase: current_phase,
                    to_phase: new_phase_u8,
                    at_block: block_number,
                });
            }
            
            // Check for halving in Phase 2
            if new_phase == EmissionPhase::Standard {
                let blocks_since_phase2 = block_u32.saturating_sub(PHASE_1_END);
                let expected_era = blocks_since_phase2 / BLOCKS_PER_HALVING;
                let current_era = Self::current_era();
                
                if expected_era > current_era {
                    CurrentEra::<T>::put(expected_era);
                    
                    let new_reward = calculate_block_reward(block_u32);
                    let new_reward_balance: BalanceOf<T> = new_reward.try_into().unwrap_or_default();
                    
                    Self::deposit_event(Event::HalvingOccurred {
                        new_era: expected_era,
                        new_reward: new_reward_balance,
                        at_block: block_number,
                    });
                }
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
            
            // Calculate reward using primitives
            let reward = calculate_block_reward(block_u32);
            
            // Check if we've exceeded total supply
            let total_emitted = Self::total_emitted();
            let total_emitted_u128: u128 = total_emitted.try_into().unwrap_or(0);
            
            if total_emitted_u128 >= TOTAL_SUPPLY {
                Self::deposit_event(Event::EmissionComplete {
                    total_emitted,
                    at_block: block_number,
                });
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
        pub fn current_block_reward() -> u128 {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(0);
            calculate_block_reward(block_u32)
        }
        
        /// Get reward for a specific block number
        pub fn reward_at_block(block_number: u32) -> u128 {
            calculate_block_reward(block_number)
        }
        
        /// Get current phase info
        pub fn phase_info() -> (u8, u128, u32) {
            let block_number = <frame_system::Pallet<T>>::block_number();
            let block_u32: u32 = block_number.try_into().unwrap_or(0);
            let phase = EmissionPhase::from_block(block_u32);
            let current_reward = calculate_block_reward(block_u32);
            
            let blocks_until_next = match phase {
                EmissionPhase::Bootstrap => PHASE_0_END.saturating_sub(block_u32),
                EmissionPhase::EarlyAdoption => PHASE_1_END.saturating_sub(block_u32),
                EmissionPhase::Standard => {
                    let blocks_since_phase2 = block_u32.saturating_sub(PHASE_1_END);
                    BLOCKS_PER_HALVING - (blocks_since_phase2 % BLOCKS_PER_HALVING)
                }
            };
            
            let phase_u8 = match phase {
                EmissionPhase::Bootstrap => 0,
                EmissionPhase::EarlyAdoption => 1,
                EmissionPhase::Standard => 2,
            };
            
            (phase_u8, current_reward, blocks_until_next)
        }
    }
}
