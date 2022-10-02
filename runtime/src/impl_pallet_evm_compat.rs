use pallet_contracts::AddressGenerator;
use pallet_evm::{AddressMapping, HashedAddressMapping};
use pallet_system_contract_deployer::CustomAddressGenerator;
use primitives::{AccountId, Balance};
use sp_core::{KeccakHasher, H160, U256};
use sp_runtime::traits::Convert;

use crate::Runtime;

impl pallet_evm_compat::Config for Runtime {
	type BalanceConvert = BalanceConvert;
	type AddressMapping = HashedAddressMapping<KeccakHasher>;
	type ContractAddressMapping = PlainContractAddressMapping;
}

pub struct PlainContractAddressMapping;

impl AddressMapping<AccountId> for PlainContractAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		let mut out = [0_u8; 32];

		out[0..12].copy_from_slice(&b"evm_contract"[..]);
		out[12..].copy_from_slice(&address.0);

		out.into()
	}
}

pub struct BalanceConvert;

impl Convert<U256, Balance> for BalanceConvert {
	fn convert(a: U256) -> Balance {
		a.as_u128()
	}
}

/// generate account address in H160 compatible form
pub struct EvmCompatAdderssGenerator;

type CodeHash<T> = <T as frame_system::Config>::Hash;

impl AddressGenerator<Runtime> for EvmCompatAdderssGenerator {
	fn generate_address(
		deploying_address: &<Runtime as frame_system::Config>::AccountId,
		code_hash: &CodeHash<Runtime>,
		salt: &[u8],
	) -> <Runtime as frame_system::Config>::AccountId {
		let generated = <CustomAddressGenerator as AddressGenerator<Runtime>>::generate_address(
			deploying_address,
			code_hash,
			salt,
		);

		let raw: [u8; 32] = generated.into();

		let h_addr = H160::from_slice(&raw[0..20]);

		PlainContractAddressMapping::into_account_id(h_addr)
	}
}
