#![cfg_attr(not(feature = "std"), no_std)]

use crate::{
	constants::{LAGUNAS, MILLI_LAGUNAS},
	impl_frame_system::BlockWeights,
	impl_pallet_currencies::NativeCurrencyId,
	Call, Event, RandomnessCollectiveFlip, Runtime, Timestamp, TransactionPayment, Weight, Vec,
};
use frame_support::{parameter_types, pallet_prelude::Decode};
use orml_tokens::CurrencyAdapter;
use pallet_contracts::{AddressGenerator, DefaultAddressGenerator, DefaultContractAccessWeight};

mod chain_extensions;
use chain_extensions::DemoExtension;
use pallet_contracts::weights::WeightInfo;
use primitives::Balance;
use sp_runtime::{Perbill, AccountId32};
use sp_core::crypto::UncheckedFrom;

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * LAGUNAS + (bytes as Balance) * (5 * MILLI_LAGUNAS / 100)) / 10
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

// If the deploying address is [0;32] and the salt is 32-byte length then the salt
// is the generated address otherwise default way of address generation is used
pub struct CustomAddressGenerator;

impl<T> AddressGenerator<T> for CustomAddressGenerator 
where
	T: frame_system::Config,
	T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>, 
{
	fn generate_address(
		deploying_address: &T::AccountId, 
		code_hash: &T::Hash, 
		salt: &[u8]
	) -> T::AccountId {

		let zero_address = AccountId32::new([0u8;32]);
		let zero_address = T::AccountId::decode(&mut zero_address.as_ref()).unwrap();

		if deploying_address == &zero_address && salt.len() == 32 {
			let salt: [u8;32] = salt.try_into().unwrap();
			let new_address = AccountId32::from(salt);
			T::AccountId::decode(&mut new_address.as_ref())
				.expect("Cannot create an AccountId from the given salt")
		} else {
			<DefaultAddressGenerator as AddressGenerator<T>>::generate_address(deploying_address, code_hash, salt)
		}
	}
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
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;

	type DepositPerByte = DepositPerByte;

	type DepositPerItem = DepositPerItem;

	type AddressGenerator = CustomAddressGenerator;

	type ContractAccessWeight = DefaultContractAccessWeight<()>;
}
