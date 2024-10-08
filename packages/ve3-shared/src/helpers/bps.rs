use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Fraction, StdError, Uint128};
use std::convert::{TryFrom, TryInto};
use std::ops::Mul;

/// BasicPoints struct implementation. BasicPoints value is within [0, 10000] interval.
/// Technically BasicPoints is wrapper over [`u16`] with additional limit checks and
/// several implementations of math functions so BasicPoints object
/// can be used in formulas along with [`Uint128`] and [`Decimal`].
#[cw_serde]
#[derive(Default, Copy)]
pub struct BasicPoints(u16);

impl BasicPoints {
  pub const MAX: u16 = 10000;

  pub fn checked_add(self, rhs: Self) -> Result<Self, StdError> {
    let next_value = self.0 + rhs.0;
    if next_value > Self::MAX {
      Err(StdError::generic_err("Basic points sum exceeds limit"))
    } else {
      Ok(Self(next_value))
    }
  }

  pub fn from_ratio(numerator: Uint128, denominator: Uint128) -> Result<Self, StdError> {
    numerator
      .checked_multiply_ratio(Self::MAX, denominator)
      .map_err(|_| StdError::generic_err("Checked multiply ratio error!"))?
      .u128()
      .try_into()
  }

  pub fn percent(percent: u16) -> BasicPoints {
    BasicPoints(percent * 100)
  }

  pub fn reverse(self) -> BasicPoints {
    BasicPoints(Self::MAX - self.0)
  }

  pub fn decimal(self) -> Decimal {
    Decimal::from_ratio(self.0, Self::MAX)
  }

  pub fn div_decimal(self, rhs: Self) -> Decimal {
    if self.is_zero() {
      return Decimal::zero();
    }
    Decimal::from_ratio(self.0, rhs.0)
  }

  #[inline]
  pub const fn max() -> Self {
    BasicPoints(BasicPoints::MAX)
  }

  #[inline]
  pub const fn one() -> Self {
    BasicPoints(BasicPoints::MAX)
  }

  #[inline]
  pub const fn zero() -> Self {
    BasicPoints(0)
  }

  pub const fn u16(self) -> u16 {
    self.0
  }

  pub const fn is_max(self) -> bool {
    self.0 == BasicPoints::MAX
  }
  pub const fn is_zero(self) -> bool {
    self.0 == 0
  }
}

impl TryFrom<u16> for BasicPoints {
  type Error = StdError;

  fn try_from(value: u16) -> Result<Self, Self::Error> {
    if value <= Self::MAX {
      Ok(Self(value))
    } else {
      Err(StdError::generic_err(format!("Basic points conversion error. {0} > 10000", value)))
    }
  }
}

impl TryFrom<u128> for BasicPoints {
  type Error = StdError;

  fn try_from(value: u128) -> Result<Self, Self::Error> {
    if value <= Self::MAX as u128 {
      Ok(Self(value as u16))
    } else {
      Err(StdError::generic_err(format!("Basic points conversion error. {0} > 10000", value)))
    }
  }
}

impl TryFrom<Decimal> for BasicPoints {
  type Error = StdError;

  fn try_from(value: Decimal) -> Result<Self, Self::Error> {
    if value > Decimal::one() {
      Err(StdError::generic_err(format!("Basic points conversion error. {0} > 10000", value)))
    } else {
      BasicPoints::from_ratio(value.numerator(), value.denominator())
    }
  }
}

impl From<BasicPoints> for u16 {
  fn from(value: BasicPoints) -> Self {
    value.0
  }
}

impl From<BasicPoints> for Uint128 {
  fn from(value: BasicPoints) -> Self {
    Uint128::from(u16::from(value))
  }
}

impl Mul<Uint128> for BasicPoints {
  type Output = Uint128;

  fn mul(self, rhs: Uint128) -> Self::Output {
    if self.is_max() {
      rhs
    } else {
      rhs.multiply_ratio(self.0, Self::MAX)
    }
  }
}

impl Mul<Decimal> for BasicPoints {
  type Output = Decimal;

  fn mul(self, rhs: Decimal) -> Self::Output {
    Decimal::from_ratio(
      rhs.numerator() * Uint128::from(self.0),
      rhs.denominator() * Uint128::from(Self::MAX),
    )
  }
}
