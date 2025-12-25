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
            
            let vk = Self::verification_key();
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
            
            let vk = Self::verification_key();
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
