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
pub trait ScheduleTransferRuntimeExt {
	type ErrorCode = ExtensionError;

	#[ink(extension = 3001, returns_result = false)]
	fn schedule_transfer(
		currency_id: [u8; 32],
		to: AccountId,
		value: Balance,
		when: u32,
		maybe_periodic: Option<(u32, u32)>,
	);

	#[ink(extension = 3002)]
	fn balance_of(currency_id: [u8; 32], owner: AccountId) -> Result<Balance, ExtensionError>;
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

	type ChainExtension = ScheduleTransferRuntimeExt;
}

// pub use self::native_fungible_token::NativeTokenRef;

#[ink::contract(env = crate::CustomEnvironment)]
mod schedule_transfer_token {

	use super::StorageVec;
	use ethereum_types::U256;
	use ink_storage::{traits::SpreadAllocate, Mapping};

	#[ink(storage)]
	#[derive(SpreadAllocate)]
	pub struct NativeToken {}

	/// Event emitted when a token transfer occurs.
	#[ink(event)]
	pub struct ScheduledTransfer {
		#[ink(topic)]
		from: Option<AccountId>,
		#[ink(topic)]
		to: Option<AccountId>,
		#[ink(topic)]
		when: Option<u32>,
		value: U256,
	}

	impl NativeToken {
		/// Returns the account balance for the specified `ow
		#[ink(constructor, selector = 0xDEADBEEF)]
		pub fn new(initial_value: bool) -> Self {
			Self {}
		}

		#[ink(message, selector = 0x70a08231)]
		pub fn balance_of(&self, owner: AccountId, currency_id: [u8; 32]) -> U256 {
			U256::from(
				self.env()
					.extension()
					.balance_of(currency_id, owner)
					.expect("TokenId once created is never destroyed"),
			)
		}
		/// Transfers `value` amount of tokens from the caller's account to account `to`.
		///
		/// On success a `Transfer` event is emitted.
		#[ink(message, selector = 0x5c4ed01e)]
		pub fn schedule_transfer(
			&mut self,
			to: AccountId,
			value: U256,
			when: u32,
			maybe_periodic: Option<(u32, u32)>,
			currency_id: [u8; 32],
		) -> bool {
			let val = match u128::try_from(value) {
				Ok(val) => val,
				Err(_) => return false,
			};
			if self
				.env()
				.extension()
				.schedule_transfer(currency_id, to, val, when, maybe_periodic)
				.is_err()
			{
				return false
			}
			self.env().emit_event(ScheduledTransfer {
				from: Some(self.env().caller()),
				to: Some(to),
				when: Some(when),
				value,
			});
			true
		}
	}
}
