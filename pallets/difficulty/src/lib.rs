//! # LUMENYX Difficulty Pallet
//!
//! Dynamic PoW difficulty adjustment like Bitcoin.
//! 
//! ## How it works:
//! - Stores current difficulty on-chain (all nodes see same value)
//! - Every ADJUSTMENT_INTERVAL blocks, recalculates difficulty
//! - Based on actual vs target time for last interval
//! - Deterministic: given same state, all nodes calculate same difficulty
//!
//! ## Parameters:
//! - Target block time: 2.5 seconds
//! - Adjustment interval: 60 blocks (~2.5 minutes)
//! - Max adjustment: ±25% per interval

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::Time;
    use frame_system::pallet_prelude::*;

    /// Target block time in milliseconds (2.5 seconds)
    pub const TARGET_BLOCK_TIME_MS: u64 = 2_500;
    
    /// Number of blocks between difficulty adjustments
    /// 60 blocks = ~2.5 minutes at target rate
    pub const ADJUSTMENT_INTERVAL: u32 = 60;
    
    /// Initial difficulty - calibrated for ~1-3 miners
    /// Higher = harder to mine
    pub const INITIAL_DIFFICULTY: u128 = 1_000_000;
    
    /// Minimum difficulty (prevents too-easy mining)
    pub const MIN_DIFFICULTY: u128 = 10_000;
    
    /// Maximum difficulty (prevents stuck chain)
    pub const MAX_DIFFICULTY: u128 = 1_000_000_000_000_000;
    
    /// Maximum adjustment factor (25% up)
    pub const MAX_ADJUSTMENT_UP: u128 = 125;
    
    /// Maximum adjustment factor (25% down)  
    pub const MAX_ADJUSTMENT_DOWN: u128 = 75;
    
    /// Adjustment denominator (100 = percentage base)
    pub const ADJUSTMENT_BASE: u128 = 100;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Time provider for getting current timestamp
        type TimeProvider: frame_support::traits::Time;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Current mining difficulty
    #[pallet::storage]
    #[pallet::getter(fn current_difficulty)]
    pub type CurrentDifficulty<T: Config> = StorageValue<_, u128, ValueQuery, InitialDifficulty>;

    #[pallet::type_value]
    pub fn InitialDifficulty() -> u128 {
        INITIAL_DIFFICULTY
    }

    /// Block number of last difficulty adjustment
    #[pallet::storage]
    #[pallet::getter(fn last_adjustment_block)]
    pub type LastAdjustmentBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Timestamp at start of current interval (in ms)
    #[pallet::storage]
    #[pallet::getter(fn interval_start_time)]
    pub type IntervalStartTime<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Difficulty was adjusted
        DifficultyAdjusted {
            old_difficulty: u128,
            new_difficulty: u128,
            block_number: BlockNumberFor<T>,
            actual_time_ms: u64,
            target_time_ms: u64,
        },
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let current_time: u64 = T::TimeProvider::now().try_into().unwrap_or(0);
            
            // Initialize on first block
            if block_number == 1u32.into() {
                IntervalStartTime::<T>::put(current_time);
                LastAdjustmentBlock::<T>::put(block_number);
                return Weight::from_parts(10_000, 0);
            }

            // Check if we need to adjust difficulty
            let last_adj_block = Self::last_adjustment_block();
            let block_num_u32: u32 = block_number.try_into().unwrap_or(0);
            let last_adj_u32: u32 = last_adj_block.try_into().unwrap_or(0);
            
            let blocks_since_adjustment = block_num_u32.saturating_sub(last_adj_u32);
            
            if blocks_since_adjustment >= ADJUSTMENT_INTERVAL {
                Self::adjust_difficulty(block_number, current_time);
            }

            Weight::from_parts(15_000, 0)
        }
    }

    impl<T: Config> Pallet<T> {
        /// Adjust difficulty based on actual vs target time
        fn adjust_difficulty(block_number: BlockNumberFor<T>, current_time: u64) {
            let interval_start = Self::interval_start_time();
            let old_difficulty = Self::current_difficulty();
            
            // Calculate actual time for this interval
            let actual_time_ms = current_time.saturating_sub(interval_start);
            
            // Target time for ADJUSTMENT_INTERVAL blocks
            let target_time_ms = (ADJUSTMENT_INTERVAL as u64) * TARGET_BLOCK_TIME_MS;
            
            // Prevent division by zero
            if actual_time_ms == 0 {
                return;
            }

            // Calculate adjustment ratio
            // If actual < target (too fast), increase difficulty
            // If actual > target (too slow), decrease difficulty
            //
            // new_diff = old_diff * target_time / actual_time
            //
            // But we clamp to ±25% per adjustment
            
            let new_difficulty = if actual_time_ms < target_time_ms {
                // Too fast - increase difficulty
                let ratio_x100 = (target_time_ms as u128)
                    .saturating_mul(100)
                    .checked_div(actual_time_ms as u128)
                    .unwrap_or(100);
                
                // Clamp to max 125%
                let clamped_ratio = ratio_x100.min(MAX_ADJUSTMENT_UP);
                
                old_difficulty
                    .saturating_mul(clamped_ratio)
                    .checked_div(ADJUSTMENT_BASE)
                    .unwrap_or(old_difficulty)
            } else {
                // Too slow - decrease difficulty
                let ratio_x100 = (target_time_ms as u128)
                    .saturating_mul(100)
                    .checked_div(actual_time_ms as u128)
                    .unwrap_or(100);
                
                // Clamp to min 75%
                let clamped_ratio = ratio_x100.max(MAX_ADJUSTMENT_DOWN);
                
                old_difficulty
                    .saturating_mul(clamped_ratio)
                    .checked_div(ADJUSTMENT_BASE)
                    .unwrap_or(old_difficulty)
            };

            // Apply min/max bounds
            let final_difficulty = new_difficulty.max(MIN_DIFFICULTY).min(MAX_DIFFICULTY);

            // Update storage
            CurrentDifficulty::<T>::put(final_difficulty);
            LastAdjustmentBlock::<T>::put(block_number);
            IntervalStartTime::<T>::put(current_time);

            // Emit event
            Self::deposit_event(Event::DifficultyAdjusted {
                old_difficulty,
                new_difficulty: final_difficulty,
                block_number,
                actual_time_ms,
                target_time_ms,
            });

            log::info!(
                "⚡ Difficulty adjusted: {} -> {} (actual: {}ms, target: {}ms)",
                old_difficulty,
                final_difficulty,
                actual_time_ms,
                target_time_ms
            );
        }

        /// Get current difficulty (for RPC/node)
        pub fn get_difficulty() -> u128 {
            Self::current_difficulty()
        }
    }
}
