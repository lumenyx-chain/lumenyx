//! ZK Cryptographic Primitives for LUMENYX Privacy
//! 
//! Full BN254 pairing-based Groth16 verification on-chain.
//! Proof generation happens off-chain using the lumenyx-zk CLI tool.

use sp_core::H256;

/// Hash two H256 values together (for Merkle tree)
pub fn hash_pair(left: H256, right: H256) -> H256 {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left.as_bytes());
    data[32..].copy_from_slice(right.as_bytes());
    H256::from_slice(&sp_io::hashing::blake2_256(&data))
}

/// Groth16 ZK Proof Verifier with FULL BN254 Pairing
/// 
/// This is a REAL cryptographic verifier that performs:
/// 1. Elliptic curve point parsing and validation
/// 2. BN254 optimal ate pairing computation
/// 3. Groth16 verification equation check
/// 
/// Security: Mathematically proves knowledge of secret without revealing it
pub struct Groth16Verifier;

impl Groth16Verifier {
    /// Verify unshield proof using full BN254 pairing
    /// 
    /// Verifies: e(A, B) = e(α, β) · e(L, γ) · e(C, δ)
    /// 
    /// This is REAL cryptographic verification - fake proofs will be REJECTED.
    pub fn verify_unshield(
        vk_bytes: &[u8],
        proof_bytes: &[u8],
        nullifier: H256,
        root: H256,
        amount: u128,
    ) -> bool {
        // Structural validation first (fast reject)
        if vk_bytes.len() < 512 || proof_bytes.len() < 256 {
            return false;
        }
        
        // FULL CRYPTOGRAPHIC VERIFICATION using BN254 pairing
        crate::bn254::verify_unshield_proof(vk_bytes, proof_bytes, nullifier, root, amount)
    }
    
    /// Verify shielded transfer proof using full BN254 pairing
    pub fn verify_transfer(
        vk_bytes: &[u8],
        proof_bytes: &[u8],
        nullifier: H256,
        new_commitment: H256,
        root: H256,
        amount: u128,
    ) -> bool {
        if vk_bytes.len() < 512 || proof_bytes.len() < 256 {
            return false;
        }
        
        crate::bn254::verify_transfer_proof(vk_bytes, proof_bytes, nullifier, new_commitment, root, amount)
    }
}
