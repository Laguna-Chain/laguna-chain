use crate::{impl_frame_system::BlockWeights, Call, Event, Origin, OriginCaller, Runtime, Weight};
use frame_support::{parameter_types, sp_runtime::Perbill, traits::EqualPrivilegeOnly};
use frame_system::EnsureRoot;
use primitives::{AccountId, BlockNumber};

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const SchedulerDelay: Option<BlockNumber> = None;
}

impl pallet_scheduler::Config for Runtime {
	// allow to invoke runtime::Call on behalf of the underlyding pallets
	type Call = Call;
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	// only root account can invoke the scheduler
	type ScheduleOrigin = EnsureRoot<AccountId>;
	// set priviledge required to cancel scheduler
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaximumWeight = MaximumSchedulerWeight;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = ();

	type PreimageProvider = ();

	type NoPreimagePostponement = SchedulerDelay;
}
