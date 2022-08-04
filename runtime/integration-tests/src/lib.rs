use frame_support::traits::GenesisBuild;
use laguna_runtime::{Runtime, System};
use primitives::{AccountId, Balance, CurrencyId, TokenId};

pub mod contracts;
pub mod fees;
pub mod native_token;

pub const BURN_ADDR: AccountId = AccountId::new([0u8; 32]);
pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
pub const FEE_TOKEN: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
	sudo: Option<AccountId>,
	fee_sources: Vec<(CurrencyId, bool)>,
	deploying_key: Option<AccountId>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![], sudo: None, fee_sources: vec![], deploying_key: None }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn enable_fee_source(mut self, fee_sources: Vec<(CurrencyId, bool)>) -> Self {
		self.fee_sources = fee_sources;
		self
	}

	pub fn sudo(mut self, sudo: AccountId) -> Self {
		self.sudo.replace(sudo);
		self
	}

	pub fn deploying_key(mut self, key: AccountId) -> Self {
		self.deploying_key.replace(key);
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

		if !self.fee_sources.is_empty() {
			<pallet_fee_enablement::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
				&pallet_fee_enablement::GenesisConfig { enabled: self.fee_sources.clone() },
				&mut t,
			)
			.unwrap();
		}

		// set deploying_key for pallet_system_contract_deployer
		match self.deploying_key {
			Some(key) => pallet_system_contract_deployer::GenesisConfig::<Runtime> {
				deploying_key: key,
				..Default::default()
			},
			None => pallet_system_contract_deployer::GenesisConfig::<Runtime>::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
