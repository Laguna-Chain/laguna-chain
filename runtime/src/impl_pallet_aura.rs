use crate::{impl_pallet_granda::MaxAuthorities, AuraId, Runtime};

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}
