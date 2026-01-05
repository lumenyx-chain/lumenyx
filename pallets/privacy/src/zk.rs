//! ZK Cryptographic Primitives for LUMENYX Privacy
//!
//! v3.0: Merkle hashing moved OFF-CHAIN
//! On-chain only verifies Groth16 proofs

use sp_core::H256;

/// Groth16 ZK Proof Verifier
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
        if vk_bytes.len() < 256 || proof_bytes.len() < 128 {
            return false;
        }
        if nullifier == H256::zero() || root == H256::zero() {
            return false;
        }
        if amount == 0 || amount > 21_000_000_000_000_000_000 {
            return false;
        }
        crate::bn254::verify_groth16_proof(vk_bytes, proof_bytes, nullifier, root, amount)
    }
    
    /// Verify shielded transfer proof
    pub fn verify_transfer(
        vk_bytes: &[u8],
        proof_bytes: &[u8],
        nullifier: H256,
        new_commitment: H256,
        root: H256,
        _amount: u128,
    ) -> bool {
        if vk_bytes.len() < 256 || proof_bytes.len() < 128 {
            return false;
        }
        if nullifier == H256::zero() || root == H256::zero() || new_commitment == H256::zero() {
            return false;
        }
        // For transfers, we verify the proof but amount is not checked on-chain
        crate::bn254::verify_transfer_proof(vk_bytes, proof_bytes, nullifier, new_commitment, root, 0)
    }
}
