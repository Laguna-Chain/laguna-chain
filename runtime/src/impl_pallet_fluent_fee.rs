use crate::{
	impl_pallet_currencies::NativeCurrencyId, Authorship, Call, Currencies, Event, FeeEnablement,
	FeeMeasurement, FluentFee, PrepaidFee, Runtime, Treasury,
};
use frame_support::{pallet_prelude::InvalidTransaction, parameter_types};
use orml_traits::MultiCurrency;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_runtime::{self, FixedPointNumber, FixedU128};
use traits::fee::{FeeDispatch, FeeMeasure, IsFeeSharingCall};

impl pallet_fluent_fee::Config for Runtime {
	type DefaultFeeAsset = NativeCurrencyId;

	type Event = Event;

	type MultiCurrency = Currencies;

	type Call = Call;

	type IsFeeSharingCall = DummyFeeSharingCall;

	type FeeSource = FeeEnablement;

	type FeeMeasure = FeeMeasurement;

	type FeeDispatch = StaticImpl;
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

		// 49% of total corrected goes to treasury by default
		let to_treasury = FixedU128::saturating_from_rational(49_u128, 100_u128);

		// 49% of total corrected goes to validator by default
		let to_author = FixedU128::saturating_from_rational(49_u128, 100_u128);

		// 2% of total corrected goes to shared by default
		let to_shared = FixedU128::saturating_from_rational(2_u128, 100_u128);

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

		if let Some(author) = Authorship::author() {
			dispatch_with(*id, &author, author_amount + tip)?;

			FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
				amount: author_amount + tip,
				receiver: author,
				currency: *id,
			});
		}

		let shared_amount = to_shared.saturating_mul_int(*corret_withdrawn);

		// TODO: investigate cases where block author cannot be found
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
pub struct DummyFeeSharingCall;

impl IsFeeSharingCall<Runtime> for DummyFeeSharingCall {
	type AccountId = AccountId;

	fn is_call(call: &<Runtime as frame_system::Config>::Call) -> Option<Self::AccountId> {
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
