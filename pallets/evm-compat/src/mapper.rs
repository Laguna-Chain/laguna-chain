use crate::Config;
use ethereum::{
	BlockV2 as EthereumBlock, EIP1559ReceiptData, EIP1559Transaction, EIP2930Transaction,
	LegacyTransaction, Log, PartialHeader, ReceiptV3 as EthereumReceipt,
	TransactionV2 as EthereumTransaction,
};
use ethereum_types::{Bloom, BloomInput};
use fp_rpc::TransactionStatus;
use frame_system::{EventRecord, Phase};

use codec::Encode;
use frame_support::{
	sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto},
	sp_std::prelude::*,
	traits::FindAuthor,
};
use sp_core::{H160, H256, U256};

pub trait BlockFilter {
	type Runtime: frame_system::Config;
	type Block: BlockT;

	fn filter_extrinsic(ext: &<Self::Block as BlockT>::Extrinsic) -> Option<EthereumTransaction>;

	fn result_event(
		record: &EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<bool>;

	fn payload_event(
		record: &EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<H160>;

	fn create_event(
		record: &EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<H160>;

	fn contract_event(
		record: &EventRecord<
			<Self::Runtime as frame_system::Config>::Event,
			<Self::Runtime as frame_system::Config>::Hash,
		>,
	) -> Option<(H160, Vec<u8>)>;
}

pub trait MapBlock<Block, T>: BlockFilter<Block = Block, Runtime = T>
where
	Block: BlockT,
	T: Config<Hash = H256>,
	<Block as BlockT>::Header: HeaderT<Hash = H256>,
	<<Block as BlockT>::Header as HeaderT>::Number: Into<U256>,
	<T as pallet_timestamp::Config>::Moment: UniqueSaturatedInto<u64>,
{
	type FindAuthor: FindAuthor<H160>;

	fn transaction_status(block: &Block) -> Vec<(TransactionStatus, EthereumReceipt)> {
		let records = frame_system::Pallet::<T>::read_events_no_consensus();

		block
			.extrinsics()
			.iter()
			.enumerate()
			.filter_map(|(idx, ext)| Self::filter_extrinsic(ext).map(|tx| (idx, tx)))
			.enumerate()
			.map(|(tx_id, (ext_id, tx))| {
				let related_records = records
					.iter()
					.filter(
						|record| matches!(record.phase, Phase::ApplyExtrinsic(i) if i == (ext_id as u32)),
					)
					.collect::<Vec<_>>();

				let status_code = related_records
					.iter()
					.find_map(|record| Self::result_event(record.as_ref()).map(|r| r as u8))
					.unwrap_or_default();

				let logs = related_records
					.iter()
					.filter_map(|record| {
						Self::contract_event(record.as_ref()).map(|(address, data)| Log {
							address,
							data,
							topics: record.topics.clone(),
						})
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
					}) => Some(to),
					_ => None,
				};

				let contract_address =
					related_records.iter().find_map(|record| Self::create_event(record.as_ref()));

				let from = related_records
					.iter()
					.find_map(|record| Self::payload_event(record.as_ref()))
					.unwrap_or_default();

				let status = TransactionStatus {
					transaction_hash: tx.hash(),
					transaction_index: tx_id as u32,
					from,
					to,
					contract_address,
					logs,
					logs_bloom: [0_u8; 256].into(),
				};

				let receipt = EthereumReceipt::EIP1559(EIP1559ReceiptData {
					logs: status.logs.clone(),
					logs_bloom: status.logs_bloom,
					used_gas: Default::default(),
					status_code,
				});

				(status, receipt)
			})
			.collect()
	}

	fn partial_header(block: &Block) -> PartialHeader {
		let header = block.header();

		let receipts =
			Self::transaction_status(block).into_iter().map(|(_, b)| b).collect::<Vec<_>>();

		let mut logs_bloom = Bloom::default();

		for logs in receipts.iter().map(|r| match r {
			EthereumReceipt::Legacy(t) |
			EthereumReceipt::EIP1559(t) |
			EthereumReceipt::EIP2930(t) => &t.logs,
		}) {
			for log in logs {
				logs_bloom.accrue(BloomInput::Raw(&log.address[..]));
				for topic in &log.topics {
					logs_bloom.accrue(BloomInput::Raw(&topic[..]));
				}
			}
		}

		let digests = header
			.digest()
			.logs()
			.iter()
			.filter_map(|v| v.as_consensus().map(|(a, b)| (a, b.to_vec())))
			.collect::<Vec<_>>();

		let beneficiary = Self::FindAuthor::find_author(digests.iter().map(|(a, b)| (*a, &b[..])))
			.unwrap_or_default();

		let receipts_root = ethereum::util::ordered_trie_root(receipts.iter().map(rlp::encode));

		PartialHeader {
			parent_hash: *header.parent_hash(),
			beneficiary,
			state_root: *header.state_root(),
			receipts_root,
			logs_bloom,
			difficulty: Default::default(),
			number: (*header.number()).into(),
			gas_limit: Default::default(),
			gas_used: Default::default(),
			timestamp: pallet_timestamp::Pallet::<T>::now().unique_saturated_into(),
			extra_data: header.digest().encode(),
			mix_hash: Default::default(),
			nonce: Default::default(),
		}
	}

	fn get_block(block: &Block) -> EthereumBlock {
		let p_header = Self::partial_header(block);

		let txs = block.extrinsics().iter().filter_map(Self::filter_extrinsic).collect::<Vec<_>>();

		EthereumBlock::new(p_header, txs, vec![])
	}
}
