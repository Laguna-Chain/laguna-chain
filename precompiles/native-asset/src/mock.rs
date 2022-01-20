use super::*;

use fp_evm::PrecompileSet;
use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::Everything,
};

use pallet_evm::{
	EnsureAddressNever, EnsureAddressRoot, HashedAddressMapping, SubstrateBlockHashMapping,
};

use pallet_evm_precompile_dispatch::Dispatch;
use primitives::{AccountId, Balance, BlockNumber, Header, Index};
use sp_core::{H160, H256};

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
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 2;
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

pub struct Precompiles<R>(PhantomData<R>);

impl<Runtime> Precompiles<Runtime>
where
	Runtime: pallet_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> sp_std::vec::Vec<H160> {
		sp_std::vec![1].into_iter().map(|x| hash(x)).collect()
	}
}

impl<Runtime> PrecompileSet for Precompiles<Runtime>
where
	Runtime: pallet_evm::Config + pallet_balances::Config,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	sp_core::U256: From<<Runtime as pallet_balances::Config>::Balance>,
{
	fn execute(
		&self,
		address: H160,
		input: &[u8],
		target_gas: Option<u64>,
		context: &Context,
		is_static: bool,
	) -> Option<PrecompileResult> {
		match address {
			a if a == hash(1) => Some(NativeCurrencyPrecompile::<Runtime>::execute(
				input, target_gas, context, is_static,
			)),
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		Self::used_addresses().contains(&address)
	}
}

parameter_types! {
	// TODO: setup correct rule for chain_id
	pub PrecompilesValue: Precompiles<Runtime> = Precompiles::<_>::new();
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = ();

	type GasWeightMapping = ();

	type BlockHashMapping = SubstrateBlockHashMapping<Self>;

	type CallOrigin = EnsureAddressRoot<AccountId>;

	type WithdrawOrigin = EnsureAddressNever<AccountId>;

	type AddressMapping = HashedAddressMapping<BlakeTwo256>;

	type Currency = Balances;

	type Event = Event;

	type PrecompilesType = Precompiles<Self>;

	type PrecompilesValue = PrecompilesValue;

	type ChainId = ();

	type BlockGasLimit = ();

	type Runner = pallet_evm::runner::stack::Runner<Self>;

	type OnChargeTransaction = ();

	type FindAuthor = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;

	type OnTimestampSet = ();

	type MinimumPeriod = MinimumPeriod;

	type WeightInfo = ();
}

construct_runtime!(

	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Evm: pallet_evm,
	}
);

// ss58 account_id
pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);

// evm H160 address
pub fn alice() -> H160 {
	H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
}

pub fn bob() -> H160 {
	H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2])
}

// ss58 account_id backed by H160
pub fn mapped_alice() -> AccountId {
	HashedAddressMapping::<BlakeTwo256>::into_account_id(alice())
}

pub fn mapped_bob() -> AccountId {
	HashedAddressMapping::<BlakeTwo256>::into_account_id(alice())
}

pub struct ExtBuilder {
	balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn balances_evm(mut self, balances: Vec<(H160, Balance)>) -> Self {
		self.balances = balances
			.into_iter()
			.map(|(a, val)| (HashedAddressMapping::<BlakeTwo256>::into_account_id(a), val))
			.collect::<Vec<_>>();
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self.balances.clone().into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

// generate address at position `a` into H160 hex string
pub fn hash(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}

// empty evm context
pub fn context() -> Context {
	Context {
		address: Default::default(),
		caller: Default::default(),
		apparent_value: From::from(0),
	}
}
