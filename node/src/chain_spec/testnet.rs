// import runtime primitives and modules
use primitives::AccountId;

use runtime::{
    // provided by construct_runtime! macro
    AuraConfig,
    BalancesConfig,
    GenesisConfig,
    GrandpaConfig,
    SchedulerConfig,
    SudoConfig,
    SystemConfig,
    WASM_BINARY,
};

use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;

use super::util::{authority_keys_from_seed, get_account_id_from_seed};
use sc_service::ChainType;
use sp_core::sr25519;

// Spec derived from runtiem GenisisConfig
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| -> String { "dev runtime wasm blob missing".into() })?;

    // create genesis state from preconfigured accounts
    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ], // prefund accounts
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn devnet_config() -> Result<ChainSpec, String> {
    unimplemented!()
}

// TODO: adjust when expanding runtime
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
    // generated by construct_runtime! macro
    GenesisConfig {
        system: SystemConfig {
            // add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        aura: AuraConfig {
            // allowed sudo account to participate in PoA with AURA
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        },
        grandpa: GrandpaConfig {
            // allowed sudo account to participate in block finalization with GRANDPA
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        },
        balances: BalancesConfig {
            // pre-fund test accounts
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        },
        transaction_payment: Default::default(),
        sudo: SudoConfig {
            // assign network admin rights.
            key: root_key,
        },
        scheduler: Default::default(),
    }
}
