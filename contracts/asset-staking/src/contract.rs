use std::cmp::min;

use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::state::{
  ASSET_BRIBES, ASSET_CONFIG, ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, CONFIG, SHARES, TOTAL,
  UNCLAIMED_REWARDS, USER_ASSET_REWARD_RATE, WHITELIST,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  attr, from_json, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdResult,
  Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetError, AssetInfo};
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::{
  AT_ASSET_GAUGE, AT_ASSET_WHITELIST_CONTROLLER, AT_TAKE_RECIPIENT, SECONDS_PER_YEAR,
};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::env_ext::EnvExt;
use ve3_shared::helpers::assets::Assets;
use ve3_shared::helpers::general::addr_opt_fallback;
use ve3_shared::helpers::take::{compute_balance_amount, compute_share_amount};
use ve3_shared::msgs_asset_staking::{
  AssetConfig, AssetConfigRuntime, AssetDistribution, AssetInfoWithConfig, CallbackMsg, Config,
  Cw20HookMsg, ExecuteMsg, InstantiateMsg,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let config = Config {
    reward_info: msg.reward_info.check(deps.api, None)?,
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    default_yearly_take_rate: msg.default_yearly_take_rate,
    gauge: msg.gauge,
  };
  CONFIG.save(deps.storage, &config)?;
  Ok(Response::new().add_attributes(vec![("action", "asset/instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> Result<Response, ContractError> {
  match msg {
    // user
    ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
    ExecuteMsg::Stake {
      recipient,
    } => {
      if info.funds.len() != 1 {
        return Err(ContractError::OnlySingleAssetAllowed {});
      }
      if info.funds[0].amount.is_zero() {
        return Err(ContractError::AmountCannotBeZero {});
      }
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender)?;
      let asset = AssetInfo::native(&info.funds[0].denom);
      stake(deps, env, asset, info.funds[0].amount, recipient)
    },
    ExecuteMsg::Unstake {
      asset,
      recipient,
    } => {
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender.clone())?;
      unstake(deps, env, info, asset, recipient)
    },
    ExecuteMsg::ClaimReward(asset) => claim_rewards(deps, info, Some(vec![asset])),
    ExecuteMsg::ClaimRewards {
      assets,
    } => claim_rewards(deps, info, assets),

    // bot
    ExecuteMsg::UpdateRewards {} => update_rewards(deps, env, info),
    ExecuteMsg::DistributeTakeRate {
      update,
      assets,
    } => distribute_take_rate(deps, env, info, update, assets),

    ExecuteMsg::DistributeBribes {
      update,
      assets,
    } => distribute_bribes(deps, env, info, update, assets),

    // controller
    ExecuteMsg::WhitelistAssets(assets) => whitelist_assets(deps, env, info, assets),
    ExecuteMsg::RemoveAssets(assets) => remove_assets(deps, info, assets),
    ExecuteMsg::UpdateAssetConfig(update) => update_asset_config(deps, env, info, update),
    ExecuteMsg::SetAssetRewardDistribution(asset_reward_distribution) => {
      set_asset_reward_distribution(deps, info, asset_reward_distribution)
    },

    // contract
    ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),
  }
}

fn callback(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: CallbackMsg,
) -> Result<Response, ContractError> {
  if env.contract.address != info.sender {
    Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
  }

  match msg {
    CallbackMsg::UpdateRewards {
      initial_balance,
    } => update_reward_callback(deps, env, info, initial_balance),
    CallbackMsg::TrackBribes {
      for_asset: asset,
      initial_balances,
    } => track_bribes_callback(deps, env, info, asset, initial_balances),
    CallbackMsg::DistributeBribes {
      assets,
    } => distribute_bribes_callback(deps, env, info, assets),
  }
}

// receive_cw20 routes a cw20 token to the proper handler in this case stake and unstake
fn receive_cw20(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
  let sender = deps.api.addr_validate(&cw20_msg.sender)?;

  match from_json(&cw20_msg.msg)? {
    Cw20HookMsg::Stake {
      recipient,
    } => {
      if cw20_msg.amount.is_zero() {
        return Err(ContractError::AmountCannotBeZero {});
      }
      let asset = AssetInfo::Cw20(info.sender.clone());
      let recipient = addr_opt_fallback(deps.api, &recipient, sender)?;
      stake(deps, env, asset, cw20_msg.amount, recipient)
    },
  }
}

fn set_asset_reward_distribution(
  deps: DepsMut,
  info: MessageInfo,
  asset_reward_distribution: Vec<AssetDistribution>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_distribution_controller(&deps, &info, &config)?;

  // Ensure the dsitributions add up to 100%
  let total_distribution = asset_reward_distribution
    .iter()
    .map(|a| a.distribution)
    .fold(Decimal::zero(), |acc, v| acc + v);

  if total_distribution != Decimal::percent(100) {
    return Err(ContractError::InvalidDistribution {});
  }

  // Simply set the asset_reward_distribution, overwriting any previous settings.
  // This means any updates should include the full existing set of AssetDistributions and not just the newly updated one.
  ASSET_REWARD_DISTRIBUTION.save(deps.storage, &asset_reward_distribution)?;
  Ok(Response::new().add_attributes(vec![("action", "asset/set_asset_reward_distribution")]))
}

fn update_asset_config(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  update: AssetInfoWithConfig<String>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let update = update.check(deps.api)?;
  assert_asset_whitelist_controller(&deps, &info, &config)?;
  assert_asset_whitelisted(&deps, &update.info)?;

  let msgs = _update_asset_config(&mut deps, &env, &update, &config)?;

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "asset/update_asset_config"),
        ("asset", &update.info.to_string()),
      ])
      .add_messages(msgs),
  )
}

fn _update_asset_config(
  deps: &mut DepsMut,
  env: &Env,
  update: &AssetInfoWithConfig<Addr>,
  config: &Config,
) -> Result<Vec<CosmosMsg>, ContractError> {
  let current = ASSET_CONFIG.may_load(deps.storage, &update.info)?.unwrap_or_default();
  let mut updated = current.clone();
  let new_config = update.config.clone().unwrap_or(AssetConfig {
    yearly_take_rate: Some(config.default_yearly_take_rate),
    stake_config: ve3_shared::stake_config::StakeConfig::Default,
  });
  updated.stake_config = new_config.stake_config;
  updated.yearly_take_rate = new_config.yearly_take_rate.unwrap_or(config.default_yearly_take_rate);

  if updated.yearly_take_rate > Decimal::percent(50) {
    return Err(ContractError::TakeRateLessOrEqual50);
  }

  ASSET_CONFIG.save(deps.storage, &update.info, &updated)?;
  let mut msgs = vec![];
  if current.stake_config != updated.stake_config {
    // if stake config changed, withdraw from one (or do nothing), deposit on the other.
    let (balance, _) = TOTAL.may_load(deps.storage, &update.info)?.unwrap_or_default();
    let in_contract = balance - current.harvested;
    let asset = update.info.with_balance(in_contract);

    let mut unstake_msgs =
      current.stake_config.unstake_check_received_msg(deps, env, asset.clone())?;
    let mut stake_msgs = updated.stake_config.stake_check_received_msg(deps, env, asset)?;

    msgs.append(&mut unstake_msgs);
    msgs.append(&mut stake_msgs);
  }

  Ok(msgs)
}

fn whitelist_assets(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  asset_configs: Vec<AssetInfoWithConfig<String>>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_asset_whitelist_controller(&deps, &info, &config)?;

  let asset_configs =
    asset_configs.into_iter().map(|a| a.check(deps.api)).collect::<Result<Vec<_>, AssetError>>()?;

  let assets_str =
    asset_configs.iter().map(|asset| asset.info.to_string()).collect::<Vec<String>>().join(",");

  let mut response = Response::new()
    .add_attributes(vec![("action", "asset/whitelist_assets"), ("assets", &assets_str)]);

  for asset_config in asset_configs {
    if asset_config.info == config.reward_info {
      return Err(ContractError::AssetInfoCannotEqualReward {});
    }

    if WHITELIST.has(deps.storage, &asset_config.info) {
      return Err(ContractError::AssetAlreadyWhitelisted);
    }

    WHITELIST.save(deps.storage, &asset_config.info, &true)?;
    ASSET_REWARD_RATE.update(deps.storage, &asset_config.info, |rate| -> StdResult<_> {
      Ok(rate.unwrap_or(Decimal::zero()))
    })?;

    let msgs = _update_asset_config(&mut deps, &env, &asset_config, &config)?;

    response = response.add_messages(msgs);
  }

  Ok(response)
}

fn remove_assets(
  deps: DepsMut,
  info: MessageInfo,
  assets: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  // Only allow the governance address to update whitelisted assets
  assert_asset_whitelist_controller(&deps, &info, &config)?;
  for asset in &assets {
    WHITELIST.remove(deps.storage, asset);
  }
  let assets_str = assets.iter().map(|asset| asset.to_string()).collect::<Vec<String>>().join(",");
  Ok(
    Response::new()
      .add_attributes(vec![("action", "asset/remove_assets"), ("assets", &assets_str)]),
  )
}

fn stake(
  mut deps: DepsMut,
  env: Env,
  asset: AssetInfo,
  amount: Uint128,
  recipient: Addr,
) -> Result<Response, ContractError> {
  assert_asset_whitelisted(&deps, &asset)?;

  let rewards = _calc_reward_share(deps.storage, recipient.clone(), asset.clone())?;
  if !rewards.is_zero() {
    UNCLAIMED_REWARDS.update(
      deps.storage,
      (recipient.clone(), &asset),
      |balance| -> Result<_, ContractError> { Ok(balance.unwrap_or(Uint128::zero()) + rewards) },
    )?;
  }

  let (balance, shares) = TOTAL.may_load(deps.storage, &asset)?.unwrap_or_default();
  let (asset_config, asset_available) = _take(&mut deps, &env, &asset, balance, true)?;
  let share_amount = compute_share_amount(shares, amount, asset_available);

  SHARES.update(
    deps.storage,
    (recipient.clone(), &asset),
    |share| -> Result<_, ContractError> {
      Ok(share.unwrap_or_default().checked_add(share_amount)?)
    },
  )?;

  TOTAL.save(
    deps.storage,
    &asset,
    &(balance.checked_add(amount)?, shares.checked_add(share_amount)?),
  )?;

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "asset/stake"),
        ("user", recipient.as_ref()),
        ("asset", &asset.to_string()),
        ("amount", &amount.to_string()),
        ("share", &share_amount.to_string()),
      ])
      .add_messages(asset_config.stake_config.stake_check_received_msg(
        &deps,
        &env,
        asset.with_balance(amount),
      )?),
  )
}

fn _take(
  deps: &mut DepsMut,
  env: &Env,
  asset: &AssetInfo,
  balance: Uint128,
  save_config: bool,
) -> Result<(AssetConfigRuntime, Uint128), ContractError> {
  // balance includes the full balance including already taken out take rate
  let config = ASSET_CONFIG.may_load(deps.storage, asset)?;

  if let Some(mut config) = config {
    if config.yearly_take_rate.is_zero() {
      let available = balance.checked_sub(config.taken)?;
      return Ok((config, available));
    }

    // only take if last taken set
    if config.last_taken_s != 0 {
      let take_diff_s = Uint128::new((env.block.time.seconds() - config.last_taken_s).into());
      // total_balance * yearly_take_rate * taken_diff_s / SECONDS_PER_YEAR

      let relevant_balance = balance.saturating_sub(config.taken);
      let take_amount = config.yearly_take_rate
        * relevant_balance.multiply_ratio(
          min(take_diff_s, Uint128::new(SECONDS_PER_YEAR.into())),
          SECONDS_PER_YEAR,
        );

      config.taken = config.taken.checked_add(take_amount)?;
    }

    config.last_taken_s = env.block.time.seconds();

    if save_config {
      ASSET_CONFIG.save(deps.storage, asset, &config)?;
    }

    let available = balance.checked_sub(config.taken)?;
    return Ok((config, available));
  }

  Ok((AssetConfigRuntime::default(), balance))
}

fn unstake(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  asset: Asset,
  recipient: Addr,
) -> Result<Response, ContractError> {
  let sender = info.sender.clone();
  if asset.amount.is_zero() {
    return Err(ContractError::AmountCannotBeZero {});
  }

  let rewards = _calc_reward_share(deps.storage, sender.clone(), asset.info.clone())?;
  if !rewards.is_zero() {
    UNCLAIMED_REWARDS.update(
      deps.storage,
      (sender.clone(), &asset.info),
      |balance| -> Result<_, ContractError> { Ok(balance.unwrap_or(Uint128::zero()) + rewards) },
    )?;
  }

  let (balance, shares) = TOTAL.may_load(deps.storage, &asset.info)?.unwrap_or_default();
  let (asset_config, asset_available) = _take(&mut deps, &env, &asset.info, balance, true)?;

  let mut withdraw_amount = asset.amount;
  let mut share_amount = compute_share_amount(shares, withdraw_amount, asset_available);

  let current_user_share =
    SHARES.may_load(deps.storage, (sender.clone(), &asset.info))?.unwrap_or_default();

  if current_user_share.is_zero() {
    return Err(ContractError::AmountCannotBeZero {});
  }

  if current_user_share < share_amount {
    share_amount = current_user_share;
    withdraw_amount = compute_balance_amount(shares, share_amount, asset_available)
  }

  let new_value = current_user_share - share_amount;
  if new_value.is_zero() {
    SHARES.remove(deps.storage, (sender, &asset.info));
  } else {
    SHARES.save(deps.storage, (sender, &asset.info), &(current_user_share - share_amount))?;
  }

  TOTAL.save(
    deps.storage,
    &asset.info,
    &(
      balance
        .checked_sub(withdraw_amount)
        .map_err(|_| SharedError::InsufficientBalance("total balance".to_string()))?,
      shares
        .checked_sub(share_amount)
        .map_err(|_| SharedError::InsufficientBalance("total shares".to_string()))?,
    ),
  )?;

  let msg = asset.info.with_balance(withdraw_amount).transfer_msg(&recipient)?;

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "asset/unstake"),
        ("user", info.sender.as_ref()),
        ("recipient", recipient.as_ref()),
        ("asset", &asset.info.to_string()),
        ("amount", &withdraw_amount.to_string()),
        ("share", &share_amount.to_string()),
      ])
      .add_messages(asset_config.stake_config.unstake_check_received_msg(
        &deps,
        &env,
        asset.info.with_balance(withdraw_amount),
      )?)
      .add_message(msg),
  )
}

fn claim_rewards(
  deps: DepsMut,
  info: MessageInfo,
  assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
  let user = info.sender;
  let config = CONFIG.load(deps.storage)?;

  let assets = if let Some(assets) = assets {
    assets
  } else {
    USER_ASSET_REWARD_RATE
      .prefix(user.clone())
      .keys(deps.storage, None, None, Order::Ascending)
      .collect::<StdResult<Vec<_>>>()?
  };

  let assets_str = assets.iter().map(|asset| asset.to_string()).collect::<Vec<String>>().join(",");

  let mut total_rewards = Uint128::zero();
  for asset in assets {
    let rewards = _calc_reward_share(deps.storage, user.clone(), asset.clone())?;
    let unclaimed_rewards =
      UNCLAIMED_REWARDS.load(deps.storage, (user.clone(), &asset)).unwrap_or(Uint128::zero());
    UNCLAIMED_REWARDS.remove(deps.storage, (user.clone(), &asset));

    total_rewards += rewards;
    total_rewards += unclaimed_rewards;
  }

  let response = Response::new().add_attributes(vec![
    ("action", "asset/claim_rewards"),
    ("user", user.as_ref()),
    ("assets", &assets_str),
    ("reward_amount", &total_rewards.to_string()),
  ]);
  if !total_rewards.is_zero() {
    let rewards_asset = config.reward_info.with_balance(total_rewards);
    Ok(response.add_message(rewards_asset.transfer_msg(&user)?))
  } else {
    Ok(response)
  }
}

fn _calc_reward_share(
  storage: &mut dyn Storage,
  user: Addr,
  asset: AssetInfo,
) -> Result<Uint128, ContractError> {
  let user_reward_rate = USER_ASSET_REWARD_RATE.load(storage, (user.clone(), &asset));
  let asset_reward_rate = ASSET_REWARD_RATE.load(storage, &asset)?;

  if let Ok(user_reward_rate) = user_reward_rate {
    let user_staked = SHARES.may_load(storage, (user.clone(), &asset))?.unwrap_or_default();

    if user_staked.is_zero() {
      USER_ASSET_REWARD_RATE.save(storage, (user, &asset), &asset_reward_rate)?;
      return Ok(Uint128::zero());
    }

    let rewards = ((asset_reward_rate - user_reward_rate) * Decimal::from_atomics(user_staked, 0)?)
      .to_uint_floor();
    if rewards.is_zero() {
      Ok(Uint128::zero())
    } else {
      USER_ASSET_REWARD_RATE.save(storage, (user, &asset), &asset_reward_rate)?;
      Ok(rewards)
    }
  } else {
    // If cannot find user_reward_rate, assume this is the first time they are staking and set it to the current asset_reward_rate
    USER_ASSET_REWARD_RATE.save(storage, (user, &asset), &asset_reward_rate)?;

    Ok(Uint128::zero())
  }
}

fn distribute_take_rate(
  mut deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  update: Option<bool>,
  assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let assets = if let Some(assets) = assets {
    assets
  } else {
    WHITELIST.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<_>>()?
  };

  let mut response = Response::new().add_attributes(vec![("action", "asset/distribute_take_rate")]);
  let recipient = config.get_address(&deps.querier, AT_TAKE_RECIPIENT)?;
  for asset in assets {
    let mut config = if update == Some(true) {
      // if it should also update extraction, take the asset config from the result.
      let (balance, _) = TOTAL.may_load(deps.storage, &asset)?.unwrap_or_default();
      // no need to save, as we will save it anyways
      let (config, _) = _take(&mut deps, &env, &asset, balance, false)?;
      config
    } else {
      // otherwise just load it.
      ASSET_CONFIG.may_load(deps.storage, &asset)?.unwrap_or_default()
    };

    let take_amount = config.taken.checked_sub(config.harvested)?;
    if take_amount.is_zero() {
      response = response.add_attribute(asset.to_string(), "skip");
      continue;
    }

    let take_asset = asset.with_balance(take_amount);

    config.harvested = config.taken;
    ASSET_CONFIG.save(deps.storage, &asset, &config)?;

    // unstake assets if necessary
    let unstake_msgs =
      config.stake_config.unstake_check_received_msg(&deps, &env, take_asset.clone())?;
    // transfer to recipient
    let take_msg = take_asset.transfer_msg(recipient.clone())?;

    // println!("unstake_msgs {unstake_msgs:?}");
    // println!("take {take_msg:?}");

    response = response
      .add_messages(unstake_msgs)
      .add_message(take_msg)
      .add_attribute("take", take_asset.to_string());
  }
  Ok(response)
}

fn distribute_bribes(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  update: Option<bool>,
  assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
  let assets = if let Some(assets) = assets {
    assets
  } else {
    WHITELIST.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<_>>()?
  };

  if update == Some(true) {
    let mut msgs = vec![];
    for asset in assets.iter() {
      let asset_config = ASSET_CONFIG.may_load(deps.storage, asset)?.unwrap_or_default();
      let claim_msgs =
        asset_config.stake_config.claim_check_received_msg(&deps, &env, asset.clone())?;
      msgs.extend(claim_msgs)
    }

    msgs.push(env.callback_msg(CallbackMsg::DistributeBribes {
      assets: Some(assets),
    })?);
    Ok(
      Response::new()
        .add_attributes(vec![("action", "asset/distribute_bribes")])
        .add_messages(msgs),
    )
  } else {
    distribute_bribes_callback(deps, env, info, Some(assets))
  }
}

fn distribute_bribes_callback(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
  let assets = if let Some(assets) = assets {
    assets
  } else {
    WHITELIST.keys(deps.storage, None, None, Order::Ascending).collect::<StdResult<_>>()?
  };

  let config = CONFIG.load(deps.storage)?;
  let bribe_manager = config.get_bribe_manager(&deps)?;

  let mut msgs = vec![];

  for asset in assets {
    if let Some(bribes) = ASSET_BRIBES.may_load(deps.storage, &asset)? {
      for bribe in bribes {
        let bribe_msgs = bribe_manager.add_bribe_msgs(
          bribe,
          config.gauge.clone(),
          asset.clone(),
          env.block.height,
        )?;

        msgs.extend(bribe_msgs);
      }

      ASSET_BRIBES.save(deps.storage, &asset, &Assets::default())?;
    }
  }

  Ok(
    Response::new()
      .add_attributes(vec![("action", "asset/distribute_bribes_callback")])
      .add_messages(msgs),
  )
}

fn update_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
  if !info.funds.is_empty() {
    Err(SharedError::NoFundsAllowed {})?;
  }

  let config = CONFIG.load(deps.storage)?;
  let connector = config.get_connector(&deps, &config.gauge)?;

  let initial_balance =
    config.reward_info.with_balance_query(&deps.querier, &env.contract.address)?;

  let msgs = vec![
    connector.claim_rewards_msg()?,
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance,
    }))?,
  ];

  Ok(Response::new().add_attributes(vec![("action", "asset/update_rewards")]).add_messages(msgs))
}

fn update_reward_callback(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  initial_balance: Asset,
) -> Result<Response, ContractError> {
  if info.sender != env.contract.address {
    Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
  }

  let previous_balance = initial_balance.amount;
  let current_balance = initial_balance.info.query_balance(&deps.querier, env.contract.address)?;

  let rewards_collected = current_balance - previous_balance;
  let rewards = initial_balance.info.with_balance(rewards_collected);

  let asset_reward_distribution = ASSET_REWARD_DISTRIBUTION.load(deps.storage)?;
  let total_distribution = asset_reward_distribution
    .iter()
    .map(|a| a.distribution)
    .fold(Decimal::zero(), |acc, v| acc + v);

  for asset_distribution in asset_reward_distribution {
    let total_reward_distributed = Decimal::from_atomics(rewards_collected, 0)?
      * asset_distribution.distribution
      / total_distribution;

    // If there are no shares, we stop updating the rate. This means that the emissions are not directed to any stakers.
    let (_, total_shares) =
      TOTAL.may_load(deps.storage, &asset_distribution.asset)?.unwrap_or_default();
    if !total_shares.is_zero() {
      let rate_to_update = total_reward_distributed / Decimal::from_atomics(total_shares, 0)?;
      if rate_to_update > Decimal::zero() {
        ASSET_REWARD_RATE.update(
          deps.storage,
          &asset_distribution.asset,
          |rate| -> StdResult<_> { Ok(rate.unwrap_or(Decimal::zero()) + rate_to_update) },
        )?;
      }
    }
  }

  Ok(Response::new().add_attributes(vec![
    ("action", "asset/update_rewards_callback"),
    ("rewards", &rewards.to_string()),
  ]))
}

fn track_bribes_callback(
  deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  asset: AssetInfo,
  initial_balances: Vec<Asset>,
) -> Result<Response, ContractError> {
  let mut bribes = ASSET_BRIBES.may_load(deps.storage, &asset)?.unwrap_or_default();

  // this just adds the newly received staking / claiming rewards to the accounting for the corresponding LP.

  let mut attrs = vec![];

  for old_balance in initial_balances {
    let bribe_info = old_balance.info;
    let new_balance = bribe_info.query_balance(&deps.querier, env.contract.address.clone())?;

    if new_balance > old_balance.amount {
      let added_amount = new_balance - old_balance.amount;
      let added = bribe_info.with_balance(added_amount);
      bribes.add(&added);
      attrs.push(attr("bribe", added.to_string()));
    }
  }

  ASSET_BRIBES.save(deps.storage, &asset, &bribes)?;

  Ok(
    Response::new()
      .add_attributes(vec![("action", "asset/track_bribes_callback")])
      .add_attributes(attrs),
  )
}

fn assert_asset_whitelisted(deps: &DepsMut, asset: &AssetInfo) -> Result<bool, ContractError> {
  WHITELIST.load(deps.storage, asset).map_err(|_| ContractError::AssetNotWhitelisted)
}

// Only governance (through a on-chain prop) can change the whitelisted assets
fn assert_asset_whitelist_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    AT_ASSET_WHITELIST_CONTROLLER,
    &info.sender,
  )?;
  Ok(())
}

// Only governance or the operator can pass through this function
fn assert_distribution_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(&deps.querier, AT_ASSET_GAUGE, &info.sender)?;
  Ok(())
}
