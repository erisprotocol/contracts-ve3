use crate::constants::MAX_LOCK_PERIODS;
use cosmwasm_std::{Decimal, StdResult, Uint128};

/// Coefficient calculation where 0 [`WEEK`] is equal to 0 and [`MAX_LOCK_TIME`] is 9. So possible range is 0-9.
pub fn calc_coefficient(interval: u64) -> Decimal {
  // coefficient = 9 * (end - start) / MAX_LOCK_TIME
  // +1 is added in the fixed part and not included here.
  Decimal::from_ratio(90_u64 * interval, MAX_LOCK_PERIODS * 10)
}

/// Adjusting voting power according to the slope. The maximum loss is 103/104 * 104 which is 0.000103
pub fn adjust_vp_and_slope(vp: &mut Uint128, dt: u64) -> StdResult<Uint128> {
  let slope = vp.checked_div(Uint128::from(dt))?;
  *vp = slope * Uint128::from(dt);
  Ok(slope)
}
