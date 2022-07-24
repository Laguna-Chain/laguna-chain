use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, Convert, IdentityLookup},
	traits::{ConstU32, Contains, EnsureOneOf, Everything},
	unsigned::TransactionValidityError,
	weights::{IdentityFee, WeightToFeePolynomial},
	PalletId,
};

use frame_system::{EnsureRoot, EnsureSigned, RawOrigin};
use pallet_contracts::{weights::WeightInfo, DefaultAddressGenerator, DefaultContractAccessWeight};
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::{H256, U256};
use sp_runtime::Perbill;
use traits::{
	currencies::TokenAccess,
	fee::{FeeDispatch, FeeMeasure, FeeSource},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	// pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::simple_max(2_000_000_000_000);
}

use sp_std::cell::RefCell;
thread_local! {
	static EXTRINSIC_BASE_WEIGHT: RefCell<u64> = RefCell::new(3);
}

// Copied directly from the pallet_transaction_payment's mock runtime implementation
// TODO: Change it as per our needs
// pub struct BlockWeights;
// impl Get<frame_system::limits::BlockWeights> for BlockWeights {
// 	fn get() -> frame_system::limits::BlockWeights {
// 		frame_system::limits::BlockWeights::builder()
// 			.base_block(0)
// 			.for_class(DispatchClass::all(), |weights| {
// 				weights.base_extrinsic = EXTRINSIC_BASE_WEIGHT.with(|v| *v.borrow()).into();
// 			})
// 			.for_class(DispatchClass::non_mandatory(), |weights| {
// 				weights.max_total = 1024.into();
// 			})
// 			.build_or_panic()
// 	}
// }

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;

	type BlockWeights = BlockWeights;

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
	fn contains(t: &AccountId) -> bool {
		// TODO: all accounts are possible to be dust-removed now
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

impl pallet_currencies::Config for Runtime {
	// type Event = Event;

	type MultiCurrency = Tokens;
	type ContractAssets = ContractAssets;

	type NativeCurrencyId = NativeAssetId;

	type ConvertIntoAccountId = AccountConvert;
}

pub struct AccountConvert;

impl Convert<[u8; 32], AccountId> for AccountConvert {
	fn convert(a: [u8; 32]) -> AccountId {
		a.into()
	}
}

// Configure FluentFee
pub struct DummyFeeSource;

impl FeeSource for DummyFeeSource {
	type AccountId = AccountId;
	type AssetId = CurrencyId;

	fn accepted(
		who: &Self::AccountId,
		id: &Self::AssetId,
	) -> Result<(), traits::fee::InvalidFeeSource> {
		match id {
			CurrencyId::NativeToken(TokenId::FeeToken | TokenId::Laguna) => Ok(()),
			// Accept any ERC token for testing purposes
			CurrencyId::Erc20(_) => Ok(()),
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
	}

	fn listed(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		match id {
			CurrencyId::NativeToken(TokenId::FeeToken | TokenId::Laguna) => Ok(()),
			CurrencyId::Erc20(_) => Ok(()),
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
	}
}

pub struct DummyFeeMeasure;

impl FeeMeasure for DummyFeeMeasure {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, TransactionValidityError> {
		match id {
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(balance),
			// demo 5% reduction
			CurrencyId::NativeToken(TokenId::FeeToken) =>
				Ok(balance.saturating_mul(95).saturating_div(100)),
			CurrencyId::Erc20(_) => Ok(balance.saturating_mul(70).saturating_div(100)),
			_ => Err(InvalidTransaction::Payment.into()),
		}
	}
}

pub struct DummyFeeDispatch<T> {
	_type: PhantomData<T>,
}

pub trait CallFilter<T>
where
	T: frame_system::Config,
{
	fn filter_call_type(call: &<T as frame_system::Config>::Call) -> CallType;
	fn estimate_fee(call: &<T as frame_system::Config>::Call) -> Balance;
}

pub enum CallType {
	ScheduleCallCharge,
	ScheduleCallExec,
	NormalCall,
}

impl CallFilter<Runtime> for DummyFeeDispatch<Tokens> {
	fn filter_call_type(call: &<Runtime as frame_system::Config>::Call) -> CallType {
		match call {
			Call::Scheduler(pallet::Call::<Runtime>::schedule_call { .. }) =>
				CallType::ScheduleCallCharge,
			Call::Scheduler(pallet::Call::<Runtime>::schedule_call_exec { .. }) =>
				CallType::ScheduleCallExec,
			_ => CallType::NormalCall,
		}
	}

	fn estimate_fee(call: &<Runtime as frame_system::Config>::Call) -> Balance {
		match call {
			Call::Scheduler(pallet::Call::<Runtime>::schedule_call {
				when,
				call,
				id,
				maybe_periodic,
				priority,
			}) => {
				let info = call.get_dispatch_info();
				// Base Fee Calculation: find capped base extrinsic weight , then compute
				// weight_to_fee.
				let fee = TransactionPayment::compute_fee(call.encoded_size() as u32, &info, 0);
				// Get the second element from the maybe_periodic tuple
				let num_times_to_execute: u128 = match maybe_periodic {
					None => 1,
					Some((_, num)) => *num as u128 + 1,
				};
				// charge twice the estimated fee to ensure that calls don't fail during volatile
				// times
				let total_fee = fee * num_times_to_execute;
				// dbg!("Inside estimate_fee {}", total_fee.clone());
				return total_fee
			},
			_ => 0,
		}
	}
}

impl FeeDispatch<Runtime> for DummyFeeDispatch<Tokens> {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		call: &<Runtime as frame_system::Config>::Call,
		balance: &Self::Balance,
		reason: &frame_support::traits::WithdrawReasons,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		match Self::filter_call_type(call) {
			// Pre-charge the estimated schedule call tx fee
			CallType::ScheduleCallCharge => {
				// dbg!(BlockWeights::get().max_block);
				// Get the estimated fee to be paid upfront
				let fee_estimate = Self::estimate_fee(call);
				// Get the origin's Laguna token balance
				let user_locked_fund_balance = Scheduler::scheduled_locked_funds_balances(account);
				// Return error if the user has insufficient funds
				if user_locked_fund_balance < fee_estimate {
					// dbg!("Insufficient funds");
					return Err(traits::fee::InvalidFeeDispatch::InsufficientBalance)
				}
				// Transfer `fee_estimate` amount of Laguna tokens from the extrinsic origin's
				// ScheduleLockedFundAccountId to SchedulePrepayAccountId
				<Tokens as MultiCurrency<AccountId>>::transfer(
					*id,
					&<Runtime as pallet::Config>::ScheduleLockedFundAccountId::get(),
					&<Runtime as pallet::Config>::SchedulePrepayAccountId::get(),
					fee_estimate,
				)
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;

				let updated_user_locked_fund_balance = user_locked_fund_balance - fee_estimate;
				// dbg!("Updating locked funds balance {}",
				// updated_user_locked_fund_balance.clone()); dbg!(balance.clone());
				// dbg!(id.clone());
				// Update the user's locked funds balances after precharging for the future
				// scheduled call
				pallet::ScheduleLockedFundBalances::<Runtime>::mutate(account, |balance| {
					*balance = updated_user_locked_fund_balance
				});
				// Update the user's scheduled call prepay balance
				// This information is needed to check if the has enough balance for the scheduled
				// call and for repayment of the remaining balance after executing all scheduled
				// calls
				let user_prepay_balance = Scheduler::schedule_prepay_balances(account);
				let updated_prepay_balance = user_prepay_balance + fee_estimate;
				pallet::SchedulePrepayBalances::<Runtime>::mutate(account, |balance| {
					*balance = updated_prepay_balance
				});

				// Also charge the tx fees for the transaction
				Tokens::withdraw(*id, account, *balance)
					.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
				// dbg!("Tx fee withdraw for scheduled call");
			},
			// Executing the actual scheduled calls
			CallType::ScheduleCallExec => {
				// Get the prepaid balance for the scheduled call, if it cannot cover the fees then
				// halt the call, otherwise proceed executing it.
				let scheduled_call_prepaid_balance = Scheduler::schedule_prepay_balances(account);
				if scheduled_call_prepaid_balance < *balance {
					// TODO: halt scheduled call, but unsure how to get the Scheduled struct's info
					// in here to add it to the queue :(
					return Err(traits::fee::InvalidFeeDispatch::InsufficientBalance)
				}

				// withdraw transaction fee from the scheduled call's prepaid balance
				Tokens::withdraw(
					*id,
					&<Runtime as pallet::Config>::SchedulePrepayAccountId::get(),
					*balance,
				)
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
				// Update the prepaid balance after paying for the transaction
				let updated_scheduled_call_prepaid_balance =
					scheduled_call_prepaid_balance - *balance;

				pallet::SchedulePrepayBalances::<Runtime>::insert(
					account,
					updated_scheduled_call_prepaid_balance,
				);
			},
			CallType::NormalCall => {
				// withdraw fees directly from the origin's balance
				match *id {
					CurrencyId::NativeToken(_) => Tokens::withdraw(*id, account, *balance)
						.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?,
					CurrencyId::Erc20(asset_address) => {
						ContractAssets::transfer(
							asset_address.into(),
							account.clone().into(),
							BOB.into(),
							U256::from(*balance),
						)
						.map(|_| ())
						.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
					},
				}
			},
		}
		Ok(())
	}

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		Ok(())
	}
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
	type Currency = Balances;
	type Event = Event;
	type Call = Call;

	type CallFilter = frame_support::traits::Nothing;
	type WeightPrice = TransactionPayment;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;

	type DepositPerByte = DepositPerByte;

	type DepositPerItem = DepositPerItem;

	type AddressGenerator = DefaultAddressGenerator;

	// TODO: use arbitrary value now, need to adjust usage later
	type ContractAccessWeight = DefaultContractAccessWeight<()>;
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

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
parameter_types! {
	pub const NativeAssetId: CurrencyId = NATIVE_CURRENCY_ID;
}

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type DefaultFeeAsset = NativeAssetId;

	type MultiCurrency = Tokens;

	type FeeSource = DummyFeeSource;
	type FeeMeasure = DummyFeeMeasure;
	type FeeDispatch = DummyFeeDispatch<Tokens>;
}

parameter_types! {
	pub const ScheduleLockedFundAccountId: AccountId = SCHEDULE_LOCKED_FUND_ACCOUNTID;
	pub const SchedulePrepayAccountId: AccountId = SCHEDULE_PREPAY_ACCOUNTID;
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl Config for Runtime {
	type Event = Event;

	type Call = Call;

	type Origin = Origin;

	type ScheduleOrigin = EnsureSigned<AccountId>;

	type ScheduleLockedFundAccountId = ScheduleLockedFundAccountId;

	type MultiCurrency = Tokens;

	type NativeAssetId = NativeAssetId;

	type SchedulePrepayAccountId = SchedulePrepayAccountId;

	type PalletsOrigin = OriginCaller;

	type MaxScheduledPerBlock = ConstU32<2>;

	type MaxScheduledCallRetries = ConstU32<2>;

	type MaxScheduledCallErrors = ConstU32<2>;

	type MaximumWeight = MaximumSchedulerWeight;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = FluentFee;

	type OperationalFeeMultiplier = OperationalFeeMultiplier;

	type WeightToFee = IdentityFee<Balance>;

	type FeeMultiplierUpdate = ();

	type LengthToFee = IdentityFee<Balance>;
}

construct_runtime!(
	pub enum Runtime where
	Block = Block,
	NodeBlock = Block,
	UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Tokens: orml_tokens,
		TransactionPayment: pallet_transaction_payment,
		FluentFee: pallet_fluent_fee,
		Scheduler: pallet,
		Contracts: pallet_contracts,
		Timestamp: pallet_timestamp,
		ContractAssets: pallet_contract_asset_registry,
		Currencies: pallet_currencies,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Balances: pallet_balances
	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId::new([3u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
pub const BURN_ACCOUNT: AccountId = AccountId::new([0u8; 32]);
pub const SCHEDULE_LOCKED_FUND_ACCOUNTID: AccountId = AccountId::new([9u8; 32]);
pub const SCHEDULE_PREPAY_ACCOUNTID: AccountId = AccountId::new([10u8; 32]);

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

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
