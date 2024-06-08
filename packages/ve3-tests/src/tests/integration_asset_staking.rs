use crate::common::{helpers::u, suite::TestingSuite};
use cosmwasm_std::{Addr, Decimal};
use cw_asset::AssetInfo;
use ve3_asset_staking::error::ContractError;
use ve3_shared::{
  constants::AT_ASSET_WHITELIST_CONTROLLER, error::SharedError, msgs_asset_staking::*,
};

#[test]
fn test_add_remove_asset() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_staking_whitelist_assets(
      vec![AssetInfo::native("native_lp").into(), AssetInfo::cw20(addr.lp_cw20.clone()).into()],
      "user1",
      |res| {
        let res = res.unwrap_err().downcast::<ContractError>().unwrap();
        assert_eq!(
          res,
          SharedError::UnauthorizedMissingRight(
            AT_ASSET_WHITELIST_CONTROLLER.to_string(),
            addr.user1.to_string()
          )
          .into()
        );
      },
    )
    .e_staking_whitelist_assets(
      vec![AssetInfo::native("native_lp").into(), AssetInfo::cw20(addr.lp_cw20.clone()).into()],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_assets(|res| {
      assert_eq!(
        res.unwrap(),
        vec![AssetInfo::cw20(addr.lp_cw20.clone()), AssetInfo::native("native_lp")]
      )
    })
    .e_staking_remove_assets(vec![AssetInfo::cw20(addr.lp_cw20.clone())], "user1", |res| {
      let res = res.unwrap_err().downcast::<ContractError>().unwrap();
      assert_eq!(
        res,
        SharedError::UnauthorizedMissingRight(
          AT_ASSET_WHITELIST_CONTROLLER.to_string(),
          addr.user1.to_string()
        )
        .into()
      );
    })
    .e_staking_remove_assets(
      vec![AssetInfo::cw20(addr.lp_cw20.clone())],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_assets(|res| {
      assert_eq!(res.unwrap(), vec![AssetInfo::native("native_lp")])
    });
}

#[test]
fn test_asset_config() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();

  suite
    .e_staking_whitelist_assets(
      vec![
        AssetInfoWithConfig::new(
          AssetInfo::native("native_lp"),
          Some(AssetConfig {
            yearly_take_rate: Decimal::percent(20),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: Addr::unchecked("test"),
              reward_infos: vec![AssetInfo::native("ibcastro")],
            },
          }),
        ),
        AssetInfo::cw20(addr.lp_cw20.clone()).into(),
      ],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .e_staking_remove_assets(
      vec![AssetInfo::cw20(addr.lp_cw20.clone())],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
    .q_staking_whitelisted_asset_details(|res| {
      assert_eq!(
        res.unwrap(),
        vec![
          AssetInfoWithRuntime {
            whitelisted: false,
            info: AssetInfo::cw20(addr.lp_cw20.clone()),
            config: AssetConfigRuntime {
              yearly_take_rate: Decimal::percent(10),
              stake_config: ve3_shared::stake_config::StakeConfig::Default,
              last_taken_s: 0,
              taken: u(0),
              harvested: u(0)
            }
          },
          AssetInfoWithRuntime {
            whitelisted: true,
            info: AssetInfo::native("native_lp"),
            config: AssetConfigRuntime {
              yearly_take_rate: Decimal::percent(20),
              stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
                contract: Addr::unchecked("test"),
                reward_infos: vec![AssetInfo::native("ibcastro")],
              },
              last_taken_s: 0,
              taken: u(0),
              harvested: u(0)
            }
          },
        ]
      )
    });
}
