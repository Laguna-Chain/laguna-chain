use core::marker::PhantomData;

use pallet_contracts::AddressGenerator;
use sp_core::crypto::ByteArray;

// evm addrss need to start with specific prefix, so that it can be converted between H160 and
// AccountIdOf<T>
pub struct EvmAddressGeneretor<A>(PhantomData<A>);

impl<A: AddressGenerator<T>, T: crate::Config> AddressGenerator<T> for EvmAddressGeneretor<A>
where
	T::AccountId: ByteArray,
{
	fn generate_address(
		deploying_address: &<T as frame_system::Config>::AccountId,
		code_hash: &T::Hash,
		salt: &[u8],
	) -> <T as frame_system::Config>::AccountId {
		let mut addr =
			<A as AddressGenerator<T>>::generate_address(deploying_address, code_hash, salt)
				.to_raw_vec();

		// override the first 12 bytes with evm compatible prefix
		addr[0..12].copy_from_slice(b"evm_contract");

		T::AccountId::from_slice(addr.as_ref()).unwrap()
	}
}
