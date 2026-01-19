//! LUMENYX P2Pool Module
//!
//! Decentralized mining pool implementation using PPLNS reward distribution.
//! No central coordinator - miners share work via gossip protocol.

pub mod gossip;
pub mod pplns;
pub mod sharechain;
pub mod types;

pub use gossip::{spawn_pool_gossip_task, PoolGossip, POOL_PROTO_NAME};
pub use pplns::compute_pplns_payouts;
pub use sharechain::Sharechain;
pub use types::{PoolAccountId, PoolPayoutEntry, PoolShare};

// ═══════════════════════════════════════════════════════════════════════════════
// MVP CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum number of payouts per block (to limit digest size)
pub const MAX_POOL_PAYOUTS: usize = 64;

/// PPLNS window size in shares
pub const PPLNS_WINDOW_SHARES: usize = 256;

/// Share difficulty divisor: share_diff = main_diff / SHARE_DIFFICULTY_DIVISOR
/// Lower = more shares = more granular payouts but more network traffic
pub const SHARE_DIFFICULTY_DIVISOR: u128 = 256;
