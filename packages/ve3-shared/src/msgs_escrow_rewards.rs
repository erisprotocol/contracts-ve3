use crate::{
  adapters::{asset_gauge::AssetGauge, global_config_adapter::ConfigExt},
  constants::AT_ASSET_GAUGE,
  error::SharedError,
  helpers::{assets::Assets, time::Time},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, QuerierWrapper, Uint128};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetUnchecked};

#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub whitelist: Vec<AssetInfoUnchecked>,
  pub fee: AssetUnchecked,
}

#[cw_serde]
pub enum BribeDistribution {
  Func {
    start: Option<u64>,
    end: u64,
    func_type: FuncType,
  },
  Next,
  Specific(Vec<(u64, Uint128)>),
}

#[cw_serde]
pub enum FuncType {
  Linear,
  Bezier,
  EaseInOutCubic,
  EaseInCubic,
  EaseOutCubic,
  Parametric,
}

#[cw_serde]
pub struct Config {
  pub whitelist: Vec<AssetInfo>,
  pub allow_any: bool,
  pub fee: Asset,
  pub global_config_addr: Addr,
}

impl Config {
  pub fn asset_gauge(&self, querier: &QuerierWrapper) -> Result<AssetGauge, SharedError> {
    self.global_config().get_address(querier, AT_ASSET_GAUGE).map(AssetGauge)
  }
}

impl BribeBuckets {
  pub fn get(&mut self, gauge: &str, asset: &AssetInfo) -> &mut BribeBucket {
    self.get_index(gauge, asset).1
  }

  pub fn get_index(&mut self, gauge: &str, asset: &AssetInfo) -> (usize, &mut BribeBucket) {
    if let Some(i) =
      self.buckets.iter().position(|a| a.gauge == gauge && a.asset.as_ref() == Some(asset))
    {
      (i, &mut self.buckets[i])
    } else {
      self.buckets.push(BribeBucket {
        gauge: gauge.to_string(),
        asset: Some(asset.clone()),
        assets: Assets::default(),
      });
      let i = self.buckets.len() - 1;
      (i, &mut self.buckets[i])
    }
  }

  pub fn add(&mut self, gauge: &str, asset: &AssetInfo, bribe: &Asset) {
    self.get(gauge, asset).assets.add(bribe);
  }

  pub fn remove(
    &mut self,
    gauge: &str,
    asset: &AssetInfo,
    bribe: &Asset,
  ) -> Result<(), SharedError> {
    let (index, bucket) = self.get_index(gauge, asset);
    bucket.assets.remove(bribe)?;
    if bucket.assets.is_empty() {
      self.buckets.remove(index);
    }
    Ok(())
  }

  pub fn is_empty(&self) -> bool {
    self.buckets.len() == 0
  }
}

#[cw_serde]
pub enum ExecuteMsg {
  AddBribe {
    bribe: AssetUnchecked,
    gauge: String,
    for_info: AssetInfoUnchecked,
    distribution: BribeDistribution,
  },

  WithdrawBribes {
    period: u64,
  },

  ClaimBribes {
    periods: Option<Vec<u64>>,
  },

  // controller
  WhitelistAssets(Vec<AssetInfoUnchecked>),
  RemoveAssets(Vec<AssetInfoUnchecked>),

  UpdateConfig {
    fee: Option<AssetUnchecked>,
    allow_any: Option<bool>,
  },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  #[returns(NextClaimPeriodResponse)]
  NextClaimPeriod {
    user: String,
  },

  #[returns(BribesResponse)]
  Bribes {
    period: Option<Time>,
  },

  #[returns(BribesResponse)]
  UserClaimable {
    user: String,
    periods: Option<Vec<u64>>,
  },
}

#[cw_serde]
pub struct NextClaimPeriodResponse {
  pub period: u64,
}

pub type BribesResponse = BribeBuckets;

#[cw_serde]
pub struct MigrateMsg {}
