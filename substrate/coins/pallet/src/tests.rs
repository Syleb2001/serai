use crate::{mock::*, primitives::*};

use frame_system::RawOrigin;
use frame_support::{traits::Hooks, assert_noop};
use sp_core::{Pair, sr25519::Public};

use serai_primitives::*;

pub type CoinsEvent = crate::Event<Test, ()>;

/// Count the `Coins` events emitted so far which match `predicate`.
fn count_coins_events(predicate: impl Fn(&CoinsEvent) -> bool) -> usize {
  System::events()
    .iter()
    .filter(|event| {
      if let RuntimeEvent::Coins(e) = &event.event {
        predicate(e)
      } else {
        false
      }
    })
    .count()
}

#[test]
fn mint() {
  new_test_ext().execute_with(|| {
    // minting u64::MAX should work
    let coin = Coin::Serai;
    let to = insecure_pair_from_name("random1").public();
    let balance = Balance { coin, amount: Amount(u64::MAX) };

    Coins::mint(to, balance).unwrap();
    assert_eq!(Coins::balance(to, coin), balance.amount);

    // minting more should fail
    assert!(Coins::mint(to, Balance { coin, amount: Amount(1) }).is_err());

    // supply now should be equal to sum of the accounts balance sum
    assert_eq!(Coins::supply(coin), balance.amount.0);

    // test events
    let mint_events = System::events()
      .iter()
      .filter_map(|event| {
        if let RuntimeEvent::Coins(e) = &event.event {
          if matches!(e, CoinsEvent::Mint { .. }) {
            Some(e.clone())
          } else {
            None
          }
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    assert_eq!(mint_events, vec![CoinsEvent::Mint { to, balance }]);
  })
}

#[test]
fn burn_with_instruction() {
  new_test_ext().execute_with(|| {
    // mint some coin
    let coin = Coin::External(ExternalCoin::Bitcoin);
    let to = insecure_pair_from_name("random1").public();
    let balance = Balance { coin, amount: Amount(10 * 10u64.pow(coin.decimals())) };

    Coins::mint(to, balance).unwrap();
    assert_eq!(Coins::balance(to, coin), balance.amount);
    assert_eq!(Coins::supply(coin), balance.amount.0);

    // we shouldn't be able to burn more than what we have
    let mut instruction = OutInstructionWithBalance {
      instruction: OutInstruction { address: ExternalAddress::new(vec![]).unwrap(), data: None },
      balance: ExternalBalance {
        coin: coin.try_into().unwrap(),
        amount: Amount(balance.amount.0 + 1),
      },
    };
    assert!(
      Coins::burn_with_instruction(RawOrigin::Signed(to).into(), instruction.clone()).is_err()
    );

    // it should now work
    instruction.balance.amount = balance.amount;
    Coins::burn_with_instruction(RawOrigin::Signed(to).into(), instruction.clone()).unwrap();

    // balance & supply now should be back to 0
    assert_eq!(Coins::balance(to, coin), Amount(0));
    assert_eq!(Coins::supply(coin), 0);

    let burn_events = System::events()
      .iter()
      .filter_map(|event| {
        if let RuntimeEvent::Coins(e) = &event.event {
          if matches!(e, CoinsEvent::BurnWithInstruction { .. }) {
            Some(e.clone())
          } else {
            None
          }
        } else {
          None
        }
      })
      .collect::<Vec<_>>();

    assert_eq!(burn_events, vec![CoinsEvent::BurnWithInstruction { from: to, instruction }]);
  })
}

#[test]
fn transfer() {
  new_test_ext().execute_with(|| {
    // mint some coin
    let coin = Coin::External(ExternalCoin::Bitcoin);
    let from = insecure_pair_from_name("random1").public();
    let balance = Balance { coin, amount: Amount(10 * 10u64.pow(coin.decimals())) };

    Coins::mint(from, balance).unwrap();
    assert_eq!(Coins::balance(from, coin), balance.amount);
    assert_eq!(Coins::supply(coin), balance.amount.0);

    // we can't send more than what we have
    let to = insecure_pair_from_name("random2").public();
    assert!(Coins::transfer(
      RawOrigin::Signed(from).into(),
      to,
      Balance { coin, amount: Amount(balance.amount.0 + 1) }
    )
    .is_err());

    // we can send it all
    Coins::transfer(RawOrigin::Signed(from).into(), to, balance).unwrap();

    // check the balances
    assert_eq!(Coins::balance(from, coin), Amount(0));
    assert_eq!(Coins::balance(to, coin), balance.amount);

    // supply shouldn't change
    assert_eq!(Coins::supply(coin), balance.amount.0);
  })
}

#[test]
fn mint_accumulates() {
  new_test_ext().execute_with(|| {
    let coin = Coin::Serai;
    let to = insecure_pair_from_name("random1").public();

    Coins::mint(to, Balance { coin, amount: Amount(10) }).unwrap();
    Coins::mint(to, Balance { coin, amount: Amount(5) }).unwrap();

    assert_eq!(Coins::balance(to, coin), Amount(15));
    assert_eq!(Coins::supply(coin), 15);
  })
}

#[test]
fn burn_reduces_balance_and_supply() {
  new_test_ext().execute_with(|| {
    let coin = Coin::External(ExternalCoin::Bitcoin);
    let from = insecure_pair_from_name("random1").public();
    Coins::mint(from, Balance { coin, amount: Amount(100) }).unwrap();

    // Burning more than held fails without mutating state.
    assert_noop!(
      Coins::burn(RawOrigin::Signed(from).into(), Balance { coin, amount: Amount(101) }),
      crate::Error::<Test, ()>::NotEnoughCoins
    );
    assert_eq!(Coins::balance(from, coin), Amount(100));
    assert_eq!(Coins::supply(coin), 100);

    // Burning part reduces both balance and supply, and emits a Burn event.
    let burned = Balance { coin, amount: Amount(40) };
    Coins::burn(RawOrigin::Signed(from).into(), burned).unwrap();
    assert_eq!(Coins::balance(from, coin), Amount(60));
    assert_eq!(Coins::supply(coin), 60);

    assert_eq!(
      count_coins_events(|e| matches!(e, CoinsEvent::Burn { from: f, balance } if *f == from && *balance == burned)),
      1
    );
  })
}

#[test]
fn transfer_emits_event_and_cleans_storage() {
  new_test_ext().execute_with(|| {
    let coin = Coin::External(ExternalCoin::Bitcoin);
    let from = insecure_pair_from_name("random1").public();
    let to = insecure_pair_from_name("random2").public();
    Coins::mint(from, Balance { coin, amount: Amount(100) }).unwrap();

    // A partial transfer keeps the sender's storage entry.
    Coins::transfer(RawOrigin::Signed(from).into(), to, Balance { coin, amount: Amount(40) })
      .unwrap();
    assert!(crate::Balances::<Test>::contains_key(from, coin));
    assert_eq!(Coins::balance(from, coin), Amount(60));

    // Transferring the remainder removes the now-zero storage entry.
    Coins::transfer(RawOrigin::Signed(from).into(), to, Balance { coin, amount: Amount(60) })
      .unwrap();
    assert!(!crate::Balances::<Test>::contains_key(from, coin));
    assert_eq!(Coins::balance(from, coin), Amount(0));
    assert_eq!(Coins::balance(to, coin), Amount(100));

    // Both transfers emitted a Transfer event; supply is unchanged.
    assert_eq!(count_coins_events(|e| matches!(e, CoinsEvent::Transfer { .. })), 2);
    assert_eq!(Coins::supply(coin), 100);
  })
}

#[test]
fn fees_are_burned_on_initialize() {
  new_test_ext().execute_with(|| {
    let fee_account: Public = FEE_ACCOUNT.into();
    let coin = Coin::Serai;

    // Simulate fees collected into the fee account during a block.
    Coins::mint(fee_account, Balance { coin, amount: Amount(1000) }).unwrap();
    assert_eq!(Coins::balance(fee_account, coin), Amount(1000));
    assert_eq!(Coins::supply(coin), 1000);

    // The next block's on_initialize burns the collected fees.
    Coins::on_initialize(1);
    assert_eq!(Coins::balance(fee_account, coin), Amount(0));
    assert_eq!(Coins::supply(coin), 0);
  })
}

#[test]
fn burn_with_instruction_not_allowed_for_liquidity_tokens() {
  new_test_ext().execute_with(|| {
    let who = insecure_pair_from_name("random1").public();
    let instruction = OutInstructionWithBalance {
      instruction: OutInstruction { address: ExternalAddress::new(vec![]).unwrap(), data: None },
      balance: ExternalBalance { coin: ExternalCoin::Bitcoin, amount: Amount(1) },
    };

    // Liquidity tokens (Instance1) must never be burnable with an out-instruction.
    assert_noop!(
      LiquidityTokens::burn_with_instruction(RawOrigin::Signed(who).into(), instruction),
      crate::Error::<Test, crate::Instance1>::BurnWithInstructionNotAllowed
    );
  })
}
