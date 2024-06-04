use cosmwasm_std::{Addr, QuerierWrapper};
use cw_asset::{Asset, AssetInfo};

use crate::error::SharedError;

use super::asset_info_ext::AssetInfoExt;

pub trait AssetInfosEx {
    fn with_balance_query(
        &self,
        querier: &QuerierWrapper,
        address: &Addr,
    ) -> Result<Vec<Asset>, SharedError>;
}

impl AssetInfosEx for Vec<AssetInfo> {
    fn with_balance_query(
        &self,
        querier: &QuerierWrapper,
        address: &Addr,
    ) -> Result<Vec<Asset>, SharedError> {
        let assets: Vec<Asset> = self
            .iter()
            .map(|asset| Ok(asset.with_balance_query(querier, address)?))
            .collect::<Result<Vec<Asset>, SharedError>>()?;

        Ok(assets.into_iter().collect())
    }
}
