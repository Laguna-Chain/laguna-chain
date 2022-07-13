use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{ConstU32, ConstU8, Contains, EnsureOneOf, Everything},
	unsigned::TransactionValidityError,
	weights::{IdentityFee, WeightToFeePolynomial},
};
use frame_system::{Account, EnsureRoot, EnsureSigned, RawOrigin};
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;
use sp_runtime::Perbill;
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	// pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::simple_max(2_000_000_000_000);
}

use sp_std::cell::RefCell;
thread_local! {
	static EXTRINSIC_BASE_WEIGHT: RefCell<u64> = RefCell::new(0);
}

// Copied directly from the pallet_transaction_payment's mock runtime implementation
// TODO: Change it as per our needs
pub struct BlockWeights;
impl Get<frame_system::limits::BlockWeights> for BlockWeights {
	fn get() -> frame_system::limits::BlockWeights {
		frame_system::limits::BlockWeights::builder()
			.base_block(0)
			.for_class(DispatchClass::all(), |weights| {
				weights.base_extrinsic = EXTRINSIC_BASE_WEIGHT.with(|v| *v.borrow()).into();
			})
			.for_class(DispatchClass::non_mandatory(), |weights| {
				weights.max_total = 1024.into();
			})
			.build_or_panic()
	}
}

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

	type AccountData = orml_tokens::AccountData<Balance>;

	type OnNewAccount = ();

	type OnKilledAccount = ();

	type SystemWeightInfo = ();

	type SS58Prefix = ();

	type OnSetCode = ();

	type MaxConsumers = ConstU32<1>;
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
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
	}

	fn listed(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
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
				maybe_periodic,
				priority,
			}) => {
				let info = call.get_dispatch_info();
				// Base Fee Calculation: find capped base extrinsic weight , then compute
				// weight_to_fee.
				let base_weight: Weight = (<Runtime as frame_system::Config>::BlockWeights::get()
					.get(info.class)
					.base_extrinsic)
					.min(<Runtime as frame_system::Config>::BlockWeights::get().max_block);
				let base_fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::calc(
					&base_weight,
				);
				// Compute the len fee
				let len_fee = <Runtime as pallet_transaction_payment::Config>::LengthToFee::calc(
					&(call.encoded_size() as u32 as Weight),
				);
				// Get the average next multiplier fee
				let avg_next_multiplier_fee = Scheduler::avg_next_fee_multiplier();
				// Compute the weight fee
				let weight_fee = <Runtime as pallet_transaction_payment::Config>::WeightToFee::calc(
					&info
						.weight
						.min(<Runtime as frame_system::Config>::BlockWeights::get().max_block),
				);
				// Get the second element from the maybe_periodic tuple
				let num_times_to_execute = maybe_periodic.unwrap().1 as u128;
				let total_fee = base_fee + len_fee + (avg_next_multiplier_fee * weight_fee);
				return total_fee * 2u128
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
				// Get the estimated fee to be paid upfront
				let fee_estimate = Self::estimate_fee(call);
				// Get the origin's Laguna token balance
				let user_locked_fund_balance = Scheduler::scheduled_locked_funds_balances(account);
				// Return error if the user has insufficient funds
				if user_locked_fund_balance < fee_estimate {
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
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute);

				let updated_user_locked_fund_balance = user_locked_fund_balance - fee_estimate;
				// Update the user's locked funds balances after precharging for the future
				// scheduled call
				pallet::ScheduleLockedFundBalances::<Runtime>::insert(
					account,
					updated_user_locked_fund_balance,
				);
				// Update the user's scheduled call prepay balance
				// This information is needed to check if the has enough balance for the scheduled
				// call and for repayment of the remaining balance after executing all scheduled
				// calls
				let user_prepay_balance = Scheduler::schedule_prepay_balances(account);
				let updated_prepay_balance = user_prepay_balance + fee_estimate;
				pallet::SchedulePrepayBalances::<Runtime>::insert(account, updated_prepay_balance);

				// Also charge the tx fees for the transaction
				Tokens::withdraw(*id, account, *balance)
					.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute);
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
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute);
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
				Tokens::withdraw(*id, account, *balance)
					.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute);
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

const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
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

	type MaxScheduledPerBlock = ConstU32<10>;

	type MaxScheduledCallRetries = ConstU8<10>;

	type MaximumWeight = MaximumSchedulerWeight;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = FluentFee;

	type OperationalFeeMultiplier = ();

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
	}
);
pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
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

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
