use crate::{
	impl_pallet_currencies::NativeCurrencyId, Authorship, Call, Currencies, Event, FeeEnablement,
	FeeMeasurement, FluentFee, Runtime,
};
use frame_support::{pallet_prelude::InvalidTransaction, parameter_types};
use orml_traits::{BasicCurrency, MultiCurrency};
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use sp_runtime::{
	traits::{PostDispatchInfoOf, Zero},
	FixedPointNumber, FixedU128,
};
use traits::fee::{FeeDispatch, FeeMeasure, IsFeeSharingCall};

parameter_types! {
	pub const SplitRatio: (i32, i32) = (50, 50);
	pub const SplitRatioShared: (i32, i32, i32 ) = (34, 33, 33);


	pub const TreasuryAccounnt: AccountId = todo!();
}

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

impl FeeDispatch<Runtime> for StaticImpl {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn withdraw(
		account: &<Runtime as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		call: &<Runtime as frame_system::Config>::Call,
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

	fn tip(
		id: &Self::AssetId,
		balance: &Self::Balance,
	) -> Result<Self::Balance, traits::fee::InvalidFeeDispatch> {
		match id {
			// Currenlty can only
			CurrencyId::NativeToken(_) =>
				if let Some(author) = Authorship::author() {
					<Currencies as MultiCurrency<AccountId>>::deposit(*id, &author, *balance)
						.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)?;
					return Ok(*balance)
				},

			// TODO: need carrier for swap -> native
			CurrencyId::Erc20(_) => todo!(),
		}

		Ok(0)
	}

	fn post_info_correction(
		id: &Self::AssetId,
		corret_withdrawn: &Self::Balance,
		benefitiary: &Option<<Runtime as frame_system::Config>::AccountId>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		// TODO: require carrier for erc20
		if let CurrencyId::Erc20(_) = id {
			unimplemented!("currently erc20 handlign is not impelmented");
		}

		let to_treasury = if benefitiary.is_some() {
			FixedU128::saturating_from_rational(34_u128, 100_u128)
		} else {
			FixedU128::saturating_from_rational(50_u128, 100_u128)
		};

		// TODO: fill in treasury account

		let treasury_amount = to_treasury.saturating_mul_int(*corret_withdrawn);

		let mut remaining = corret_withdrawn.saturating_sub(treasury_amount);

		// pay block_author
		let author = Authorship::author().expect("unable to find author");

		let to_author = if benefitiary.is_some() {
			FixedU128::saturating_from_rational(50_u128, 50_u128)
		} else {
			FixedU128::saturating_from_rational(100_u128, 100_u128)
		};

		let author_amount = to_author.saturating_mul_int(remaining);

		<Currencies as MultiCurrency<AccountId>>::deposit(*id, &author, author_amount)
			.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)?;

		FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
			amount: author_amount,
			receiver: author.clone(),
			currency: *id,
		});

		remaining = remaining.saturating_sub(author_amount);

		// pay beneficiary if exists
		if let Some(target) = benefitiary {
			<Currencies as MultiCurrency<AccountId>>::deposit(*id, target, remaining)
				.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)?;
			FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
				receiver: target.clone(),
				currency: *id,
				amount: remaining,
			});
		} else {
			<Currencies as MultiCurrency<AccountId>>::deposit(*id, &author, remaining)
				.map_err(|_| traits::fee::InvalidFeeDispatch::CorrectionError)?;

			FluentFee::deposit_event(pallet_fluent_fee::Event::<Runtime>::FeePayout {
				receiver: author,
				currency: *id,
				amount: remaining,
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
