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

use sp_core::{Decode, Encode, RuntimeDebug};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::scale_info::TypeInfo;

#[derive(Encode, Decode, RuntimeDebug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    NativeToken(TokenId), // Currently only one native type is defined
                          // TODO: Erc20, expose whitelisted evm token later
}

#[derive(Encode, Decode, RuntimeDebug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenId {
    Hydro, // Native token of the hydro-chain
    GasToken,
}
