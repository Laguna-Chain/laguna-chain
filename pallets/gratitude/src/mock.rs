use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
};

use frame_support::pallet_prelude::*;
use orml_currencies::BasicCurrencyAdapter;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Hydro);
pub const GRATITUDE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::GratitudeToken);

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
	pub const NativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
	pub const GratitudeCurrency: CurrencyId = GRATITUDE_CURRENCY_ID;
	pub const GratitudeAccountId: AccountId = ALICE;
	pub const MaxReasonLength: u32 = 128;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(_t: &AccountId) -> bool {
		false
	}
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency: CurrencyId| -> Balance {
		1000
	};
}

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
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Self, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = NativeCurrencyId;
	type WeightInfo = ();
}

impl Config for Runtime {
	type Event = Event;
	type Call = Call;
	type MultiCurrency = Tokens;
	type GratitudeAccountId = GratitudeAccountId;
	type GratitudeCurrency = GratitudeCurrency;
	type MaxReasonLength = MaxReasonLength;
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Tokens: orml_tokens,
		Balances: pallet_balances,
		Currencies: orml_currencies,
		Gratitude: pallet,
	}
);

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: vec![
				(ALICE, GRATITUDE_CURRENCY_ID, 10000),
				(BOB, GRATITUDE_CURRENCY_ID, 20000),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
