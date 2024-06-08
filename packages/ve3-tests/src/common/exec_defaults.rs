use super::suite::TestingSuite;
use cosmwasm_std::{Addr, Decimal};
use cw_asset::AssetInfo;
use ve3_shared::msgs_asset_staking::*;

#[allow(dead_code)]
impl TestingSuite {
  pub fn def_staking_whitelist_recapture(&mut self) -> &mut Self {
    let addr = self.addresses.clone();
    self.e_staking_whitelist_assets(
      vec![
        AssetInfoWithConfig::new(
          AssetInfo::native("lp"),
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
  }

  /// sets lp and lp_cw20 as whitelisted assets
  pub fn init_def_staking_whitelist(&mut self) -> &mut Self {
    let addr = self.addresses.clone();
    self.e_staking_whitelist_assets(
      vec![AssetInfo::native("lp").into(), AssetInfo::cw20(addr.lp_cw20.clone()).into()],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| {
        res.unwrap();
      },
    )
  }
}
