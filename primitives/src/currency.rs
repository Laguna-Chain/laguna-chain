//! # The currency module
//!
//! ## currency types: `CurrencyId`
//!
//! **native token**
//!
//! native tokens are defined in substrate where token are regulated by related pallet
//!
//! **(WIP) erc20 tokens**
//!
//! erc20 tokens are tokens implemented by solidity erc20 smart contracts
//!
//! ## native token id: `TokenId`
//!
//! `TokenId` defines Token issued on the platform for various use cases.

use codec::MaxEncodedLen;
use sp_core::{Decode, Encode, RuntimeDebug};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::scale_info::TypeInfo;

pub type AddressRaw = [u8; 32];

#[derive(
	Encode,
	Decode,
	RuntimeDebug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	NativeToken(TokenId),
	Erc20(AddressRaw),
}

#[derive(
	Encode,
	Decode,
	RuntimeDebug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenId {
	Laguna, // Native token of the laguna-chain
	FeeToken,
}

/// metadata about a issued token, provide additional info about token issued on substrate to evm
pub trait TokenMetadata {
	fn symbol(&self) -> &'static str;

	fn name(&self) -> &'static str;

	fn decimals(&self) -> u8;

	fn is_native(&self) -> bool;
}

impl TokenMetadata for CurrencyId {
	fn symbol(&self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Laguna => "LAGUNA",
				TokenId::FeeToken => "HFEE",
			},
			CurrencyId::Erc20(_) => todo!(),
		}
	}

	fn name(&self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Laguna => "LAGUNA",
				TokenId::FeeToken => "LAGUNA fee",
			},
			CurrencyId::Erc20(_) => todo!(),
		}
	}

	fn decimals(&self) -> u8 {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Laguna => 18,
				TokenId::FeeToken => 18,
			},
			CurrencyId::Erc20(_) => todo!(),
		}
	}

	fn is_native(&self) -> bool {
		match self {
			CurrencyId::NativeToken(_) => true,
			CurrencyId::Erc20(_) => false,
		}
	}
}
