use frame_support::assert_ok;
use hydro_runtime::{
	constants::HYDROS, Call, Currencies, Event, Evm, Origin, Runtime, Sudo, System,
};
use orml_traits::MultiCurrency;
use pallet_evm::{AddressMapping, Runner};
use precompile_utils::{Address, EvmDataReader, EvmDataWriter};
use sp_core::{H160, U256};
use std::process;

use super::prepare_smart_contract;
use sp_core::bytes::from_hex;

use crate::{ExtBuilder, ALICE, NATIVE_CURRENCY_ID};

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
enum IERC20Action {
	Name = "name()",
	Symbol = "symbol()",
	Decimals = "decimals()",
	Transfer = "transfer(address,uint256)",
	BalanceOf = "balanceOf(address)",
	TotalSupply = "totalSupply()",
	Allowance = "allowance(address,address)",
	Approve = "approve(address,uint256)",
	TransferFrom = "transferFrom(address,address,uint256)",
}

#[test]
fn native_as_erc20() {
	let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
	let mapped_account =
		<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

	ExtBuilder::default()
		.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
		.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
		.build()
		.execute_with(|| {
			let contract = from_hex(&prepare_smart_contract("NativeERC20", "NativeToken"))
				.expect("unable to parse hex string");

			// prepare the evm call to be submitted by the Sudo pallet
			let evm_call = Call::Evm(pallet_evm::Call::create {
				source: evm_address,
				init: contract,
				value: 0_u64.into(),
				gas_limit: u64::MAX,
				max_fee_per_gas: 0_u64.into(),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list: vec![],
			});

			// sudo should be able to call evm with sudo
			let rs = Sudo::sudo(Origin::signed(mapped_account.clone()), Box::new(evm_call));
			assert_ok!(&rs);

			let events = System::events();
			assert!(events.len() != 0);

			// check if the last evm event is of expected type
			let evm_evts = events
				.iter()
				.map(|record| &record.event)
				.filter_map(
					|event| {
						if let Event::Evm(evm_evt) = event {
							Some(evm_evt)
						} else {
							None
						}
					},
				)
				.collect::<Vec<_>>();

			let evt = evm_evts.last().unwrap();

			match &evt {
				// extract deployed address from pallet_evm::Event::Created
				pallet_evm::Event::Created(deployed_address) => {
					let input = EvmDataWriter::new_with_selector(IERC20Action::Name).build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					let value: Result<precompile_utils::Bytes, _> =
						EvmDataReader::new(&info.value).read();
					assert_ok!(&value);

					let value = value.unwrap();

					assert!(value.as_str().ok().filter(|v| { *v == "HYDRO" }).is_some());

					let input = EvmDataWriter::new_with_selector(IERC20Action::BalanceOf)
						.write(Address(evm_address))
						.build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
					assert_ok!(&value);

					assert!(value
						.ok()
						.filter(|v| {
							*v == U256::from(Currencies::free_balance(
								NATIVE_CURRENCY_ID,
								&mapped_account,
							))
						})
						.is_some());

					let input = EvmDataWriter::new_with_selector(IERC20Action::Decimals).build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					let value: Result<u64, _> = EvmDataReader::new(&info.value).read();
					assert_ok!(&value);

					assert!(value.ok().filter(|v| { *v == 17 }).is_some());

					let input = EvmDataWriter::new_with_selector(IERC20Action::TotalSupply).build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					let value: Result<U256, _> = EvmDataReader::new(&info.value).read();
					assert_ok!(&value);

					assert!(value.ok().filter(|v| { *v == U256::from(1000000) }).is_some());
				},
				_ => {
					panic!("shouldn't be of any other type");
				},
			}
		});
}

#[test]
fn native_as_erc20_transfer() {
	let evm_address_a = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
	let mapped_account_a =
		<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address_a);

	let evm_address_b = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
	let mapped_account_b =
		<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address_b);

	ExtBuilder::default()
		.evm_balances(vec![
			(evm_address_a, NATIVE_CURRENCY_ID, 10 * HYDROS),
			(evm_address_b, NATIVE_CURRENCY_ID, 10 * HYDROS),
		])
		.sudo(mapped_account_a.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
		.build()
		.execute_with(|| {
			let contract = from_hex(&prepare_smart_contract("NativeERC20", "NativeToken"))
				.expect("unable to parse hex string");

			// prepare the evm call to be submitted by the Sudo pallet
			let evm_call = Call::Evm(pallet_evm::Call::create {
				source: evm_address_a,
				init: contract,
				value: 0_u64.into(),
				gas_limit: u64::MAX,
				max_fee_per_gas: 0_u64.into(),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list: vec![],
			});

			// sudo should be able to call evm with sudo
			let rs = Sudo::sudo(Origin::signed(mapped_account_a.clone()), Box::new(evm_call));
			assert_ok!(&rs);

			let events = System::events();
			assert!(events.len() != 0);

			// check if the last evm event is of expected type
			let evm_evts = events
				.iter()
				.map(|record| &record.event)
				.filter_map(
					|event| {
						if let Event::Evm(evm_evt) = event {
							Some(evm_evt)
						} else {
							None
						}
					},
				)
				.collect::<Vec<_>>();

			let evt = evm_evts.last().unwrap();

			match &evt {
				// extract deployed address from pallet_evm::Event::Created
				pallet_evm::Event::Created(deployed_address) => {
					let input = EvmDataWriter::new_with_selector(IERC20Action::Transfer)
						.write(Address(evm_address_b))
						.write(U256::from(100_u64))
						.build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address_a,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					assert!(EvmDataReader::new(&info.value).read::<bool>().unwrap_or(false));

					assert_eq!(
						10 * HYDROS - 100,
						Currencies::free_balance(NATIVE_CURRENCY_ID, &mapped_account_a)
					);

					assert_eq!(
						10 * HYDROS + 100,
						Currencies::free_balance(NATIVE_CURRENCY_ID, &mapped_account_b)
					)
				},
				_ => {
					panic!("shouldn't be of any other type");
				},
			}
		});
}

#[test]
fn native_as_erc20_total_supply() {
	let evm_address = H160([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
	let mapped_account =
		<Runtime as pallet_evm::Config>::AddressMapping::into_account_id(evm_address);

	ExtBuilder::default()
		.evm_balances(vec![(evm_address, NATIVE_CURRENCY_ID, 10 * HYDROS)])
		.sudo(mapped_account.clone()) // TODO: currently we're temoprarily only allowing Sudo account for evm call
		.build()
		.execute_with(|| {
			let contract = from_hex(&prepare_smart_contract("NativeERC20", "NativeToken"))
				.expect("unable to parse hex string");

			// prepare the evm call to be submitted by the Sudo pallet
			let evm_call = Call::Evm(pallet_evm::Call::create {
				source: evm_address,
				init: contract,
				value: 0_u64.into(),
				gas_limit: u64::MAX,
				max_fee_per_gas: 0_u64.into(),
				max_priority_fee_per_gas: None,
				nonce: None,
				access_list: vec![],
			});

			// sudo should be able to call evm with sudo
			let rs = Sudo::sudo(Origin::signed(mapped_account.clone()), Box::new(evm_call));
			assert_ok!(&rs);

			let events = System::events();
			assert!(events.len() != 0);

			// check if the last evm event is of expected type
			let evm_evts = events
				.iter()
				.map(|record| &record.event)
				.filter_map(
					|event| {
						if let Event::Evm(evm_evt) = event {
							Some(evm_evt)
						} else {
							None
						}
					},
				)
				.collect::<Vec<_>>();

			let evt = evm_evts.last().unwrap();

			match &evt {
				// extract deployed address from pallet_evm::Event::Created
				pallet_evm::Event::Created(deployed_address) => {
					let input = EvmDataWriter::new_with_selector(IERC20Action::TotalSupply).build();
					// raw evm execution using T::Runner so we can inspect output
					let rs = <Runtime as pallet_evm::Config>::Runner::call(
						evm_address,
						*deployed_address,
						input,
						0_u64.into(),
						u64::MAX,
						None,
						None,
						None,
						vec![],
						<Runtime as pallet_evm::Config>::config(),
					);

					assert_ok!(&rs);
					let info = rs.unwrap();

					// extract evm output value from raw bytes
					let rs = EvmDataReader::new(&info.value).read::<U256>();
					assert_ok!(&rs);

					assert_eq!(
						rs.unwrap(),
						pallet_balances::Pallet::<Runtime>::total_issuance().into()
					);
				},
				_ => {
					panic!("shouldn't be of any other type");
				},
			}
		});
}
