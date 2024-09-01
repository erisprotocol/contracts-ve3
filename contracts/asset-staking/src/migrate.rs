use crate::{
  constants::{CONTRACT_NAME, CONTRACT_VERSION},
  error::ContractError,
  state::ASSET_BRIBES,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, Order, Response, StdResult, Uint128};
use cw2::{get_contract_version, set_contract_version};
use cw_asset::AssetInfo;
use ve3_shared::{
  error::SharedError, extensions::asset_info_ext::AssetInfoExt, msgs_global_config::MigrateMsg,
};

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
  let contract_version = get_contract_version(deps.storage)?;
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  if contract_version.contract != CONTRACT_NAME {
    return Err(
      SharedError::ContractMismatch(contract_version.contract, CONTRACT_VERSION.to_string()).into(),
    );
  }

  if CONTRACT_VERSION == "1.0.2" {
    let astro =
      AssetInfo::native("ibc/8D8A7F7253615E5F76CB6252A1E1BD921D5EDB7BBAAF8913FB1C77FF125D9995")
        .with_balance_query(&deps.querier, &env.contract.address)?;

    let bribes = ASSET_BRIBES
      .range(deps.storage, None, None, Order::Ascending)
      .collect::<StdResult<Vec<_>>>()?;

    let total: Uint128 = bribes
      .iter()
      .map(|a| a.clone().1.get(&astro.info).map(|a| a.amount).unwrap_or_default())
      .sum();

    for (asset, mut bribe) in bribes {
      let existing = bribe.get(&astro.info);

      if let Some(existing) = existing {
        let real_amount = existing.amount.multiply_ratio(astro.amount, total);
        let reduction = existing.amount - real_amount;
        bribe.remove(&astro.info.with_balance(reduction))?;
        ASSET_BRIBES.save(deps.storage, &asset, &bribe)?;
      }
    }
  }

  Ok(
    Response::new()
      .add_attribute("previous_contract_name", &contract_version.contract)
      .add_attribute("previous_contract_version", &contract_version.version)
      .add_attribute("new_contract_name", CONTRACT_NAME)
      .add_attribute("new_contract_version", CONTRACT_VERSION),
  )
}
