#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::WithdrawReasons};
use orml_traits::{arithmetic::Zero, MultiCurrency};
pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;

mod mock;
mod tests;

/// # fluent fee
///
/// this modules customize and replace the how fee is charged for a given transaction
#[frame_support::pallet]
pub mod pallet {

    use frame_support::pallet_prelude::*;
    use frame_system::{ensure_root, ensure_signed, pallet_prelude::OriginFor};
    use orml_traits::MultiCurrency;
    use primitives::CurrencyId;
    use scale_info::TypeInfo;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

        #[pallet::constant]
        type NativeCurrencyId: Get<CurrencyId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FeeSourceAdded((CurrencyId, FeeRatePoint)),
        FeeSourceRemoved((CurrencyId, FeeRatePoint)),
        PreferenceSet(CurrencyId),
        PreferenceUnset,
    }

    #[pallet::error]
    pub enum Error<T> {
        DuplicateCurrencyEntry,
        IllegalFeeRate,
        UnsetBlankPreference,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// customize reduction fee when paid via the specified token
    #[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq)]
    pub struct FeeRatePoint {
        pub base: i32,
        pub point: i32,
    }

    #[pallet::storage]
    #[pallet::getter(fn get_fee_source)]
    pub type FeeSource<T: Config> = StorageMap<_, Blake2_128Concat, CurrencyId, FeeRatePoint>;

    pub(super) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    #[pallet::storage]
    #[pallet::getter(fn get_fee_preference)]
    pub type FeePreference<T: Config> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, CurrencyId>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1000_000)]
        pub fn add_currency_to_fee_source(
            origin: OriginFor<T>,
            currency: CurrencyId,
            fee_rate: FeeRatePoint,
        ) -> DispatchResult {
            // only root is allowed to add token to be fee-source
            let _ = ensure_root(origin)?;

            let target_currency = Pallet::<T>::get_fee_source(&currency);

            ensure!(
                target_currency.is_none(),
                Error::<T>::DuplicateCurrencyEntry
            );

            ensure!(
                fee_rate.base >= fee_rate.point && fee_rate.point != 0 && fee_rate.base != 0,
                Error::<T>::IllegalFeeRate
            );

            FeeSource::<T>::insert(&currency, &fee_rate);
            Pallet::<T>::deposit_event(Event::<T>::FeeSourceAdded((currency, fee_rate)));

            Ok(())
        }

        #[pallet::weight(1000_000)]
        pub fn set_fee_preference(origin: OriginFor<T>, currency: CurrencyId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            FeePreference::<T>::insert(who, &currency);
            Pallet::<T>::deposit_event(Event::<T>::PreferenceSet(currency));

            Ok(())
        }

        #[pallet::weight(1000_000)]
        pub fn unset_fee_preference(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let old = Pallet::<T>::get_fee_preference(&who);

            ensure!(old.is_some(), Error::<T>::UnsetBlankPreference);

            FeePreference::<T>::remove(who);
            Pallet::<T>::deposit_event(Event::<T>::PreferenceUnset);

            Ok(())
        }
    }
}

impl<T> OnChargeTransaction<T> for Pallet<T>
where
    T: Config,
    T: pallet_transaction_payment::Config,
{
    type Balance =
        <T::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

    // TODO: deal with correct liquidity info logic
    type LiquidityInfo = ();

    fn withdraw_fee(
        who: &T::AccountId,
        call: &T::Call,
        dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, frame_support::unsigned::TransactionValidityError> {
        let withdraw_reason = if tip.is_zero() {
            WithdrawReasons::TRANSACTION_PAYMENT
        } else {
            WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
        };

        match <T as Config>::MultiCurrency::withdraw(
            <T as Config>::NativeCurrencyId::get(),
            who,
            fee,
        ) {
            Ok(_) => {
                log::info!(target: "fee withdrawn", "succsefully withdrawn using native_currency");
                Ok(())
            }
            Err(_) => Err(InvalidTransaction::Payment.into()),
        }

        // TODO: pay fee with user preferred currency
        // get preference currency used to pay fee for a given account
        // let account_pref = Pallet::<T>::get_fee_preference(who);

        // pay fee with platform currency if not specified
        // if account_pref.is_none() {}
    }

    fn correct_and_deposit_fee(
        who: &T::AccountId,
        dispatch_info: &frame_support::sp_runtime::traits::DispatchInfoOf<T::Call>,
        post_info: &frame_support::sp_runtime::traits::PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), frame_support::unsigned::TransactionValidityError> {
        // TODO: execute refund plan from already_withdrawn

        log::info!(target: "fee correction", "deposit without refund");

        Ok(())
    }
}
