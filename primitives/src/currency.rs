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
	NativeToken(TokenId), /* Currently only one native type is defined
	                       * TODO: Erc20, expose whitelisted evm token later */
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
	Hydro, // Native token of the hydro-chain
	FeeToken,
}

/// metadata about a issued token, provide additional info about token issued on substrate to evm
pub trait TokenMetadata {
	fn symbol(self) -> &'static str;

	fn name(self) -> &'static str;

	fn decimals(self) -> u8;
}

impl TokenMetadata for CurrencyId {
	fn symbol(self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Hydro => "HYDRO",
				TokenId::FeeToken => todo!(),
			},
		}
	}

	fn name(self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Hydro => "HYDRO",
				TokenId::FeeToken => todo!(),
			},
		}
	}

	fn decimals(self) -> u8 {
		// TODO: correct mapping between substrate issued tokens and tokens issued within evm
		return 18;
	}
}
