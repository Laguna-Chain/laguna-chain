use crate::{Call, Event, Runtime};
use frame_support::{parameter_types, sp_runtime::KeyTypeId, traits::KeyOwnerProofSystem};
use pallet_grandpa::AuthorityId as GrandpaId;

// borderline aura and grandpa impl from substrate-node-template
parameter_types! {
	pub const MaxAuthorities: u32 = 32;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = ();

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	type HandleEquivocation = ();

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}
