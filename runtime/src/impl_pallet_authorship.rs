use primitives::AccountId;

use crate::Runtime;

pub struct AuraAccountAdapter;
impl frame_support::traits::FindAuthor<AccountId> for AuraAccountAdapter {
	fn find_author<'a, I>(digests: I) -> Option<AccountId>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		pallet_aura::AuraAuthorId::<Runtime>::find_author(digests)
			.and_then(|k| AccountId::try_from(k.as_ref()).ok())
	}
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = AuraAccountAdapter;

	type UncleGenerations = ();

	type FilterUncle = ();

	type EventHandler = ();
}
