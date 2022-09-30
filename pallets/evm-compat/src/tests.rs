use core::str::FromStr;

use crate::{
	mock::{Call, ChainId, Origin, *},
	RawOrigin, Transaction,
};
use codec::Encode;
use ethereum::{
	LegacyTransaction, LegacyTransactionMessage, TransactionAction, TransactionSignature,
};
use frame_support::{
	assert_ok,
	crypto::ecdsa::ECDSAExt,
	sp_runtime::{MultiSignature, MultiSigner},
	weights::GetDispatchInfo,
};
use primitives::{AccountId, IdentifyAccount, Signature};
use rlp::RlpStream;
use sp_core::{blake2_256, ecdsa, keccak_256, Bytes, Pair, H160, H256, U256};
type SignedPayload = frame_support::sp_runtime::generic::SignedPayload<Call, ()>;
use fp_self_contained::{CheckedExtrinsic, SelfContainedCall};
use sp_io::crypto::{secp256k1_ecdsa_recover, secp256k1_ecdsa_recover_compressed};

// NOTICE: many of the underlying construct are taken from pallet-ethereum's test

pub struct LegacyTxMsg(LegacyTransactionMessage);

impl LegacyTxMsg {
	pub fn sign(&self, key: &H256) -> Transaction {
		self.sign_with_chain_id(key, self.0.chain_id.unwrap_or_else(ChainId::get))
	}

	pub fn sign_with_chain_id(&self, private_key: &H256, chain_id: u64) -> Transaction {
		// prepare the unsigned msg as keccak_256 hashed

		let pair = ecdsa::Pair::from_seed(&private_key.0);
		let hash = self.0.hash();

		let s = pair.sign_prehashed(&hash.0);

		let sig = &s.0[0..64];

		// recovery_id is the last byte of the signature
		let recid = &s.0[64];

		let sig = TransactionSignature::new(
			*recid as u64 % 2 + chain_id * 2 + 35,
			H256::from_slice(&sig[0..32]),
			H256::from_slice(&sig[32..64]),
		)
		.unwrap();

		Transaction::Legacy(ethereum::LegacyTransaction {
			nonce: self.0.nonce,
			gas_price: self.0.gas_price,
			gas_limit: self.0.gas_limit,
			action: self.0.action,
			value: self.0.value,
			input: self.0.input.clone(),
			signature: sig,
		})
	}
}

fn dummy_call(target: H160, chain_id: u64) -> LegacyTransactionMessage {
	LegacyTransactionMessage {
		nonce: Default::default(),
		gas_price: Default::default(),
		gas_limit: U256::MAX,
		action: ethereum::TransactionAction::Call(target),
		value: Default::default(),
		chain_id: Some(chain_id),
		input: vec![],
	}
}

fn dummy_contract_call(chain_id: u64) -> LegacyTransactionMessage {
	let codehash =
		std::fs::read("../../runtime/integration-tests/contracts-data/ink/basic/dist/basic.wasm")
			.unwrap();

	let selector = Bytes::from_str("0xed4b9d1b").unwrap();

	let mut input_buf = vec![];

	(selector, codehash).encode_to(&mut input_buf);

	LegacyTransactionMessage {
		nonce: Default::default(),
		gas_price: Default::default(),
		gas_limit: U256::from_dec_str("200000000000").unwrap(),
		action: ethereum::TransactionAction::Create,
		value: Default::default(),
		chain_id: Some(chain_id),
		input: input_buf,
	}
}

// test basic ECDSA signing from pubkey generated from ethereum tools
#[test]
fn test_sign() {
	let private_key = libsecp256k1::SecretKey::parse(&[
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11,
	])
	.unwrap();

	// substrate ecdsa generate pubkey with compress on, so recovered address is in compressed form
	let pair = ecdsa::Pair::from_seed_slice(&private_key.serialize()).unwrap();

	let msg = b"hello";
	let hashed_payload = keccak_256(msg);

	// sign raw payload without hashing it with blake2_256
	let sig = pair.sign_prehashed(&hashed_payload);

	let recovered = secp256k1_ecdsa_recover(&sig.0, &hashed_payload).ok().unwrap();

	// eth tool generated pubkey are in full form
	assert_eq!(ecdsa::Public::from_full(&recovered[..]).unwrap(), pair.public());
}

#[test]
fn test_basic() {
	let private_key = libsecp256k1::SecretKey::parse(&[
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11,
	])
	.unwrap();

	let pair = ecdsa::Pair::from_seed_slice(&private_key.serialize()).unwrap();

	let signer = MultiSigner::Ecdsa(pair.public());

	// this is generated from blake2_256 hashed value of the original pub-key in compressed form
	let acc = <MultiSigner as IdentifyAccount>::into_account(signer);

	ExtBuilder::default().build().execute_with(|| {
		let chain_id = 1000;

		let eth_raw_call = dummy_call(H160::from([0; 20]), chain_id);

		let eth_signed =
			LegacyTxMsg(eth_raw_call).sign_with_chain_id(&private_key.serialize().into(), chain_id);

		// we expect the signature to come from eth signed payload, signing it on the substrate side
		// will not work
		assert!(EvmCompat::transact(Origin::signed(acc), eth_signed.clone()).is_err());

		let call = crate::Call::<Runtime>::transact { t: eth_signed };
		let info = call.check_self_contained().unwrap().unwrap();

		let (source, origin, _) = &info;

		let eth_addr = pair.public().to_eth_address().map(H160).unwrap();
		assert_eq!(*source, eth_addr);
	});
}

#[test]
fn test_create() {
	let raw_seed = [
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
		0x11, 0x11,
	];

	let private_key = libsecp256k1::SecretKey::parse(&raw_seed).unwrap();

	let pair = ecdsa::Pair::from_seed_slice(&private_key.serialize()).unwrap();

	let signer = MultiSigner::Ecdsa(pair.public());

	// this is generated from blake2_256 hashed value of the original pub-key in compressed form
	let acc = <MultiSigner as IdentifyAccount>::into_account(signer);

	let dev_acc = EvmCompat::source_lookup(H160(pair.public().to_eth_address().unwrap())).unwrap();

	ExtBuilder::default()
		.balances(vec![(dev_acc, 2 << 64)])
		.build()
		.execute_with(|| {
			let chain_id = 1000;

			let eth_raw_call = dummy_contract_call(chain_id);

			let eth_signed = LegacyTxMsg(eth_raw_call)
				.sign_with_chain_id(&private_key.serialize().into(), chain_id);

			// we expect the signature to come from eth signed payload, signing it on the substrate
			// side will not work
			assert!(EvmCompat::transact(Origin::signed(acc), eth_signed.clone()).is_err());

			let call = crate::Call::<Runtime>::transact { t: eth_signed };
			let info = call.check_self_contained().unwrap().unwrap();

			let (source, origin, _) = &info;

			let eth_addr = pair.public().to_eth_address().map(H160).unwrap();
			assert_eq!(*source, eth_addr);

			assert_ok!(Call::EvmCompat(call).apply_self_contained(info).unwrap());
		});
}
