use anyhow::Result;
use tokio;
use tracing_subscriber;

#[cfg(feature = "local-metadata")]
use subxt_tests::{native_fee, runtime_from_local_metadata};

#[tokio::main]
async fn main() -> Result<()> {
	#[cfg(feature = "local-metadata")]
	{
		tracing_subscriber::fmt::init();
		let api = runtime_from_local_metadata().await?;

		let native_fee_runner = native_fee::NativeFeeRunner::from_api(&api);
		native_fee_runner.run().await?;
	}
	Ok(())
}
