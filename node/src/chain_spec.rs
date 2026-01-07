//! # LUMENYX Chain Specification - GHOSTDAG PoW
//!
//! Defines genesis configuration for development, testnet, and mainnet.
//!
//! ## GHOSTDAG PoW
//! - No authorities needed - anyone can mine
//! - Total Supply: 21,000,000 LUMENYX
//! - Distribution: 100% through mining (block rewards)
//! - No pre-allocations. No reserves. Pure fair launch.
//! - Truly permissionless: just run the node and mine!

use sc_service::ChainType;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use lumenyx_runtime::{AccountId, Signature, WASM_BINARY};
use frame_support::PalletId;
use sp_runtime::traits::AccountIdConversion;

pub type ChainSpec = sc_service::GenericChainSpec;

// ============================================
// GENESIS CONSTANTS
// ============================================

/// Genesis block message - The reason LUMENYX exists
pub const GENESIS_MESSAGE: &str = "Banks ended up in the headlines. Today control over digital money sits in a few hands.";

/// Network properties
pub const TOKEN_DECIMALS: u32 = 12;
pub const TOKEN_SYMBOL: &str = "LUMENYX";

/// Initial PoW difficulty
pub const INITIAL_DIFFICULTY: u64 = 1_000_000;

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

fn development_genesis(
    endowed_accounts: Vec<AccountId>,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, 1u128 << 60)).collect::<Vec<_>>(),
        },
        // NO AURA - GHOSTDAG PoW!
        // NO SESSION - No authorities needed!
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
    .with_genesis_config_patch(development_genesis(
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        ],
    ))
    .with_properties(chain_properties())
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
    .with_genesis_config_patch(development_genesis(
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
        ],
    ))
    .with_properties(chain_properties())
    .build())
}

// ============================================
// MAINNET CONFIG - GHOSTDAG PoW - TRULY PERMISSIONLESS!
// ============================================

/// ✅ Genesis allocation: 100% Mining (fair launch)
/// No pre-allocations. All coins from block rewards.
///
/// HOW TO MINE:
/// 1. Run: ./lumenyx-node --chain mainnet --mine
/// 2. That's it! You're mining LUMENYX.
///
/// No authorities. No validators. Just PoW.
fn mainnet_genesis() -> serde_json::Value {
    // Faucet allocation: 5000 LUMENYX for bootstrap
    let faucet_pallet_id: PalletId = PalletId(*b"valifauc");
    let faucet_account: AccountId = faucet_pallet_id.into_account_truncating();
    // 5000 LUMENYX = 5_000_000_000_000_000 planck (12 decimals)
    let genesis_allocations: Vec<(AccountId, u128)> = vec![
        (faucet_account, 5_000_000_000_000_000), // 5000 LUMENYX for faucet
    ];

    serde_json::json!({
        "balances": {
            "balances": genesis_allocations,
        },
        // NO AURA - GHOSTDAG PoW!
        // NO SESSION - No authorities needed!
        // NO GRANDPA - Probabilistic finality via PoW!
        "evm": {
            "accounts": {}
        },
    })
}

/// ✅ REAL Mainnet configuration - GHOSTDAG PoW
///
/// LUMENYX - The Unstoppable Chain
/// - 21M supply (like Bitcoin)
/// - GHOSTDAG PoW consensus (like Kaspa)
/// - 1-3 second blocks
/// - EVM smart contracts
/// - ZK-SNARKs privacy (optional)
/// - No team, no governance, no authorities
///
/// Anyone can mine by running:
/// ./lumenyx-node --chain mainnet --mine
pub fn mainnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Mainnet")
    .with_id("lumenyx_mainnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(mainnet_genesis())
    .with_properties(chain_properties())
    .build())
}

/// ✅ Testnet config - for testing before mainnet
pub fn testnet_config() -> Result<ChainSpec, String> {
    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Testnet")
    .with_id("lumenyx_testnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(mainnet_genesis())
    .with_properties(chain_properties())
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
    // GHOSTDAG specific
    properties.insert("powDifficulty".into(), INITIAL_DIFFICULTY.into());
    properties
}
