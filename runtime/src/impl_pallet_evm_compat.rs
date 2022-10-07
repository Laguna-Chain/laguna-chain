use frame_support::{
	sp_runtime::traits::{AccountIdConversion, Convert, Keccak256},
	traits::ConstU64,
};
use pallet_contracts::AddressGenerator;
use pallet_evm::{AddressMapping, HashedAddressMapping};
use pallet_system_contract_deployer::CustomAddressGenerator;
use primitives::{AccountId, Balance};
use sp_core::{H160, U256};

use crate::Runtime;

impl pallet_evm_compat::Config for Runtime {
	type AddressMapping = HashedAddressMapping<Keccak256>;
	type ContractAddressMapping = PlainContractAddressMapping;

	type ChainId = ConstU64<1000>;

	type WeightToFee = <Runtime as pallet_transaction_payment::Config>::WeightToFee;
}

pub const ETH_ACC_PREFIX: &[u8; 4] = b"evm:";
pub const ETH_CONTRACT_PREFIX: &[u8; 12] = b"evm_contract";

pub struct PlainContractAddressMapping;

impl AddressMapping<AccountId> for PlainContractAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		let mut out = [0_u8; 32];

		out[0..12].copy_from_slice(ETH_CONTRACT_PREFIX);
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
		let key: AccountId = <Runtime as pallet_system_contract_deployer::Config>::PalletId::get()
			.try_into_account()
			.expect("Invalid PalletId");

		let generated = <CustomAddressGenerator as AddressGenerator<Runtime>>::generate_address(
			deploying_address,
			code_hash,
			salt,
		);
		let raw: [u8; 32] = generated.into();

		let h_addr = if *deploying_address == key {
			// we took trailing 20 bytes as input for system contracts
			H160::from_slice(&raw[12..])
		} else {
			// we took leading 20 bytes as input from normal contracts
			H160::from_slice(&raw[0..20])
		};

		// add contract-specific prefix
		PlainContractAddressMapping::into_account_id(h_addr)
	}
}
