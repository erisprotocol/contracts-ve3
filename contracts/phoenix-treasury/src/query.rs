#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Addr, Binary, Decimal, Deps, Env, Order, StdError};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
use cw_storage_plus::Bound;
use ve3_shared::{
  adapters::{pair::Pair, router::Router},
  constants::{DEFAULT_LIMIT, MAX_LIMIT},
  extensions::asset_info_ext::AssetInfoExt,
  helpers::assets::Assets,
  msgs_phoenix_treasury::{
    BalancesResponse, Direction, Oracle, OraclesResponse, QueryMsg, TreasuryAction,
  },
};

use crate::{
  error::ContractError,
  state::{ACTIONS, CONFIG, ORACLES, STATE, USER_ACTIONS, VALIDATORS},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => get_config(deps),
    QueryMsg::State {} => get_state(deps, env),
    QueryMsg::Validators {} => get_validators(deps),
    QueryMsg::Actions {
      limit,
      start_after,
      direction,
    } => get_actions(deps, start_after, limit, direction),
    QueryMsg::Action {
      id,
    } => get_action(deps, id),
    QueryMsg::UserActions {
      user,
      limit,
      start_after,
    } => get_user_actions(deps, user, start_after, limit),
    QueryMsg::Balances {
      assets,
    } => get_balances(deps, env, assets),
    QueryMsg::OraclePrices {
      assets,
    } => get_oracle_prices(deps, env, assets),
  }
}

fn get_config(deps: Deps) -> Result<Binary, ContractError> {
  let res = CONFIG.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}

fn get_state(deps: Deps, _env: Env) -> Result<Binary, ContractError> {
  let res = STATE.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}

fn get_validators(deps: Deps) -> Result<Binary, ContractError> {
  let res = VALIDATORS.load(deps.storage)?;
  Ok(to_json_binary(&res)?)
}

fn get_actions(
  deps: Deps,
  start_after: Option<u64>,
  limit: Option<u32>,
  direction: Option<Direction>,
) -> Result<Binary, ContractError> {
  let direction = direction.unwrap_or(Direction::Asc);
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let start_after = start_after.map(Bound::exclusive);

  let actions: Vec<TreasuryAction> = match direction {
    Direction::Asc => ACTIONS
      .range(deps.storage, start_after, None, Order::Ascending)
      .map(|a| {
        let (_, a) = a?;
        Ok(a)
      })
      .take(limit)
      .collect::<Result<Vec<_>, ContractError>>()?,
    Direction::Desc => ACTIONS
      .range(deps.storage, None, start_after, Order::Descending)
      .map(|a| {
        let (_, a) = a?;
        Ok(a)
      })
      .take(limit)
      .collect::<Result<Vec<_>, ContractError>>()?,
  };

  Ok(to_json_binary(&actions)?)
}
fn get_action(deps: Deps, id: u64) -> Result<Binary, ContractError> {
  let action: TreasuryAction = ACTIONS.load(deps.storage, id)?;
  Ok(to_json_binary(&action)?)
}

fn get_user_actions(
  deps: Deps,
  user: String,
  start_after: Option<u64>,
  limit: Option<u32>,
) -> Result<Binary, ContractError> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
  let start_after = start_after.map(Bound::exclusive);

  let addr = deps.api.addr_validate(&user)?;

  let actions: Vec<TreasuryAction> = USER_ACTIONS
    .prefix(&addr)
    .range(deps.storage, None, start_after, Order::Descending)
    .map(|a| {
      let action = ACTIONS.load(deps.storage, a?.0)?;
      Ok(action)
    })
    .take(limit)
    .collect::<Result<Vec<_>, ContractError>>()?;

  Ok(to_json_binary(&actions)?)
}

fn get_balances(
  deps: Deps,
  env: Env,
  assets: Option<Vec<AssetInfoUnchecked>>,
) -> Result<Binary, ContractError> {
  let state = STATE.load(deps.storage)?;
  let config = CONFIG.load(deps.storage)?;

  let total_assets: Vec<Asset> = match assets {
    Some(assets) => assets
      .into_iter()
      .map(|a| {
        Ok(a.check(deps.api, None)?.with_balance_query(&deps.querier, &env.contract.address)?)
      })
      .collect::<Result<Vec<Asset>, ContractError>>()?,
    None => {
      deps.querier.query_all_balances(env.contract.address)?.into_iter().map(|a| a.into()).collect()
    },
  };

  let mut available: Assets = total_assets.into();
  for reserved in &state.reserved.0 {
    if let Some(available) = available.get_mut(&reserved.info) {
      available.amount = available.amount.checked_sub(reserved.amount)?;
    }
  }

  // alliance denom should be ignored
  let alliance_token = AssetInfo::native(config.alliance_token_denom.clone());
  if let Some(x) = available.get(&alliance_token) {
    available.remove(&x)?;
  }

  Ok(to_json_binary(&BalancesResponse {
    reserved: state.reserved,
    available,
  })?)
}

fn get_oracle_prices(
  deps: Deps,
  _env: Env,
  assets: Option<Vec<AssetInfoUnchecked>>,
) -> Result<Binary, ContractError> {
  let oracles = match assets {
    Some(assets) => {
      let mut result = vec![];
      for asset in assets {
        let asset = asset.check(deps.api, None)?;
        result.push((asset.clone(), ORACLES.load(deps.storage, &asset)?));
      }
      result
    },
    None => ORACLES
      .range(deps.storage, None, None, Order::Ascending)
      .collect::<Result<Vec<(AssetInfo, Oracle<Addr>)>, StdError>>()?,
  };

  let mut prices: OraclesResponse = vec![];

  for (info, oracle) in oracles {
    let price = match oracle {
      Oracle::Usdc => Decimal::one(),
      Oracle::Pair {
        contract,
        simulation_amount,
        from_decimals,
      } => {
        let result = Pair(contract).query_simulate(
          &deps.querier,
          info.with_balance(simulation_amount),
          None,
        )?;

        Decimal::from_ratio(result.return_amount, simulation_amount)
          * Decimal::from_ratio(u32::pow(10, from_decimals.unwrap_or(6)), u32::pow(10, 6))
      },

      Oracle::Route {
        contract,
        path,
        simulation_amount,
        from_decimals,
      } => {
        let result = Router(contract).query_simulate(
          &deps.querier,
          info.with_balance(simulation_amount),
          path,
        )?;

        Decimal::from_ratio(result.amount, simulation_amount)
          * Decimal::from_ratio(u32::pow(10, from_decimals.unwrap_or(6)), u32::pow(10, 6))
      },
      Oracle::RouteAsset {
        contract,
        path,
        simulation_amount,
        from_decimals,
      } => {
        let result = Router(contract).query_simulate(
          &deps.querier,
          simulation_amount.clone(),
          path,
        )?;

        Decimal::from_ratio(result.amount, simulation_amount.amount)
          * Decimal::from_ratio(u32::pow(10, from_decimals.unwrap_or(6)), u32::pow(10, 6))
      },
    };

    prices.push((info, price));
  }

  Ok(to_json_binary(&prices)?)
}
