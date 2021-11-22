// include runtime primitives and modules
use runtime::{
    AccountId,
    // provided by construct_runtime! macro
    AuraConfig,
    GenesisConfig,
    GrandpaConfig,
    Signature,
    SudoConfig,
    SystemConfig,
    WASM_BINARY,
};

// Spec derived from runtiem GenisisConfig
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    unimplemented!()
}

pub fn devnet_config() -> Result<ChainSpec, String> {
    unimplemented!()
}

// fn testnet_genesis(
//     wasm_binary: &[u8],
//     initial_authorities: Vec<(AuraId, GrandpaId)>,
//     root_key: AccountId,
//     endowed_accounts: Vec<AccountId>,
//     _enable_println: bool,
// ) -> GenesisConfig {
//     unimplemented!()
// }
