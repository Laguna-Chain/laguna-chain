use super::{laguna_runtime, LagunaRuntimeApi};

pub struct ERC20FeeRunner<'a> {
	api: &'a LagunaRuntimeApi,
}

impl<'a> ERC20FeeRunner<'a> {
	pub fn from_api(api: &'a LagunaRuntimeApi) -> Self {
		Self { api }
	}
}
