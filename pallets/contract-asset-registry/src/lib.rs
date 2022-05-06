//! ## pallet-contract-asset-registry
//!
//! This pallet allows contract based asset to be represented as native tokens

#![cfg_attr(not(feature = "std"), no_std)]

use core::str::FromStr;
use frame_support::{pallet_prelude::*, PalletId};
use frame_system::{pallet_prelude::*, Account};
use hex_literal::hex;
use pallet_contracts_rpc_runtime_api::runtime_decl_for_ContractsApi::ContractsApi;
use sp_runtime::{
	app_crypto::UncheckedFrom,
	traits::{AccountIdConversion, Block as BlockT},
};

pub use pallet::*;
use sp_core::U256;

pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_contracts::Config + pallet_balances::Config
	{
		// generate unique account_id and sub_account_id for this pallet
		type PalletId: Get<PalletId>;
		type TokenAccess: TokenAccess<Self, Balance = <Self as pallet_balances::Config>::Balance>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);
}

enum Selector<T: frame_system::Config> {
	TotalSupply,
	BalanceOf(AccountIdFor<T>),
	Transfer(AccountIdFor<T>, U256),
	Allowance(AccountIdFor<T>, AccountIdFor<T>),
	Approve(AccountIdFor<T>, U256),
	TransferFrom(AccountIdFor<T>, AccountIdFor<T>, U256),
}

// TODO: create selector buf at compile-time using proc-macro
impl<T: Config> Selector<T> {
	/// generate buffer of method selector from contract abi
	fn method_selector(&self) -> [u8; 4] {
		match self {
			Selector::TotalSupply => hex!("18160ddd"),
			Selector::BalanceOf(_) => hex!("70a08231"),
			Selector::Transfer(_, _) => hex!("a9059cbb"),
			Selector::Allowance(_, _) => hex!("dd62ed3e"),
			Selector::Approve(_, _) => hex!("095ea7b3"),
			Selector::TransferFrom(_, _, _) => hex!("23b872dd"),
		}
	}

	/// generate full method selector with encoded arguments appended
	pub fn selector_buf(&self) -> Vec<u8> {
		let mut selector = self.method_selector().to_vec();

		match self {
			Selector::TotalSupply => {},
			Selector::BalanceOf(account) => {
				selector.append(&mut account.encode());
			},
			Selector::Transfer(account, amount) => {
				selector.append(&mut account.encode());
				selector.append(&mut amount.encode());
			},
			Selector::Allowance(owner, spender) => {
				selector.append(&mut owner.encode());
				selector.append(&mut spender.encode());
			},
			Selector::Approve(account, amount) => {
				selector.append(&mut account.encode());
				selector.append(&mut amount.encode());
			},
			Selector::TransferFrom(from, to, amount) => {
				selector.append(&mut from.encode());
				selector.append(&mut to.encode());
				selector.append(&mut amount.encode());
			},
		}
		selector
	}
}

trait TokenAccess<T: Config> {
	type Balance;

	fn total_supply(asset_address: AccountIdFor<T>) -> Option<Self::Balance>;
	fn balance_of(asset_address: AccountIdFor<T>, target: AccountIdFor<T>)
		-> Option<Self::Balance>;

	fn transfer(
		asset_address: AccountIdFor<T>,
		target: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult;

	fn allowance(
		asset_address: AccountIdFor<T>,
		owner: AccountIdFor<T>,
		spender: AccountIdFor<T>,
	) -> Option<Self::Balance>;

	fn approve(
		asset_address: AccountIdFor<T>,
		spender: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult;

	fn transfer_from(
		asset_address: AccountIdFor<T>,
		from: AccountIdFor<T>,
		to: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult;
}

type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

impl<T: Config> Pallet<T> {
	fn total_supply(asset_address: AccountIdFor<T>) -> Option<BalanceOf<T>> {
		T::TokenAccess::total_supply(asset_address)
	}
	fn balance_of(asset_address: AccountIdFor<T>, target: AccountIdFor<T>) -> Option<BalanceOf<T>> {
		T::TokenAccess::balance_of(asset_address, target)
	}

	fn transfer(
		asset_address: AccountIdFor<T>,
		target: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult {
		T::TokenAccess::transfer(asset_address, target, amount)
	}

	fn allowance(
		asset_address: AccountIdFor<T>,
		owner: AccountIdFor<T>,
		spender: AccountIdFor<T>,
	) -> Option<BalanceOf<T>> {
		T::TokenAccess::allowance(asset_address, owner, spender)
	}

	fn approve(
		asset_address: AccountIdFor<T>,
		spender: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult {
		T::TokenAccess::approve(asset_address, spender, amount)
	}

	fn transfer_from(
		asset_address: AccountIdFor<T>,
		from: AccountIdFor<T>,
		to: AccountIdFor<T>,
		amount: U256,
	) -> DispatchResult {
		T::TokenAccess::transfer_from(asset_address, from, to, amount)
	}
}
