//! ## tx-adapter
//!
//! after receiving the signed eth-payload, adapter over various input
//!
//! currently three posible actions will be trigger:
//! 1. create_contract if it has not target, we treat the input as (Code, Selector, Salt) in
//! scale-encoded form 2. if it has target but not no input defined, than we do plain token transfer
//! 3. if it has target and has input, than we contract call

use crate::{fee_details, AccountIdOf, BalanceOf, Config, Error, Pallet};
use codec::HasCompact;
use ethereum::{TransactionAction, TransactionV2 as Transaction};
use frame_support::{
	pallet_prelude::*,
	sp_std::{fmt::Debug, prelude::*},
	traits::Currency,
};

use codec::Decode;

use fp_ethereum::TransactionData;
use frame_support::{sp_runtime::traits::StaticLookup, traits::tokens::ExistenceRequirement};
use sp_core::{crypto::UncheckedFrom, H160, U256};

// once we have the TransactionData we can start mapping it to pallet_contract call args
pub struct ContractTransactionAdapter<T> {
	inner: TransactionData,
	max_allowed: U256,
	_marker: PhantomData<T>,
}

impl<T: Config> ContractTransactionAdapter<T>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub(crate) fn from_tx(tx: &Transaction) -> Self {
		let (max_allowed, _) = fee_details::<T>(tx);
		Self { inner: TransactionData::from(tx), max_allowed, _marker: Default::default() }
	}

	pub(crate) fn call_or_create(&self, source: H160) -> DispatchResultWithPostInfo {
		match self.inner.action {
			TransactionAction::Call(target) =>
				if self.inner.input.is_empty() {
					// otherwise we recognize it as normal transfer
					self.execute_transfer_request(source, target)
				} else {
					self.execute_call_request(source, target)
				},
			TransactionAction::Create => self.execute_create_request(source),
		}
	}

	fn execute_transfer_request(&self, source: H160, target: H160) -> DispatchResultWithPostInfo {
		let from = Pallet::<T>::to_mapped_account(source);

		// assume receiver has mapped address
		let to = Pallet::<T>::to_mapped_account(target);
		let value =
			BalanceOf::<T>::try_from(self.inner.value).map_err(|_| Error::<T>::ConvertionFailed)?;

		<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::transfer(
			&from,
			&to,
			value,
			ExistenceRequirement::KeepAlive,
		)?;

		Ok(().into())
	}

	/// the actual substrate weight allowed for from a eth transaction
	fn execute_call_request(&self, source: H160, target: H160) -> DispatchResultWithPostInfo {
		let contract_addr = Pallet::<T>::account_from_contract_addr(target);

		let contract_addr_source =
			<<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(contract_addr);

		// mapped origin has no known key pair
		let elevated_origin = Pallet::<T>::to_mapped_origin(source);

		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(self.max_allowed)
			.ok()
			.map(Into::<<BalanceOf<T> as codec::HasCompact>::Type>::into);

		// we accept input of both scale-encoded Bytes or plain Bytes
		// TODO: check endianess of eth-clients
		let input = <Vec<u8>>::decode(&mut &self.inner.input[..])
			.unwrap_or_else(|_| self.inner.input.to_vec());

		pallet_contracts::Pallet::<T>::call(
			elevated_origin,
			contract_addr_source,
			self.inner.value.try_into().unwrap_or_default(),
			self.max_allowed.as_u64(),
			storage_deposit_limit,
			input,
		)
	}

	fn execute_create_request(&self, source: H160) -> DispatchResultWithPostInfo {
		// FIXME: etherem use same input field to contain both code and data, we need a way to
		// communicate with tool about our choice of this.
		let mut input_buf = &self.inner.input[..];

		// scale-codec can split vec's on the fly
		let (code, data, salt) = <(Vec<u8>, Vec<u8>, Vec<u8>)>::decode(&mut input_buf)
			.or(Err(Error::<T>::InputBufferUndecodable))?;

		// this origin cannot be controled from outside
		let elevated_origin = Pallet::<T>::to_mapped_origin(source);

		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(self.max_allowed)
			.ok()
			.map(Into::<<BalanceOf<T> as codec::HasCompact>::Type>::into);

		pallet_contracts::Pallet::<T>::instantiate_with_code(
			elevated_origin,
			self.inner.value.try_into().unwrap_or_default(),
			self.max_allowed.as_u64(),
			storage_deposit_limit,
			code,
			data,
			salt,
		)
	}
}
