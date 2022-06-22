#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::{AccountId, Environment};
use ink_lang as ink;
use ink_prelude::vec::Vec as StorageVec;
use scale::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExtensionError {
	InvalidTokenId,
	InsufficientBalance,
	InsufficientAllowance,
	UnknownStatusCode,
	InvalidScaleEncoding,
}

impl From<scale::Error> for ExtensionError {
	fn from(_: scale::Error) -> Self {
		ExtensionError::InvalidScaleEncoding
	}
}

impl ink_env::chain_extension::FromStatusCode for ExtensionError {
	fn from_status_code(status_code: u32) -> Result<(), Self> {
		match status_code {
			0 => Ok(()),
			1 => Err(Self::InvalidTokenId),
			2 => Err(Self::InsufficientBalance),
			_ => Err(Self::UnknownStatusCode),
		}
	}
}

type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;

#[ink::chain_extension]
pub trait NativeTokenRuntimeExt {
	type ErrorCode = ExtensionError;

	#[ink(extension = 0010, returns_result = false)]
	fn whitelist_contract();

	#[ink(extension = 2000, returns_result = false)]
	fn is_valid_token(token_id: u32);

	#[ink(extension = 2001)]
	fn name(token_id: u32) -> Result<StorageVec<u8>, ExtensionError>;

	#[ink(extension = 2002)]
	fn symbol(token_id: u32) -> Result<StorageVec<u8>, ExtensionError>;

	#[ink(extension = 2003)]
	fn decimals(token_id: u32) -> Result<u8, ExtensionError>;

	#[ink(extension = 2004)]
	fn total_supply(token_id: u32) -> Result<Balance, ExtensionError>;

	#[ink(extension = 2005)]
	fn balance_of(token_id: u32, owner: AccountId) -> Result<Balance, ExtensionError>;

	#[ink(extension = 2006, returns_result = false)]
	fn transfer(token_id: u32, to: AccountId, value: Balance);

	#[ink(extension = 2007, returns_result = false)]
	fn transfer_from(token_id: u32, from: AccountId, to: AccountId, value: Balance);
}

// Contract needs the environment that understand our extension
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
	const MAX_EVENT_TOPICS: usize = <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

	type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
	type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
	type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
	type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
	type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;

	type ChainExtension = NativeTokenRuntimeExt;
}

pub use self::native_fungible_token::NativeTokenRef;

#[ink::contract(env = crate::CustomEnvironment)]
mod native_fungible_token {

	use super::{ExtensionError, StorageVec};
	use ink_storage::{traits::SpreadAllocate, Mapping};

	#[ink(storage)]
	#[derive(SpreadAllocate)]
	pub struct NativeToken {
		/// Native runtime token ID
		token_id: u32,
		/// Mapping of the token amount which an account is allowed to withdraw
		/// from another account.
		allowances: Mapping<(AccountId, AccountId), Balance>,
	}

	/// Event emitted when a token transfer occurs.
	#[ink(event)]
	pub struct Transfer {
		#[ink(topic)]
		from: Option<AccountId>,
		#[ink(topic)]
		to: Option<AccountId>,
		value: Balance,
	}

	/// Event emitted when an approval occurs that `spender` is allowed to withdraw
	/// up to the amount of `value` tokens from `owner`.
	#[ink(event)]
	pub struct Approval {
		#[ink(topic)]
		owner: AccountId,
		#[ink(topic)]
		spender: AccountId,
		value: Balance,
	}

	/// The ERC-20 result type
	pub type Result<T> = core::result::Result<T, ExtensionError>;

	impl NativeToken {
		/// Creates an ERC-20 contract wrapper around an existing native token
		#[ink(constructor)]
		pub fn create_wrapper_token(token_id: u32) -> Self {
			// Checks if a native token with given token_id exists in the runtime
			if Self::env().extension().is_valid_token(token_id).is_err() {
				panic!("Invalid tokenId")
			}

			// Allows instantaition from priviledged account only (ROOT for now)
			if Self::env().extension().whitelist_contract().is_err() {
				panic!("Failed to whitelist the contract")
			}
			ink_lang::utils::initialize_contract(|contract| Self::new_init(contract, token_id))
		}

		fn new_init(&mut self, token_id: u32) {
			self.token_id = token_id
		}

		/// Returns the name of the token
		#[ink(message)]
		pub fn name(&self) -> StorageVec<u8> {
			self.env()
				.extension()
				.name(self.token_id)
				.expect("TokenId once created is never destroyed")
		}

		/// Returns the ticker of the token
		#[ink(message)]
		pub fn symbol(&self) -> StorageVec<u8> {
			self.env()
				.extension()
				.symbol(self.token_id)
				.expect("TokenId once created is never destroyed")
		}

		/// Returns the decimals places used in the token
		#[ink(message)]
		pub fn decimals(&self) -> u8 {
			self.env()
				.extension()
				.decimals(self.token_id)
				.expect("TokenId once created is never destroyed")
		}

		/// Returns the total token supply
		#[ink(message)]
		pub fn total_supply(&self) -> Balance {
			self.env()
				.extension()
				.total_supply(self.token_id)
				.expect("TokenId once created is never destroyed")
		}

		/// Returns the account balance for the specified `owner`
		#[ink(message)]
		pub fn balance_of(&self, owner: AccountId) -> Balance {
			self.env()
				.extension()
				.balance_of(self.token_id, owner)
				.expect("TokenId once created is never destroyed")
		}

		/// Returns the amount which `spender` is still allowed to withdraw from `owner`.
		///
		/// Returns `0` if no allowance has been set.
		#[ink(message)]
		pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
			self.allowances.get((owner, spender)).unwrap_or_default()
		}

		/// Transfers `value` amount of tokens from the caller's account to account `to`.
		///
		/// On success a `Transfer` event is emitted.
		///
		/// # Errors
		///
		/// Returns `InsufficientBalance` error if there are not enough tokens on
		/// the caller's account balance.
		#[ink(message)]
		pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
			self.env().extension().transfer(self.token_id, to, value)?;
			self.env().emit_event(Transfer {
				from: Some(self.env().caller()),
				to: Some(to),
				value,
			});
			Ok(())
		}

		/// Allows `spender` to withdraw from the caller's account multiple times, up to
		/// the `value` amount.
		///
		/// If this function is called again it overwrites the current allowance with `value`.
		///
		/// An `Approval` event is emitted.
		#[ink(message)]
		pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
			let owner = self.env().caller();
			self.allowances.insert((&owner, &spender), &value);
			self.env().emit_event(Approval { owner, spender, value });
			Ok(())
		}

		/// Transfers `value` tokens on the behalf of `from` to the account `to`.
		///
		/// On success a `Transfer` event is emitted.
		///
		/// # Errors
		///
		/// Returns `InsufficientAllowance` error if there are not enough tokens allowed
		/// for the caller to withdraw from `from`.
		///
		/// Returns `InsufficientBalance` error if there are not enough tokens on
		/// the account balance of `from`.
		#[ink(message)]
		pub fn transfer_from(
			&mut self,
			from: AccountId,
			to: AccountId,
			value: Balance,
		) -> Result<()> {
			let caller = self.env().caller();
			let allowance = self.allowance(from, caller);
			if allowance < value {
				return Err(ExtensionError::InsufficientAllowance)
			}
			self.env().extension().transfer_from(self.token_id, from, to, value)?;
			self.allowances.insert((&from, &caller), &(allowance - value));
			self.env().emit_event(Transfer { from: Some(from), to: Some(to), value });
			Ok(())
		}
	}
}
