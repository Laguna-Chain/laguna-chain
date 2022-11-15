//! If a contract is being deployed from the pallet-system-contract-deployer then
//! the salt is expected to contain a 32-byte encoded value of the destined address
//! otherwise the DefaultAddressGenerator provided in the pallet-contract is used

use core::marker::PhantomData;

use frame_support::{sp_runtime, traits::Get};
use pallet_contracts::AddressGenerator;
use sp_core::ByteArray;
use sp_runtime::traits::AccountIdConversion;

/// generate fixed-address if the deployer is the system-deployer, otherwise use the specified
/// address generator
pub struct FixedAddressGenerator<A>(PhantomData<A>);

impl<A: AddressGenerator<T>, T: crate::Config> AddressGenerator<T> for FixedAddressGenerator<A>
where
	T::AccountId: ByteArray,
{
	fn generate_address(
		deploying_address: &<T as frame_system::Config>::AccountId,
		code_hash: &T::Hash,
		salt: &[u8],
	) -> <T as frame_system::Config>::AccountId {
		// Retrieving the pallet_system_contract_deployer AccountId
		let key = <T as crate::Config>::PalletId::get()
			.try_into_account()
			.expect("Invalid PalletId");

		if deploying_address == &key {
			// NOTICE: address check should be done inside the call, we do not check for 0x0 address
			// here
			if let Ok(addr) = T::AccountId::from_slice(salt) {
				return addr
			}
		}

		<A as AddressGenerator<T>>::generate_address(deploying_address, code_hash, salt)
	}
}
