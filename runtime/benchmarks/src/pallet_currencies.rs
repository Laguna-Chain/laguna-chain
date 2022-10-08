#[allow(unused)]
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
	dispatch::{Encode, HasCompact},
	traits::tokens::currency::Currency,
};
use frame_system::{Config as SysConfig, Origin, RawOrigin};
use laguna_runtime::{Contracts, Event, Systems};
use mock_runtime;
use pallet_contract_asset_registry;
use pallet_contracts::chain_extension::UncheckedFrom;
use pallet_currencies::Pallet as Currencies;
use primitives::{AccountId, CurrencyId, TokenId};
use sp_core::{Bytes, U256};
use std::str::FromStr;

const SEED: u32 = 0;
const INIT_BALANCE: u128 = 1000_000;

fn create_token<T, U>(
	owner: T::AccountId,
	tkn_name: &str,
	tkn_symbol: &str,
	init_amount: U,
) -> AccountId
where
	T: pallet_contracts::Config + pallet_contract_asset_registry::Config,
	<<<T as pallet_contracts::Config>::Currency as Currency<<T as SysConfig>::AccountId>>::Balance as HasCompact>::Type: Clone + std::cmp::Eq + PartialEq + std::fmt::Debug + TypeInfo + Encode,
	<T as pallet_contracts::Config>::Currency: Currency<<T as SysConfig>::AccountId, Balance = u128>,
	<T as frame_system::Config>::AccountId: UncheckedFrom<<T as SysConfig>::Hash> + AsRef<[u8]>,
// <T as SysConfig>::Origin: From<RawOrigin<AccountId32>>,
	U256: From<U>,
{
	let blob = std::fs::read(
		"../../runtime/integration-tests/contracts-data/solidity/token/dist/DemoToken.wasm",
	)
	.expect("unable to read contract");

	let mut sel_constuctor = Bytes::from_str("0x835a15cb")
		.map(|v| v.to_vec())
		.expect("unable to parse selector");

	sel_constuctor.append(&mut tkn_name.encode());
	sel_constuctor.append(&mut tkn_symbol.encode());
	sel_constuctor.append(&mut U256::from(init_amount).encode());

	Contracts::instantiate_with_code(
		Origin::Signed(owner).into(),
		0,
		<T as pallet_contract_asset_registry::Config>::MaxGas::get(),
		None, /* if not specified, it's allowed to charge the max amount of free balance of the
		       * creator */
		blob,
		sel_constuctor,
		vec![],
	)
	.expect("Error instantiating the code");

	let evts = Systems::events();
	let deployed = evts
		.iter()
		.rev()
		.find_map(|rec| {
			if let Event::Contracts(pallet_contracts::Event::Instantiated {
				deployer: _,
				contract,
			}) = &rec.event
			{
				Some(contract)
			} else {
				None
			}
		})
		.expect("unable to find deployed contract");

	deployed.clone()
}

benchmarks! {
	where_clause {
		where
		T: crate::Config,
		T::MultiCurrency: MultiCurrency<T::AccountId, CurrencyId = CurrencyId, Balance = u128>
	}
	// Transfer native tokens, presumably it is less expensive in terms
	// of gas than ERC20 token transfers which are contract calls
	transfer {
		let caller: T::AccountId = whitelisted_caller();
		let a in 0..100000;
		let recipient: T::AccountId = account("recipient", 0, SEED);

		// Deposit some free balance for the caller
		<T::MultiCurrency as MultiCurrency<T::AccountId>>::deposit(
			T::NativeCurrencyId::get(),
			&caller,
			INIT_BALANCE.clone(),
		)?;
		// Currency to transfer
		let currency_id: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	}: _(RawOrigin::Signed(caller.clone()), recipient.clone(), currency_id.clone(), a.clone().into())
	verify {
		assert_eq!(<T::MultiCurrency as MultiCurrency<T::AccountId>>::free_balance(currency_id.clone(), &recipient), a.into());
		assert_eq!(<T::MultiCurrency as MultiCurrency<T::AccountId>>::free_balance(currency_id.clone(), &caller), INIT_BALANCE - u128::from(a));
	}

	impl_benchmark_test_suite!(Currencies, mock_runtime::ExtBuilder::default().build(), mock_runtime::Test);
}

#[cfg[test]]
pub mod mock_runtime {

	use frame_support::{
		construct_runtime, parameter_types,
		sp_runtime::traits::{BlakeTwo256, IdentityLookup},
		traits::{Contains, Everything},
		weights::IdentityFee,
		PalletId,
	};

	use frame_support::sp_runtime::Perbill;
	use frame_system::EnsureRoot;
	use pallet_contracts::{
		weights::WeightInfo, DefaultAddressGenerator, DefaultContractAccessWeight,
	};
	use primitives::{AccountId, Amount, Balance, BlockNumber, Hash, Header, Index, TokenId};

	use orml_tokens::CurrencyAdapter as TokenCurrencyAdapter;

	use pallet_transaction_payment::CurrencyAdapter as PaymentCurrencyAdapter;

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
		// TODO: add benchmark around cross pallet interaction between fee
		type Event = Event;
		type OnChargeTransaction =
			PaymentCurrencyAdapter<TokenCurrencyAdapter<Test, NativeCurrencyId>, ()>;
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
		type Currency = TokenCurrencyAdapter<Test, NativeCurrencyId>;
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

		type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
		type RelaxedMaxCodeLen = ConstU32<{ 512 * 1024 }>;
		type MaxStorageKeyLen = ConstU32<128>;
	}

	parameter_types! {
		pub const PId: PalletId = PalletId(*b"tkn/reg_");
		pub const MaxGas: u64 = 200_000_000_000;
		pub const DebugFlag: bool = true;
	}

	impl pallet_contract_asset_registry::Config for Test {
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

	impl orml_tokens::Config for Test {
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

		type OnNewTokenAccount = ();

		type OnKilledTokenAccount = ();
	}

	parameter_types! {
		pub const NativeCurrencyId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
	}

	impl pallet_currencies::Config for Test {
		// type NativeCurrency = CurrencyAdapter<Test, NativeCurrencyId>;
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
		pub enum Test where
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
			let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

			orml_tokens::GenesisConfig::<Test> {
				balances: self.balances.into_iter().collect::<Vec<_>>(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			let mut ext = sp_io::TestExternalities::new(t);
			ext.execute_with(|| System::set_block_number(1));

			ext
		}
	}
}
