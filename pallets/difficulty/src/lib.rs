//! # LUMENYX Difficulty Pallet - ASERT Algorithm
//!
//! Dynamic PoW difficulty adjustment using ASERT (aserti3-2d).
//! Based on Bitcoin Cash's battle-tested implementation.
//!
//! ## How ASERT works:
//! - Adjusts difficulty EVERY BLOCK (not every N blocks)
//! - Uses exponential formula: next_diff = anchor_diff * 2^((ideal_time - real_time) / halflife)
//! - Anchor = reference point (block #1) for all calculations
//! - Deterministic: all nodes calculate identical difficulty
//!
//! ## Parameters for LUMENYX:
//! - Target block time: 2.5 seconds (2500ms)
//! - Halflife: 720 seconds (12 minutes)
//! - Initial difficulty: 25,000,000

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{SaturatedConversion, Saturating};

    // ============================================
    // ASERT CONSTANTS
    // ============================================

    /// Fixed-point radix = 2^16 (from aserti3-2d specification)
    const RADIX: i128 = 1i128 << 16;

    /// Target block time in milliseconds (2.5 seconds)
    pub const TARGET_BLOCK_TIME_MS: u64 = 2_500;

    /// Halflife in milliseconds (720 seconds = 12 minutes)
    /// This controls how fast difficulty responds to hashrate changes
    pub const HALF_LIFE_MS: u64 = 720_000;

    /// Initial difficulty - calibrated for ~2.5 sec/block with 1 miner
    pub const INITIAL_DIFFICULTY: u128 = 25_000_000;

    /// Minimum difficulty (prevents too-easy mining)
    pub const MIN_DIFFICULTY: u128 = 10_000;

    /// Maximum difficulty (prevents stuck chain)
    pub const MAX_DIFFICULTY: u128 = 1_000_000_000_000_000;

    /// Minimum solve time clamp (prevents timestamp manipulation)
    pub const MIN_SOLVE_TIME_MS: u64 = 1;

    /// Maximum solve time clamp (10x target, prevents timestamp manipulation)
    pub const MAX_SOLVE_TIME_MS: u64 = 25_000;

    // ============================================
    // ANCHOR STRUCTURE
    // ============================================

    /// Anchor info for ASERT calculations
    /// The anchor is set at block #1 and used as reference for all future calculations
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct AnchorInfo<BlockNumber> {
        /// Height of the anchor block
        pub anchor_height: BlockNumber,
        /// Timestamp (ms) of the parent of the anchor block
        pub anchor_parent_time_ms: u64,
        /// Difficulty at the anchor block
        pub anchor_difficulty: u128,
    }

    // ============================================
    // PALLET CONFIG
    // ============================================

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ============================================
    // STORAGE
    // ============================================

    /// Current mining difficulty (read by miner for next block)
    #[pallet::storage]
    #[pallet::getter(fn current_difficulty)]
    pub type CurrentDifficulty<T: Config> = StorageValue<_, u128, ValueQuery, InitialDifficultyValue>;

    #[pallet::type_value]
    pub fn InitialDifficultyValue() -> u128 {
        INITIAL_DIFFICULTY
    }

    /// Last effective timestamp (ms) - used for deterministic time series
    #[pallet::storage]
    #[pallet::getter(fn last_effective_time_ms)]
    pub type LastEffectiveTimeMs<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// ASERT Anchor - set at block #1, used for all calculations
    #[pallet::storage]
    #[pallet::getter(fn anchor)]
    pub type Anchor<T: Config> = StorageValue<_, AnchorInfo<BlockNumberFor<T>>, OptionQuery>;

    // ============================================
    // EVENTS
    // ============================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Difficulty updated for next block
        DifficultyUpdated {
            block_number: BlockNumberFor<T>,
            old_difficulty: u128,
            new_difficulty: u128,
        },
        /// Anchor was set (happens once at block #1)
        AnchorSet {
            height: BlockNumberFor<T>,
            parent_time_ms: u64,
            difficulty: u128,
        },
    }

    // ============================================
    // HOOKS - CALLED EVERY BLOCK
    // ============================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_number: BlockNumberFor<T>) {
            // 1) Get current timestamp from pallet_timestamp (in ms)
            let now_ms: u64 = pallet_timestamp::Pallet::<T>::get().saturated_into::<u64>();

            // 2) Calculate effective timestamp with clamp (prevents manipulation)
            let mut prev_eff_ms = LastEffectiveTimeMs::<T>::get();
            // First block: initialize with real timestamp
            if prev_eff_ms == 0 {
                prev_eff_ms = now_ms.saturating_sub(TARGET_BLOCK_TIME_MS);
                LastEffectiveTimeMs::<T>::put(prev_eff_ms);
            }
            
            let mut solve_ms = now_ms.saturating_sub(prev_eff_ms);
            
            // Clamp solve time to prevent timestamp manipulation
            if solve_ms < MIN_SOLVE_TIME_MS {
                solve_ms = MIN_SOLVE_TIME_MS;
            }
            if solve_ms > MAX_SOLVE_TIME_MS {
                solve_ms = MAX_SOLVE_TIME_MS;
            }

            let eff_now_ms = prev_eff_ms.saturating_add(solve_ms);
            LastEffectiveTimeMs::<T>::put(eff_now_ms);

            // 3) Set anchor if not exists (first block)
            let anchor = match Anchor::<T>::get() {
                Some(a) => a,
                None => {
                    let a = AnchorInfo {
                        anchor_height: block_number,
                        anchor_parent_time_ms: now_ms.saturating_sub(TARGET_BLOCK_TIME_MS),
                        anchor_difficulty: CurrentDifficulty::<T>::get(),
                    };
                    Anchor::<T>::put(&a);
                    Self::deposit_event(Event::AnchorSet {
                        height: block_number,
                        parent_time_ms: prev_eff_ms,
                        difficulty: a.anchor_difficulty,
                    });
                    log::info!(
                        "🎯 ASERT Anchor set at block {:?}: difficulty={}, parent_time={}ms",
                        block_number,
                        a.anchor_difficulty,
                        prev_eff_ms
                    );
                    a
                }
            };

            // 4) Calculate next difficulty using ASERT
            let old_difficulty = CurrentDifficulty::<T>::get();
            let new_difficulty = Self::calculate_asert_difficulty(&anchor, block_number, eff_now_ms);

            // 5) Update storage
            CurrentDifficulty::<T>::put(new_difficulty);

            // 6) Emit event and log
            Self::deposit_event(Event::DifficultyUpdated {
                block_number,
                old_difficulty,
                new_difficulty,
            });

            // Log significant changes (more than 1%)
            let change_percent = if new_difficulty > old_difficulty {
                ((new_difficulty - old_difficulty) * 100) / old_difficulty
            } else {
                ((old_difficulty - new_difficulty) * 100) / old_difficulty
            };

            if change_percent > 0 {
                log::info!(
                    "⚡ Difficulty: {} -> {} ({}% change)",
                    old_difficulty,
                    new_difficulty,
                    if new_difficulty > old_difficulty { "+" } else { "-" }
                );
            }
        }
    }

    // ============================================
    // ASERT IMPLEMENTATION
    // ============================================

    impl<T: Config> Pallet<T> {
        /// Calculate next difficulty using ASERT formula:
        /// next_difficulty = anchor_difficulty * 2^((ideal_time - real_time) / halflife)
        ///
        /// Where:
        /// - ideal_time = TARGET_BLOCK_TIME * (height_delta + 1)
        /// - real_time = current_time - anchor_parent_time
        fn calculate_asert_difficulty(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64,
        ) -> u128 {
            // Safety check: halflife must not be zero
            if HALF_LIFE_MS == 0 {
                return Self::clamp_difficulty(anchor.anchor_difficulty);
            }

            // Calculate height delta (how many blocks since anchor)
            let height_delta_u64: u64 = eval_height
                .saturating_sub(anchor.anchor_height)
                .saturated_into::<u64>();

            // ideal_time = how much time SHOULD have passed for this many blocks
            // We use (height_delta + 1) because we're calculating for the NEXT block
            let ideal_time_ms: i128 = (TARGET_BLOCK_TIME_MS as i128)
                .saturating_mul((height_delta_u64.saturating_add(1)) as i128);

            // real_time = how much time ACTUALLY passed since anchor
            let real_time_ms: i128 =
                (eval_time_ms as i128).saturating_sub(anchor.anchor_parent_time_ms as i128);

            // exponent = (ideal - real) / halflife
            // If blocks are too fast: ideal > real → positive exponent → difficulty increases
            // If blocks are too slow: ideal < real → negative exponent → difficulty decreases
            let ideal_minus_real: i128 = ideal_time_ms.saturating_sub(real_time_ms);
            
            // Convert to fixed-point for precision
            let exponent_fixed: i128 = ideal_minus_real
                .saturating_mul(RADIX)
                / (HALF_LIFE_MS as i128);

            // Calculate 2^exponent using aserti3-2d approximation
            let (num_shifts, factor_q16) = Self::approx_pow2_fixed(exponent_fixed);

            // next = anchor_difficulty * factor
            let mut next: u128 = match anchor.anchor_difficulty.checked_mul(factor_q16) {
                Some(v) => v,
                None => return MAX_DIFFICULTY, // Overflow protection
            };

            // Apply the integer part of the exponent (2^num_shifts)
            if num_shifts < 0 {
                // Difficulty decreasing (shift right)
                let s = (-num_shifts) as u32;
                if s >= 128 {
                    next = 0;
                } else {
                    next >>= s;
                }
            } else if num_shifts > 0 {
                // Difficulty increasing (shift left)
                let s = num_shifts as u32;
                if s >= 128 {
                    return MAX_DIFFICULTY;
                }
                next = match next.checked_shl(s) {
                    Some(v) => v,
                    None => return MAX_DIFFICULTY,
                };
            }

            // Divide by RADIX to convert from Q16 to integer
            next >>= 16;

            // Safety: if result is 0, use minimum
            if next == 0 {
                return MIN_DIFFICULTY;
            }

            Self::clamp_difficulty(next)
        }

        /// Approximate 2^(exponent_fixed / 2^16) using cubic polynomial
        /// This is the aserti3-2d algorithm from Bitcoin Cash
        ///
        /// Returns:
        /// - num_shifts: integer part of exponent (for 2^n via bit shift)
        /// - factor_q16: fractional part as Q16 fixed-point [65536..131072)
        fn approx_pow2_fixed(exponent_fixed: i128) -> (i128, u128) {
            // Split into integer and fractional parts
            // num_shifts = floor(exponent / 2^16) using arithmetic shift
            let num_shifts: i128 = exponent_fixed >> 16;

            // frac = exponent - num_shifts * RADIX, result in [0, 65535]
            let frac: i128 = exponent_fixed.saturating_sub(num_shifts.saturating_mul(RADIX));
            let x: u128 = frac as u128;

            // Cubic polynomial approximation of 2^x for x in [0, 1)
            // Constants from aserti3-2d specification:
            // factor = ((A*x + B*x^2 + C*x^3 + 2^47) >> 48) + 65536
            let x2 = x.saturating_mul(x);
            let x3 = x2.saturating_mul(x);

            // Magic constants from Bitcoin Cash aserti3-2d
            let a: u128 = 195_766_423_245_049u128;
            let b: u128 = 971_821_376u128;
            let c: u128 = 5_127u128;

            let poly = a.saturating_mul(x)
                .saturating_add(b.saturating_mul(x2))
                .saturating_add(c.saturating_mul(x3))
                .saturating_add(1u128 << 47);

            let factor_q16 = (poly >> 48).saturating_add(65_536u128);

            (num_shifts, factor_q16)
        }

        /// Clamp difficulty to min/max bounds
        fn clamp_difficulty(d: u128) -> u128 {
            if d < MIN_DIFFICULTY {
                MIN_DIFFICULTY
            } else if d > MAX_DIFFICULTY {
                MAX_DIFFICULTY
            } else {
                d
            }
        }

        /// Get current difficulty (for RPC/node)
        pub fn get_difficulty() -> u128 {
            Self::current_difficulty()
        }
    }
}
