use codec::Encode;
use frame_support::dispatch::{DispatchInfo, Weight};

pub fn info_from_weight(w: Weight) -> DispatchInfo {
	// pays_fee: Pays::Yes -- class: DispatchClass::Normal
	DispatchInfo { weight: w, ..Default::default() }
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::*;
	use frame_support::{
		assert_ok,
		dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
		sp_runtime::traits::SignedExtension,
		weights::{DispatchClass, Pays},
	};
	use laguna_runtime::{constants::LAGUNAS, Currencies, FluentFee, Origin, TransactionPayment};
	use pallet_transaction_payment::{ChargeTransactionPayment, OnChargeTransaction};

	#[test]
	fn test_basic_fee_payout() {
		ExtBuilder::default()
			.balances(vec![(ALICE, NATIVE_CURRENCY_ID, 10 * LAGUNAS)])
			.enable_fee_source(vec![(NATIVE_CURRENCY_ID, true)])
			.build()
			.execute_with(|| {
				// prepare a call
				let call = laguna_runtime::Call::Currencies(pallet_currencies::Call::transfer {
					to: BOB,
					currency_id: NATIVE_CURRENCY_ID,
					balance: 1 * LAGUNAS,
				});

				let len = call.encoded_size();
				let info = call.get_dispatch_info();

				let pre_dispatch_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				// pre_dispatch will trigger all of the SignedExtension, and since one of them is
				// consumed by `TransactionPayment --> OnchargeTransaction --> FluentFee`
				// we can test fee chargin logic by calling validate once
				let pre = ChargeTransactionPayment::<Runtime>::from(0)
					.pre_dispatch(&ALICE, &call, &info, len)
					.expect("should pass");

				// calculate actual fee with all the parameter including base_fee, length_fee and
				// byte_multiplier etc.
				let fee = TransactionPayment::compute_actual_fee(
					len as u32,
					&info,
					&PostDispatchInfo { actual_weight: Some(info.weight), pays_fee: Pays::Yes },
					0,
				);

				let post_dispatch_amount = Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID);

				assert_eq!(pre_dispatch_amount, post_dispatch_amount + fee);

				let post =
					call.clone().dispatch(Origin::signed(ALICE)).expect("should be dispatched");

				// TODO: refund logic and payout to validator etc should work
				assert_ok!(ChargeTransactionPayment::<Runtime>::post_dispatch(
					Some(pre),
					&info,
					&post,
					len,
					&Ok(()),
				));

				// expected final states
				assert_eq!(
					Currencies::free_balance(ALICE, NATIVE_CURRENCY_ID),
					10 * LAGUNAS - 1 * LAGUNAS - fee
				);
			});
	}
}
