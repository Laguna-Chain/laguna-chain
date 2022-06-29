use anyhow::Result;
use subxt::{self, ClientBuilder, DefaultConfig, PolkadotExtrinsicParams};

#[cfg(feature = "local-metadata")]
pub mod import_runtime {
    #[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
	pub mod laguna_runtime {}
}

#[cfg(feature = "local-metadata")]
use crate::import_runtime::laguna_runtime;

pub type LagunaRuntimeApi =
	laguna_runtime::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>>;

// construct an api to connect to the running substrate node, metadata should exist at compile time
#[cfg(feature = "local-metadata")]
pub async fn runtime_from_local_metadata() -> Result<LagunaRuntimeApi> {
	let api: LagunaRuntimeApi = ClientBuilder::new().build().await?.to_runtime_api();
	Ok(api)
}
pub mod erc20_fee;
pub mod native_fee;
