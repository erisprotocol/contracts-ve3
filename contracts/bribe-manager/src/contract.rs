use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  easing::BribeDistributionExt,
  error::{ContractError, ContractResult},
  state::{BRIBE_BUCKETS, BRIBE_CREATOR, CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use cw_asset::{Asset, AssetInfo};
use ve3_shared::{
  adapters::global_config_adapter::ConfigExt,
  constants::{AT_ASSET_WHITELIST_CONTROLLER, AT_FEE_COLLECTOR},
  contract_bribe_manager::{BribeDistribution, Config, ExecuteMsg, InstantiateMsg},
  extensions::{
    asset_ext::{AssetExt, AssetsExt},
    asset_info_ext::AssetInfoExt,
  },
  helpers::governance::get_period,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  msg.fee.info.assert_native()?;

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      whitelist: msg.whitelisted,
      fee: msg.fee,
      allow_any: false,
    },
  )?;

  Ok(Response::new().add_attributes(vec![("action", "bribe/instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::AddBribe {
      asset,
      distribution,
    } => add_bribe(deps, info, env, asset, distribution),
    ExecuteMsg::WithdrawBribes {
      period,
    } => withdraw_bribes(deps, info, env, period),

    // controller
    ExecuteMsg::WhitelistAssets(assets) => whitelist_assets(deps, info, assets),
    ExecuteMsg::RemoveAssets(assets) => remove_assets(deps, info, assets),
    ExecuteMsg::UpdateConfig {
      fee,
      allow_any,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(fee) = fee {
        fee.info.assert_native()?;
        config.fee = fee;
      }

      if let Some(allow_any) = allow_any {
        config.allow_any = allow_any;
      }

      CONFIG.save(deps.storage, &config)?;
      Ok(Response::new().add_attribute("action", "bribe/update_config"))
    },
  }
}

fn add_bribe(
  deps: DepsMut,
  info: MessageInfo,
  env: Env,
  asset: Asset,
  distribution: BribeDistribution,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  assert_asset_whitelisted(&config, &asset.info)?;

  let block_period = get_period(env.block.time.seconds())?;
  let user = &info.sender;
  let mut msgs = vec![];

  if asset.info == config.fee.info {
    // deposit and fee same (fee always native, so just add both)
    let expected_deposit = asset.info.with_balance(asset.amount.checked_add(config.fee.amount)?);
    expected_deposit.assert_sent(&info)?
  } else if let AssetInfo::Native(_) = &asset.info {
    // if it is not the same, expect both to be sent
    vec![&asset, &config.fee].assert_sent(&info)?;
  } else if let AssetInfo::Cw20(_) = &asset.info {
    // if cw20, expect fee to be sent (fee always native)
    config.fee.assert_sent(&info)?;
    msgs.push(asset.transfer_from_msg(user, env.contract.address)?)
  }

  // if fee charged, transfer to fee collector
  if !config.fee.amount.is_zero() {
    let fee_collector = config.global_config().get_address(&deps.querier, AT_FEE_COLLECTOR)?;
    msgs.push(config.fee.transfer_msg(fee_collector)?)
  }

  let bribes: Vec<(u64, Uint128)> = distribution.create_distribution(block_period, asset.amount)?;

  asset_sum_equal(&asset, &bribes)?;
  asset_future_only(block_period, &bribes)?;

  for (period, amount) in bribes {
    let asset = asset.info.with_balance(amount);

    let user_key = (user.as_str(), period);
    let mut global_bucket = BRIBE_BUCKETS.load(deps.storage, period).unwrap_or_default();
    let mut user_bucket = BRIBE_CREATOR.load(deps.storage, user_key).unwrap_or_default();

    global_bucket.deposit(asset.clone());
    user_bucket.deposit(asset.clone());

    BRIBE_BUCKETS.save(deps.storage, period, &global_bucket)?;
    BRIBE_CREATOR.save(deps.storage, user_key, &user_bucket)?;
  }

  Ok(Response::new().add_attribute("action", "bribe/add_bribe").add_messages(msgs))
}

fn asset_sum_equal(asset: &Asset, bribes: &Vec<(u64, Uint128)>) -> Result<(), ContractError> {
  let sum: Uint128 = bribes.iter().map(|(a, b)| b).sum();
  if sum == asset.amount {
    Ok(())
  } else {
    Err(ContractError::BribeDistribution("sum not equal to deposit".to_string()))
  }
}

fn asset_future_only(block_period: u64, bribes: &Vec<(u64, Uint128)>) -> Result<(), ContractError> {
  if bribes.iter().any(|(period, _)| *period <= block_period) {
    Err(ContractError::BribesAlreadyDistributing {})
  } else {
    Ok(())
  }
}

fn withdraw_bribes(
  deps: DepsMut,
  info: MessageInfo,
  env: Env,
  period: u64,
) -> Result<Response, ContractError> {
  let block_period = get_period(env.block.time.seconds())?;

  if period <= block_period {
    return Err(ContractError::BribesAlreadyDistributing {});
  }

  let user = &info.sender;
  let user_bucket = BRIBE_CREATOR.load(deps.storage, (user.as_str(), period)).unwrap_or_default();

  if user_bucket.is_empty() {
    return Err(ContractError::NoBribes {});
  }

  let mut bucket = BRIBE_BUCKETS.load(deps.storage, period)?;

  let mut transfer_msgs = vec![];
  for bribe in user_bucket.assets {
    bucket.withdraw(&bribe)?;
    transfer_msgs.push(bribe.transfer_msg(user)?)
  }

  if bucket.is_empty() {
    BRIBE_BUCKETS.remove(deps.storage, period);
  } else {
    BRIBE_BUCKETS.save(deps.storage, period, &bucket)?;
  }
  BRIBE_CREATOR.remove(deps.storage, (user.as_str(), period));

  Ok(Response::new().add_attribute("action", "bribe/withdraw").add_messages(transfer_msgs))
}

fn whitelist_assets(
  deps: DepsMut,
  info: MessageInfo,
  assets: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
  let mut config = CONFIG.load(deps.storage)?;
  assert_asset_whitelist_controller(&deps, &info, &config)?;
  let assets_str = assets.iter().map(|asset| asset.to_string()).collect::<Vec<_>>().join(",");

  for asset in assets {
    if !config.whitelist.contains(&asset) {
      config.whitelist.push(asset);
    }
  }
  CONFIG.save(deps.storage, &config)?;

  Ok(
    Response::new()
      .add_attributes(vec![("action", "bribe/whitelist_assets"), ("assets", &assets_str)]),
  )
}

fn remove_assets(
  deps: DepsMut,
  info: MessageInfo,
  assets: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
  let mut config = CONFIG.load(deps.storage)?;
  // Only allow the governance address to update whitelisted assets
  assert_asset_whitelist_controller(&deps, &info, &config)?;

  config.whitelist.retain(|a| !assets.contains(a));
  CONFIG.save(deps.storage, &config)?;

  let assets_str = assets.iter().map(|asset| asset.to_string()).collect::<Vec<_>>().join(",");
  Ok(
    Response::new()
      .add_attributes(vec![("action", "bribe/remove_assets"), ("assets", &assets_str)]),
  )
}

fn assert_asset_whitelisted(
  config: &Config,
  asset_info: &AssetInfo,
) -> Result<bool, ContractError> {
  if config.allow_any || config.whitelist.contains(asset_info) {
    Ok(true)
  } else {
    Err(ContractError::AssetNotWhitelisted {})
  }
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
