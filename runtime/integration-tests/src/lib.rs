use frame_support::traits::GenesisBuild;
use laguna_runtime::{Runtime, System};
use primitives::{AccountId, Balance, CurrencyId, TokenId};

pub mod contracts;
pub mod native_token;

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);

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

		// prefund alternative token balances for tester accounts
		orml_tokens::GenesisConfig::<Runtime> {
			balances: self.balances.clone().into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		// setup sudo account
		if let Some(key) = self.sudo {
			pallet_sudo::GenesisConfig::<Runtime> { key: Some(key) }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
