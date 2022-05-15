#![cfg_attr(not(feature = "std"), no_std)]

use crate::{
	constants::{HYDROS, MILLI_HYDRO},
	impl_frame_system::BlockWeights,
	impl_orml_tokens::NativeCurrencyId,
	Balances, Call, Event, RandomnessCollectiveFlip, Runtime, Timestamp, TransactionPayment,
	Weight,
};
use frame_support::parameter_types;
use orml_tokens::CurrencyAdapter;
use pallet_contracts::DefaultAddressGenerator;

mod chain_extensions;
use chain_extensions::DemoExtension;
use pallet_contracts::weights::WeightInfo;
use primitives::Balance;
use sp_runtime::Perbill;

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * HYDROS + (bytes as Balance) * (5 * MILLI_HYDRO / 100)) / 10
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
		BlockWeights::get().max_block;
	pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
		)) / 5) as u32;
	pub Schedule: pallet_contracts::Schedule<Runtime> = {
		let mut schedule = pallet_contracts::Schedule::<Runtime>::default();
		// We decided to **temporarily* increase the default allowed contract size here
		// (the default is `128 * 1024`).
		//
		// Our reasoning is that a number of people ran into `CodeTooLarge` when trying
		// to deploy their contracts. We are currently introducing a number of optimizations
		// into ink! which should bring the contract sizes lower. In the meantime we don't
		// want to pose additional friction on developers.
		schedule.limits.code_len = 256 * 1024;
		schedule
	};
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = CurrencyAdapter<Runtime, NativeCurrencyId>;
	type Event = Event;
	type Call = Call;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = frame_support::traits::Nothing;
	type WeightPrice = TransactionPayment;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = DemoExtension;
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 31];

	type DepositPerByte = DepositPerByte;

	type DepositPerItem = DepositPerItem;

	type AddressGenerator = DefaultAddressGenerator;
}
