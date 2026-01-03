//! # LUMENYX Privacy Pallet v2.1 - Full ZK Privacy with BN254 Pairing
//!
//! Provides optional privacy using Groth16 ZK proofs with FULL on-chain verification.
//! 
//! ## Security Model
//! - Proof generation: off-chain (lumenyx-zk CLI with arkworks)
//! - Proof verification: on-chain with REAL BN254 pairing (no trusted validators needed)
//! 
//! ## Cryptographic Components
//! - BN254 elliptic curve arithmetic (Fp, Fp2, Fp6, Fp12 tower)
//! - Optimal Ate pairing implementation
//! - Full Groth16 verification equation: e(A,B) = e(α,β)·e(L,γ)·e(C,δ)

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;


/// Hardcoded verification key for Groth16 ZK proofs
pub const HARDCODED_VK: [u8; 360] = [
    0x01, 0x0a, 0x4d, 0xa7, 0x5a, 0x69, 0x5a, 0x76, 0x2b, 0x32, 0x07, 0x46,
    0x01, 0xe7, 0x39, 0x59, 0x32, 0x71, 0xda, 0x99, 0xe3, 0x48, 0x3f, 0x23,
    0xde, 0x0f, 0xa7, 0x1d, 0x27, 0x54, 0x27, 0xab, 0xad, 0xf8, 0xca, 0x37,
    0x0e, 0x8f, 0xd8, 0xdc, 0x04, 0x8f, 0x7a, 0x95, 0x28, 0x5d, 0xc6, 0x7b,
    0x32, 0xf4, 0xc1, 0xc4, 0xb3, 0x56, 0x9b, 0x2b, 0x0c, 0x9c, 0xe8, 0x43,
    0x96, 0x55, 0xd3, 0x17, 0xfe, 0xa7, 0x97, 0x1d, 0x4b, 0x95, 0x5f, 0xe4,
    0x19, 0x74, 0x22, 0x09, 0x28, 0xc4, 0x50, 0x15, 0xa9, 0xd6, 0xc1, 0x02,
    0xbe, 0x70, 0x38, 0x7f, 0x94, 0x0e, 0xf3, 0xf6, 0xb0, 0xe6, 0x22, 0x1b,
    0x93, 0x78, 0x97, 0xcf, 0x3b, 0x47, 0x4f, 0x1a, 0xf3, 0xe0, 0xa5, 0xda,
    0xa2, 0x0e, 0x70, 0x35, 0x18, 0xa6, 0x50, 0x03, 0x6f, 0xf4, 0x7a, 0x35,
    0x83, 0x2a, 0x94, 0x98, 0xa9, 0x57, 0x45, 0x06, 0x75, 0xc8, 0xaf, 0xbc,
    0xbc, 0x50, 0xa5, 0xe0, 0x68, 0x56, 0xc8, 0xbc, 0x58, 0xf9, 0x61, 0xb5,
    0xd4, 0x34, 0xa5, 0x44, 0x59, 0x34, 0xfd, 0x46, 0xc4, 0x99, 0x68, 0xe7,
    0x94, 0x2e, 0xee, 0xa4, 0x9a, 0x22, 0x8e, 0xbb, 0xf7, 0xa1, 0xf4, 0x5b,
    0xc4, 0xff, 0xcd, 0x4f, 0x99, 0xa6, 0x95, 0x95, 0xf4, 0x30, 0x97, 0xeb,
    0xc1, 0xff, 0x88, 0x35, 0x3a, 0x2f, 0x88, 0x94, 0x87, 0x5c, 0x34, 0x24,
    0xe9, 0x34, 0x9d, 0x85, 0xe5, 0x4a, 0xd3, 0xbd, 0xc6, 0x40, 0xdd, 0x96,
    0xc7, 0xb2, 0x05, 0xe6, 0x4b, 0xad, 0x8e, 0x13, 0xc3, 0x28, 0x87, 0xe8,
    0x57, 0x05, 0x2b, 0x6e, 0x35, 0x0f, 0x3d, 0x1a, 0x04, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x27, 0xbb, 0x33, 0xe2, 0xcb, 0x86, 0x61, 0x67,
    0xd1, 0xa1, 0xa3, 0xf5, 0x94, 0x25, 0x70, 0xd9, 0x7d, 0x28, 0xf3, 0xc0,
    0x95, 0x8a, 0xcc, 0x7a, 0x9b, 0x66, 0xf0, 0xd0, 0x84, 0xa0, 0x2e, 0x80,
    0x56, 0xe4, 0x72, 0x8f, 0x0d, 0x44, 0x50, 0xdd, 0xf4, 0x4c, 0xbe, 0xe0,
    0xf4, 0xd2, 0x71, 0x06, 0xeb, 0xfa, 0x4d, 0x01, 0xf2, 0x57, 0xff, 0x9b,
    0x7f, 0x62, 0xa0, 0xce, 0x6a, 0xa5, 0x12, 0x0f, 0x96, 0x61, 0x06, 0x5f,
    0xb4, 0x34, 0x13, 0x57, 0x2d, 0x6b, 0x2d, 0xbf, 0x8d, 0xa1, 0xfa, 0x31,
    0x12, 0x34, 0xc5, 0x0b, 0x5e, 0x30, 0x2d, 0xdd, 0x75, 0x61, 0xc4, 0x5f,
    0x76, 0x9d, 0x48, 0x15, 0x28, 0x10, 0x80, 0x43, 0xa0, 0x50, 0x4c, 0xb2,
    0x7c, 0xd1, 0xf1, 0xf2, 0x04, 0xb1, 0xaa, 0xd5, 0x7b, 0x41, 0xf7, 0xda,
    0xb8, 0x63, 0xdc, 0x87, 0x42, 0x5e, 0xcc, 0xa0, 0xe4, 0xb9, 0x44, 0x8a,
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
    pub type Proof = BoundedVec<u8, ConstU32<256>>;

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
        VerificationKeySet { size: u32 },
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
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(100_000, 0))]
        pub fn shield(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            commitment: Commitment,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);
            
            let idx = Self::next_index();
            ensure!(idx < T::MaxNotes::get(), Error::<T>::TreeFull);
            
            ensure!(T::Currency::free_balance(&who) >= amount, Error::<T>::InsufficientBalance);
            
            T::Currency::withdraw(
                &who,
                amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::KeepAlive,
            )?;
            
            ShieldedPool::<T>::mutate(|p| *p = p.saturating_add(amount));
            Commitments::<T>::insert(idx, commitment);
            NextIndex::<T>::put(idx + 1);
            
            let new_root = Self::compute_merkle_root();
            CurrentMerkleRoot::<T>::put(new_root);
            KnownRoots::<T>::insert(new_root, true);
            
            TotalShielded::<T>::mutate(|t| *t = t.saturating_add(amount));
            NoteCount::<T>::mutate(|n| *n = n.saturating_add(1));
            
            Self::deposit_event(Event::Shielded { who, amount, commitment, leaf_index: idx });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(500_000, 0))]
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
            
            let vk: BoundedVec<u8, ConstU32<2048>> = HARDCODED_VK.to_vec().try_into().expect("VK fits"); // Hardcoded ZK key
            ensure!(!vk.is_empty(), Error::<T>::NoVerificationKey);
            
            let amount_u128: u128 = amount.try_into().unwrap_or(0);
            ensure!(
                Groth16Verifier::verify_unshield(&vk, &proof, nullifier, root, amount_u128),
                Error::<T>::InvalidProof
            );
            
            SpentNullifiers::<T>::insert(nullifier, true);
            ShieldedPool::<T>::mutate(|p| *p = p.saturating_sub(amount));
            T::Currency::deposit_creating(&who, amount);
            TotalUnshielded::<T>::mutate(|t| *t = t.saturating_add(amount));
            
            Self::deposit_event(Event::Unshielded { who, amount, nullifier });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(600_000, 0))]
        pub fn shielded_transfer(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            nullifier: Nullifier,
            new_commitment: Commitment,
            root: MerkleRoot,
            proof: Proof,
        ) -> DispatchResult {
            let _relay = ensure_signed(origin)?;
            
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);
            ensure!(
                Self::merkle_root() == root || KnownRoots::<T>::get(root),
                Error::<T>::UnknownRoot
            );
            ensure!(!SpentNullifiers::<T>::get(nullifier), Error::<T>::NullifierSpent);
            
            let idx = Self::next_index();
            ensure!(idx < T::MaxNotes::get(), Error::<T>::TreeFull);
            
            let vk: BoundedVec<u8, ConstU32<2048>> = HARDCODED_VK.to_vec().try_into().expect("VK fits"); // Hardcoded ZK key
            ensure!(!vk.is_empty(), Error::<T>::NoVerificationKey);
            
            let amount_u128: u128 = amount.try_into().unwrap_or(0);
            ensure!(
                Groth16Verifier::verify_transfer(&vk, &proof, nullifier, new_commitment, root, amount_u128),
                Error::<T>::InvalidProof
            );
            
            SpentNullifiers::<T>::insert(nullifier, true);
            Commitments::<T>::insert(idx, new_commitment);
            NextIndex::<T>::put(idx + 1);
            
            let new_root = Self::compute_merkle_root();
            CurrentMerkleRoot::<T>::put(new_root);
            KnownRoots::<T>::insert(new_root, true);
            NoteCount::<T>::mutate(|n| *n = n.saturating_add(1));
            
            Self::deposit_event(Event::ShieldedTransfer { nullifier, new_commitment });
            Ok(())
        }

        #[pallet::call_index(10)]
        #[pallet::weight(Weight::from_parts(50_000, 0))]
        pub fn set_verification_key(
            origin: OriginFor<T>,
            vk: BoundedVec<u8, ConstU32<2048>>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let size = vk.len() as u32;
            VerificationKey::<T>::put(vk);
            Self::deposit_event(Event::VerificationKeySet { size });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn compute_merkle_root() -> MerkleRoot {
            let count = Self::next_index();
            if count == 0 {
                return H256::zero();
            }
            
            let depth = T::TreeDepth::get() as usize;
            let size = 1usize << depth;
            
            let mut leaves: Vec<H256> = Vec::new();
            for i in 0..count {
                if let Some(c) = Commitments::<T>::get(i) {
                    leaves.push(c);
                }
            }
            
            while leaves.len() < size {
                leaves.push(H256::zero());
            }
            
            let mut current = leaves;
            while current.len() > 1 {
                let mut next = Vec::new();
                for chunk in current.chunks(2) {
                    let left = chunk[0];
                    let right = if chunk.len() > 1 { chunk[1] } else { H256::zero() };
                    next.push(crate::zk::hash_pair(left, right));
                }
                current = next;
            }
            
            current.first().copied().unwrap_or(H256::zero())
        }
    }
}
