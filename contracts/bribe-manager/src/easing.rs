use cosmwasm_std::{Decimal, Uint128};
use ve3_shared::msgs_bribe_manager::{BribeDistribution, FuncType};

use crate::error::ContractError;

pub trait BribeDistributionExt {
  fn create_distribution(
    self,
    block_period: u64,
    amount: Uint128,
  ) -> Result<Vec<(u64, Uint128)>, ContractError>;
}

impl BribeDistributionExt for BribeDistribution {
  fn create_distribution(
    self,
    block_period: u64,
    amount: Uint128,
  ) -> Result<Vec<(u64, Uint128)>, ContractError> {
    match self {
      BribeDistribution::Func {
        start,
        end,
        func_type,
      } => {
        let start = start.unwrap_or(block_period + 1);
        if end < start {
          Err(ContractError::BribeDistribution(format!(
            "from ({0}) must be <= to ({1}).",
            start, end
          )))?;
        }

        let periods = end + 1 - start;

        let func: fn(Decimal) -> Decimal = match func_type {
          FuncType::Linear => linear,
          FuncType::Bezier => bezier,
          FuncType::EaseInCubic => ease_in_cubic,
          FuncType::EaseOutCubic => ease_out_cubic,
          FuncType::EaseInOutCubic => ease_in_out_cubic,
          FuncType::Parametric => parametric,
        };

        let mut results = vec![];
        let mut last = Uint128::zero();
        for n in start..=end {
          let progress = Decimal::from_ratio(n + 1 - start, periods);
          let expected: Decimal = func(progress);
          let total = expected * amount;
          let delta = if n == end {
            amount - last
          } else {
            total - last
          };
          last = total;
          results.push((n, delta));
        }

        Ok(results)
      },
      BribeDistribution::Next => Ok(vec![(block_period + 1, amount)]),
      BribeDistribution::Specific(specific) => Ok(specific),
    }
  }
}

pub fn linear(t: Decimal) -> Decimal {
  t
}

pub fn bezier(t: Decimal) -> Decimal {
  t * t * (dec(3) - dec(2) * t)
}

pub fn parametric(t: Decimal) -> Decimal {
  let sqr = t * t;
  sqr / (dec(2) * sqr + Decimal::one() - dec(2) * t)
}

pub fn ease_in_cubic(t: Decimal) -> Decimal {
  t * t * t
}

pub fn ease_out_cubic(t: Decimal) -> Decimal {
  dec(3) * t + t * t * t - dec(3) * t * t
}

pub fn ease_in_out_cubic(t: Decimal) -> Decimal {
  if t < Decimal::from_ratio(1u128, 2u128) {
    dec(4) * t * t * t
  } else {
    dec(4) * t * t * t + dec(12) * t - dec(12) * t * t - dec(3)
  }
}

#[inline]
fn dec(numb: u128) -> Decimal {
  Decimal::from_ratio(numb, 1u128)
}

#[cfg(test)]
mod test {
  use cosmwasm_std::Uint128;
  use ve3_shared::msgs_bribe_manager::{BribeDistribution, FuncType};

  use crate::error::ContractError;

  use super::BribeDistributionExt;

  #[test]
  fn test_bezier() -> Result<(), ContractError> {
    let distribution = BribeDistribution::Func {
      start: Some(1),
      end: 100,
      func_type: FuncType::Bezier,
    }
    .create_distribution(0, Uint128::new(100_000000))?;

    println!("{distribution:?}");

    for (x, y) in distribution {
      let amount = y.u128();
      println!("{x} {amount}")
    }

    Ok(())
  }

  #[test]
  fn test_ease_in_cubic() -> Result<(), ContractError> {
    let distribution = BribeDistribution::Func {
      start: Some(1),
      end: 10,
      func_type: FuncType::Linear,
    }
    .create_distribution(0, Uint128::new(100_000005))?;

    println!("{distribution:?}");

    for (x, y) in distribution {
      let amount = y.u128();
      println!("{x} {amount}")
    }

    Ok(())
  }
}
