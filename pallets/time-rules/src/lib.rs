#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32BitUnsigned, SaturatedConversion};

    pub const MTP_WINDOW: u32 = 11;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Must match pallet_timestamp::Config::Moment in runtime (u64).
        type Moment: Parameter + AtLeast32BitUnsigned + Copy + MaxEncodedLen;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Ring buffer index.
    #[pallet::storage]
    pub type TsIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Last 11 timestamps (ms) as ring buffer.
    #[pallet::storage]
    pub type LastTimestamps<T: Config> =
        StorageValue<_, BoundedVec<u64, ConstU32<MTP_WINDOW>>, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Timestamp must be strictly greater than Median Time Past of last 11 blocks.
        TimestampNotGreaterThanMTP,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {
        /// Call from pallet_timestamp::OnTimestampSet.
        ///
        /// Bootstrap: allow until we have 11 timestamps.
        /// Rule: require now_ms > median(last_11).
        pub fn on_timestamp_set(now: T::Moment) -> DispatchResult {
            let now_ms: u64 = now.saturated_into::<u64>();

            Self::check_mtp(now_ms)?;
            Self::push_ts(now_ms);

            Ok(())
        }

        fn check_mtp(now_ms: u64) -> DispatchResult {
            let v = LastTimestamps::<T>::get();

            if v.len() < MTP_WINDOW as usize {
                // Not enough history yet (bootstrap phase).
                return Ok(());
            }

            let mut a = [0u64; MTP_WINDOW as usize];
            a.copy_from_slice(&v[..MTP_WINDOW as usize]);
            a.sort_unstable();
            let mtp = a[MTP_WINDOW as usize / 2]; // index 5 = median

            ensure!(now_ms > mtp, Error::<T>::TimestampNotGreaterThanMTP);
            Ok(())
        }

        fn push_ts(now_ms: u64) {
            let mut v = LastTimestamps::<T>::get();

            if v.len() < MTP_WINDOW as usize {
                let _ = v.try_push(now_ms);
                LastTimestamps::<T>::put(v);
                return;
            }

            let idx = (TsIndex::<T>::get() % MTP_WINDOW) as usize;
            v[idx] = now_ms;
            LastTimestamps::<T>::put(v);
            TsIndex::<T>::put(TsIndex::<T>::get().wrapping_add(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_noop, assert_ok, derive_impl};
    use sp_runtime::BuildStorage;
    use sp_runtime::traits::IdentityLookup;

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            TimeRules: crate::pallet,
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Test {
        type Block = Block;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
    }

    impl pallet::Config for Test {
        type Moment = u64;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        t.into()
    }

    #[test]
    fn mtp_bootstrap_allows_first_10() {
        new_test_ext().execute_with(|| {
            for i in 0..10u64 {
                assert_ok!(pallet::Pallet::<Test>::on_timestamp_set(1_000 + i));
            }
        });
    }

    #[test]
    fn mtp_rejects_timestamp_not_strictly_greater_than_median() {
        new_test_ext().execute_with(|| {
            // Fill 11 timestamps: 1000..1010 (median=1005)
            for i in 0..11u64 {
                assert_ok!(pallet::Pallet::<Test>::on_timestamp_set(1_000 + i));
            }
            // now == median should fail
            assert_noop!(
                pallet::Pallet::<Test>::on_timestamp_set(1_005),
                pallet::Error::<Test>::TimestampNotGreaterThanMTP
            );
            // now > median should pass
            assert_ok!(pallet::Pallet::<Test>::on_timestamp_set(1_006));
        });
    }

    #[test]
    fn ring_buffer_keeps_len_11() {
        new_test_ext().execute_with(|| {
            for i in 0..25u64 {
                assert_ok!(pallet::Pallet::<Test>::on_timestamp_set(10_000 + i));
            }
            let v = pallet::LastTimestamps::<Test>::get();
            assert_eq!(v.len(), 11);
        });
    }
}
