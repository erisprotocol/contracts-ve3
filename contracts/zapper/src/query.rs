use crate::error::ContractError;
use crate::state::{CONFIG, ROUTES};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, Env, Order, StdResult};
use cw_asset::AssetInfo;
use cw_storage_plus::Bound;
use ve3_shared::constants::{DEFAULT_LIMIT, MAX_LIMIT};
use ve3_shared::msgs_zapper::{QueryMsg, RouteResponseItem, SupportsSwapResponse};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
  match msg {
    QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),

    QueryMsg::GetRoutes {
      start_after,
      limit,
    } => Ok(to_json_binary(&get_routes(deps, start_after, limit)?)?),

    QueryMsg::GetRoute {
      from,
      to,
    } => Ok(to_json_binary(&get_route(deps, from, to)?)?),

    QueryMsg::SupportsSwap {
      from,
      to,
    } => Ok(to_json_binary(&query_supports_swap(deps, from, to)?)?),
  }
}

pub fn get_routes(
  deps: Deps,
  start_after: Option<(AssetInfo, AssetInfo)>,
  limit: Option<u32>,
) -> StdResult<Vec<RouteResponseItem>> {
  let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

  let owned: (String, String);

  let start = if let Some(start_after) = start_after {
    owned = (start_after.0.to_string(), start_after.1.to_string());
    Some(Bound::exclusive((owned.0, owned.1)))
  } else {
    None
  };

  ROUTES
    .range(deps.storage, start, None, Order::Ascending)
    .take(limit)
    .map(|item| {
      let (_, v) = item?;

      let from = v.stages[0].from.clone();
      let to = v.stages[v.stages.len() - 1].to.clone();

      Ok(RouteResponseItem {
        key: (from, to),
        stages: v.stages,
      })
    })
    .collect()
}

pub fn get_route(deps: Deps, from: AssetInfo, to: AssetInfo) -> StdResult<RouteResponseItem> {
  let key = (from.to_string(), to.to_string());
  let route_config = ROUTES.load(deps.storage, key)?;

  Ok(RouteResponseItem {
    key: (from, to),
    stages: route_config.stages,
  })
}

pub fn query_supports_swap(
  deps: Deps,
  from: AssetInfo,
  to: AssetInfo,
) -> StdResult<SupportsSwapResponse> {
  let suppored = if from == to {
    true
  } else {
    let key = (from.to_string(), to.to_string());
    ROUTES.has(deps.storage, key)
  };

  Ok(SupportsSwapResponse {
    suppored,
  })
}
