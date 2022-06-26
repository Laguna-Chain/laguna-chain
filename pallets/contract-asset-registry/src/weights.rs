use frame_support::weights::Weight;

pub trait WeightInfo {
	fn register_asset() -> Weight;

	fn unregister_asset() -> Weight;

	fn suspend_asset() -> Weight;
}

impl WeightInfo for () {
	fn register_asset() -> Weight {
		1000_u64 as _
	}

	fn unregister_asset() -> Weight {
		1000_u64 as _
	}

	fn suspend_asset() -> Weight {
		1000_u64 as _
	}
}
