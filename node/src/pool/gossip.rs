//! Pool Gossip Protocol - Share broadcasting via sc_network notifications
//!
//! stable2409 notes:
//! - `open_substream()` can be unimplemented (panic).
//! - `NotificationService::clone()` may not share `message_sink()` state as expected in practice.
//!   Use a single task owning the NotificationService: handle both RX events and TX sends there.

use codec::{Decode, Encode};
use sc_network::service::traits::{NotificationEvent, NotificationService, ValidationResult};
use sc_network_types::PeerId;

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;

use super::types::PoolShare;

/// Protocol name for pool share gossip
pub const POOL_PROTO_NAME: &str = "/lumenyx/poolshare/1";

pub struct PoolGossip {
    pub tx_out: mpsc::UnboundedSender<PoolShare>,
    pub rx_in: mpsc::UnboundedReceiver<PoolShare>,
    pub peer_count: Arc<AtomicU64>,
}

pub fn spawn_pool_gossip_task(
    mut notification_service: Box<dyn NotificationService>,
    spawn_handle: sc_service::SpawnTaskHandle,
) -> PoolGossip {
    let (tx_out, mut rx_out) = mpsc::unbounded_channel::<PoolShare>();
    let (tx_in, rx_in) = mpsc::unbounded_channel::<PoolShare>();

    let peer_count = Arc::new(AtomicU64::new(0));
    let peer_count_update = peer_count.clone();

    spawn_handle.spawn("poolshare-gossip", None, async move {
        // Peers for which sc-network says: "remote peer ready to receive notifications".
        let mut ready_peers: HashSet<PeerId> = HashSet::new();

        // Outgoing queue (global). Keeps shares produced before peers become ready.
        let mut pending: VecDeque<Vec<u8>> = VecDeque::new();
        const MAX_PENDING: usize = 1024;

        loop {
            tokio::select! {
                // A) outgoing shares produced locally
                maybe_share = rx_out.recv() => {
                    let Some(share) = maybe_share else {
                        log::warn!("🏊 poolshare-gossip: tx_out closed");
                        break;
                    };

                    let bytes = share.encode();
                    if pending.len() >= MAX_PENDING {
                        pending.pop_front();
                    }
                    pending.push_back(bytes);

                    // Try to flush immediately (if peers ready)
                    if !ready_peers.is_empty() && !pending.is_empty() {
                        let peers: Vec<PeerId> = ready_peers.iter().cloned().collect();
                        let mut msgs: Vec<Vec<u8>> = Vec::new();
                        while let Some(m) = pending.pop_front() {
                            msgs.push(m);
                        }

                        for peer in peers {
                            if notification_service.message_sink(&peer).is_none() {
                                log::debug!("🏊 Ready peer {:?} but message_sink is None; skip flush", peer);
                                continue;
                            }

                            for msg in &msgs {
                                match notification_service.send_async_notification(&peer, msg.clone()).await {
                                    Ok(()) => log::info!("🏊 Sent share to peer {:?}", peer),
                                    Err(e) => log::warn!("🏊 send_async_notification failed to {:?}: {:?}", peer, e),
                                }
                            }
                        }
                    }
                }

                // B) notification events from network
                ev = notification_service.next_event() => {
                    match ev {
                        Some(NotificationEvent::ValidateInboundSubstream { peer, handshake: _, result_tx }) => {
                            let _ = result_tx.send(ValidationResult::Accept);
                            log::info!("🏊 Pool: accepted inbound substream from {:?}", peer);
                        }

                        Some(NotificationEvent::NotificationStreamOpened { peer, direction, handshake: _, negotiated_fallback }) => {
                            log::info!("🏊 Pool stream opened: {:?} ({:?}) fallback={:?}", peer, direction, negotiated_fallback);
                            ready_peers.insert(peer);

                            peer_count_update.store(ready_peers.len() as u64, Ordering::Relaxed);

                            // Flush anything queued now that at least one peer is ready
                            if !pending.is_empty() {
                                let peers: Vec<PeerId> = ready_peers.iter().cloned().collect();
                                let mut msgs: Vec<Vec<u8>> = Vec::new();
                                while let Some(m) = pending.pop_front() {
                                    msgs.push(m);
                                }

                                for p in peers {
                                    if notification_service.message_sink(&p).is_none() {
                                        continue;
                                    }

                                    for msg in &msgs {
                                        match notification_service.send_async_notification(&p, msg.clone()).await {
                                            Ok(()) => log::info!("🏊 Sent share to peer {:?}", p),
                                            Err(e) => log::warn!("🏊 send_async_notification failed to {:?}: {:?}", p, e),
                                        }
                                    }
                                }
                            }
                        }

                        Some(NotificationEvent::NotificationStreamClosed { peer }) => {
                            log::info!("🏊 Pool stream closed: {:?}", peer);
                            ready_peers.remove(&peer);
                            peer_count_update.store(ready_peers.len() as u64, Ordering::Relaxed);
                        }

                        Some(NotificationEvent::NotificationReceived { peer, notification }) => {
                            match PoolShare::decode(&mut &notification[..]) {
                                Ok(share) => {
                                    log::info!("🏊 Received share {:?} from {:?}", share.id, peer);
                                    let _ = tx_in.send(share);
                                }
                                Err(e) => {
                                    log::debug!("🏊 Dropping invalid share from {:?}: decode error {:?}", peer, e);
                                }
                            }
                        }

                        None => {
                            log::warn!("🏊 Pool notification stream ended");
                            break;
                        }
                    }
                }

                // C) periodic flush attempt (helps if sink becomes available slightly after StreamOpened)
                _ = tokio::time::sleep(std::time::Duration::from_millis(300)) => {
                    if !ready_peers.is_empty() && !pending.is_empty() {
                        let peers: Vec<PeerId> = ready_peers.iter().cloned().collect();
                        let mut msgs: Vec<Vec<u8>> = Vec::new();
                        while let Some(m) = pending.pop_front() {
                            msgs.push(m);
                        }

                        for peer in peers {
                            if notification_service.message_sink(&peer).is_none() {
                                continue;
                            }

                            for msg in &msgs {
                                match notification_service.send_async_notification(&peer, msg.clone()).await {
                                    Ok(()) => log::info!("🏊 Sent share to peer {:?}", peer),
                                    Err(e) => log::warn!("🏊 send_async_notification failed to {:?}: {:?}", peer, e),
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    PoolGossip {
        tx_out,
        rx_in,
        peer_count,
    }
}
