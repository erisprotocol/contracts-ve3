use crate::contract::execute;
use crate::error::ContractError;
use crate::query::query;
use crate::state::WHITELIST;
use crate::tests::helpers::{mock_dependencies, remove_assets, setup_contract, whitelist_assets};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_json, Response};
use cw_asset::AssetInfo;
use ve3_shared::msgs_asset_staking::*;

#[test]
fn test_whitelist_assets() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let res = whitelist_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string()).into()]);
  assert_eq!(
    res,
    Response::default()
      .add_attributes(vec![("action", "asset/whitelist_assets"), ("assets", "native:asset1")])
  );

  let res = whitelist_assets(
    deps.as_mut(),
    vec![
      AssetInfo::Native("asset2".to_string()).into(),
      AssetInfo::Native("asset3".to_string()).into(),
    ],
  );
  assert_eq!(
    res,
    Response::default().add_attributes(vec![
      ("action", "asset/whitelist_assets"),
      ("assets", "native:asset2,native:asset3")
    ])
  );

  let exists =
    WHITELIST.load(deps.as_ref().storage, &AssetInfo::Native("asset2".to_string())).unwrap();

  assert!(exists);

  let res: WhitelistedAssetsResponse =
    from_json(query(deps.as_ref(), mock_env(), QueryMsg::WhitelistedAssets {}).unwrap()).unwrap();
  assert_eq!(
    res,
    vec![
      AssetInfo::Native("asset1".to_string()),
      AssetInfo::Native("asset2".to_string()),
      AssetInfo::Native("asset3".to_string())
    ]
  );
}

#[test]
fn test_whitelist_asset_unauthorized() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let err = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("admin", &[]),
    ExecuteMsg::WhitelistAssets(vec![AssetInfo::Native("asset1".to_string()).into()]),
  )
  .unwrap_err();
  assert_eq!(
    err,
    ContractError::SharedError(ve3_shared::error::SharedError::UnauthorizedMissingRight(
      "ASSET_WHITELIST_CONTROLLER".to_string(),
      "admin".to_string()
    ))
  );
}

#[test]
fn test_remove_assets() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  whitelist_assets(
    deps.as_mut(),
    vec![
      AssetInfo::Native("asset1".to_string()).into(),
      AssetInfo::Native("asset2".to_string()).into(),
    ],
  );

  let response = remove_assets(deps.as_mut(), vec![AssetInfo::Native("asset1".to_string())]);
  assert_eq!(
    response,
    Response::default()
      .add_attributes(vec![("action", "asset/remove_assets"), ("assets", "native:asset1")])
  );

  WHITELIST.load(deps.as_ref().storage, &AssetInfo::Native("asset1".to_string())).unwrap_err();
}

#[test]
fn test_remove_assets_unauthorized() {
  let mut deps = mock_dependencies();
  setup_contract(deps.as_mut());
  let err = execute(
    deps.as_mut(),
    mock_env(),
    mock_info("admin", &[]),
    ExecuteMsg::RemoveAssets(vec![AssetInfo::Native("".to_string())]),
  )
  .unwrap_err();
  assert_eq!(
    err,
    ContractError::SharedError(ve3_shared::error::SharedError::UnauthorizedMissingRight(
      "ASSET_WHITELIST_CONTROLLER".to_string(),
      "admin".to_string()
    ))
  );
}
