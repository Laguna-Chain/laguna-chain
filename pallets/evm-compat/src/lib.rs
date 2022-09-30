//! ## pallet-evm-compat
//!
//! this pallet allows triggering pallet_contract related operation from a signed ethereum
//! transaction request
//!
//! for this to work, the runtiem need to:
//! 1. prove that incoming eth signed requeset is self_contained
//! 2. lookup from H160 to AccountId is possible

#![cfg_attr(not(feature = "std"), no_std)]

use codec::HasCompact;
use ethereum::{TransactionAction, TransactionV2 as Transaction};
use frame_support::{
	crypto::ecdsa::ECDSAExt,
	dispatch::Dispatchable,
	pallet_prelude::*,
	sp_io,
	sp_runtime::traits::Convert,
	sp_std::{fmt::Debug, prelude::*},
	traits::{Currency, OriginTrait},
	weights::{DispatchInfo, PostDispatchInfo},
};
use frame_system::pallet_prelude::*;

use codec::Decode;
use frame_support::sp_runtime::traits::StaticLookup;
pub use pallet::*;
use sp_core::{crypto::UncheckedFrom, H160, U256};
use sp_io::crypto::secp256k1_ecdsa_recover;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

type BalanceOf<T> =
	<<T as pallet_contracts::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

type SysOrigin<T> = <T as frame_system::Config>::Origin;

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
		type AddrLookup: StaticLookup<Source = H160, Target = AccountIdOf<Self>>;

		type BalanceConvert: Convert<U256, BalanceOf<Self>>;
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
		pub fn transact(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
			let source = ensure_ethereum_transaction(origin)?;

			let runner = ContractTransactionAdapter::<T>::from_tx(&t);
			runner.call_or_create(source)?;

			Ok(())
		}
	}
}

use fp_ethereum::TransactionData;

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

	fn call_or_create(&self, source: H160) -> DispatchResult {
		match self.inner().action {
			TransactionAction::Call(_) => self.execute_call_request(source),
			TransactionAction::Create => self.execute_create_request(source),
		}
	}

	fn execute_call_request(&self, source: H160) -> DispatchResult {
		let target = if let TransactionAction::Call(target) = self.inner().action {
			Some(target)
		} else {
			None
		}
		.unwrap();

		let contract_addr = <T::AddrLookup as StaticLookup>::lookup(target)?;

		Ok(())
	}

	fn execute_create_request(&self, source: H160) -> DispatchResult {
		// FIXME: etherem use same input field to contain both code and data, we need a way to
		// communicate with tool about our choice of this.
		let mut input_buf = &self.inner().input[..];
		dbg!(&input_buf.len());

		// decode first few bytes as U256 with scale_codec
		let (sel, code) = <(Vec<u8>, Vec<u8>)>::decode(&mut input_buf).unwrap();

		let acc = Pallet::<T>::source_lookup(source).unwrap();

		let raw_origin = frame_support::dispatch::RawOrigin::Signed(acc);
		let sys_origin = OriginFor::<T>::from(raw_origin);

		pallet_contracts::Pallet::<T>::instantiate_with_code(
			sys_origin,
			Pallet::<T>::balance_convert(self.inner().value),
			self.inner().gas_limit.as_u64(),
			None,
			code,
			sel,
			Default::default(),
		)
		.map_err(|e| e.error)
		.map(|o| {
			dbg!(o);
		})?;

		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	fn source_lookup(source: H160) -> Option<AccountIdOf<T>> {
		<<T as Config>::AddrLookup as StaticLookup>::lookup(source).ok()
	}

	fn balance_convert(eth_balance: U256) -> BalanceOf<T> {
		<<T as Config>::BalanceConvert as Convert<U256, BalanceOf<T>>>::convert(eth_balance)
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
		Self::recover_signer(&msg, &sig)
	}

	/// try recover uncompressed signer from a raw payload
	pub(crate) fn recover_signer(msg_raw: &[u8], signed_payload_raw: &[u8]) -> Option<H160> {
		let sig = sp_core::ecdsa::Signature::try_from(signed_payload_raw).ok()?;

		let msg = <[u8; 32]>::try_from(msg_raw).ok()?;

		secp256k1_ecdsa_recover(&sig.0, &msg)
			.ok()
			.and_then(|o| sp_core::ecdsa::Public::from_full(&o).ok())
			.and_then(|pk| pk.to_eth_address().ok())
			.map(H160)
	}

	pub(crate) fn unpack_transaction(transaction: &Transaction) -> ([u8; 32], [u8; 65]) {
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

	fn call_or_create(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
		let action = match &t {
			Transaction::Legacy(t) => t.action,
			Transaction::EIP2930(t) => t.action,
			Transaction::EIP1559(t) => t.action,
		};

		match action {
			ethereum::TransactionAction::Call(_) => Self::call(origin, t),
			ethereum::TransactionAction::Create => Self::create(origin, t),
		}?;

		Ok(())
	}

	fn call(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
		todo!()
	}

	fn create(origin: OriginFor<T>, t: Transaction) -> DispatchResult {
		todo!()
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

	pub fn check_self_contained(
		&self,
	) -> Option<Result<(H160, AccountIdOf<T>, (U256, U256)), TransactionValidityError>> {
		match self {
			Call::transact { t } => {
				let rs = Pallet::<T>::recover_tx_signer(t)
					.and_then(|s| {
						<<T as Config>::AddrLookup as StaticLookup>::lookup(s).ok().map(|o| {
							let extra = self.expose_extra();
							(s, o, extra)
						})
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

	pub fn expose_extra(&self) -> (U256, U256) {
		if let Call::transact { t } = self {
			let TransactionData { nonce, max_priority_fee_per_gas, .. } = TransactionData::from(t);
			return (nonce, max_priority_fee_per_gas.unwrap_or_default())
		}

		Default::default()
	}
}
