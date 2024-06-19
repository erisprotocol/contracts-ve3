use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  error::{ContractError, ContractResult},
  state::CONFIG,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;
use ve3_shared::{
  adapters::{global_config_adapter::ConfigExt, mint_proxy::MintProxy},
  constants::{
    at_asset_staking, AT_MINT_PROXY, AT_TEAM_WALLET, SECONDS_PER_WEEK, SECONDS_PER_YEAR,
  },
  extensions::asset_info_ext::AssetInfoExt,
  msgs_connector_emission::{Config, ExecuteMsg, InstantiateMsg, RebaseConfg},
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
      emissions_per_week: msg.emissions_per_week,
      last_claim_s: 0,
      global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
      team_share: msg.team_share,
      enabled: false,
      rebase_config: msg.rebase_config,
      mint_config: msg.mint_config,
      emission_token: msg.emission_token.check(deps.api, None)?,
      gauge: msg.gauge,
    },
  )?;

  Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
  match msg {
    ExecuteMsg::ClaimRewards {} => claim_rewards(deps, env, info),
    ExecuteMsg::UpdateConfig {
      emissions_per_s: emissions_per_week,
      team_share,
      rebase_config,
      mint_config,
      enabled,
      gauge,
    } => {
      let mut config = CONFIG.load(deps.storage)?;
      config.global_config().assert_owner(&deps.querier, &info.sender)?;

      if let Some(emissions_per_week) = emissions_per_week {
        config.emissions_per_week = emissions_per_week;
      }

      if let Some(team_share) = team_share {
        config.team_share = team_share;
      }

      if let Some(rebase_config) = rebase_config {
        config.rebase_config = rebase_config;
      }

      if let Some(gauge) = gauge {
        config.gauge = gauge;
      }

      if let Some(mint_config) = mint_config {
        config.mint_config = mint_config;
      }

      if let Some(enabled) = enabled {
        config.enabled = enabled;
        if enabled {
          config.last_claim_s = env.block.time.seconds();
        }
      }

      CONFIG.save(deps.storage, &config)?;
      Ok(Response::default().add_attribute("action", "ce/update_config"))
    },
  }
}

fn claim_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> ContractResult {
  let mut config = CONFIG.load(deps.storage)?;

  assert_asset_staking_right_gauge(&deps, &info, &config)?;
  let asset_staking_addr = info.sender;

  if config.enabled {
    let diff_claim_time_seconds =
      env.block.time.seconds().checked_sub(config.last_claim_s).unwrap_or_default();

    let emission_amount = Uint128::new(diff_claim_time_seconds.into())
      .multiply_ratio(config.emissions_per_week, SECONDS_PER_WEEK);

    if !emission_amount.is_zero() {
      let team_amount = emission_amount * config.team_share;
      let rebase_amount = match config.rebase_config {
        RebaseConfg::Fixed(percent) => emission_amount * percent,
        RebaseConfg::TargetYearlyApy(apy) => {
          let voting_escrow = config.voting_escrow(&deps.querier)?;
          let total_fixed = voting_escrow.query_total_fixed(&deps.querier, None)?.fixed;

          // println!("percent: {apy}");
          // println!("total_fixed: {total_fixed}");
          apy
            * Uint128::new(diff_claim_time_seconds.into())
              .multiply_ratio(total_fixed, SECONDS_PER_YEAR)
        },
        RebaseConfg::Dynamic {} => {
          // weeklyEmissions × (1 - (VP.totalSupply / 10) / TOKEN.totalSupply)ˆ2 × 0.5

          let voting_escrow = config.voting_escrow(&deps.querier)?;
          let total_vp = voting_escrow.query_total_vp(&deps.querier, None)?.vp;
          let token_supply = config.emission_token.total_supply(&deps.querier)?;

          let quotient = Decimal::from_ratio(total_vp, token_supply) * Decimal::percent(10);
          let reverse_quotient = Decimal::one() - quotient;
          let factor = reverse_quotient * reverse_quotient * Decimal::percent(50);

          // println!("total_vp: {total_vp}");
          // println!("token_supply: {token_supply}");
          // println!("quotient: {quotient}");
          // println!("reverse_quotient: {reverse_quotient}");
          // println!("factor: {factor}");

          emission_amount * factor
        },
      };
      // println!("rebase_amount: {rebase_amount}");

      config.last_claim_s = env.block.time.seconds();

      let mut msgs = vec![];

      let total = emission_amount + team_amount + rebase_amount;
      match config.mint_config {
        ve3_shared::msgs_connector_emission::MintConfig::UseBalance => {
          // nothing to do, as we expect the balance already to be there in the contract
        },
        ve3_shared::msgs_connector_emission::MintConfig::MintDirect => {
          match &config.emission_token {
            cw_asset::AssetInfoBase::Native(denom) => {
              let mint_self: CosmosMsg = ve3_shared::helpers::denom::MsgMint {
                sender: env.contract.address.to_string(),
                amount: Some(ve3_shared::helpers::denom::Coin {
                  denom: denom.to_string(),
                  amount: total.to_string(),
                }),
                mint_to_address: env.contract.address.to_string(),
              }
              .into();
              msgs.push(mint_self);
            },
            _ => Err(ContractError::SharedError(ve3_shared::error::SharedError::NotSupported(
              "only native".to_string(),
            )))?,
          }
        },
        ve3_shared::msgs_connector_emission::MintConfig::MintProxy => {
          let mint_proxy_addr = config.global_config().get_address(&deps.querier, AT_MINT_PROXY)?;
          let mint_proxy = MintProxy(mint_proxy_addr);
          msgs.push(mint_proxy.mint_msg(total)?);
        },
      };

      msgs.push(
        config.emission_token.with_balance(emission_amount).transfer_msg(asset_staking_addr)?,
      );

      if !team_amount.is_zero() {
        let team_wallet = config.global_config().get_address(&deps.querier, AT_TEAM_WALLET)?;
        msgs.push(config.emission_token.with_balance(team_amount).transfer_msg(team_wallet)?)
      }

      if !rebase_amount.is_zero() {
        let asset_gauge = config.asset_gauge(&deps.querier)?;
        let rebase_asset = config.emission_token.with_balance(rebase_amount);
        let msg = asset_gauge.add_rebase_msg(rebase_asset)?;
        msgs.push(msg);
      }

      CONFIG.save(deps.storage, &config)?;

      return Ok(
        Response::default()
          .add_attribute("action", "ce/claim_rewards")
          .add_attribute("emission_amount", emission_amount)
          .add_attribute("rebase_amount", rebase_amount)
          .add_attribute("team_amount", team_amount)
          .add_messages(msgs),
      );
    }
  }
  Ok(Response::default().add_attribute("action", "ce/claim_rewards_noop"))
}

fn assert_asset_staking_right_gauge(
  deps: &DepsMut,
  info: &MessageInfo,
  config: &Config,
) -> Result<(), ContractError> {
  config.global_config().assert_has_access(
    &deps.querier,
    &at_asset_staking(&config.gauge),
    &info.sender,
  )?;
  Ok(())
}
