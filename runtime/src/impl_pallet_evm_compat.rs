use crate::{impl_pallet_authorship::AuraAccountAdapter, Event, Runtime};
use ethereum::TransactionV2 as EthereumTransaction;

use frame_support::sp_std::prelude::*;

use frame_support::{
	sp_runtime::traits::{AccountIdConversion, Convert, Keccak256},
	traits::{ConstU64, FindAuthor},
};
use pallet_contracts::AddressGenerator;
use pallet_evm::{AddressMapping, HashedAddressMapping};
use pallet_evm_compat::mapper::{BlockFilter, MapBlock};
use pallet_system_contract_deployer::CustomAddressGenerator;
use primitives::{AccountId, Balance};
use sp_core::{H160, U256};

impl pallet_evm_compat::Config for Runtime {
	type AddressMapping = HashedAddressMapping<Keccak256>;

	type ContractAddressMapping = PlainContractAddressMapping;

	type ChainId = ConstU64<1000>;

	type WeightToFee = <Runtime as pallet_transaction_payment::Config>::WeightToFee;

	type Event = Event;
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

pub struct BlockMapper;

impl BlockFilter for BlockMapper {
	type Runtime = crate::Runtime;
	type Block = crate::Block;

	fn filter_extrinsic(
		ext: &<Self::Block as sp_api::BlockT>::Extrinsic,
	) -> Option<EthereumTransaction> {
		match &ext.0.function {
			crate::Call::EvmCompat(pallet_evm_compat::Call::transact { t }) => Some(t.clone()),
			_ => None,
		}
	}

	fn result_event(
		record: &frame_system::EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<bool> {
		match record.event {
			crate::Event::System(frame_system::Event::ExtrinsicSuccess { .. }) => Some(true),
			crate::Event::System(frame_system::Event::ExtrinsicFailed { .. }) => Some(false),

			_ => None,
		}
	}

	fn payload_event(
		record: &frame_system::EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<H160> {
		match record.event {
			crate::Event::EvmCompat(pallet_evm_compat::Event::PayloadInfo { address, .. }) =>
				Some(address),
			_ => None,
		}
	}

	fn create_event(
		record: &frame_system::EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<H160> {
		match &record.event {
			crate::Event::Contracts(pallet_contracts::Event::Instantiated { contract, .. }) => {
				let addr_slice: &[u8; 32] = contract.as_ref();
				let contract_addr = H160::from_slice(&addr_slice[12..]);

				Some(contract_addr)
			},

			_ => None,
		}
	}

	fn contract_event(
		record: &frame_system::EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<(H160, Vec<u8>)> {
		match &record.event {
			crate::Event::Contracts(pallet_contracts::Event::ContractEmitted {
				contract,
				data,
			}) => {
				let addr_slice: &[u8; 32] = contract.as_ref();
				let contract_addr = H160::from_slice(&addr_slice[12..]);

				Some((contract_addr, data.clone()))
			},

			_ => None,
		}
	}
}

pub struct EvmAuthorFinder;

impl FindAuthor<H160> for EvmAuthorFinder {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		AuraAccountAdapter::find_author(digests).map(|author| {
			// return the first 20 bytes as h160
			let buf: &[u8; 32] = author.as_ref();
			H160::from_slice(&buf[0..20])
		})
	}
}

impl MapBlock<crate::Block, crate::Runtime> for BlockMapper {
	type FindAuthor = EvmAuthorFinder;
}
