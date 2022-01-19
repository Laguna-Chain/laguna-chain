use frame_support::dispatch::Weight;

pub trait WeightInfo {
	fn dummy() -> Weight;
}

// Default weight if no type specified
impl WeightInfo for () {
	fn dummy() -> Weight {
		100_000
	}
}
