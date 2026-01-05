// node/src/dag_protocol.rs

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use codec::{Decode, Encode};
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;

use sc_client_api::BlockBackend;
use sc_network::{
    config::FullNetworkConfiguration,
    request_responses::{IncomingRequest, OutgoingResponse},
    NetworkService, NetworkWorker, PeerId, ReputationChange, RequestFailure,
    IfDisconnected, NetworkRequest,
};

use lumenyx_runtime::opaque::Block;
use crate::service::FullClient;

pub type SignedBlock = sp_runtime::generic::SignedBlock<Block>;
type BlockHash = <Block as BlockT>::Hash;

pub const DAG_BLOCKS_PROTO: &str = "/lumenyx/dag-blocks/1";

#[derive(Debug, Clone, Encode, Decode)]
pub enum DagReq {
    GetBlocks { hashes: Vec<H256> },
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum DagResp {
    Blocks { blocks: Vec<SignedBlock> },
    NotFound { missing: Vec<H256> },
    Error { msg: Vec<u8> },
}

/// Registra il protocollo request-response
pub fn register_dag_blocks_protocol(
    net_config: &mut FullNetworkConfiguration<Block, BlockHash, NetworkWorker<Block, BlockHash>>,
) -> async_channel::Receiver<IncomingRequest> {
    let (tx, rx) = async_channel::bounded(1024);

    let cfg = sc_network::config::RequestResponseConfig {
        name: DAG_BLOCKS_PROTO.into(),
        fallback_names: vec![],
        max_request_size: 2 * 1024 * 1024,
        max_response_size: 64 * 1024 * 1024,
        request_timeout: Duration::from_secs(10),
        inbound_queue: Some(tx),
    };

    net_config.add_request_response_protocol(cfg);
    rx
}

/// Server-side handler
pub async fn run_dag_blocks_server(
    inbound_rx: async_channel::Receiver<IncomingRequest>,
    client: Arc<FullClient>,
) {
    while let Ok(req) = inbound_rx.recv().await {
        let resp_bytes = match DagReq::decode(&mut &req.payload[..]) {
            Ok(DagReq::GetBlocks { hashes }) => {
                let mut blocks = Vec::with_capacity(hashes.len());
                let mut missing = Vec::new();

                for h in hashes {
                    let hash: BlockHash = h.into();
                    match client.block(hash) {
                        Ok(Some(sb)) => blocks.push(sb),
                        _ => missing.push(h),
                    }
                }

                if missing.is_empty() {
                    DagResp::Blocks { blocks }.encode()
                } else {
                    DagResp::NotFound { missing }.encode()
                }
            }
            Err(e) => DagResp::Error {
                msg: format!("bad request: {e:?}").into_bytes(),
            }
            .encode(),
        };

        let out = OutgoingResponse {
            result: Ok(resp_bytes),
            reputation_changes: Vec::<ReputationChange>::new(),
            sent_feedback: None,
        };

        let _ = req.pending_response.send(out);
    }
}

/// Client-side: richiede blocchi by-hash
pub async fn request_blocks_by_hash(
    network: Arc<NetworkService<Block, BlockHash>>,
    peer: PeerId,
    hashes: Vec<H256>,
) -> Result<Vec<u8>, RequestFailure> {
    let payload = DagReq::GetBlocks { hashes }.encode();

    let (response, _protocol) = network.request(
        peer.into(),
        DAG_BLOCKS_PROTO.into(),
        payload,
        None,
        IfDisconnected::ImmediateError,
    ).await?;

    Ok(response)
}
