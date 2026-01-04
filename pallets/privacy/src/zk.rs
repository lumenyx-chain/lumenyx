//! ZK Cryptographic Primitives for LUMENYX Privacy
//!
//! Uses Poseidon hash for Merkle tree (ZK-friendly)
//! Groth16 verification for ZK proofs

use sp_core::H256;
use crate::bn254::{poseidon_hash_pair, h256_to_fp, fp_to_h256, Fp};

/// Hash two H256 values together using Poseidon (for Merkle tree)
/// This MUST match the off-chain ZK circuit implementation
pub fn hash_pair(left: H256, right: H256) -> H256 {
    let left_fp = h256_to_fp(left).unwrap_or_else(|| Fp::from_bytes(&[0u8; 32]).unwrap());
    let right_fp = h256_to_fp(right).unwrap_or_else(|| Fp::from_bytes(&[0u8; 32]).unwrap());
    let result = poseidon_hash_pair(left_fp, right_fp);
    fp_to_h256(&result)
}

/// Groth16 ZK Proof Verifier
///
/// Verifies zero-knowledge proofs for private transactions
pub struct Groth16Verifier;

impl Groth16Verifier {
    /// Verify unshield proof
    pub fn verify_unshield(
        vk_bytes: &[u8],
        proof_bytes: &[u8],
        nullifier: H256,
        root: H256,
        amount: u128,
    ) -> bool {
        // Structural validation
        if vk_bytes.len() < 256 || proof_bytes.len() < 128 {
            return false;
        }

        // Verify nullifier and root are non-zero
        if nullifier == H256::zero() || root == H256::zero() {
            return false;
        }

        // Verify amount is reasonable
        if amount == 0 || amount > 21_000_000_000_000_000_000 {
            return false;
        }

        // Call full BN254 verification
        crate::bn254::verify_groth16_proof(vk_bytes, proof_bytes, nullifier, root, amount)
    }

    /// Verify shielded transfer proof
    pub fn verify_transfer(
        vk_bytes: &[u8],
        proof_bytes: &[u8],
        nullifier: H256,
        new_commitment: H256,
        root: H256,
        amount: u128,
    ) -> bool {
        // Structural validation
        if vk_bytes.len() < 256 || proof_bytes.len() < 128 {
            return false;
        }

        // Verify inputs are non-zero
        if nullifier == H256::zero() || root == H256::zero() || new_commitment == H256::zero() {
            return false;
        }

        // Verify amount is reasonable
        if amount == 0 || amount > 21_000_000_000_000_000_000 {
            return false;
        }

        // Call full BN254 verification
        crate::bn254::verify_transfer_proof(vk_bytes, proof_bytes, nullifier, new_commitment, root, amount)
    }
}
