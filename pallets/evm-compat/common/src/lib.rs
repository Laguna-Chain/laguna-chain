#![cfg_attr(not(feature = "std"), no_std)]

use ethereum::{
	EIP1559TransactionMessage, EIP2930TransactionMessage, LegacyTransactionMessage,
	TransactionAction, TransactionV2 as EthereumTransaction,
};

use frame_support::sp_runtime::sp_std::prelude::*;
use sp_core::{H160, U256};

/// transaction input aggregated from either request or signed payload
#[derive(Clone)]
pub enum TransactionMessage {
	Legacy(LegacyTransactionMessage),
	EIP2930(EIP2930TransactionMessage),
	EIP1559(EIP1559TransactionMessage),
}

pub enum ActionRequest {
	Create,
	Transfer(H160),
	Call(H160),
}

impl From<EthereumTransaction> for TransactionMessage {
	fn from(tx: EthereumTransaction) -> Self {
		match tx {
			EthereumTransaction::Legacy(t) => TransactionMessage::Legacy(t.into()),
			EthereumTransaction::EIP2930(t) => TransactionMessage::EIP2930(t.into()),
			EthereumTransaction::EIP1559(t) => TransactionMessage::EIP1559(t.into()),
		}
	}
}

/// map evm-fee-request into substrate-standards using pallet-contracts as backend
pub trait EvmFeeRequest {
	/// unit price multiplier
	fn gas_price(&self) -> U256;

	/// max spendings allowed, including both transaction-fee and storage-deposit
	fn max_allowed(&self) -> U256;

	/// max weight allowed
	fn weight_limit(&self) -> U256;

	/// tip added
	fn tip(&self) -> U256;
}

impl EvmFeeRequest for TransactionMessage {
	fn gas_price(&self) -> U256 {
		match self {
			TransactionMessage::Legacy(LegacyTransactionMessage { gas_price, .. })
			| TransactionMessage::EIP2930(EIP2930TransactionMessage { gas_price, .. }) => *gas_price,
			TransactionMessage::EIP1559(EIP1559TransactionMessage { max_fee_per_gas, .. }) => {
				*max_fee_per_gas
			},
		}
	}

	fn max_allowed(&self) -> U256 {
		self.gas_price().saturating_mul(self.weight_limit())
	}

	// use max_priority_fee_per_gas as tip
	fn tip(&self) -> U256 {
		if let TransactionMessage::EIP1559(EIP1559TransactionMessage {
			max_priority_fee_per_gas,
			..
		}) = self
		{
			*max_priority_fee_per_gas
		} else {
			U256::zero()
		}
	}

	fn weight_limit(&self) -> U256 {
		match self {
			TransactionMessage::Legacy(LegacyTransactionMessage { gas_limit, .. })
			| TransactionMessage::EIP2930(EIP2930TransactionMessage { gas_limit, .. })
			| TransactionMessage::EIP1559(EIP1559TransactionMessage { gas_limit, .. }) => *gas_limit,
		}
	}
}

impl EvmFeeRequest for EthereumTransaction {
	fn max_allowed(&self) -> U256 {
		TransactionMessage::from(self.clone()).max_allowed()
	}

	fn tip(&self) -> U256 {
		TransactionMessage::from(self.clone()).tip()
	}

	fn gas_price(&self) -> U256 {
		TransactionMessage::from(self.clone()).gas_price()
	}

	fn weight_limit(&self) -> U256 {
		TransactionMessage::from(self.clone()).weight_limit()
	}
}

pub trait EvmActionRequest {
	fn action_request(&self) -> ActionRequest;

	fn input(&self) -> Vec<u8>;

	fn value(&self) -> U256;
}

impl EvmActionRequest for TransactionMessage {
	fn action_request(&self) -> ActionRequest {
		match self {
			TransactionMessage::Legacy(LegacyTransactionMessage { action, input, .. })
			| TransactionMessage::EIP2930(EIP2930TransactionMessage { action, input, .. })
			| TransactionMessage::EIP1559(EIP1559TransactionMessage { action, input, .. }) => {
				match (action, input) {
					(TransactionAction::Create, _) => ActionRequest::Create,
					(TransactionAction::Call(target), input) => {
						if input.is_empty() {
							ActionRequest::Transfer(*target)
						} else {
							ActionRequest::Call(*target)
						}
					},
				}
			},
		}
	}

	fn input(&self) -> Vec<u8> {
		match self {
			TransactionMessage::Legacy(LegacyTransactionMessage { input, .. })
			| TransactionMessage::EIP2930(EIP2930TransactionMessage { input, .. })
			| TransactionMessage::EIP1559(EIP1559TransactionMessage { input, .. }) => input.clone(),
		}
	}

	fn value(&self) -> U256 {
		match self {
			TransactionMessage::Legacy(LegacyTransactionMessage { value, .. })
			| TransactionMessage::EIP2930(EIP2930TransactionMessage { value, .. })
			| TransactionMessage::EIP1559(EIP1559TransactionMessage { value, .. }) => *value,
		}
	}
}

impl EvmActionRequest for EthereumTransaction {
	fn action_request(&self) -> ActionRequest {
		TransactionMessage::from(self.clone()).action_request()
	}

	fn input(&self) -> Vec<u8> {
		TransactionMessage::from(self.clone()).input()
	}

	fn value(&self) -> U256 {
		TransactionMessage::from(self.clone()).value()
	}
}
