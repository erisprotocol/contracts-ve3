use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};
use ve3_shared::helpers::token_factory::CustomExecuteMsg;

use crate::mocks::{alliance_rewards_mock, eris_hub_mock};

pub fn ve3_global_config() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_global_config::contract::execute,
    ve3_global_config::contract::instantiate,
    ve3_global_config::query::query,
  )
  .with_migrate(ve3_global_config::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_asset_gauge() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_asset_gauge::contract::execute,
    ve3_asset_gauge::contract::instantiate,
    ve3_asset_gauge::query::query,
  )
  .with_migrate(ve3_asset_gauge::contract::migrate);

  Box::new(contract)
}

pub fn ve3_asset_staking() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_asset_staking::contract::execute,
    ve3_asset_staking::contract::instantiate,
    ve3_asset_staking::query::query,
  )
  .with_migrate(ve3_asset_staking::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_bribe_manager() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_bribe_manager::contract::execute,
    ve3_bribe_manager::contract::instantiate,
    ve3_bribe_manager::query::query,
  )
  .with_migrate(ve3_bribe_manager::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_connector_alliance() -> Box<dyn Contract<CustomExecuteMsg>> {
  let contract = ContractWrapper::new(
    ve3_connector_alliance::contract::execute,
    ve3_connector_alliance::contract::instantiate,
    ve3_connector_alliance::query::query,
  )
  .with_migrate(ve3_connector_alliance::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_voting_escrow() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_voting_escrow::contract::execute,
    ve3_voting_escrow::contract::instantiate,
    ve3_voting_escrow::query::query,
  )
  .with_migrate(ve3_voting_escrow::contract::migrate);

  Box::new(contract)
}

pub fn alliance_rewards_mock() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    alliance_rewards_mock::execute,
    alliance_rewards_mock::instantiate,
    alliance_rewards_mock::query,
  );

  Box::new(contract)
}

pub fn eris_hub_mock() -> Box<dyn Contract<Empty>> {
  let contract =
    ContractWrapper::new(eris_hub_mock::execute, eris_hub_mock::instantiate, eris_hub_mock::query);

  Box::new(contract)
}

pub fn eris_hub_cw20_mock() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    eris_staking_token::execute,
    eris_staking_token::instantiate,
    eris_staking_token::query,
  );

  Box::new(contract)
}
