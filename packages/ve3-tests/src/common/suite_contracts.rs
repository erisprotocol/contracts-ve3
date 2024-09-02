use crate::mocks::{
  alliance_rewards_mock, astroport_pair_mock, eris_hub_mock, incentive_mock, zapper_mock,
};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

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
  .with_migrate(ve3_asset_gauge::migrate::migrate);

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

pub fn ve3_connector_alliance() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_connector_alliance::contract::execute,
    ve3_connector_alliance::contract::instantiate,
    ve3_connector_alliance::query::query,
  )
  .with_migrate(ve3_connector_alliance::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_connector_emission() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_connector_emission::contract::execute,
    ve3_connector_emission::contract::instantiate,
    ve3_connector_emission::query::query,
  )
  .with_migrate(ve3_connector_emission::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_voting_escrow() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_voting_escrow::contract::execute,
    ve3_voting_escrow::contract::instantiate,
    ve3_voting_escrow::query::query,
  )
  .with_migrate(ve3_voting_escrow::migrate::migrate);

  Box::new(contract)
}

pub fn ve3_zapper() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    ve3_zapper::contract::execute,
    ve3_zapper::contract::instantiate,
    ve3_zapper::query::query,
  )
  .with_migrate(ve3_zapper::migrate::migrate);

  Box::new(contract)
}
pub fn ve3_zapper_mock() -> Box<dyn Contract<Empty>> {
  let contract =
    ContractWrapper::new(zapper_mock::execute, zapper_mock::instantiate, zapper_mock::query);

  Box::new(contract)
}

pub fn pdt() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    phoenix_treasury::contract::execute,
    phoenix_treasury::contract::instantiate,
    phoenix_treasury::query::query,
  )
  .with_migrate(phoenix_treasury::migrate::migrate);

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

pub fn astroport_pair_mock() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    astroport_pair_mock::execute,
    astroport_pair_mock::instantiate,
    astroport_pair_mock::query,
  );

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

pub fn eris_hub() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    eris_staking_hub::contract::execute,
    eris_staking_hub::contract::instantiate,
    eris_staking_hub::contract::query,
  )
  .with_migrate(eris_staking_hub::contract::migrate)
  .with_reply(eris_staking_hub::contract::reply);

  Box::new(contract)
}

pub fn incentive_mock() -> Box<dyn Contract<Empty>> {
  let contract = ContractWrapper::new(
    incentive_mock::execute,
    incentive_mock::instantiate,
    incentive_mock::query,
  );

  Box::new(contract)
}
