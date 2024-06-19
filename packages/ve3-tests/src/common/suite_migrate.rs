use cosmwasm_std::{Addr, Decimal};
use cw_asset::{AssetInfoBase, AssetInfoUnchecked};
use cw_multi_test::Executor;
use serde::Serialize;
use ve3_shared::{msgs_connector_alliance, msgs_connector_emission};

use super::{helpers::u, suite::TestingSuite};
use crate::common::suite_contracts::*;

impl TestingSuite {
  #[track_caller]
  fn migrate_contract<T: Serialize>(&mut self, contract: &Addr, code_id: u64, msg: T) {
    let creator = self.creator().clone();
    self.app.migrate_contract(creator, contract.clone(), &msg, code_id).unwrap();
  }

  pub(crate) fn migrate(&mut self) -> &mut Self {
    let addr = self.addresses.clone();

    let code_id = self.app.store_code(ve3_global_config());
    let msg = ve3_shared::msgs_global_config::MigrateMsg {};
    self.migrate_contract(&addr.ve3_global_config, code_id, msg);

    let code_id = self.app.store_code(ve3_asset_gauge());
    let msg = ve3_shared::msgs_asset_gauge::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_gauge, code_id, msg);

    let code_id = self.app.store_code(ve3_asset_staking());
    let msg = ve3_shared::msgs_asset_staking::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_staking_1, code_id, msg);
    let msg = ve3_shared::msgs_asset_staking::MigrateMsg {};
    self.migrate_contract(&addr.ve3_asset_staking_2, code_id, msg);

    let code_id = self.app.store_code(ve3_bribe_manager());
    let msg = ve3_shared::msgs_bribe_manager::MigrateMsg {};
    self.migrate_contract(&addr.ve3_bribe_manager, code_id, msg);

    let code_id = self.app.store_code(ve3_voting_escrow());
    let msg = ve3_shared::msgs_voting_escrow::MigrateMsg {};
    self.migrate_contract(&addr.ve3_voting_escrow, code_id, msg);

    // TEST ALLIANCE CONNECTOR
    let code_id = self.app.store_code(ve3_connector_alliance());
    let init = msgs_connector_alliance::InstantiateMsg {
      alliance_token_denom: "test".to_string(),
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_1.clone(),
      reward_denom: "uluna".to_string(),

      zasset_denom: "zluna".to_string(),
      lst_hub_address: self.addresses.eris_hub.to_string(),
      lst_asset_info: AssetInfoUnchecked::cw20(self.addresses.eris_hub_cw20_ampluna.to_string()),
    };
    let alliance_connector = self
      .app
      .instantiate_contract(
        code_id,
        addr.creator.clone(),
        &init,
        &[],
        "init-connector",
        Some(addr.creator.to_string()),
      )
      .unwrap();
    let code_id = self.app.store_code(ve3_connector_alliance());
    let msg = ve3_shared::msgs_connector_alliance::MigrateMsg {};
    self.migrate_contract(&alliance_connector, code_id, msg);

    // TEST EMISSION CONNECTOR
    let code_id = self.app.store_code(ve3_connector_emission());
    let init = msgs_connector_emission::InstantiateMsg {
      global_config_addr: self.addresses.ve3_global_config.to_string(),
      gauge: self.addresses.gauge_1.clone(),
      emission_token: AssetInfoBase::Native("uluna".to_string()),
      emissions_per_week: u(100),
      mint_config: msgs_connector_emission::MintConfig::MintDirect,
      rebase_config: msgs_connector_emission::RebaseConfg::Fixed(Decimal::percent(10)),
      team_share: Decimal::percent(10),
    };
    let emission_connector = self
      .app
      .instantiate_contract(
        code_id,
        addr.creator.clone(),
        &init,
        &[],
        "init-emission",
        Some(addr.creator.to_string()),
      )
      .unwrap();
    let code_id = self.app.store_code(ve3_connector_emission());
    let msg = ve3_shared::msgs_connector_emission::MigrateMsg {};
    self.migrate_contract(&emission_connector, code_id, msg);

    self
  }
}
