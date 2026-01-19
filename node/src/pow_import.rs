// node/src/pow_import.rs
//
// LUMENYX - PoW BlockImport wrapper con:
// - Verifica PoW RX-LX (Seal nonce) prima dell'import
// - Total Difficulty tracking in AuxStore
// - Fork choice: imposta best chain in base a TD (heaviest chain)
// - (Opzionale ma consigliato) SelectChain per mining basata su TD
//
// Target: polkadot-sdk stable2409
// Dipendenze: async-trait, parking_lot (già in node/Cargo.toml)

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use codec::{Decode, Encode};

use sp_core::{H256, U256};
use sp_runtime::generic::DigestItem;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, SaturatedConversion};

use sc_client_api::{AuxStore, HeaderBackend, StorageProvider};
use sc_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult,
};
use sp_consensus::SelectChain;

use super::{
    difficulty_to_target, hash_meets_target, read_difficulty_from_storage, Block, FullBackend,
    RxLxPow, LUMENYX_ENGINE_ID,
};

const AUX_TD_PREFIX: &[u8] = b"lumenyx:td:";

fn td_key(hash: &H256) -> Vec<u8> {
    let mut k = Vec::with_capacity(AUX_TD_PREFIX.len() + 32);
    k.extend_from_slice(AUX_TD_PREFIX);
    k.extend_from_slice(hash.as_ref());
    k
}

fn decode_u256(raw: &[u8]) -> Option<U256> {
    U256::decode(&mut &raw[..]).ok()
}

fn encode_u256(x: U256) -> Vec<u8> {
    x.encode()
}

fn read_td<C: AuxStore>(client: &C, hash: H256) -> U256 {
    let k = td_key(&hash);
    match client.get_aux(&k) {
        Ok(Some(raw)) => decode_u256(&raw).unwrap_or_else(U256::zero),
        _ => U256::zero(),
    }
}

fn write_td<C: AuxStore>(client: &C, hash: H256, td: U256) {
    let k = td_key(&hash);
    let v = encode_u256(td);
    let _ = client.insert_aux(&[(k.as_slice(), v.as_slice())], &[]);
}

fn get_block_hash_by_height<C: HeaderBackend<Block>>(client: &C, height: u64) -> H256 {
    let n: NumberFor<Block> = height.saturated_into::<NumberFor<Block>>();
    match client.hash(n) {
        Ok(Some(h)) => h,
        _ => H256::zero(),
    }
}

fn remove_seal_from_header(header: &mut <Block as BlockT>::Header) {
    header
        .digest_mut()
        .logs
        .retain(|d| !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID));
}

fn ensure_single_seal_in_post_digests(params: &mut BlockImportParams<Block>, nonce: [u8; 32]) {
    params
        .post_digests
        .retain(|d| !matches!(d, DigestItem::Seal(id, _) if *id == LUMENYX_ENGINE_ID));
    params
        .post_digests
        .push(DigestItem::Seal(LUMENYX_ENGINE_ID, nonce.to_vec()));
}

fn extract_seal_nonce(params: &BlockImportParams<Block>) -> Option<[u8; 32]> {
    // Preferisci post_digests (RxLxVerifier sposta Seal lì).
    for d in &params.post_digests {
        if let DigestItem::Seal(id, bytes) = d {
            if *id == LUMENYX_ENGINE_ID {
                if bytes.len() != 32 {
                    return None;
                }
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&bytes[..32]);
                return Some(nonce);
            }
        }
    }

    // Fallback: seal in header.digest.logs (blocchi non normalizzati)
    for d in params.header.digest().logs.iter() {
        if let DigestItem::Seal(id, bytes) = d {
            if *id == LUMENYX_ENGINE_ID {
                if bytes.len() != 32 {
                    return None;
                }
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&bytes[..32]);
                return Some(nonce);
            }
        }
    }

    None
}

fn compute_block_hash_like_client(
    header_without_seal: &<Block as BlockT>::Header,
    post_digests: &[DigestItem],
) -> H256 {
    let mut h = header_without_seal.clone();
    h.digest_mut().logs.extend_from_slice(post_digests);
    h.hash()
}

/// Wrapper block import PoW+TD.
///
/// Deve essere pubblico verso service.rs (parent), quindi pub(crate).
pub(crate) struct LumenyxPowBlockImport<C> {
    inner: Arc<C>,
    rxlx: Arc<Mutex<Option<RxLxPow>>>,
}

impl<C> Clone for LumenyxPowBlockImport<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            rxlx: self.rxlx.clone(),
        }
    }
}

impl<C> LumenyxPowBlockImport<C> {
    pub(crate) fn new(inner: Arc<C>) -> Self {
        Self {
            inner,
            rxlx: Arc::new(Mutex::new(None)),
        }
    }

    fn verify_pow(
        &self,
        client: &C,
        header_without_seal: &<Block as BlockT>::Header,
        nonce: &[u8; 32],
    ) -> bool
    where
        C: HeaderBackend<Block> + StorageProvider<Block, FullBackend>,
    {
        let height_u64: u64 = (*header_without_seal.number()).saturated_into::<u64>();
        let parent_hash: H256 = *header_without_seal.parent_hash();

        let difficulty = read_difficulty_from_storage(client, parent_hash);
        let target = difficulty_to_target(difficulty);

        let pre_hash = header_without_seal.hash();

        let mut guard = self.rxlx.lock();
        if guard.is_none() {
            let get_block_hash = |h: u64| get_block_hash_by_height(client, h);
            *guard = RxLxPow::new(height_u64, get_block_hash).ok();
        }
        let Some(pow) = guard.as_mut() else {
            return false;
        };

        let get_block_hash = |h: u64| get_block_hash_by_height(client, h);
        if pow.maybe_reseed(height_u64, get_block_hash).is_err() {
            return false;
        }

        let pow_hash = pow.hash(&pre_hash, nonce);
        hash_meets_target(&pow_hash, &target)
    }

    fn compute_total_difficulty(&self, client: &C, parent_hash: H256) -> U256
    where
        C: AuxStore + StorageProvider<Block, FullBackend>,
    {
        let parent_td = read_td(client, parent_hash);
        let diff_u128 = read_difficulty_from_storage(client, parent_hash);
        parent_td.saturating_add(U256::from(diff_u128))
    }

    fn should_be_best(&self, client: &C, new_hash: H256, new_td: U256) -> bool
    where
        C: AuxStore + HeaderBackend<Block>,
    {
        let best_hash = client.info().best_hash;
        let best_td = read_td(client, best_hash);

        if new_td > best_td {
            return true;
        }
        if new_td < best_td {
            return false;
        }

        // Tie-break deterministico: preferisci hash minore (stabile tra nodi).
        new_hash < best_hash
    }
}

#[async_trait]
impl<C> BlockImport<Block> for LumenyxPowBlockImport<C>
where
    C: BlockImport<Block>
        + HeaderBackend<Block>
        + AuxStore
        + StorageProvider<Block, FullBackend>
        + Send
        + Sync
        + 'static,
{
    type Error = <C as BlockImport<Block>>::Error;

    async fn check_block(
        &self,
        params: BlockCheckParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(params).await
    }

    async fn import_block(
        &self,
        mut params: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        // 1) Extract nonce
        let Some(nonce) = extract_seal_nonce(&params) else {
            return Ok(ImportResult::KnownBad);
        };

        // 2) Normalize digests: seal fuori dall'header, dentro post_digests.
        remove_seal_from_header(&mut params.header);
        ensure_single_seal_in_post_digests(&mut params, nonce);

        // 3) Verify RX-LX PoW
        if !self.verify_pow(&*self.inner, &params.header, &nonce) {
            return Ok(ImportResult::KnownBad);
        }

        // 4) TD + fork-choice (heaviest chain)
        let parent_hash = *params.header.parent_hash();
        let new_td = self.compute_total_difficulty(&*self.inner, parent_hash);

        let new_hash = compute_block_hash_like_client(&params.header, &params.post_digests);
        let is_best = self.should_be_best(&*self.inner, new_hash, new_td);

        // IMPORTANTE: fork_choice basato su TD, non LongestChain
        params.fork_choice = Some(ForkChoiceStrategy::Custom(is_best));

        // 5) Import
        let res = self.inner.import_block(params).await?;

        // 6) Scrivi TD in aux (best-effort). Anche se il blocco non è best, serve per confronti futuri.
        write_td(&*self.inner, new_hash, new_td);

        Ok(res)
    }
}

/// SelectChain per mining basato su TD.
/// Non è indispensabile per la sync, ma è indispensabile per un PoW corretto lato authoring.
pub(crate) struct TotalDifficultySelectChain<S, C> {
    inner: S,
    client: Arc<C>,
}

impl<S: Clone, C> Clone for TotalDifficultySelectChain<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            client: self.client.clone(),
        }
    }
}

impl<S, C> TotalDifficultySelectChain<S, C> {
    pub(crate) fn new(inner: S, client: Arc<C>) -> Self {
        Self { inner, client }
    }
}

#[async_trait]
impl<S, C> SelectChain<Block> for TotalDifficultySelectChain<S, C>
where
    S: SelectChain<Block> + Send + Sync + Clone + 'static,
    C: HeaderBackend<Block> + AuxStore + Send + Sync + 'static,
{
    async fn leaves(&self) -> Result<Vec<<Block as BlockT>::Hash>, sp_consensus::Error> {
        self.inner.leaves().await
    }

    async fn best_chain(&self) -> Result<<Block as BlockT>::Header, sp_consensus::Error> {
        let leaves = self.inner.leaves().await?;

        let mut best: Option<(H256, U256)> = None;
        for h in leaves {
            let td = read_td(&*self.client, h);
            best = match best {
                None => Some((h, td)),
                Some((best_h, best_td)) => {
                    if td > best_td || (td == best_td && h < best_h) {
                        Some((h, td))
                    } else {
                        Some((best_h, best_td))
                    }
                }
            };
        }

        let (best_hash, _) =
            best.ok_or(sp_consensus::Error::StateUnavailable("no leaves".into()))?;
        let header = self
            .client
            .header(best_hash)
            .map_err(|_| sp_consensus::Error::ClientImport("header error".into()))?
            .ok_or(sp_consensus::Error::StateUnavailable(
                "header not found".into(),
            ))?;

        Ok(header)
    }
}
