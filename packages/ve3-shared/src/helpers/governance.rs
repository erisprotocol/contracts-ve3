use crate::constants::{EPOCH_START, WEEK};
use cosmwasm_std::{StdError, StdResult, Uint128};

/// Calculates the period number. Time should be formatted as a timestamp.
pub fn get_period(time: u64) -> StdResult<u64> {
  if time < EPOCH_START {
    Err(StdError::generic_err("Invalid time"))
  } else {
    Ok((time - EPOCH_START) / WEEK)
  }
}

/// converts the period to the start time of the period (EPOCH_START + period * WEEK)
pub fn get_s_from_period(period: u64) -> u64 {
  EPOCH_START + period * WEEK
}

/// Calculates how many periods are in the specified time interval. The time should be in seconds.
pub fn get_periods_count(interval: u64) -> u64 {
  interval / WEEK
}

/// Main function used to calculate a user's voting power at a specific period as: previous_power - slope*(x - previous_x).
pub fn calc_voting_power(
  slope: Uint128,
  old_vp: Uint128,
  start_period: u64,
  end_period: u64,
) -> Uint128 {
  let shift =
    slope.checked_mul(Uint128::from(end_period - start_period)).unwrap_or_else(|_| Uint128::zero());
  old_vp.saturating_sub(shift)
}
