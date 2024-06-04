use cosmwasm_std::{Addr, QuerierWrapper, Uint128};
use cw_asset::{Asset, AssetError, AssetInfo};

pub trait AssetInfoExt {
    /// simplifies converting an AssetInfo to an Asset with balance
    fn with_balance(&self, balance: Uint128) -> Asset;
    fn with_balance_u128(&self, amount: u128) -> Asset;

    fn with_balance_query(
        &self,
        querier: &QuerierWrapper,
        address: &Addr,
    ) -> Result<Asset, AssetError>;
    
}

impl AssetInfoExt for AssetInfo {
    fn with_balance(&self, amount: Uint128) -> Asset {
        match self {
            cw_asset::AssetInfoBase::Native(denom) => Asset::native(denom, amount),
            cw_asset::AssetInfoBase::Cw20(contract_addr) => {
                Asset::cw20(contract_addr.clone(), amount)
            },
            _ => todo!(),
        }
    }

    fn with_balance_u128(&self, amount: u128) -> Asset {
        self.with_balance(Uint128::new(amount))
    }

    fn with_balance_query(
        &self,
        querier: &QuerierWrapper,
        address: &Addr,
    ) -> Result<Asset, AssetError> {
        let balance = self.query_balance(querier, address.clone())?;
        Ok(self.with_balance(balance))
    }
}
