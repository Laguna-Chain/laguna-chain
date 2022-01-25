use std::collections::BTreeMap;

use frame_support::traits::GenesisBuild;
use hydro_runtime::{Runtime, System};
use pallet_evm::AddressMapping;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_core::H160;

pub mod native_token;
pub mod native_token_precompile;

// only enable this branch of testing if required solidity tooling is present
#[cfg(feature = "evm")]
pub mod evm_compat;

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Hydro);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
	evm_balances: Vec<(H160, CurrencyId, Balance)>,
	sudo: Option<AccountId>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![], evm_balances: vec![], sudo: None }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn sudo(mut self, sudo: AccountId) -> Self {
		self.sudo.replace(sudo);
		self
	}

	pub fn evm_balances(mut self, balances: Vec<(H160, CurrencyId, Balance)>) -> Self {
		self.evm_balances = balances;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		// prefund native_blances for tester accounts
		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.evm_balances
				.clone()
				.into_iter()
				.map(|(address, currency_id, amount)| {
					let acc =
						<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(address);
					(acc, currency_id, amount)
				})
				.chain(self.balances.clone().into_iter())
				.filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		// prefund token_balances for tester accounts
		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.evm_balances
				.clone()
				.into_iter()
				.map(|(address, currency_id, amount)| {
					let acc =
						<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(address);
					(acc, currency_id, amount)
				})
				.chain(self.balances.clone().into_iter())
				.filter(|(_, currency_id, _)| *currency_id != NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		// setup sudo account
		if let Some(key) = self.sudo {
			pallet_sudo::GenesisConfig::<Runtime> { key }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
