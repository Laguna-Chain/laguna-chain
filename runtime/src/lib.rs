// assume no_std if build for wasm, which sp-std provides alternative impl
#![cfg_attr(not(feature = "std"), no_std)]
// increate recursive limit for construct_runtime! macro
#![recursion_limit = "256"]

// wasm_binary.rs is provided by build.rs
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub use frame_support::construct_runtime;

use sp_api::{impl_runtime_apis, ApisVec};
use sp_runtime::create_runtime_str;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// TODO: include all needed pallets and their impl

// TODO: import all type aliases from primives

// TODO: include all needed const as well

// version declaration for wasm runtime copy from substrate-node-template
// #[sp_version::runtime_version]
// pub const VERSION: RuntimeVersion = RuntimeVersion {
//     spec_name: create_runtime_str!("hydro-runtime-placeholder"),
//     impl_name: create_runtime_str!("hydro-runtime-placeholder"),
//     authoring_version: 1,
//     spec_version: 100,
//     impl_version: 1,
//     apis: RUNTIME_API_VERSIONS,
//     transaction_version: 1,
// };

// version declaration for native runtime
// #[cfg(feature = "std")]
// pub fn native_version() -> NativeVersion {
//     NativeVersion {
//         runtime_version: VERSION,
//         can_author_with: Default::default(),
//     }
// }

// TODO: properly impl frame_system for Runtime
// impl frame_system::Config for Runtime {}

// TODO: type holder before we can properly construct runtime
#[cfg(feature = "runtime-placeholder")]
mod placeholder {

    #[derive(Eq, PartialEq, Clone)]
    pub enum Runtime {}
}

#[cfg(feature = "runtime-placeholder")]
use placeholder::*;

// runtime as enum, can cross reference enum variants as pallet impl type associates
// construct_runtime!(
//     pub enum Runtime {}
// );

// expose runtime apis
// impl_runtime_apis! {}
