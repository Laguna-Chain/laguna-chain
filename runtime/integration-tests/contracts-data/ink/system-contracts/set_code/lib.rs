#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;
pub use self::set_code::SetCodeRef;

#[derive(scale::Encode, scale::Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExtensionError {
	ExecutionFailed,
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
			_ => Err(Self::ExecutionFailed),
		}
	}
}

#[ink::chain_extension]
pub trait UpgradeContractExt {
	type ErrorCode = ExtensionError;

	#[ink(extension = 11)]
	fn set_code(acc: ink_env::AccountId, ch: ink_env::Hash) -> Result<(), ExtensionError>;
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

	type ChainExtension = UpgradeContractExt;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod set_code {

	use ink_storage::{traits::SpreadAllocate, Mapping};

	#[ink(storage)]
	#[derive(SpreadAllocate)]
	pub struct SetCode {
		/// Stores the code versions of the contracts
		version: Mapping<AccountId, u32>,
	}

	/// Event emitted when a contract successfully upgrades
	#[ink(event)]
	pub struct ContractUpgraded {
		#[ink(topic)]
		account_id: AccountId,
		version: u32,
		code_hash: Hash,
	}

	impl SetCode {
		/// Constructor that initializes the `bool` value to the given `init_value`.
		#[ink(constructor)]
		pub fn new() -> Self {
			ink_lang::utils::initialize_contract(|_| {})
		}

		/// Contracts can call this method to change their code in-place.
		/// @dev: It doesn't change any storage items. For more details refer to
		/// https://docs.openzeppelin.com/upgrades-plugins/writing-upgradeable#modifying-your-contracts
		#[ink(message, selector = 0xe4d2b1ca)]
		pub fn replace_code(&mut self, code_hash: Hash) -> bool {
			let caller = self.env().caller();
			let version = self.code_version(caller);

			// Verifies that the caller is a contract address and version doesn't overflow from the
			// upgrade
			if !self.env().is_contract(&caller) || version == u32::MAX {
				return false
			}

			let present_code_hash = self.env().code_hash(&caller).unwrap();

			// Calls runtime to set new code_hash and checks status
			if self.env().extension().set_code(caller, code_hash).is_err() {
				return false
			}

			// For consistency purposes, Emits event for the genesis version of the contract too
			if version == 0 {
				self.env().emit_event(ContractUpgraded {
					account_id: caller,
					version,
					code_hash: present_code_hash,
				});
			}

			self.version.insert(&caller, &(version + 1));
			self.env().emit_event(ContractUpgraded {
				account_id: caller,
				version: version + 1,
				code_hash,
			});
			true
		}

		/// Returns the number of times the contract has been updated
		#[ink(message, selector = 0xe82d14a6)]
		pub fn code_version(&self, addr: AccountId) -> u32 {
			self.version.get(&addr).unwrap_or(0)
		}
	}
}
