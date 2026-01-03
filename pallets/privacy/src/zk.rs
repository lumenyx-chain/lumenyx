//! ZK Cryptographic Primitives for LUMENYX Privacy
//!
//! Uses Blake2b for Merkle tree (simple, fast, secure)
//! Groth16 verification for ZK proofs

use sp_core::H256;
use sp_io::hashing::blake2_256;

/// Hash two H256 values together using Blake2 (for Merkle tree)
/// This MUST match the off-chain Python implementation
pub fn hash_pair(left: H256, right: H256) -> H256 {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left.as_bytes());
    data[32..].copy_from_slice(right.as_bytes());
    H256::from(blake2_256(&data))
}

/// Hash inputs for commitment: Blake2(amount || secret || blinding)
pub fn hash_commitment(amount: u128, secret: H256, blinding: H256) -> H256 {
    let mut data = [0u8; 80]; // 16 + 32 + 32
    data[..16].copy_from_slice(&amount.to_le_bytes());
    data[16..48].copy_from_slice(secret.as_bytes());
    data[48..80].copy_from_slice(blinding.as_bytes());
    H256::from(blake2_256(&data))
}

/// Hash for nullifier: Blake2(commitment || secret)
pub fn hash_nullifier(commitment: H256, secret: H256) -> H256 {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(commitment.as_bytes());
    data[32..].copy_from_slice(secret.as_bytes());
    H256::from(blake2_256(&data))
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
