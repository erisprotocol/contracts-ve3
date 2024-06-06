use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw_asset::{Asset, AssetInfo};

use crate::error::SharedError;

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
    asset: Asset,
    distribution: BribeDistribution,
  },

  WithdrawBribes {
    period: u64,
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

#[cw_serde]
#[derive(Default)]
pub struct BribeBucket {
  pub assets: Vec<Asset>,
}

impl BribeBucket {
  pub fn is_empty(&self) -> bool {
    self.assets.len() == 0
  }

  pub fn withdraw(&mut self, asset: &Asset) -> Result<(), SharedError> {
    let in_bucket = self.assets.iter_mut().find(|a| a.info == asset.info);

    match in_bucket {
      Some(in_bucket) if in_bucket.amount < asset.amount => Err(SharedError::InsufficientBalance(
        format!("existing: {0} withdrawing: {1}", in_bucket.to_string(), asset.to_string()),
      )),
      Some(in_bucket) => {
        in_bucket.amount = in_bucket.amount - asset.amount;

        if in_bucket.amount.is_zero() {
          self.assets.retain(|a| !a.amount.is_zero())
        }
        Ok(())
      },
      None => Err(SharedError::NotFound(format!("asset {0}", asset.info))),
    }
  }

  pub fn deposit(&mut self, asset: Asset) -> () {
    let existing = self.assets.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(in_bucket) => {
        in_bucket.amount = in_bucket.amount + asset.amount;
      },
      None => {
        self.assets.push(asset);
      },
    }
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

  use crate::error::SharedError;

  use super::BribeBucket;

  #[test]
  fn test_bribe_bucket() {
    let mut bucket = BribeBucket {
      assets: vec![],
    };

    bucket.deposit(Asset::native("uluna", 1000u128));
    bucket.deposit(Asset::native("uluna", 500u128));
    bucket.deposit(Asset::cw20(Addr::unchecked("test"), 500u128));
    bucket.withdraw(&Asset::native("uluna", 1000u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        assets: vec![
          Asset::native("uluna", 500u128),
          Asset::cw20(Addr::unchecked("test"), 500u128)
        ]
      }
    );

    bucket.withdraw(&Asset::native("uluna", 500u128)).unwrap();

    assert_eq!(
      bucket,
      BribeBucket {
        assets: vec![Asset::cw20(Addr::unchecked("test"), 500u128)]
      }
    );
  }

  #[test]
  fn test_bribe_bucket_fail_not_existing() {
    let mut bucket = BribeBucket {
      assets: vec![],
    };

    let err = bucket.withdraw(&Asset::native("uluna", 1000u128)).unwrap_err();

    assert_eq!(
      err,
      SharedError::NotFound(format!("asset {0}", AssetInfo::native("uluna".to_string())))
    )
  }

  #[test]
  fn test_bribe_bucket_fail_insufficient_balance() {
    let mut bucket = BribeBucket {
      assets: vec![],
    };

    bucket.deposit(Asset::native("uluna", 500u128));
    let err = bucket.withdraw(&Asset::native("uluna", 1000u128)).unwrap_err();

    assert_eq!(
      err,
      SharedError::InsufficientBalance(format!(
        "existing: {0} withdrawing: {1}",
        Asset::native("uluna", 500u128).to_string(),
        Asset::native("uluna", 1000u128).to_string()
      ),)
    )
  }
}
