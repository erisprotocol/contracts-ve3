use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  easing::BribeDistributionExt,
  error::{ContractError, ContractResult},
  query::_claim_periods,
  state::{ClaimContext, BRIBE_AVAILABLE, BRIBE_CLAIMED, BRIBE_CREATOR, BRIBE_TOTAL, CONFIG},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use cw_asset::{Asset, AssetInfo};
use ve3_shared::{
  adapters::global_config_adapter::ConfigExt,
  constants::{AT_BRIBE_WHITELIST_CONTROLLER, AT_FEE_COLLECTOR, AT_FREE_BRIBES},
  error::SharedError,
  extensions::{
    asset_ext::{AssetExt, AssetsExt, AssetsUncheckedExt},
    asset_info_ext::AssetInfoExt,
  },
  helpers::{assets::Assets, governance::get_period, time::Times},
  msgs_asset_gauge::UserShare,
  msgs_bribe_manager::{BribeBuckets, BribeDistribution, Config, ExecuteMsg, InstantiateMsg},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  let fee = msg.fee.check(deps.api, None)?;
  fee.info.assert_native()?;

  let whitelist = msg.whitelist.check(deps.api)?;

  CONFIG.save(
    deps.storage,
    &Config {
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      whitelist,
      fee,
      allow_any: false,
    },
  )?;

  Ok(Response::new().add_attributes(vec![("action", "bribe/instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::AddBribe {
      bribe,
      distribution,
      gauge,
      for_info,
    } => {
      let bribe = bribe.check(deps.api, None)?;
      let for_info = for_info.check(deps.api, None)?;

      add_bribe(deps, info, env, bribe, gauge, for_info, distribution)
    },
    ExecuteMsg::WithdrawBribes {
      period,
    } => withdraw_bribes(deps, info, env, period),
    ExecuteMsg::ClaimBribes {
      periods,
    } => claim_bribes(deps, env, info, periods),

    // controller
    ExecuteMsg::WhitelistAssets(assets) => {
      let assets = assets.check(deps.api)?;
      whitelist_assets(deps, info, assets)
    },
    ExecuteMsg::RemoveAssets(assets) => {
      let assets = assets.check(deps.api)?;
      remove_assets(deps, info, assets)
    },
    ExecuteMsg::UpdateConfig {
      fee,
      allow_any,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(fee) = fee {
        let fee = fee.check(deps.api, None)?;
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
  bribe: Asset,
  gauge: String,
  for_info: AssetInfo,
  distribution: BribeDistribution,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;

  let block_period = get_period(env.block.time.seconds())?;
  let user = &info.sender;
  let mut msgs = vec![];

  if bribe.amount.is_zero() {
    Err(SharedError::NotSupported("bribes required".to_string()))?;
  }

  let free_bribes = config.global_config().is_in_list(&deps.querier, AT_FREE_BRIBES, user)?;
  let has_fee = !config.fee.amount.is_zero() && !free_bribes;

  let contract = env.contract.address;

  // println!("{has_fee} {bribe:?}");
  match (has_fee, &bribe.info) {
    (false, AssetInfo::Native(_)) => {
      // no fee and native bribe
      bribe.assert_sent(&info)?
    },
    (false, AssetInfo::Cw20(_)) => {
      // no fee and cw20 bribe
      msgs.push(bribe.transfer_from_msg(user, contract)?)
    },
    (true, AssetInfo::Native(_)) if bribe.info == config.fee.info => {
      // fee and native bribe same asset
      let expected_amount = bribe.amount.checked_add(config.fee.amount)?;
      let expected_deposit = bribe.info.with_balance(expected_amount);
      expected_deposit.assert_sent(&info)?
    },
    (true, AssetInfo::Native(_)) => {
      // fee and native bribe different asset
      vec![&bribe, &config.fee].assert_sent(&info)?
    },
    (true, AssetInfo::Cw20(_)) => {
      // fee and cw20 bribe
      config.fee.assert_sent(&info)?;
      msgs.push(bribe.transfer_from_msg(user, contract)?)
    },

    _ => Err(SharedError::WrongDeposit("combination not supported".to_string()))?,
  }

  if has_fee {
    let fee_collector = config.global_config().get_address(&deps.querier, AT_FEE_COLLECTOR)?;
    msgs.push(config.fee.transfer_msg(fee_collector)?)
  }

  let bribes: Vec<(u64, Uint128)> = distribution.create_distribution(block_period, bribe.amount)?;

  let start = bribes.first().map(|a| a.0).unwrap_or_default();
  let end = bribes.last().map(|a| a.0).unwrap_or_default();

  assert_asset_whitelisted(&config, &bribe.info)?;
  asset_sum_equal(&bribe, &bribes)?;
  asset_future_only(block_period, &bribes)?;

  for (period, amount) in bribes {
    let bribe_split = bribe.info.with_balance(amount);

    let user_key = (user.as_str(), period);
    let mut global_bucket = BRIBE_AVAILABLE.load(deps.storage, period).unwrap_or_default();
    let mut user_bucket = BRIBE_CREATOR.load(deps.storage, user_key).unwrap_or_default();

    global_bucket.add(&gauge, &for_info, &bribe_split);
    user_bucket.add(&gauge, &for_info, &bribe_split);

    BRIBE_AVAILABLE.save(deps.storage, period, &global_bucket)?;
    BRIBE_CREATOR.save(deps.storage, user_key, &user_bucket)?;
  }

  Ok(
    Response::new()
      .add_attribute("action", "bribe/add_bribe")
      .add_attribute("start", start.to_string())
      .add_attribute("end", end.to_string())
      .add_attribute("added", bribe.to_string())
      .add_messages(msgs),
  )
}

fn withdraw_bribes(
  deps: DepsMut,
  info: MessageInfo,
  env: Env,
  period: u64,
) -> Result<Response, ContractError> {
  let block_period = get_period(env.block.time.seconds())?;

  if period <= block_period {
    return Err(ContractError::BribesAlreadyDistributing);
  }

  let user = &info.sender;
  let user_bucket = BRIBE_CREATOR.load(deps.storage, (user.as_str(), period)).unwrap_or_default();

  if user_bucket.is_empty() {
    return Err(ContractError::NoBribes {});
  }

  let mut available = BRIBE_AVAILABLE.load(deps.storage, period)?;

  let mut together = Assets::default();
  for bucket in user_bucket.buckets {
    for bribe in bucket.assets {
      if let Some(asset) = &bucket.asset {
        available.remove(&bucket.gauge, asset, &bribe)?
      } else {
        // buckets always have some asset, except for the group result
      }

      together.add(&bribe);
    }
  }
  let transfer_msgs = together.transfer_msgs(user)?;

  if available.is_empty() {
    BRIBE_AVAILABLE.remove(deps.storage, period);
  } else {
    BRIBE_AVAILABLE.save(deps.storage, period, &available)?;
  }
  BRIBE_CREATOR.remove(deps.storage, (user.as_str(), period));

  Ok(Response::new().add_attribute("action", "bribe/withdraw_bribes").add_messages(transfer_msgs))
}

fn claim_bribes(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  periods: Option<Vec<u64>>,
) -> Result<Response, ContractError> {
  let config = CONFIG.load(deps.storage)?;
  let user = &info.sender;
  let block_period = get_period(env.block.time.seconds())?;
  let asset_gauge = config.asset_gauge(&deps.querier)?;

  let periods = _claim_periods(&deps.as_ref(), user, periods, block_period, &asset_gauge)?;

  if periods.is_empty() {
    return Err(ContractError::NoPeriodsValid);
  }

  // this is ordered by period alread
  let shares =
    asset_gauge.query_user_shares(&deps.querier, user.clone(), Some(Times::Periods(periods)))?;

  if shares.shares.is_empty() {
    return Err(ContractError::NoValidShares);
  }

  let mut context = ClaimContext::default();
  let mut bribe_total = Assets::default();

  let mut periods = vec![];

  for share in shares.shares {
    // shares list sorted by period, each time we find a new one, context is updated.
    // starts with 0
    if share.period != context.period {
      context.maybe_save(deps.storage, user)?;
      periods.push(share.period);

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

      let bribe_totals = match BRIBE_TOTAL.may_load(deps.storage, share.period)? {
        Some(buckets) => buckets,
        None => {
          // if no totals yet, first time this period is touched -> copy it over
          BRIBE_TOTAL.save(deps.storage, share.period, &bribe_available)?;
          bribe_available.clone()
        },
      };

      // checking that not double claim
      if BRIBE_CLAIMED.has(deps.storage, (user.as_str(), share.period)) {
        return Err(ContractError::BribeAlreadyClaimed(share.period));
      }

      context = ClaimContext {
        should_save: true,
        skip: false,
        period: share.period,
        bribe_available,
        bribe_totals,
        bribe_claimed: BribeBuckets::default(),
      };
    }

    if context.skip {
      // skip until a period with bribes is found again.
      continue;
    }

    let UserShare {
      gauge,
      asset,
      user_vp: vp,
      total_vp,
      ..
    } = share;

    // see how much total bribe rewards for the asset in the gauge
    let total_bribe_bucket = context.bribe_totals.get(&gauge, &asset);
    // calculate the reward share based on the user vp compared to total vp
    let rewards = total_bribe_bucket.assets.calc_share_amounts(vp, total_vp)?;
    // add these rewards to the claimed bucket by the user
    context.bribe_claimed.get(&gauge, &asset).assets.add_multi(&rewards);
    // remove the rewards from the available bribe bucket
    context.bribe_available.get(&gauge, &asset).assets.remove_multi(&rewards).map_err(|s| {
      // safety if we try to take more than what is available in the bucket for the asset, then it fails for the user
      ContractError::SharedErrorExtended(
        s,
        format!("gauge: {gauge} asset {asset} vp {vp} total {total_vp} user {user}"),
      )
    })?;
    // add the rewards also to the flattened
    bribe_total.add_multi(&rewards);
  }

  context.maybe_save(deps.storage, user)?;

  let transfer_msgs = bribe_total.transfer_msgs(user)?;
  let periods = periods.iter().map(|asset| asset.to_string()).collect::<Vec<_>>().join(",");
  Ok(
    Response::new()
      .add_attribute("action", "bribe/claim_bribes")
      .add_attribute("periods", periods)
      .add_messages(transfer_msgs),
  )
}

fn whitelist_assets(
  deps: DepsMut,
  info: MessageInfo,
  assets: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
  let mut config = CONFIG.load(deps.storage)?;
  assert_bribe_whitelist_controller(&deps, &info, &config)?;

  if assets.is_empty() {
    return Err(ContractError::RequiresAssetInfos);
  }

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
  assert_bribe_whitelist_controller(&deps, &info, &config)?;

  if assets.is_empty() {
    return Err(ContractError::RequiresAssetInfos);
  }

  config.whitelist.retain(|a| !assets.contains(a));
  CONFIG.save(deps.storage, &config)?;

  let assets_str = assets.iter().map(|asset| asset.to_string()).collect::<Vec<_>>().join(",");
  Ok(
    Response::new()
      .add_attributes(vec![("action", "bribe/remove_assets"), ("assets", &assets_str)]),
  )
}

fn asset_sum_equal(asset: &Asset, bribes: &[(u64, Uint128)]) -> Result<(), ContractError> {
  let sum: Uint128 = bribes.iter().map(|(_, b)| b).sum();
  if sum == asset.amount {
    Ok(())
  } else {
    Err(ContractError::BribeDistribution("sum not equal to deposit".to_string()))
  }
}

fn asset_future_only(block_period: u64, bribes: &[(u64, Uint128)]) -> Result<(), ContractError> {
  if bribes.iter().any(|(period, _)| *period <= block_period) {
    Err(ContractError::BribesAlreadyDistributing)
  } else {
    Ok(())
  }
}

fn assert_asset_whitelisted(
  config: &Config,
  asset_info: &AssetInfo,
) -> Result<bool, ContractError> {
  if config.allow_any || config.whitelist.contains(asset_info) {
    Ok(true)
  } else {
    Err(ContractError::AssetNotWhitelisted)
  }
}

// Only governance (through a on-chain prop) can change the whitelisted assets
fn assert_bribe_whitelist_controller(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    AT_BRIBE_WHITELIST_CONTROLLER,
    &info.sender,
  )?;
  Ok(())
}
