use std::path::Path;

use super::*;

// use crate::adapter::CurrencyAdapter;

use codec::Decode;
use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Currency, Everything},
	weights::IdentityFee,
	PalletId,
};

use frame_system::EnsureRoot;
use pallet_contracts::{weights::WeightInfo, DefaultAddressGenerator, DefaultContractAccessWeight};
use primitives::{AccountId, Amount, Balance, BlockNumber, Hash, Header, Index, TokenId};
use sp_runtime::Perbill;

use orml_tokens::CurrencyAdapter as TokenCurrencyAdapter;

use pallet_transaction_payment::CurrencyAdapter as PaymentCurrencyAdapter;

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

	type Hash = Hash;

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

// impl pallet_balances::Config for Runtime {
// 	type Balance = Balance;
// 	type DustRemoval = ();
// 	type Event = Event;
// 	type ExistentialDeposit = ExistentialDeposit;
// 	type AccountStore = frame_system::Pallet<Runtime>;
// 	type MaxLocks = ();
// 	type MaxReserves = ();
// 	type ReserveIdentifier = [u8; 8];
// 	type WeightInfo = ();
// }

impl pallet_randomness_collective_flip::Config for Runtime {}

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	// TODO: add benchmark around cross pallet interaction between fee
	type OnChargeTransaction =
		PaymentCurrencyAdapter<TokenCurrencyAdapter<Runtime, NativeCurrencyId>, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();

	type LengthToFee = IdentityFee<Balance>;
}

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

pub const UNIT: u128 = 100_000_000_000_000_000;
const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT + (bytes as Balance) * (5 * UNIT / 10000 / 100)) / 10
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

const WEIGHT_PER_SECOND: Weight = 1_000_000_000_000;

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
	::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
		BlockWeights::get().max_block;
	pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
		)) / 5) as u32;
	pub Schedule: pallet_contracts::Schedule<Runtime> = {
		let mut schedule = pallet_contracts::Schedule::<Runtime>::default();
		schedule.limits.code_len = 256 * 1024;
		schedule
	};
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = TokenCurrencyAdapter<Runtime, NativeCurrencyId>;
	type Event = Event;
	type Call = Call;

	type CallFilter = frame_support::traits::Nothing;
	type WeightPrice = Payment;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;

	type DepositPerByte = DepositPerByte;

	type DepositPerItem = DepositPerItem;

	type AddressGenerator = DefaultAddressGenerator;

	type ContractAccessWeight = DefaultContractAccessWeight<()>;
}

parameter_types! {
	pub const PId: PalletId = PalletId(*b"tkn/reg_");
	pub const MaxGas: u64 = 200_000_000_000;
	pub const DebugFlag: bool = true;
}

impl pallet_contract_asset_registry::Config for Runtime {
	type AllowedOrigin = EnsureRoot<AccountId>;

	type PalletId = PId;

	type MaxGas = MaxGas;

	type ContractDebugFlag = DebugFlag;

	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Balance::MIN
	};
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(_t: &AccountId) -> bool {
		false
	}
}

pub type ReserveIdentifier = [u8; 8];

impl orml_tokens::Config for Runtime {
	type Event = Event;
	// how tokens are measured
	type Balance = Balance;
	type Amount = Amount;

	// how tokens are represented
	type CurrencyId = primitives::CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ();
	type DustRemovalWhitelist = DustRemovalWhitelist;

	type MaxReserves = ConstU32<2>;

	type ReserveIdentifier = ReserveIdentifier;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
}

impl Config for Runtime {
	// type NativeCurrency = CurrencyAdapter<Runtime, NativeCurrencyId>;
	type NativeCurrencyId = NativeCurrencyId;

	type MultiCurrency = Tokens;
	type ContractAssets = ContractTokenRegistry;
	type ConvertIntoAccountId = AccountConvert;
}

pub struct AccountConvert;

impl Convert<[u8; 32], AccountId> for AccountConvert {
	fn convert(a: [u8; 32]) -> AccountId {
		a.into()
	}
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,

		Contracts: pallet_contracts,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Payment: pallet_transaction_payment,

		Tokens: orml_tokens,
		Currencies: crate,
		ContractTokenRegistry: pallet_contract_asset_registry,
	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);

#[derive(Default)]
pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
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

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
