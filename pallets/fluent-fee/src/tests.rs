//! Unit test for the fluent-fee pallet

use super::*;

use frame_support::{
	assert_ok,
	dispatch::{DispatchInfo, Dispatchable, GetDispatchInfo},
};
use mock::*;
use pallet_transaction_payment::ChargeTransactionPayment;
use sp_runtime::traits::SignedExtension;
