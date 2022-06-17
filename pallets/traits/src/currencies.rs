use frame_support::dispatch::DispatchResultWithPostInfo;
use sp_core::U256;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

// TODO: distinguish between erc20 token and ink based contracts
/// interface to provide access to erc20 based token contract
pub trait TokenAccess<T: frame_system::Config> {
	type Balance;

	fn total_supply(asset_address: AccountIdOf<T>) -> Option<Self::Balance>;

	fn balance_of(asset_address: AccountIdOf<T>, who: AccountIdOf<T>) -> Option<Self::Balance>;

	fn transfer(
		asset_address: AccountIdOf<T>,
		who: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;

	fn allowance(
		asset_address: AccountIdOf<T>,
		owner: AccountIdOf<T>,
		spender: AccountIdOf<T>,
	) -> Option<Self::Balance>;

	fn approve(
		asset_address: AccountIdOf<T>,
		owner: AccountIdOf<T>,
		spender: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;

	fn transfer_from(
		asset_address: AccountIdOf<T>,
		who: AccountIdOf<T>,
		from: AccountIdOf<T>,
		to: AccountIdOf<T>,
		amount: U256,
	) -> DispatchResultWithPostInfo;
}
