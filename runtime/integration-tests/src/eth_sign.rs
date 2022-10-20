#![cfg(test)]

use fp_self_contained::SelfContainedCall;
use frame_support::crypto::ecdsa::ECDSAExt;
use laguna_runtime::SignedPayload;
use primitives::Signature;
use sp_core::{ecdsa, Pair};

#[test]
fn test_raw_extrinsic() {
	// let (pair, s, seed) = ecdsa::Pair::generate_with_phrase(None);
	// let eth_addr = pair.public().to_eth_address();

	// let call = laguna_runtime::Call::EvmCompat(pallet_evm_compat::Call::transact { t: todo!() });
	// let raw_payload = SignedPayload::new(call, (0));
}
