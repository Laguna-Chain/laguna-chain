use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	sp_runtime::traits::{BlakeTwo256, IdentityLookup},
	traits::{Contains, Everything, PalletInfo},
	unsigned::TransactionValidityError,
	weights::IdentityFee,
};
use frame_system::Account;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;
use traits::fee::FeeDispatch;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;

	type BlockWeights = ();

	type BlockLength = ();

	type Origin = Origin;

	type Call = Call;

	type Index = Index;

	type BlockNumber = BlockNumber;

	type Hash = H256;

	type Hashing = BlakeTwo256;

	type AccountId = AccountId;

	type Lookup = IdentityLookup<Self::AccountId>;

	type Header = Header;

	type Event = Event;

	type BlockHashCount = BlockHashCount;

	type DbWeight = ();

	type Version = ();

	type PalletInfo = PalletInfo;

	type AccountData = orml_tokens::AccountData<Balance>;

	type OnNewAccount = ();

	type OnKilledAccount = ();

	type SystemWeightInfo = ();

	type SS58Prefix = ();

	type OnSetCode = ();

	type MaxConsumeers = ConstU32<1>;
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
    fn contains(t: AccountId) -> bool {
        // TODO: all accounts are possible to be dust-removed now
        false
    }
}

orml_traits::paramater_type_with_key! {
    pub ExistentialDeposits: |currency: CurrencyId| -> Balance {
        Balance::min_value()
    };
}

pub type ReserveIdentifier = [u8; 8];

impl orml_tokens::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = ();
    type DustRemovalWhitelist = DustRemovalWhitelist;
    type MaxReserves = ConstU32<2>;
    type ReserveIdentifier = ReserveIdentifier;
}

// Configure FluentFee 
pub struct DummyFeeSource;

impl FeeSource for DummyFeeSource {
    type AccountId = AccountId;
    type AssetId = CurrencyId;

    fn accepted(
        who: &Self::AccountId,
        id: &Self::AssetId
    ) -> Result<(), traits::fee::InvalidFeeSource> {
        match id {
            CurrencyId::NativeToken(TokenId::FeeToken | TokenId::Laguna) => Ok(()),
            _ => Err(traits::fee::InvalidFeeSource::Unlisted),
        }
    }

    fn listed(id: &Self::AssetId) -> Result<(), traits::fee::InvalidFeeSource> {
        todo!()
    }
}

pub struct DummyFeeMeasure;

impl FeeMeasure for DummyFeeMeasure {
    type AssetId = CurrencyId;
    type Balance = Balance;

    fn measure(id: &Self::AssetId, balance: Self::Balance) -> Result<Self::Balance, TransactionValidityError> {
        match id {
            CurrencyId::NativeToken(TokenId::Laguna) => Ok(balance),
            // demo 5% reduction
            CurrencyId::NativeToken(TokenId::FeeToken) => Ok(balance.saturating_mul(95).saturating_div(100)),
            _ => Err(InvalidTransaction::Payment.into()),
        }
    }
}

pub struct DummyFeeDispatch<T> {
    _type: PhantomData<T>,
}

impl FeeDispatch<Runtime> for DummyFeeDispatch<Tokens> {
    type AssetId = CurrencyId;
    type Balance = Balance;

    fn withdraw(
            account: &<Runtime as frame_system::Config>::AccountId,
            id: &Self::AssetId,
            balance: &Self::Balance,
            reason: &frame_support::traits::WithdrawReasons,
        ) -> Result<(), traits::fee::InvalidFeeDispatch> {
            
    }
    fn post_info_correction(
            id: &Self::AssetId,
            post_info: &sp_runtime::traits::PostDispatchInfoOf<<Runtime as frame_system::Config>::Call>,
        ) -> Result<(), traits::fee::InvalidFeeDispatch> {
            Ok(())
    }
}




construct_runtime!(
    pub enum Runtime where 
    Block = Block,
    NodeBlock = Block,
    UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system, 
        Balances: pallet_balances,
        Tokens: orml_tokens,
        Scheduler: pallet,
        TransactionPayment: pallet_transaction_payment,
        FluentFee: pallet_fluent_fee,
    }
)
