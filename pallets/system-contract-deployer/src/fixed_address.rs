//! If a contract is being deployed from the pallet-system-contract-deployer then
//! the salt is expected to contain a 32-byte encoded value of the destined address
//! otherwise the DefaultAddressGenerator provided in the pallet-contract is used

use frame_support::{pallet_prelude::Decode, sp_runtime, traits::Get};
use pallet_contracts::{AddressGenerator, DefaultAddressGenerator};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

pub struct CustomAddressGenerator;

impl<T> AddressGenerator<T> for CustomAddressGenerator
where
	T: crate::Config,
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
	fn generate_address(
		deploying_address: &T::AccountId,
		code_hash: &T::Hash,
		salt: &[u8],
	) -> T::AccountId {
		// Retrieving the pallet_system_contract_deployer AccountId
		let key = <T as crate::Config>::PalletId::get().into_account_truncating();

		if deploying_address == &key {
			// Decoding the salt to the destined deployment contract address
			let destined_addr: [u8; 32] = salt.try_into().unwrap();
			let contract_addr = AccountId32::from(destined_addr);
			return T::AccountId::decode(&mut contract_addr.as_ref())
				.expect("Cannot create an AccountId from the given salt")
		}

		<DefaultAddressGenerator as AddressGenerator<T>>::generate_address(
			deploying_address,
			code_hash,
			salt,
		)
	}
}
