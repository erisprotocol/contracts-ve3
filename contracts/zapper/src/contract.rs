use std::collections::{HashMap, HashSet};

use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION, DEFAULT_MAX_SPREAD, DEFAULT_SLIPPAGE},
  error::{ContractError, ContractResult},
  optimal_swap::callback_optimal_swap,
  state::{RouteConfig, TokenConfig, CONFIG, ROUTES, TOKEN_CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::{Cw20QueryMsg, Expiration, MinterResponse};
use cw_asset::{Asset, AssetError, AssetInfo, AssetInfoBase};
use ve3_shared::{
  adapters::{
    asset_staking::AssetStaking,
    compounder::Compounder,
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

  let center_asset_infos = msg
    .center_asset_infos
    .into_iter()
    .map(|a| a.check(deps.api, None))
    .collect::<Result<Vec<_>, AssetError>>()?;

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      center_asset_infos,
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
            get_swap_stages(deps.storage, &pair_info.asset_infos, &vec![asset.clone()], true)?;

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
      let pair_info = stage.get_pair_info(&deps.querier)?;
      let lp_token = pair_info.liquidity_token.clone();

      let messages = zap(deps, env, info, lp_token, assets, min_received, post_action)?;

      Ok(Response::default().add_messages(messages).add_attribute("action", "zapper/create_lp"))
    },

    ExecuteMsg::Zap {
      into,
      assets,
      min_received,
      post_action,
    } => {
      let into = into.check(deps.api, None)?;
      let messages = zap(deps, env, info, into, assets, min_received, post_action)?;
      Ok(Response::default().add_messages(messages).add_attribute("action", "zapper/zap"))
    },

    ExecuteMsg::Swap {
      into,
      assets,
      min_received,
      receiver,
    } => {
      let into = into.check(deps.api, None)?;
      let messages = zap(
        deps,
        env,
        info,
        into,
        assets,
        min_received,
        Some(PostActionCreate::SendResult {
          receiver,
        }),
      )?;

      Ok(Response::default().add_messages(messages).add_attribute("action", "zapper/swap"))
    },
    ExecuteMsg::UpdateConfig {
      insert_routes,
      delete_routes,
      update_centers,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
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

      if let Some(update_centers) = update_centers {
        let centers = update_centers
          .into_iter()
          .map(|a| a.check(deps.api, None))
          .collect::<Result<Vec<_>, AssetError>>()?;

        config.center_asset_infos = centers;

        CONFIG.save(deps.storage, &config)?;
      }

      Ok(Response::default().add_attribute("action", "zapper/update_config"))
    },
    ExecuteMsg::Callback(callback) => handle_callback(deps, env, info, callback),
  }
}

fn zap(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  into: AssetInfo,
  assets: Vec<AssetInfo>,
  min_received: Option<Uint128>,
  post_action: Option<PostActionCreate>,
) -> Result<Vec<CosmosMsg>, ContractError> {
  assert_uniq_assets(&assets)?;

  // println!(
  //   "ZAP {0}->{1}",
  //   assets.clone().into_iter().map(|a| a.to_string()).collect::<Vec<_>>().join(","),
  //   into
  // );

  let no_swap_required = assets.len() == 1 && assets[0] == into;
  let mut callbacks = if no_swap_required {
    vec![]
  } else {
    let token_config = get_token_config(&mut deps, into.clone())?;

    match token_config {
      TokenConfig::TargetPair(stage) => {
        let pair_info = stage.get_pair_info(&deps.querier)?;
        let mut pair_msgs = get_swap_stages(deps.storage, &assets, &pair_info.asset_infos, true)?;

        pair_msgs.push(CallbackMsg::OptimalSwap {
          pair_info: pair_info.clone(),
        });

        pair_msgs.push(CallbackMsg::ProvideLiquidity {
          pair_info,
          receiver: None,
        });
        pair_msgs
      },
      TokenConfig::TargetSwap => get_swap_stages(deps.storage, &assets, &vec![into.clone()], true)?,
    }
  };

  if let Some(min_received) = min_received {
    callbacks.push(CallbackMsg::AssertReceived {
      asset: into.with_balance(min_received),
    });
  }

  callbacks.push(match post_action {
    Some(PostActionCreate::Stake {
      asset_staking,
      receiver,
    }) => CallbackMsg::Stake {
      asset_staking,
      token: into,
      receiver: receiver.unwrap_or(info.sender.to_string()),
    },

    Some(PostActionCreate::LiquidStake {
      compounder,
      gauge,
      receiver,
    }) => CallbackMsg::LiquidStake {
      token: into,
      compounder,
      gauge,
      receiver: receiver.unwrap_or(info.sender.to_string()),
    },

    Some(PostActionCreate::SendResult {
      receiver,
    }) => CallbackMsg::SendResult {
      token: into,
      receiver: receiver.unwrap_or(info.sender.to_string()),
    },

    None => CallbackMsg::SendResult {
      token: into,
      receiver: info.sender.to_string(),
    },
  });

  let messages = callbacks
    .into_iter()
    .map(|a| a.into_cosmos_msg(&env.contract.address))
    .collect::<StdResult<Vec<_>>>()?;

  Ok(messages)
}

fn get_token_config(
  deps: &mut DepsMut,
  lp: AssetInfoBase<Addr>,
) -> Result<TokenConfig, ContractError> {
  if let Some(lp_config) = TOKEN_CONFIG.may_load(deps.storage, &lp)? {
    Ok(lp_config)
  } else {
    // check if we can find a pair for an LP address
    let potential_pair_addr = match &lp {
      AssetInfoBase::Native(native) => {
        if native.starts_with("factory/") {
          let contract = native.split('/').take(2).collect::<Vec<&str>>()[1];
          Some(deps.api.addr_validate(contract)?)
        } else {
          None
        }
      },
      AssetInfoBase::Cw20(cw20) => {
        let minter: MinterResponse =
          deps.querier.query_wasm_smart(cw20, &Cw20QueryMsg::Minter {})?;
        Some(deps.api.addr_validate(&minter.minter)?)
      },
      _ => Err(SharedError::NotSupportedAssetInfo())?,
    };

    // check the found pair address if it is really a pair
    let token_config = match potential_pair_addr {
      Some(pair_addr) => {
        let pair = Pair(pair_addr.clone());
        if pair.query_ww_pair_info(&deps.querier).is_ok() {
          TokenConfig::TargetPair(StageType::WhiteWhale {
            pair: pair_addr,
          })
        } else if pair.query_astroport_pair_info(&deps.querier).is_ok() {
          TokenConfig::TargetPair(StageType::Astroport {
            pair: pair_addr,
          })
        } else {
          TokenConfig::TargetSwap
        }
      },
      None => TokenConfig::TargetSwap,
    };

    TOKEN_CONFIG.save(deps.storage, &lp, &token_config)?;

    // println!("token_config {:?}", token_config);
    Ok(token_config)
  }
}

/// finding the shortest path, from each of the from assets to a SINGLE to asset.
fn get_swap_stages(
  storage: &dyn Storage,
  from_assets: &Vec<AssetInfo>,
  to_assets: &Vec<AssetInfo>,
  check_centers: bool,
) -> Result<Vec<CallbackMsg>, ContractError> {
  let mut stages: Vec<Stage> = vec![];
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

    if shortest_route.is_none() && check_centers {
      let config = CONFIG.load(storage)?;

      for center in config.center_asset_infos {
        match (
          get_swap_stages(storage, from_assets, &vec![center.clone()], false),
          get_swap_stages(storage, &vec![center], to_assets, false),
        ) {
          (Ok(segment1), Ok(segment2)) => {
            let stages = [segment1, segment2]
              .concat()
              .into_iter()
              .filter_map(|a| match a {
                CallbackMsg::SwapStage {
                  stage,
                } => Some(stage),
                _ => None,
              })
              .collect::<Vec<_>>();

            let route = RouteConfig {
              stages,
            };

            match &shortest_route {
              Some(current) => {
                if route.stages.len() < current.stages.len() {
                  shortest_route = Some(route);
                }
              },
              None => shortest_route = Some(route),
            }

            break;
          },
          _ => continue,
        }
      }
    }

    match shortest_route {
      Some(route) => stages.extend(route.stages),

      None => {
        let to = to_assets.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",");

        Err(ContractError::NoRouteFound {
          from: from.clone(),
          to,
        })?
      },
    };
  }

  // OPTIMIZING STAGES
  let stages = if from_assets.len() > 1 {
    let mut result = vec![];

    stages.reverse();

    let mut ignored_denoms: HashSet<AssetInfoBase<Addr>> = HashSet::new();
    let mut searched_to: HashSet<AssetInfoBase<Addr>> = HashSet::from_iter(to_assets.clone());

    while !stages.is_empty() {
      for stage in stages.clone().into_iter() {
        let mut remove = false;

        if ignored_denoms.contains(&stage.from)
          || (searched_to.contains(&stage.from) && searched_to.contains(&stage.to))
        {
          remove = true;
        }

        if searched_to.contains(&stage.to) && !remove {
          remove = true;

          ignored_denoms.insert(stage.from.clone());
          ignored_denoms.insert(stage.to.clone());
          searched_to.insert(stage.from.clone());
          result.push(stage.clone());
        }

        if remove {
          stages.retain(|a| a != &stage);
        }
      }
    }

    result.reverse();
    result
  } else {
    stages
  };

  let callbacks = stages
    .into_iter()
    .map(|stage| CallbackMsg::SwapStage {
      stage,
    })
    .collect::<Vec<_>>();

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
    CallbackMsg::LiquidStake {
      compounder,
      gauge,
      token,
      receiver,
    } => callback_liquid_stake_result(deps, env, info, token, compounder, gauge, receiver),

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
      .add_attribute("action", "zapper/callback_send_result")
      .add_attribute("amount", return_amount.to_string()),
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

fn callback_liquid_stake_result(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  info: AssetInfo,
  compounder: Addr,
  gauge: String,
  receiver: String,
) -> Result<Response, ContractError> {
  let asset = info.with_balance_query(&deps.querier, &env.contract.address)?;

  Ok(
    Response::new()
      .add_message(Compounder(compounder).deposit_msg(asset, gauge, Some(receiver))?)
      .add_attribute("action", "zapper/callback_liquid_stake_result"),
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
