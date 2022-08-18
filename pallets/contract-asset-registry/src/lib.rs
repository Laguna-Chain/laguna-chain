//! ## pallet-contract-asset-registry
//!
//! This pallet allows contract based asset to be represented as native tokens

#![cfg_attr(not(feature = "std"), no_std)]

use codec::HasCompact;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		app_crypto::UncheckedFrom,
		traits::{AccountIdConversion, StaticLookup},
	},
	sp_std::{fmt::Debug, prelude::*},
	traits::Currency,
	PalletId,
};
use frame_system::{pallet_prelude::*, RawOrigin};
use hex_literal::hex;
pub use pallet::*;
use sp_core::{hexdisplay::AsBytesRef, U256};
use traits::currencies::TokenAccess;
use weights::WeightInfo;

pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type BalanceOf<T> =
	<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type AllowedOrigin: EnsureOrigin<Self::Origin>;

		// generate unique account_id and sub_account_id for this pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type MaxGas: Get<u64>;

		#[pallet::constant]
		type ContractDebugFlag: Get<bool>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		InvalidAsset,
	}

	#[pallet::storage]
	#[pallet::getter(fn get_registered)]
	pub type RegisteredAsset<T: Config> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, bool>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::register_asset())]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset_contract_address: AccountIdOf<T>,
			enabled: bool,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;

			RegisteredAsset::<T>::insert(asset_contract_address, enabled);

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::suspend_asset())]
		pub fn suspend_asset(
			origin: OriginFor<T>,
			asset_contract_address: AccountIdOf<T>,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;

			RegisteredAsset::<T>::mutate(asset_contract_address, |val| *val = Some(false));
			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::unregister_asset())]
		pub fn unregister_asset(
			origin: OriginFor<T>,
			asset_contract_address: AccountIdOf<T>,
		) -> DispatchResult {
			T::AllowedOrigin::ensure_origin(origin)?;

			RegisteredAsset::<T>::remove(asset_contract_address);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn enabled_assets() -> Vec<AccountIdOf<T>> {
		RegisteredAsset::<T>::iter()
			.filter_map(|(k, v)| if v { Some(k) } else { None })
			.collect::<Vec<_>>()
	}
}

// TODO: hard-coded erc20 selector, we should extend support to ink tokens as well, or come up with
// an adapter to bridge to both types of assets(solang sol and ink)
enum Selector<T: frame_system::Config> {
	TotalSupply,
	BalanceOf { owner: AccountIdOf<T> },
	Transfer { to: AccountIdOf<T>, amount: U256 },
	Allowance { owner: AccountIdOf<T>, spender: AccountIdOf<T> },
	Approve { spender: AccountIdOf<T>, amount: U256 },
	TransferFrom { from: AccountIdOf<T>, to: AccountIdOf<T>, amount: U256 },
}

// TODO: create selector buf at compile-time using proc-macro
impl<T: Config> Selector<T> {
	/// generate buffer of method selector from contract abi
	fn method_selector(&self) -> [u8; 4] {
		match self {
			Selector::TotalSupply => hex!("18160ddd"),
			Selector::BalanceOf { owner: _ } => hex!("70a08231"),
			Selector::Transfer { to: _, amount: _ } => hex!("a9059cbb"),
			Selector::Allowance { owner: _, spender: _ } => hex!("dd62ed3e"),
			Selector::Approve { spender: _, amount: _amout } => hex!("095ea7b3"),
			Selector::TransferFrom { from: _, to: _, amount: _ } => hex!("23b872dd"),
		}
	}

	/// generate full method selector with encoded arguments appended
	pub fn selector_buf(&self) -> Vec<u8> {
		let mut selector = self.method_selector().to_vec();

		match self {
			Selector::TotalSupply => {},
			Selector::BalanceOf { owner } => {
				selector.append(&mut owner.encode());
			},
			Selector::Transfer { to, amount } => {
				selector.append(&mut to.encode());
				selector.append(&mut amount.encode());
			},
			Selector::Allowance { owner, spender } => {
				selector.append(&mut owner.encode());
				selector.append(&mut spender.encode());
			},
			Selector::Approve { spender, amount } => {
				selector.append(&mut spender.encode());
				selector.append(&mut amount.encode());
			},
			Selector::TransferFrom { from, to, amount } => {
				selector.append(&mut from.encode());
				selector.append(&mut to.encode());
				selector.append(&mut amount.encode());
			},
		}
		selector
	}
}

impl<T> TokenAccess<T> for Pallet<T>
where
	T: Config,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	type Balance = BalanceOf<T>;

	fn total_supply(asset_address: AccountIdOf<T>) -> Option<Self::Balance> {
		Self::get_registered(asset_address.clone()).and_then(
			|rv| {
				if rv {
					Some(())
				} else {
					None
				}
			},
		)?;

		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account_truncating(),
			asset_address,
			Self::Balance::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::TotalSupply.selector_buf(),
			T::ContractDebugFlag::get(),
		)
		.result
		.ok()
		.filter(|v| !v.did_revert())
		.and_then(|res| -> Option<Self::Balance> {
			Decode::decode(&mut res.data.as_bytes_ref()).ok()
		})
	}

	fn balance_of(asset_address: AccountIdOf<T>, who: AccountIdOf<T>) -> Option<Self::Balance> {
		Self::get_registered(asset_address.clone()).and_then(
			|rv| {
				if rv {
					Some(())
				} else {
					None
				}
			},
		)?;

		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account_truncating(),
			asset_address,
			Self::Balance::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::BalanceOf { owner: who }.selector_buf(),
			T::ContractDebugFlag::get(),
		)
		.result
		.ok()
		.filter(|v| !v.did_revert())
		.and_then(|res| -> Option<Self::Balance> {
			Decode::decode(&mut res.data.as_bytes_ref()).ok()
		})
	}

	fn transfer(
		asset_address: AccountIdOf<T>,
		who: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		if Self::get_registered(asset_address.clone())
			.and_then(|rv| if rv { Some(()) } else { None })
			.is_none()
		{
			return Err(Error::<T>::InvalidAsset.into())
		}

		pallet_contracts::Pallet::<T>::call(
			RawOrigin::Signed(who).into(),
			T::Lookup::unlookup(asset_address),
			Default::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::Transfer { to, amount }.selector_buf(),
		)
	}

	fn allowance(
		asset_address: AccountIdOf<T>,
		owner: AccountIdOf<T>,
		spender: AccountIdOf<T>,
	) -> Option<Self::Balance> {
		Self::get_registered(asset_address.clone()).and_then(
			|rv| {
				if rv {
					Some(())
				} else {
					None
				}
			},
		)?;
		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account_truncating(),
			asset_address,
			Self::Balance::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::Allowance { owner, spender }.selector_buf(),
			T::ContractDebugFlag::get(),
		)
		.result
		.ok()
		.filter(|v| !v.did_revert())
		.and_then(|res| -> Option<Self::Balance> {
			Decode::decode(&mut res.data.as_bytes_ref()).ok()
		})
	}

	fn approve(
		asset_address: AccountIdOf<T>,
		owner: AccountIdOf<T>,
		spender: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		if Self::get_registered(asset_address.clone())
			.and_then(|rv| if rv { Some(()) } else { None })
			.is_none()
		{
			return Err(Error::<T>::InvalidAsset.into())
		}
		pallet_contracts::Pallet::<T>::call(
			RawOrigin::Signed(owner).into(),
			T::Lookup::unlookup(asset_address),
			Default::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::Approve { spender, amount }.selector_buf(),
		)
	}

	fn transfer_from(
		asset_address: AccountIdOf<T>,
		who: AccountIdOf<T>,
		from: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		if Self::get_registered(asset_address.clone())
			.and_then(|rv| if rv { Some(()) } else { None })
			.is_none()
		{
			return Err(Error::<T>::InvalidAsset.into())
		}
		pallet_contracts::Pallet::<T>::call(
			RawOrigin::Signed(who).into(),
			T::Lookup::unlookup(asset_address),
			Default::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::TransferFrom { from, to, amount }.selector_buf(),
		)
	}
}
