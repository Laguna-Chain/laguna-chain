use super::{laguna_runtime, LagunaRuntimeApi};
use anyhow::{Ok, Result};
use sp_keyring::AccountKeyring;
use subxt::{self, DefaultConfig, PairSigner};

pub struct NativeFeeRunner<'a> {
	api: &'a LagunaRuntimeApi,
}
use laguna_runtime::runtime_types::primitives::currency::{
	CurrencyId::NativeToken as c_id, TokenId::Laguna as t_id,
};

impl<'a> NativeFeeRunner<'a> {
	pub fn from_api(api: &'a LagunaRuntimeApi) -> Self {
		Self { api }
	}

	pub async fn run(&self) -> Result<()> {
		let alice = PairSigner::<DefaultConfig, _>::new(AccountKeyring::Alice.pair());
		let prepaid_amount: u128 = 100000000000;
		// Native token of Laguna Chain
		let native_token = c_id(t_id);
		// Treasury account address
		let treasury_account = self.api.constants().fluent_fee().treasury_account().unwrap();

		// Initial treasury balance before any user prepaid it to cover their transaction costs
		let treasury_balance_before_prepay = self
			.api
			.storage()
			.tokens()
			.accounts(&treasury_account, &native_token, None)
			.await?
			.free;
		// fund the treasury with some laguna tokens which can later be used as prepayment
		// to cover transaction gas fees
		// prepay the treasury using Alice's balance
		self.api
			.tx()
			.fluent_fee()
			.prepay_fees(c_id(t_id), prepaid_amount)?
			.sign_and_submit_then_watch_default(&alice)
			.await?
			.wait_for_in_block()
			.await?;

		// check if pallet-fluent-fee's manual accounting reflects the prepayment from Alice by
		// fetching the storage
		let prepaid_amount_from_storage = self
			.api
			.storage()
			.fluent_fee()
			.treasury_balance_per_account(&alice.account_id(), None)
			.await?;

		// check the treasury's laguna account balance after the prepayment
		let treasury_balance_after_prepay = self
			.api
			.storage()
			.tokens()
			.accounts(&treasury_account, &c_id(t_id), None)
			.await?
			.free;

		println!("Treasury balance before/after: {treasury_balance_before_prepay} / {treasury_balance_after_prepay}");
		assert!(treasury_balance_after_prepay >= treasury_balance_before_prepay);
		assert!(treasury_balance_after_prepay == prepaid_amount_from_storage);

		/* The below block checks if Alice's balance is being used for covering transaction fees
		or the treasury is covering on behalf of Alice */

		// alice's balance before newer transactions, i.e., current balance serves as reference.
		let alice_balance_before_tx = self
			.api
			.storage()
			.tokens()
			.accounts(&alice.account_id(), &c_id(t_id), None)
			.await?
			.free;

		// Let alice make an arbitrary extrinsic call which costs gas. Ideally, if the treasury
		// holds enough balance on behalf of alice, then it will be used for covering alice's
		// transaction fee

		// TODO: call an extrinsic

		let treasury_balance_after_tx = self
			.api
			.storage()
			.tokens()
			.accounts(&treasury_account, &c_id(t_id), None)
			.await?
			.free;

		let alice_balance_after_tx = self
			.api
			.storage()
			.tokens()
			.accounts(&alice.account_id(), &c_id(t_id), None)
			.await?
			.free;
		// Alice's balance must remain the same as the Treasury will pay for the tx on behalf of
		// Alice
		println!(
			"Alice balance before tx, after tx: {}, {}",
			alice_balance_before_tx, alice_balance_after_tx
		);
		assert!(alice_balance_before_tx == alice_balance_after_tx);
		// Treasury balance must decrease as it pays for the tx cost on behalf of Alice
		println!(
			"Treasury balance before tx, after tx: {}, {}",
			treasury_balance_after_prepay, treasury_balance_after_tx
		);
		assert!(treasury_balance_after_prepay > treasury_balance_after_tx);

		Ok(())
	}
}
