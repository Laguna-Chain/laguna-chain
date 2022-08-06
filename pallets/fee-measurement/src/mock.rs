use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
};

use orml_traits::{DataProvider, DefaultPriceProvider, LockIdentifier};
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, Price};
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

	type AccountData = pallet_balances::AccountData<Balance>;

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

pub struct DummyProvider;

impl DataProvider<CurrencyId, Price> for DummyProvider {
	fn get(key: &CurrencyId) -> Option<Price> {
		None
	}
}

parameter_types! {
	pub static ConvertRate: Price = Price::checked_from_rational(11, 10).unwrap();
}

impl Config for Runtime {
	type PrepaidConversionRate = ConvertRate;
	type AltConversionRate = DefaultPriceProvider<CurrencyId, DummyProvider>;
}

construct_runtime!(

	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Tokens: orml_tokens,
		FeeMeasurement: crate,

	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

#[derive(Default)]
pub struct ExtBuilder {}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
