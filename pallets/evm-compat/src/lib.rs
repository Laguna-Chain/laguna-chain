//! ## pallet-evm-compat
//!
//! this pallet allows triggering pallet_contract related operation from a signed ethereum transaction request
//!
//! for this to work, the runtiem need to:
//! 1. prove that incoming eth signed requeset is self_contained
//! 2. lookup from H160 to AccountId is possible

#![cfg_attr(not(feature = "std"), no_std)]

use core::ops::Add;

use codec::HasCompact;
use frame_support::{
	dispatch::Dispatchable,
	pallet_prelude::*,
	sp_io,
	sp_runtime::traits::DispatchInfoOf,
	sp_std::fmt::Debug,
	traits::Currency,
	weights::{DispatchInfo, PostDispatchInfo},
};
use frame_system::{pallet_prelude::*, CheckWeight};

use ethereum::TransactionV2 as Transaction;

use frame_support::sp_runtime::traits::StaticLookup;
pub use pallet::*;
use sp_core::{crypto::UncheckedFrom, H160, H256};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum RawOrigin {
	EthereumTransaction(H160),
}

pub fn ensure_ethereum_transaction<OuterOrigin>(o: OuterOrigin) -> Result<H160, &'static str>
where
	OuterOrigin: Into<Result<RawOrigin, OuterOrigin>>,
{
	match o.into() {
		Ok(RawOrigin::EthereumTransaction(n)) => Ok(n),
		_ => Err("bad origin: expected to be an Ethereum transaction"),
	}
}

pub struct EnsureEthereumTransaction;
impl<O: Into<Result<RawOrigin, O>> + From<RawOrigin>> EnsureOrigin<O>
	for EnsureEthereumTransaction
{
	type Success = H160;
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().map(|o| match o {
			RawOrigin::EthereumTransaction(id) => id,
		})
	}
}

#[frame_support::pallet]
mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type AddrLookup: StaticLookup<Source = H160, Target = AccountIdOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
		T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
	{
		#[pallet::weight(200_000_000)]
		pub fn transact(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
			Self::call_or_create(origin, t)?;

			Ok(())
		}
	}
}

// NOTICE: this is direct copy from pallet-ethereum

impl<T: Config> Pallet<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	fn recover_signer(transaction: &Transaction) -> Option<H160> {
		let mut sig = [0u8; 65];
		let mut msg = [0u8; 32];
		match transaction {
			Transaction::Legacy(t) => {
				sig[0..32].copy_from_slice(&t.signature.r()[..]);
				sig[32..64].copy_from_slice(&t.signature.s()[..]);
				sig[64] = t.signature.standard_v();
				msg.copy_from_slice(
					&ethereum::LegacyTransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP2930(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&ethereum::EIP2930TransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP1559(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&ethereum::EIP1559TransactionMessage::from(t.clone()).hash()[..],
				);
			},
		}
		let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg).ok()?;
		Some(H160::from(H256::from(sp_io::hashing::keccak_256(&pubkey))))
	}
	fn call_or_create(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
		let action = match &t {
			Transaction::Legacy(t) => t.action,
			Transaction::EIP2930(t) => t.action,
			Transaction::EIP1559(t) => t.action,
		};

		match action {
			ethereum::TransactionAction::Call(target) => Self::call(origin, target),
			ethereum::TransactionAction::Create => Self::create(origin, t),
		}?;

		Ok(())
	}

	fn call(origin: OriginFor<T>, target: H160) -> DispatchResult {
		todo!()
	}

	fn create(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
		todo!()
	}
}

#[repr(u8)]
enum TransactionValidationError {
	#[allow(dead_code)]
	UnknownError,
	InvalidChainId,
	InvalidSignature,
	InvalidGasLimit,
	MaxFeePerGasTooLow,
}

impl<T> Call<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T: Send + Sync + Config,
	<T as frame_system::Config>::Call:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. })
	}

	pub fn check_self_contained(&self) -> Option<Result<AccountIdOf<T>, TransactionValidityError>> {
		if let Call::transact { t } = self {
			let check = || {
				let origin = Pallet::<T>::recover_signer(t)
					.and_then(|addr| <T as Config>::AddrLookup::lookup(addr).ok())
					.ok_or(InvalidTransaction::Custom(
						TransactionValidationError::InvalidSignature as u8,
					))?;

				Ok(origin)
			};

			Some(check())
		} else {
			None
		}
	}

	pub fn pre_dispatch_self_contained(
		&self,
		origin: &AccountIdOf<T>,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::Call>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		if let Call::transact { t } = self {
			if let Err(e) = CheckWeight::<T>::do_pre_dispatch(dispatch_info, len) {
				return Some(Err(e));
			}
		}
		None
	}

	pub fn validate_self_contained(
		&self,
		origin: &AccountIdOf<T>,
		dispatch_info: &DispatchInfoOf<<T as frame_system::Config>::Call>,
		len: usize,
	) -> Option<TransactionValidity> {
		if let Call::transact { t } = self {
			if let Err(e) = CheckWeight::<T>::do_validate(dispatch_info, len) {
				return Some(Err(e));
			}
		}
		None
	}
}
