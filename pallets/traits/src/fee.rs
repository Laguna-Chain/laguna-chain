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
	type AccountId;
	type AssetId;

	/// whether both the caller and the asset are in good condition to be used as fee source
	fn accepted(who: &Self::AccountId, id: &Self::AssetId) -> Result<(), InvalidFeeSource>;

	/// whether an assets is enabled globally to be consider as an fee source
	fn listed(id: &Self::AssetId) -> Result<(), InvalidFeeSource>;
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

pub enum HealthStatusError {
	Unverified,
	Unstable,
	Unavailable,
}

/// to determine whether an asset's health status to be included as fee source
pub trait FeeAssetHealth {
	type AssetId;

	fn health_status(asset_id: &Self::AssetId) -> Result<(), HealthStatusError>;
}

pub enum EligibilityError {
	NotAllowed,
}

pub trait Eligibility {
	type AccountId;
	type AssetId;

	fn eligible(who: &Self::AccountId, asset_id: &Self::AssetId) -> Result<(), EligibilityError>;
}
