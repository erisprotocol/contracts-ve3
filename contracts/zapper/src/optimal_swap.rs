use crate::{
  constants::{COMMISSION_DEFAULT, COMMISSION_DENOM, DEFAULT_MAX_SPREAD},
  error::ContractError,
  optimal_swap_stable::calculate_optimal_swap_stableswap,
};
use cosmwasm_std::{
  attr, CosmosMsg, Decimal, Decimal256, DepsMut, Env, Isqrt, QuerierWrapper, Response, StdError,
  StdResult, Uint128, Uint256,
};
use cw_asset::Asset;
use ve3_shared::adapters::pair::{Pair, PairInfo, PairType};

/// # Description
/// Performs optimal swap of assets in the pair contract.
pub fn callback_optimal_swap(
  deps: DepsMut,
  env: Env,
  pair_info: PairInfo,
) -> Result<Response, ContractError> {
  let mut messages: Vec<CosmosMsg> = vec![];

  let mut attrs = vec![];
  match &pair_info.pair_type {
    PairType::Stable {} => {
      //Do nothing for stable pair
    },
    PairType::StableWhiteWhale {} => {
      let assets = pair_info.query_pools(&deps.querier, &env.contract.address)?;
      let asset_a = assets[0].clone();
      let asset_b = assets[1].clone();
      let max_spread = DEFAULT_MAX_SPREAD;
      if !asset_a.amount.is_zero() || !asset_b.amount.is_zero() {
        let (
          swap_asset_a_amount,
          swap_asset_b_amount,
          return_a_amount,
          return_b_amount,
          iterations,
          difference,
        ) = calculate_optimal_swap_stableswap(
          &deps.querier,
          &pair_info,
          asset_a,
          asset_b,
          &mut messages,
          max_spread,
        )?;

        attrs.push(attr("swap_asset_a_amount", swap_asset_a_amount));
        attrs.push(attr("swap_asset_b_amount", swap_asset_b_amount));
        attrs.push(attr("return_a_amount", return_a_amount));
        attrs.push(attr("return_b_amount", return_b_amount));
        attrs.push(attr("iterations", iterations.to_string()));
        attrs.push(attr("difference", difference.to_string()));
      }
    },
    PairType::Custom(custom) => {
      if custom == "concentrated" {
        //Do nothing for stable pair
      } else {
        let assets = pair_info.query_pools(&deps.querier, &env.contract.address)?;
        let asset_a = assets[0].clone();
        let asset_b = assets[1].clone();
        let max_spread = DEFAULT_MAX_SPREAD;
        if !asset_a.amount.is_zero() || !asset_b.amount.is_zero() {
          calculate_optimal_swap(
            &deps.querier,
            &pair_info,
            asset_a,
            asset_b,
            &mut messages,
            max_spread,
          )?;
        }
      }
    },
    _ => {
      let assets = pair_info.query_pools(&deps.querier, &env.contract.address)?;
      let asset_a = assets[0].clone();
      let asset_b = assets[1].clone();
      let max_spread = DEFAULT_MAX_SPREAD;
      if !asset_a.amount.is_zero() || !asset_b.amount.is_zero() {
        calculate_optimal_swap(
          &deps.querier,
          &pair_info,
          asset_a,
          asset_b,
          &mut messages,
          max_spread,
        )?;
      }
    },
  }

  Ok(
    Response::new()
      .add_messages(messages)
      .add_attribute("action", "zapper/optimal_swap")
      .add_attributes(attrs),
  )
}

/// # Description
/// Calculates the amount of asset in the pair contract that need to be swapped before providing liquidity.
/// The swap messages will be added to **messages**.
fn calculate_optimal_swap(
  querier: &QuerierWrapper,
  pair_info: &PairInfo,
  asset_a: Asset,
  asset_b: Asset,
  messages: &mut Vec<CosmosMsg>,
  max_spread: Decimal,
) -> Result<(Uint128, Uint128, Uint128, Uint128), ContractError> {
  let mut swap_asset_a_amount = Uint128::zero();
  let mut swap_asset_b_amount = Uint128::zero();
  let mut return_a_amount = Uint128::zero();
  let mut return_b_amount = Uint128::zero();

  let pair_contract = pair_info.contract_addr.clone();

  let provide_a_amount: Uint256 = asset_a.amount.into();
  let provide_b_amount: Uint256 = asset_b.amount.into();

  let pool_a_amount: Uint256 =
    pair_info.asset_infos[0].query_balance(querier, &pair_contract)?.into();
  let pool_b_amount: Uint256 =
    pair_info.asset_infos[1].query_balance(querier, &pair_contract)?.into();
  let provide_a_area = provide_a_amount * pool_b_amount;
  let provide_b_area = provide_b_amount * pool_a_amount;

  #[allow(clippy::comparison_chain)]
  if provide_a_area > provide_b_area {
    let swap_amount = get_swap_amount(
      provide_a_amount,
      provide_b_amount,
      pool_a_amount,
      pool_b_amount,
      COMMISSION_DEFAULT,
    )?;
    if !swap_amount.is_zero() {
      let swap_asset = Asset {
        info: asset_a.info,
        amount: swap_amount,
      };
      return_b_amount = simulate(
        pool_a_amount,
        pool_b_amount,
        swap_asset.amount.into(),
        Decimal256::from_ratio(COMMISSION_DEFAULT, COMMISSION_DENOM),
      )?;
      if !return_b_amount.is_zero() {
        swap_asset_a_amount = swap_asset.amount;
        messages.push(Pair(pair_contract).swap_msg(&swap_asset, None, Some(max_spread), None)?);
      }
    }
  } else if provide_a_area < provide_b_area {
    let swap_amount = get_swap_amount(
      provide_b_amount,
      provide_a_amount,
      pool_b_amount,
      pool_a_amount,
      COMMISSION_DEFAULT,
    )?;
    if !swap_amount.is_zero() {
      let swap_asset = Asset {
        info: asset_b.info,
        amount: swap_amount,
      };
      return_a_amount = simulate(
        pool_b_amount,
        pool_a_amount,
        swap_asset.amount.into(),
        Decimal256::from_ratio(COMMISSION_DEFAULT, COMMISSION_DENOM),
      )?;
      if !return_a_amount.is_zero() {
        swap_asset_b_amount = swap_asset.amount;
        messages.push(Pair(pair_contract).swap_msg(&swap_asset, None, Some(max_spread), None)?);
      }
    }
  };

  Ok((swap_asset_a_amount, swap_asset_b_amount, return_a_amount, return_b_amount))
}

/// Calculate swap amount
fn get_swap_amount(
  amount_a: Uint256,
  amount_b: Uint256,
  pool_a: Uint256,
  pool_b: Uint256,
  commission_bps: u64,
) -> StdResult<Uint128> {
  let pool_ax = amount_a + pool_a;
  let pool_bx = amount_b + pool_b;
  let area_ax = pool_ax * pool_b;
  let area_bx = pool_bx * pool_a;

  let a = Uint256::from(commission_bps * commission_bps) * area_ax
    + Uint256::from(4u64 * (COMMISSION_DENOM - commission_bps) * COMMISSION_DENOM) * area_bx;
  let b = Uint256::from(commission_bps) * area_ax + area_ax.isqrt() * a.isqrt();
  let result = (b / Uint256::from(2u64 * COMMISSION_DENOM) / pool_bx).saturating_sub(pool_a);

  result.try_into().map_err(|_| StdError::generic_err("overflow"))
}

/// Simulates return amount from the swap
fn simulate(
  offer_pool: Uint256,
  ask_pool: Uint256,
  offer_amount: Uint256,
  commission_rate: Decimal256,
) -> StdResult<Uint128> {
  // offer => ask
  // ask_amount = (ask_pool - cp / (offer_pool + offer_amount)) * (1 - commission_rate)
  let cp: Uint256 = offer_pool * ask_pool;
  let return_amount: Uint256 = (Decimal256::from_ratio(ask_pool, 1u64)
    - Decimal256::from_ratio(cp, offer_pool + offer_amount))
    * Uint256::from(1u64);

  // calculate commission
  let commission_amount: Uint256 = return_amount * commission_rate;

  // commission will be absorbed to pool
  let return_amount: Uint256 = return_amount - commission_amount;

  return_amount.try_into().map_err(|_| StdError::generic_err("overflow"))
}
