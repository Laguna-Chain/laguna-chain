//! pallet-reverse-evm-call
//!
//! > WIP: structure and name may change dramatically
//!
//! this pallet allow setting up multiple smart contract address to be delegated by other
//! substrate-pallets the expected usage would be, using the scheduler to call airdropping methods
//! within solidity
//!
//! by combining this pallet and other exposed feature as precompile:
//! smart contract developer should be able to create subscription like mechanism, where:
//!
//! - smart contract prepare and subscribe to the scheduler precompile library
//! - chain scheduler use triggered an reverse-evm-call for the smart contract to complete its items
//! - evm storages are mutated thus completes the subscription process

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
mod pallet {
	use sp_core::H160;
	use sp_std::vec;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_evm::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// evm identity to call the smart contract
		#[pallet::constant]
		type Caller: Get<H160>;

		/// the specified smart contract address and it's function selector in hashed bytes
		#[pallet::constant]
		type TargetAddress: Get<(H160, Vec<u8>)>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		TargetExecuted,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	// TODO: use WeightInfo trait to compose weight used on this call

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// delecate evm_call of T::TargetAddress as the T::Caller
		#[pallet::weight(100_000)]
		pub fn delegate_call(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// retrieve the contract address and calling function selector
			let (contract, call) = <T::TargetAddress>::get();

			// TODO: get gas_limit and max_fee_per_gas right

			// delegate the targted evm call from this pallet
			let res = pallet_evm::Pallet::<T>::call(
				origin,
				<T::Caller>::get(),
				contract,
				call,
				0_u64.into(),
				1000_000,
				0_u64.into(),
				None,
				None,
				vec![],
			)?;

			// deposit result of the evm call
			Self::deposit_event(Event::TargetExecuted);

			// TODO: customize return type and metadata

			Ok(res)
		}
	}
}
