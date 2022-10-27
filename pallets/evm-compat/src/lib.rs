//! ## pallet-evm-compat
//!
//! this pallet allows triggering pallet_contract related operation from a signed ethereum
//! transaction request
//!
//! for this to work, the runtiem need to:
//! 1. prove that incoming eth signed requeset is self_contained
//! 2. lookup from H160 to AccountId is possible
//!
//! ### account and origin mapping
//!
//! considered the following scenario:
//! 1. user start with one substrate account, presumably sr25519 pairs.
//! 2. user start with one ethereum account, a ECDSA pair

#![cfg_attr(not(feature = "std"), no_std)]

use codec::HasCompact;
use ethereum::TransactionV2 as Transaction;
use frame_support::{
	crypto::ecdsa::ECDSAExt,
	pallet_prelude::*,
	sp_core_hashing_proc_macro::keccak_256,
	sp_io,
	sp_runtime::traits::{Hash as HashT, Keccak256},
	sp_std::{fmt::Debug, prelude::*},
	traits::Currency,
};
use frame_system::pallet_prelude::*;
use orml_traits::arithmetic::Zero;
use scale_info::prelude::format;

use codec::Decode;
use pallet_contracts_primitives::Code;
use pallet_evm::AddressMapping;

use frame_support::traits::tokens::ExistenceRequirement;
use hex::FromHex;
pub use pallet::*;
use sp_core::{crypto::UncheckedFrom, ecdsa, H160, H256, U256};
use sp_io::{crypto::secp256k1_ecdsa_recover_compressed, hashing::keccak_256};
type CurrencyOf<T> = <T as pallet_contracts::Config>::Currency;
use frame_support::weights::WeightToFee;
use pallet_contracts_primitives::ExecReturnValue;

pub(crate) mod self_contained;
pub(crate) mod tx_adapter;

use tx_adapter::ContractTransactionAdapter;

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

	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_contracts::Config + pallet_proxy::Config
	{
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type AddressMapping: AddressMapping<AccountIdOf<Self>>;

		type ContractAddressMapping: AddressMapping<AccountIdOf<Self>>;

		/// used communicate between gas <-> weight
		type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;

		type ChainId: Get<u64>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::origin]
	pub type Origin = RawOrigin;

	#[pallet::storage]
	#[pallet::getter(fn has_proxy)]
	pub type ProxyAccount<T: Config> = StorageMap<_, Blake2_128Concat, H160, AccountIdOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		PayloadInfo { address: H160, max_fee_allowed: BalanceOf<T>, tip: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		TargetAlreadyProxying,
		NoProxyFound,
		InputBufferUndecodable,
		ConvertionFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
		T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
		BalanceOf<T>: TryFrom<U256> + Into<U256>,
		<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
	{
		// we rely on self_contained call to fetch the correct origin from the eth-transaction
		// payload
		#[pallet::weight(200_000_000)]
		pub fn transact(origin: OriginFor<T>, t: Transaction) -> DispatchResultWithPostInfo {
			// only allow origin obtained from self_contained_call
			let source = ensure_ethereum_transaction(origin)?;

			let (max_allowed, tip) = fee_details::<T>(&t);

			Self::deposit_event(Event::<T>::PayloadInfo {
				address: source,
				max_fee_allowed: max_allowed.try_into().unwrap_or_default(),
				tip: tip.try_into().unwrap_or_default(),
			});

			// convert it to pallet_contract instructions
			let runner = ContractTransactionAdapter::<T>::from_tx(&t);

			runner.call_or_create(source)
		}

		#[pallet::weight(200_000_000)]
		pub fn set_proxy(
			origin: OriginFor<T>,
			who: Option<AccountIdOf<T>>,
			_nonce: U256,
			_sig: Vec<u8>,
		) -> DispatchResult {
			let source = ensure_ethereum_transaction(origin)?;

			if let Some(target) = who {
				// has target, mutate the existing proxy
				// cannot mutate the proxy if the target is already backing someone else
				ensure!(
					Pallet::<T>::acc_is_backing(&target).is_none(),
					Error::<T>::TargetAlreadyProxying
				);
				Pallet::<T>::allow_proxy(source, target)
			} else {
				// cannot remove if it has no proxy
				if let Some(backing) = Pallet::<T>::has_proxy(source) {
					Pallet::<T>::remove_proxy(source, backing)
				} else {
					Err(Error::<T>::NoProxyFound.into())
				}
			}
		}

		/// helper function that could be used by substrate users to prefund the destinated eth
		/// address
		#[pallet::weight(200_000_000)]
		pub fn transfer(origin: OriginFor<T>, target: H160, value: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let target_acc = Self::to_mapped_account(target);

			CurrencyOf::<T>::transfer(&who, &target_acc, value, ExistenceRequirement::KeepAlive)
		}
	}
}

fn fee_details<T: Config>(t: &Transaction) -> (U256, U256)
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
{
	match t {
		Transaction::Legacy(t) => {
			let max_allowed = t.gas_limit * t.gas_price;
			(max_allowed, Default::default())
		},
		Transaction::EIP2930(t) => {
			let max_allowed = t.gas_limit * t.gas_price;
			(max_allowed, Default::default())
		},
		Transaction::EIP1559(t) => {
			let max_allowed = t.gas_limit * t.max_priority_fee_per_gas;
			let tip = t.max_fee_per_gas * t.max_priority_fee_per_gas;
			(max_allowed, tip)
		},
	}
}

impl<T: Config> Pallet<T> {
	// given a signed source: H160, elevate it to a substrate signed AccountId
	fn to_mapped_origin(source: H160) -> OriginFor<T> {
		let account_id = Self::to_mapped_account(source);

		frame_system::RawOrigin::Signed(account_id).into()
	}

	pub fn to_mapped_account(source: H160) -> AccountIdOf<T> {
		<T::AddressMapping as AddressMapping<AccountIdOf<T>>>::into_account_id(source)
	}

	pub fn account_from_contract_addr(target: H160) -> AccountIdOf<T> {
		<T::ContractAddressMapping as AddressMapping<AccountIdOf<T>>>::into_account_id(target)
	}

	pub fn allow_proxy(source: H160, target: AccountIdOf<T>) -> DispatchResult {
		let source_account = Pallet::<T>::to_mapped_account(source);

		let rs = pallet_proxy::Pallet::<T>::add_proxy_delegate(
			&source_account,
			target.clone(),
			Default::default(),
			Default::default(),
		);
		//  update the backing account to the new target
		ProxyAccount::<T>::set(source, Some(target));

		rs
	}

	pub fn remove_proxy(source: H160, target: AccountIdOf<T>) -> DispatchResult {
		let source_account = Pallet::<T>::to_mapped_account(source);

		let rs = pallet_proxy::Pallet::<T>::remove_proxy_delegate(
			&source_account,
			target,
			Default::default(),
			Default::default(),
		);

		ProxyAccount::<T>::remove(source);

		rs
	}

	pub fn is_delegated_by(source: H160) -> Option<AccountIdOf<T>> {
		Pallet::<T>::has_proxy(source)
	}

	// NOTICE: this is derived from Acala's implementation
	fn evm_domain_separator() -> [u8; 32] {
		let domain_hash =
			keccak_256!(b"EIP712Domain(string name,string version,uint256 chainId,bytes32 salt)");
		let mut domain_seperator_msg = domain_hash.to_vec();
		domain_seperator_msg.extend_from_slice(&keccak_256(b"LGNA Proxy")); // name
		domain_seperator_msg.extend_from_slice(&keccak_256(b"1")); // version
		domain_seperator_msg.extend_from_slice(&Into::<[u8; 32]>::into(<u64 as Into<U256>>::into(
			T::ChainId::get(),
		))); // chain id
		domain_seperator_msg.extend_from_slice(
			frame_system::Pallet::<T>::block_hash(T::BlockNumber::zero()).as_ref(),
		); // genesis block hash as salt

		keccak_256!(domain_seperator_msg)
	}

	fn evm_proxy_set_payload(who: &T::AccountId, nonce: &U256) -> [u8; 32] {
		let tx_type_hash = keccak_256(b"Transaction(bytes proxyAccount, uint256 nonce)");
		let mut tx_msg = tx_type_hash.to_vec();

		tx_msg.extend_from_slice(&keccak_256(&who.encode()));
		tx_msg.extend_from_slice(&Into::<[u8; 32]>::into(*nonce));
		keccak_256(tx_msg.as_slice())
	}

	fn evm_proxy_remove_payload(nonce: &U256) -> [u8; 32] {
		let tx_type_hash = keccak_256(b"Transaction(uint256 nonce)");
		let mut tx_msg = tx_type_hash.to_vec();

		tx_msg.extend_from_slice(&Into::<[u8; 32]>::into(*nonce));
		keccak_256(tx_msg.as_slice())
	}

	fn eip712_payload(proxy_account: &Option<AccountIdOf<T>>, nonce: &U256) -> Vec<u8> {
		let domain_separator = Self::evm_domain_separator();

		let payload_hash = if let Some(acc) = proxy_account {
			Self::evm_proxy_set_payload(acc, nonce)
		} else {
			Self::evm_proxy_remove_payload(nonce)
		};

		let mut msg = b"\x19\x01".to_vec();
		msg.extend_from_slice(&domain_separator);
		msg.extend_from_slice(&payload_hash);
		msg
	}

	pub fn storage_key(index: impl Into<u32>) -> Option<Vec<u8>> {
		// string padded to [u8 ;32]
		let key_str = format!("{:064X}", index.into());
		<[u8; 32]>::from_hex(key_str).ok().map(|v| v.to_vec())
	}

	// check whether this account is backing any h160 account
	pub fn acc_is_backing(account: &AccountIdOf<T>) -> Option<H160> {
		ProxyAccount::<T>::iter().find_map(
			|(backed, backer)| {
				if backer == *account {
					Some(backed)
				} else {
					None
				}
			},
		)
	}
}

// NOTICE: this is mostly copy from pallet-ethereum
impl<T: Config> Pallet<T>
where
	BalanceOf<T>: TryFrom<U256> + Into<U256>,
	OriginFor<T>: Into<Result<RawOrigin, OriginFor<T>>>,
	T::AccountId: UncheckedFrom<<T as frame_system::Config>::Hash> + AsRef<[u8]>,
	<BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + TypeInfo + Encode + Debug,
{
	pub fn recover_tx_signer(transaction: &Transaction) -> Option<H160> {
		let (msg, sig) = Self::unpack_transaction(transaction);
		Self::recover_signer(&msg, &sig).and_then(|p| p.to_eth_address().map(H160).ok())
	}

	/// try recover uncompressed signer from a raw payload
	pub fn recover_signer(msg_raw: &[u8], signed_payload_raw: &[u8]) -> Option<ecdsa::Public> {
		let sig = sp_core::ecdsa::Signature::try_from(signed_payload_raw).ok()?;

		let msg = <[u8; 32]>::try_from(msg_raw).ok()?;

		secp256k1_ecdsa_recover_compressed(&sig.0, &msg)
			.ok()
			.map(sp_core::ecdsa::Public::from_raw)
	}

	pub(crate) fn unpack_transaction(transaction: &Transaction) -> ([u8; 32], [u8; 65]) {
		// in addition to typical ECDSA signature, eth tx use chain_id in it's signature to avoid
		// replay attack0

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

	fn verify_proxy_request(
		who: &Option<AccountIdOf<T>>,
		nonce: &U256,
		sig: &[u8],
	) -> Option<H160> {
		let msg = keccak_256(&Self::eip712_payload(who, nonce)[..]);
		Self::recover_signer(&msg, sig).and_then(|pk| pk.to_eth_address().ok().map(H160))
	}

	pub fn storage_at(source: &H160, index: impl Into<u32>) -> Option<H256> {
		let contract_addr = Self::account_from_contract_addr(*source);

		// TODO: properly handle storage key decoding problem
		pallet_contracts::Pallet::<T>::get_storage(
			contract_addr,
			Self::storage_key(index).unwrap_or_default(),
		)
		.ok()
		.flatten()
		.map(|v| <Keccak256 as HashT>::hash(&v[..]))
	}

	/// return try_call result and total fee consumed within this call, excludes base fee and length
	/// fee
	pub fn try_call_or_create(
		from: Option<H160>,
		target: Option<H160>,
		value: BalanceOf<T>,
		gas_limit: u64,
		input: Vec<u8>,
	) -> Result<(BalanceOf<T>, ExecReturnValue), DispatchError> {
		let origin = Self::to_mapped_account(from.unwrap_or_default());

		if let Some(to) = target {
			let dest = Self::account_from_contract_addr(to);

			let allowed_max =
				<<T as Config>::WeightToFee as WeightToFee>::weight_to_fee(&gas_limit);

			let call_result = pallet_contracts::Pallet::<T>::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				Some(allowed_max),
				input,
				true,
			);

			let return_value = call_result.result?;

			let fee_consumed = <<T as Config>::WeightToFee as WeightToFee>::weight_to_fee(
				&call_result.gas_consumed,
			);

			Ok((fee_consumed, return_value))
		} else {
			let (code, selector, salt) = <(Vec<u8>, Vec<u8>, Vec<u8>)>::decode(&mut &input[..])
				.map_err(|_| Error::<T>::InputBufferUndecodable)?;

			let allowed_max =
				<<T as Config>::WeightToFee as WeightToFee>::weight_to_fee(&gas_limit);

			let uploaded_code = pallet_contracts::Pallet::<T>::bare_upload_code(
				origin.clone(),
				code,
				Some(allowed_max),
			)?;

			let reserved = uploaded_code.deposit;

			let create_result = pallet_contracts::Pallet::<T>::bare_instantiate(
				origin,
				value,
				gas_limit,
				Some(allowed_max),
				Code::Existing(uploaded_code.code_hash),
				selector,
				salt,
				true,
			);

			let return_value = create_result.result?.result;
			let fee_consumed = <<T as Config>::WeightToFee as WeightToFee>::weight_to_fee(
				&create_result.gas_consumed,
			);

			// the reserved is also required to make this call successful
			Ok((fee_consumed + reserved, return_value))
		}
	}
}
