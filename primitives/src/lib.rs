//! Primitives are used for internal representation across runtime and node

#![cfg_attr(not(feature = "std"), no_std)]

pub use sp_runtime::traits::IdentifyAccount;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Verify},
	FixedU128, MultiSignature,
};

pub(crate) mod currency;
pub use currency::*;

pub type BlockNumber = u32;

pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type AccountPublic = <Signature as Verify>::Signer;

pub type Balance = u128;

pub type Price = FixedU128;

pub type Amount = i128;

pub type Index = u32;

pub type Hash = sp_core::H256;

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
