//! # LUMENYX Difficulty Pallet - ASERT Algorithm
//!
//! Dynamic PoW difficulty adjustment using ASERT (aserti3-2d).
//! Based on Bitcoin Cash's battle-tested implementation.
//!
//! ## How ASERT works:
//! - Adjusts difficulty EVERY BLOCK (not every N blocks)
//! - Uses exponential formula
//! - Deterministic: all nodes calculate identical difficulty
//!
//! ## Parameters for LUMENYX:
//! - Target block time: 2.5 seconds (2500ms)
//! - Halflife: 60 seconds (1 minute)
//! - Initial difficulty: 1

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

    /// Halflife in milliseconds (60 seconds = 1 minute)
    /// This controls how fast difficulty responds to hashrate changes
    pub const HALF_LIFE_MS: u64 = 60_000;

    /// Initial difficulty - calibrated for ~2.5 sec/block with 1 miner
    pub const INITIAL_DIFFICULTY: u128 = 1;

    /// Minimum difficulty (prevents too-easy mining)
    pub const MIN_DIFFICULTY: u128 = 1;

    /// Maximum difficulty (prevents stuck chain)
    pub const MAX_DIFFICULTY: u128 = u128::MAX;

    /// Minimum solve time clamp (prevents timestamp manipulation) - legacy (pre-fork only)
    pub const MIN_SOLVE_TIME_MS: u64 = 50;

    /// Maximum solve time clamp (legacy pre-fork only)
    pub const MAX_SOLVE_TIME_MS: u64 = 25_000;

    /// Hard fork activation height (dual-rule switch)
    /// NOTE: emergency consensus change; nodes must upgrade before this height.
    pub const FORK_HEIGHT: u64 = 75_000;

    /// Safety valve (post-fork only): if parent->now gap exceeds this, force MIN_DIFFICULTY
    pub const SAFETY_VALVE_MS: u64 = 600_000; // 10 minutes

    // ============================================
    // ANCHOR STRUCTURE
    // ============================================

    /// Anchor info for ASERT calculations
    /// The anchor is set at block #1 and used as reference for all future calculations
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct AnchorInfo<BlockNumber> {
        /// Height of the anchor block
        pub anchor_height: BlockNumber,
        /// Timestamp (ms) of the parent of the anchor block (legacy naming; in LUMENYX v1 it's an offset)
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

    /// Flag to track if pallet has been initialized (prevents re-genesis)
    #[pallet::storage]
    #[pallet::getter(fn initialized)]
    pub type Initialized<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Current mining difficulty (read by miner for next block)
    /// OptionQuery to detect missing storage
    #[pallet::storage]
    #[pallet::getter(fn current_difficulty)]
    pub type CurrentDifficulty<T: Config> = StorageValue<_, u128, OptionQuery>;

    /// Last effective timestamp (ms) - used for deterministic time series (legacy pre-fork)
    #[pallet::storage]
    #[pallet::getter(fn last_effective_time_ms)]
    pub type LastEffectiveTimeMs<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Last real timestamp (ms) of the previous block.
    /// Updated every block (pre and post fork), so first post-fork block has a valid parent time.
    #[pallet::storage]
    #[pallet::getter(fn last_block_time_ms)]
    pub type LastBlockTimeMs<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// ASERT Anchor - set at block #1, used for all calculations
    #[pallet::storage]
    #[pallet::getter(fn anchor)]
    pub type Anchor<T: Config> = StorageValue<_, AnchorInfo<BlockNumberFor<T>>, OptionQuery>;

    // ============================================
    // GENESIS CONFIG
    // ============================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_difficulty: u128,
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Only initialize if not already initialized
            if !Initialized::<T>::get() {
                CurrentDifficulty::<T>::put(self.initial_difficulty);
                LastEffectiveTimeMs::<T>::put(0);
                LastBlockTimeMs::<T>::put(0);
                Anchor::<T>::kill(); // Ensure clean state
                Initialized::<T>::put(true);
                log::info!(
                    "🎯 Difficulty initialized at genesis: {}",
                    self.initial_difficulty
                );
            } else {
                log::info!("✅ Difficulty already initialized, skipping genesis init");
            }
        }
    }

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

            // 2) Get current difficulty with explicit None handling
            let old_difficulty = match CurrentDifficulty::<T>::get() {
                Some(d) => d,
                None => {
                    log::warn!(
                        "⚠️ CurrentDifficulty storage missing! Initializing to {}",
                        INITIAL_DIFFICULTY
                    );
                    CurrentDifficulty::<T>::put(INITIAL_DIFFICULTY);
                    Initialized::<T>::put(true);
                    INITIAL_DIFFICULTY
                }
            };

            // 3) Set anchor if not exists (first block or after missing storage)
            let anchor = match Anchor::<T>::get() {
                Some(a) => a,
                None => {
                    let cur = CurrentDifficulty::<T>::get().unwrap_or(INITIAL_DIFFICULTY);
                    let a = AnchorInfo {
                        anchor_height: block_number,
                        anchor_parent_time_ms: now_ms.saturating_sub(TARGET_BLOCK_TIME_MS),
                        anchor_difficulty: cur,
                    };
                    Anchor::<T>::put(&a);
                    Self::deposit_event(Event::AnchorSet {
                        height: block_number,
                        parent_time_ms: now_ms.saturating_sub(TARGET_BLOCK_TIME_MS),
                        difficulty: a.anchor_difficulty,
                    });
                    log::info!(
                        "🎯 ASERT Anchor set at block {:?}: difficulty={}, parent_time={}ms",
                        block_number,
                        a.anchor_difficulty,
                        now_ms.saturating_sub(TARGET_BLOCK_TIME_MS)
                    );
                    a
                }
            };

            // 4) Dual-rule switch
            let bn_u64: u64 = block_number.saturated_into::<u64>();
            let new_difficulty: u128;

            if bn_u64 < FORK_HEIGHT {
                // ==========================
                // PRE-FORK (legacy behavior)
                // ==========================

                // Calculate effective timestamp with clamp (prevents manipulation)
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

                new_difficulty =
                    Self::calculate_asert_difficulty_legacy(&anchor, block_number, eff_now_ms);
            } else {
                // ==========================
                // POST-FORK (new behavior)
                // ==========================

                // Parent time from storage written by previous block.
                let mut parent_time_ms: u64 = LastBlockTimeMs::<T>::get();

                // Handle node that upgrades/starts after fork and didn't populate the new storage yet.
                if parent_time_ms == 0 {
                    parent_time_ms = now_ms.saturating_sub(TARGET_BLOCK_TIME_MS);
                }

                // Safety valve (post-fork only)
                if now_ms.saturating_sub(parent_time_ms) > SAFETY_VALVE_MS {
                    new_difficulty = MIN_DIFFICULTY;
                } else {
                    // Use real timestamp now_ms (no effective time, no clamp)
                    new_difficulty =
                        Self::calculate_asert_difficulty_postfork(&anchor, block_number, now_ms);
                }
            }

            // 5) Update storage
            CurrentDifficulty::<T>::put(new_difficulty);

            // 6) Emit event and log
            Self::deposit_event(Event::DifficultyUpdated {
                block_number,
                old_difficulty,
                new_difficulty,
            });

            // Log significant changes (more than 0%)
            if old_difficulty > 0 {
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

            // IMPORTANT: always write last real time, pre and post fork.
            // This ensures block (FORK_HEIGHT) reads a correct parent time from block (FORK_HEIGHT-1).
            LastBlockTimeMs::<T>::put(now_ms);
        }
    }

    // ============================================
    // ASERT IMPLEMENTATION
    // ============================================

    impl<T: Config> Pallet<T> {
        /// Legacy ASERT (pre-fork): uses effective time clamp and legacy pow2.
        /// next_difficulty = anchor_difficulty * 2^((ideal_time - real_time) / halflife)
        fn calculate_asert_difficulty_legacy(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64,
        ) -> u128 {
            if HALF_LIFE_MS == 0 {
                return Self::clamp_difficulty(anchor.anchor_difficulty);
            }

            let height_delta_u64: u64 = eval_height
                .saturating_sub(anchor.anchor_height)
                .saturated_into::<u64>();

            let ideal_time_ms: i128 = (TARGET_BLOCK_TIME_MS as i128)
                .saturating_mul((height_delta_u64.saturating_add(1)) as i128);

            let real_time_ms: i128 =
                (eval_time_ms as i128).saturating_sub(anchor.anchor_parent_time_ms as i128);

            let ideal_minus_real: i128 = ideal_time_ms.saturating_sub(real_time_ms);

            let exponent_fixed: i128 =
                ideal_minus_real.saturating_mul(RADIX) / (HALF_LIFE_MS as i128);

            let (num_shifts, factor_q16) = Self::approx_pow2_fixed(exponent_fixed);

            let mut next: u128 = match anchor.anchor_difficulty.checked_mul(factor_q16) {
                Some(v) => v,
                None => return MAX_DIFFICULTY,
            };

            if num_shifts < 0 {
                let s = (-num_shifts) as u32;
                if s >= 128 {
                    next = 0;
                } else {
                    next >>= s;
                }
            } else if num_shifts > 0 {
                let s = num_shifts as u32;
                if s >= 128 {
                    return MAX_DIFFICULTY;
                }
                next = match next.checked_shl(s) {
                    Some(v) => v,
                    None => return MAX_DIFFICULTY,
                };
            }

            next >>= 16;

            if next == 0 {
                return MIN_DIFFICULTY;
            }

            Self::clamp_difficulty(next)
        }

        /// Post-fork ASERT: no effective time clamp, robust pow2 normalization.
        ///
        /// NOTE: ASERT spec defines the formula for TARGET (not difficulty). LUMENYX uses "difficulty"
        /// directly (hash < MAX_HASH / difficulty), i.e. difficulty is inverse of target.
        /// Therefore we use the inverted sign form:
        /// next_diff = anchor_diff * 2^((ideal_time - time_delta)/halflife)  [web:5]
        fn calculate_asert_difficulty_postfork(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64, // now_ms
        ) -> u128 {
            if HALF_LIFE_MS == 0 {
                return Self::clamp_difficulty(anchor.anchor_difficulty);
            }

            let height_delta_u64: u64 = eval_height
                .saturating_sub(anchor.anchor_height)
                .saturated_into::<u64>();

            let ideal_time_ms: i128 = (TARGET_BLOCK_TIME_MS as i128)
                .saturating_mul((height_delta_u64.saturating_add(1)) as i128);

            let time_delta_ms: i128 =
                (eval_time_ms as i128).saturating_sub(anchor.anchor_parent_time_ms as i128);

            let ideal_minus_time: i128 = ideal_time_ms.saturating_sub(time_delta_ms);

            let exponent_fixed: i128 =
                ideal_minus_time.saturating_mul(RADIX) / (HALF_LIFE_MS as i128);

            let (num_shifts, factor_q16) = Self::approx_pow2_fixed_postfork(exponent_fixed);

            let mut next: u128 = match anchor.anchor_difficulty.checked_mul(factor_q16) {
                Some(v) => v,
                None => return MAX_DIFFICULTY,
            };

            if num_shifts < 0 {
                let s = (-num_shifts) as u32;
                if s >= 128 {
                    next = 0;
                } else {
                    next >>= s;
                }
            } else if num_shifts > 0 {
                let s = num_shifts as u32;
                if s >= 128 {
                    return MAX_DIFFICULTY;
                }
                next = match next.checked_shl(s) {
                    Some(v) => v,
                    None => return MAX_DIFFICULTY,
                };
            }

            next >>= 16;

            if next == 0 {
                return MIN_DIFFICULTY;
            }

            Self::clamp_difficulty(next)
        }

        /// Approximate 2^(exponent_fixed / 2^16) using cubic polynomial
        /// Legacy version (pre-fork) - kept unchanged for compatibility.
        fn approx_pow2_fixed(exponent_fixed: i128) -> (i128, u128) {
            let num_shifts: i128 = exponent_fixed >> 16;

            let frac: i128 = exponent_fixed.saturating_sub(num_shifts.saturating_mul(RADIX));
            let x: u128 = frac as u128;

            let x2 = x.saturating_mul(x);
            let x3 = x2.saturating_mul(x);

            let a: u128 = 195_766_423_245_049u128;
            let b: u128 = 971_821_376u128;
            let c: u128 = 5_127u128;

            let poly = a
                .saturating_mul(x)
                .saturating_add(b.saturating_mul(x2))
                .saturating_add(c.saturating_mul(x3))
                .saturating_add(1u128 << 47);

            let factor_q16 = (poly >> 48).saturating_add(65_536u128);

            (num_shifts, factor_q16)
        }

        /// Post-fork pow2 approximation with robust frac normalization.
        /// Prevents negative frac casting to u128.
        fn approx_pow2_fixed_postfork(exponent_fixed: i128) -> (i128, u128) {
            let mut num_shifts: i128 = exponent_fixed >> 16;
            let mut frac: i128 = exponent_fixed - num_shifts * RADIX;

            if frac < 0 {
                frac += RADIX;
                num_shifts -= 1;
            }
            if frac >= RADIX {
                frac = RADIX - 1;
            }

            let x: u128 = frac as u128;

            let x2 = x.saturating_mul(x);
            let x3 = x2.saturating_mul(x);

            let a: u128 = 195_766_423_245_049u128;
            let b: u128 = 971_821_376u128;
            let c: u128 = 5_127u128;

            let poly = a
                .saturating_mul(x)
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
        /// Returns INITIAL_DIFFICULTY if not set (instead of panicking)
        pub fn get_difficulty() -> u128 {
            Self::current_difficulty().unwrap_or(INITIAL_DIFFICULTY)
        }
    }
}
