use crate::{mock::*, Event};

use frame_support::traits::Hooks;

use serai_primitives::{Amount, ExternalCoin, EXTERNAL_COINS};

/// Without any oracle value, `on_initialize` must not mark any network as having
/// reached economic security.
#[test]
fn no_economic_security_without_oracle_value() {
  new_test_ext().execute_with(|| {
    EconomicSecurity::on_initialize(1);

    for coin in EXTERNAL_COINS {
      assert!(EconomicSecurity::economic_security_block(coin.network()).is_none());
    }
  });
}

/// Once a coin has an oracle value (and minting is allowed, as it is by default in
/// the mock), `on_initialize` must record the current block for that coin's network
/// and emit `EconomicSecurityReached` exactly once.
#[test]
fn reaches_economic_security() {
  new_test_ext().execute_with(|| {
    let secured = ExternalCoin::Bitcoin;

    // Inject an oracle value for a single coin.
    dex::SecurityOracleValue::<Test>::set(secured, Some(Amount(1)));

    let block = 5u64;
    System::set_block_number(block);
    System::reset_events();
    EconomicSecurity::on_initialize(block);

    // The secured coin's network is now recorded at this block.
    assert_eq!(EconomicSecurity::economic_security_block(secured.network()), Some(block));

    // Coins without an oracle value remain unsecured.
    for coin in EXTERNAL_COINS {
      if coin != secured {
        assert!(EconomicSecurity::economic_security_block(coin.network()).is_none());
      }
    }

    // Exactly one `EconomicSecurityReached` event was emitted, for the secured network.
    let reached = System::events()
      .into_iter()
      .filter(|record| {
        record.event ==
          RuntimeEvent::EconomicSecurity(Event::EconomicSecurityReached {
            network: secured.network(),
          })
      })
      .count();
    assert_eq!(reached, 1);
  });
}

/// `on_initialize` is idempotent: a coin already marked as secured keeps its original
/// block and does not emit a second event.
#[test]
fn economic_security_is_not_overwritten() {
  new_test_ext().execute_with(|| {
    let secured = ExternalCoin::Bitcoin;
    dex::SecurityOracleValue::<Test>::set(secured, Some(Amount(1)));

    EconomicSecurity::on_initialize(5);
    assert_eq!(EconomicSecurity::economic_security_block(secured.network()), Some(5));

    System::set_block_number(10);
    System::reset_events();
    EconomicSecurity::on_initialize(10);

    // Still recorded at the original block.
    assert_eq!(EconomicSecurity::economic_security_block(secured.network()), Some(5));

    // No new event emitted.
    let reached = System::events()
      .into_iter()
      .filter(|record| {
        matches!(
          record.event,
          RuntimeEvent::EconomicSecurity(Event::EconomicSecurityReached { .. })
        )
      })
      .count();
    assert_eq!(reached, 0);
  });
}
