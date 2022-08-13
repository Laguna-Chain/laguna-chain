use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
};

use frame_system::EnsureRoot;
use orml_traits::LockIdentifier;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;

	type BlockWeights = ();

	type BlockLength = ();

	type Origin = Origin;

	type Call = Call;

	type Index = Index;

	type BlockNumber = BlockNumber;

	type Hash = H256;

	type Hashing = BlakeTwo256;

	type AccountId = AccountId;

	type Lookup = IdentityLookup<Self::AccountId>;

	type Header = Header;

	type Event = Event;

	type BlockHashCount = BlockHashCount;

	type DbWeight = ();

	type Version = ();

	type PalletInfo = PalletInfo;

	type AccountData = orml_tokens::AccountData<Balance>;

	type OnNewAccount = ();

	type OnKilledAccount = ();

	type SystemWeightInfo = ();

	type SS58Prefix = ();

	type OnSetCode = ();

	type MaxConsumers = ConstU32<1>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 2;
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(_t: &AccountId) -> bool {
		// TODO: all account are possible to be dust-removed now
		false
	}
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency: CurrencyId| -> Balance {
		Balance::min_value()
	};
}

pub type ReserveIdentifier = [u8; 8];

impl orml_tokens::Config for Runtime {
	type Event = Event;

	type Balance = Balance;

	type Amount = Amount;

	type CurrencyId = CurrencyId;

	type WeightInfo = ();

	type ExistentialDeposits = ExistentialDeposits;

	type OnDust = ();

	type MaxLocks = ();

	type DustRemovalWhitelist = DustRemovalWhitelist;

	type MaxReserves = ConstU32<2>;

	type ReserveIdentifier = ReserveIdentifier;
}

pub struct DummyImpl;

impl FeeAssetHealth for DummyImpl {
	type AssetId = CurrencyId;

	fn health_status(asset_id: &Self::AssetId) -> Result<(), traits::fee::HealthStatusError> {
		match asset_id {
			// a made-up case where not enough liquidity exist for the target asset
			CurrencyId::NativeToken(TokenId::FeeToken) => {
				if Tokens::total_issuance(asset_id) > 1000 {
					Ok(())
				} else {
					Err(traits::fee::HealthStatusError::Unstable)
				}
			},
			_ => Ok(()),
		}
	}
}

impl Eligibility for DummyImpl {
	type AccountId = AccountId;

	type AssetId = CurrencyId;

	fn eligible(
		who: &Self::AccountId,
		asset_id: &Self::AssetId,
	) -> Result<(), traits::fee::EligibilityError> {
		match (who, asset_id) {
			(&BOB, CurrencyId::NativeToken(TokenId::FeeToken)) =>
				Err(traits::fee::EligibilityError::NotAllowed),
			_ => Ok(()),
		}
	}
}

impl Config for Runtime {
	type MultiCurrency = Tokens;
	type AllowedOrigin = EnsureRoot<AccountId>;

	type HealthStatus = DummyImpl;

	type Eligibility = DummyImpl;

	type WeightInfo = ();
}

construct_runtime!(

	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		FeeEnablement: crate,
		Tokens: orml_tokens

	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";

#[derive(Default)]
pub struct ExtBuilder {
	enabled: Vec<(CurrencyId, bool)>,
}

impl ExtBuilder {
	pub fn enabled(mut self, enabled: Vec<(CurrencyId, bool)>) -> Self {
		self.enabled = enabled;

		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		if !self.enabled.is_empty() {
			GenesisBuild::<Runtime>::assimilate_storage(
				&crate::GenesisConfig { enabled: self.enabled },
				&mut t,
			)
			.expect("unable to build genesis");
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
