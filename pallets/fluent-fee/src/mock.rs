use super::*;

use frame_support::{
    construct_runtime, parameter_types,
    sp_runtime::traits::{BlakeTwo256, IdentityLookup},
    traits::{Contains, Everything},
    weights::IdentityFee,
};

use orml_currencies::BasicCurrencyAdapter;
use orml_traits::LockIdentifier;
use primitives::{AccountId, Amount, Balance, BlockNumber, CurrencyId, Header, Index, TokenId};
use sp_core::H256;

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

    type AccountData = pallet_balances::AccountData<Balance>;

    type OnNewAccount = ();

    type OnKilledAccount = ();

    type SystemWeightInfo = ();

    type SS58Prefix = ();

    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 2;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type WeightInfo = ();
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
    fn contains(t: &AccountId) -> bool {
        // TODO: all account are possible to be dust-removed now
        false
    }
}

orml_traits::parameter_type_with_key! {
    pub ExistentialDeposits: |currency: CurrencyId| -> Balance {
        Balance::min_value()
    };
}

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
}

pub const NATIVE_CURRENCY_ID: CurrencyId = CurrencyId::NativeToken(TokenId::Hydro);

parameter_types! {
    pub const NativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
}

impl orml_currencies::Config for Runtime {
    type Event = Event;

    type MultiCurrency = Tokens;

    type NativeCurrency = BasicCurrencyAdapter<Self, Balances, Amount, BlockNumber>;

    type GetNativeCurrencyId = NativeCurrencyId;

    type WeightInfo = ();
}

impl Config for Runtime {
    type Event = Event;

    type MultiCurrency = Tokens;

    type NativeCurrencyId = NativeCurrencyId;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = FluentFee;

    type TransactionByteFee = ();

    type OperationalFeeMultiplier = ();

    type WeightToFee = IdentityFee<Balance>;

    type FeeMultiplierUpdate = ();
}

construct_runtime!(

    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Tokens: orml_tokens,
        Balances: pallet_balances,
        Currencies: orml_currencies,
        FluentFee: pallet,
        Payment: pallet_transaction_payment
    }
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const EVA: AccountId = AccountId::new([5u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";

pub struct ExtBuilder {
    balances: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self { balances: vec![] }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        // construct test storage for the mock runtime
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self
                .balances
                .clone()
                .into_iter()
                .filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
                .map(|(account_id, _, initial_balance)| (account_id, initial_balance))
                .collect::<Vec<_>>(),
        }
        .assimilate_storage(&mut t)
        .unwrap();

        orml_tokens::GenesisConfig::<Runtime> {
            balances: self
                .balances
                .into_iter()
                .filter(|(_, currency_id, _)| *currency_id != NATIVE_CURRENCY_ID)
                .collect::<Vec<_>>(),
        }
        .assimilate_storage(&mut t)
        .unwrap();

        t.into()
    }
}