use frame_support::weights::Weight;

pub trait WeightInfo {
	fn onboard_asset() -> Weight;

	fn enable_asset() -> Weight;

	fn disable_asset() -> Weight;

	fn suspend_asset() -> Weight;
}

impl WeightInfo for () {
	fn enable_asset() -> Weight {
		1000_u64
	}

	fn disable_asset() -> Weight {
		1000_u64
	}

	fn suspend_asset() -> Weight {
		1000_u64
	}

	fn onboard_asset() -> Weight {
		1000_u64
	}
}
