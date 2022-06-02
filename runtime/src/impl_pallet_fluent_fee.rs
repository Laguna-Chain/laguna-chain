use core::marker::PhantomData;

use crate::{Call, Currencies, Event, Runtime};
use frame_support::pallet_prelude::InvalidTransaction;
use orml_traits::MultiCurrency;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Currencies;

	type FeeSource = StaticImpl;

	type FeeMeasure = StaticImpl;

	type FeeDispatch = StaticImpl;
}

pub struct StaticImpl;

impl FeeSource for StaticImpl {
	type AssetId = CurrencyId;

	type Balance = Balance;

	fn accepted(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		match id {
			CurrencyId::NativeToken(TokenId::Hydro) => Ok(()),
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
	}

	fn listing_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}

	fn denounce_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}

	fn disable_asset(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		todo!()
	}
}

impl FeeMeasure for StaticImpl {
	type AssetId = CurrencyId;
	type Balance = Balance;

	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, frame_support::unsigned::TransactionValidityError> {
		match id {
			CurrencyId::NativeToken(TokenId::Hydro) => Ok(balance),
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
		balance: &Self::Balance,
		reason: &frame_support::traits::WithdrawReasons,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		Currencies::withdraw(*id, account, *balance)
			.map_err(|e| traits::fee::InvalidFeeDispatch::UnresolvedRoute)
	}

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
	) -> Result<(), traits::fee::InvalidFeeDispatch> {
		Ok(())
	}
}
