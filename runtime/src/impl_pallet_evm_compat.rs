use core::f32::consts::E;

use crate::{
	impl_pallet_authorship::AuraAccountAdapter, Block, Call, Event, EvmCompat, Runtime, System,
	Timestamp,
};
use codec::Encode;
use ethereum::{
	BlockV2 as EthereumBlock, EIP1559Transaction, EIP2930Transaction, EIP658ReceiptData,
	LegacyTransaction, Log, PartialHeader, TransactionV2 as EthereumTransaction,
};

use fp_rpc::TransactionStatus;
use frame_support::sp_std::prelude::*;

use frame_support::{
	sp_runtime::traits::{AccountIdConversion, Convert, Keccak256},
	traits::{ConstU64, FindAuthor},
	weights::DispatchInfo,
};
use frame_system::Phase;
use pallet_contracts::AddressGenerator;
use pallet_evm::{AddressMapping, HashedAddressMapping};
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

pub fn get_receipts(block: Block) -> Vec<EIP658ReceiptData> {
	let statuses = transaction_statuses(block);

	let all_records = System::read_events_no_consensus();

	let mut receipts = vec![];

	for status in statuses.into_iter() {
		// find all related records that matches the same extrinsic
		let related = all_records
			.iter()
			.filter_map(|record| {
				if let Phase::ApplyExtrinsic(i) = record.phase {
					if i == status.transaction_index {
						return Some(&record.event)
					}
				}

				None
			})
			.collect::<Vec<_>>();

		// check the final execution result
		let (status_code, used_weight) = related
			.iter()
			.find_map(|e| match e {
				Event::System(frame_system::Event::ExtrinsicSuccess { dispatch_info }) =>
					Some((1_u8, dispatch_info.weight)),
				Event::System(frame_system::Event::ExtrinsicFailed {
					dispatch_error,
					dispatch_info,
				}) => Some((0_u8, dispatch_info.weight)),
				_ => None,
			})
			.unwrap_or_default();

		let receipt = EIP658ReceiptData {
			status_code,
			used_gas: used_weight.into(),
			logs_bloom: status.logs_bloom,
			logs: status.logs,
		};

		receipts.push(receipt);
	}

	receipts
}

pub fn map_block(block: Block) -> EthereumBlock {
	let statuses = transaction_statuses(block.clone());
	let receipts = get_receipts(block.clone());

	let header = block.header;

	let timestamp = Timestamp::now();

	let digests = header
		.digest
		.logs()
		.iter()
		.filter_map(|v| v.as_consensus().map(|(a, b)| (a, b.to_vec())))
		.collect::<Vec<_>>();

	let beneficiary = AuraAccountAdapter::find_author(digests.iter().map(|(a, b)| (*a, &b[..])))
		.map(|author| {
			// return the first 20 bytes as h160
			let buf: &[u8; 32] = author.as_ref();
			H160::from_slice(&buf[0..20])
		})
		.unwrap_or_default();

	let all_records = System::read_events_no_consensus();

	let eth_txs = block
		.extrinsics
		.into_iter()
		.filter_map(|xt| {
			if let Call::EvmCompat(pallet_evm_compat::Call::transact { t }) = xt.0.function {
				Some(t)
			} else {
				None
			}
		})
		.collect::<Vec<_>>();

	let mut total_used = 0;

	for status in statuses.into_iter() {
		// find all related records that matches the same extrinsic
		let related = all_records
			.iter()
			.filter_map(|record| {
				if let Phase::ApplyExtrinsic(i) = record.phase {
					if i == status.transaction_index {
						return Some(&record.event)
					}
				}

				None
			})
			.collect::<Vec<_>>();

		// check the final execution result
		let used_weight = related
			.iter()
			.find_map(|e| match e {
				Event::System(frame_system::Event::ExtrinsicSuccess {
					dispatch_info: DispatchInfo { weight, .. },
				}) |
				Event::System(frame_system::Event::ExtrinsicFailed {
					dispatch_info: DispatchInfo { weight, .. },
					..
				}) => Some(*weight),
				_ => None,
			})
			.unwrap_or_default();

		total_used += used_weight;
	}

	let receipts_root = ethereum::util::ordered_trie_root(receipts.iter().map(rlp::encode));

	let partial = PartialHeader {
		parent_hash: header.parent_hash,
		beneficiary,
		state_root: header.state_root,
		receipts_root,
		logs_bloom: [0_u8; 256].into(),
		difficulty: Default::default(),
		number: header.number.into(),
		gas_limit: Default::default(), // this should be handled by CheckWeight
		gas_used: total_used.into(),   // total weight used for all eth-txs
		timestamp,
		extra_data: header.digest.encode(),
		mix_hash: Default::default(),
		nonce: Default::default(),
	};

	EthereumBlock::new(partial, eth_txs, vec![])
}

pub fn transaction_statuses(block: Block) -> Vec<TransactionStatus> {
	let mut statuses = vec![];

	let eth_txs = block
		.extrinsics
		.into_iter()
		.filter_map(|xt| {
			if let Call::EvmCompat(pallet_evm_compat::Call::transact { t }) = xt.0.function {
				Some(t)
			} else {
				None
			}
		})
		.collect::<Vec<_>>();

	let all_records = System::read_events_no_consensus();

	for (idx, tx) in eth_txs.iter().enumerate() {
		// find all related records that matches the same extrinsic
		let related = all_records
			.iter()
			.filter_map(|record| {
				if let Phase::ApplyExtrinsic(i) = record.phase {
					if i == (idx as u32) {
						return Some(&record.event)
					}
				}

				None
			})
			.collect::<Vec<_>>();

		// extract all contract events from this call
		let logs = related
			.iter()
			.filter_map(|e| {
				if let Event::Contracts(pallet_contracts::Event::ContractEmitted {
					contract,
					data,
				}) = e
				{
					let addr_slice: &[u8; 32] = contract.as_ref();
					let contract_addr = H160::from_slice(&addr_slice[12..]);

					Some(Log { address: contract_addr, topics: vec![], data: data.clone() })
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		let to = match tx {
			EthereumTransaction::Legacy(LegacyTransaction {
				action: ethereum::TransactionAction::Call(to),
				..
			}) |
			EthereumTransaction::EIP2930(EIP2930Transaction {
				action: ethereum::TransactionAction::Call(to),
				..
			}) |
			EthereumTransaction::EIP1559(EIP1559Transaction {
				action: ethereum::TransactionAction::Call(to),
				..
			}) => Some(*to),
			_ => None,
		};

		let contract_address = related.iter().find_map(|e| {
			if let Event::Contracts(pallet_contracts::Event::Instantiated { deployer, contract }) =
				e
			{
				let addr_slice: &[u8; 32] = contract.as_ref();
				let contract_addr = H160::from_slice(&addr_slice[12..]);

				Some(contract_addr)
			} else {
				None
			}
		});

		let status = TransactionStatus {
			transaction_hash: tx.hash(),
			transaction_index: idx as u32,
			from: EvmCompat::recover_tx_signer(tx).unwrap_or_default(),
			to,
			contract_address,
			logs,
			logs_bloom: [0_u8; 256].into(),
		};

		statuses.push(status);
	}

	statuses
}
