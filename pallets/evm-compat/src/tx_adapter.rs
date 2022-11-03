//! ## tx-adapter
//!
//! after receiving the signed eth-payload, adapter over various input
//!
//! currently three posible actions will be trigger:
//! 1. create_contract if it has not target, we treat the input as (Code, Selector, Salt) in
//! scale-encoded form 2. if it has target but not no input defined, than we do plain token transfer
//! 3. if it has target and has input, than we contract call

use crate::{AccountIdOf, BalanceOf, Config, Error, Pallet};
use codec::HasCompact;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::SaturatedConversion,
	sp_std::{fmt::Debug, prelude::*},
	traits::Currency,
};

use pallet_contracts_primitives::{
	Code, ContractExecResult, ContractInstantiateResult, StorageDeposit,
};

use pallet_evm_compat_common::{
	ActionRequest, EvmActionRequest, EvmFeeRequest, TransactionMessage,
};

use codec::Decode;
use ethereum::TransactionV2 as EthereumTransaction;

use frame_support::{sp_runtime::traits::StaticLookup, traits::tokens::ExistenceRequirement};
use sp_core::{crypto::UncheckedFrom, H160, U256};

/// wasm-based virtual matchine
///
/// this is the rapper around parsing eth-evm request into pallet-contracts wasm-based vm
/// environemnt
///
/// > the provided methods are not taking verification into mind, please do any neccesary
/// > verification beforehand.
pub struct WEVMAdapter<T, V> {
	pub inner: TransactionMessage,
	_marker: PhantomData<T>,
	_signed: PhantomData<V>,
}

// use unit type as generic type to limit public access to state changing methods
mod verification {
	pub struct Signed;
	pub struct Raw;
}

use verification::*;

impl<T: Config> WEVMAdapter<T, ()>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub fn new_from_signed(tx: &EthereumTransaction) -> WEVMAdapter<T, Signed> {
		WEVMAdapter::<T, Signed> {
			inner: TransactionMessage::from(tx.clone()),
			_marker: Default::default(),
			_signed: Default::default(),
		}
	}

	pub fn new_from_raw(tx: &TransactionMessage) -> WEVMAdapter<T, Raw> {
		WEVMAdapter::<T, Raw> {
			inner: tx.clone(),
			_marker: Default::default(),
			_signed: Default::default(),
		}
	}
}

impl<T: Config> WEVMAdapter<T, Signed>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	/// execute verified tx-payload
	pub fn execute(&self, source: &H160) -> DispatchResultWithPostInfo {
		match self.inner.action_request() {
			ActionRequest::Create => Runner::<T>::create(
				source,
				&self.inner.max_allowed(),
				&self.inner.storage_deposit(),
				&(self.inner.value().try_into().unwrap_or_default()),
				&self.inner.input()[..],
			),
			ActionRequest::Call(target) => Runner::<T>::call(
				source,
				&target,
				&self.inner.max_allowed(),
				&self.inner.storage_deposit(),
				&(self.inner.value().try_into().unwrap_or_default()),
				&self.inner.input()[..],
			),
			ActionRequest::Transfer(target) =>
				Runner::<T>::transfer(source, &target, &self.inner.value()),
		}
	}
}

impl<T: Config> WEVMAdapter<T, Raw>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	/// execute verified tx-payload
	pub fn try_call(
		&self,
		source: &H160,
	) -> Result<ContractExecResult<BalanceOf<T>>, DispatchError> {
		let target = if let ActionRequest::Call(t) = self.inner.action_request() {
			Ok(t)
		} else {
			Err(DispatchError::CannotLookup)
		}?;

		Ok(Runner::<T>::try_call(
			source,
			&target,
			&self.inner.max_allowed(),
			&self.inner.storage_deposit(),
			&(self.inner.value().try_into().unwrap_or_default()),
			&self.inner.input(),
		))
	}

	/// execute verified tx-payload
	pub fn try_create(
		&self,
		source: &H160,
	) -> Result<ContractInstantiateResult<AccountIdOf<T>, BalanceOf<T>>, DispatchError> {
		Runner::<T>::try_create(
			source,
			&self.inner.max_allowed(),
			&self.inner.storage_deposit(),
			&(self.inner.value().try_into().unwrap_or_default()),
			&self.inner.input()[..],
		)
	}
}

pub struct Runner<T>(PhantomData<T>);

impl<T: Config> Runner<T>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub fn create(
		source: &H160,
		max_allowed: &U256,
		storage_deposit_limit: &U256,
		value: &BalanceOf<T>,
		input: &[u8],
	) -> DispatchResultWithPostInfo {
		// scale-codec can split vec's on the fly
		let (code, data, salt) = <(Vec<u8>, Vec<u8>, Vec<u8>)>::decode(&mut &*input)
			.or(Err(Error::<T>::InputBufferUndecodable))?;

		// this origin cannot be controled from outside
		let elevated_origin = Pallet::<T>::to_mapped_origin(*source);

		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(*storage_deposit_limit)
			.ok()
			.map(Into::<<BalanceOf<T> as codec::HasCompact>::Type>::into);

		pallet_contracts::Pallet::<T>::instantiate_with_code(
			elevated_origin,
			*value,
			(*max_allowed).saturated_into(),
			storage_deposit_limit,
			code,
			data,
			salt,
		)
	}

	pub fn try_create(
		source: &H160,
		max_allowed: &U256,
		storage_deposit_limit: &U256,
		value: &BalanceOf<T>,
		input: &[u8],
	) -> Result<ContractInstantiateResult<AccountIdOf<T>, BalanceOf<T>>, DispatchError> {
		// scale-codec can split vec's on the fly
		let (code, data, salt) = <(Vec<u8>, Vec<u8>, Vec<u8>)>::decode(&mut &*input)
			.or(Err(Error::<T>::InputBufferUndecodable))?;

		// this origin cannot be controled from outside
		let from = Pallet::<T>::to_mapped_account(*source);

		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(*storage_deposit_limit).ok();

		let upload_result = pallet_contracts::Pallet::<T>::bare_upload_code(
			from.clone(),
			code,
			storage_deposit_limit,
		)?;

		let code = Code::Existing(upload_result.code_hash);

		let mut instantiate_result = pallet_contracts::Pallet::<T>::bare_instantiate(
			from,
			*value,
			(*max_allowed).saturated_into(),
			storage_deposit_limit,
			code,
			data,
			salt,
			true,
		);

		let reserve_deposit =
			if let StorageDeposit::Charge(reserved) = &instantiate_result.storage_deposit {
				StorageDeposit::Charge(*reserved + upload_result.deposit)
			} else {
				StorageDeposit::Charge(Default::default())
			};

		instantiate_result.storage_deposit = reserve_deposit;

		Ok(instantiate_result)
	}

	pub fn call(
		source: &H160,
		target: &H160,
		max_allowed: &U256,
		storage_deposit_limit: &U256,
		value: &BalanceOf<T>,
		input: &[u8],
	) -> DispatchResultWithPostInfo {
		let contract_addr = Pallet::<T>::account_from_contract_addr(*target);

		let contract_addr_source =
			<<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(contract_addr);

		// mapped origin has no known key pair
		let elevated_origin = Pallet::<T>::to_mapped_origin(*source);

		// into compact form
		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(*storage_deposit_limit)
			.ok()
			.map(Into::<<BalanceOf<T> as codec::HasCompact>::Type>::into);

		pallet_contracts::Pallet::<T>::call(
			elevated_origin,
			contract_addr_source,
			*value,
			(*max_allowed).saturated_into(),
			storage_deposit_limit,
			input.to_vec(),
		)
	}

	pub fn try_call(
		source: &H160,
		target: &H160,
		max_allowed: &U256,
		storage_deposit_limit: &U256,
		value: &BalanceOf<T>,
		input: &[u8],
	) -> ContractExecResult<BalanceOf<T>> {
		// mapped origin has no known key pair
		let from = Pallet::<T>::to_mapped_account(*source);

		let dest = Pallet::<T>::account_from_contract_addr(*target);

		let storage_deposit_limit = TryInto::<BalanceOf<T>>::try_into(*storage_deposit_limit).ok();

		pallet_contracts::Pallet::<T>::bare_call(
			from,
			dest,
			*value,
			(*max_allowed).saturated_into(),
			storage_deposit_limit,
			input.to_vec(),
			true,
		)
	}

	pub fn transfer(source: &H160, target: &H160, value: &U256) -> DispatchResultWithPostInfo {
		let from = Pallet::<T>::to_mapped_account(*source);

		// assume receiver has mapped address
		let to = Pallet::<T>::to_mapped_account(*target);

		let value = BalanceOf::<T>::try_from(*value).map_err(|_| Error::<T>::ConvertionFailed)?;

		<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::transfer(
			&from,
			&to,
			value,
			ExistenceRequirement::KeepAlive,
		)?;

		// no post-correction since we only charge what's neccesary
		Ok(().into())
	}
}
