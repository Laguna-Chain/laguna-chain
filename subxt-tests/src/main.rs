use primitives::AccountId;
use sp_keyring::AccountKeyring;
use subxt::{self, Client, ClientBuilder, DefaultConfig, PairSigner, SubstrateExtrinsicParams};
use tokio::{
	self,
	time::{sleep, Duration},
};
use tracing_subscriber;

#[subxt::subxt(runtime_metadata_path = "./metadata.scale")]
pub mod polkadot {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt::init();

	let alice = PairSigner::new(AccountKeyring::Alice.pair());

	let client: Client<DefaultConfig> =
		ClientBuilder::new().set_url("ws://127.0.0.1:9944").build().await?;
	let client_copy: Client<DefaultConfig> =
		ClientBuilder::new().set_url("ws://127.0.0.1:9944").build().await?;

	let api = client
		.to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, SubstrateExtrinsicParams<DefaultConfig>>>(
		);

	// let client_api: LagunaRuntimeApi = ClientBuilder::new().build().await?.to_runtime_api();

	let prepayed_amount: u128 = 100000000000;
	let native_token = polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
		polkadot::runtime_types::primitives::currency::TokenId::Laguna,
	);

	// let treasury_account = [9u8; 32];
	let treasury_account = &polkadot::fluent_fee::constants::ConstantsApi::new(&client_copy)
		.treasury_account()
		.unwrap();

	let treasury_balance_before_prepay = api
		.storage()
		.tokens()
		.accounts(&treasury_account, &native_token, None)
		.await?
		.free;

	// Prepay to the treasury
	api.tx()
		.fluent_fee()
		.prepay_fees(
			polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			prepayed_amount,
		)?
		.sign_and_submit_then_watch_default(&alice)
		.await?
		// .wait_for_finalized()
		.wait_for_in_block()
		.await?;

	// wait for a while for the block to get finalized
	// sleep(Duration::from_millis(100)).await;

	let prepayed_amount_from_storage = api
		.storage()
		.fluent_fee()
		.treasury_balance_per_account(&alice.account_id(), None)
		.await?;

	let treasury_balance_after_prepay = api
		.storage()
		.tokens()
		.accounts(
			&treasury_account,
			&polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			None,
		)
		.await?
		.free;

	println!(
		"Treasury balance before prepay, after prepay, from storage: {}, {}, {}",
		treasury_balance_before_prepay, treasury_balance_after_prepay, prepayed_amount_from_storage
	);
	// Alice's prepaid funds must go to the treasury's balance
	assert!(treasury_balance_after_prepay >= treasury_balance_before_prepay);
	assert!(treasury_balance_after_prepay == prepayed_amount_from_storage);

	println!(
		"Treasury balance before prepay, after prepay: {}, {}",
		treasury_balance_before_prepay, treasury_balance_after_prepay
	);

	let alice_balance_before_tx = api
		.storage()
		.tokens()
		.accounts(
			&alice.account_id(),
			&polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			None,
		)
		.await?
		.free;

	api.tx()
		.fluent_fee()
		.prepay_fees(
			polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			prepayed_amount,
		)?
		.sign_and_submit_then_watch_default(&alice)
		.await?
		// .wait_for_finalized()
		.wait_for_in_block()
		.await?;

	// sleep(Duration::from_millis(100)).await;

	let treasury_balance_after_tx = api
		.storage()
		.tokens()
		.accounts(
			&treasury_account,
			&polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			None,
		)
		.await?
		.free;

	let alice_balance_after_tx = api
		.storage()
		.tokens()
		.accounts(
			&alice.account_id(),
			&polkadot::runtime_types::primitives::currency::CurrencyId::NativeToken(
				polkadot::runtime_types::primitives::currency::TokenId::Laguna,
			),
			None,
		)
		.await?
		.free;
	// Alice's balance must remain the same as the Treasury will pay for the tx on behalf of Alice
	assert!(alice_balance_before_tx == alice_balance_after_tx);
	// Treasury balance must decrease as it pays for the tx cost on behalf of Alice
	println!(
		"Treasury balance before tx, after tx: {}, {}",
		treasury_balance_after_prepay, treasury_balance_after_tx
	);
	assert!(treasury_balance_after_prepay > treasury_balance_after_tx);
	Ok(())
}

// 1. ./target/release/laguna-node --dev --base-path /tmp/n1 --alice --node-key
// 0000000000000000000000000000000000000000000000000000000000000001 --validator --port 30333
// 2. ./target/release/laguna-node purge-chain --base-path /tmp/n2 --dev
// 3. ./target/release/laguna-node --dev --base-path /tmp/n2 --bob --port 30334 --validator
// // --bootnodes
// ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp

// pub fn main() {}
