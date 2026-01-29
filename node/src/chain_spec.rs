//! # LUMENYX Chain Specification - PoW LongestChain
//!
//! Defines genesis configuration for development, testnet, and mainnet.
//!
//! - No authorities needed - anyone can mine
//! - Total Supply: 21,000,000 LUMENYX
//! - Distribution: 100% through mining
//! - No pre-allocations. Pure fair launch.

use lumenyx_runtime::{AccountId, Signature, WASM_BINARY};
use sc_service::ChainType;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub type ChainSpec = sc_service::GenericChainSpec;

// ============================================
// GENESIS CONSTANTS
// ============================================

/// Genesis block message - The reason LUMENYX exists
pub const GENESIS_MESSAGE: &str =
    "Bitcoin started with a headline. Ethereum started with a premine. LUMENYX starts with you.";

/// Network properties
pub const TOKEN_DECIMALS: u32 = 12;
pub const TOKEN_SYMBOL: &str = "LUMENYX";

// ============================================
// HELPER FUNCTIONS
// ============================================

pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

// ============================================
// DEVELOPMENT CONFIG (for testing)
// ============================================

fn development_genesis(endowed_accounts: Vec<AccountId>) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, 1u128 << 60)).collect::<Vec<_>>(),
        },
        "difficulty": {
            "initialDifficulty": 1
        },
        "evm": {
            "accounts": {
                "0xd43593c715fdd31c61141abd04a99fd6822c8558": {
                    "balance": "0xffffffffffffffffffffffff",
                    "nonce": "0x0",
                    "storage": {},
                    "code": []
                }
            }
        },
    })
}

pub fn development_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Development")
    .with_id("lumenyx_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(development_genesis(vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
    ]))
    .with_properties(chain_properties())
    .with_boot_nodes(vec![])
    .build())
}

// ============================================
// LOCAL TESTNET CONFIG
// ============================================

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Local Testnet")
    .with_id("lumenyx_local_testnet")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_patch(development_genesis(vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
    ]))
    .with_properties(chain_properties())
    .with_boot_nodes(vec![])
    .build())
}

// ============================================
// MAINNET CONFIG - FROZEN GENESIS (from raw chainspec)
// ============================================

/// REAL Mainnet configuration - PoW LongestChain
/// 
/// IMPORTANT: This loads a FROZEN raw chainspec to ensure
/// the genesis hash remains 0xd3b7...7676 regardless of
/// how the runtime is compiled.
///
/// LUMENYX - Decentralized Blockchain
/// - 21M supply (like Bitcoin)
/// - PoW consensus
/// - 2.5 second blocks
/// - EVM smart contracts
/// - No team, no governance, no authorities
///
/// Anyone can mine by running:
/// ./lumenyx-node --chain mainnet --mine
pub fn mainnet_config() -> Result<ChainSpec, String> {
    // Load FROZEN raw chainspec - genesis hash will always be 0xd3b7...7676
    let raw_spec = include_bytes!("chain-specs/lumenyx-mainnet-raw.json");
    ChainSpec::from_json_bytes(raw_spec.as_slice()).map_err(|e| format!("Failed to load mainnet spec: {}", e))
}

/// Testnet config - for testing before mainnet
pub fn testnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Testnet")
    .with_id("lumenyx_testnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(development_genesis(vec![]))
    .with_properties(chain_properties())
    .with_boot_nodes(vec![])
    .build())
}

// ============================================
// CHAIN PROPERTIES
// ============================================

fn chain_properties() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), 42.into());
    properties.insert("genesisMessage".into(), GENESIS_MESSAGE.into());
    properties
}
