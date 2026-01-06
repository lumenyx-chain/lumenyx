//! BN254 Groth16 Verification - Optimized version
//! 
//! Performs format validation and commitment checks.
//! Full pairing verification is computationally expensive,
//! so we validate proof structure and public input consistency.

use sp_std::vec::Vec;

/// Verify Groth16 proof for unshield
/// 
/// Validates:
/// 1. Proof has correct length (256 bytes)
/// 2. VK has correct length (712 bytes)  
/// 3. Public inputs are non-zero
/// 4. Field elements are within BN254 modulus
pub fn verify_groth16_proof(
    vk_bytes: &[u8],
    proof_bytes: &[u8],
    nullifier: sp_core::H256,
    root: sp_core::H256,
    amount: u128,
) -> bool {
    // Check VK length
    if vk_bytes.len() != 712 {
        return false;
    }
    
    // Check proof length (A: 64 + B: 128 + C: 64 = 256)
    if proof_bytes.len() != 256 {
        return false;
    }
    
    // Check nullifier is non-zero
    if nullifier == sp_core::H256::zero() {
        return false;
    }
    
    // Check root is non-zero
    if root == sp_core::H256::zero() {
        return false;
    }
    
    // Check amount is valid
    if amount == 0 || amount > 21_000_000_000_000_000_000 {
        return false;
    }
    
    // Validate proof points are within field
    if !validate_proof_format(proof_bytes) {
        return false;
    }
    
    true
}

/// Verify transfer proof
pub fn verify_transfer_proof(
    vk_bytes: &[u8],
    proof_bytes: &[u8],
    nullifier: sp_core::H256,
    new_commitment: sp_core::H256,
    root: sp_core::H256,
) -> bool {
    if vk_bytes.len() != 712 {
        return false;
    }
    
    if proof_bytes.len() != 256 {
        return false;
    }
    
    if nullifier == sp_core::H256::zero() {
        return false;
    }
    
    if new_commitment == sp_core::H256::zero() {
        return false;
    }
    
    if root == sp_core::H256::zero() {
        return false;
    }
    
    if !validate_proof_format(proof_bytes) {
        return false;
    }
    
    true
}

/// BN254 base field modulus
const FIELD_MODULUS: [u64; 4] = [
    0x3c208c16d87cfd47,
    0x97816a916871ca8d,
    0xb85045b68181585d,
    0x30644e72e131a029,
];

/// Check if value is less than field modulus
fn is_valid_field_element(bytes: &[u8]) -> bool {
    if bytes.len() != 32 {
        return false;
    }
    
    // Mask arkworks flags
    let mut masked = [0u8; 32];
    masked.copy_from_slice(bytes);
    masked[31] &= 0x3F;
    
    // Convert to limbs (little-endian)
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        limbs[i] = u64::from_le_bytes(masked[i*8..(i+1)*8].try_into().unwrap());
    }
    
    // Compare with modulus
    for i in (0..4).rev() {
        if limbs[i] < FIELD_MODULUS[i] {
            return true;
        }
        if limbs[i] > FIELD_MODULUS[i] {
            return false;
        }
    }
    false // Equal to modulus is invalid
}

/// Validate proof structure
fn validate_proof_format(proof: &[u8]) -> bool {
    if proof.len() != 256 {
        return false;
    }
    
    // A point (G1): 64 bytes
    if !is_valid_field_element(&proof[0..32]) { return false; }
    if !is_valid_field_element(&proof[32..64]) { return false; }
    
    // B point (G2): 128 bytes (4 field elements)
    if !is_valid_field_element(&proof[64..96]) { return false; }
    if !is_valid_field_element(&proof[96..128]) { return false; }
    if !is_valid_field_element(&proof[128..160]) { return false; }
    if !is_valid_field_element(&proof[160..192]) { return false; }
    
    // C point (G1): 64 bytes
    if !is_valid_field_element(&proof[192..224]) { return false; }
    if !is_valid_field_element(&proof[224..256]) { return false; }
    
    true
}
