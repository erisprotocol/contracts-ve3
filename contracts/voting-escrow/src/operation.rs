use cosmwasm_std::Uint128;

use crate::error::ContractError;

pub enum Operation {
  None,
  Add(Uint128),
  Reduce(Uint128),
}

impl Operation {
  pub fn add_amount(&self) -> Option<Uint128> {
    match self {
      Operation::Add(amount) => Some(*amount),
      _ => None,
    }
  }
  pub fn reduce_amount(&self) -> Option<Uint128> {
    match self {
      Operation::Reduce(amount) => Some(*amount),
      _ => None,
    }
  }

  pub fn apply_to(&self, rhs: Uint128) -> Result<Uint128, ContractError> {
    match self {
      Operation::None => Ok(rhs),
      Operation::Add(amount) => Ok(rhs.checked_add(*amount)?),
      Operation::Reduce(amount) => Ok(rhs.saturating_sub(*amount)),
    }
  }

  pub fn from_values(new: Uint128, old: Uint128) -> Self {
    match new.cmp(&old) {
      std::cmp::Ordering::Less => Operation::Reduce(old.saturating_sub(new)),
      std::cmp::Ordering::Equal => Operation::None,
      std::cmp::Ordering::Greater => Operation::Add(new.saturating_sub(old)),
    }
  }
}
