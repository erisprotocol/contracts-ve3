use crate::{
  common::suite::TestingSuite,
  extensions::app_response_ext::{EventChecker, Valid},
};
use cosmwasm_std::{attr, Decimal};
use cw_asset::AssetInfoUnchecked;
use ve3_shared::msgs_asset_staking::*;

#[test]
fn test_asset_take_rate_multi_rebase() {
  let mut suite = TestingSuite::def();
  suite.init();

  let addr = suite.addresses.clone();
  let take_recipient = suite.address("AT_TAKE_RECIPIENT");

  suite
    .def_asset_config_astro(|res| {
      res.assert_attribute(attr("action", "asset/whitelist_assets"));
    })
    .e_staking_whitelist_assets(
      vec![AssetInfoWithConfig::new(
        addr.fake_native(0).info.into(),
        Some(AssetConfig {
          yearly_take_rate: Some(Decimal::percent(10)),
          stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
            contract: addr.incentive_mock.to_string(),
            reward_infos: vec![AssetInfoUnchecked::native("astro")],
          },
        }),
      )],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| res.assert_valid(),
    )
    .e_staking_stake(None, addr.lp_native(10_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "10000000"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", "native:lp:10000000"));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .e_staking_stake(None, addr.fake_native(10_000_000), "user1", |res| {
      res.assert_attribute(attr("action", "asset/stake"));
      res.assert_attribute(attr("share", "10000000"));
      res.assert_attribute(attr("action", "mock/deposit"));
      res.assert_attribute(attr("mock/amount", "native:xxx:10000000"));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
    })
    .add_one_period()
    .e_staking_distribute_take_rate(Some(true), None, "user1", |res| {
      res.assert_attribute(attr("action", "asset/distribute_take_rate"));
      res.assert_attribute(attr("take", "native:lp:19178"));
      res.assert_attribute(attr("action", "mock/withdraw"));
      res.assert_attribute(attr("mock/amount", "native:lp:19178"));
      res.assert_attribute(attr("action", "asset/track_bribes_callback"));
      // setup to receive 10000 astro per week
      res.assert_attribute(attr("bribe", "native:astro:10000"));
      res.assert_attribute(attr("bribe", "native:astro:5000"));
      res.assert_attribute_ty("transfer", attr("recipient", take_recipient.to_string()));
      // 7 / 365 * 10% * 10_000000 = 19,178
      res.assert_attribute_ty("transfer", attr("amount", "19178lp"));
    });
}
