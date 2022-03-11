#![cfg_attr(not(feature = "std"), no_std)]

use crate::{Balances, Event, Runtime};
use frame_support::parameter_types;
use pallet_evm::{
	EnsureAddressRoot, EnsureAddressTruncated, HashedAddressMapping, SubstrateBlockHashMapping,
};
use precompiles::HydroPrecompiles;
use primitives::AccountId;
use sp_core::U256;
use sp_runtime::traits::BlakeTwo256;

pub mod precompiles;

parameter_types! {

	// TODO: setup correct rule for chain_id
	pub const ChainId: u64 = 1234;
	pub BlockGasLimit: U256 = U256::from(u32::max_value());
	pub PrecompilesValue: HydroPrecompiles<Runtime> = HydroPrecompiles::<_>::new();
}

impl pallet_evm::Config for Runtime {
	type Event = Event;

	// Use platform native token as evm's native token as well
	// type Currency = orml_tokens::CurrencyAdapter<Runtime, NativeCurrencyId>;
	type Currency = Balances;

	// limit the max op allowed in a block
	type BlockGasLimit = BlockGasLimit;

	// will limit min gas needed, combined with gas reduction for finer gas control, default at
	// min=0
	type FeeCalculator = ();

	// TODO: decide which identity should be executing evm
	type CallOrigin = EnsureAddressRoot<AccountId>;

	// specify Origin allowed to receive evm balance
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;

	// eth.gas <=> substrate.weight mapping, default is 1:1
	type GasWeightMapping = ();

	// maintain block order of substrate
	// TODO: evaluate whether we do custom block mapping for external tooling, may at the cost of
	// bigger block-size
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;

	// expose functionalities to evm
	// TODO: include platform specific pallet features to assist solidity developers
	type PrecompilesType = HydroPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;

	type ChainId = ChainId;
	type Runner = pallet_evm::runner::stack::Runner<Self>;

	// for evm gas `gas(op) = max(gas) - reduced`
	// we can specify gas reduction if certain criteria is met, e.g, early return
	type OnChargeTransaction = ();

	// finding the block author in H160 format
	type FindAuthor = ();
}
