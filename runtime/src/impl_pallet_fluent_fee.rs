use crate::{
	impl_pallet_currencies::NativeCurrencyId, Call, ContractAssetsRegistry, Currencies, Event,
	FeeEnablement, FeeMeasurement, Runtime,
};
use frame_support::pallet_prelude::InvalidTransaction;
use orml_traits::MultiCurrency;
use primitives::{AccountId, Balance, CurrencyId, TokenId};
use traits::fee::{FeeDispatch, FeeMeasure, FeeSource, IsFeeSharingCall};

impl pallet_fluent_fee::Config for Runtime {
	type DefaultFeeAsset = NativeCurrencyId;

	type Event = Event;

	type MultiCurrency = Currencies;

	type Call = Call;

	type IsFeeSharingCall = DummyFeeSharingCall;

	type FeeSource = FeeEnablement;

	type FeeMeasure = FeeMeasurement;

	type FeeDispatch = StaticImpl;

	type WeightInfo = pallet_fluent_fee::weights::SubstrateWeight<Runtime>;
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

// TODO: the below part is currently not included in the withdraw_fee() implementation. For now it
// is only included to satisfy the compiler errors
pub struct DummyFeeSharingCall;

impl IsFeeSharingCall<Runtime> for DummyFeeSharingCall {
	type AccountId = AccountId;

	fn is_call(call: &<Runtime as frame_system::Config>::Call) -> Option<Self::AccountId> {
		if let Call::FluentFee(pallet_fluent_fee::Call::<Runtime>::fee_sharing_wrapper {
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
