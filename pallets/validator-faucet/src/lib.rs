//! # Validator Faucet Pallet
//!
//! Permissionless validator bootstrap for LUMENYX.
//! Anyone can claim a small amount of LUMENYX to pay for session.setKeys() fee.
//!
//! ## How it works:
//! 1. New user generates keys
//! 2. Calls claim_for_validator() with PoW proof (UNSIGNED - no account needed!)
//! 3. Receives 2 LUMENYX (enough for ED + setKeys fee)
//! 4. Can now call session.setKeys() and become a validator
//!
//! ## Security:
//! - ValidateUnsigned: No account needed to claim
//! - PoW required: Prevents spam (18-bit difficulty = ~2 seconds)
//! - One claim per account: Cannot claim twice
//! - Max claims per block: 5
//! - Pool is finite: 5000 LUMENYX total = 2,500 claims max
//! - Immutable: No admin, no governance, founder disappears

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_io::hashing::blake2_256;
    use sp_runtime::traits::AccountIdConversion;
    use sp_std::vec::Vec;

    /// The pallet's configuration trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Amount given per claim (2 LUMENYX = 2_000_000_000_000 planck with 12 decimals)
        #[pallet::constant]
        type ClaimAmount: Get<u128>;
        
        /// PoW difficulty (number of leading zero bits required)
        #[pallet::constant]
        type PowDifficulty: Get<u32>;
        
        /// Max claims per block
        #[pallet::constant]
        type MaxClaimsPerBlock: Get<u32>;
    }

    /// Pallet ID for the faucet pool account
    pub const PALLET_ID: PalletId = PalletId(*b"valifauc");

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Accounts that have already claimed
    #[pallet::storage]
    #[pallet::getter(fn claimed)]
    pub type Claimed<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// Total number of claims made
    #[pallet::storage]
    #[pallet::getter(fn total_claims)]
    pub type TotalClaims<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Claims in current block
    #[pallet::storage]
    #[pallet::getter(fn claims_this_block)]
    pub type ClaimsThisBlock<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Someone claimed from the faucet
        Claimed { who: T::AccountId, amount: u128 },
        /// Faucet is empty
        FaucetEmpty,
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account has already claimed
        AlreadyClaimed,
        /// Invalid proof of work
        InvalidPow,
        /// Faucet pool is empty
        FaucetEmpty,
        /// Too many claims this block
        TooManyClaimsThisBlock,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Reset claims per block counter
            ClaimsThisBlock::<T>::put(0u32);
            Weight::from_parts(1_000, 0)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Claim LUMENYX from faucet to become a validator
        /// 
        /// This is called as an UNSIGNED transaction - no account needed!
        /// Requires proof-of-work to prevent spam.
        ///
        /// Parameters:
        /// - `target`: Account to receive the LUMENYX
        /// - `nonce`: Random nonce used for PoW
        /// - `pow_hash`: The PoW hash (must have required leading zeros)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(100_000_000, 0))]
        pub fn claim_for_validator(
            origin: OriginFor<T>,
            target: T::AccountId,
            nonce: u64,
            pow_hash: H256,
        ) -> DispatchResult {
            // This must be unsigned
            ensure_none(origin)?;
            
            // Check not already claimed
            ensure!(!Claimed::<T>::get(&target), Error::<T>::AlreadyClaimed);
            
            // Check claims this block limit
            let claims_this_block = ClaimsThisBlock::<T>::get();
            ensure!(claims_this_block < T::MaxClaimsPerBlock::get(), Error::<T>::TooManyClaimsThisBlock);
            
            // Verify PoW
            ensure!(Self::verify_pow(&target, nonce, pow_hash), Error::<T>::InvalidPow);
            
            // Get faucet account
            let faucet_account = PALLET_ID.into_account_truncating();
            
            // Get claim amount
            let amount = T::ClaimAmount::get();
            
            // Check faucet has enough balance
            let faucet_balance = pallet_balances::Pallet::<T>::free_balance(&faucet_account);
            let faucet_balance_u128: u128 = faucet_balance.try_into().unwrap_or(0);
            ensure!(faucet_balance_u128 >= amount, Error::<T>::FaucetEmpty);
            
            // Transfer from faucet to target
            let amount_balance: T::Balance = amount.try_into().unwrap_or_else(|_| 0u32.into());
            pallet_balances::Pallet::<T>::transfer(
                &faucet_account,
                &target,
                amount_balance,
                ExistenceRequirement::AllowDeath,
            )?;
            
            // Mark as claimed
            Claimed::<T>::insert(&target, true);
            
            // Increment counters
            TotalClaims::<T>::mutate(|c| *c = c.saturating_add(1));
            ClaimsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));
            
            // Emit event
            Self::deposit_event(Event::Claimed { who: target, amount });
            
            log::info!(
                target: "runtime::validator-faucet",
                "✅ Validator faucet claim successful! Total claims: {}",
                TotalClaims::<T>::get()
            );
            
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the faucet pool account
        pub fn faucet_account() -> T::AccountId {
            PALLET_ID.into_account_truncating()
        }
        
        /// Get current faucet balance
        pub fn faucet_balance() -> T::Balance {
            pallet_balances::Pallet::<T>::free_balance(&Self::faucet_account())
        }
        
        /// Verify proof of work
        /// PoW = blake2_256(target_account ++ nonce) must have N leading zero bits
        fn verify_pow(target: &T::AccountId, nonce: u64, provided_hash: H256) -> bool {
            // Compute expected hash
            let mut data = target.encode();
            data.extend_from_slice(&nonce.to_le_bytes());
            let computed_hash = blake2_256(&data);
            
            // Check provided hash matches computed
            if provided_hash.as_bytes() != &computed_hash {
                return false;
            }
            
            // Check leading zeros (difficulty)
            let difficulty = T::PowDifficulty::get();
            Self::has_leading_zeros(&computed_hash, difficulty)
        }
        
        /// Check if hash has required number of leading zero bits
        fn has_leading_zeros(hash: &[u8; 32], required_zeros: u32) -> bool {
            let mut zeros = 0u32;
            for byte in hash.iter() {
                if *byte == 0 {
                    zeros += 8;
                } else {
                    zeros += byte.leading_zeros();
                    break;
                }
                if zeros >= required_zeros {
                    return true;
                }
            }
            zeros >= required_zeros
        }
    }

    /// Validate unsigned transactions
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::claim_for_validator { target, nonce, pow_hash } => {
                    // Quick checks (must be very cheap!)
                    
                    // 1. Not already claimed
                    if Claimed::<T>::get(target) {
                        return InvalidTransaction::Custom(1).into();
                    }
                    
                    // 2. Claims this block limit
                    if ClaimsThisBlock::<T>::get() >= T::MaxClaimsPerBlock::get() {
                        return InvalidTransaction::Custom(2).into();
                    }
                    
                    // 3. Verify PoW (this is the spam protection!)
                    if !Self::verify_pow(target, *nonce, *pow_hash) {
                        return InvalidTransaction::Custom(3).into();
                    }
                    
                    // 4. Check faucet has balance
                    let faucet_account: T::AccountId = PALLET_ID.into_account_truncating();
                    let balance = pallet_balances::Pallet::<T>::free_balance(&faucet_account);
                    let balance_u128: u128 = balance.try_into().unwrap_or(0);
                    if balance_u128 < T::ClaimAmount::get() {
                        return InvalidTransaction::Custom(4).into();
                    }
                    
                    // Valid!
                    ValidTransaction::with_tag_prefix("ValidatorFaucet")
                        .priority(100)
                        .and_provides((target, nonce))
                        .longevity(3)
                        .propagate(true)
                        .build()
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
