use super::helpers::Addr;
use super::suite::TestingSuite;
use crate::common::suite_contracts::*;
use crate::extensions::app_response_ext::EventChecker;
use astroport::factory::PairConfig;
use cosmwasm_std::Addr;
use cw_asset::{Asset, AssetInfo, AssetInfoBase};
use cw_multi_test::Executor;
use std::vec;
use ve3_shared::extensions::asset_ext::AssetExt;

impl TestingSuite {
  pub(super) fn create_astroport(&mut self, big: bool) {
    let addr = self.addresses.clone();

    let factory_code_id = self.app.store_code(astroport_factory());
    let native_coin_registry_code_id = self.app.store_code(astroport_native_coin_registry());
    let pair_code_id = self.app.store_code(astroport_pair());

    let coin_registry = self.init_contract(
      native_coin_registry_code_id,
      astroport::native_coin_registry::InstantiateMsg {
        owner: addr.creator.to_string(),
      },
      "astroport_native_coin_registry",
    );

    let msg = astroport::factory::InstantiateMsg {
      tracker_config: None,
      owner: self.creator().to_string(),
      fee_address: None,
      generator_address: None,
      pair_configs: vec![PairConfig {
        code_id: pair_code_id,
        is_disabled: false,
        is_generator_disabled: true,
        permissioned: false,
        total_fee_bps: 30,
        maker_fee_bps: 10,
        pair_type: astroport::factory::PairType::Xyk {},
      }],
      coin_registry_address: coin_registry.to_string(),
      token_code_id: 0,
      whitelist_code_id: 0,
    };

    self.addresses.astroport_factory =
      self.init_contract(factory_code_id, msg, "astroport_factory");

    let res = self
      .app
      .execute_contract(
        addr.user1.clone(),
        self.addresses.astroport_factory.clone(),
        &astroport::factory::ExecuteMsg::CreatePair {
          pair_type: astroport::factory::PairType::Xyk {},
          asset_infos: vec![
            to_astro_info(addr.ampluna_info_checked()),
            to_astro_info(addr.uluna_info_checked()),
          ],
          init_params: None,
        },
        &[],
      )
      .unwrap();

    self.addresses.astroport_ampluna_luna_lp =
      res.get_attribute_value("wasm", "lp_denom".to_string());
    self.addresses.astroport_ampluna_luna_pair =
      Addr(&res.get_attribute_value("wasm", "pair_contract_addr".to_string()));

    let res = self
      .app
      .execute_contract(
        addr.user1.clone(),
        self.addresses.astroport_factory.clone(),
        &astroport::factory::ExecuteMsg::CreatePair {
          pair_type: astroport::factory::PairType::Xyk {},
          asset_infos: vec![
            to_astro_info(addr.uluna_info_checked()),
            to_astro_info(addr.usdc_info_checked()),
          ],
          init_params: None,
        },
        &[],
      )
      .unwrap();

    self.addresses.astroport_luna_usdc_lp = res.get_attribute_value("wasm", "lp_denom".to_string());
    self.addresses.astroport_luna_usdc_pair =
      Addr(&res.get_attribute_value("wasm", "pair_contract_addr".to_string()));

    self.create_lp_luna_usdc(addr.creator.clone(), big);
    self.create_lp_luna_usdc(addr.user1.clone(), big);
    self.create_lp_luna_usdc(addr.user2.clone(), big);
    self.create_lp_luna_ampluna(addr.user1.clone(), big);
    self.create_lp_luna_ampluna(addr.user2.clone(), big);
  }

  pub(super) fn create_lp_luna_usdc(&mut self, user: Addr, big: bool) {
    let addr = self.addresses.clone();

    let uluna = addr.uluna(if big {
      30000_000000
    } else {
      30000
    });
    let usdc = addr.usdc(if big {
      10000_000000
    } else {
      10000
    });

    self
      .app
      .execute_contract(
        user,
        self.addresses.astroport_luna_usdc_pair.clone(),
        &astroport::pair::ExecuteMsg::ProvideLiquidity {
          assets: vec![to_astro(uluna.clone()), to_astro(usdc.clone())],
          slippage_tolerance: None,
          auto_stake: None,
          receiver: None,
          min_lp_to_receive: None,
        },
        &[uluna.to_coin().unwrap(), usdc.to_coin().unwrap()],
      )
      .unwrap();
  }
  pub(super) fn create_lp_luna_ampluna(&mut self, user: Addr, big: bool) {
    let addr = self.addresses.clone();

    let uluna = addr.uluna(if big {
      36000_000000
    } else {
      36000
    });
    let ampluna = addr.ampluna(if big {
      30000_000000
    } else {
      30000
    });

    self
      .app
      .execute_contract(
        user.clone(),
        self.addresses.eris_hub_cw20_ampluna.clone(),
        &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
          spender: self.addresses.astroport_ampluna_luna_pair.to_string(),
          amount: ampluna.amount,
          expires: None,
        },
        &[],
      )
      .unwrap();

    self
      .app
      .execute_contract(
        user,
        self.addresses.astroport_ampluna_luna_pair.clone(),
        &astroport::pair::ExecuteMsg::ProvideLiquidity {
          assets: vec![to_astro(uluna.clone()), to_astro(ampluna.clone())],
          slippage_tolerance: None,
          auto_stake: None,
          receiver: None,
          min_lp_to_receive: None,
        },
        &[uluna.to_coin().unwrap()],
      )
      .unwrap();
  }
}

pub fn to_astro_info(asset: AssetInfo) -> astroport::asset::AssetInfo {
  match asset {
    AssetInfoBase::Native(native) => astroport::asset::AssetInfo::NativeToken {
      denom: native,
    },
    AssetInfoBase::Cw20(addr) => astroport::asset::AssetInfo::Token {
      contract_addr: addr,
    },
    _ => todo!(),
  }
}
pub fn to_astro(asset: Asset) -> astroport::asset::Asset {
  match asset.info {
    AssetInfoBase::Native(native) => astroport::asset::Asset::native(native, asset.amount),
    AssetInfoBase::Cw20(addr) => astroport::asset::Asset::cw20(addr, asset.amount),
    _ => todo!(),
  }
}
