// Test environment for the Economic Security pallet.

use super::*;
use crate as economic_security;

use frame_support::{
  construct_runtime, derive_impl,
  traits::{ConstU16, ConstU32, ConstU64},
};

use sp_core::sr25519::Public;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use serai_primitives::{Coin, Balance, Amount, system_address};

pub use coins_pallet as coins;
pub use dex_pallet as dex;

type Block = frame_system::mocking::MockBlock<Test>;

pub const MEDIAN_PRICE_WINDOW_LENGTH: u16 = 10;

construct_runtime!(
  pub enum Test
  {
    System: frame_system,
    CoinsPallet: coins,
    LiquidityTokens: coins::<Instance1>::{Pallet, Call, Storage, Event<T>},
    Dex: dex,
    EconomicSecurity: economic_security,
  }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
  type AccountId = Public;
  type Lookup = IdentityLookup<Self::AccountId>;
  type Block = Block;
}

impl coins::Config for Test {
  type AllowMint = ();
}

impl coins::Config<coins::Instance1> for Test {
  type AllowMint = ();
}

impl dex::Config for Test {
  type WeightInfo = ();
  type LPFee = ConstU32<3>; // means 0.3%
  type MaxSwapPathLength = ConstU32<4>;

  type MedianPriceWindowLength = ConstU16<{ MEDIAN_PRICE_WINDOW_LENGTH }>;

  // 100 is good enough when the main currency has 12 decimals.
  type MintMinLiquidity = ConstU64<100>;
}

impl economic_security::Config for Test {}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

  let accounts: Vec<Public> = vec![
    system_address(b"account1").into(),
    system_address(b"account2").into(),
    system_address(b"account3").into(),
    system_address(b"account4").into(),
  ];
  coins::GenesisConfig::<Test> {
    accounts: accounts
      .into_iter()
      .map(|a| (a, Balance { coin: Coin::Serai, amount: Amount(1 << 60) }))
      .collect(),
    _ignore: Default::default(),
  }
  .assimilate_storage(&mut t)
  .unwrap();

  let mut ext = sp_io::TestExternalities::new(t);
  ext.execute_with(|| System::set_block_number(1));
  ext
}
