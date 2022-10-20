use codec::HasCompact;
use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	sp_std::{fmt::Debug, prelude::*},
	weights::{DispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;

use fp_ethereum::TransactionData;
use sp_core::{crypto::UncheckedFrom, H160, U256};

use crate::{fee_details, AccountIdOf, BalanceOf, Call, Config, Pallet, RawOrigin};

#[repr(u8)]
enum TransactionValidationError {
	#[allow(dead_code)]
	UnknownError,
	InvalidSignature,
}

type CheckedInfo<T> = (H160, AccountIdOf<T>, (U256, U256));

impl<T> Call<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T: Send + Sync + Config,
	<T as frame_system::Config>::Call:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	/// filter all calls that are self_contained
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. } | Call::set_proxy { .. })
	}

	/// checks the call by extrinsic
	pub fn check_self_contained(&self) -> Option<Result<CheckedInfo<T>, TransactionValidityError>> {
		match self {
			Call::transact { t } => {
				let rs = Pallet::<T>::recover_tx_signer(t)
					.map(|s| {
						let o = Pallet::<T>::to_mapped_account(s);
						let extra = self.expose_extra();
						(s, o, extra)
					})
					.ok_or_else(|| {
						InvalidTransaction::Custom(
							TransactionValidationError::InvalidSignature as u8,
						)
						.into()
					});

				Some(rs)
			},

			Call::set_proxy { who, nonce, sig } => {
				let rs = Pallet::<T>::verify_proxy_request(who, nonce, sig)
					.map(|s| {
						let o = Pallet::<T>::to_mapped_account(s);
						let extra = self.expose_extra();
						(s, o, extra)
					})
					.ok_or_else(|| {
						InvalidTransaction::Custom(
							TransactionValidationError::InvalidSignature as u8,
						)
						.into()
					});

				Some(rs)
			},
			_ => None,
		}
	}

	/// expose several common information for the runtime to do signed_extension insepction.
	fn expose_extra(&self) -> (U256, U256) {
		match self {
			Call::transact { t } => {
				let nonce = TransactionData::from(t).nonce;
				let (_, tips) = fee_details::<T>(t);
				(nonce, tips)
			},
			Call::set_proxy { nonce, .. } => (*nonce, Default::default()),
			_ => Default::default(),
		}
	}
}
