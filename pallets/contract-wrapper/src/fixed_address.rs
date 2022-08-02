use frame_support::pallet_prelude::Decode;
use pallet_contracts::{AddressGenerator, DefaultAddressGenerator};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::AccountId32;

// If the deploying address is [0;32] and the salt is 32-byte length then the salt
// is the generated address otherwise default way of address generation is used
pub struct CustomAddressGenerator;

impl<T> AddressGenerator<T> for CustomAddressGenerator
where
	T: frame_system::Config,
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
	fn generate_address(
		deploying_address: &T::AccountId,
		code_hash: &T::Hash,
		destined_addr: &[u8],
	) -> T::AccountId {
		let zero_address = AccountId32::new([0u8; 32]);
		let zero_address = T::AccountId::decode(&mut zero_address.as_ref()).unwrap();

		if deploying_address == &zero_address && destined_addr.len() == 32 {
			let destined_addr: [u8; 32] = destined_addr.try_into().unwrap();
			let new_address = AccountId32::from(destined_addr);
			T::AccountId::decode(&mut new_address.as_ref())
				.expect("Cannot create an AccountId from the given destined_addr")
		} else {
			<DefaultAddressGenerator as AddressGenerator<T>>::generate_address(
				deploying_address,
				code_hash,
				destined_addr,
			)
		}
	}
}
