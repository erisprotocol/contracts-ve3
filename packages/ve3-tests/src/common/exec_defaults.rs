use crate::extensions::app_response_ext::Valid;

use super::suite::TestingSuite;
use cosmwasm_std::Decimal;
use cw_asset::{AssetInfo, AssetInfoUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::msgs_asset_staking::{AssetConfig, AssetInfoWithConfig};

#[allow(dead_code)]
impl TestingSuite {
  pub fn def_staking_whitelist_recapture(&mut self) -> &mut Self {
    let addr = self.addresses.clone();
    self.e_staking_whitelist_assets(
      vec![
        AssetInfoWithConfig::new(
          AssetInfoUnchecked::native("lp"),
          Some(AssetConfig {
            yearly_take_rate: Decimal::percent(20),
            stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
              contract: addr.incentive_mock.to_string(),
              reward_infos: vec![AssetInfoUnchecked::native("astro")],
            },
          }),
        ),
        AssetInfoUnchecked::cw20(addr.lp_cw20.clone()).into(),
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

  pub fn def_gauge_1_vote(
    &mut self,
    lp: u16,
    cw20: u16,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let addr = self.addresses.clone();
    let allowed_cw20 = addr.lp_cw20.to_string();
    let msg = ve3_shared::msgs_asset_gauge::ExecuteMsg::Vote {
      gauge: addr.gauge_1.to_string(),
      votes: vec![("native:lp".to_string(), lp), (format!("cw20:{allowed_cw20}"), cw20)],
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, addr.ve3_asset_gauge.clone(), &msg, &[]));
    self
  }

  pub fn def_gauge_2_vote(
    &mut self,
    lp: u16,
    cw20: u16,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let addr = self.addresses.clone();
    let allowed_cw20 = addr.lp_cw20.to_string();
    let msg = ve3_shared::msgs_asset_gauge::ExecuteMsg::Vote {
      gauge: addr.gauge_2.to_string(),
      votes: vec![("native:lp".to_string(), lp), (format!("cw20:{allowed_cw20}"), cw20)],
    };
    let sender = self.address(sender);
    result(self.app.execute_contract(sender, addr.ve3_asset_gauge.clone(), &msg, &[]));
    self
  }

  pub fn def_asset_config_astro(
    &mut self,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let addr = self.addresses.clone();

    self.e_staking_whitelist_assets(
      vec![AssetInfoWithConfig::new(
        AssetInfoUnchecked::native("lp"),
        Some(AssetConfig {
          yearly_take_rate: Decimal::percent(10),
          stake_config: ve3_shared::stake_config::StakeConfig::Astroport {
            contract: addr.incentive_mock.to_string(),
            reward_infos: vec![AssetInfoUnchecked::native("astro")],
          },
        }),
      )],
      "AT_ASSET_WHITELIST_CONTROLLER",
      result,
    )
  }

  pub fn def_asset_config_no_staking(&mut self) -> &mut TestingSuite {
    let addr = self.addresses.clone();

    self.e_staking_whitelist_assets(
      vec![
        AssetInfoWithConfig::new(
          AssetInfoUnchecked::native("lp"),
          Some(AssetConfig {
            yearly_take_rate: Decimal::percent(10),
            stake_config: ve3_shared::stake_config::StakeConfig::Default,
          }),
        ),
        AssetInfo::cw20(addr.lp_cw20.clone()).into(),
      ],
      "AT_ASSET_WHITELIST_CONTROLLER",
      |res| res.assert_valid(),
    )
  }
}
