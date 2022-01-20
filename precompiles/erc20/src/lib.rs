#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// function selectors of IERC20.sol
#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
	TotalSupply = "totalSupply()",
	BalanceOf = "balanceOf(address account)",
	Transfer = "transfer(address recipient, uint256 amount)",
	Allowance = "allowance(address owner, address spender)",
	Approve = "approve(address spender, uint256 amount)",
	TransferFrom = "transferFrom(address sender, address recipient, uint256 amount)",
}
