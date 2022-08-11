use super::*;

use frame_support::{
	construct_runtime,
	dispatch::DispatchInfo,
	parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
	unsigned::TransactionValidityError,
	weights::IdentityFee,
	PalletId,
};

use orml_traits::LockIdentifier;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;

use sp_runtime::{FixedPointNumber, FixedU128};
use traits::fee::IsFeeSharingCall;

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
	fn contains(t: &AccountId) -> bool {
		// TODO: all account are possible to be dust-removed now
		false
	}
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency: CurrencyId| -> Balance {
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

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
pub const FEE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
	pub const PrepaidCurrencyId: CurrencyId = FEE_CURRENCY_ID;

	pub const PALLETID: PalletId = PalletId(*b"pretoken");

}

pub struct MaxRatio;
impl Get<FixedU128> for MaxRatio {
	fn get() -> FixedU128 {
		FixedU128::saturating_from_rational(20_u128, 100_u128)
	}
}

impl Config for Runtime {
	type Event = Event;

	type MaxPrepaidRaio = MaxRatio;

	type MultiCurrency = Tokens;

	type NativeCurrencyId = NativeCurrencyId;

	type PrepaidCurrencyId = PrepaidCurrencyId;

	type PalletId = PALLETID;
}

construct_runtime!(

	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Tokens: orml_tokens,
		PrepaidFee: pallet,
	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self.balances.into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
