use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, QuerierWrapper, Uint128};
use cw_asset::{AssetInfo, AssetInfoUnchecked};
#[allow(unused_imports)]
use std::collections::HashSet;

use crate::{
  adapters::{
    asset_gauge::AssetGauge, global_config_adapter::ConfigExt, voting_escrow::VotingEscrow,
  },
  constants::AT_ASSET_GAUGE,
  error::SharedError,
};

#[cw_serde]
pub struct InstantiateMsg {
  pub emissions_per_week: Uint128,
  pub team_share: Decimal,
  pub rebase_config: RebaseConfg,
  pub mint_config: MintConfig,

  pub gauge: String,
  pub global_config_addr: String,
  pub emission_token: AssetInfoUnchecked,
}

#[cw_serde]
pub enum RebaseConfg {
  Fixed(Decimal),

  // weeklyEmissions × (1 - (VP.totalSupply / 10) ÷ TOKEN.totalsupply)ˆ2 × 0.5
  Dynamic {},
}

#[cw_serde]
pub enum MintConfig {
  /// send amount from existing balance
  UseBalance,

  MintDirect,

  MintProxy,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,
  pub gauge: String,

  pub emissions_per_week: Uint128,
  pub team_share: Decimal,
  pub enabled: bool,
  pub rebase_config: RebaseConfg,
  pub mint_config: MintConfig,
  pub last_claim_s: u64,
  pub emission_token: AssetInfo,
}

impl Config {
  pub fn voting_escrow(&self, querier: &QuerierWrapper) -> Result<VotingEscrow, SharedError> {
    self.global_config().get_address(querier, AT_ASSET_GAUGE).map(VotingEscrow)
  }

  pub fn asset_gauge(&self, querier: &QuerierWrapper) -> Result<AssetGauge, SharedError> {
    self.global_config().get_address(querier, AT_ASSET_GAUGE).map(AssetGauge)
  }
}

#[cw_serde]
pub enum ExecuteMsg {
  // Privileged functions
  ClaimRewards {},

  UpdateConfig {
    emissions_per_s: Option<Uint128>,
    team_share: Option<Decimal>,
    rebase_config: Option<RebaseConfg>,
    mint_config: Option<MintConfig>,
    enabled: Option<bool>,
    gauge: Option<String>,
  },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},
}
