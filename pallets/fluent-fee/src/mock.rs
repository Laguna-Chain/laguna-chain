use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
	unsigned::TransactionValidityError,
	weights::IdentityFee,
	PalletId,
};

use frame_system::EnsureRoot;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::LockIdentifier;
use pallet_contracts::{weights::WeightInfo, DefaultAddressGenerator, DefaultContractAccessWeight};
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::{H256, U256};
use sp_runtime::{DispatchError, Perbill};
use sp_std::ops::Deref;

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
		// TODO: all account are possible to be dust-removed now
		false
	}
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency: CurrencyId| -> Balance {
		Balance::min_value()
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

	type MaxLocks = MaxLocks;

	type DustRemovalWhitelist = DustRemovalWhitelist;

	type MaxReserves = ConstU32<2>;

	type ReserveIdentifier = [u8; 8];
}

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
pub const FEE_TOKEN_ID: CurrencyId = CurrencyId::NativeToken(TokenId::FeeToken);
pub const TREASURY_ACCOUNT: AccountId = AccountId::new([9u8; 32]);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
	pub const TreasuryAccount: AccountId = TREASURY_ACCOUNT;
	pub const LockId: LockIdentifier = ID_1;
	pub const MaxLocks: u32 = 10;

}

impl orml_currencies::Config for Runtime {
	// type Event = Event;

	type MultiCurrency = Tokens;

	type NativeCurrency = BasicCurrencyAdapter<Self, Balances, Amount, BlockNumber>;

	type GetNativeCurrencyId = NativeCurrencyId;

	type WeightInfo = ();
}

pub struct DummyFeeSource;

impl FeeSource for DummyFeeSource {
	type AssetId = CurrencyId;

	type Balance = Balance;

	fn accepted(id: &Self::AssetId) -> Result<(), DispatchError> {
		if let CurrencyId::NativeToken(TokenId::Laguna | TokenId::FeeToken) = id {
			Ok(())
		} else if FluentFee::accepted_assets(&id) {
			Ok(())
		} else {
			Err(DispatchError::Other("InvalidFeeSource: Unlisted"))
		}
	}

	fn listing_asset(id: &Self::AssetId) -> Result<(), DispatchError> {
		match id {
			CurrencyId::Erc20(_) => {
				pallet::AcceptedAssets::<Runtime>::insert(&id, true);
				Ok(())
			},
			CurrencyId::NativeToken(_) => {
				let staked_amount = FluentFee::total_staked(id);
				let total_supply = Tokens::total_issuance(id);

				if (staked_amount * 100 / total_supply) < 30 {
					Err(DispatchError::Other("InvalidFeeSource: Ineligible"))
				} else {
					pallet::AcceptedAssets::<Runtime>::insert(&id, true);
					Ok(())
				}
			},
		}
	}

	fn denounce_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}

	fn disable_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
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
			CurrencyId::Erc20(_) => Ok(balance.saturating_mul(70).saturating_div(100)),
			_ => Err(InvalidTransaction::Payment.into()),
		}
	}
}

pub struct DummyFeeDispatch<T> {
	_type: PhantomData<T>,
}

impl FeeDispatch<Runtime> for DummyFeeDispatch<Tokens> {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		native_balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), DispatchError> {
		let current_user_balance = FluentFee::treasury_balance_per_account(account);
		// Let the treasury pay the fee on behalf of the user if they have already prepaid
		if current_user_balance >= *native_balance {
			Tokens::withdraw(NATIVE_CURRENCY_ID, &TREASURY_ACCOUNT, *native_balance)?;
			let new_user_balance = current_user_balance - native_balance;
			pallet::TreasuryBalancePerAccount::<Runtime>::insert(&account, new_user_balance);
		}
		// If there doesn't exist enough balance for the user in the treasury make the user directly
		// pay for the transaction.
		else {
			match *id {
				CurrencyId::NativeToken(_) => Tokens::withdraw(*id, &account, *balance)?,
				CurrencyId::Erc20(asset_address) => ContractAssets::transfer(
					asset_address.into(),
					account.clone(),
					TREASURY_ACCOUNT,
					U256::from(*balance),
				)
				.map(|_| ())
				.map_err(|_| DispatchError::CannotLookup)?,
			}
		}
		Ok(())
	}

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
	) -> Result<(), traits::fee::InvalidFeeSource> {
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

impl pallet::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Tokens;

	type Call = Call;

	type FeeSource = DummyFeeSource;
	type FeeMeasure = DummyFeeMeasure;
	type FeeDispatch = DummyFeeDispatch<Tokens>;

	type TreasuryAccount = TreasuryAccount;

	// type NativeCurrencyId = NativeCurrencyId;

	type LockId = LockId;
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
		Balances: pallet_balances,
		Currencies: orml_currencies,
		FluentFee: pallet,
		Payment: pallet_transaction_payment,
		Contracts: pallet_contracts,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		ContractAssets: pallet_contract_asset_registry
	}
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";

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
				.filter(|(_, currency_id, _)| *currency_id != NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
