use core::{
  ops::{Add, Sub, Mul},
  fmt::Debug,
};

#[cfg(feature = "std")]
use zeroize::Zeroize;

#[cfg(feature = "borsh")]
use borsh::{BorshSerialize, BorshDeserialize};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use scale::{Encode, Decode, DecodeWithMemTracking, MaxEncodedLen};

/// The type used for amounts within Substrate.
// Distinct from Amount due to Substrate's requirements on this type.
// While Amount could have all the necessary traits implemented, not only are they many, it'd make
// Amount a large type with a variety of misc functions.
// The current type's minimalism sets clear bounds on usage.
pub type SubstrateAmount = u64;
/// The type used for amounts.
#[derive(
  Clone,
  Copy,
  PartialEq,
  Eq,
  PartialOrd,
  Debug,
  Encode,
  Decode,
  DecodeWithMemTracking,
  MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Zeroize))]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Amount(pub SubstrateAmount);

impl Amount {
  /// Add two amounts, returning `None` on overflow instead of panicking.
  pub fn checked_add(self, other: Amount) -> Option<Amount> {
    self.0.checked_add(other.0).map(Amount)
  }

  /// Subtract two amounts, returning `None` on underflow instead of panicking.
  pub fn checked_sub(self, other: Amount) -> Option<Amount> {
    self.0.checked_sub(other.0).map(Amount)
  }

  /// Multiply two amounts, returning `None` on overflow instead of panicking.
  pub fn checked_mul(self, other: Amount) -> Option<Amount> {
    self.0.checked_mul(other.0).map(Amount)
  }

  /// Add two amounts, saturating at the numeric bound instead of panicking.
  pub fn saturating_add(self, other: Amount) -> Amount {
    Amount(self.0.saturating_add(other.0))
  }

  /// Subtract two amounts, saturating at zero instead of panicking.
  pub fn saturating_sub(self, other: Amount) -> Amount {
    Amount(self.0.saturating_sub(other.0))
  }
}

// The `Add`/`Sub`/`Mul` operator impls below panic on overflow/underflow. They are kept for
// ergonomic use where the operands are known to be in range. Code paths which may overflow (and
// must not be able to panic the runtime) should use the `checked_*`/`saturating_*` methods above.
impl Add for Amount {
  type Output = Amount;
  fn add(self, other: Amount) -> Amount {
    // Explicitly use checked_add so even if range checks are disabled, this is still checked
    Amount(self.0.checked_add(other.0).unwrap())
  }
}

impl Sub for Amount {
  type Output = Amount;
  fn sub(self, other: Amount) -> Amount {
    Amount(self.0.checked_sub(other.0).unwrap())
  }
}

impl Mul for Amount {
  type Output = Amount;
  fn mul(self, other: Amount) -> Amount {
    Amount(self.0.checked_mul(other.0).unwrap())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn checked_add_handles_overflow() {
    assert_eq!(Amount(2).checked_add(Amount(3)), Some(Amount(5)));
    assert_eq!(Amount(u64::MAX).checked_add(Amount(1)), None);
  }

  #[test]
  fn checked_sub_handles_underflow() {
    assert_eq!(Amount(5).checked_sub(Amount(3)), Some(Amount(2)));
    assert_eq!(Amount(0).checked_sub(Amount(1)), None);
  }

  #[test]
  fn checked_mul_handles_overflow() {
    assert_eq!(Amount(4).checked_mul(Amount(5)), Some(Amount(20)));
    assert_eq!(Amount(u64::MAX).checked_mul(Amount(2)), None);
  }

  #[test]
  fn saturating_ops_clamp() {
    assert_eq!(Amount(u64::MAX).saturating_add(Amount(10)), Amount(u64::MAX));
    assert_eq!(Amount(3).saturating_sub(Amount(10)), Amount(0));
    assert_eq!(Amount(3).saturating_add(Amount(4)), Amount(7));
    assert_eq!(Amount(10).saturating_sub(Amount(4)), Amount(6));
  }

  #[test]
  fn operators_still_work_in_range() {
    assert_eq!(Amount(2) + Amount(3), Amount(5));
    assert_eq!(Amount(5) - Amount(3), Amount(2));
    assert_eq!(Amount(4) * Amount(5), Amount(20));
  }
}
