use crate::extensions::app_response_ext::Valid;

use super::suite::TestingSuite;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked};
use cw_multi_test::{AppResponse, Executor};
use ve3_shared::{
  extensions::{asset_ext::AssetExt, asset_info_ext::AssetInfoExt},
  msgs_asset_staking::{AssetConfig, AssetInfoWithConfig},
};

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

  pub fn def_gauge_3_vote(
    &mut self,
    lp: u16,
    cw20: u16,
    sender: &str,
    result: impl Fn(Result<AppResponse, anyhow::Error>),
  ) -> &mut TestingSuite {
    let addr = self.addresses.clone();
    let allowed_cw20 = addr.lp_cw20.to_string();
    let msg = ve3_shared::msgs_asset_gauge::ExecuteMsg::Vote {
      gauge: addr.gauge_3.to_string(),
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

  pub fn def_get_ampluna(&mut self, sender: &str, amount: u32) -> &mut TestingSuite {
    let addr = self.addresses.clone();

    let sender = self.address(sender);
    let res = self.app.execute_contract(
      sender.clone(),
      self.addresses.eris_hub.clone(),
      &eris::hub::ExecuteMsg::Bond {
        receiver: None,
      },
      &[addr.uluna(amount).to_coin().unwrap()],
    );
    res.assert_valid();
    self
  }
  pub fn def_harvest(&mut self) -> &mut TestingSuite {
    let sender = self.address("creator");
    let res = self.app.execute_contract(
      sender.clone(),
      self.addresses.eris_hub.clone(),
      &eris::hub::ExecuteMsg::Harvest {},
      &[],
    );
    res.assert_valid();
    self
  }

  pub fn def_send(&mut self, sender: &str, to: Addr, asset: Asset) -> &mut TestingSuite {
    let sender = self.address(sender);
    self.app.execute(sender, asset.transfer_msg(to).unwrap()).assert_valid();
    self
  }

  pub fn def_change_exchange_rate(&mut self, goal: Decimal) -> &mut TestingSuite {
    let addr = self.addresses.clone();
    let sender = self.address("creator");
    let contract = self.addresses.eris_hub.clone();

    self
      .app
      .execute_contract(
        sender.clone(),
        contract.clone(),
        &eris::hub::ExecuteMsg::UpdateConfig {
          protocol_fee_contract: None,
          protocol_reward_fee: None,
          allow_donations: Some(true),
          delegation_strategy: None,
          vote_operator: None,
          epoch_period: None,
          unbond_period: None,
        },
        &[],
      )
      .assert_valid();

    let state: eris::hub::StateResponse =
      self.app.wrap().query_wasm_smart(contract.clone(), &eris::hub::QueryMsg::State {}).unwrap();

    if state.total_uluna.is_zero() {
      self
        .app
        .execute_contract(
          sender.clone(),
          contract.clone(),
          &eris::hub::ExecuteMsg::Bond {
            receiver: None,
          },
          &[addr.uluna(1000).to_coin().unwrap()],
        )
        .assert_valid();

      let donation = goal * Uint128::new(1000) - Uint128::new(1000);

      self
        .app
        .execute_contract(
          sender.clone(),
          contract.clone(),
          &eris::hub::ExecuteMsg::Donate {},
          &[addr.uluna_info_checked().with_balance(donation).to_coin().unwrap()],
        )
        .assert_valid();
    } else {
      let goal_amount = state.total_ustake * goal;
      let missing = goal_amount - state.total_uluna;

      self
        .app
        .execute_contract(
          sender.clone(),
          contract.clone(),
          &eris::hub::ExecuteMsg::Donate {},
          &[addr.uluna_info_checked().with_balance(missing).to_coin().unwrap()],
        )
        .assert_valid();
    }

    self
  }

  pub fn def_setup_staking(&mut self) -> &mut TestingSuite {
    let addr = self.addresses.clone();

    self
      .use_connector_alliance_eris()
      .use_staking_2()
      .e_ve_create_lock_time_any(None, addr.uluna(1200), "user1", |res| res.assert_valid())
      .e_ve_create_lock_time_any(None, addr.ampluna(2000), "user2", |res| res.assert_valid())
      .def_staking_whitelist_recapture()
      // 600 - 600
      .def_gauge_2_vote(5000, 5000, "user1", |res| res.assert_valid())
      // 1800 - 600
      // = 2400 - 1200 = 2:1
      .def_gauge_2_vote(7500, 2500, "user2", |res| res.assert_valid())
      .add_one_period()
      .e_gauge_set_distribution("user1", |res| res.assert_valid());
    self
  }

  pub fn def_add_staking_rewards(&mut self, amount: u32) -> &mut TestingSuite {
    let addr = self.addresses.clone();

    self
      .def_send("creator", addr.ve3_connector_alliance_eris.clone(), addr.uluna(amount))
      .e_staking_update_rewards("user1", |res| res.assert_valid());
    self
  }
}
