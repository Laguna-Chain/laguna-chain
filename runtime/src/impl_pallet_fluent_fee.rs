use crate::{Currencies, Event, FeeEnablement, Runtime};
use frame_support::pallet_prelude::InvalidTransaction;
use orml_traits::MultiCurrency;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource};

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Currencies;

	type FeeSource = FeeEnablement;

	type FeeMeasure = StaticImpl;

	type FeeDispatch = StaticImpl;
}

pub struct StaticImpl;

impl FeeSource for StaticImpl {
	type AccountId = AccountId;
	type AssetId = CurrencyId;

	fn accepted(
		who: &Self::AccountId,
		id: &Self::AssetId,
	) -> Result<(), traits::fee::InvalidFeeSource> {
		match id {
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(()),
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
	}

	fn listed(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
		match id {
			CurrencyId::NativeToken(TokenId::Laguna) => Ok(()),
			_ => Err(traits::fee::InvalidFeeSource::Unlisted),
		}
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
