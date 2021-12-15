#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;

#[frame_support::pallet]
pub mod pallet {

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Balance;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
    T: Config,
{
    type Balance = todo!();

    type LiquidityInfo = todo!();

    fn withdraw_fee(
        who: &T::AccountId,
        call: &T::Call,
        dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, frame_support::unsigned::TransactionValidityError> {
        todo!()
    }

    fn correct_and_deposit_fee(
        who: &T::AccountId,
        dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<T::Call>,
        post_info: &frame_support::sp_runtime::traits::PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), frame_support::unsigned::TransactionValidityError> {
        todo!()
    }
}
