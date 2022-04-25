#[cfg(test)]
mod tests {
	use crate::ExtBuilder;
	use hydro_runtime::Contracts;

	#[test]
	fn test_deploy_ink() {
		ExtBuilder::default().build().execute_with(|| {
			// TODO: add contracts tests from sample contracts
		});
	}
}
