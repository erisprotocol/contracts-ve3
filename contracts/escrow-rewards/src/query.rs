use std::cmp::min;

use crate::{
  error::ContractError,
  state::{fetch_last_claimed, ClaimContext, BRIBE_AVAILABLE, BRIBE_CLAIMED, BRIBE_TOTAL, CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult};
use itertools::Itertools;
use ve3_shared::{
  adapters::asset_gauge::AssetGauge,
  error::SharedError,
  helpers::time::{GetPeriod, Time, Times},
  msgs_asset_gauge::UserShare,
  msgs_bribe_manager::{BribeBuckets, BribesResponse, NextClaimPeriodResponse, QueryMsg},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
    QueryMsg::NextClaimPeriod {
      user,
    } => Ok(to_json_binary(&next_claim_period(deps, env, user)?)?),
    QueryMsg::Bribes {
      period,
    } => Ok(to_json_binary(&bribes(deps, env, period)?)?),
    QueryMsg::UserClaimable {
      periods,
      user,
    } => Ok(to_json_binary(&user_claimable(deps, env, user, periods)?)?),
  }
}

fn next_claim_period(
  deps: Deps,
  env: Env,
  user: String,
) -> Result<NextClaimPeriodResponse, SharedError> {
  let user_addr = deps.api.addr_validate(&user)?;
  let block_period = Time::Current.get_period(&env)?;
  let asset_gauge = CONFIG.load(deps.storage)?.asset_gauge(&deps.querier)?;
  let period = _next_claim_period(&deps, &user_addr, block_period, &asset_gauge)?;

  Ok(NextClaimPeriodResponse {
    period,
  })
}

pub(crate) fn _next_claim_period(
  deps: &Deps,
  user: &Addr,
  block_period: u64,
  asset_gauge: &AssetGauge,
) -> Result<u64, SharedError> {
  // this queries the period when the user last claimed the bribes
  let last_claim = fetch_last_claimed(deps.storage, user.as_str(), block_period)?;

  let start = match last_claim {
    // start claiming from the next period
    Some((period, _)) => period + 1,
    // if not yet claimed, it queries the period of the first participation in the gauges
    None => match asset_gauge.query_first_participation(&deps.querier, user.clone())?.period {
      Some(period) => period,
      // if there is no participation, just start with the current block
      None => block_period,
    },
  };
  Ok(start)
}

pub(crate) fn _claim_periods(
  deps: &Deps,
  user: &Addr,
  periods: Option<Vec<u64>>,
  block_period: u64,
  asset_gauge: &AssetGauge,
) -> Result<Vec<u64>, ContractError> {
  let periods = match periods {
    Some(periods) => periods,
    None => {
      let start = _next_claim_period(deps, user, block_period, asset_gauge)?;
      let end = min(start + 101, block_period);
      let numbs = (start + 1)..end;
      numbs.collect()
    },
  };
  let periods: Vec<_> = periods.into_iter().sorted().take_while(|a| *a <= block_period).collect();
  Ok(periods)
}

fn bribes(deps: Deps, env: Env, time: Option<Time>) -> StdResult<BribesResponse> {
  let period = time.get_period(&env)?;
  let bribes = if let Some(bribes) = BRIBE_TOTAL.may_load(deps.storage, period)? {
    bribes
  } else {
    BRIBE_AVAILABLE.may_load(deps.storage, period)?.unwrap_or_default()
  };

  Ok(bribes)
}

fn user_claimable(
  deps: Deps,
  env: Env,
  user: String,
  periods: Option<Vec<u64>>,
) -> Result<BribesResponse, ContractError> {
  let user_addr = deps.api.addr_validate(&user)?;
  let block_period = Time::Current.get_period(&env)?;
  let asset_gauge = CONFIG.load(deps.storage)?.asset_gauge(&deps.querier)?;
  let periods = _claim_periods(&deps, &user_addr, periods, block_period, &asset_gauge)?;

  if periods.is_empty() {
    return Ok(BribeBuckets {
      buckets: vec![],
    });
  }

  let shares =
    asset_gauge.query_user_shares(&deps.querier, user_addr, Some(Times::Periods(periods)))?;

  let mut context = ClaimContext::default();
  let mut claimed = BribeBuckets::default();

  for share in shares.shares {
    // shares list sorted by period, each time we find a new one, context is updated.
    // starts with 0
    if share.period != context.period {
      let bribe_available = match BRIBE_AVAILABLE.may_load(deps.storage, share.period)? {
        Some(buckets) => buckets,
        None => {
          // if no bribes for the period, just skip till next period or end
          context = ClaimContext::default();
          context.period = share.period;
          context.skip = true;
          continue;
        },
      };

      // checking that not double claim
      if BRIBE_CLAIMED.has(deps.storage, (user.as_str(), share.period)) {
        context = ClaimContext::default();
        context.period = share.period;
        context.skip = true;
        continue;
      }

      let bribe_totals = match BRIBE_TOTAL.may_load(deps.storage, share.period)? {
        Some(buckets) => buckets,
        None => bribe_available,
      };

      context = ClaimContext {
        skip: false,
        period: share.period,
        bribe_totals,
        ..ClaimContext::default()
      };
    }

    if context.skip {
      // skip until a period with bribes is found again.
      continue;
    }

    let UserShare {
      gauge,
      asset,
      vp,
      total_vp,
      ..
    } = share;

    // see how much total bribe rewards for the asset in the gauge
    let total_bribe_bucket = context.bribe_totals.get(&gauge, &asset);
    // calculate the reward share based on the user vp compared to total vp
    let rewards = total_bribe_bucket.assets.calc_share_amounts(vp, total_vp)?;
    // add these rewards to the claimed bucket by the user
    claimed.get(&gauge, &asset).assets.add_multi(&rewards);
  }

  Ok(claimed)
}
