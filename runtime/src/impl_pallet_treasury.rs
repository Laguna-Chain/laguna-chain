use frame_support::{parameter_types, traits::ConstU32, PalletId};
use frame_system::EnsureRoot;
use orml_tokens::CurrencyAdapter;
use primitives::{AccountId, Balance, BlockNumber};
use sp_runtime::Permill;

use crate::{constants::HOURS, impl_pallet_currencies::NativeCurrencyId, Event, Runtime};

parameter_types! {

	pub const TreasuryPalletId: PalletId = PalletId(*b"lgn/trsy");
	pub const ProposalBond: Permill = Permill::from_percent(0);

	pub const ProposalBondMinimum: Balance = 1;

	pub const SpendPeriod: BlockNumber = 4 * HOURS;
	pub const Burn: Permill = Permill::from_percent(0);
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = CurrencyAdapter<Runtime, NativeCurrencyId>;

	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EnsureRoot<AccountId>;

	type Event = Event;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
}
