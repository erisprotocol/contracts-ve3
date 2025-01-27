use std::convert::TryInto;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coins, CosmosMsg, Uint128};
use cw_asset::{Asset, AssetInfo};

use crate::{error::SharedError, extensions::asset_info_ext::AssetInfoExt};

#[cw_serde]
#[derive(Default)]
pub struct Assets(pub Vec<Asset>);

impl IntoIterator for Assets {
  type Item = Asset;
  type IntoIter = std::vec::IntoIter<Asset>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl From<Vec<Asset>> for Assets {
  fn from(value: Vec<Asset>) -> Self {
    let mut assets = Assets::default();

    for asset in value {
      assets.add(&asset);
    }

    assets
  }
}

impl From<Asset> for Assets {
  fn from(value: Asset) -> Self {
    let mut assets = Assets::default();
    assets.add(&value);
    assets
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

  pub fn remove_overtaken(&mut self, asset: &mut Asset) -> Result<Option<Asset>, SharedError> {
    let existing = self.0.iter_mut().find(|a| a.info == asset.info);

    match existing {
      Some(existing) if existing.amount < asset.amount => {
        let overtaken_amount = asset.amount - existing.amount;
        let overtaken = existing.info.with_balance(overtaken_amount);
        asset.amount -= overtaken_amount;

        existing.amount = Uint128::zero();

        if existing.amount.is_zero() {
          self.0.retain(|a| !a.amount.is_zero())
        }

        Ok(Some(overtaken))
      },
      Some(existing) => {
        existing.amount -= asset.amount;

        if existing.amount.is_zero() {
          self.0.retain(|a| !a.amount.is_zero())
        }
        Ok(None)
      },
      None => Err(SharedError::NotFound(format!("asset {0}", asset.info))),
    }
  }

  pub fn get(&mut self, info: &AssetInfo) -> Option<Asset> {
    self.0.iter().find(|a| a.info == *info).cloned()
  }

  pub fn get_mut(&mut self, info: &AssetInfo) -> Option<&mut Asset> {
    self.0.iter_mut().find(|a| a.info == *info)
  }

  pub fn remove_multi(&mut self, assets: &Vec<Asset>) -> Result<(), SharedError> {
    for asset in assets {
      self.remove(asset)?;
    }

    Ok(())
  }

  pub fn remove_multi_overtaken(
    &mut self,
    assets: &mut Vec<Asset>,
  ) -> Result<Vec<Asset>, SharedError> {
    let mut overtakens = vec![];
    for asset in assets {
      if let Some(overtaken) = self.remove_overtaken(asset)? {
        overtakens.push(overtaken);
      }
    }

    Ok(overtakens)
  }

  pub fn add(&mut self, asset: &Asset) {
    if asset.amount.is_zero() {
      return;
    }

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

  pub fn get_coins(&self) -> Result<Coins, SharedError> {
    let mut coins = Coins::default();
    for native in self.0.iter().filter(|a| a.info.is_native()) {
      coins.add(native.try_into()?)?;
    }
    Ok(coins)
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
    let mut results = vec![];
    if total_vp.is_zero() {
      return Ok(results);
    }

    for a in &self.0 {
      let share_amount = a.amount.checked_multiply_ratio(vp, total_vp)?;

      if share_amount.is_zero() {
        continue;
      }

      results.push(a.info.with_balance(share_amount))
    }

    Ok(results)
  }

  pub fn transfer_msgs(&self, to: &Addr) -> Result<Vec<CosmosMsg>, SharedError> {
    let mut results = vec![];
    for asset in self.0.iter() {
      results.push(asset.transfer_msg(to)?);
    }
    Ok(results)
  }
}
