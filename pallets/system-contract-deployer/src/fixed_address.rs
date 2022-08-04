use frame_support::pallet_prelude::Decode;
use pallet_contracts::{AddressGenerator, DefaultAddressGenerator};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::AccountId32;

// If the deploying address is [0;32] and the salt is 32-byte length then the salt
// is the generated address otherwise default way of address generation is used
pub struct CustomAddressGenerator;

impl<T> AddressGenerator<T> for CustomAddressGenerator
where
	T: crate::Config,
	T: frame_system::Config,
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
	fn generate_address(
		deploying_address: &T::AccountId,
		code_hash: &T::Hash,
		salt: &[u8],
	) -> T::AccountId {
		if let Some(key) = crate::Pallet::<T>::deploying_key() {
			if deploying_address == &key && salt.len() == 32 {
				let salt: [u8; 32] = salt.try_into().unwrap();
				let contract_addr = AccountId32::from(salt);
				return T::AccountId::decode(&mut contract_addr.as_ref())
					.expect("Cannot create an AccountId from the given salt")
			}
		}

		<DefaultAddressGenerator as AddressGenerator<T>>::generate_address(
			deploying_address,
			code_hash,
			salt,
		)
	}
}
