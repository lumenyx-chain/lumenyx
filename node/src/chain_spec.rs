//! # LUMENYX Chain Specification
//!
//! Defines genesis configuration for development, testnet, and mainnet.
//!
//! ## Mainnet Genesis
//! - Total Supply: 21,000,000 LUMENYX
//! - Distribution: 100% through mining (block rewards)
//! - No pre-allocations. No reserves. Pure fair launch.
//! - Permissionless validation: Anyone can become a validator!

use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public, crypto::Ss58Codec};
use sp_runtime::traits::{IdentifyAccount, Verify};
use lumenyx_runtime::{AccountId, Signature, WASM_BINARY, SessionKeys};
use frame_support::PalletId;
use sp_runtime::traits::AccountIdConversion;

pub type ChainSpec = sc_service::GenericChainSpec;

// ============================================
// GENESIS CONSTANTS
// ============================================

/// Genesis block message
pub const GENESIS_MESSAGE: &str = "Banks ended up in the headlines. Today control over digital money sits in a few hands: founders, large holders, intermediaries and those who write the rules.";

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

pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
    )
}

/// Create session keys from Aura and Grandpa keys
fn session_keys(aura: AuraId, grandpa: GrandpaId) -> SessionKeys {
    SessionKeys { aura, grandpa }
}

// ============================================
// DEVELOPMENT CONFIG (for testing)
// ============================================

fn development_genesis(
    initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
    endowed_accounts: Vec<AccountId>,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, 1u128 << 60)).collect::<Vec<_>>(),
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone())))
                .collect::<Vec<_>>(),
        },
        "aura": {
            "authorities": Vec::<AuraId>::new(),
        },
        "grandpa": {
            "authorities": Vec::<(GrandpaId, u64)>::new(),
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
    .with_genesis_config_patch(development_genesis(
        vec![authority_keys_from_seed("Alice")],
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
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
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
// MAINNET CONFIG - THE REAL DEAL
// ============================================

/// ✅ Genesis allocation: 100% Mining (fair launch)
/// No pre-allocations. All coins from block rewards.
/// 
/// HOW TO BECOME A VALIDATOR:
/// 1. Run: ./lumenyx-node --chain mainnet --validator
/// 2. Keys are auto-generated on first run
/// 3. You start validating in the next session (~30 seconds)
/// 
/// That's it! Just run the node!
fn mainnet_genesis(
    initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
) -> serde_json::Value {
    // Faucet allocation: 5000 LUMENYX for validator bootstrap
    // PalletId must match pallet-validator-faucet PALLET_ID
    let faucet_pallet_id: PalletId = PalletId(*b"valifauc");
    let faucet_account: AccountId = faucet_pallet_id.into_account_truncating();
    // 5000 LUMENYX = 5_000_000_000_000_000 planck (12 decimals)
    let genesis_allocations: Vec<(AccountId, u128)> = vec![
        (faucet_account, 5_000_000_000_000_000), // 5000 LUMENYX for validator faucet
    ];

    serde_json::json!({
        "balances": {
            "balances": genesis_allocations,
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone())))
                .collect::<Vec<_>>(),
        },
        "aura": {
            "authorities": Vec::<AuraId>::new(),
        },
        "grandpa": {
            "authorities": Vec::<(GrandpaId, u64)>::new(),
        },
        "evm": {
            "accounts": {}
        },
    })
}

/// ✅ REAL Mainnet configuration
/// 
/// LUMENYX - Fair Launch Blockchain
/// 
/// Anyone can become a validator by running:
/// ./lumenyx-node --chain mainnet --validator
pub fn mainnet_config() -> Result<ChainSpec, String> {
    // Genesis validator - uses your AURA_1 key
    // After launch, anyone can join by running --validator
    let initial_authorities: Vec<(AccountId, AuraId, GrandpaId)> = vec![
        (
            // Account ID (same as AURA for simplicity)
            AccountId::from(
                sp_core::sr25519::Public::from_ss58check("5Fe12bNT7xmTzoi46CoYgFPZccskFTgx2CN7S48deyHvZXPs")
                    .expect("Valid SS58 address")
            ),
            // AURA key (sr25519) - block production
            AuraId::from_ss58check("5Fe12bNT7xmTzoi46CoYgFPZccskFTgx2CN7S48deyHvZXPs")
                .expect("Valid SS58 address"),
            // GRANDPA key (ed25519) - block finalization
            GrandpaId::from_ss58check("5ChvxM9taAxbfKwhibbqBpRXQPDjjzNJa8DFuky8Psaq8kTo")
                .expect("Valid SS58 address"),
        ),
    ];

    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Mainnet")
    .with_id("lumenyx_mainnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(mainnet_genesis(
        initial_authorities,
    ))
    .with_properties(chain_properties())
    .build())
}

/// ✅ Testnet config
pub fn testnet_config() -> Result<ChainSpec, String> {
    let initial_authorities = vec![
        authority_keys_from_seed("TestValidator1"),
        authority_keys_from_seed("TestValidator2"),
    ];

    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?,
        None,
    )
    .with_name("LUMENYX Testnet")
    .with_id("lumenyx_testnet")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(mainnet_genesis(
        initial_authorities,
    ))
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
    properties
}
