use frame_support::{
	dispatch::Dispatchable, sp_runtime::traits::PostDispatchInfoOf, traits::WithdrawReasons,
	unsigned::TransactionValidityError,
};

#[derive(Debug)]
pub enum InvalidFeeSource {
	Inactive,
	Unlisted,
}

#[derive(Debug)]
pub enum InvalidFeeDispatch {
	InsufficientBalance,
	UnresolvedRoute,
	CorrectionError,
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
	) -> Result<(), InvalidFeeDispatch>;

	fn post_info_correction(
		id: &Self::AssetId,
		post_info: &PostDispatchInfoOf<T::Call>,
	) -> Result<(), InvalidFeeDispatch>;
}
