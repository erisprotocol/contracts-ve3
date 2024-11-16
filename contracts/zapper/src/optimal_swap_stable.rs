use crate::{constants::OPTIMAL_SWAP_ITERATIONS, error::ContractError};
use cosmwasm_std::CosmosMsg;
use cosmwasm_std::{Decimal, Decimal256, QuerierWrapper, Uint128, Uint256};
use cw_asset::Asset;
use ve3_shared::{
  adapters::pair::{Pair, PairInfo},
  extensions::asset_info_ext::AssetInfoExt,
};

/// # Description
/// Calculates the amount of asset in the pair contract that need to be swapped before providing liquidity.
/// The swap messages will be added to **messages**.
pub fn calculate_optimal_swap_stableswap(
  querier: &QuerierWrapper,
  pair_info: &PairInfo,
  asset_a: Asset,
  asset_b: Asset,
  messages: &mut Vec<CosmosMsg>,
  max_spread: Decimal,
) -> Result<(Uint128, Uint128, Uint128, Uint128, u64, Decimal256), ContractError> {
  let pair_contract = pair_info.contract_addr.clone();

  let y_a: Uint128 = asset_a.amount;
  let y_b: Uint128 = asset_b.amount;

  let x_a: Uint256 = pair_info.asset_infos[0].query_balance(querier, &pair_contract)?.into();
  let x_b: Uint256 = pair_info.asset_infos[1].query_balance(querier, &pair_contract)?.into();

  let pool_ratio = Decimal256::from_ratio(x_a, x_b);

  let pair = Pair(pair_contract);
  let is_ww = pair_info.is_ww();

  let mut swap_a_to_b = Uint128::zero();
  let mut swap_b_to_a = Uint128::zero();
  let mut adjusted_y_a: Uint128 = y_a;
  let mut adjusted_y_b: Uint128 = y_b;
  let mut iterations = 0;
  let mut adjusted_ratio: Decimal256 = Decimal256::zero();
  let mut difference: Decimal256 = Decimal256::zero();

  // Handle cases where one of the available balances is zero
  if y_a.is_zero() {
    let mut left = Uint128::zero();
    let mut right = y_b;

    for i in 0..OPTIMAL_SWAP_ITERATIONS {
      iterations = i;
      let mid = (left + right).checked_div(Uint128::new(2))?;
      let s_a =
        pair.query_simulate(querier, is_ww, asset_b.info.with_balance(mid), None)?.return_amount;

      adjusted_y_a = y_a + s_a;
      adjusted_y_b = y_b - mid;

      adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);

      if adjusted_ratio > pool_ratio {
        right = mid;
      } else {
        left = mid;
      }
      swap_b_to_a = mid;

      if right - left <= Uint128::one()
        || assert_ratio(adjusted_ratio, pool_ratio, "".to_string()).is_ok()
      {
        break;
      }
    }

    difference = assert_ratio(
      adjusted_ratio,
      pool_ratio,
      format!(
        "swapping {0} b = {1} a + {2} b {3} iterations",
        swap_b_to_a, adjusted_y_a, adjusted_y_b, iterations
      ),
    )?;

    // (adjusted_y_a, adjusted_y_b) =
    //   validate_ratio_b_to_a(&pair, querier, is_ww, &asset_b, swap_b_to_a, y_a, y_b, pool_ratio)?;

    if !swap_b_to_a.is_zero() {
      messages.push(pair.swap_msg(
        &asset_b.info.with_balance(swap_b_to_a),
        None,
        Some(max_spread),
        None,
      )?);
    }

    return Ok((swap_a_to_b, swap_b_to_a, adjusted_y_a, adjusted_y_b, iterations, difference));
  }

  if y_b.is_zero() {
    let mut left = Uint128::zero();
    let mut right = y_a;

    for i in 0..OPTIMAL_SWAP_ITERATIONS {
      iterations = i;
      let mid = (left + right).checked_div(Uint128::new(2))?;
      let s_b =
        pair.query_simulate(querier, is_ww, asset_a.info.with_balance(mid), None)?.return_amount;

      adjusted_y_a = y_a - mid;
      adjusted_y_b = y_b + s_b;

      adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);

      if adjusted_ratio < pool_ratio {
        right = mid;
      } else {
        left = mid;
      }

      swap_a_to_b = mid;

      if right - left <= Uint128::one()
        || assert_ratio(adjusted_ratio, pool_ratio, "".to_string()).is_ok()
      {
        break;
      }
    }

    difference = assert_ratio(
      adjusted_ratio,
      pool_ratio,
      format!(
        "swapping {0} a = {1} a + {2} b {3} iterations",
        swap_a_to_b, adjusted_y_a, adjusted_y_b, iterations
      ),
    )?;

    // swap_a_to_b = left;
    // (adjusted_y_a, adjusted_y_b) =
    //   validate_ratio_a_to_b(&pair, querier, is_ww, &asset_a, swap_a_to_b, y_a, y_b, pool_ratio)?;

    if !swap_a_to_b.is_zero() {
      messages.push(pair.swap_msg(
        &asset_a.info.with_balance(swap_a_to_b),
        None,
        Some(max_spread),
        None,
      )?);
    }

    return Ok((swap_a_to_b, swap_b_to_a, adjusted_y_a, adjusted_y_b, iterations, difference));
  }

  let user_ratio = Decimal256::from_ratio(y_a, y_b);
  let mut left = Uint128::zero();
  let mut right = if user_ratio > pool_ratio {
    y_a
  } else {
    y_b
  };

  // If the user's ratio already matches the pool ratio, no swap needed
  if assert_ratio(user_ratio, pool_ratio, "".to_string()).is_ok() {
    return Ok((swap_a_to_b, swap_b_to_a, y_a, y_b, iterations, difference)); // No swap needed
  }

  for i in 0..OPTIMAL_SWAP_ITERATIONS {
    iterations = i;
    let mid = (left + right).checked_div(Uint128::new(2))?;

    // Simulate swap
    if user_ratio > pool_ratio {
      // Swap Asset A to B
      let s_b =
        pair.query_simulate(querier, is_ww, asset_a.info.with_balance(mid), None)?.return_amount;
      // Calculate amount of B received
      adjusted_y_a = y_a - mid;
      adjusted_y_b = y_b + s_b;

      adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);

      if adjusted_ratio < pool_ratio {
        right = mid;
      } else {
        left = mid;
      }

      swap_a_to_b = mid;
    } else {
      // Swap Asset B to A
      let s_a =
        pair.query_simulate(querier, is_ww, asset_b.info.with_balance(mid), None)?.return_amount;
      // Calculate amount of A received
      adjusted_y_a = y_a + s_a;
      adjusted_y_b = y_b - mid;

      adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);

      if adjusted_ratio > pool_ratio {
        right = mid;
      } else {
        left = mid;
      }

      swap_b_to_a = mid;
    }

    if right - left <= Uint128::one()
      || assert_ratio(adjusted_ratio, pool_ratio, "".to_string()).is_ok()
    {
      break;
    }
  }

  if !swap_a_to_b.is_zero() {
    difference = assert_ratio(
      adjusted_ratio,
      pool_ratio,
      format!(
        "swapping {0} a = {1} a + {2} b {3} iterations",
        swap_a_to_b, adjusted_y_a, adjusted_y_b, iterations
      ),
    )?;

    // (adjusted_y_a, adjusted_y_b) =
    //   validate_ratio_a_to_b(&pair, querier, is_ww, &asset_a, swap_a_to_b, y_a, y_b, pool_ratio)?;

    messages.push(pair.swap_msg(
      &asset_a.info.with_balance(swap_a_to_b),
      None,
      Some(max_spread),
      None,
    )?);
  } else if !swap_b_to_a.is_zero() {
    difference = assert_ratio(
      adjusted_ratio,
      pool_ratio,
      format!(
        "swapping {0} b = {1} a + {2} b {3} iterations",
        swap_b_to_a, adjusted_y_a, adjusted_y_b, iterations
      ),
    )?;

    // (adjusted_y_a, adjusted_y_b) =
    //   validate_ratio_b_to_a(&pair, querier, is_ww, &asset_b, swap_b_to_a, y_a, y_b, pool_ratio)?;

    messages.push(pair.swap_msg(
      &asset_b.info.with_balance(swap_b_to_a),
      None,
      Some(max_spread),
      None,
    )?);
  }

  Ok((swap_a_to_b, swap_b_to_a, adjusted_y_a, adjusted_y_b, iterations, difference))
}

// #[allow(clippy::too_many_arguments)]
// fn validate_ratio_b_to_a(
//   pair: &Pair,
//   querier: &QuerierWrapper<'_>,
//   is_ww: bool,
//   asset_b: &cw_asset::AssetBase<cosmwasm_std::Addr>,
//   swap_b_to_a: Uint128,
//   y_a: Uint128,
//   y_b: Uint128,
//   pool_ratio: Decimal256,
// ) -> Result<(Uint128, Uint128), ContractError> {
//   let s_a = pair
//     .query_simulate(querier, is_ww, asset_b.info.with_balance(swap_b_to_a), None)?
//     .return_amount;
//   let adjusted_y_a = y_a + s_a;
//   let adjusted_y_b = y_b - swap_b_to_a;
//   let adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);
//   assert_ratio(
//     adjusted_ratio,
//     pool_ratio,
//     format!(
//       "swapping {0} b to {1} a = {2} a + {3} b",
//       swap_b_to_a, s_a, adjusted_y_a, adjusted_y_b
//     ),
//   )?;
//   Ok((adjusted_y_a, adjusted_y_b))
// }

// #[allow(clippy::too_many_arguments)]
// fn validate_ratio_a_to_b(
//   pair: &Pair,
//   querier: &QuerierWrapper<'_>,
//   is_ww: bool,
//   asset_a: &cw_asset::AssetBase<cosmwasm_std::Addr>,
//   swap_a_to_b: Uint128,
//   y_a: Uint128,
//   y_b: Uint128,
//   pool_ratio: Decimal256,
// ) -> Result<(Uint128, Uint128), ContractError> {
//   let s_b = pair
//     .query_simulate(querier, is_ww, asset_a.info.with_balance(swap_a_to_b), None)?
//     .return_amount;
//   let adjusted_y_a = y_a - swap_a_to_b;
//   let adjusted_y_b = y_b + s_b;
//   let adjusted_ratio = Decimal256::from_ratio(adjusted_y_a, adjusted_y_b);
//   assert_ratio(
//     adjusted_ratio,
//     pool_ratio,
//     format!(
//       "swapping {0} a to {1} b = {2} a + {3} b",
//       swap_a_to_b, s_b, adjusted_y_a, adjusted_y_b
//     ),
//   )?;
//   Ok((adjusted_y_a, adjusted_y_b))
// }

fn assert_ratio(
  adjusted_ratio: Decimal256,
  pool_ratio: Decimal256,
  msg: String,
) -> Result<Decimal256, ContractError> {
  let diff = adjusted_ratio.abs_diff(pool_ratio);
  let difference = diff / pool_ratio;
  if difference > Decimal256::percent(1) {
    return Err(ContractError::OptimalSwapNoSolution {
      deposit: adjusted_ratio.to_string(),
      pool: pool_ratio.to_string(),
      msg,
    });
  }
  Ok(difference)
}
