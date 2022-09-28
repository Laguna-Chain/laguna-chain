use crate::{
	mock::{Call, Origin, *},
	RawOrigin,
};
use frame_support::{assert_ok, sp_runtime::generic::CheckedExtrinsic, weights::GetDispatchInfo};
use sp_core::H160;

#[test]
fn test_basic() {
	ExtBuilder::default().build().execute_with(|| {
		let eth_acc = H160::from([0; 20]);
	});
}
