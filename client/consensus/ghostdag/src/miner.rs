//! GHOSTDAG Miner - PoW mining with multi-parent block production

use sp_core::H256;
use sc_client_api::AuxStore;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::store::GhostdagStore;
use crate::dag::DagManager;
use crate::ghostdag::GhostdagManager;

/// Mining configuration
#[derive(Clone)]
pub struct MiningConfig {
    /// Target block time in milliseconds
    pub target_block_time_ms: u64,
    /// Maximum parents per block
    pub max_parents: usize,
    /// Initial difficulty
    pub initial_difficulty: u64,
    /// Difficulty adjustment window (blocks)
    pub difficulty_window: u64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            target_block_time_ms: 1000, // 1 second target
            max_parents: 10,
            initial_difficulty: 1_000_000,
            difficulty_window: 2016,
        }
    }
}

/// Block template for mining
#[derive(Clone, Debug)]
pub struct BlockTemplate {
    /// Parents to reference
    pub parents: Vec<H256>,
    /// Selected parent (heaviest)
    pub selected_parent: H256,
    /// Current difficulty target
    pub difficulty: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Block number (blue_score + 1)
    pub number: u64,
}

/// Mining result
#[derive(Clone, Debug)]
pub struct MinedBlock {
    pub template: BlockTemplate,
    pub nonce: [u8; 32],
    pub hash: H256,
    pub work: u128,
}

/// GHOSTDAG Miner
pub struct GhostdagMiner<C> {
    dag: DagManager<C>,
    ghostdag: GhostdagManager<C>,
    store: GhostdagStore<C>,
    config: MiningConfig,
    current_difficulty: Arc<RwLock<u64>>,
    is_mining: Arc<RwLock<bool>>,
}

impl<C> GhostdagMiner<C> {
    pub fn new(
        dag: DagManager<C>,
        ghostdag: GhostdagManager<C>,
        store: GhostdagStore<C>,
        config: MiningConfig,
    ) -> Self {
        let initial_diff = config.initial_difficulty;
        Self {
            dag,
            ghostdag,
            store,
            config,
            current_difficulty: Arc::new(RwLock::new(initial_diff)),
            is_mining: Arc::new(RwLock::new(false)),
        }
    }
}

impl<C: sc_client_api::AuxStore + Send + Sync + 'static> GhostdagMiner<C> {
    /// Create a block template for mining
    pub fn create_template(&self) -> Option<BlockTemplate> {
        // Get current tips
        let tips = self.dag.get_tips();
        if tips.is_empty() {
            log::warn!("No tips available for mining");
            return None;
        }

        // Select parents (up to max_parents, sorted by blue_work)
        let parents = self.dag.select_parents(self.config.max_parents);
        if parents.is_empty() {
            return None;
        }

        // Find selected parent
        let selected_parent = self.dag.find_selected_parent(&parents)?;

        // Get current blue score for block number
        let blue_score = self.store.get_blue_score(&selected_parent).unwrap_or(0);

        // Get current difficulty
        let difficulty = *self.current_difficulty.read();

        // Current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Some(BlockTemplate {
            parents,
            selected_parent,
            difficulty,
            timestamp,
            number: blue_score + 1,
        })
    }

    /// Mine a block (find valid nonce)
    pub fn mine_block(&self, template: &BlockTemplate) -> Option<MinedBlock> {
        let target = difficulty_to_target(template.difficulty);
        let mut nonce = [0u8; 32];
        let start = Instant::now();

        // Prepare data to hash
        let mut base_data = Vec::new();
        for parent in &template.parents {
            base_data.extend_from_slice(parent.as_ref());
        }
        base_data.extend_from_slice(&template.timestamp.to_le_bytes());
        base_data.extend_from_slice(&template.number.to_le_bytes());

        let mut attempts = 0u64;
        loop {
            // Check if we should stop
            if !*self.is_mining.read() {
                return None;
            }

            // Increment nonce
            increment_nonce(&mut nonce);
            attempts += 1;

            // Compute hash
            let hash = compute_pow_hash(&base_data, &nonce);
            
            // Check if valid
            if hash_meets_target(&hash, &target) {
                let elapsed = start.elapsed();
                let hashrate = attempts as f64 / elapsed.as_secs_f64();
                
                log::info!(
                    "⛏️  Block mined! hash={:?}, nonce={}, attempts={}, time={:?}, hashrate={:.2} H/s",
                    hash,
                    hex::encode(&nonce[..8]),
                    attempts,
                    elapsed,
                    hashrate
                );

                return Some(MinedBlock {
                    template: template.clone(),
                    nonce,
                    hash,
                    work: difficulty_to_work(template.difficulty),
                });
            }

            // Log progress periodically
            if attempts % 1_000_000 == 0 {
                let elapsed = start.elapsed();
                let hashrate = attempts as f64 / elapsed.as_secs_f64();
                log::debug!("Mining... {} attempts, {:.2} H/s", attempts, hashrate);
            }
        }
    }

    /// Start mining loop (async)
    pub async fn start_mining(&self, block_tx: mpsc::Sender<MinedBlock>) {
        *self.is_mining.write() = true;
        log::info!("🚀 GHOSTDAG miner started");

        while *self.is_mining.read() {
            // Create template
            let Some(template) = self.create_template() else {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };

            log::debug!(
                "Mining block #{} with {} parents, difficulty={}",
                template.number,
                template.parents.len(),
                template.difficulty
            );

            // Mine block (in blocking task)
            let miner = self.clone();
            let template_clone = template.clone();
            
            let result = tokio::task::spawn_blocking(move || {
                miner.mine_block(&template_clone)
            }).await;

            match result {
                Ok(Some(block)) => {
                    // Send mined block
                    if block_tx.send(block).await.is_err() {
                        log::error!("Failed to send mined block");
                        break;
                    }
                    
                    // Adjust difficulty
                    self.adjust_difficulty();
                }
                Ok(None) => {
                    // Mining stopped or interrupted
                    continue;
                }
                Err(e) => {
                    log::error!("Mining task error: {:?}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        log::info!("🛑 GHOSTDAG miner stopped");
    }

    /// Stop mining
    pub fn stop_mining(&self) {
        *self.is_mining.write() = false;
    }

    /// Adjust difficulty based on recent block times
    pub fn adjust_difficulty(&self) {
        let tips = self.dag.get_tips();
        if tips.is_empty() {
            return;
        }

        // Get virtual tip
        let Some(tip) = self.ghostdag.get_virtual_tip() else {
            return;
        };

        // Get recent blocks for timing
        let chain = self.store.get_ghostdag_data(&tip);
        let Some(data) = chain else {
            return;
        };

        // Simple difficulty adjustment
        // In production, use Kaspa's DAA (difficulty adjustment algorithm)
        let blue_score = data.blue_score;
        if blue_score > 0 && blue_score % self.config.difficulty_window == 0 {
            let current = *self.current_difficulty.read();
            
            // Placeholder: adjust based on recent block rate
            // TODO: Implement proper DAA based on timestamps
            let new_difficulty = current; // Keep same for now
            
            *self.current_difficulty.write() = new_difficulty;
            log::info!("📊 Difficulty adjusted: {} -> {}", current, new_difficulty);
        }
    }

    /// Get current difficulty
    pub fn get_difficulty(&self) -> u64 {
        *self.current_difficulty.read()
    }

    /// Set difficulty (for testing)
    pub fn set_difficulty(&self, difficulty: u64) {
        *self.current_difficulty.write() = difficulty;
    }
}

impl<C> Clone for GhostdagMiner<C> {
    fn clone(&self) -> Self {
        Self {
            dag: self.dag.clone(),
            ghostdag: self.ghostdag.clone(),
            store: self.store.clone(),
            config: self.config.clone(),
            current_difficulty: self.current_difficulty.clone(),
            is_mining: self.is_mining.clone(),
        }
    }
}

// ============ PoW Helper Functions ============

/// Compute PoW hash using Blake3
fn compute_pow_hash(data: &[u8], nonce: &[u8; 32]) -> H256 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(nonce);
    let result = hasher.finalize();
    H256::from_slice(&result.as_bytes()[..32])
}

/// Convert difficulty to target (higher difficulty = lower target)
fn difficulty_to_target(difficulty: u64) -> H256 {
    if difficulty == 0 {
        return H256::repeat_byte(0xff);
    }
    
    // Target = MAX_TARGET / difficulty
    let max_target = [0xff_u8; 32];
    let mut target = [0u8; 32];
    
    // Simple division for MVP
    let divisor = difficulty as u128;
    let mut remainder = 0u128;
    
    for i in 0..32 {
        let current = (remainder << 8) | (max_target[i] as u128);
        target[i] = (current / divisor) as u8;
        remainder = current % divisor;
    }
    
    H256::from_slice(&target)
}

/// Check if hash meets target (hash <= target)
fn hash_meets_target(hash: &H256, target: &H256) -> bool {
    hash.as_bytes() <= target.as_bytes()
}

/// Convert difficulty to work value
fn difficulty_to_work(difficulty: u64) -> u128 {
    difficulty as u128
}

/// Increment nonce (little-endian)
fn increment_nonce(nonce: &mut [u8; 32]) {
    for byte in nonce.iter_mut() {
        if *byte == 255 {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
}
