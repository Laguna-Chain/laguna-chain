use crate as pallet_system_contract_deployer;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::{
		self,
		traits::{BlakeTwo256, IdentityLookup},
	},
	traits::{Everything, GenesisBuild},
	weights::IdentityFee,
};

use frame_support::{
	pallet_prelude::{ConstU32, Weight},
	PalletId,
};
use pallet_contracts::{weights::WeightInfo, DefaultContractAccessWeight};
use pallet_system_contract_deployer::CustomAddressGenerator;
use pallet_transaction_payment::CurrencyAdapter;
use primitives::{AccountId, Balance, BlockNumber, Hash, Header, Index};
use sp_runtime::Perbill;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

impl frame_system::Config for Test {
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

impl pallet_sudo::Config for Test {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 2;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl pallet_randomness_collective_flip::Config for Test {}

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Test {
	type Event = Event;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
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
			<Test as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
			<Test as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
		)) / 5) as u32;
	pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
}

impl pallet_contracts::Config for Test {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
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
	type AddressGenerator = CustomAddressGenerator;
	type ContractAccessWeight = DefaultContractAccessWeight<()>;

	type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
	type RelaxedMaxCodeLen = ConstU32<{ 512 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
}

parameter_types! {
	pub const PId: PalletId = PalletId(*b"sys_depl");
}

impl pallet_system_contract_deployer::Config for Test {
	type Event = Event;
	type PalletId = PId;
}

construct_runtime!(

	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Sudo: pallet_sudo,
		Balances: pallet_balances,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Payment: pallet_transaction_payment,
		Contracts: pallet_contracts,
		SudoContracts: pallet_system_contract_deployer
	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);

pub struct ExtBuilder {
	balances: Vec<(AccountId, Balance)>,
	sudo: Option<AccountId>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![], sudo: None }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn sudo(mut self, account: AccountId) -> Self {
		self.sudo.replace(account);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		pallet_balances::GenesisConfig::<Test> {
			balances: self.balances.clone().into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		// setup sudo account
		if let Some(key) = self.sudo {
			// FIXME #1578 make this available through chainspec
			pallet_sudo::GenesisConfig::<Test> { key: Some(key) }
				.assimilate_storage(&mut t)
				.unwrap();
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
