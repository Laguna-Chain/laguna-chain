use frame_support::{
	dispatch::Dispatchable, sp_runtime::traits::PostDispatchInfoOf, traits::WithdrawReasons,
	unsigned::TransactionValidityError,
};

#[derive(Debug)]
pub enum InvalidFeeSource {
	Inactive,
	Unlisted,
}

pub trait FeeSource {
	type AssetId;
	type Balance;

	fn accepted(id: &Self::AssetId) -> Result<(), InvalidFeeSource>;

	fn listing_asset(id: &Self::AssetId) -> Result<(), InvalidFeeSource>;
	fn denounce_asset(id: &Self::AssetId) -> Result<(), InvalidFeeSource>;
	fn disable_asset(id: &Self::AssetId) -> Result<(), InvalidFeeSource>;
}

pub trait FeeMeasure {
	type AssetId;
	type Balance;
	fn measure(
		id: &Self::AssetId,
		balance: Self::Balance,
	) -> Result<Self::Balance, TransactionValidityError>;
}

pub trait FeeDispatch<T>
where
	T: frame_system::Config,
{
	type AssetId;
	type Balance;

	fn withdraw(
		account: &<T as frame_system::Config>::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), InvalidFeeSource>;

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &PostDispatchInfoOf<T::Call>,
	) -> Result<(), InvalidFeeSource>;
}
