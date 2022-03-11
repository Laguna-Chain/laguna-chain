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
	GratitudeToken,
}

/// metadata about a issued token, provide additional info about token issued on substrate to evm
pub trait TokenMetadata {
	fn symbol(&self) -> &'static str;

	fn name(&self) -> &'static str;

	fn decimals(&self) -> u8;

	fn is_native(&self) -> bool;
}

// FIXME: consider implementing macros as in Acala's primitives/currency.rs
impl TokenMetadata for CurrencyId {
	fn symbol(&self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Hydro => "HYDRO",
				TokenId::FeeToken => "HFEE",
				TokenId::GratitudeToken => "HGRAT",
			},
		}
	}

	fn name(&self) -> &'static str {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Hydro => "HYDRO",
				TokenId::FeeToken => "HYDRO fee",
				TokenId::GratitudeToken => "HYDRO gratitude",
			},
		}
	}

	fn decimals(&self) -> u8 {
		match self {
			CurrencyId::NativeToken(token) => match token {
				TokenId::Hydro => 18,
				TokenId::FeeToken => 18,
				TokenId::GratitudeToken => 18,
			},
		}
	}

	fn is_native(&self) -> bool {
		match self {
			CurrencyId::NativeToken(_) => true,
		}
	}
}
