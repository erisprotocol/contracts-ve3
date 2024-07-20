use std::collections::{HashMap, HashSet};

use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION, DEFAULT_MAX_SPREAD, DEFAULT_SLIPPAGE},
  error::{ContractError, ContractResult},
  optimal_swap::callback_optimal_swap,
  state::{RouteConfig, CONFIG, ROUTES},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Expiration;
use cw_asset::{Asset, AssetInfo};
use ve3_shared::{
  adapters::{
    asset_staking::AssetStaking,
    global_config_adapter::ConfigExt,
    pair::{Pair, PairInfo},
  },
  error::SharedError,
  extensions::{asset_ext::AssetExt, asset_info_ext::AssetInfoExt},
  msgs_zapper::{
    CallbackMsg, Config, ExecuteMsg, InstantiateMsg, PostActionCreate, PostActionWithdraw, Stage,
    StageType,
  },
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    },
  )?;

  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::WithdrawLp {
      stage,
      min_received,
      post_action,
    } => {
      let pair_info = stage.get_pair_info(&deps.querier)?;

      let lp =
        pair_info.liquidity_token.with_balance_query(&deps.querier, &env.contract.address)?;

      if lp.amount.is_zero() {
        Err(SharedError::InsufficientBalance("no lp balance".to_string()))?;
      }

      let withdraw_lp_msg = Pair(pair_info.contract_addr).withdraw_liquidity_msg(lp)?;

      let msgs = match post_action {
        Some(PostActionWithdraw::SwapTo {
          min_received,
          asset,
          receiver,
        }) => {
          let mut callbacks =
            get_swap_stages(deps.storage, &pair_info.asset_infos, &vec![asset.clone()])?;

          if let Some(min_received) = min_received {
            callbacks.push(CallbackMsg::AssertReceived {
              asset: asset.with_balance(min_received),
            });
          }

          callbacks.push(CallbackMsg::SendResult {
            token: asset,
            receiver: receiver.unwrap_or(info.sender.to_string()),
          });

          callbacks
            .into_iter()
            .map(|a| a.into_cosmos_msg(&env.contract.address))
            .collect::<StdResult<Vec<_>>>()?
        },
        None => {
          // without a post action, just send the resulting assets to the caller
          let transfer_lp = CallbackMsg::SendResults {
            tokens: pair_info.asset_infos.clone(),
            receiver: info.sender.to_string(),
            min_received,
          }
          .into_cosmos_msg(&env.contract.address)?;
          vec![transfer_lp]
        },
      };

      Ok(
        Response::default()
          .add_message(withdraw_lp_msg)
          .add_messages(msgs)
          .add_attribute("action", "zapper/withdraw_lp"),
      )
    },
    ExecuteMsg::CreateLp {
      stage,
      assets,
      min_received,
      post_action,
    } => {
      assert_uniq_assets(&assets)?;

      let pair_info = stage.get_pair_info(&deps.querier)?;
      let lp_token = pair_info.liquidity_token.clone();

      let mut callbacks = get_swap_stages(deps.storage, &assets, &pair_info.asset_infos)?;

      callbacks.push(CallbackMsg::OptimalSwap {
        pair_info: pair_info.clone(),
      });

      callbacks.push(CallbackMsg::ProvideLiquidity {
        pair_info,
        receiver: None,
      });

      if let Some(min_received) = min_received {
        callbacks.push(CallbackMsg::AssertReceived {
          asset: lp_token.with_balance(min_received),
        });
      }

      callbacks.push(match post_action {
        Some(PostActionCreate::Stake {
          asset_staking,
          receiver,
        }) => CallbackMsg::Stake {
          asset_staking,
          token: lp_token,
          receiver: receiver.unwrap_or(info.sender.to_string()),
        },

        Some(PostActionCreate::SendResult {
          receiver,
        }) => CallbackMsg::SendResult {
          token: lp_token,
          receiver: receiver.unwrap_or(info.sender.to_string()),
        },

        None => CallbackMsg::SendResult {
          token: lp_token,
          receiver: info.sender.to_string(),
        },
      });

      let messages = callbacks
        .into_iter()
        .map(|a| a.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<_>>>()?;

      Ok(Response::default().add_messages(messages).add_attribute("action", "zapper/create_lp"))
    },

    ExecuteMsg::Swap {
      into,
      assets,
      min_received,
      receiver,
    } => {
      assert_uniq_assets(&assets)?;
      let to = into.check(deps.api, None)?;

      let mut callbacks: Vec<CallbackMsg> = vec![];
      for from in assets {
        let route = ROUTES.load(deps.storage, (from.to_string(), to.to_string()))?;

        callbacks.extend(route.stages.into_iter().map(|stage| CallbackMsg::SwapStage {
          stage,
        }));
      }

      if let Some(min_received) = min_received {
        callbacks.push(CallbackMsg::AssertReceived {
          asset: to.with_balance(min_received),
        })
      }

      callbacks.push(CallbackMsg::SendResult {
        token: to,
        receiver: receiver.unwrap_or(info.sender.to_string()),
      });

      let messages = callbacks
        .into_iter()
        .map(|a| a.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<_>>>()?;

      Ok(Response::default().add_messages(messages).add_attribute("action", "zapper/swap"))
    },
    ExecuteMsg::UpdateConfig {
      insert_routes,
      delete_routes,
    } => {
      let config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(insert_routes) = insert_routes {
        for route in insert_routes {
          let length = route.routes.len();
          for i in 0..length {
            for j in i..length {
              let stages = route.routes[i..=j].to_vec();

              let start = stages[0].from.clone();
              let end = stages[stages.len() - 1].to.clone();

              if start != end {
                ROUTES.save(
                  deps.storage,
                  (start.to_string(), end.to_string()),
                  &RouteConfig {
                    stages: stages.clone(),
                  },
                )?;

                let reversed = stages
                  .into_iter()
                  .rev()
                  .map(|mut item| {
                    std::mem::swap(&mut item.from, &mut item.to);
                    item
                  })
                  .collect::<Vec<_>>();

                ROUTES.save(
                  deps.storage,
                  (end.to_string(), start.to_string()),
                  &RouteConfig {
                    stages: reversed,
                  },
                )?
              }
            }
          }
        }
      }

      if let Some(delete_routes) = delete_routes {
        for route in delete_routes {
          let key = (route.from.to_string(), route.to.to_string());
          ROUTES.remove(deps.storage, key)
        }
      }

      Ok(Response::default().add_attribute("action", "zapper/update_config"))
    },
    ExecuteMsg::Callback(callback) => handle_callback(deps, env, info, callback),
  }
}

fn get_swap_stages(
  storage: &dyn Storage,
  from_assets: &Vec<AssetInfo>,
  to_assets: &Vec<AssetInfo>,
) -> Result<Vec<CallbackMsg>, ContractError> {
  let mut callbacks: Vec<CallbackMsg> = vec![];
  for from in from_assets {
    if to_assets.contains(from) {
      // can skip assets that are right already
      continue;
    }

    let mut shortest_route: Option<RouteConfig> = None;

    for to in to_assets {
      let route = ROUTES.may_load(storage, (from.to_string(), to.to_string()))?;

      match (&shortest_route, route) {
        (Some(current), Some(new)) => {
          if new.stages.len() < current.stages.len() {
            shortest_route = Some(new);
          }
        },
        (None, Some(new)) => shortest_route = Some(new),
        (Some(_), None) => (),
        (None, None) => (),
      }
    }

    match shortest_route {
      Some(route) => {
        callbacks.extend(route.stages.into_iter().map(|stage| CallbackMsg::SwapStage {
          stage,
        }))
      },

      None => {
        let to = to_assets.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",");

        Err(ContractError::NoRouteFound {
          from: from.clone(),
          to,
        })?
      },
    };
  }

  Ok(callbacks)
}

pub fn handle_callback(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: CallbackMsg,
) -> Result<Response, ContractError> {
  // Callback functions can only be called by this contract itself
  if info.sender != env.contract.address {
    Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
  }

  match msg {
    CallbackMsg::OptimalSwap {
      pair_info,
    } => callback_optimal_swap(deps, env, pair_info),

    CallbackMsg::ProvideLiquidity {
      pair_info,
      receiver,
    } => callback_provide_liquidity(deps, env, pair_info, receiver),

    CallbackMsg::SwapStage {
      stage,
    } => callback_swap(deps, env, stage),

    CallbackMsg::AssertReceived {
      asset,
    } => callback_assert_received(asset, deps, env),

    CallbackMsg::Stake {
      asset_staking,
      token,
      receiver,
    } => callback_stake_result(deps, env, info, token, asset_staking, receiver),

    CallbackMsg::SendResult {
      token,
      receiver,
    } => callback_send_result(deps, env, info, token, receiver),

    CallbackMsg::SendResults {
      tokens,
      receiver,
      min_received,
    } => callback_send_results(deps, env, info, tokens, receiver, min_received),
  }
}

fn callback_send_result(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  info: AssetInfo,
  receiver: String,
) -> Result<Response, ContractError> {
  let return_amount = info.with_balance_query(&deps.querier, &env.contract.address)?;
  let receiver = deps.api.addr_validate(&receiver)?;
  Ok(
    Response::new()
      .add_message(return_amount.transfer_msg(receiver)?)
      .add_attribute("action", "ampc/callback_send_result"),
  )
}
fn callback_send_results(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  tokens: Vec<AssetInfo>,
  receiver: String,
  min_received: Option<Vec<Asset>>,
) -> Result<Response, ContractError> {
  let receiver = deps.api.addr_validate(&receiver)?;
  let mut response = Response::new().add_attribute("action", "zapper/callback_send_results");

  let mut min_received_hashmap = min_received
    .unwrap_or_default()
    .into_iter()
    .map(|a| (a.info, a.amount))
    .collect::<HashMap<AssetInfo, Uint128>>();

  for token in tokens {
    let return_amount = token.with_balance_query(&deps.querier, &env.contract.address)?;
    if let Some(min_received) = min_received_hashmap.get(&token) {
      if return_amount.amount < *min_received {
        return Err(ContractError::AssertionFailed {
          actual: return_amount.amount,
          expected: *min_received,
        });
      }

      min_received_hashmap.remove(&token);
    }

    response = response
      .add_message(return_amount.transfer_msg(receiver.clone())?)
      .add_attribute("returned", return_amount.to_string())
  }

  if !min_received_hashmap.is_empty() {
    return Err(ContractError::ExpectingUnknownAssets());
  }

  Ok(response)
}

fn callback_stake_result(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  info: AssetInfo,
  asset_staking: Addr,
  receiver: String,
) -> Result<Response, ContractError> {
  let amount = info.with_balance_query(&deps.querier, &env.contract.address)?;

  Ok(
    Response::new()
      .add_message(AssetStaking(asset_staking).deposit_msg(amount, Some(receiver))?)
      .add_attribute("action", "zapper/callback_stake_result"),
  )
}

fn callback_assert_received(
  asset: cw_asset::AssetBase<Addr>,
  deps: DepsMut,
  env: Env,
) -> Result<Response, ContractError> {
  let balance = asset.info.query_balance(&deps.querier, env.contract.address)?;
  if balance < asset.amount {
    return Err(ContractError::AssertionFailed {
      actual: balance,
      expected: asset.amount,
    });
  }
  Ok(Response::default().add_attribute("action", "zapper/assert_received"))
}

fn callback_provide_liquidity(
  deps: DepsMut,
  env: Env,
  pair_info: PairInfo,
  receiver: Option<String>,
) -> Result<Response, ContractError> {
  let pair_contract = pair_info.contract_addr.clone();

  let assets = pair_info.query_pools(&deps.querier, &env.contract.address)?;

  let mut messages: Vec<CosmosMsg> = vec![];
  let mut provide_assets: Vec<Asset> = vec![];
  let mut funds: Vec<Coin> = vec![];
  for provide_asset in assets.iter() {
    provide_assets.push(provide_asset.clone());

    if !provide_asset.amount.is_zero() {
      match &provide_asset.info {
        cw_asset::AssetInfoBase::Native(denom) => funds.push(Coin {
          denom: denom.to_string(),
          amount: provide_asset.amount,
        }),
        cw_asset::AssetInfoBase::Cw20(_) => messages.push(provide_asset.increase_allowance_msg(
          pair_contract.to_string(),
          Some(Expiration::AtHeight(env.block.height + 1)),
        )?),
        _ => Err(SharedError::NotSupportedAssetInfo())?,
      }
    }
  }

  let provide_liquidity = Pair(pair_contract).provide_liquidity_msg(
    provide_assets,
    Some(DEFAULT_SLIPPAGE),
    receiver,
    funds,
  )?;
  messages.push(provide_liquidity);

  Ok(Response::new().add_messages(messages).add_attribute("action", "zapper/provide_liquidity"))
}

fn callback_swap(deps: DepsMut, env: Env, stage: Stage) -> Result<Response, ContractError> {
  let from_asset = stage.from.with_balance_query(&deps.querier, &env.contract.address)?;

  if from_asset.amount.is_zero() {
    return Ok(
      Response::new()
        .add_attribute("action", "zapper/execute_swap_noop")
        .add_attribute("asset", from_asset.info.to_string()),
    );
  }

  let msg = match stage.stage_type {
    StageType::WhiteWhale {
      pair,
    }
    | StageType::Astroport {
      pair,
    } => Pair(pair).swap_msg(&from_asset, None, Some(DEFAULT_MAX_SPREAD), None)?,
  };

  Ok(Response::new().add_message(msg).add_attribute("action", "zapper/execute_swap"))
}

pub fn assert_uniq_assets(assets: &[AssetInfo]) -> StdResult<()> {
  let mut uniq = HashSet::new();
  if !assets.iter().all(|a| uniq.insert(a.to_string())) {
    return Err(StdError::generic_err("duplicated asset"));
  }

  Ok(())
}
