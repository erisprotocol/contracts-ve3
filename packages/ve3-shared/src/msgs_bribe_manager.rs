use crate::{
  adapters::{asset_gauge::AssetGauge, global_config_adapter::ConfigExt},
  constants::AT_ASSET_GAUGE,
  error::SharedError,
  extensions::asset_info_ext::AssetInfoExt,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CheckedMultiplyRatioError, CosmosMsg, QuerierWrapper, Uint128};
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
    asset: AssetInfo,
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
  pub assets: Vec<Asset>,
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
        assets: vec![],
      });
      let i = self.buckets.len() - 1;
      (i, &mut self.buckets[i])
    }
  }

  pub fn remove(
    &mut self,
    gauge: &str,
    asset: &AssetInfo,
    bribe: &Asset,
  ) -> Result<(), SharedError> {
    let (index, bucket) = self.get_index(gauge, asset);
    bucket.remove(bribe)?;
    if bucket.is_empty() {
      self.buckets.remove(index);
    }
    Ok(())
  }

  pub fn is_empty(&self) -> bool {
    self.buckets.len() == 0
  }
}

impl BribeBucket {
  pub fn is_empty(&self) -> bool {
    self.assets.len() == 0
  }

  pub fn remove(&mut self, asset: &Asset) -> Result<(), SharedError> {
    let existing = self.assets.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(existing) if existing.amount < asset.amount => Err(SharedError::InsufficientBalance(
        format!("existing: {0} withdrawing: {1}", existing, asset),
      )),
      Some(existing) => {
        existing.amount -= asset.amount;

        if existing.amount.is_zero() {
          self.assets.retain(|a| !a.amount.is_zero())
        }
        Ok(())
      },
      None => Err(SharedError::NotFound(format!("asset {0}", asset.info))),
    }
  }

  pub fn remove_multi(&mut self, assets: &Vec<Asset>) -> Result<(), SharedError> {
    for asset in assets {
      self.remove(asset)?;
    }

    Ok(())
  }

  pub fn add(&mut self, asset: &Asset) {
    let existing = self.assets.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(in_bucket) => {
        in_bucket.amount += asset.amount;
      },
      None => {
        self.assets.push(asset.clone());
      },
    }
  }

  pub fn add_multi(&mut self, assets: &Vec<Asset>) {
    for asset in assets {
      self.add(asset);
    }
  }

  pub fn calc_share_amounts(
    &self,
    vp: Uint128,
    total_vp: Uint128,
  ) -> Result<Vec<Asset>, CheckedMultiplyRatioError> {
    self
      .assets
      .iter()
      .map(|a| {
        a.amount.checked_multiply_ratio(vp, total_vp).map(|amount| a.info.with_balance(amount))
      })
      .collect()
  }

  pub fn transfer_msgs(&self, to: &Addr) -> Result<Vec<CosmosMsg>, SharedError> {
    let mut results = vec![];
    for asset in self.assets.iter() {
      results.push(asset.transfer_msg(to)?);
    }
    Ok(results)
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
      assets: vec![],
    };

    bucket.add(&Asset::native("uluna", 1000u128));
    bucket.add(&Asset::native("uluna", 500u128));
    bucket.add(&Asset::cw20(Addr::unchecked("test"), 500u128));
    bucket.remove(&Asset::native("uluna", 1000u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        asset: Some(asset.clone()),
        gauge: gauge.clone(),
        assets: vec![
          Asset::native("uluna", 500u128),
          Asset::cw20(Addr::unchecked("test"), 500u128)
        ]
      }
    );

    bucket.remove(&Asset::native("uluna", 500u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        asset: Some(asset.clone()),
        gauge,
        assets: vec![Asset::cw20(Addr::unchecked("test"), 500u128)]
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
      assets: vec![],
    };

    let err = bucket.remove(&Asset::native("uluna", 1000u128)).unwrap_err();

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
      assets: vec![],
    };

    bucket.add(&Asset::native("uluna", 500u128));
    let err = bucket.remove(&Asset::native("uluna", 1000u128)).unwrap_err();

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

    bucket.get(&gauge, &asset).add(&Asset::native("uluna", 1000u128));
    bucket.get(&gauge, &asset).add(&Asset::native("uluna", 500u128));
    bucket.get(&gauge, &asset).add(&Asset::cw20(Addr::unchecked("test"), 500u128));
    bucket.get(&gauge, &asset).remove(&Asset::native("uluna", 1000u128)).unwrap();

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
          assets: vec![Asset::cw20(Addr::unchecked("test"), 500u128)]
        }]
      }
    );
  }
}
