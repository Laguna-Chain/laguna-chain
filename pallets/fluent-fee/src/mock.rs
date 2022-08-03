use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything},
	unsigned::TransactionValidityError,
	weights::IdentityFee,
	dispatch::DispatchInfo,
};

use orml_currencies::BasicCurrencyAdapter;
use orml_traits::LockIdentifier;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;

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
}

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
		Ok(())
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

pub enum CallType {
	FeeSharingCall,
	NormalCall,
}

pub trait CallFilter<T>
where
	T: frame_system::Config,
{
	fn filter_call_type(call: &<T as frame_system::Config>::Call) -> CallType;
	fn get_beneficiary_account(call: &<T as frame_system::Config>::Call) -> Option<AccountId>;
}

pub struct DummyFeeDispatch<T> {
	_type: PhantomData<T>,
}

impl CallFilter<Runtime> for DummyFeeDispatch<Tokens> {
	fn filter_call_type(call: &<Runtime as frame_system::Config>::Call) -> CallType {
		match call {
			Call::FluentFee(pallet::Call::<Runtime>::fee_sharing_wrapper { .. }) =>
				CallType::FeeSharingCall,
			_ => CallType::NormalCall,
		}
	}
	fn get_beneficiary_account(
		call: &<Runtime as frame_system::Config>::Call,
	) -> Option<AccountId> {
		match call {
			Call::FluentFee(pallet::Call::<Runtime>::fee_sharing_wrapper {
				beneficiary, ..
			}) => beneficiary.clone(),
			_ => None,
		}
	}
}

impl FeeDispatch<Runtime> for DummyFeeDispatch<Tokens> {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		Ok(())
	}

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		call: &<Runtime as frame_system::Config>::Call,
		balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		match Self::filter_call_type(call) {
			CallType::FeeSharingCall => {
				let beneficiary_exists = Self::get_beneficiary_account(&call);
				match beneficiary_exists {
					Some(beneficiary_account) => {
						// Get the fee equivalent to 1 unit of weight that can be shared with the
						// beneficiary
						let fee_details = Payment::compute_fee_details(
							0,
							&DispatchInfo {
								pays_fee: Pays::Yes,
								weight: 1u64,
								class: DispatchClass::Normal,
							},
							0,
						);
						// unpack the fee_details struct to get the adjusted_weight_fee
						let unit_weight_fee =
							fee_details.inclusion_fee.unwrap().adjusted_weight_fee;
						// let unit_weight_fee = <Runtime as
						// pallet_transaction_payment::Config>::WeightToFee::weight_to_fee(1u64);
						// Send the unit weight fee to the beneficiary account
						<Tokens as MultiCurrency<AccountId>>::transfer(
							*id,
							account,
							&beneficiary_account,
							unit_weight_fee,
						)
						.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
						// normal transaction fee withdrawal
						// NOTE: `balance` already includes `unit_weight_fee` as computed in the
						// SignedExtension, so we need to subtract that amount before paying the
						// validators
						Tokens::withdraw(*id, account, *balance - unit_weight_fee)
							.map_err(|err| traits::fee::InvalidFeeDispatch::UnresolvedRoute)
					},
					None => Tokens::withdraw(*id, account, *balance)
						.map_err(|err| traits::fee::InvalidFeeDispatch::UnresolvedRoute),
				}

				// Ok(())
			},
			CallType::NormalCall => Tokens::withdraw(*id, account, *balance)
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute),
		}
	}
}

parameter_types! {
	pub const NativeAssetId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);
}

impl Config for Runtime {
	type Event = Event;

	type DefaultFeeAsset = NativeAssetId;

	type MultiCurrency = Tokens;
	type Call = Call;

	type FeeSource = DummyFeeSource;
	type FeeMeasure = DummyFeeMeasure;
	type FeeDispatch = DummyFeeDispatch<Tokens>;
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
		FluentFee: pallet,
		Payment: pallet_transaction_payment
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

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self.balances.into_iter().collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
