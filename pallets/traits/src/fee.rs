use frame_support::{traits::WithdrawReasons, unsigned::TransactionValidityError};

use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Debug, TypeInfo, Encode, Decode)]
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

pub trait FeeDispatch {
	type AccountId;
	type AssetId;
	type Balance;

	/// handle withdrawn
	fn withdraw(
		account: &Self::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
		reason: &WithdrawReasons,
	) -> Result<(), InvalidFeeDispatch>;

	/// handle overcharged amount
	fn refund(
		account: &Self::AccountId,
		id: &Self::AssetId,
		balance: &Self::Balance,
	) -> Result<Self::Balance, InvalidFeeDispatch>;

	fn post_info_correction(
		id: &Self::AssetId,
		tip: &Self::Balance,
		correted_withdrawn: &Self::Balance,
		benefitiary: &Option<Self::AccountId>,
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

pub trait IsFeeSharingCall<T>
where
	T: frame_system::Config,
{
	type AccountId;
	// returns the Some(beneficiary_account) if the call is fee sharing type, otherwise returns None
	fn is_call(call: &<T as frame_system::Config>::Call) -> Option<Self::AccountId>;
}

pub trait IsSchedulerCall<T>
where
	T: frame_system::Config,
{
	type Output;

	fn is_call(call: &<T as frame_system::Config>::Call) -> Self::Output;
}
