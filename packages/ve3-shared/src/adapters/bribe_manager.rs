use cosmwasm_std::{coin, to_json_binary, Addr, CosmosMsg, WasmMsg};
use cw_asset::{Asset, AssetInfo};

use crate::{
  error::SharedError,
  extensions::asset_ext::AssetExt,
  msgs_bribe_manager::{BribeDistribution, ExecuteMsg},
};

pub struct BribeManager(pub Addr);

impl BribeManager {
  pub fn add_bribe_msgs(
    &self,
    bribe: Asset,
    gauge: String,
    for_info: AssetInfo,
    block_height: u64,
  ) -> Result<Vec<CosmosMsg>, SharedError> {
    let res = match &bribe.info {
      cw_asset::AssetInfoBase::Native(denom) => {
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
          contract_addr: self.0.to_string(),
          msg: to_json_binary(&ExecuteMsg::AddBribe {
            bribe: bribe.clone().into(),
            gauge,
            for_info: for_info.into(),
            distribution: BribeDistribution::Next,
          })?,
          funds: vec![coin(bribe.amount.u128(), denom)],
        })]
      },
      cw_asset::AssetInfoBase::Cw20(_) => vec![
        // increase allowance
        bribe.increase_allowance_msg(
          self.0.to_string(),
          Some(cw20::Expiration::AtHeight(block_height + 1)),
        )?,
        // register bribe
        CosmosMsg::Wasm(WasmMsg::Execute {
          contract_addr: self.0.to_string(),
          msg: to_json_binary(&ExecuteMsg::AddBribe {
            bribe: bribe.into(),
            gauge,
            for_info: for_info.into(),
            distribution: BribeDistribution::Next,
          })?,
          funds: vec![],
        }),
      ],
      _ => todo!(),
    };

    Ok(res)
  }
}
