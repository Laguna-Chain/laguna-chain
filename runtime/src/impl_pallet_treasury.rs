use frame_support::{parameter_types, traits::ConstU32, PalletId};
use frame_system::EnsureRoot;
use orml_tokens::CurrencyAdapter;
use primitives::AccountId;

use crate::{impl_pallet_currencies::NativeCurrencyId, Event, Runtime};

parameter_types! {

	pub const TreasuryPalletId: PalletId = PalletId(*b"lgn/trsy");
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = CurrencyAdapter<Runtime, NativeCurrencyId>;

	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EnsureRoot<AccountId>;

	type Event = Event;
	type OnSlash = ();
	type ProposalBond = ();
	type ProposalBondMinimum = ();
	type ProposalBondMaximum = ();
	type SpendPeriod = ();
	type Burn = ();
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = ();
	type MaxApprovals = ConstU32<30>;
}
