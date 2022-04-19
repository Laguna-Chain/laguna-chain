use frame_support::traits::GenesisBuild;
use hydro_runtime::{Runtime, System};
use pallet_evm::AddressMapping;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_core::H160;

pub mod native_token;

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Hydro);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
	sudo: Option<AccountId>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![], sudo: None }
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

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		// prefund token_balances for tester accounts
		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.clone()
				.into_iter()
				.chain(self.balances.clone().into_iter())
				.filter(|(_, currency_id, _)| *currency_id != NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		// setup sudo account
		if let Some(key) = self.sudo {
			// FIXME #1578 make this available through chainspec
			pallet_sudo::GenesisConfig::<Runtime> { key: Some(key) }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
