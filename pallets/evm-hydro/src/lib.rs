#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_evm::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);
}
