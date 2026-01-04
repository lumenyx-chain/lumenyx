//! DAG Digest - Multi-parent block support for GHOSTDAG
//!
//! This module defines the digest item that carries extra parent
//! references and PoW data for GHOSTDAG blocks.

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::{RuntimeDebug, DigestItem};
use sp_std::vec::Vec;

/// Maximum number of parents per block
pub const MAX_PARENTS: u32 = 10;

/// DAG Parents Digest - carries multi-parent info in block header
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Eq)]
pub struct DagParentsDigest {
    /// Extra parents beyond the standard parent_hash
    #[codec(compact)]
    pub parent_count: u32,
    /// Parent hashes (excluding the one in header.parent_hash)
    pub extra_parents: Vec<H256>,
    /// PoW nonce that satisfies difficulty
    pub nonce: [u8; 32],
    /// Difficulty target this block was mined against
    #[codec(compact)]
    pub difficulty: u64,
    /// Timestamp in milliseconds
    #[codec(compact)]
    pub timestamp_ms: u64,
}

impl DagParentsDigest {
    /// Create new DAG parents digest
    pub fn new(
        extra_parents: Vec<H256>,
        nonce: [u8; 32],
        difficulty: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            parent_count: (extra_parents.len() + 1) as u32,
            extra_parents,
            nonce,
            difficulty,
            timestamp_ms,
        }
    }

    /// Get all parents (header.parent_hash + extra_parents)
    pub fn all_parents(&self, header_parent: H256) -> Vec<H256> {
        let mut parents = sp_std::vec![header_parent];
        parents.extend(self.extra_parents.iter().cloned());
        parents
    }

    /// Convert to DigestItem
    pub fn to_digest_item(&self) -> DigestItem {
        DigestItem::Other(self.encode())
    }

    /// Try to extract from DigestItem
    pub fn from_digest_item(item: &DigestItem) -> Option<Self> {
        match item {
            DigestItem::Other(data) => {
                Self::decode(&mut &data[..]).ok()
            }
            _ => None,
        }
    }

    /// Extract from block digest logs
    pub fn from_digest(digest: &sp_runtime::Digest) -> Option<Self> {
        for log in digest.logs() {
            if let Some(dag_digest) = Self::from_digest_item(log) {
                return Some(dag_digest);
            }
        }
        None
    }
}

/// Consensus engine ID for GHOSTDAG
pub const GHOSTDAG_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"GDAG";

/// Pre-runtime digest for GHOSTDAG
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Eq)]
pub enum GhostdagPreDigest {
    /// Primary block author slot
    #[codec(index = 0)]
    Primary {
        /// Parent hashes
        parents: Vec<H256>,
        /// PoW nonce
        nonce: [u8; 32],
    },
}

impl GhostdagPreDigest {
    /// Create pre-runtime digest item
    pub fn to_pre_runtime(&self) -> DigestItem {
        DigestItem::PreRuntime(GHOSTDAG_ENGINE_ID, self.encode())
    }

    /// Try to extract from pre-runtime digest
    pub fn from_pre_runtime(item: &DigestItem) -> Option<Self> {
        match item {
            DigestItem::PreRuntime(id, data) if *id == GHOSTDAG_ENGINE_ID => {
                Self::decode(&mut &data[..]).ok()
            }
            _ => None,
        }
    }
}

/// Seal for GHOSTDAG blocks (PoW proof)
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, PartialEq, Eq)]
pub struct GhostdagSeal {
    /// PoW nonce
    pub nonce: [u8; 32],
    /// Resulting hash (for quick verification)
    pub hash: H256,
    /// Work done (derived from difficulty)
    pub work: u128,
}

impl GhostdagSeal {
    /// Create seal digest item
    pub fn to_seal(&self) -> DigestItem {
        DigestItem::Seal(GHOSTDAG_ENGINE_ID, self.encode())
    }

    /// Try to extract from seal digest
    pub fn from_seal(item: &DigestItem) -> Option<Self> {
        match item {
            DigestItem::Seal(id, data) if *id == GHOSTDAG_ENGINE_ID => {
                Self::decode(&mut &data[..]).ok()
            }
            _ => None,
        }
    }
}
