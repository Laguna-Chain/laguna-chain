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
	sp_runtime::{
		app_crypto::ed25519,
		traits::{Hash, StaticLookup},
		MultiSignature, MultiSigner,
	},
	weights::GetDispatchInfo,
};
use orml_traits::arithmetic::Zero;
use primitives::{AccountId, IdentifyAccount, Signature};
use rlp::RlpStream;
use sp_core::{blake2_256, ecdsa, keccak_256, sr25519, Bytes, Pair, H160, H256, U256};
type SignedPayload = frame_support::sp_runtime::generic::SignedPayload<Call, ()>;
use fp_self_contained::{CheckedExtrinsic, SelfContainedCall};
use sp_io::crypto::{secp256k1_ecdsa_recover, secp256k1_ecdsa_recover_compressed};

// NOTICE: many of the underlying construct are taken from pallet-ethereum's test

pub struct LegacyTxMsg(LegacyTransactionMessage);

const RawSeed: [u8; 32] = [0x11; 32];

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

fn dummy_contract_create(
	chain_id: u64,
	blob: Vec<u8>,
	selector: Vec<u8>,
) -> LegacyTransactionMessage {
	let mut input_buf = vec![];

	(selector, blob).encode_to(&mut input_buf);

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

fn dummy_contract_call(target: H160, input: Vec<u8>, chain_id: u64) -> LegacyTransactionMessage {
	LegacyTransactionMessage {
		nonce: Default::default(),
		gas_price: Default::default(),
		gas_limit: U256::from_dec_str("200000000000").unwrap(),
		action: ethereum::TransactionAction::Call(target),
		value: Default::default(),
		chain_id: Some(chain_id),
		input,
	}
}

// test basic ECDSA signing from pubkey generated from ethereum tools
#[test]
fn test_sign() {
	// substrate ecdsa generate pubkey with compress on, so recovered address is in compressed form
	let pair = ecdsa::Pair::from_seed_slice(&RawSeed).unwrap();

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
	let pair = ecdsa::Pair::from_seed_slice(&RawSeed).unwrap();

	let signer = MultiSigner::Ecdsa(pair.public());

	// this is generated from blake2_256 hashed value of the original pub-key in compressed form
	let acc = <MultiSigner as IdentifyAccount>::into_account(signer);

	ExtBuilder::default().build().execute_with(|| {
		let chain_id = 1000;

		let eth_raw_call = dummy_call(H160::from([0; 20]), chain_id);

		let eth_signed =
			LegacyTxMsg(eth_raw_call).sign_with_chain_id(&pair.seed().into(), chain_id);

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
	let pair = ecdsa::Pair::from_seed_slice(&RawSeed).unwrap();

	let signer = MultiSigner::Ecdsa(pair.public());

	// this is generated from blake2_256 hashed value of the original pub-key in compressed form
	let acc = <MultiSigner as IdentifyAccount>::into_account(signer);

	let dev_acc = EvmCompat::to_mapped_account(H160(pair.public().to_eth_address().unwrap()));

	ExtBuilder::default()
		.balances(vec![(dev_acc, 2 << 64)])
		.build()
		.execute_with(|| {
			let chain_id = 1000;

			let blob = std::fs::read(
				"../../runtime/integration-tests/contracts-data/ink/basic/dist/basic.wasm",
			)
			.unwrap();

			let selector = Bytes::from_str("0xed4b9d1b").unwrap();

			let eth_raw_call = dummy_contract_create(chain_id, blob.clone(), selector.to_vec());

			let eth_signed =
				LegacyTxMsg(eth_raw_call).sign_with_chain_id(&pair.seed().into(), chain_id);

			// we expect the signature to come from eth signed payload, signing it on the substrate
			// side will not work
			assert!(EvmCompat::transact(Origin::signed(acc), eth_signed.clone()).is_err());

			let call = crate::Call::<Runtime>::transact { t: eth_signed };
			let info = call.check_self_contained().unwrap().unwrap();

			let (source, origin, _) = &info;

			let eth_addr = pair.public().to_eth_address().map(H160).unwrap();
			assert_eq!(*source, eth_addr);

			assert_ok!(Call::EvmCompat(call).apply_self_contained(info.clone()).unwrap());

			let mapped_origin = EvmCompat::to_mapped_account(*source);

			let codehash = <<Runtime as frame_system::Config>::Hashing as Hash>::hash(&blob[..]);

			// contract address can be compute with the deployer, codehash and the salt
			let addr = pallet_contracts::Pallet::<Runtime>::contract_address(
				&mapped_origin,
				&codehash,
				&[],
			);

			let contract_addr_raw: [u8; 32] = (addr.clone()).into();
			assert!(contract_addr_raw.starts_with(b"evm_contract"));

			let mut contract_addr = [0_u8; 20];
			contract_addr.copy_from_slice(&contract_addr_raw[12..]);

			assert!(!Balances::reserved_balance(addr).is_zero());

			let input = Bytes::from_str("0x633aa551").unwrap();

			let eth_raw_call = dummy_contract_call(H160(contract_addr), input.to_vec(), chain_id);
			let eth_signed =
				LegacyTxMsg(eth_raw_call).sign_with_chain_id(&pair.seed().into(), chain_id);

			let call = crate::Call::<Runtime>::transact { t: eth_signed };
			let info = call.check_self_contained().unwrap().unwrap();

			let (source, origin, _) = &info;

			let eth_addr = pair.public().to_eth_address().map(H160).unwrap();
			assert_eq!(*source, eth_addr);

			assert_ok!(Call::EvmCompat(call).apply_self_contained(info.clone()).unwrap());
		});
}

#[test]
fn test_proxy() {
	let pair = ecdsa::Pair::from_seed_slice(&RawSeed).unwrap();

	let dev_acc = EvmCompat::to_mapped_account(H160(pair.public().to_eth_address().unwrap()));

	ExtBuilder::default()
		.balances(vec![(dev_acc, 2 << 64), (ALICE, 2 << 64)])
		.build()
		.execute_with(|| {
			let source_addr = pair.public().to_eth_address().map(H160).unwrap();

			let source_acc = EvmCompat::to_mapped_account(source_addr);
			assert_eq!(Balances::free_balance(&source_acc), 2 << 64);

			let call = Call::Balances(pallet_balances::Call::transfer { dest: ALICE, value: 1000 });
			assert!(Proxy::proxy(
				Origin::signed(ALICE),
				source_acc.clone(),
				None,
				Box::new(call.clone())
			)
			.is_err());
			assert_ok!(EvmCompat::allow_proxy(source_addr, ALICE));

			assert_ok!(Proxy::proxy(Origin::signed(ALICE), source_acc, None, Box::new(call)));
		});
}

#[test]
fn test_proxy_self_contained() {
	let pair = ecdsa::Pair::from_seed_slice(&RawSeed).unwrap();
	let dev_addr = H160(pair.public().to_eth_address().unwrap());
	let dev_acc = EvmCompat::to_mapped_account(dev_addr);

	ExtBuilder::default()
		.balances(vec![(dev_acc, 2 << 64), (ALICE, 2 << 64)])
		.build()
		.execute_with(|| {
			let payload = EvmCompat::eip712_payload(&ALICE, &Default::default());
			let payload_hash = keccak_256(&payload[..]);

			let sig = pair.sign_prehashed(&payload_hash).0.to_vec();

			let call =
				crate::Call::<Runtime>::set_proxy { nonce: Default::default(), who: ALICE, sig };
			let info = call.check_self_contained().unwrap().unwrap();

			assert_ok!(Call::EvmCompat(call).apply_self_contained(info).unwrap());

			assert_eq!(EvmCompat::has_proxy(dev_addr), Some(ALICE));
		});
}
