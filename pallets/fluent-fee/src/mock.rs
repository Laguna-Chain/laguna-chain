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
};

use crate::weights::SubstrateWeight;

use orml_currencies::BasicCurrencyAdapter;
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

pub struct DummyFeeSharingCall;

impl CallFilterWithOutput for DummyFeeSharingCall {
	type Call = Call;

	type Output = Option<AccountId>;

	fn is_call(call: &Self::Call) -> Self::Output {
		if let Call::FluentFee(pallet::Call::<Runtime>::fee_sharing_wrapper {
			beneficiary, ..
		}) = call
		{
			beneficiary.clone()
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
		// SHARE or BURN 2% of the transaction fee based on whether the beneficiary is set to an
		// eligible address or None
		let fee_shared_or_burned = balance.saturating_mul(2).saturating_div(100);
		let fee_payout = balance - fee_shared_or_burned;
		// If the transaction is of fee sharing type, transfer unit weight worth of fees to the
		// given beneficiary
		if let Some(beneficiary) =
			<IsSharingCall<Runtime> as IsFeeSharingCall<Runtime>>::is_call(call)
		{
			// Send the shared fee to the beneficiary account
			// NOTE: we are not reverting the transaction if the transfer to the beneficiary
			// fails as it does not constitute core logic expressed by the transaction but is merely
			// a tip given to the beneficiary of the signer's choice.
			// NOTE: We emit an event to indicate that unit weight fee transfer to the beneficiary
			// succeeded.
			if let Ok(_) = <Tokens as MultiCurrency<AccountId>>::transfer(
				*id,
				account,
				&beneficiary,
				fee_shared_or_burned.clone(),
			) {
				FluentFee::deposit_event(pallet::Event::FeeSharedWithTheBeneficiary((
					Some(beneficiary),
					fee_shared_or_burned,
				)));
			}

			// normal transaction fee withdrawal
			Tokens::withdraw(*id, account, fee_payout)
				.map_err(|err| traits::fee::InvalidFeeDispatch::UnresolvedRoute)
		} else {
			// BURN a portion of the fee if no beneficiary is chosen
			Tokens::withdraw(*id, account, fee_shared_or_burned)
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute);
			// Validator payout amount
			// NOTE: currently it is also being burned for the sake of simplicity, but with future
			// staking upgrades it will change
			Tokens::withdraw(*id, account, fee_payout)
				.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)
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
		benefitiary: &Option<<Runtime as frame_system::Config>::AccountId>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		let payouts = corret_withdrawn.saturating_sub(*tip);

		let ratio = FixedU128::saturating_from_rational(2_u128, 100_u128);

		// 2% of total control paid to beneficiary
		let beneficiary_cut = ratio.saturating_mul_int(payouts);

		if let Some(target) = benefitiary {
			Tokens::deposit(*id, target, beneficiary_cut)
				.map_err(|_| traits::fee::InvalidFeeDispatch::UnresolvedRoute)?;
		}

		Ok(())
	}
}

parameter_types! {
	pub const NativeAssetId: CurrencyId = CurrencyId::NativeToken(TokenId::Laguna);


}

pub struct PayoutSplits;

impl Get<(Price, Price, Price)> for PayoutSplits {
	fn get() -> (Price, Price, Price) {
		(
			FixedPointNumber::saturating_from_rational(49_u128, 100_u128),
			FixedPointNumber::saturating_from_rational(49_u128, 100_u128),
			FixedPointNumber::saturating_from_rational(2_u128, 100_u128),
		)
	}
}

impl Config for Runtime {
	type Event = Event;

	type DefaultFeeAsset = NativeAssetId;

	type MultiCurrency = Tokens;
	type Call = Call;

	type IsFeeSharingCall = DummyFeeSharingCall;

	type FeeSource = DummyFeeSource;
	type FeeMeasure = DummyFeeMeasure;
	type FeeDispatch = DummyFeeDispatch<Tokens>;

	type WeightInfo = SubstrateWeight<Runtime>;
	type Ratio = Price;

	type PayoutSplits = PayoutSplits;
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
