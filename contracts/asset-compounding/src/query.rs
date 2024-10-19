use crate::state::{CONFIG, EXCHANGE_HISTORY};
use crate::{error::ContractError, state::asset_config_map};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Decimal, Deps, Env, Order, StdResult};
use cw_asset::AssetInfo;
use cw_storage_plus::Bound;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT, SECONDS_PER_DAY};
use ve3_shared::msgs_asset_compounding::{
  CompoundingAssetConfig, ExchangeHistory, ExchangeRatesResponse, QueryMsg, UserInfoResponse,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
    QueryMsg::AssetConfig {
      asset_info,
      gauge,
    } => Ok(to_json_binary(&asset_config_map().load(deps.storage, (&gauge, &asset_info))?)?),
    QueryMsg::UserInfos {
      assets,
      addr,
    } => Ok(to_json_binary(&query_user_infos(deps, env, addr, assets)?)?),
    QueryMsg::ExchangeRates {
      assets,
      start_after,
      limit,
    } => Ok(to_json_binary(&query_exchange_rates(deps, assets, start_after, limit)?)?),
  }
}

fn query_user_infos(
  deps: Deps,
  env: Env,
  addr: String,
  assets: Option<Vec<(String, AssetInfo)>>,
) -> Result<Vec<UserInfoResponse>, ContractError> {
  let mut results = vec![];

  let addr = deps.api.addr_validate(&addr)?;

  match assets {
    Some(assets) => {
      for (gauge, asset) in assets {
        let asset_config = asset_config_map().load(deps.storage, (&gauge, &asset))?;
        results.push(get_user_info(&deps, &env, asset_config, &addr)?);
      }
    },
    None => {
      for entry in asset_config_map().range(deps.storage, None, None, Order::Ascending) {
        let (_, asset_config) = entry?;
        results.push(get_user_info(&deps, &env, asset_config, &addr)?);
      }
    },
  }

  Ok(results)
}

fn get_user_info(
  deps: &Deps,
  env: &Env,
  asset_config: CompoundingAssetConfig,
  user: &Addr,
) -> Result<UserInfoResponse, ContractError> {
  let staked_balance = asset_config.staking.query_staked_balance(
    &deps.querier,
    &env.contract.address,
    asset_config.asset_info.clone(),
  )?;

  let total_amplp = deps.querier.query_supply(asset_config.amp_denom.clone())?.amount;
  let total_lp = staked_balance.asset.amount;

  let user_amplp = deps.querier.query_balance(user, asset_config.amp_denom)?.amount;
  let user_lp = total_lp.multiply_ratio(user_amplp, total_amplp);

  Ok(UserInfoResponse {
    gauge: asset_config.gauge,
    asset: asset_config.asset_info,
    total_lp,
    total_amplp,
    user_lp,
    user_amplp,
  })
}

fn query_exchange_rates(
  deps: Deps,
  assets: Option<Vec<(String, AssetInfo)>>,
  start_after: Option<u64>,
  limit: Option<u32>,
) -> Result<Vec<ExchangeRatesResponse>, ContractError> {
  let mut results = vec![];

  let assets = get_assets(deps, assets)?;

  for (gauge, asset) in assets {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_after.map(Bound::exclusive);
    let exchange_rates = EXCHANGE_HISTORY
      .prefix((&gauge, &asset))
      .range(deps.storage, None, end, Order::Descending)
      .take(limit)
      .collect::<StdResult<Vec<(u64, ExchangeHistory)>>>()?;

    let apr: Option<Decimal> = if exchange_rates.len() > 1 {
      let current = &exchange_rates[0].1;
      let last = &exchange_rates[exchange_rates.len() - 1].1;

      let delta_time_s = current.time_s - last.time_s;
      let delta_rate = current.exchange_rate.checked_sub(last.exchange_rate).unwrap_or_default();

      Some(delta_rate.checked_mul(
        Decimal::from_ratio(SECONDS_PER_DAY, delta_time_s).checked_div(last.exchange_rate)?,
      )?)
    } else {
      None
    };

    results.push(ExchangeRatesResponse {
      gauge,
      asset,
      exchange_rates,
      apr,
    });
  }

  Ok(results)
}

fn get_assets(
  deps: Deps,
  assets: Option<Vec<(String, AssetInfo)>>,
) -> Result<Vec<(String, AssetInfo)>, ContractError> {
  let assets = match assets {
    Some(assets) => assets,
    None => asset_config_map()
      .keys(deps.storage, None, None, Order::Ascending)
      .collect::<StdResult<Vec<_>>>()?,
  };
  Ok(assets)
}
