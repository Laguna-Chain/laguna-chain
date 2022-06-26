#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;
use scale::{Decode, Encode};

// define the extension that match the interface defined in your runtime
#[ink::chain_extension]
pub trait DummyRuntimeExt {
	type ErrorCode = ExtensionError;

	// match extension ID on your runtime
	#[ink(extension = 1000)]
	fn exposed_method(input: [u8; 32]) -> Result<[u8; 32], ExtensionError>;
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExtensionError {
	ExtError,
}

// contract need the environment that understand your extension
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

	type ChainExtension = DummyRuntimeExt;
}

impl ink_env::chain_extension::FromStatusCode for ExtensionError {
	fn from_status_code(status_code: u32) -> Result<(), Self> {
		match status_code {
			0 => Ok(()),
			1 => Err(Self::ExtError),
			_ => panic!("encountered unknown status code"),
		}
	}
}

// specify the environment in your contract
#[ink::contract(env = crate::CustomEnvironment)]
mod dummy_extension_consumer {

	use super::ExtensionError;

	#[ink(storage)]
	pub struct DummyExtensionConsumer {}

	impl DummyExtensionConsumer {
		#[ink(constructor)]
		pub fn default() -> Self {
			Self {}
		}

		#[ink(message)]
		pub fn call_extension(&self, input: [u8; 32]) -> Result<[u8; 32], ExtensionError> {
			let out = self.env().extension().exposed_method(input)?;

			Ok(out)
		}
	}
}
