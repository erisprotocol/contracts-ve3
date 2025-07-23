use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION, MAX_FEE, RELAVANT_EXCHANGE_RATES},
  error::{ContractError, ContractResult},
  state::{asset_config_map, CONFIG, EXCHANGE_HISTORY, TOKEN_INDEX},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
  attr, from_json, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdResult,
  Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use std::ops::Div;
use ve3_shared::{
  adapters::global_config_adapter::ConfigExt,
  constants::{AT_BOT, SECONDS_PER_DAY, SECONDS_PER_WEEK},
  error::SharedError,
  extensions::{asset_ext::AssetExt, asset_info_ext::AssetInfoExt, env_ext::EnvExt},
  helpers::{
    denom::{Coin, MsgBurn, MsgCreateDenom, MsgMint},
    general::addr_opt_fallback,
  },
  msgs_asset_compounding::{
    CallbackMsg, CompoundingAssetConfig, Config, Cw20HookMsg, ExchangeHistory, ExecuteMsg,
    InstantiateMsg,
  },
  msgs_zapper::PostActionCreate,
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
      fee_collector: deps.api.addr_validate(&msg.fee_collector)?,
      denom_creation_fee: msg.denom_creation_fee.check(deps.api, None)?,
      deposit_profit_delay_s: assert_deposit_profit_delay(msg.deposit_profit_delay_s)?,
      fee: assert_fee(msg.fee)?,
    },
  )?;

  TOKEN_INDEX.save(deps.storage, &0)?;

  Ok(Response::default())
}

fn assert_fee(fee: Decimal) -> Result<Decimal, ContractError> {
  if fee > MAX_FEE {
    Err(ContractError::ConfigValueTooHigh("fee".to_string()))
  } else {
    Ok(fee)
  }
}

fn assert_deposit_profit_delay(deposit_profit_delay_s: u64) -> Result<u64, ContractError> {
  if deposit_profit_delay_s > SECONDS_PER_WEEK {
    Err(ContractError::ConfigValueTooHigh("deposit_profit_delay_s".to_string()))
  } else {
    Ok(deposit_profit_delay_s)
  }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    // user
    ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
    ExecuteMsg::Stake {
      recipient,
      gauge,
    } => {
      if info.funds.len() != 1 {
        return Err(ContractError::OnlySingleAssetAllowed {});
      }
      if info.funds[0].amount.is_zero() {
        return Err(ContractError::AmountCannotBeZero {});
      }
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender)?;
      let asset = AssetInfo::native(&info.funds[0].denom);
      stake(deps, env, gauge, asset, info.funds[0].amount, recipient)
    },
    ExecuteMsg::Unstake {
      recipient,
    } => {
      let recipient = addr_opt_fallback(deps.api, &recipient, info.sender.clone())?;
      unstake(deps, env, info, recipient)
    },

    ExecuteMsg::InitializeAsset {
      asset_info,
      gauge,
    } => initialize_asset(deps, env, info, asset_info, gauge),
    ExecuteMsg::Compound {
      minimum_receive,
      asset_info,
      gauge,
    } => compound(deps, env, info, minimum_receive, asset_info, gauge),

    ExecuteMsg::ClaimTransfer {
      asset_info,
      gauge,
      receiver,
    } => claim_transfer(deps, env, info, asset_info, gauge, receiver),

    ExecuteMsg::Callback(callback) => handle_callback(deps, env, info, callback),

    ExecuteMsg::UpdateConfig {
      denom_creation_fee,
      deposit_profit_delay_s,
      fee,
      fee_collector,

      fee_for_assets,
    } => {
      let mut config: Config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(fee) = fee {
        config.fee = assert_fee(fee)?;
      }

      if let Some(fee_collector) = fee_collector {
        config.fee_collector = deps.api.addr_validate(&fee_collector)?;
      }

      if let Some(deposit_profit_delay_s) = deposit_profit_delay_s {
        config.deposit_profit_delay_s = assert_deposit_profit_delay(deposit_profit_delay_s)?;
      }

      if let Some(denom_creation_fee) = denom_creation_fee {
        config.denom_creation_fee = denom_creation_fee.check(deps.api, None)?;
      }

      if let Some(fee_for_assets) = fee_for_assets {
        for (gauge, asset, fee) in fee_for_assets {
          let asset = asset.check(deps.api, None)?;
          let mut asset_config = assert_asset_whitelisted(deps.storage, &gauge, &asset)?;
          asset_config.fee = match fee {
            Some(fee) => Some(assert_fee(fee)?),
            None => None,
          };
          asset_config_map().save(deps.storage, (&gauge, &asset), &asset_config)?
        }
      }

      CONFIG.save(deps.storage, &config)?;

      Ok(Response::default().add_attribute("action", "asset-compounding/update_config"))
    },
  }
}

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
      gauge,
    } => {
      if cw20_msg.amount.is_zero() {
        return Err(ContractError::AmountCannotBeZero {});
      }
      let asset = AssetInfo::Cw20(info.sender.clone());
      let recipient = addr_opt_fallback(deps.api, &recipient, sender)?;
      stake(deps, env, gauge, asset, cw20_msg.amount, recipient)
    },
  }
}

fn stake(
  deps: DepsMut,
  env: Env,
  gauge: String,
  asset_info: AssetInfo,
  amount: Uint128,
  recipient: Addr,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let asset_config = assert_asset_whitelisted(deps.storage, &gauge, &asset_info)?;

  let staked_balance = asset_config.staking.query_staked_balance_fallback(
    &deps.querier,
    &env.contract.address,
    asset_info.clone(),
  )?;

  let amplp_supply = deps.querier.query_supply(asset_config.amp_denom.clone())?.amount;
  let lp_staked = staked_balance.asset.amount;

  let bond_share = calc_bond_share(amount, amplp_supply, lp_staked);
  let bond_share_adjusted = calc_adjusted_share(
    deps.storage,
    &gauge,
    &asset_info,
    config.deposit_profit_delay_s,
    bond_share,
  )?;
  let adjustment_amount = bond_share.saturating_sub(bond_share_adjusted);

  let deposit_msg = asset_config
    .staking
    .deposit_msg(asset_info.with_balance(amount), Some(env.contract.address.to_string()))?;

  let mut msgs = vec![
    MsgMint {
      sender: env.contract.address.to_string(),
      amount: Some(Coin {
        // mint full bond_share
        amount: bond_share.to_string(),
        denom: asset_config.amp_denom.to_string(),
      }),
      // not working on terra
      mint_to_address: env.contract.address.to_string(),
    }
    .into(),
    // transfer adjusted to depositor
    AssetInfo::native(asset_config.amp_denom.clone())
      .with_balance(bond_share_adjusted)
      .transfer_msg(recipient.clone())?,
  ];

  if !adjustment_amount.is_zero() {
    // transfer adjustment to fee collector
    msgs.push(
      AssetInfo::native(asset_config.amp_denom)
        .with_balance(adjustment_amount)
        .transfer_msg(config.fee_collector.clone())?,
    );
  }

  Ok(
    Response::new()
      .add_attributes(vec![
        attr("action", "asset-compounding/stake"),
        attr("user", recipient),
        attr("asset", asset_info.to_string()),
        attr("gauge", gauge),
        attr("bond_amount", amount),
        attr("bond_share_adjusted", bond_share_adjusted),
        attr("bond_share", bond_share),
      ])
      .add_message(deposit_msg)
      .add_messages(msgs),
  )
}

fn calc_bond_share(lp_amount: Uint128, amplp_supply: Uint128, lp_staked: Uint128) -> Uint128 {
  if amplp_supply.is_zero() || lp_staked.is_zero() {
    lp_amount
  } else {
    // 100 amount
    // 500 amplp_supply
    // 600 lp_staked (ratio 0.83)
    // 100/600 * 500 = 83.333

    // ->
    // 583 amplp_supply
    // 700 lp_staked (ratio 0.83)
    lp_amount.multiply_ratio(amplp_supply, lp_staked)
  }
}

pub fn calc_bond_amount(
  amplp_amount: Uint128,
  amplp_supply: Uint128,
  lp_staked: Uint128,
) -> Uint128 {
  if amplp_supply.is_zero() {
    Uint128::zero()
  } else {
    lp_staked.multiply_ratio(amplp_amount, amplp_supply)
  }
}

pub fn calc_adjusted_share(
  storage: &mut dyn Storage,
  gauge: &str,
  asset_info: &AssetInfo,
  deposit_profit_delay_s: u64,
  bond_share: Uint128,
) -> StdResult<Uint128> {
  if deposit_profit_delay_s == 0 {
    return Ok(bond_share);
  }

  let exchange_rates = EXCHANGE_HISTORY
    .prefix((gauge, asset_info))
    .range(storage, None, None, Order::Descending)
    .take(RELAVANT_EXCHANGE_RATES)
    .map(|a| {
      let (_, item) = a?;
      Ok(item)
    })
    .collect::<StdResult<Vec<ExchangeHistory>>>()?;

  if exchange_rates.len() < 2 {
    Ok(bond_share)
  } else {
    let current = &exchange_rates[0];
    let last = &exchange_rates[exchange_rates.len() - 1];

    let delta_time_s = current.time_s - last.time_s;
    // if the exchange rate has been reduced (which cant happen), ignore it.
    let delta_rate = current.exchange_rate.checked_sub(last.exchange_rate).unwrap_or_default();
    // specifies how much the exchange rate has increased in comparison to the start point. (e.g. 50% since last)
    let delta_rate_percent = delta_rate.div(last.exchange_rate);

    // delta_rate_percent = delta_rate / start
    // factor = delta_rate_percent / delta_time_s * deposit_profit_delay_s = delta_rate_percent * (deposit_profit_delay_s / delta_time_s)
    // e.g. delta_rate_percent 0.1, delta_time_s: 3d, deposit_porift_delay_s: 1d
    // -> factor = 0.03333
    // adjusted_share = share / (1 + factor)

    let factor_plus_one = delta_rate_percent
      .checked_mul(Decimal::from_ratio(deposit_profit_delay_s, delta_time_s))?
      .checked_add(Decimal::one())?;

    let adjusted_share = bond_share * Decimal::one().div(factor_plus_one);
    Ok(adjusted_share)
  }
}

fn unstake(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  recipient: Addr,
) -> Result<Response, ContractError> {
  if info.funds.len() != 1 {
    return Err(ContractError::OnlySingleAssetAllowed {});
  }
  if info.funds[0].amount.is_zero() {
    return Err(ContractError::AmountCannotBeZero {});
  }

  let amplp_denom = info.funds[0].denom.clone();
  let amplp_amount = info.funds[0].amount;

  let asset_config = assert_asset_whitelisted_by_amplp_denom(&deps, &amplp_denom)?;

  let staked_balance = asset_config.staking.query_staked_balance_fallback(
    &deps.querier,
    &env.contract.address,
    asset_config.asset_info.clone(),
  )?;

  let amplp_supply = deps.querier.query_supply(asset_config.amp_denom)?.amount;
  let lp_staked = staked_balance.asset.amount;

  let returned_amount = calc_bond_amount(amplp_amount, amplp_supply, lp_staked);

  let returned = asset_config.asset_info.with_balance(returned_amount);
  let burn_msg: CosmosMsg = MsgBurn {
    sender: env.contract.address.to_string(),
    amount: Some(Coin {
      denom: amplp_denom,
      amount: amplp_amount.to_string(),
    }),
    burn_from_address: env.contract.address.to_string(),
  }
  .into();
  let withdraw_msg =
    asset_config.staking.withdraw_msg(returned.clone(), Some(recipient.to_string()))?;

  Ok(
    Response::new()
      .add_attributes(vec![
        attr("action", "asset-compounding/unstake"),
        attr("user", info.sender.as_ref()),
        attr("recipient", recipient.as_ref()),
        attr("returned", returned.to_string()),
      ])
      .add_message(burn_msg)
      .add_message(withdraw_msg),
  )
}

fn compound(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  minimum_receive: Option<Uint128>,
  asset_info: AssetInfoUnchecked,
  gauge: String,
) -> ContractResult {
  let asset_info = asset_info.check(deps.api, None)?;
  let config = CONFIG.load(deps.storage)?;
  config.global_config().assert_has_access(&deps.querier, AT_BOT, &info.sender)?;

  let asset_config = assert_asset_whitelisted(deps.storage, &gauge, &asset_info)?;
  let zapper = config.zapper(&deps.querier)?;
  let connector = config.connector(&deps.querier, &gauge)?;

  let msgs = vec![
    // 1. claim zassets
    asset_config.staking.claim_reward_msg(asset_info.clone(), None)?,
    // 2. withdraw zassets
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::WithdrawZasset {
      zasset_denom: asset_config.zasset_denom.clone(),
      connector,
    }))?,
    // 3. send zapper
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::ZapRewards {
      zapper,
      config,
      asset_config: asset_config.clone(),
      minimum_receive,
    }))?,
    // 4. track new exchange rate
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::TrackExchangeRate {
      asset_config: asset_config.clone(),
      asset_info,
      gauge,
    }))?,
  ];

  Ok(Response::default().add_attribute("action", "asset-compounding/compound").add_messages(msgs))
}

fn claim_transfer(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  asset_info: AssetInfoUnchecked,
  gauge: String,
  receiver: Option<String>,
) -> ContractResult {
  let asset_info = asset_info.check(deps.api, None)?;
  let config = CONFIG.load(deps.storage)?;
  config.global_config().assert_owner(&deps.querier, &info.sender)?;
  let receiver = addr_opt_fallback(deps.api, &receiver, info.sender)?;

  let asset_config = assert_asset_whitelisted(deps.storage, &gauge, &asset_info)?;
  let connector = config.connector(&deps.querier, &gauge)?;

  let msgs = vec![
    // 1. claim zassets
    asset_config.staking.claim_reward_msg(asset_info.clone(), None)?,
    // 2. withdraw zassets
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::WithdrawZasset {
      zasset_denom: asset_config.zasset_denom.clone(),
      connector,
    }))?,
    // 3. transfer
    env.callback_msg(ExecuteMsg::Callback(CallbackMsg::Transfer {
      config,
      asset_config,
      receiver,
    }))?,
  ];

  Ok(
    Response::default()
      .add_attribute("action", "asset-compounding/claim_transfer")
      .add_messages(msgs),
  )
}

fn initialize_asset(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  asset_info: AssetInfoUnchecked,
  gauge: String,
) -> ContractResult {
  let asset_info = asset_info.check(deps.api, None)?;
  let config = CONFIG.load(deps.storage)?;
  let asset_staking = config.asset_staking(&deps.querier, &gauge)?;
  let connector = config.connector(&deps.querier, &gauge)?;
  let has_asset = asset_staking.query_whitelisted_assets(&deps.querier)?.contains(&asset_info);

  if !has_asset {
    return Err(ContractError::AssetNotWhitelisted(gauge, asset_info.to_string()));
  }

  if asset_config_map().has(deps.storage, (&gauge, &asset_info)) {
    return Err(ContractError::AssetAlreadyInitialized(gauge, asset_info.to_string()));
  }

  config.denom_creation_fee.assert_sent(&info)?;

  let token_index = TOKEN_INDEX.load(deps.storage)?;
  TOKEN_INDEX.save(deps.storage, &(token_index + 1))?;

  let connector_config = connector.query_config(&deps.querier)?;

  let subdenom = format!("{0}/{1}/amplp", token_index, gauge);
  let amplp_full_denom = format!("factory/{0}/{1}", env.contract.address, subdenom);

  let amplp_create_msg: CosmosMsg = MsgCreateDenom {
    sender: env.contract.address.to_string(),
    subdenom,
  }
  .into();

  asset_config_map().save(
    deps.storage,
    (&gauge.clone(), &asset_info.clone()),
    &CompoundingAssetConfig {
      asset_info,
      gauge,
      amp_denom: amplp_full_denom.clone(),
      total_bond_share: Uint128::zero(),
      fee: None,
      zasset_denom: connector_config.zasset_denom,
      reward_asset_info: connector_config.lst_asset_info,
      staking: asset_staking.clone(),
    },
  )?;

  Ok(
    Response::default()
      .add_attribute("action", "asset-compounding/initialize_asset")
      .add_attribute("amplp", amplp_full_denom)
      .add_attribute("staking", asset_staking.0.to_string())
      .add_message(amplp_create_msg),
  )
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
    CallbackMsg::WithdrawZasset {
      connector,
      zasset_denom,
    } => {
      let zasset =
        AssetInfo::native(zasset_denom).with_balance_query(&deps.querier, &env.contract.address)?;

      if zasset.amount.is_zero() {
        return Err(ContractError::NoRewards);
      }

      let withdraw_msg = connector.withdraw_msg(zasset.to_coin()?)?;

      Ok(
        Response::new()
          .add_attribute("action", "asset-compounding/callback_withdraw_zasset")
          .add_attribute("zasset", zasset.to_string())
          .add_message(withdraw_msg),
      )
    },
    CallbackMsg::ZapRewards {
      config,
      zapper,
      asset_config,
      minimum_receive,
    } => {
      let reward_asset_info = asset_config.reward_asset_info;
      let reward_amount = reward_asset_info.query_balance(&deps.querier, &env.contract.address)?;

      let fee = asset_config.fee.unwrap_or(config.fee) * reward_amount;
      let zap_amount = reward_amount.checked_sub(fee)?;

      let mut msgs = vec![
        // transfer rewards to zapper
        reward_asset_info.with_balance(zap_amount).transfer_msg(zapper.0.to_string())?,
        // converts ampLP to LP or asset info + stakes it for current contract
        zapper.zap(
          // target asset
          asset_config.asset_info.into(),
          // source asset
          vec![reward_asset_info.clone()],
          minimum_receive,
          Some(PostActionCreate::Stake {
            asset_staking: asset_config.staking.0.clone(),
            receiver: Some(env.contract.address.to_string()),
          }),
        )?,
      ];

      if !fee.is_zero() {
        msgs.push(reward_asset_info.with_balance(fee).transfer_msg(config.fee_collector)?);
      }

      Ok(
        Response::new()
          .add_attribute("action", "asset-compounding/callback_zap_rewards")
          .add_attribute("rewards", zap_amount.to_string())
          .add_attribute("fee", fee.to_string())
          .add_messages(msgs),
      )
    },

    CallbackMsg::Transfer {
      config,
      asset_config,
      receiver,
    } => {
      let reward_asset_info = asset_config.reward_asset_info;
      let reward_amount = reward_asset_info.query_balance(&deps.querier, &env.contract.address)?;

      let fee = asset_config.fee.unwrap_or(config.fee) * reward_amount;
      let transfer_amount = reward_amount.checked_sub(fee)?;

      let mut msgs = vec![
        // transfer rewards to zapper
        reward_asset_info.with_balance(transfer_amount).transfer_msg(receiver.to_string())?,
      ];

      if !fee.is_zero() {
        msgs.push(reward_asset_info.with_balance(fee).transfer_msg(config.fee_collector)?);
      }

      Ok(
        Response::new()
          .add_attribute("action", "asset-compounding/callback_transfer")
          .add_attribute("rewards", transfer_amount.to_string())
          .add_attribute("fee", fee.to_string())
          .add_messages(msgs),
      )
    },

    CallbackMsg::TrackExchangeRate {
      asset_config,
      asset_info,
      gauge,
    } => {
      let staked_balance = asset_config.staking.query_staked_balance_fallback(
        &deps.querier,
        &env.contract.address,
        asset_info.clone(),
      )?;

      let amplp_supply = deps.querier.query_supply(asset_config.amp_denom)?.amount;
      let lp_staked = staked_balance.asset.amount;

      let exchange_rate = Decimal::from_ratio(lp_staked, amplp_supply);
      EXCHANGE_HISTORY.save(
        deps.storage,
        (&gauge, &asset_info, env.block.time.seconds().div(SECONDS_PER_DAY)),
        &ExchangeHistory {
          exchange_rate,
          time_s: env.block.time.seconds(),
        },
      )?;

      Ok(
        Response::new()
          .add_attribute("action", "asset-compounding/callback_track_exchange_rate")
          .add_attribute("exchange_rate", exchange_rate.to_string()),
      )
    },
  }
}

pub fn assert_asset_whitelisted(
  storage: &dyn Storage,
  gauge: &str,
  asset: &AssetInfo,
) -> Result<CompoundingAssetConfig, ContractError> {
  asset_config_map()
    .load(storage, (gauge, asset))
    .map_err(|_| ContractError::AssetNotWhitelisted(gauge.to_string(), asset.to_string()))
}

fn assert_asset_whitelisted_by_amplp_denom(
  deps: &DepsMut,
  amplp_denom: &str,
) -> Result<CompoundingAssetConfig, ContractError> {
  let mut element = asset_config_map()
    .idx
    .by_denom
    .prefix(amplp_denom.to_string())
    .range(deps.storage, None, None, Order::Ascending)
    .take(1)
    .collect::<StdResult<Vec<((String, AssetInfo), CompoundingAssetConfig)>>>()?;

  if element.is_empty() {
    return Err(ContractError::AmplpNotFound(amplp_denom.to_string()));
  }

  Ok(element.swap_remove(0).1)
}
