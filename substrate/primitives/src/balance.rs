use core::ops::{Add, Sub, Mul};

#[cfg(feature = "std")]
use zeroize::Zeroize;

#[cfg(feature = "borsh")]
use borsh::{BorshSerialize, BorshDeserialize};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use scale::{Encode, Decode, DecodeWithMemTracking, MaxEncodedLen};

use crate::{Amount, Coin, ExternalCoin};

/// The type used for balances (a Coin and Balance).
#[derive(
  Clone, Copy, PartialEq, Eq, Debug, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Zeroize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Balance {
  pub coin: Coin,
  pub amount: Amount,
}

/// The type used for balances (a Coin and Balance).
#[derive(
  Clone, Copy, PartialEq, Eq, Debug, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Zeroize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ExternalBalance {
  pub coin: ExternalCoin,
  pub amount: Amount,
}

impl From<ExternalBalance> for Balance {
  fn from(balance: ExternalBalance) -> Self {
    Balance { coin: balance.coin.into(), amount: balance.amount }
  }
}

impl TryFrom<Balance> for ExternalBalance {
  type Error = ();

  fn try_from(balance: Balance) -> Result<Self, Self::Error> {
    match balance.coin {
      Coin::Serai => Err(())?,
      Coin::External(coin) => Ok(ExternalBalance { coin, amount: balance.amount }),
    }
  }
}

impl Balance {
  /// Add an amount to this balance, returning `None` on overflow instead of panicking.
  pub fn checked_add(self, other: Amount) -> Option<Balance> {
    Some(Balance { coin: self.coin, amount: self.amount.checked_add(other)? })
  }

  /// Subtract an amount from this balance, returning `None` on underflow instead of panicking.
  pub fn checked_sub(self, other: Amount) -> Option<Balance> {
    Some(Balance { coin: self.coin, amount: self.amount.checked_sub(other)? })
  }

  /// Multiply this balance by an amount, returning `None` on overflow instead of panicking.
  pub fn checked_mul(self, other: Amount) -> Option<Balance> {
    Some(Balance { coin: self.coin, amount: self.amount.checked_mul(other)? })
  }
}

impl ExternalBalance {
  /// Add an amount to this balance, returning `None` on overflow instead of panicking.
  pub fn checked_add(self, other: Amount) -> Option<ExternalBalance> {
    Some(ExternalBalance { coin: self.coin, amount: self.amount.checked_add(other)? })
  }

  /// Subtract an amount from this balance, returning `None` on underflow instead of panicking.
  pub fn checked_sub(self, other: Amount) -> Option<ExternalBalance> {
    Some(ExternalBalance { coin: self.coin, amount: self.amount.checked_sub(other)? })
  }

  /// Multiply this balance by an amount, returning `None` on overflow instead of panicking.
  pub fn checked_mul(self, other: Amount) -> Option<ExternalBalance> {
    Some(ExternalBalance { coin: self.coin, amount: self.amount.checked_mul(other)? })
  }
}

// The operator impls below panic on overflow/underflow. Code paths which must not be able to panic
// the runtime should use the `checked_*` methods above.
impl Add<Amount> for Balance {
  type Output = Balance;
  fn add(self, other: Amount) -> Balance {
    Balance { coin: self.coin, amount: self.amount + other }
  }
}

impl Sub<Amount> for Balance {
  type Output = Balance;
  fn sub(self, other: Amount) -> Balance {
    Balance { coin: self.coin, amount: self.amount - other }
  }
}

impl Mul<Amount> for Balance {
  type Output = Balance;
  fn mul(self, other: Amount) -> Balance {
    Balance { coin: self.coin, amount: self.amount * other }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn balance_checked_ops() {
    let balance = Balance { coin: Coin::Serai, amount: Amount(10) };

    assert_eq!(
      balance.checked_add(Amount(5)),
      Some(Balance { coin: Coin::Serai, amount: Amount(15) })
    );
    assert_eq!(
      balance.checked_sub(Amount(4)),
      Some(Balance { coin: Coin::Serai, amount: Amount(6) })
    );
    assert_eq!(
      balance.checked_mul(Amount(3)),
      Some(Balance { coin: Coin::Serai, amount: Amount(30) })
    );

    // Underflow/overflow return None rather than panicking.
    assert_eq!(balance.checked_sub(Amount(11)), None);
    let max = Balance { coin: Coin::Serai, amount: Amount(u64::MAX) };
    assert_eq!(max.checked_add(Amount(1)), None);
    assert_eq!(max.checked_mul(Amount(2)), None);

    // The coin is preserved across operations.
    assert_eq!(balance.checked_add(Amount(1)).unwrap().coin, Coin::Serai);
  }

  #[test]
  fn external_balance_checked_ops() {
    let balance = ExternalBalance { coin: ExternalCoin::Bitcoin, amount: Amount(10) };

    assert_eq!(
      balance.checked_add(Amount(5)),
      Some(ExternalBalance { coin: ExternalCoin::Bitcoin, amount: Amount(15) })
    );
    assert_eq!(balance.checked_sub(Amount(11)), None);
    assert_eq!(balance.checked_sub(Amount(10)).unwrap().amount, Amount(0));
  }
}
