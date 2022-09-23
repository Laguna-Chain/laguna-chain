use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::{
		self,
		traits::{BlakeTwo256, IdentityLookup},
	},
	traits::{Contains, Everything},
	unsigned::TransactionValidityError,
	weights::IdentityFee,
	PalletId,
};

use orml_traits::LockIdentifier;
use primitives::{
	AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, Price, TokenId,
};
use sp_core::H256;

use sp_runtime::{FixedPointNumber, FixedU128};
use traits::fee::CallFilterWithOutput;

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

	type OnNewTokenAccount = ();

	type OnKilledTokenAccount = ();
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

pub struct DummyValueAddedCall;

impl CallFilterWithOutput for DummyValueAddedCall {
	type Call = Call;

	type Output = Option<(AccountId, Balance)>;

	fn is_call(call: &Self::Call) -> Self::Output {
		if let Call::FluentFee(pallet::Call::<Runtime>::fluent_fee_wrapper {
			value_added_info,
			..
		}) = call
		{
			value_added_info.to_owned()
		} else {
			None
		}
	}
}

pub struct DummyCarrierCall;

impl CallFilterWithOutput for DummyCarrierCall {
	type Call = Call;

	type Output = Option<(AccountId, Vec<u8>, Balance, Weight, Option<Balance>, bool)>;

	fn is_call(call: &Self::Call) -> Self::Output {
		if let Call::FluentFee(pallet::Call::<Runtime>::fluent_fee_wrapper {
			carrier_info, ..
		}) = call
		{
			carrier_info.to_owned()
		} else {
			None
		}
	}
}

// alias
type IsSharingCall<T> = <T as pallet::Config>::IsFeeSharingCall;

pub struct DummyFeeDispatch<T> {
	_type: PhantomData<T>,
}

impl FeeDispatch for DummyFeeDispatch<Tokens> {
	type AccountId = AccountId;
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		// If the transaction is of fee sharing type, transfer unit weight worth of fees to the
		// given beneficiary

		match id {
			CurrencyId::NativeToken(_) => Tokens::withdraw(*id, account, *balance)
				.map_err(|_| traits::fee::InvalidFeeDispatch::UnresolvedRoute),
			CurrencyId::Erc20(_) => unimplemented!("erc20 need carrier not enabled right now"),
		}
	}

	fn refund(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
	) -> Result<Self::Balance, traits::fee::InvalidFeeDispatch> {
		Tokens::withdraw(*id, account, *balance)
			.map(|_| *balance)
			.map_err(|_| traits::fee::InvalidFeeDispatch::UnresolvedRoute)
	}

	fn post_info_correction(
		id: &Self::AssetId,
		tip: &Self::Balance,
		corret_withdrawn: &Self::Balance,
		value_added_fee: &Option<(AccountId, Balance)>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		let payouts = corret_withdrawn.saturating_sub(*tip);

		let ratio = FixedU128::saturating_from_rational(2_u128, 100_u128);

		if let Some((target, amount)) = value_added_fee {
			Tokens::deposit(*id, target, *amount)
				.map_err(|_| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
		}

		Ok(())
	}
}

impl FeeCarrier for DummyFeeDispatch<Runtime> {
	type AccountId = AccountId;
	type Balance = Balance;

	fn execute_carrier(
		account: &Self::AccountId,
		carrier_addr: &Self::AccountId,
		carrier_data: frame_support::sp_std::vec::Vec<u8>,
		value: Self::Balance,
		gas_limit: Weight,
		storage_deposit_limit: Option<Self::Balance>,
		required: Self::Balance,
		post_transfer: bool,
	) -> Result<Self::Balance, traits::fee::InvalidFeeDispatch> {
		todo!()
	}
}

parameter_types! {
	pub const NativeAssetId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);

	pub const PALLETID: PalletId = PalletId(*b"lgn/carr");

}

pub struct PayoutSplits;

impl Get<(Price, Price)> for PayoutSplits {
	fn get() -> (Price, Price) {
		(
			FixedPointNumber::saturating_from_rational(49_u128, 100_u128),
			FixedPointNumber::saturating_from_rational(49_u128, 100_u128),
		)
	}
}

impl Config for Runtime {
	type Event = Event;

	type DefaultFeeAsset = NativeAssetId;

	type MultiCurrency = Tokens;
	type Call = Call;

	type IsFeeSharingCall = DummyValueAddedCall;

	type FeeSource = DummyFeeSource;
	type FeeMeasure = DummyFeeMeasure;
	type FeeDispatch = DummyFeeDispatch<Tokens>;

	type Ratio = Price;

	type PayoutSplits = PayoutSplits;

	type IsCarrierAttachedCall = DummyCarrierCall;

	type PalletId = PALLETID;

	type Carrier = DummyFeeDispatch<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type Event = Event;

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
