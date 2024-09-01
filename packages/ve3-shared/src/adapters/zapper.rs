use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Uint128, WasmMsg};
use cw_asset::{Asset, AssetInfoUnchecked};

use crate::{error::SharedError, extensions::asset_ext::AssetExt, msgs_zapper};

pub struct Zapper(pub Addr);

impl Zapper {
  pub fn swap_msgs(
    &self,
    into: AssetInfoUnchecked,
    assets: Vec<Asset>,
    min_received: Option<Uint128>,
    receiver: Option<String>,
  ) -> Result<Vec<CosmosMsg>, SharedError> {
    let mut funds = vec![];
    let mut msgs = vec![];
    let mut infos = vec![];

    for asset in assets {
      match asset.info {
        cw_asset::AssetInfoBase::Native(_) => funds.push(asset.to_coin()?),
        cw_asset::AssetInfoBase::Cw20(_) => msgs.push(asset.transfer_msg(self.0.clone())?),
        _ => return Err(SharedError::NotSupportedAssetInfo()),
      }
      infos.push(asset.info);
    }

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: self.0.to_string(),
      msg: to_json_binary(&msgs_zapper::ExecuteMsg::Swap {
        into,
        assets: infos,
        min_received,
        receiver,
      })?,
      funds,
    }));

    Ok(msgs)
  }
}
