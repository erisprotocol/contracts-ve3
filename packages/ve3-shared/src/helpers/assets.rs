use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Uint128};
use cw_asset::Asset;

use crate::{error::SharedError, extensions::asset_info_ext::AssetInfoExt};

#[cw_serde]
#[derive(Default)]
pub struct Assets(Vec<Asset>);

impl IntoIterator for Assets {
  type Item = Asset;
  type IntoIter = std::vec::IntoIter<Asset>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl From<Vec<Asset>> for Assets {
  fn from(value: Vec<Asset>) -> Self {
    Assets(value)
  }
}

impl Assets {
  pub fn is_empty(&self) -> bool {
    self.0.len() == 0
  }

  pub fn remove(&mut self, asset: &Asset) -> Result<(), SharedError> {
    let existing = self.0.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(existing) if existing.amount < asset.amount => Err(SharedError::InsufficientBalance(
        format!("existing: {0} withdrawing: {1}", existing, asset),
      )),
      Some(existing) => {
        existing.amount -= asset.amount;

        if existing.amount.is_zero() {
          self.0.retain(|a| !a.amount.is_zero())
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
    let existing = self.0.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(in_bucket) => {
        in_bucket.amount += asset.amount;
      },
      None => {
        self.0.push(asset.clone());
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
  ) -> Result<Vec<Asset>, SharedError> {
    self
      .0
      .iter()
      .map(|a| {
        a.amount
          .checked_multiply_ratio(vp, total_vp)
          .map(|amount| a.info.with_balance(amount))
          .map_err(SharedError::CheckedMultiplyRatioError)
      })
      .collect()
  }

  pub fn transfer_msgs(&self, to: &Addr) -> Result<Vec<CosmosMsg>, SharedError> {
    let mut results = vec![];
    for asset in self.0.iter() {
      results.push(asset.transfer_msg(to)?);
    }
    Ok(results)
  }
}
