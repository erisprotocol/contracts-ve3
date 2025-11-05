use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoUnchecked, AssetUnchecked};

use crate::adapters::{asset_staking::AssetStaking, connector::Connector, zapper::Zapper};

#[cw_serde]
pub struct InstantiateMsg {
  pub global_config_addr: String,

  pub fee: Decimal,
  pub fee_collector: String,
  pub deposit_profit_delay_s: u64,
  pub denom_creation_fee: AssetUnchecked,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,
  pub fee: Decimal,
  pub fee_collector: Addr,
  pub deposit_profit_delay_s: u64,
  pub denom_creation_fee: Asset,
}

#[cw_serde]
pub struct CompoundingAssetConfig {
  pub asset_info: AssetInfo,
  pub gauge: String,
  pub staking: AssetStaking,

  pub amp_denom: String,
  pub total_bond_share: Uint128,
  pub zasset_denom: String,
  pub reward_asset_info: AssetInfo,
  pub fee: Option<Decimal>,
}

#[cw_serde]
pub enum Cw20HookMsg {
  Stake {
    recipient: Option<String>,
    gauge: String,
  },
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
  Receive(Cw20ReceiveMsg),
  // user
  Stake {
    recipient: Option<String>,
    gauge: String,
  },

  Unstake {
    recipient: Option<String>,
  },

  Compound {
    minimum_receive: Option<Uint128>,
    asset_info: AssetInfoUnchecked,
    gauge: String,
  },

  ClaimTransfer {
    asset_info: AssetInfoUnchecked,
    gauge: String,
    receiver: Option<String>,
  },

  InitializeAsset {
    asset_info: AssetInfoUnchecked,
    gauge: String,
  },

  UpdateConfig {
    fee: Option<Decimal>,
    fee_collector: Option<String>,
    deposit_profit_delay_s: Option<u64>,
    denom_creation_fee: Option<AssetUnchecked>,

    fee_for_assets: Option<Vec<(String, AssetInfoUnchecked, Option<Decimal>)>>,
  },

  Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
  WithdrawZasset {
    connector: Connector,
    zasset_denom: String,
  },
  ZapRewards {
    config: Config,
    zapper: Zapper,
    asset_config: CompoundingAssetConfig,
    minimum_receive: Option<Uint128>,
  },
  Transfer {
    config: Config,
    asset_config: CompoundingAssetConfig,
    receiver: Addr,
  },
  TrackExchangeRate {
    asset_config: CompoundingAssetConfig,
    asset_info: AssetInfo,
    gauge: String,
  },
}

impl From<CallbackMsg> for ExecuteMsg {
  fn from(val: CallbackMsg) -> Self {
    ExecuteMsg::Callback(val)
  }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  #[returns(CompoundingAssetConfig)]
  AssetConfig {
    asset_info: AssetInfo,
    gauge: String,
  },

  #[returns(Vec<CompoundingAssetConfig>)]
  AssetConfigs {
    assets: Option<Vec<(String, AssetInfo)>>,
  },

  #[returns(Vec<UserInfoResponse>)]
  UserInfos {
    assets: Option<Vec<(String, AssetInfo)>>,
    addr: String,
  },

  #[returns(Vec<ExchangeRatesResponse>)]
  ExchangeRates {
    assets: Option<Vec<(String, AssetInfo)>>,

    start_after: Option<u64>,
    limit: Option<u32>,
  },

  #[returns(Vec<AmplpExchangeRatesResponse>)]
  AmplpExchangeRates {},
}

#[cw_serde]
pub struct AmplpExchangeRatesResponse {
  pub gauge: String,
  pub asset: AssetInfo,
  pub amplp_denom: String,
  pub exchange_rate: Decimal,
}

#[cw_serde]
pub struct UserInfoResponse {
  pub gauge: String,
  pub asset: AssetInfo,
  pub total_lp: Uint128,
  pub total_amplp: Uint128,
  pub user_lp: Uint128,
  pub user_amplp: Uint128,
}

#[cw_serde]
pub struct ExchangeRatesResponse {
  pub gauge: String,
  pub asset: AssetInfo,

  pub exchange_rates: Vec<(u64, ExchangeHistory)>,
  // APR normalized per DAY
  pub apr: Option<Decimal>,
}

#[cw_serde]
pub struct ExchangeHistory {
  pub exchange_rate: Decimal,
  pub time_s: u64,
}
