// TODO: adapt the helper copied from substrate upstream to our internal needs

use crate::service::FullClient;

use laguna_runtime::impl_frame_system::BlockHashCount;

use frame_system::Call as SystemCall;
use sc_cli::Result;
use sc_client_api::BlockBackend;
use sp_core::{Encode, Pair};
use sp_inherents::{InherentData, InherentDataProvider};
use sp_keyring::Sr25519Keyring;
use sp_runtime::{OpaqueExtrinsic, SaturatedConversion};

type SignedPayload =
	sp_runtime::generic::SignedPayload<laguna_runtime::Call, laguna_runtime::SignedExtra>;

use std::{sync::Arc, time::Duration};

/// Generates extrinsics for the `benchmark overhead` command.
///
/// Note: Should only be used for benchmarking.
pub struct RemarkBuilder {
	client: Arc<FullClient>,
}

impl RemarkBuilder {
	/// Creates a new [`Self`] from the given client.
	pub fn new(client: Arc<FullClient>) -> Self {
		Self { client }
	}
}

impl frame_benchmarking_cli::ExtrinsicBuilder for RemarkBuilder {
	fn pallet(&self) -> &str {
		"system"
	}

	fn extrinsic(&self) -> &str {
		"remark"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let acc = Sr25519Keyring::Bob.pair();
		let extrinsic: OpaqueExtrinsic = create_benchmark_extrinsic(
			self.client.as_ref(),
			acc,
			SystemCall::remark { remark: vec![] }.into(),
			nonce,
		)
		.into();

		Ok(extrinsic)
	}
}

/// Create a transaction using the given `call`.
///
/// Note: Should only be used for benchmarking.
pub fn create_benchmark_extrinsic(
	client: &FullClient,
	sender: sp_core::sr25519::Pair,
	call: laguna_runtime::Call,
	nonce: u32,
) -> laguna_runtime::UncheckedExtrinsic {
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;

	let period =
		BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
	let extra: laguna_runtime::SignedExtra = (
		frame_system::CheckNonZeroSender::<laguna_runtime::Runtime>::new(),
		frame_system::CheckSpecVersion::<laguna_runtime::Runtime>::new(),
		frame_system::CheckTxVersion::<laguna_runtime::Runtime>::new(),
		frame_system::CheckGenesis::<laguna_runtime::Runtime>::new(),
		frame_system::CheckEra::<laguna_runtime::Runtime>::from(sp_runtime::generic::Era::mortal(
			period,
			best_block.saturated_into(),
		)),
		frame_system::CheckNonce::<laguna_runtime::Runtime>::from(nonce),
		frame_system::CheckWeight::<laguna_runtime::Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<laguna_runtime::Runtime>::from(0),
	);

	let raw_payload = SignedPayload::from_raw(
		call.clone(),
		extra.clone(),
		(
			(),
			laguna_runtime::VERSION.spec_version,
			laguna_runtime::VERSION.transaction_version,
			genesis_hash,
			best_hash,
			(),
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|e| sender.sign(e));

	laguna_runtime::UncheckedExtrinsic::new_signed(
		call,
		sp_runtime::AccountId32::from(sender.public()).into(),
		primitives::Signature::Sr25519(signature),
		extra,
	)
}

/// Generates inherent data for the `benchmark overhead` command.
///
/// Note: Should only be used for benchmarking.
pub fn inherent_benchmark_data() -> Result<InherentData> {
	let mut inherent_data = InherentData::new();
	let d = Duration::from_millis(0);
	let timestamp = sp_timestamp::InherentDataProvider::new(d.into());

	timestamp
		.provide_inherent_data(&mut inherent_data)
		.map_err(|e| format!("creating inherent data: {:?}", e))?;
	Ok(inherent_data)
}
