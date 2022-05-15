//! ## pallet-contract-asset-registry
//!
//! This pallet allows contract based asset to be represented as native tokens

#![cfg_attr(not(feature = "std"), no_std)]

use codec::HasCompact;
use core::str::FromStr;
use frame_support::{pallet_prelude::*, traits::Currency, PalletId};
use frame_system::{pallet_prelude::*, RawOrigin};
use hex_literal::hex;
use primitives::{Balance, CurrencyId};
use sp_core::hexdisplay::AsBytesRef;
use sp_runtime::{
	app_crypto::UncheckedFrom,
	traits::{AccountIdConversion, AccountIdLookup, IdentityLookup},
	MultiAddress,
};

pub use pallet::*;
use sp_core::U256;
use sp_std::fmt::Debug;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type BalanceOf<T> =
	<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
use sp_std::prelude::*;

#[frame_support::pallet]
mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		// generate unique account_id and sub_account_id for this pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type MaxGas: Get<u64>;

		#[pallet::constant]
		type ContractDebugFlag: Get<bool>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
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
			Selector::BalanceOf { owner } => hex!("70a08231"),
			Selector::Transfer { to, amount } => hex!("a9059cbb"),
			Selector::Allowance { owner, spender } => hex!("dd62ed3e"),
			Selector::Approve { spender, amount: amout } => hex!("095ea7b3"),
			Selector::TransferFrom { from, to, amount } => hex!("23b872dd"),
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

// TODO: move this trait to the traits package later
pub trait TokenAccess<T: frame_system::Config> {
	type Balance;

	fn total_supply(asset_address: AccountIdOf<T>) -> Option<Self::Balance>;

	fn balance_of(asset_address: AccountIdOf<T>, who: AccountIdOf<T>) -> Option<Self::Balance>;

	fn transfer(
		asset_address: AccountIdOf<T>,
		who: OriginFor<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;

	fn allowance(
		asset_address: AccountIdOf<T>,
		owner: AccountIdOf<T>,
		spender: AccountIdOf<T>,
	) -> Option<Self::Balance>;

	fn approve(
		asset_address: AccountIdOf<T>,
		owner: OriginFor<T>,
		spender: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;

	fn transfer_from(
		asset_address: AccountIdOf<T>,
		who: OriginFor<T>,
		from: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;
}

impl<T> TokenAccess<T> for Pallet<T>
where
	T: Config<Lookup = IdentityLookup<AccountIdOf<T>>>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
{
	type Balance = BalanceOf<T>;

	fn total_supply(asset_address: AccountIdOf<T>) -> Option<Self::Balance> {
		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account(),
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
		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account(),
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
		who: OriginFor<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		pallet_contracts::Pallet::<T>::call(
			who,
			asset_address,
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
		pallet_contracts::Pallet::<T>::bare_call(
			T::PalletId::get().into_account(),
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
		owner: OriginFor<T>,
		spender: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		pallet_contracts::Pallet::<T>::call(
			owner,
			asset_address,
			Default::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::Approve { spender, amount }.selector_buf(),
		)
	}

	fn transfer_from(
		asset_address: AccountIdOf<T>,
		who: OriginFor<T>,
		from: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo {
		pallet_contracts::Pallet::<T>::call(
			who,
			asset_address,
			Default::default(),
			T::MaxGas::get(),
			None,
			Selector::<T>::TransferFrom { from, to, amount }.selector_buf(),
		)
	}
}
