// Primitives are used for internal representation across runtime and node

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature,
};

pub type BlockNumber = u32;

pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type Balance = u128;

pub type Index = u32;

pub type Hash = sp_core::H256;

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

// time measurement based on adjustable blocktime
// allow conversion based on runtime specified milliseconds per block
// derived from substrate-node-template

pub const fn minutes(millisec_per_block: u64) -> BlockNumber {
    60_000 / (millisec_per_block as BlockNumber)
}

pub const fn hours(millisec_per_block: u64) -> BlockNumber {
    minutes(millisec_per_block) * 60
}

pub const fn days(millisec_per_block: u64) -> BlockNumber {
    hours(millisec_per_block) * 24
}
