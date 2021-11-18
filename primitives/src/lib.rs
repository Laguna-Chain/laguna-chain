// Primitives are used for internal representation across runtime and node

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

pub type BlockNumber = u32;

pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type Balance = u128;

pub type Index = u32;

pub type Hash = sp_core::H256;
