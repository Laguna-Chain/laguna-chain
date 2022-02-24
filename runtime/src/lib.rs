// assume no_std if build for wasm, which sp-std provides alternative impl
#![cfg_attr(not(feature = "std"), no_std)]
// increate recursive limit for construct_runtime! macro
#![recursion_limit = "256"]

// wasm_binary.rs is provided by build.rs
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, Contains, EqualPrivilegeOnly, KeyOwnerProofSystem},
	weights::{
		constants::{RocksDbWeight, WEIGHT_PER_SECOND},
		IdentityFee,
	},
};

use sp_core::H160;

pub mod precompiles;
use pallet_transaction_payment::CurrencyAdapter;
use precompiles::HydroPrecompiles;

use frame_system::EnsureRoot;
use orml_currencies::BasicCurrencyAdapter;
use pallet_evm::{
	EnsureAddressRoot, EnsureAddressTruncated, HashedAddressMapping, SubstrateBlockHashMapping,
};

use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{
	app_crypto::sp_core::{OpaqueMetadata, U256},
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, NumberFor},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, KeyTypeId, Perbill,
};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// include all needed pallets and their impl below

use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};

use frame_support::weights::Weight;
use primitives::*;

pub mod constants;

// placeholder module to collect WeightInfo provided by runtime-benchmark
mod weights;

// opaque module copied from substrate-node-template, allows cli to sync the network without knowing
// runtime specific formats
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

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

// TODO: include all needed const as well
use constants::*;

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("hydro-runtime-placeholder"),
	impl_name: create_runtime_str!("hydro-runtime-placeholder"),
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

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

// TODO: copied from substrate-node-template for now, plug our pallet impl later
// define const variable for frame_system::Config
parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
	::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
	::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

// TODO: copied from substrate-node-template for now, plug our pallet impl later
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();

	type MaxConsumers = ConstU32<1>;
}

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

// borderline aura and grandpa impl from substrate-node-template
parameter_types! {
	pub const MaxAuthorities: u32 = 32;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = ();

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	type HandleEquivocation = ();

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}

impl pallet_rando::Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;

	// we can either:
	// 1. use SubstrateWeight recomended by substrate on standard hardware
	// 2. use WeightInfo generated by benchmark running on our target hardware
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const SchedulerDelay: Option<BlockNumber> = None;
}

impl pallet_scheduler::Config for Runtime {
	// allow to invoke runtime::Call on behalf of the underlyding pallets
	type Call = Call;
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	// only root account can invoke the scheduler
	type ScheduleOrigin = EnsureRoot<AccountId>;
	// set priviledge required to cancel scheduler
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaximumWeight = MaximumSchedulerWeight;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();

	type PreimageProvider = ();

	type NoPreimagePostponement = SchedulerDelay;
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(t: &AccountId) -> bool {
		// TODO: all account are possible to be dust-removed now
		false
	}
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {

		match currency_id {
			&CurrencyId::NativeToken(token) => {
				match token {
					TokenId::Hydro => MICRO_HYDRO,
					TokenId::FeeToken => MICRO_HYDRO,
				}
			},
			_ => Balance::max_value() // unreachable ED value for unverified currency type
		}
	};
}

// use orml's token to represent both native and other tokens
impl orml_tokens::Config for Runtime {
	type Event = Event;
	// how tokens are measured
	type Balance = Balance;
	type Amount = Amount;

	// how's tokens represented
	type CurrencyId = primitives::CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = DustRemovalWhitelist;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::NativeToken(NATIVE_TOKEN);
}

impl orml_currencies::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Tokens;

	// Native transfer will trigger the underlying mechanism via the underlying `Balances` module
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

	type GetNativeCurrencyId = NativeCurrencyId;
	type WeightInfo = ();
}

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

impl evm_hydro::Config for Runtime {
	type Event = Event;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	// TODO: add benchmark around cross pallet interaction between fee
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_fluent_fee::Config for Runtime {
	type Event = Event;

	type MultiCurrency = Currencies;
	type NativeCurrencyId = NativeCurrencyId;
}

parameter_types! {
	pub Caller: H160 = H160::from_slice(&hex_literal::hex!("37C54011486B797FAA83c5CF6de88C567843a23F"));
	pub TargetAddress: (H160, Vec<u8>) = (H160::from_low_u64_be(9001), precompile_utils::EvmDataWriter::new_with_selector(pallet_rando_precompile::Action::CallRando).build());
}

impl pallet_reverse_evm_call::Config for Runtime {
	type Event = Event;

	type Caller = Caller;

	type TargetAddress = TargetAddress;
}

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
			Balances: pallet_balances ,
			Currencies: orml_currencies,
			Tokens: orml_tokens,
			// weight and fee management
			TransactionPayment: pallet_transaction_payment ,
			FluentFee: pallet_fluent_fee,

			// conseus mechanism
			Aura: pallet_aura ,
			Grandpa: pallet_grandpa ,

			// evm the bytecode execution environment, can preload precompiles
			Evm: pallet_evm,
			EvmHydro: evm_hydro,
			ReverseEvmCall: pallet_reverse_evm_call,

			// dummy pallet for testing interface coupling
			Rando: pallet_rando ,
		}
);

// The following types are copied from substrate-node-template to boostrap development

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The SignedExtension to the basic transaction logic
pub type SignedExtra = (
	// frame_system required once
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

// expose runtime apis, required by node services
// this allow software outside of the wasm blob to access internal functionalities
// often referred to as breaking the "wasm boundary" within substrate ecosystem

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
			list_benchmark!(list, extra, pallet_balances, Balances);

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
			add_benchmark!(params, batches, pallet_balances, Balances);

			// TODO: add pallet-specific bench items below

			if batches.is_empty() {
				return Err("no benchmark items found".into())
			}


			Ok(batches)
		}
	}

}
