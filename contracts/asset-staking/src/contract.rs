use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::state::{
  ASSET_BRIBES, ASSET_CONFIG, ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, BALANCES, CONFIG,
  TOTAL_BALANCES, UNCLAIMED_REWARDS, USER_ASSET_REWARD_RATE, WHITELIST,
};
use crate::token_factory::CustomExecuteMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  from_json, Addr, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
  Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo};
use ve3_shared::adapters::global_config_adapter::ConfigExt;
use ve3_shared::constants::{
  AT_ASSET_WHITELIST_CONTROLLER, AT_REWARD_DISTRIBUTION_CONTROLLER, AT_TAKE_RECIPIENT,
  SECONDS_PER_YEAR,
};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::env_ext::EnvExt;
use ve3_shared::helpers::general::addr_opt_fallback;
use ve3_shared::msgs_asset_staking::{
  AssetConfigRuntime, AssetDistribution, CallbackMsg, Config, Cw20HookMsg, ExecuteMsg,
  InstantiateMsg, UpdateAssetConfig,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response<CustomExecuteMsg>, ContractError> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let config = Config {
    reward_info: msg.reward_info,
    global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    default_yearly_take_rate: msg.default_yearly_take_rate,
    gauge: msg.gauge,
  };
  CONFIG.save(deps.storage, &config)?;
  Ok(Response::new().add_attributes(vec![("action", "instantiate")]))
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
      let recipient = addr_opt_fallback(deps.api, &recipient, &info.sender)?;
      let asset = AssetInfo::native(&info.funds[0].denom);
      stake(deps, env, info.clone(), asset, info.funds[0].amount, recipient)
    },
    ExecuteMsg::Unstake(asset) => unstake(deps, env, info, asset),
    ExecuteMsg::ClaimRewards(asset) => claim_rewards(deps, info, asset),

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
    ExecuteMsg::WhitelistAssets(assets) => whitelist_assets(deps, info, assets),
    ExecuteMsg::RemoveAssets(assets) => remove_assets(deps, info, assets),
    ExecuteMsg::UpdateAssetConfig(update) => update_asset_config(deps, env, info, update),
    ExecuteMsg::SetAssetRewardDistribution(asset_reward_distribution) => {
      set_asset_reward_distribution(deps, info, asset_reward_distribution)
    },

    // contract
    ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),

    _ => Err(ContractError::Std(StdError::generic_err("unsupported action"))),
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
      let recipient = addr_opt_fallback(deps.api, &recipient, &sender)?;
      stake(deps, env, info, asset, cw20_msg.amount, recipient)
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
  Ok(Response::new().add_attributes(vec![("action", "set_asset_reward_distribution")]))
}

fn update_asset_config(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  update: UpdateAssetConfig,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_asset_whitelist_controller(&deps, &info, &config)?;
  assert_asset_whitelisted(&deps, &update.asset)?;
  let current = ASSET_CONFIG.may_load(deps.storage, &update.asset)?.unwrap_or_default();

  let mut updated = current.clone();
  updated.stake_config = update.config.stake_config;
  updated.yearly_take_rate = update.config.yearly_take_rate;
  ASSET_CONFIG.save(deps.storage, &update.asset, &updated)?;

  let mut msgs = vec![];
  if current.stake_config != updated.stake_config {
    // if stake config changed, withdraw from one (or do nothing), deposit on the other.
    let (balance, _) = TOTAL_BALANCES.load(deps.storage, &update.asset)?;
    let available = balance - current.taken;
    let asset = update.asset.with_balance(available);

    let mut unstake_msgs =
      current.stake_config.unstake_check_received_msg(&deps, &env, asset.clone())?;
    let mut stake_msgs = updated.stake_config.stake_check_received_msg(&deps, &env, asset)?;

    msgs.append(&mut unstake_msgs);
    msgs.append(&mut stake_msgs);
  }

  Ok(
    Response::new().add_attributes(vec![
      ("action", "update_asset_config"),
      ("asset", &update.asset.to_string()),
    ]),
  )
}

fn whitelist_assets(
  deps: DepsMut,
  info: MessageInfo,
  infos: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_asset_whitelist_controller(&deps, &info, &config)?;

  for info in &infos {
    if info == &config.reward_info {
      return Err(ContractError::AssetInfoCannotEqualReward {});
    }

    WHITELIST.save(deps.storage, info, &true)?;
    ASSET_REWARD_RATE
      .update(deps.storage, info, |rate| -> StdResult<_> { Ok(rate.unwrap_or(Decimal::zero())) })?;

    ASSET_CONFIG.save(
      deps.storage,
      info,
      &AssetConfigRuntime {
        yearly_take_rate: config.default_yearly_take_rate,
        ..Default::default()
      },
    )?;
  }

  let assets_str = infos.iter().map(|asset| asset.to_string()).collect::<Vec<String>>().join(",");

  Ok(Response::new().add_attributes(vec![("action", "whitelist_assets"), ("assets", &assets_str)]))
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
  Ok(Response::new().add_attributes(vec![("action", "remove_assets"), ("assets", &assets_str)]))
}

fn stake(
  mut deps: DepsMut,
  env: Env,
  _info: MessageInfo,
  asset: AssetInfo,
  amount: Uint128,
  recipient: Addr,
) -> Result<Response, ContractError> {
  assert_asset_whitelisted(&deps, &asset)?;

  let rewards = _claim_reward(deps.storage, recipient.clone(), asset.clone())?;
  if !rewards.is_zero() {
    UNCLAIMED_REWARDS.update(
      deps.storage,
      (recipient.clone(), &asset),
      |balance| -> Result<_, ContractError> { Ok(balance.unwrap_or(Uint128::zero()) + rewards) },
    )?;
  }

  let (balance, shares) = TOTAL_BALANCES.may_load(deps.storage, &asset)?.unwrap_or_default();
  let (asset_config, asset_available) = _take(&mut deps, &env, &asset, balance, true)?;
  let share_amount = compute_share_amount(shares, amount, asset_available);

  BALANCES.update(
    deps.storage,
    (recipient.clone(), &asset),
    |balance| -> Result<_, ContractError> {
      Ok(balance.unwrap_or_default().checked_add(share_amount)?)
    },
  )?;

  TOTAL_BALANCES.save(
    deps.storage,
    &asset,
    &(balance.checked_add(amount)?, shares.checked_add(share_amount)?),
  )?;

  let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset).unwrap_or(Decimal::zero());
  USER_ASSET_REWARD_RATE.save(deps.storage, (recipient.clone(), &asset), &asset_reward_rate)?;

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "stake"),
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

pub(crate) fn compute_share_amount(
  shares: Uint128,
  balance_amount: Uint128,
  asset_available: Uint128,
) -> Uint128 {
  if asset_available.is_zero() {
    balance_amount
  } else if shares == asset_available {
    return balance_amount;
  } else {
    balance_amount.multiply_ratio(shares, asset_available)
  }
}

pub(crate) fn compute_balance_amount(
  shares: Uint128,
  share_amount: Uint128,
  asset_available: Uint128,
) -> Uint128 {
  if shares.is_zero() {
    Uint128::zero()
  } else if shares == asset_available {
    return share_amount;
  } else {
    share_amount.multiply_ratio(asset_available, shares)
  }
}

fn _take(
  deps: &mut DepsMut,
  env: &Env,
  asset: &AssetInfo,
  total_balance: Uint128,
  save_config: bool,
) -> Result<(AssetConfigRuntime, Uint128), ContractError> {
  let config = ASSET_CONFIG.may_load(deps.storage, asset)?;

  if let Some(mut config) = config {
    if config.yearly_take_rate.is_zero() {
      let available = total_balance.checked_sub(config.taken)?;
      return Ok((config, available));
    }

    // only take if last taken set
    if config.last_taken_s != 0 {
      let take_diff_s = Uint128::new((env.block.time.seconds() - config.last_taken_s).into());
      // total_balance * yearly_take_rate * taken_diff_s / SECONDS_PER_YEAR
      let take_amount =
        config.yearly_take_rate * total_balance.multiply_ratio(take_diff_s, SECONDS_PER_YEAR);

      config.taken = config.taken.checked_add(take_amount)?;
    }

    config.last_taken_s = env.block.time.seconds();

    if save_config {
      ASSET_CONFIG.save(deps.storage, asset, &config)?;
    }

    let available = total_balance.checked_sub(config.taken)?;
    return Ok((config, available));
  }

  Ok((AssetConfigRuntime::default(), total_balance))
}

fn unstake(
  mut deps: DepsMut,
  env: Env,
  info: MessageInfo,
  asset: Asset,
) -> Result<Response, ContractError> {
  let sender = info.sender.clone();
  if asset.amount.is_zero() {
    return Err(ContractError::AmountCannotBeZero {});
  }

  let rewards = _claim_reward(deps.storage, sender.clone(), asset.info.clone())?;
  if !rewards.is_zero() {
    UNCLAIMED_REWARDS.update(
      deps.storage,
      (sender.clone(), &asset.info),
      |balance| -> Result<_, ContractError> { Ok(balance.unwrap_or(Uint128::zero()) + rewards) },
    )?;
  }

  let (balance, shares) = TOTAL_BALANCES.may_load(deps.storage, &asset.info)?.unwrap_or_default();
  let (asset_config, asset_available) = _take(&mut deps, &env, &asset.info, balance, true)?;

  let mut withdraw_amount = asset.amount;
  let mut share_amount = compute_share_amount(shares, withdraw_amount, asset_available);

  let current_user_share =
    BALANCES.may_load(deps.storage, (sender.clone(), &asset.info))?.unwrap_or_default();

  if current_user_share.is_zero() {
    return Err(ContractError::AmountCannotBeZero {});
  }

  if current_user_share < share_amount {
    share_amount = current_user_share;
    withdraw_amount = compute_balance_amount(shares, share_amount, asset_available)
  }

  BALANCES.save(deps.storage, (sender, &asset.info), &(current_user_share - share_amount))?;

  TOTAL_BALANCES.save(
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

  let msg = asset.transfer_msg(&info.sender)?;

  Ok(
    Response::new()
      .add_attributes(vec![
        ("action", "unstake"),
        ("user", info.sender.as_ref()),
        ("asset", &asset.info.to_string()),
        ("amount", &withdraw_amount.to_string()),
        ("shares", &share_amount.to_string()),
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
  asset: AssetInfo,
) -> Result<Response, ContractError> {
  let user = info.sender;
  let config = CONFIG.load(deps.storage)?;
  let rewards = _claim_reward(deps.storage, user.clone(), asset.clone())?;
  let unclaimed_rewards =
    UNCLAIMED_REWARDS.load(deps.storage, (user.clone(), &asset)).unwrap_or(Uint128::zero());
  let final_rewards = rewards + unclaimed_rewards;
  UNCLAIMED_REWARDS.remove(deps.storage, (user.clone(), &asset));
  let response = Response::new().add_attributes(vec![
    ("action", "claim_rewards"),
    ("user", user.as_ref()),
    ("asset", &asset.to_string()),
    ("reward_amount", &final_rewards.to_string()),
  ]);
  if !final_rewards.is_zero() {
    let rewards_asset = config.reward_info.with_balance(final_rewards);
    Ok(response.add_message(rewards_asset.transfer_msg(&user)?))
  } else {
    Ok(response)
  }
}

fn _claim_reward(
  storage: &mut dyn Storage,
  user: Addr,
  asset: AssetInfo,
) -> Result<Uint128, ContractError> {
  let user_reward_rate = USER_ASSET_REWARD_RATE.load(storage, (user.clone(), &asset));
  let asset_reward_rate = ASSET_REWARD_RATE.load(storage, &asset)?;

  if let Ok(user_reward_rate) = user_reward_rate {
    let user_staked = BALANCES.load(storage, (user.clone(), &asset))?;
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

  let mut response = Response::new().add_attributes(vec![("action", "distribute_take_rate")]);
  let recipient = config.get_address(&deps.querier, AT_TAKE_RECIPIENT)?;
  for asset in assets {
    let mut config = if update == Some(true) {
      // if it should also update extraction, take the asset config from the result.
      let (balance, _) = TOTAL_BALANCES.may_load(deps.storage, &asset)?.unwrap_or_default();
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

    response = response
      .add_messages(unstake_msgs)
      .add_message(take_msg)
      .add_attribute("take", format!("{0}{1}", take_amount, asset));
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
    Ok(Response::new().add_attributes(vec![("action", "distribute_bribes")]).add_messages(msgs))
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
    }
  }

  Ok(
    Response::new()
      .add_attributes(vec![("action", "distribute_bribes_callback")])
      .add_messages(msgs),
  )
}

fn update_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
  if !info.funds.is_empty() {
    Err(SharedError::NoFundsAllowed {})?;
  }

  let config = CONFIG.load(deps.storage)?;
  let connector = config.get_connector(&deps)?;

  let initial_balance =
    config.reward_info.with_balance_query(&deps.querier, &env.contract.address)?;

  let msgs = vec![
    connector.claim_rewards_msg()?,
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::UpdateRewards {
      initial_balance,
    }))?,
  ];

  Ok(Response::new().add_attributes(vec![("action", "update_rewards")]).add_messages(msgs))
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
      TOTAL_BALANCES.load(deps.storage, &asset_distribution.asset).unwrap_or_default();
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

  Ok(Response::new().add_attributes(vec![("action", "update_rewards_callback")]))
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
  for old_balance in initial_balances {
    let bribe_info = old_balance.info;
    let new_balance = bribe_info.query_balance(&deps.querier, env.contract.address.clone())?;
    if new_balance > old_balance.amount {
      let added_amount = new_balance - old_balance.amount;
      bribes.add(&bribe_info.with_balance(added_amount));
    }
  }

  ASSET_BRIBES.save(deps.storage, &asset, &bribes)?;

  Ok(Response::new().add_attributes(vec![("action", "track_bribes_callback")]))
}

fn assert_asset_whitelisted(deps: &DepsMut, asset: &AssetInfo) -> Result<bool, ContractError> {
  WHITELIST.load(deps.storage, asset).map_err(|_| ContractError::AssetNotWhitelisted {})
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
  config.global_config().assert_has_access(
    &deps.querier,
    AT_REWARD_DISTRIBUTION_CONTROLLER,
    &info.sender,
  )?;
  Ok(())
}
