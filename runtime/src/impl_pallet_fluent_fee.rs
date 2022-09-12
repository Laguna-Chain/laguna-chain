use crate::{
	constants::NATIVE_TOKEN, impl_pallet_currencies::NativeCurrencyId, Authorship, Call, Contracts,
	Currencies, Event, FeeEnablement, FeeMeasurement, FluentFee, Origin, PrepaidFee, Runtime,
	Treasury,
};
use frame_support::{
	pallet_prelude::InvalidTransaction,
	parameter_types,
	sp_runtime::{
		sp_std::vec::Vec,
		traits::{AccountIdConversion, StaticLookup},
	},
	traits::Get,
	PalletId,
};
use orml_traits::MultiCurrency;
use primitives::{AccountId, Balance, CurrencyId, Price, TokenId};
use sp_runtime::{self, FixedPointNumber, FixedU128};
use traits::fee::{CallFilterWithOutput, FeeCarrier, FeeDispatch, FeeMeasure};

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

parameter_types! {
	pub const PALLETID: PalletId = PalletId(*b"lgn/carr");
}

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type DefaultFeeAsset = NativeCurrencyId;

	type MultiCurrency = Currencies;

	type Call = Call;

	type IsFeeSharingCall = IsFeeSharingCall;

	type FeeSource = FeeEnablement;

	type FeeMeasure = FeeMeasurement;

	type FeeDispatch = StaticImpl;

	type Ratio = Price;

	type PayoutSplits = PayoutSplits;

	type PalletId = PALLETID;

	type IsCarrierAttachedCall = IsCarrierAttachedCall;

	type Carrier = StaticImpl;
}

pub struct StaticImpl;

impl FeeMeasure for StaticImpl {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, frame_support::unsigned::TransactionValidityError> {
		match id {
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(balance),
			_ => Err(InvalidTransaction::Payment.into()),
		}
	}
}

impl FeeCarrier for StaticImpl {
	type AccountId = AccountId;
	type Balance = Balance;

	fn execute_carrier(
		account: &Self::AccountId,
		carrier_addr: &Self::AccountId,
		carrier_data: sp_std::vec::Vec<u8>,
		required: Self::Balance,
	) -> Result<Self::Balance, traits::fee::InvalidFeeDispatch> {
		let acc: AccountId = <Runtime as pallet_fluent_fee::Config>::PalletId::get()
			.try_into_account()
			.unwrap();
		let before = Currencies::free_balance(acc.clone(), NativeCurrencyId::get());

		let addr = <Runtime as frame_system::Config>::Lookup::unlookup(carrier_addr.clone());

		Contracts::call(
			Origin::signed(account.clone()),
			addr,
			Default::default(),
			Default::default(),
			None,
			carrier_data,
		)
		.unwrap();

		let after = Currencies::free_balance(acc, NativeCurrencyId::get());

		let collected = after.saturating_sub(before);

		if collected >= required {
			Ok(collected)
		} else {
			Err(traits::fee::InvalidFeeDispatch::InsufficientBalance)
		}
	}
}

impl FeeDispatch for StaticImpl {
	type AccountId = AccountId;
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		reason: &frame_support::traits::WithdrawReasons,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		match id {
			CurrencyId::NativeToken(_) =>
				<Currencies as MultiCurrency<AccountId>>::withdraw(*id, account, *balance)
					.map_err(|_| traits::fee::InvalidFeeDispatch::UnresolvedRoute),

			// TODO: need carrier for swap -> native
			CurrencyId::Erc20(_) => todo!(),
		}
	}

	fn refund(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
	) -> Result<Self::Balance, traits::fee::InvalidFeeDispatch> {
		match id {
			CurrencyId::NativeToken(_) => {
				<Currencies as MultiCurrency<AccountId>>::deposit(*id, account, *balance)
					.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)?;
				Ok(*balance)
			},

			// TODO: need carrier for swap -> native
			CurrencyId::Erc20(_) => todo!(),
		}
	}

	fn post_info_correction(
		id: &Self::AssetId,
		tip: &Self::Balance,
		corret_withdrawn: &Self::Balance,
		benefitiary: &Option<<Runtime as frame_system::Config>::AccountId>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		// TODO: require carrier for erc20
		if let CurrencyId::Erc20(_) = id {
			unimplemented!("currently erc20 handling is not impelmented");
		}

		let (to_treasury, to_author, to_shared) =
			<Runtime as pallet_fluent_fee::Config>::PayoutSplits::get();

		let treasury_account_id = Treasury::account_id();

		let treasury_amount = to_treasury.saturating_mul_int(*corret_withdrawn);

		// TODO: provide token specific payout routes
		let dispatch_with = match id {
			CurrencyId::NativeToken(TokenId::Laguna) =>
				|id: CurrencyId, who: &AccountId, amount: Balance| {
					<Currencies as MultiCurrency<AccountId>>::deposit(id, who, amount)
						.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)
				},
			CurrencyId::NativeToken(TokenId::FeeToken) =>
				|_: CurrencyId, who: &AccountId, amount: Balance| {
					PrepaidFee::unserve_to(who.clone(), amount)
						.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)
				},
			_ => {
				unimplemented!("non native token unspported");
			},
		};

		dispatch_with(*id, &treasury_account_id, treasury_amount)?;

		FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
			amount: treasury_amount,
			receiver: treasury_account_id,
			currency: *id,
		});

		let author_amount = to_author.saturating_mul_int(*corret_withdrawn);

		// TODO: investigate cases where block author cannot be found
		if let Some(author) = Authorship::author() {
			dispatch_with(*id, &author, author_amount + tip)?;

			FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
				amount: author_amount + tip,
				receiver: author,
				currency: *id,
			});
		}

		let shared_amount = to_shared.saturating_mul_int(*corret_withdrawn);

		if let Some(target) = benefitiary {
			dispatch_with(*id, target, shared_amount)?;

			FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
				receiver: target.clone(),
				currency: *id,
				amount: shared_amount,
			});
		}

		Ok(())
	}
}

// TODO: the below part is currently not included in the withdraw_fee() implementation. For now it
// is only included to satisfy the compiler errors
pub struct IsFeeSharingCall;

impl CallFilterWithOutput for IsFeeSharingCall {
	type Call = Call;

	type Output = Option<AccountId>;

	fn is_call(call: &<Runtime as frame_system::Config>::Call) -> Self::Output {
		if let Call::FluentFee(pallet_fluent_fee::pallet::Call::<Runtime>::fee_sharing_wrapper {
			beneficiary,
			..
		}) = call
		{
			beneficiary.clone()
		} else {
			None
		}
	}
}

pub struct IsCarrierAttachedCall;

impl CallFilterWithOutput for IsCarrierAttachedCall {
	type Call = Call;

	type Output = Option<(AccountId, Vec<u8>)>;

	fn is_call(call: &<Runtime as frame_system::Config>::Call) -> Self::Output {
		if let Call::FluentFee(pallet_fluent_fee::pallet::Call::<Runtime>::carrier_wrapper {
			carrier,
			carrier_data,
			..
		}) = call
		{
			Some((carrier.clone(), carrier_data.clone()))
		} else {
			None
		}
	}
}
