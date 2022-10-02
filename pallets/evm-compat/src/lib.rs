//! ## pallet-evm-compat
//!
//! this pallet allows triggering pallet_contract related operation from a signed ethereum
//! transaction request
//!
//! for this to work, the runtiem need to:
//! 1. prove that incoming eth signed requeset is self_contained
//! 2. lookup from H160 to AccountId is possible
//!
//!
//!
//! ### account and origin mapping
//!
//! considered the following scenario:
//! 1. user start with one substrate account, presumably sr25519 pairs.
//! 2. user start with one ethereum account, a ECDSA pair

#![cfg_attr(not(feature = "std"), no_std)]

use codec::HasCompact;
use ethereum::{Account, TransactionAction, TransactionV2 as Transaction};
use frame_support::{
	crypto::ecdsa::ECDSAExt,
	dispatch::Dispatchable,
	pallet_prelude::*,
	sp_io,
	sp_runtime::traits::{Convert, IdentifyAccount},
	sp_std::{fmt::Debug, prelude::*},
	traits::Currency,
	weights::{DispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;
use pallet_contracts::AddressGenerator;
use pallet_evm::AddressMapping;

use codec::Decode;
use frame_support::sp_runtime::traits::StaticLookup;
pub use pallet::*;
use sp_core::{crypto::UncheckedFrom, ecdsa, H160, H256, U256};
use sp_io::crypto::secp256k1_ecdsa_recover_compressed;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type BalanceOf<T> =
	<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum RawOrigin {
	EthereumTransaction(H160),
}

pub fn ensure_ethereum_transaction<OuterOrigin>(o: OuterOrigin) -> Result<H160, &'static str>
where
	OuterOrigin: Into<Result<RawOrigin, OuterOrigin>>,
{
	match o.into() {
		Ok(RawOrigin::EthereumTransaction(n)) => Ok(n),
		_ => Err("bad origin: expected to be an Ethereum transaction"),
	}
}

/// allow the call to be limited at signed eth transaction
pub struct EnsureEthereumTransaction;
impl<O: Into<Result<RawOrigin, O>> + From<RawOrigin>> EnsureOrigin<O>
	for EnsureEthereumTransaction
{
	type Success = H160;
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().map(|o| match o {
			RawOrigin::EthereumTransaction(id) => id,
		})
	}
}

#[frame_support::pallet]
mod pallet {

	use frame_support::sp_runtime::traits::Convert;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_contracts::Config {
		type BalanceConvert: Convert<U256, BalanceOf<Self>>;
		type AddressMapping: AddressMapping<AccountIdOf<Self>>;

		type ContractAddressMapping: AddressMapping<AccountIdOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
		T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
	{
		// we rely on self_contained call to fetch the correct origin from the eth-transaction
		// payload
		#[pallet::weight(200_000_000)]
		pub fn transact(origin: OriginFor<T>, t: Transaction) -> DispatchResultWithPostInfo {
			// only allow origin obtained from self_contained_call
			let source = ensure_ethereum_transaction(origin)?;

			// convert it to pallet_contract instructions
			let runner = ContractTransactionAdapter::<T>::from_tx(&t);

			runner.call_or_create(source)
		}

		#[pallet::weight(200_000_000)]
		pub fn claim(origin: OriginFor<T>, signed_claim: H256) -> DispatchResultWithPostInfo {
			todo!()
		}
	}
}

use fp_ethereum::TransactionData;

// once we have the TransactionData we can start mapping it to pallet_contract call args
struct ContractTransactionAdapter<T>((TransactionData, PhantomData<T>));

impl<T: Config> ContractTransactionAdapter<T>
where
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	fn inner(&self) -> &TransactionData {
		&self.0 .0
	}
	fn from_tx(tx: &Transaction) -> Self {
		Self((TransactionData::from(tx), Default::default()))
	}

	fn call_or_create(&self, source: H160) -> DispatchResultWithPostInfo {
		match self.inner().action {
			TransactionAction::Call(target) => self.execute_call_request(source, target),
			TransactionAction::Create => self.execute_create_request(source),
		}
	}

	fn execute_call_request(&self, source: H160, target: H160) -> DispatchResultWithPostInfo {
		let contract_addr = Pallet::<T>::account_from_contract_addr(target);

		let contract_addr_source =
			<<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(contract_addr);

		// mapped origin has no known key pair
		let elevated_origin = Pallet::<T>::to_mapped_origin(source);

		// FIXME: make storage_deposit configurable
		pallet_contracts::Pallet::<T>::call(
			elevated_origin,
			contract_addr_source,
			Pallet::<T>::balance_convert(self.inner().value),
			self.inner().gas_limit.as_u64(),
			None,
			self.inner().input.clone(),
		)
	}

	fn execute_create_request(&self, source: H160) -> DispatchResultWithPostInfo {
		// FIXME: etherem use same input field to contain both code and data, we need a way to
		// communicate with tool about our choice of this.
		let mut input_buf = &self.inner().input[..];

		// scale-codec can split vec's on the fly
		let (sel, code) = <(Vec<u8>, Vec<u8>)>::decode(&mut input_buf).unwrap();

		// this origin cannot be controled from outside
		let elevated_origin = Pallet::<T>::to_mapped_origin(source);

		// FIXME: make storage_deposit configurable, make salt configurable
		pallet_contracts::Pallet::<T>::instantiate_with_code(
			elevated_origin,
			Pallet::<T>::balance_convert(self.inner().value),
			self.inner().gas_limit.as_u64(),
			None,
			code,
			sel,
			Default::default(),
		)
	}
}

impl<T: Config> Pallet<T> {
	fn balance_convert(eth_balance: U256) -> BalanceOf<T> {
		<<T as Config>::BalanceConvert as Convert<U256, BalanceOf<T>>>::convert(eth_balance)
	}

	// given a signed source: H160, elevate it to a substrate signed AccountId
	fn to_mapped_origin(source: H160) -> OriginFor<T> {
		let account_id = Self::to_mapped_account(source);

		frame_system::RawOrigin::Signed(account_id).into()
	}

	fn to_mapped_account(source: H160) -> AccountIdOf<T> {
		<T::AddressMapping as AddressMapping<AccountIdOf<T>>>::into_account_id(source)
	}

	pub fn account_from_contract_addr(target: H160) -> AccountIdOf<T> {
		<T::ContractAddressMapping as AddressMapping<AccountIdOf<T>>>::into_account_id(target)
	}
}

// NOTICE: this is mostly copy from pallet-ethereum
impl<T: Config> Pallet<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub(crate) fn recover_tx_signer(transaction: &Transaction) -> Option<H160> {
		let (msg, sig) = Self::unpack_transaction(transaction);
		Self::recover_signer(&msg, &sig).and_then(|p| p.to_eth_address().map(H160).ok())
	}

	/// try recover uncompressed signer from a raw payload
	pub(crate) fn recover_signer(
		msg_raw: &[u8],
		signed_payload_raw: &[u8],
	) -> Option<ecdsa::Public> {
		let sig = sp_core::ecdsa::Signature::try_from(signed_payload_raw).ok()?;

		let msg = <[u8; 32]>::try_from(msg_raw).ok()?;

		secp256k1_ecdsa_recover_compressed(&sig.0, &msg)
			.ok()
			.map(sp_core::ecdsa::Public::from_raw)
	}

	pub(crate) fn unpack_transaction(transaction: &Transaction) -> ([u8; 32], [u8; 65]) {
		// in addition to typical ECDSA signature, eth tx use chain_id in it's signature to avoid
		// replay attack

		let mut sig = [0u8; 65];
		let mut msg = [0u8; 32];

		match transaction {
			Transaction::Legacy(t) => {
				sig[0..32].copy_from_slice(&t.signature.r()[..]);
				sig[32..64].copy_from_slice(&t.signature.s()[..]);
				sig[64] = t.signature.standard_v();
				msg.copy_from_slice(
					&ethereum::LegacyTransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP2930(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&ethereum::EIP2930TransactionMessage::from(t.clone()).hash()[..],
				);
			},
			Transaction::EIP1559(t) => {
				sig[0..32].copy_from_slice(&t.r[..]);
				sig[32..64].copy_from_slice(&t.s[..]);
				sig[64] = t.odd_y_parity as u8;
				msg.copy_from_slice(
					&ethereum::EIP1559TransactionMessage::from(t.clone()).hash()[..],
				);
			},
		}

		(msg, sig)
	}
}

#[repr(u8)]
enum TransactionValidationError {
	#[allow(dead_code)]
	UnknownError,
	InvalidChainId,
	InvalidSignature,
	InvalidGasLimit,
	MaxFeePerGasTooLow,
}

type CheckedInfo<T> = (H160, AccountIdOf<T>, (U256, U256));

impl<T> Call<T>
where
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T: Send + Sync + Config,
	<T as frame_system::Config>::Call:
		Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub fn is_self_contained(&self) -> bool {
		matches!(self, Call::transact { .. })
	}

	pub fn check_self_contained(&self) -> Option<Result<CheckedInfo<T>, TransactionValidityError>> {
		match self {
			Call::transact { t } => {
				let rs = Pallet::<T>::recover_tx_signer(t)
					.map(|s| {
						let o = <<T as Config>::AddressMapping as AddressMapping<AccountIdOf<T>>>::into_account_id(
							s,
						);
						let extra = self.expose_extra();
						(s, o, extra)
					})
					.ok_or_else(|| {
						InvalidTransaction::Custom(
							TransactionValidationError::InvalidSignature as u8,
						)
						.into()
					});

				Some(rs)
			},
			_ => None,
		}
	}

	fn expose_extra(&self) -> (U256, U256) {
		if let Call::transact { t } = self {
			let TransactionData { nonce, max_priority_fee_per_gas, .. } = TransactionData::from(t);
			return (nonce, max_priority_fee_per_gas.unwrap_or_default())
		}

		Default::default()
	}
}
