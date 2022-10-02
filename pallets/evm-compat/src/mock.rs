use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::{
		self,
		traits::{
			BlakeTwo256, DispatchInfoOf, IdentityLookup, PostDispatchInfoOf, SignedExtension,
		},
	},
	traits::{ConstU64, Everything},
	weights::IdentityFee,
};

use frame_support::sp_runtime::Perbill;
use pallet_contracts::{
	weights::WeightInfo, AddressGenerator, DefaultAddressGenerator, DefaultContractAccessWeight,
};
use pallet_evm::HashedAddressMapping;
use pallet_transaction_payment::CurrencyAdapter;
use primitives::{AccountId, Balance, BlockNumber, CurrencyId, Hash, Header, Index};
use sp_core::{keccak_256, KeccakHasher, H256};

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
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
		)) / 5) as u32;
	pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
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

	type AddressGenerator = EvmCompatAdderssGenerator;

	// TODO: use arbitrary value now, need to adjust usage later
	type ContractAccessWeight = DefaultContractAccessWeight<()>;

	type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
	type RelaxedMaxCodeLen = ConstU32<{ 512 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
}

parameter_types! {
	pub const ProxyDepositBase: u64 = 1;
	pub const ProxyDepositFactor: u64 = 1;
	pub const MaxProxies: u16 = 4;
	pub const MaxPending: u32 = 2;
	pub const AnnouncementDepositBase: u64 = 1;
	pub const AnnouncementDepositFactor: u64 = 1;
}

impl pallet_proxy::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ();
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = ();
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

impl Config for Runtime {
	type BalanceConvert = BalanceConvert;

	type AddressMapping = HashedAddressMapping<KeccakHasher>;

	type ContractAddressMapping = PlainContractAddressMapping;

	type ChainId = ConstU64<100>;
}

pub struct PlainContractAddressMapping;

impl AddressMapping<AccountId> for PlainContractAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		let mut out = [0_u8; 32];

		out[0..12].copy_from_slice(&b"evm_contract"[..]);
		out[12..].copy_from_slice(&address.0);

		out.into()
	}
}

pub struct BalanceConvert;

impl Convert<U256, Balance> for BalanceConvert {
	fn convert(a: U256) -> Balance {
		a.as_u128()
	}
}

/// generate account address in H160 compatible form
pub struct EvmCompatAdderssGenerator;

type CodeHash<T> = <T as frame_system::Config>::Hash;

impl AddressGenerator<Runtime> for EvmCompatAdderssGenerator {
	fn generate_address(
		deploying_address: &<Runtime as frame_system::Config>::AccountId,
		code_hash: &CodeHash<Runtime>,
		salt: &[u8],
	) -> <Runtime as frame_system::Config>::AccountId {
		let generated = <DefaultAddressGenerator as AddressGenerator<Runtime>>::generate_address(
			deploying_address,
			code_hash,
			salt,
		);

		let raw: [u8; 32] = generated.into();

		let h_addr = H160::from_slice(&raw[0..20]);

		PlainContractAddressMapping::into_account_id(h_addr)
	}
}

construct_runtime!(

	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		// Sudo: pallet_sudo,

		Balances: pallet_balances,
		Contracts: pallet_contracts,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Payment: pallet_transaction_payment,
		Proxy: pallet_proxy,

		EvmCompat: crate
	}
);

pub struct AccountInfo {
	pub address: H160,
	pub account_id: AccountId,
	pub private_key: H256,
}

impl AccountInfo {
	fn from_seed(seed: &[u8]) -> Self {
		let private_key = H256::from_slice(seed); //H256::from_low_u64_be((i + 1) as u64);
		let secret_key = libsecp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
		let public_key = &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
		let address = H160::from(H256::from(keccak_256(public_key)));

		let mut data = [0u8; 32];
		data[0..20].copy_from_slice(&address[..]);

		AccountInfo {
			private_key,
			account_id: AccountId::from(Into::<[u8; 32]>::into(data)),
			address,
		}
	}
}

pub fn address_build(seed: u8) -> AccountInfo {
	let private_key = H256::from_slice(&[(seed + 0x11) as u8; 32]);
	let secret_key = libsecp256k1::SecretKey::parse_slice(&private_key[..]).unwrap();
	let public_key = &libsecp256k1::PublicKey::from_secret_key(&secret_key).serialize()[1..65];
	let address = H160::from(H256::from(keccak_256(public_key)));

	let prefix = b"evm";

	let mut data = [0u8; 32];
	data.copy_from_slice(prefix);

	data[0..20].copy_from_slice(&address[..]);

	AccountInfo { private_key, account_id: AccountId::from(Into::<[u8; 32]>::into(data)), address }
}

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);

parameter_types! {
	pub const ChainId: u64 = 42;
}

type Extra = ();

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = (H160, AccountId, (U256, U256));

	fn is_self_contained(&self) -> bool {
		match self {
			Call::EvmCompat(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::EvmCompat(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<TransactionValidity> {
		if let Call::EvmCompat(call) = self {
			return Some(().validate(&0, &(), &(), len))
		}

		None
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::EvmCompat(call) => Some(().pre_dispatch(&0, &(), &(), len)),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ Call::EvmCompat(_) =>
				Some(call.dispatch(Origin::from(crate::RawOrigin::EthereumTransaction(info.0)))),
			_ => None,
		}
	}
}

#[derive(Default)]
pub struct ExtBuilder {
	balances: Vec<(AccountId, Balance)>,
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		// construct test storage for the mock runtime
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self.balances.into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}
