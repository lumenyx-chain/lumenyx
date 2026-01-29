//! # LUMENYX Difficulty Pallet - ASERT Algorithm
//!
//! Dynamic PoW difficulty adjustment using ASERT (aserti3-2d).
//! Based on Bitcoin Cash's battle-tested implementation.

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

    const RADIX: i128 = 1i128 << 16;
    pub const TARGET_BLOCK_TIME_MS: u64 = 2_500;
    pub const HALF_LIFE_MS: u64 = 60_000;
    pub const INITIAL_DIFFICULTY: u128 = 1;
    pub const MIN_DIFFICULTY: u128 = 1;
    pub const MAX_DIFFICULTY: u128 = u128::MAX;
    pub const MIN_SOLVE_TIME_MS: u64 = 50;
    pub const MAX_SOLVE_TIME_MS: u64 = 25_000;

    // ============================================
    // HARD FORK (NO NEW STORAGE)
    // ============================================

    /// Hard fork height for ASERT fix (must be > current height).
    pub const FORK_HEIGHT: u64 = 125_000;

    /// Safety valve (post-fork): if observed gap since last effective time is huge -> MIN difficulty.
    pub const SAFETY_VALVE_MS: u64 = 600_000; // 10 minutes

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct AnchorInfo<BlockNumber> {
        pub anchor_height: BlockNumber,
        pub anchor_parent_time_ms: u64,
        pub anchor_difficulty: u128,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ============================================
    // STORAGE (ONLY EXISTING ON MAINNET)
    // ============================================

    #[pallet::storage]
    #[pallet::getter(fn initialized)]
    pub type Initialized<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_difficulty)]
    pub type CurrentDifficulty<T: Config> = StorageValue<_, u128, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_effective_time_ms)]
    pub type LastEffectiveTimeMs<T: Config> = StorageValue<_, u64, ValueQuery>;

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
            if !Initialized::<T>::get() {
                CurrentDifficulty::<T>::put(self.initial_difficulty);
                LastEffectiveTimeMs::<T>::put(0);
                Anchor::<T>::kill();
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DifficultyUpdated {
            block_number: BlockNumberFor<T>,
            old_difficulty: u128,
            new_difficulty: u128,
        },
        AnchorSet {
            height: BlockNumberFor<T>,
            parent_time_ms: u64,
            difficulty: u128,
        },
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_number: BlockNumberFor<T>) {
            let now_ms: u64 = pallet_timestamp::Pallet::<T>::get().saturated_into::<u64>();

            // Initialize effective time if needed (legacy storage)
            let mut prev_eff_ms = LastEffectiveTimeMs::<T>::get();
            if prev_eff_ms == 0 {
                prev_eff_ms = now_ms.saturating_sub(TARGET_BLOCK_TIME_MS);
                LastEffectiveTimeMs::<T>::put(prev_eff_ms);
            }

            // Observed real delta since last effective time (not clamped yet)
            let observed_solve_ms = now_ms.saturating_sub(prev_eff_ms);

            // Read difficulty
            let old_difficulty = match CurrentDifficulty::<T>::get() {
                Some(d) => d,
                None => {
                    CurrentDifficulty::<T>::put(INITIAL_DIFFICULTY);
                    Initialized::<T>::put(true);
                    INITIAL_DIFFICULTY
                }
            };

            // Anchor
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
                        parent_time_ms: prev_eff_ms,
                        difficulty: a.anchor_difficulty,
                    });
                    a
                }
            };

            let bn_u64: u64 = block_number.saturated_into::<u64>();
            let new_difficulty: u128;

            if bn_u64 < FORK_HEIGHT {
                // ==========================
                // PRE-FORK: ORIGINAL LOGIC
                // ==========================
                let mut solve_ms = observed_solve_ms;

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
                // POST-FORK: FIXED LOGIC (NO NEW STORAGE)
                // ==========================

                // Safety valve using observed delta since last effective time.
                if observed_solve_ms > SAFETY_VALVE_MS {
                    new_difficulty = MIN_DIFFICULTY;
                    // Reset effective time to avoid repeatedly tripping the valve
                    LastEffectiveTimeMs::<T>::put(now_ms);
                } else {
                    // Update effective time WITHOUT MAX clamp (so effective time can catch up).
                    // Keep only the MIN clamp to prevent "backdating" from collapsing time.
                    let mut solve_ms = observed_solve_ms;
                    if solve_ms < MIN_SOLVE_TIME_MS {
                        solve_ms = MIN_SOLVE_TIME_MS;
                    }
                    let eff_now_ms = prev_eff_ms.saturating_add(solve_ms);
                    LastEffectiveTimeMs::<T>::put(eff_now_ms);

                    // Critical fix: for ASERT evaluation, use REAL timestamp (now_ms),
                    // not the clamped effective time.
                    new_difficulty =
                        Self::calculate_asert_difficulty_postfork(&anchor, block_number, now_ms);
                }
            }

            CurrentDifficulty::<T>::put(new_difficulty);

            Self::deposit_event(Event::DifficultyUpdated {
                block_number,
                old_difficulty,
                new_difficulty,
            });

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
        }
    }

    impl<T: Config> Pallet<T> {
        // -------------------------
        // Legacy difficulty function
        // -------------------------
        fn calculate_asert_difficulty_legacy(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64,
        ) -> u128 {
            Self::calculate_asert_difficulty_inner(anchor, eval_height, eval_time_ms, false)
        }

        // -------------------------
        // Post-fork difficulty function
        // -------------------------
        fn calculate_asert_difficulty_postfork(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64,
        ) -> u128 {
            Self::calculate_asert_difficulty_inner(anchor, eval_height, eval_time_ms, true)
        }

        fn calculate_asert_difficulty_inner(
            anchor: &AnchorInfo<BlockNumberFor<T>>,
            eval_height: BlockNumberFor<T>,
            eval_time_ms: u64,
            robust_pow2: bool,
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

            let (num_shifts, factor_q16) = if robust_pow2 {
                Self::approx_pow2_fixed_robust(exponent_fixed)
            } else {
                Self::approx_pow2_fixed_legacy(exponent_fixed)
            };

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

        // -------------------------
        // Legacy pow2 (unchanged)
        // -------------------------
        fn approx_pow2_fixed_legacy(exponent_fixed: i128) -> (i128, u128) {
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

        // -------------------------
        // Robust pow2 (post-fork)
        // -------------------------
        fn approx_pow2_fixed_robust(exponent_fixed: i128) -> (i128, u128) {
            let mut num_shifts: i128 = exponent_fixed >> 16;
            let mut frac: i128 = exponent_fixed - num_shifts * RADIX;

            // normalize frac into [0, RADIX)
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

        fn clamp_difficulty(d: u128) -> u128 {
            if d < MIN_DIFFICULTY {
                MIN_DIFFICULTY
            } else if d > MAX_DIFFICULTY {
                MAX_DIFFICULTY
            } else {
                d
            }
        }

        pub fn get_difficulty() -> u128 {
            Self::current_difficulty().unwrap_or(INITIAL_DIFFICULTY)
        }
    }
}
