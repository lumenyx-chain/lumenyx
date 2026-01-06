//! # LUMENYX ZK CLI
//!
//! Command-line tool for LUMENYX privacy operations:
//! - Generate verification keys (trusted setup)
//! - Create commitments for shielding
//! - Generate ZK proofs for unshield/transfer
//!
//! ## Usage
//!
//! ```bash
//! # Generate verification key (one-time setup)
//! lumenyx-zk setup --output vk.bin
//!
//! # Create a new shielded note
//! lumenyx-zk commitment --amount 100
//!
//! # Generate proof for unshield
//! lumenyx-zk prove-unshield --secret <hex> --blinding <hex> --amount 100 --merkle-path <file>
//! ```

use ark_bn254::{Bn254, Fr};
use ark_ff::{PrimeField, UniformRand};
use ark_groth16::Groth16;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_r1cs_std::{fields::fp::FpVar, prelude::*};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use clap::{Parser, Subcommand};
use std::fs;
use std::time::Instant;

const TREE_DEPTH: usize = 20;

// ==================== CLI DEFINITION ====================

#[derive(Parser)]
#[command(name = "lumenyx-zk")]
#[command(about = "LUMENYX ZK Privacy CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate proving and verification keys (trusted setup)
    Setup {
        /// Output file for verification key
        #[arg(short, long, default_value = "verification_key.bin")]
        vk_output: String,
        
        /// Output file for proving key
        #[arg(short, long, default_value = "proving_key.bin")]
        pk_output: String,
    },
    
    /// Generate a new commitment for shielding
    Commitment {
        /// Amount to shield
        #[arg(short, long)]
        amount: u64,
        
        /// Optional secret (hex), random if not provided
        #[arg(short, long)]
        secret: Option<String>,
        
        /// Optional blinding factor (hex), random if not provided
        #[arg(short, long)]
        blinding: Option<String>,
    },
    
    /// Generate proof for unshielding
    ProveUnshield {
        /// Amount to unshield
        #[arg(short, long)]
        amount: u64,
        
        /// Secret (hex)
        #[arg(short, long)]
        secret: String,
        
        /// Blinding factor (hex)
        #[arg(short, long)]
        blinding: String,
        
        /// Merkle path file (JSON)
        #[arg(short, long)]
        merkle_path: String,
        
        /// Proving key file
        #[arg(short, long, default_value = "proving_key.bin")]
        pk_file: String,
    },
    
    /// Verify a proof
    Verify {
        /// Proof file
        #[arg(short, long)]
        proof: String,
        
        /// Public inputs file (JSON)
        #[arg(short, long)]
        inputs: String,
        
        /// Verification key file
        #[arg(short, long, default_value = "verification_key.bin")]
        vk_file: String,
    },
    
    /// Test the ZK system
    Test,
}

// ==================== ZK CIRCUIT ====================

/// Poseidon-like hash (MiMC-style)
fn poseidon_hash(inputs: &[Fr]) -> Fr {
    let mut state = Fr::from(0u64);
    for (i, input) in inputs.iter().enumerate() {
        state += input;
        let x2 = state * state;
        let x4 = x2 * x2;
        state = x4 * state;
        state += Fr::from((i + 1) as u64);
    }
    state
}

fn poseidon_hash_var(
    _cs: ConstraintSystemRef<Fr>,
    inputs: &[FpVar<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut state = FpVar::zero();
    for (i, input) in inputs.iter().enumerate() {
        state = &state + input;
        let x2 = &state * &state;
        let x4 = &x2 * &x2;
        state = &x4 * &state;
        state = &state + FpVar::Constant(Fr::from((i + 1) as u64));
    }
    Ok(state)
}

fn merkle_root_var(
    cs: ConstraintSystemRef<Fr>,
    leaf: &FpVar<Fr>,
    path: &[FpVar<Fr>],
    indices: &[Boolean<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut current = leaf.clone();
    for (sibling, is_right) in path.iter().zip(indices.iter()) {
        let left = is_right.select(sibling, &current)?;
        let right = is_right.select(&current, sibling)?;
        current = poseidon_hash_var(cs.clone(), &[left, right])?;
    }
    Ok(current)
}

/// Spend circuit for unshield/transfer
#[derive(Clone)]
pub struct SpendCircuit {
    pub amount: Option<Fr>,
    pub secret: Option<Fr>,
    pub blinding: Option<Fr>,
    pub merkle_path: Vec<Option<Fr>>,
    pub path_indices: Vec<Option<bool>>,
    pub nullifier: Option<Fr>,
    pub merkle_root: Option<Fr>,
    pub public_amount: Option<Fr>,
}

impl SpendCircuit {
    pub fn new(
        amount: Fr,
        secret: Fr,
        blinding: Fr,
        merkle_path: Vec<Fr>,
        path_indices: Vec<bool>,
        nullifier: Fr,
        merkle_root: Fr,
    ) -> Self {
        Self {
            amount: Some(amount),
            secret: Some(secret),
            blinding: Some(blinding),
            merkle_path: merkle_path.into_iter().map(Some).collect(),
            path_indices: path_indices.into_iter().map(Some).collect(),
            nullifier: Some(nullifier),
            merkle_root: Some(merkle_root),
            public_amount: Some(amount),
        }
    }
    
    pub fn empty(depth: usize) -> Self {
        Self {
            amount: None,
            secret: None,
            blinding: None,
            merkle_path: vec![None; depth],
            path_indices: vec![None; depth],
            nullifier: None,
            merkle_root: None,
            public_amount: None,
        }
    }
}

impl ConstraintSynthesizer<Fr> for SpendCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let amount = FpVar::new_witness(cs.clone(), || {
            self.amount.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let secret = FpVar::new_witness(cs.clone(), || {
            self.secret.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let blinding = FpVar::new_witness(cs.clone(), || {
            self.blinding.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let mut path = Vec::new();
        for p in &self.merkle_path {
            path.push(FpVar::new_witness(cs.clone(), || {
                p.ok_or(SynthesisError::AssignmentMissing)
            })?);
        }
        
        let mut indices = Vec::new();
        for i in &self.path_indices {
            indices.push(Boolean::new_witness(cs.clone(), || {
                i.ok_or(SynthesisError::AssignmentMissing)
            })?);
        }
        
        let nullifier_pub = FpVar::new_input(cs.clone(), || {
            self.nullifier.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let merkle_root_pub = FpVar::new_input(cs.clone(), || {
            self.merkle_root.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let public_amount = FpVar::new_input(cs.clone(), || {
            self.public_amount.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        // Commitment
        let commitment = poseidon_hash_var(cs.clone(), &[
            amount.clone(),
            secret.clone(),
            blinding,
        ])?;
        
        // Merkle membership
        let computed_root = merkle_root_var(cs.clone(), &commitment, &path, &indices)?;
        computed_root.enforce_equal(&merkle_root_pub)?;
        
        // Nullifier
        let computed_nullifier = poseidon_hash_var(cs.clone(), &[commitment, secret])?;
        computed_nullifier.enforce_equal(&nullifier_pub)?;
        
        // Amount
        amount.enforce_equal(&public_amount)?;
        
        Ok(())
    }
}

// ==================== HELPERS ====================

fn compute_commitment(amount: Fr, secret: Fr, blinding: Fr) -> Fr {
    poseidon_hash(&[amount, secret, blinding])
}

fn compute_nullifier(commitment: Fr, secret: Fr) -> Fr {
    poseidon_hash(&[commitment, secret])
}

fn build_merkle_tree(leaves: &[Fr]) -> Vec<Vec<Fr>> {
    let mut tree = vec![leaves.to_vec()];
    let mut current = leaves.to_vec();
    while current.len() > 1 {
        let mut next = Vec::new();
        for chunk in current.chunks(2) {
            let left = chunk[0];
            let right = if chunk.len() > 1 { chunk[1] } else { Fr::from(0u64) };
            next.push(poseidon_hash(&[left, right]));
        }
        tree.push(next.clone());
        current = next;
    }
    tree
}

fn get_merkle_path(tree: &[Vec<Fr>], leaf_index: usize) -> (Vec<Fr>, Vec<bool>) {
    let mut path = Vec::new();
    let mut indices = Vec::new();
    let mut idx = leaf_index;
    
    for level in tree.iter().take(tree.len() - 1) {
        let is_right = idx % 2 == 1;
        let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
        let sibling = if sibling_idx < level.len() {
            level[sibling_idx]
        } else {
            Fr::from(0u64)
        };
        path.push(sibling);
        indices.push(is_right);
        idx /= 2;
    }
    
    (path, indices)
}

fn fr_to_hex(f: &Fr) -> String {
    let mut bytes = Vec::new();
    f.serialize_uncompressed(&mut bytes).unwrap();
    hex::encode(bytes)
}

fn hex_to_fr(s: &str) -> Result<Fr, String> {
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    Fr::deserialize_uncompressed(&bytes[..]).map_err(|e| e.to_string())
}

// ==================== MAIN ====================

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Setup { vk_output, pk_output } => {
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║         LUMENYX ZK - TRUSTED SETUP                          ║");
            println!("╚═══════════════════════════════════════════════════════════╝\n");
            
            println!("⚠️  This generates the proving and verification keys.");
            println!("   The verification key will be deployed on-chain.\n");
            
            let start = Instant::now();
            let mut rng = thread_rng();
            
            println!("🔧 Creating empty circuit...");
            let circuit = SpendCircuit::empty(TREE_DEPTH);
            
            println!("🔐 Running trusted setup (this may take a moment)...");
            let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
                .expect("Setup failed");
            
            println!("   ✅ Setup complete in {:?}\n", start.elapsed());
            
            // Serialize PK
            let mut pk_bytes = Vec::new();
            pk.serialize_uncompressed(&mut pk_bytes).expect("PK serialization failed");
            fs::write(&pk_output, &pk_bytes).expect("Failed to write PK");
            println!("📁 Proving key: {} ({} bytes)", pk_output, pk_bytes.len());
            
            // Serialize VK
            let mut vk_bytes = Vec::new();
            vk.serialize_uncompressed(&mut vk_bytes).expect("VK serialization failed");
            fs::write(&vk_output, &vk_bytes).expect("Failed to write VK");
            println!("📁 Verification key: {} ({} bytes)", vk_output, vk_bytes.len());
            
            println!("\n✅ Setup complete!");
            println!("\n📋 Next steps:");
            println!("   1. Deploy verification key to LUMENYX chain:");
            println!("      sudo.setVerificationKey(0x{})", hex::encode(&vk_bytes[..64.min(vk_bytes.len())]));
            println!("   2. Keep proving key safe for proof generation");
        }
        
        Commands::Commitment { amount, secret, blinding } => {
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║         LUMENYX ZK - CREATE COMMITMENT                      ║");
            println!("╚═══════════════════════════════════════════════════════════╝\n");
            
            let mut rng = thread_rng();
            
            let secret_fr = match secret {
                Some(s) => hex_to_fr(&s).expect("Invalid secret hex"),
                None => Fr::rand(&mut rng),
            };
            
            let blinding_fr = match blinding {
                Some(b) => hex_to_fr(&b).expect("Invalid blinding hex"),
                None => Fr::rand(&mut rng),
            };
            
            let amount_fr = Fr::from(amount);
            let commitment = compute_commitment(amount_fr, secret_fr, blinding_fr);
            
            println!("💰 Amount: {} LUMENYX", amount);
            println!("🔑 Secret: {}", fr_to_hex(&secret_fr));
            println!("🎲 Blinding: {}", fr_to_hex(&blinding_fr));
            println!("📦 Commitment: {}", fr_to_hex(&commitment));
            
            println!("\n⚠️  SAVE YOUR SECRET AND BLINDING!");
            println!("   You need them to withdraw your funds.");
            
            println!("\n📋 To shield, call:");
            println!("   privacy.shield({}, 0x{})", amount, fr_to_hex(&commitment));
        }
        
        Commands::ProveUnshield { amount, secret, blinding, merkle_path, pk_file } => {
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║         LUMENYX ZK - GENERATE UNSHIELD PROOF                ║");
            println!("╚═══════════════════════════════════════════════════════════╝\n");
            
            // Load proving key
            println!("📁 Loading proving key...");
            let pk_bytes = fs::read(&pk_file).expect("Failed to read proving key");
            let pk = ark_groth16::ProvingKey::<Bn254>::deserialize_uncompressed(&pk_bytes[..])
                .expect("Failed to deserialize PK");
            
            // Parse inputs
            let secret_fr = hex_to_fr(&secret).expect("Invalid secret");
            let blinding_fr = hex_to_fr(&blinding).expect("Invalid blinding");
            let amount_fr = Fr::from(amount);
            
            // Load merkle path
            println!("🌳 Loading Merkle path...");
            let path_json = fs::read_to_string(&merkle_path).expect("Failed to read merkle path");
            let path_data: serde_json::Value = serde_json::from_str(&path_json).expect("Invalid JSON");
            
            let merkle_path_fr: Vec<Fr> = path_data["path"]
                .as_array()
                .expect("Missing path")
                .iter()
                .map(|v| hex_to_fr(v.as_str().unwrap()).unwrap())
                .collect();
            
            let path_indices: Vec<bool> = path_data["indices"]
                .as_array()
                .expect("Missing indices")
                .iter()
                .map(|v| v.as_bool().unwrap())
                .collect();
            
            let root_fr = hex_to_fr(path_data["root"].as_str().expect("Missing root")).unwrap();
            
            // Compute values
            let commitment = compute_commitment(amount_fr, secret_fr, blinding_fr);
            let nullifier = compute_nullifier(commitment, secret_fr);
            
            println!("📦 Commitment: {}", fr_to_hex(&commitment));
            println!("🔐 Nullifier: {}", fr_to_hex(&nullifier));
            
            // Create circuit
            let circuit = SpendCircuit::new(
                amount_fr,
                secret_fr,
                blinding_fr,
                merkle_path_fr,
                path_indices,
                nullifier,
                root_fr,
            );
            
            // Generate proof
            println!("🔮 Generating ZK proof...");
            let start = Instant::now();
            let mut rng = thread_rng();
            let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
                .expect("Proof generation failed");
            println!("   ✅ Proof generated in {:?}", start.elapsed());
            
            // Serialize
            let mut proof_bytes = Vec::new();
            proof.serialize_uncompressed(&mut proof_bytes).unwrap();
            
            println!("\n📋 Call unshield with:");
            println!("   privacy.unshield(");
            println!("     amount: {},", amount);
            println!("     nullifier: 0x{},", fr_to_hex(&nullifier));
            println!("     root: 0x{},", fr_to_hex(&root_fr));
            println!("     proof: 0x{}", hex::encode(&proof_bytes));
            println!("   )");
        }
        
        Commands::Verify { proof, inputs, vk_file } => {
            println!("Verifying proof...");
            
            let vk_bytes = fs::read(&vk_file).expect("Failed to read VK");
            let vk = ark_groth16::VerifyingKey::<Bn254>::deserialize_uncompressed(&vk_bytes[..])
                .expect("Failed to deserialize VK");
            
            let proof_bytes = fs::read(&proof).expect("Failed to read proof");
            let proof = ark_groth16::Proof::<Bn254>::deserialize_uncompressed(&proof_bytes[..])
                .expect("Failed to deserialize proof");
            
            let inputs_json = fs::read_to_string(&inputs).expect("Failed to read inputs");
            let inputs_data: serde_json::Value = serde_json::from_str(&inputs_json).unwrap();
            
            let public_inputs: Vec<Fr> = inputs_data["inputs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| hex_to_fr(v.as_str().unwrap()).unwrap())
                .collect();
            
            let pvk = ark_groth16::prepare_verifying_key(&vk);
            let valid = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &public_inputs, &proof)
                .expect("Verification failed");
            
            if valid {
                println!("✅ Proof is VALID");
            } else {
                println!("❌ Proof is INVALID");
            }
        }
        
        Commands::Test => {
            println!("╔═══════════════════════════════════════════════════════════╗");
            println!("║         LUMENYX ZK - FULL TEST                              ║");
            println!("╚═══════════════════════════════════════════════════════════╝\n");
            
            let mut rng = thread_rng();
            
            // Setup
            println!("📐 Running setup...");
            let start = Instant::now();
            let circuit = SpendCircuit::empty(TREE_DEPTH);
            let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng).unwrap();
            println!("   ✅ Setup: {:?}", start.elapsed());
            
            // Create note
            let amount = Fr::from(100u64);
            let secret = Fr::rand(&mut rng);
            let blinding = Fr::rand(&mut rng);
            let commitment = compute_commitment(amount, secret, blinding);
            let nullifier = compute_nullifier(commitment, secret);
            
            println!("\n💰 Created note: 100 LUMENYX");
            
            // Build tree
            let mut leaves: Vec<Fr> = (0..16).map(|i| Fr::from(i as u64)).collect();
            leaves[7] = commitment;
            
            let tree_size = 1 << TREE_DEPTH;
            while leaves.len() < tree_size {
                leaves.push(Fr::from(0u64));
            }
            
            let tree = build_merkle_tree(&leaves);
            let root = tree.last().unwrap()[0];
            let (path, indices) = get_merkle_path(&tree, 7);
            
            println!("🌳 Built Merkle tree (depth {})", TREE_DEPTH);
            
            // Prove
            println!("\n🔮 Generating proof...");
            let start = Instant::now();
            let circuit = SpendCircuit::new(amount, secret, blinding, path, indices, nullifier, root);
            let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();
            println!("   ✅ Proof generated: {:?}", start.elapsed());
            
            // Verify
            println!("\n✅ Verifying proof...");
            let start = Instant::now();
            let pvk = ark_groth16::prepare_verifying_key(&vk);
            let public_inputs = vec![nullifier, root, amount];
            let valid = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &public_inputs, &proof).unwrap();
            println!("   Result: {}", if valid { "VALID ✅" } else { "INVALID ❌" });
            println!("   Time: {:?}", start.elapsed());
            
            println!("\n╔═══════════════════════════════════════════════════════════╗");
            if valid {
                println!("║  ✅ ALL TESTS PASSED - ZK SYSTEM WORKING!                 ║");
            } else {
                println!("║  ❌ TEST FAILED                                           ║");
            }
            println!("╚═══════════════════════════════════════════════════════════╝");
        }
    }
}
