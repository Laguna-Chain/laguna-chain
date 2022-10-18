// assume no_std if build for wasm, which sp-std provides alternative impl
#![cfg_attr(not(feature = "std"), no_std)]
// increate recursive limit for construct_runtime! macro
#![recursion_limit = "256"]

// wasm_binary.rs is provided by build.rs
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode};
use constants::LAGUNA_NATIVE_CURRENCY;
use frame_support::{
	self, construct_runtime,
	dispatch::{Dispatchable, GetDispatchInfo},
	pallet_prelude::TransactionValidityError,
	sp_runtime::{
		app_crypto::sp_core::OpaqueMetadata,
		create_runtime_str, generic, impl_opaque_keys,
		traits::{
			BlakeTwo256, Block as BlockT, DispatchInfoOf, NumberFor, PostDispatchInfoOf,
			SignedExtension,
		},
		transaction_validity::{TransactionSource, TransactionValidity},
		ApplyExtrinsicResult, KeyTypeId, SaturatedConversion,
	},
	traits::{FindAuthor, Get},
};
use impl_frame_system::BlockHashCount;
use pallet_contracts_primitives::{ExecReturnValue, ReturnFlags};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{Bytes, H160, H256, U256};
use sp_runtime::{traits::UniqueSaturatedInto, DispatchError};

use ethereum::TransactionV2;
use frame_support::sp_std::prelude::*;
use scale_info::prelude::format;

pub mod contract_extensions;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};

use frame_support::weights::Weight;
use primitives::{
	AccountId, Address, Balance, BlockNumber, CurrencyId, Hash, Header, Index, Signature,
};

use pallet_evm_compat_rpc_runtime_api::ConesensusDigest;

// include all needed pallets and their impl below
// we put palelt implementation code in a separate module to enhahce readability
pub mod impl_frame_system;
pub mod impl_orml_tokens;
pub mod impl_pallet_aura;
pub mod impl_pallet_contract_asset_registry;
pub mod impl_pallet_contracts;
pub mod impl_pallet_fee_measurement;
pub mod impl_pallet_treasury;

pub mod impl_pallet_authorship;
pub mod impl_pallet_currencies;
pub mod impl_pallet_evm_compat;
pub mod impl_pallet_fee_enablement;
pub mod impl_pallet_fluent_fee;
pub mod impl_pallet_granda;
pub mod impl_pallet_prepaid;
pub mod impl_pallet_proxy;
pub mod impl_pallet_scheduler;
pub mod impl_pallet_sudo;
pub mod impl_pallet_system_contract_deployer;
pub mod impl_pallet_timestamp;
pub mod impl_pallet_transaction_payment;

use impl_pallet_authorship::AuraAccountAdapter;
use impl_pallet_evm_compat::{ETH_ACC_PREFIX, ETH_CONTRACT_PREFIX};

pub mod constants;

// placeholder module to collect WeightInfo provided by runtime-benchmark
mod weights;

// opaque module copied from substrate-node-template, allows cli to sync the network without knowing
// runtime specific formats
pub mod opaque {
	use super::*;

	pub use frame_support::sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("laguna-runtime-placeholder"),
	impl_name: create_runtime_str!("laguna-runtime-placeholder"),
	authoring_version: 1,
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

// version declaration for native runtime
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

use frame_support::sp_runtime;

// runtime as enum, can cross reference enum variants as pallet impl type associates
// this macro also mixed type to all pallets so that they can adapt through a shared type
// be cautious that compile error arise if the pallet and construct_runtime can't be build at the
// same time, most of the time they cross reference each other
construct_runtime!(
	pub enum Runtime
	where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
		{
			// import needed part of the pallet
			// NOTICE: will effect life cycle of a pallet

			// system pallets
			System: frame_system ,
			Timestamp: pallet_timestamp ,
			Sudo: pallet_sudo ,
			Scheduler: pallet_scheduler ,

			// token and currency
			Tokens: orml_tokens,
			Currencies: pallet_currencies,
			ContractAssetsRegistry: pallet_contract_asset_registry,

			// weight and fee management
			TransactionPayment: pallet_transaction_payment ,
			FluentFee: pallet_fluent_fee,
			FeeEnablement: pallet_fee_enablement,
			FeeMeasurement: pallet_fee_measurement,
			PrepaidFee: pallet_prepaid,

			// conseus mechanism
			Aura: pallet_aura ,
			Grandpa: pallet_grandpa ,
			Authorship: pallet_authorship,

			// government
			Treasury: pallet_treasury,

			Contracts: pallet_contracts,
			SystemContractDeployer: pallet_system_contract_deployer,
			RandomnessCollectiveFlip: pallet_randomness_collective_flip,
			EvmCompat: pallet_evm_compat,
			Proxy: pallet_proxy,
		}
);

// The following types are copied from substrate-node-template to boostrap development

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

// pub type CheckedExtrinsic = <UncheckedExtrinsic as Checkable>::Checked;
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, Call, SignedExtra, H160>;

pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The SignedExtension to the basic transaction logic
pub type SignedExtra = (
	// frame_system required once
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	// fee and tipping related
	// TODO: justify whether we need to include if "feeless" transaction is included
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Executive: handles dispatch to the various modules -> virtual dispatch caller
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// create default extra for tx request coming from eth rpc
fn new_extra(nonce: Index, tip: Balance) -> SignedExtra {
	// TODO: allow eth-client to contain custom Era
	let period =
		BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

	let current_block = System::block_number()
		.saturated_into::<u64>()
		// The `System::block_number` is initialized with `n+1`,
		// so the actual block number is `n`.
		.saturating_sub(1);

	(
		frame_system::CheckNonZeroSender::<Runtime>::new(),
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
		frame_system::CheckNonce::<Runtime>::from(nonce),
		frame_system::CheckWeight::<Runtime>::new(),
		// fee and tipping related
		// TODO: justify whether we need to include if "feeless" transaction is included
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
	)
}

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = (H160, AccountId, SignedExtra);

	fn is_self_contained(&self) -> bool {
		match self {
			Call::EvmCompat(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		if let Call::EvmCompat(call) = self {
			if let Some(Ok((source, origin, (nonce, tip)))) = call.check_self_contained() {
				let extra = new_extra(nonce.as_u32(), tip.as_u128());

				return Some(Ok((source, origin, extra)))
			}
		}

		None
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<TransactionValidity> {
		if let Call::EvmCompat(call) = self {
			let (source, origin, extra) = info;
			return Some(extra.validate(origin, self, dispatch_info, len))
		}

		None
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		if let Call::EvmCompat(call) = self {
			let (source, origin, extra) = info;
			return Some(extra.clone().pre_dispatch(origin, self, dispatch_info, len).map(|_| ()))
		}

		None
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ Call::EvmCompat(pallet_evm_compat::Call::transact { .. }) =>
				Some(call.dispatch(Origin::from(
					pallet_evm_compat::RawOrigin::EthereumTransaction(info.0),
				))),
			_ => None,
		}
	}
}

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, t: ethereum::TransactionV2) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(pallet_evm_compat::Call::<Runtime>::transact { t }.into())
	}
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, t: ethereum::TransactionV2) -> opaque::UncheckedExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			pallet_evm_compat::Call::<Runtime>::transact { t }.into(),
		);
		let encoded = extrinsic.encode();
		opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

// expose runtime apis, required by node services
// this allow software outside of the wasm blob to access internal functionalities
// often referred to as breaking the "wasm boundary" within substrate ecosystem

const CONTRACTS_DEBUG_OUTPUT: bool = true;

// TODO: common api impl are derived from substrate-node-template, add custom runtime-api later
impl_runtime_apis! {

	// provide generic api required by the node client
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}


	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}


	// consensus related api below
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}


	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}


		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>
		for Runtime
	{

		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: u64,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts_primitives::ContractExecResult<Balance> {
			Contracts::bare_call(origin, dest, value, gas_limit, storage_deposit_limit, input_data, CONTRACTS_DEBUG_OUTPUT)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: u64,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts_primitives::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance>
		{
			Contracts::bare_instantiate(origin, value, gas_limit, storage_deposit_limit, code, data, salt, CONTRACTS_DEBUG_OUTPUT)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts_primitives::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	impl pallet_currencies_rpc_runtime_api::CurrenciesApi<Block, AccountId, Balance> for Runtime {
		fn list_assets() -> Vec<CurrencyId> {
			ContractAssetsRegistry::enabled_assets().iter().map(|v| CurrencyId::Erc20(*v.as_ref())).collect::<_>()
		}

		fn free_balance(account: AccountId, asset: CurrencyId) -> Option<Balance> {
			Some(Currencies::free_balance(account, asset))
		}

		fn total_balance(account: AccountId, asset: CurrencyId) -> Option<Balance> {
			Some(Currencies::total_balance(account, asset))
		}
	}


	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {

		fn convert_transaction(t: ethereum::TransactionV2) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_evm_compat::Call::<Runtime>::transact { t }.into(),
			)
		}
	}


	impl pallet_evm_compat_rpc_runtime_api::EvmCompatApi<Block, AccountId, Balance> for Runtime {

		fn source_to_mapped_address(source: H160) -> AccountId {
			EvmCompat::to_mapped_account(source)
		}

		fn source_is_backed_by(source: H160) -> Option<AccountId>{
			EvmCompat::has_proxy(source)
		}

		fn check_contract_is_evm_compat(contract_addr: AccountId) -> Option<H160>{
			let addr_raw = <[u8; 32]>::from(contract_addr);
			if addr_raw.starts_with(ETH_CONTRACT_PREFIX)
			 {
				let source = H160::from_slice(&addr_raw[12..]);
				Some(source)

			 } else {
				None
			 }
		}

		fn chain_id() -> u64 {
			<Runtime as pallet_evm_compat::Config>::ChainId::get()
		}

		fn balances(address: H160) -> U256 {
			let addr = EvmCompat::to_mapped_account(address);
			Currencies::free_balance(addr, LAGUNA_NATIVE_CURRENCY).into()
		}


		fn block_hash(number: u32) -> H256 {
			H256::from_slice(System::block_hash(number).as_ref())
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			EvmCompat::storage_at(&address, index.as_u32()).unwrap_or_default()
		}

		fn account_nonce(address: H160) -> U256 {
			let addr = EvmCompat::to_mapped_account(address);
			let nonce = System::account_nonce(&addr);
			U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(nonce))
		}

		fn call(from: Option<H160>, target: Option<H160>, value: Balance, input: Vec<u8>, gas_limit: u64) -> Result<(Balance, ExecReturnValue), DispatchError> {

			if target.is_some() && input.is_empty() {
				let to = EvmCompat::to_mapped_account(target.unwrap_or_default());
				let call = pallet_currencies::Call::<Runtime>::transfer{to, currency_id: LAGUNA_NATIVE_CURRENCY, balance: value};
				let info = call.get_dispatch_info();
				let len = call.encode().len();
				let final_fee = TransactionPayment::compute_fee(len as _, &info, 0);

				let rv = ExecReturnValue {
					data: Bytes::from(vec![]),
					flags: ReturnFlags::empty(),
				};

				Ok((final_fee, rv))

			} else {
				EvmCompat::try_call_or_create(from, target, value,  gas_limit, input)
			}
		}

		fn author(digests: Vec<ConesensusDigest>) -> Option<H160>{

			// find author using all consensus digests
			AuraAccountAdapter::find_author(digests.iter().map(|(a, b)| {
				(*a, &b[..])
			})).and_then(|author| {
				// find the h160 address that the author account is backing
				EvmCompat::acc_is_backing(&author)
			})
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<TransactionV2>{

			xts.into_iter().filter_map(|xt|{
				if let Call::EvmCompat(pallet_evm_compat::Call::transact {t}) = xt.0.function { Some(t)} else {
					None
				}
			}).collect()


		}
	}

	// TODO: add other needed runtime-api

	// expose pallet defined rpc below

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();

			// include system-level benchmarks
			list_benchmark!(list, extra, frame_benchmarking, BaselineBench::<Runtime>);
			list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);

			// include pallet benchmarks

			// TODO: add all benchmarks defined by pallets


			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			// escape params below
			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			// system level bench items
			add_benchmark!(params, batches, frame_benchmarking, BaselineBench::<Runtime>);
			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);

			// pallet-specific bench items

			// TODO: add pallet-specific bench items below

			if batches.is_empty() {
				return Err("no benchmark items found".into())
			}
			Ok(batches)
		}
	}

}
