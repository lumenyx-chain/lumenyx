// node/src/dag_sync.rs

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use codec::Decode;
use sp_core::H256;
use parking_lot::Mutex;

use sc_network::{NetworkRequest, PeerId, IfDisconnected};
use sc_network::service::traits::NetworkService;
use sp_runtime::traits::Block as BlockT;

use lumenyx_runtime::opaque::Block;
use crate::dag_protocol::{DagResp, SignedBlock, DAG_BLOCKS_PROTO, DagReq};
use codec::Encode;

const MAX_ORPHANS: usize = 20_000;
const MAX_BATCH: usize = 64;

pub struct DagSync {
    network: Arc<dyn NetworkService>,

    // child_hash -> (child_block, missing parents)
    orphans: Mutex<HashMap<H256, (SignedBlock, HashSet<H256>)>>,
    // parent_hash -> children waiting
    waiting: Mutex<HashMap<H256, Vec<H256>>>,
    // requested parent hashes (dedup)
    requested: Mutex<HashSet<H256>>,

    // "peer hint" per fare request-response
    last_peer: Mutex<Option<PeerId>>,

    // ready blocks (tutti parents disponibili)
    ready: Mutex<VecDeque<SignedBlock>>,

    /// reinject verso la tua pipeline di import
    reinject: Arc<dyn Fn(SignedBlock) + Send + Sync>,
}

impl DagSync {
    pub fn new(
        network: Arc<dyn NetworkService>,
        reinject: Arc<dyn Fn(SignedBlock) + Send + Sync>,
    ) -> Self {
        Self {
            network,
            orphans: Mutex::new(HashMap::new()),
            waiting: Mutex::new(HashMap::new()),
            requested: Mutex::new(HashSet::new()),
            last_peer: Mutex::new(None),
            ready: Mutex::new(VecDeque::new()),
            reinject,
        }
    }

    pub fn note_peer(&self, peer: PeerId) {
        *self.last_peer.lock() = Some(peer);
    }

    /// Chiamalo quando `verify()` scopre ORPHAN (missing parents).
    pub fn on_orphan(&self, sb: SignedBlock, missing: Vec<H256>) -> Result<(), String> {
        if missing.is_empty() {
            return Ok(());
        }

        let bh: H256 = H256::from_slice(sb.block.hash().as_ref());

        {
            let mut orphans = self.orphans.lock();
            if orphans.len() >= MAX_ORPHANS {
                return Err("Orphan pool full".into());
            }
            orphans.insert(bh, (sb.clone(), missing.iter().cloned().collect()));
        }

        {
            let mut waiting = self.waiting.lock();
            for p in &missing {
                waiting.entry(*p).or_default().push(bh);
            }
        }

        self.fetch_missing(missing)
    }

    fn fetch_missing(&self, missing: Vec<H256>) -> Result<(), String> {
        let peer = self
            .last_peer
            .lock()
            .clone()
            .ok_or_else(|| "No connected peer known yet".to_string())?;

        // dedup
        let mut to_req = Vec::new();
        {
            let mut requested = self.requested.lock();
            for h in missing {
                if requested.insert(h) {
                    to_req.push(h);
                }
            }
        }

        if to_req.is_empty() {
            return Ok(());
        }

        for chunk in to_req.chunks(MAX_BATCH) {
            let net = self.network.clone();
            let peer2 = peer;
            let hashes = chunk.to_vec();
            let reinject = self.reinject.clone();

            // Non bloccare verify/import - spawn task
            tokio::spawn(async move {
                let payload = DagReq::GetBlocks { hashes }.encode();
                
                match net.request(
                    peer2.into(),
                    DAG_BLOCKS_PROTO.into(),
                    payload,
                    None,
                    IfDisconnected::ImmediateError,
                ).await {
                    Ok((resp_bytes, _protocol)) => {
                        if let Ok(resp) = DagResp::decode(&mut &resp_bytes[..]) {
                            match resp {
                                DagResp::Blocks { blocks } => {
                                    for sb in blocks {
                                        log::info!("📥 DAG sync received block {:?}", sb.block.hash());
                                        (reinject)(sb);
                                    }
                                }
                                DagResp::NotFound { missing } => {
                                    log::debug!("DAG sync: peer missing {} blocks", missing.len());
                                }
                                DagResp::Error { msg } => {
                                    log::warn!("DAG sync error: {}", String::from_utf8_lossy(&msg));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("DAG request failed: {:?}", e);
                    }
                }
            });
        }

        Ok(())
    }

    /// Chiamalo da `client.import_notification_stream()`.
    pub fn on_block_imported(&self, imported: H256) {
        self.requested.lock().remove(&imported);

        let children = {
            let mut waiting = self.waiting.lock();
            waiting.remove(&imported).unwrap_or_default()
        };

        for child in children {
            let maybe_ready = {
                let mut orphans = self.orphans.lock();
                if let Some((sb, missing)) = orphans.get_mut(&child) {
                    missing.remove(&imported);
                    if missing.is_empty() { Some(sb.clone()) } else { None }
                } else {
                    None
                }
            };

            if let Some(sb) = maybe_ready {
                self.orphans.lock().remove(&child);
                self.ready.lock().push_back(sb);
            }
        }

        self.drain_ready();
    }

    fn drain_ready(&self) {
        loop {
            let next = self.ready.lock().pop_front();
            let Some(sb) = next else { break };
            log::info!("🔷 DAG sync: orphan {:?} now ready, reinjecting", sb.block.hash());
            (self.reinject)(sb);
        }
    }

    pub fn orphan_count(&self) -> usize {
        self.orphans.lock().len()
    }
}
