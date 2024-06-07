use crate::{
  adapters::{asset_gauge::AssetGauge, global_config_adapter::ConfigExt},
  constants::AT_ASSET_GAUGE,
  error::SharedError,
  helpers::assets::Assets,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, QuerierWrapper, Uint128};
use cw_asset::{Asset, AssetInfo};

#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,
  pub whitelisted: Vec<AssetInfo>,
  pub fee: Asset,
  pub fee_recipient: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
  AddBribe {
    bribe: Asset,
    gauge: String,
    for_info: AssetInfo,
    distribution: BribeDistribution,
  },

  WithdrawBribes {
    period: u64,
  },

  ClaimBribes {
    periods: Option<Vec<u64>>,
  },

  // controller
  WhitelistAssets(Vec<AssetInfo>),
  RemoveAssets(Vec<AssetInfo>),

  UpdateConfig {
    fee: Option<Asset>,
    allow_any: Option<bool>,
  },
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

#[cw_serde]
#[derive(Default)]
pub struct BribeBucket {
  pub gauge: String,
  pub asset: Option<AssetInfo>,
  pub assets: Assets,
}

#[cw_serde]
#[derive(Default)]
pub struct BribeBuckets {
  pub buckets: Vec<BribeBucket>,
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
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cfg(test)]
mod test {
  use cosmwasm_std::Addr;
  use cw_asset::{Asset, AssetInfo};

  use crate::{error::SharedError, msgs_bribe_manager::BribeBuckets};

  use super::BribeBucket;

  #[test]
  fn test_bribe_bucket() {
    let asset = AssetInfo::cw20(Addr::unchecked("ampLUNA"));
    let gauge = "test".to_string();

    let mut bucket = BribeBucket {
      asset: Some(asset.clone()),
      gauge: gauge.clone(),
      assets: vec![].into(),
    };

    bucket.assets.add(&Asset::native("uluna", 1000u128));
    bucket.assets.add(&Asset::native("uluna", 500u128));
    bucket.assets.add(&Asset::cw20(Addr::unchecked("test"), 500u128));
    bucket.assets.remove(&Asset::native("uluna", 1000u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        asset: Some(asset.clone()),
        gauge: gauge.clone(),
        assets: vec![
          Asset::native("uluna", 500u128),
          Asset::cw20(Addr::unchecked("test"), 500u128)
        ]
        .into()
      }
    );

    bucket.assets.remove(&Asset::native("uluna", 500u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        asset: Some(asset.clone()),
        gauge,
        assets: vec![Asset::cw20(Addr::unchecked("test"), 500u128)].into()
      }
    );
  }

  #[test]
  fn test_bribe_bucket_fail_not_existing() {
    let asset = AssetInfo::cw20(Addr::unchecked("ampLUNA"));
    let gauge = "test".to_string();

    let mut bucket = BribeBucket {
      asset: Some(asset.clone()),
      gauge,
      assets: vec![].into(),
    };

    let err = bucket.assets.remove(&Asset::native("uluna", 1000u128)).unwrap_err();

    assert_eq!(
      err,
      SharedError::NotFound(format!("asset {0}", AssetInfo::native("uluna".to_string())))
    )
  }

  #[test]
  fn test_bribe_bucket_fail_insufficient_balance() {
    let asset = AssetInfo::cw20(Addr::unchecked("ampLUNA"));
    let gauge = "test".to_string();
    let mut bucket = BribeBucket {
      asset: Some(asset.clone()),
      gauge,
      assets: vec![].into(),
    };

    bucket.assets.add(&Asset::native("uluna", 500u128));
    let err = bucket.assets.remove(&Asset::native("uluna", 1000u128)).unwrap_err();

    assert_eq!(
      err,
      SharedError::InsufficientBalance(format!(
        "existing: {0} withdrawing: {1}",
        Asset::native("uluna", 500u128),
        Asset::native("uluna", 1000u128)
      ),)
    )
  }

  #[test]
  fn test_bribe_buckets() {
    let mut bucket = BribeBuckets::default();

    let asset = AssetInfo::cw20(Addr::unchecked("ampLUNA"));
    let gauge = "test".to_string();

    bucket.get(&gauge, &asset).assets.add(&Asset::native("uluna", 1000u128));
    bucket.get(&gauge, &asset).assets.add(&Asset::native("uluna", 500u128));
    bucket.get(&gauge, &asset).assets.add(&Asset::cw20(Addr::unchecked("test"), 500u128));
    bucket.get(&gauge, &asset).assets.remove(&Asset::native("uluna", 1000u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBuckets {
        buckets: vec![BribeBucket {
          asset: Some(asset.clone()),
          gauge: gauge.clone(),
          assets: vec![
            Asset::native("uluna", 500u128),
            Asset::cw20(Addr::unchecked("test"), 500u128)
          ]
          .into()
        }]
      }
    );

    bucket.remove(&gauge, &asset, &Asset::native("uluna", 500u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBuckets {
        buckets: vec![BribeBucket {
          asset: Some(asset.clone()),
          gauge: gauge.clone(),
          assets: vec![Asset::cw20(Addr::unchecked("test"), 500u128)].into()
        }]
      }
    );
  }
}
