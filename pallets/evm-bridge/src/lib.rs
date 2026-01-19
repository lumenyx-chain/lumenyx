#![cfg_attr(not(feature = "std"), no_std)]

//! LUMENYX EVM Bridge (Substrate <-> EVM)
//!
//! deposit: Signed Substrate account -> EVM H160 (credits mapped AccountId)
//! withdraw: EVM H160 -> Substrate AccountId, authorized via ECDSA signature + nonce

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::Encode;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
        weights::Weight,
    };
    use frame_system::pallet_prelude::*;
    use pallet_evm::AddressMapping;
    use sp_core::{ecdsa, H160};
    use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
    use sp_runtime::traits::Zero;
    use sp_std::vec::Vec;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Native currency (Balances).
        type Currency: Currency<Self::AccountId>;

        /// Same mapping used by pallet-evm (H160 -> AccountId).
        type AddressMapping: pallet_evm::AddressMapping<Self::AccountId>;

        /// Chain id domain-separation (use the same value as EVM chain id).
        #[pallet::constant]
        type EvmChainId: Get<u64>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Anti-replay nonce per EVM address for withdraw signatures.
    #[pallet::storage]
    #[pallet::getter(fn withdraw_nonce)]
    pub type WithdrawNonce<T: Config> = StorageMap<_, Blake2_128Concat, H160, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Deposited {
            from: T::AccountId,
            evm_address: H160,
            amount: BalanceOf<T>,
        },
        Withdrawn {
            evm_address: H160,
            to: T::AccountId,
            amount: BalanceOf<T>,
            nonce: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AmountZero,
        BadNonce,
        InvalidSignature,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Substrate -> EVM deposit.
        ///
        /// Credits the mapped AccountId for `evm_address`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000, 0))]
        pub fn deposit(
            origin: OriginFor<T>,
            evm_address: H160,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResult {
            let from = ensure_signed(origin)?;

            ensure!(!amount.is_zero(), Error::<T>::AmountZero);

            let to = T::AddressMapping::into_account_id(evm_address);

            T::Currency::transfer(&from, &to, amount, ExistenceRequirement::AllowDeath)?;

            Self::deposit_event(Event::Deposited {
                from,
                evm_address,
                amount,
            });
            Ok(())
        }

        /// EVM -> Substrate withdraw.
        ///
        /// Anyone can submit and pay Substrate fee, but funds move only if:
        /// - signature recovers to `evm_address`
        /// - `nonce` matches storage nonce for that evm_address (anti-replay)
        ///
        /// Signing rule (wallet side):
        /// - payload = SCALE("LUMENYX_EVM_BRIDGE_WITHDRAW", chain_id, evm_address, to, amount, nonce)
        /// - digest = keccak256("\x19Ethereum Signed Message:\n32" ++ keccak256(payload))
        /// - sign digest with secp256k1 (recoverable signature 65 bytes r,s,v)
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(120_000, 0))]
        pub fn withdraw(
            origin: OriginFor<T>,
            evm_address: H160,
            to: T::AccountId,
            #[pallet::compact] amount: BalanceOf<T>,
            nonce: u64,
            sig: ecdsa::Signature,
        ) -> DispatchResult {
            let _fee_payer = ensure_signed(origin)?;

            ensure!(!amount.is_zero(), Error::<T>::AmountZero);

            let expected_nonce = WithdrawNonce::<T>::get(evm_address);
            ensure!(nonce == expected_nonce, Error::<T>::BadNonce);

            let chain_id = T::EvmChainId::get();

            let payload = (
                b"LUMENYX_EVM_BRIDGE_WITHDRAW",
                chain_id,
                evm_address,
                &to,
                amount,
                nonce,
            )
                .encode();

            let payload_hash = keccak_256(&payload);
            let digest = eip191_hash_32(payload_hash);

            ensure_evm_sig_matches(evm_address, &digest, &sig)
                .map_err(|_| Error::<T>::InvalidSignature)?;

            // increment nonce before transfer (best practice)
            WithdrawNonce::<T>::insert(evm_address, expected_nonce.saturating_add(1));

            let from = T::AddressMapping::into_account_id(evm_address);
            T::Currency::transfer(&from, &to, amount, ExistenceRequirement::AllowDeath)?;

            Self::deposit_event(Event::Withdrawn {
                evm_address,
                to,
                amount,
                nonce,
            });
            Ok(())
        }
    }

    fn eip191_hash_32(msg32: [u8; 32]) -> [u8; 32] {
        // "\x19Ethereum Signed Message:\n32" || msg32
        let mut v: Vec<u8> = Vec::with_capacity(28 + 32);
        v.extend_from_slice(b"\x19Ethereum Signed Message:\n32");
        v.extend_from_slice(&msg32);
        keccak_256(&v)
    }

    fn ensure_evm_sig_matches(
        expected: H160,
        msg_hash: &[u8; 32],
        sig: &ecdsa::Signature,
    ) -> Result<(), ()> {
        let mut sig_bytes = sig.0;

        // Normalize V: accept 27/28 and 0/1
        let v = sig_bytes[64];
        if v == 27 || v == 28 {
            sig_bytes[64] = v - 27;
        } else if v != 0 && v != 1 {
            // V must be 0, 1, 27, or 28
            return Err(());
        }

        // Recover pubkey (64 bytes, no 0x04 prefix)
        let pubkey64 = secp256k1_ecdsa_recover(&sig_bytes, msg_hash).map_err(|_| ())?;

        let addr = eth_address_from_pubkey64(&pubkey64);
        if addr == expected {
            Ok(())
        } else {
            Err(())
        }
    }

    fn eth_address_from_pubkey64(pubkey64: &[u8; 64]) -> H160 {
        // Ethereum address = last 20 bytes of keccak256(uncompressed_pubkey_without_0x04)
        let h = keccak_256(pubkey64);
        H160::from_slice(&h[12..32])
    }
}
