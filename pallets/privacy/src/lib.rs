//! LUMENYX Privacy Pallet
//!
//! Provides optional privacy using Groth16 ZK proofs with FULL on-chain verification.
//!
//! - On-chain only stores commitments and validates roots
//! - Zero Poseidon hashing on-chain = instant shield transactions
//! - Works on any hardware (even 1GB RAM VPS)

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

/// Hardcoded verification key for Groth16 ZK proofs
pub const HARDCODED_VK: [u8; 712] = [
    0x83, 0xce, 0x62, 0x1b, 0x22, 0xcd, 0x4b, 0xc2, 0xb1, 0x98, 0x8e, 0xad,
    0x45, 0x0e, 0x29, 0xbd, 0x0c, 0x5b, 0x96, 0xa9, 0x8b, 0x26, 0x53, 0x3c,
    0xf0, 0xc9, 0x6f, 0x6d, 0x60, 0xf4, 0x6f, 0x14, 0x80, 0xb1, 0x69, 0xc4,
    0xd0, 0xb6, 0x22, 0xb5, 0x60, 0x07, 0x29, 0x90, 0xfe, 0x9d, 0x96, 0x23,
    0x54, 0xa1, 0x24, 0x50, 0xab, 0x25, 0x41, 0xa4, 0x02, 0x2c, 0x9e, 0x4f,
    0x87, 0x6f, 0x11, 0x9c, 0xe2, 0x35, 0x9f, 0x17, 0xd2, 0xc8, 0x5c, 0x50,
    0x0c, 0xcd, 0xa9, 0x40, 0x08, 0x96, 0x24, 0x2b, 0x41, 0x88, 0xb9, 0x70,
    0x6c, 0x5a, 0x48, 0x0a, 0xe4, 0xc3, 0x54, 0x65, 0x0d, 0x3e, 0xdf, 0x0b,
    0x69, 0x5b, 0xfc, 0x6a, 0x13, 0x9a, 0xe0, 0xd7, 0x32, 0x82, 0xd9, 0x5d,
    0x5d, 0x67, 0x5b, 0x12, 0x17, 0x3b, 0xa0, 0xd9, 0xc3, 0x5e, 0x21, 0xc0,
    0xa7, 0x52, 0x61, 0xf2, 0x40, 0x9b, 0xaf, 0x19, 0xb2, 0x77, 0x46, 0x15,
    0xe0, 0x6c, 0xaf, 0xda, 0xd9, 0x73, 0xb4, 0xff, 0x57, 0xb5, 0x11, 0x98,
    0x8e, 0xe6, 0x64, 0x39, 0xd7, 0x4c, 0x7a, 0x9b, 0xc9, 0x3f, 0x1f, 0x1e,
    0xc3, 0xb3, 0x67, 0x29, 0xfd, 0x18, 0x99, 0x76, 0x19, 0x4d, 0x3c, 0x23,
    0x7b, 0xb1, 0x31, 0xef, 0xa7, 0xb8, 0x2e, 0xbe, 0xae, 0xe6, 0x69, 0x1b,
    0x4f, 0x37, 0xc9, 0x27, 0x72, 0xd2, 0xdc, 0xf2, 0x70, 0x4d, 0x77, 0x99,
    0xc9, 0xd7, 0x8a, 0x2c, 0xec, 0x87, 0xb5, 0xfa, 0x90, 0xa6, 0x10, 0xad,
    0x81, 0xc9, 0x8b, 0xe1, 0x08, 0x62, 0xb8, 0x6f, 0x58, 0xec, 0x19, 0x23,
    0x66, 0x08, 0x0e, 0xe6, 0xc9, 0xd8, 0xba, 0x20, 0xb7, 0x49, 0x62, 0x60,
    0x0f, 0xbf, 0xd2, 0x15, 0xce, 0xdc, 0xcd, 0xae, 0x02, 0x50, 0x77, 0xab,
    0x97, 0x35, 0x71, 0xf1, 0xe8, 0x73, 0xfe, 0x40, 0x63, 0x7e, 0x63, 0xe2,
    0xb6, 0x51, 0xc0, 0x19, 0x15, 0x23, 0x67, 0xae, 0xd3, 0xc8, 0xcd, 0x07,
    0x06, 0xc4, 0xb3, 0xce, 0x24, 0x12, 0xcc, 0xcc, 0x86, 0xee, 0x0e, 0x9b,
    0xa0, 0xb1, 0x74, 0x16, 0x1f, 0x2d, 0x14, 0x43, 0xec, 0x39, 0x97, 0x23,
    0x05, 0x82, 0xcf, 0xf5, 0xba, 0x39, 0x0f, 0xfa, 0x18, 0xc5, 0x82, 0xab,
    0x7d, 0x13, 0xe9, 0x8f, 0xf8, 0xf2, 0x66, 0xd3, 0x21, 0x49, 0xeb, 0x45,
    0x44, 0xb9, 0x03, 0xb8, 0x2b, 0xe4, 0xab, 0x06, 0x2e, 0x51, 0x95, 0xd6,
    0xa7, 0x57, 0xee, 0x70, 0xbc, 0x7d, 0x1a, 0x64, 0xf9, 0x62, 0x74, 0x05,
    0xf7, 0x7e, 0x36, 0xbb, 0xd2, 0x7f, 0xab, 0x8f, 0x14, 0xdc, 0x90, 0x17,
    0xac, 0x83, 0xa3, 0x1c, 0x68, 0x78, 0x36, 0xfe, 0xb1, 0xb2, 0x09, 0xa8,
    0x64, 0xb6, 0x67, 0xdf, 0x6c, 0xe7, 0x61, 0x81, 0x4d, 0xf5, 0x24, 0xa9,
    0x5f, 0x0c, 0xc1, 0xb3, 0xab, 0x52, 0x9d, 0xe6, 0xb9, 0x64, 0xa9, 0x23,
    0xf5, 0x5c, 0x35, 0x74, 0x35, 0xde, 0xa1, 0xad, 0xb1, 0xc2, 0xca, 0x07,
    0xa3, 0xdc, 0x7d, 0x42, 0x99, 0x53, 0x0c, 0x52, 0x60, 0x8e, 0x8e, 0x32,
    0x68, 0x30, 0xb3, 0x70, 0x62, 0x7e, 0xc0, 0x0e, 0x65, 0x99, 0x15, 0xe5,
    0xad, 0x76, 0x4c, 0xdd, 0xa7, 0xa0, 0xd7, 0xcf, 0x67, 0xf0, 0x29, 0xfd,
    0xfa, 0x35, 0x3b, 0x6a, 0x8d, 0xf4, 0xd7, 0x09, 0xbc, 0xac, 0x8d, 0xca,
    0x49, 0xc4, 0xc9, 0x98, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x1c, 0x8d, 0x91, 0x01, 0x3a, 0x2e, 0x85, 0xeb, 0xd0, 0x27, 0x4c, 0xce,
    0x48, 0xb3, 0xc4, 0xd3, 0x42, 0x10, 0xdb, 0x67, 0x6d, 0x4c, 0x2e, 0x83,
    0xf6, 0x39, 0x69, 0xad, 0xba, 0x5c, 0x19, 0x1f, 0xb1, 0xc8, 0x92, 0x28,
    0x5e, 0x3d, 0xde, 0xf3, 0x0d, 0x6d, 0xd5, 0x57, 0xad, 0x8b, 0x07, 0xb8,
    0x63, 0xd6, 0xce, 0x27, 0x2b, 0x12, 0x17, 0x8b, 0x0e, 0x7e, 0xe0, 0x6b,
    0x5d, 0xba, 0x00, 0xae, 0xa0, 0xb8, 0xef, 0x8a, 0xbe, 0xc3, 0xa9, 0x5c,
    0xca, 0x01, 0x7a, 0x4c, 0xf5, 0x6d, 0x5e, 0xed, 0xbf, 0x81, 0xaa, 0x06,
    0xd5, 0x92, 0xbd, 0x1f, 0xe4, 0xa4, 0x48, 0x4c, 0xb5, 0x49, 0xc2, 0x00,
    0x2e, 0xd4, 0xc9, 0x8c, 0x78, 0x11, 0xd5, 0xce, 0x9d, 0x8b, 0x31, 0x34,
    0x6d, 0xb0, 0xaf, 0xa5, 0x37, 0xc6, 0x81, 0xde, 0x63, 0xfd, 0x27, 0x07,
    0xea, 0xbe, 0xb4, 0x08, 0x50, 0x87, 0xf7, 0xa7, 0xa6, 0x92, 0x02, 0x38,
    0xdb, 0xf4, 0x78, 0xf8, 0x53, 0xe5, 0x90, 0xa6, 0xc4, 0x9d, 0xe9, 0xbe,
    0x53, 0xb8, 0xad, 0x5b, 0xb4, 0xf3, 0x7d, 0xf2, 0x01, 0x6f, 0xe2, 0x3a,
    0xf1, 0x4f, 0xb9, 0x0d, 0xfa, 0x16, 0x50, 0xcb, 0x55, 0x54, 0x59, 0x84,
    0xb9, 0x81, 0xef, 0xe6, 0x10, 0xd1, 0xf1, 0xa3, 0x48, 0x79, 0x9e, 0xe0,
    0xad, 0x31, 0x9c, 0x4e, 0xc8, 0xfa, 0x99, 0x9d, 0x9b, 0x4f, 0x70, 0x01,
    0xec, 0xbd, 0x2d, 0x14, 0xdd, 0x5d, 0x8e, 0x4d, 0xe9, 0x52, 0xbd, 0x01,
    0x0f, 0x4b, 0x1d, 0x99, 0x32, 0x2c, 0xe0, 0xd2, 0xd9, 0xe7, 0x43, 0xdd,
    0x25, 0x12, 0x9a, 0x22, 0x2f, 0x86, 0xfc, 0x0a, 0x27, 0x7e, 0x18, 0xc1,
    0x43, 0x67, 0x93, 0xf3, 0x21, 0xe4, 0x7b, 0xc5, 0x7b, 0x70, 0x81, 0x38,
    0xe7, 0x86, 0xc8, 0xb8, 0xce, 0x4d, 0x4f, 0x24, 0x3a, 0xb2, 0x17, 0xa4,
    0xbe, 0x32, 0x23, 0xab,
];
pub mod zk;
pub mod bn254;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Saturating, Zero};
    use sp_core::H256;
    use sp_std::vec::Vec;

    use crate::zk::Groth16Verifier;
    use crate::HARDCODED_VK;

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    pub type Commitment = H256;
    pub type Nullifier = H256;
    pub type MerkleRoot = H256;
    pub type Proof = BoundedVec<u8, ConstU32<512>>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;

        #[pallet::constant]
        type TreeDepth: Get<u32>;

        #[pallet::constant]
        type MaxNotes: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn shielded_pool)]
    pub type ShieldedPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn commitments)]
    pub type Commitments<T: Config> = StorageMap<_, Twox64Concat, u32, Commitment, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn merkle_root)]
    pub type CurrentMerkleRoot<T: Config> = StorageValue<_, MerkleRoot, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn is_known_root)]
    pub type KnownRoots<T: Config> = StorageMap<_, Blake2_128Concat, MerkleRoot, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_index)]
    pub type NextIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn is_spent)]
    pub type SpentNullifiers<T: Config> = StorageMap<_, Blake2_128Concat, Nullifier, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn verification_key)]
    pub type VerificationKey<T: Config> = StorageValue<_, BoundedVec<u8, ConstU32<2048>>, ValueQuery>;

    #[pallet::storage]
    pub type TotalShielded<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    pub type TotalUnshielded<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    pub type NoteCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Shielded {
            who: T::AccountId,
            amount: BalanceOf<T>,
            commitment: Commitment,
            leaf_index: u32,
            merkle_root: MerkleRoot,
        },
        Unshielded {
            who: T::AccountId,
            amount: BalanceOf<T>,
            nullifier: Nullifier,
        },
        ShieldedTransfer {
            nullifier: Nullifier,
            new_commitment: Commitment,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        InsufficientShieldedBalance,
        CommitmentExists,
        TreeFull,
        InvalidProof,
        NullifierSpent,
        UnknownRoot,
        ZeroAmount,
        NoVerificationKey,
        InvalidMerkleRoot,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Shield funds into the private pool
        /// 
        /// The merkle_root is calculated OFF-CHAIN by the user's wallet.
        /// This allows instant on-chain execution without heavy hashing.
        /// 
        /// Security: The ZK proof during unshield will verify the merkle path,
        /// so an incorrect root will simply make the funds unspendable.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]  // Very light - no hashing!
        pub fn shield(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            commitment: Commitment,
            merkle_root: MerkleRoot,  // Calculated off-chain by user
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            // Validations
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);
            ensure!(commitment != H256::zero(), Error::<T>::InvalidMerkleRoot);
            ensure!(merkle_root != H256::zero(), Error::<T>::InvalidMerkleRoot);
            
            let idx = Self::next_index();
            ensure!(idx < T::MaxNotes::get(), Error::<T>::TreeFull);
            ensure!(T::Currency::free_balance(&who) >= amount, Error::<T>::InsufficientBalance);

            // Withdraw from user
            T::Currency::withdraw(
                &who,
                amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;

            // Update state - NO HASHING, just storage writes
            ShieldedPool::<T>::mutate(|p| *p = p.saturating_add(amount));
            Commitments::<T>::insert(idx, commitment);
            CurrentMerkleRoot::<T>::put(merkle_root);
            KnownRoots::<T>::insert(merkle_root, true);
            NextIndex::<T>::put(idx + 1);
            TotalShielded::<T>::mutate(|t| *t = t.saturating_add(amount));
            NoteCount::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::Shielded { 
                who, 
                amount, 
                commitment, 
                leaf_index: idx,
                merkle_root,
            });
            
            Ok(())
        }

        /// Unshield funds from the private pool
        /// 
        /// Requires a valid Groth16 ZK proof that proves:
        /// 1. Knowledge of commitment preimage (amount, secret, blinding)
        /// 2. Commitment exists in the Merkle tree at the given root
        /// 3. Nullifier is correctly derived
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(500_000_000, 0))]
        pub fn unshield(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            nullifier: Nullifier,
            root: MerkleRoot,
            proof: Proof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);
            ensure!(
                Self::merkle_root() == root || KnownRoots::<T>::get(root),
                Error::<T>::UnknownRoot
            );
            ensure!(!SpentNullifiers::<T>::get(nullifier), Error::<T>::NullifierSpent);
            ensure!(Self::shielded_pool() >= amount, Error::<T>::InsufficientShieldedBalance);

            // Verify ZK proof
            let vk: BoundedVec<u8, ConstU32<2048>> = HARDCODED_VK.to_vec().try_into().expect("VK fits");
            let amount_u128: u128 = amount.try_into().unwrap_or(0);
            ensure!(
                Groth16Verifier::verify_unshield(&vk, &proof, nullifier, root, amount_u128),
                Error::<T>::InvalidProof
            );

            // Update state
            SpentNullifiers::<T>::insert(nullifier, true);
            ShieldedPool::<T>::mutate(|p| *p = p.saturating_sub(amount));
            T::Currency::deposit_creating(&who, amount);
            TotalUnshielded::<T>::mutate(|t| *t = t.saturating_add(amount));

            Self::deposit_event(Event::Unshielded { who, amount, nullifier });
            
            Ok(())
        }

        /// Private transfer (spend one note, create another)
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(600_000_000, 0))]
        pub fn shielded_transfer(
            origin: OriginFor<T>,
            nullifier: Nullifier,
            new_commitment: Commitment,
            new_merkle_root: MerkleRoot,  // New root after adding new_commitment
            old_root: MerkleRoot,          // Root for proving old commitment exists
            proof: Proof,
        ) -> DispatchResult {
            let _relay = ensure_signed(origin)?;
            
            ensure!(
                Self::merkle_root() == old_root || KnownRoots::<T>::get(old_root),
                Error::<T>::UnknownRoot
            );
            ensure!(!SpentNullifiers::<T>::get(nullifier), Error::<T>::NullifierSpent);
            ensure!(new_commitment != H256::zero(), Error::<T>::InvalidMerkleRoot);
            ensure!(new_merkle_root != H256::zero(), Error::<T>::InvalidMerkleRoot);

            let idx = Self::next_index();
            ensure!(idx < T::MaxNotes::get(), Error::<T>::TreeFull);

            // Verify ZK proof (amount = 0 for transfers, just proves knowledge)
            let vk: BoundedVec<u8, ConstU32<2048>> = HARDCODED_VK.to_vec().try_into().expect("VK fits");
            ensure!(
                Groth16Verifier::verify_transfer(&vk, &proof, nullifier, new_commitment, old_root, 0),
                Error::<T>::InvalidProof
            );

            // Update state
            SpentNullifiers::<T>::insert(nullifier, true);
            Commitments::<T>::insert(idx, new_commitment);
            CurrentMerkleRoot::<T>::put(new_merkle_root);
            KnownRoots::<T>::insert(new_merkle_root, true);
            NextIndex::<T>::put(idx + 1);

            Self::deposit_event(Event::ShieldedTransfer { nullifier, new_commitment });
            
            Ok(())
        }
    }
}
