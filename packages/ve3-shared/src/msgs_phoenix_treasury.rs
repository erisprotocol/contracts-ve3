use crate::{
  adapters::{global_config_adapter::ConfigExt, zapper::Zapper},
  constants::AT_ZAPPER,
  error::SharedError,
  helpers::assets::Assets,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, QuerierWrapper, Uint128};
use cw_address_like::AddressLike;
use cw_asset::{Asset, AssetError, AssetInfo, AssetInfoBase};
#[allow(unused_imports)]
use std::collections::HashSet;

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
  pub reward_denom: String,
  pub alliance_token_denom: String,
  pub global_config_addr: String,
  pub vetos: Vec<VetoRight<String>>,
  pub oracles: Vec<(AssetInfoBase<String>, Oracle<String>)>,
}

#[cw_serde]
pub struct VetoRight<T: AddressLike> {
  pub vetoer: T,
  pub min_amount_usd: Uint128,
  pub delay_s: u64,
}

impl VetoRight<String> {
  pub fn check(self, api: &dyn Api) -> Result<VetoRight<Addr>, SharedError> {
    Ok(VetoRight {
      vetoer: api.addr_validate(&self.vetoer)?,
      min_amount_usd: self.min_amount_usd,
      delay_s: self.delay_s,
    })
  }
}

pub trait Validate<T> {
  fn check(self, api: &dyn Api) -> Result<T, SharedError>;
}

impl Validate<Vec<VetoRight<Addr>>> for Vec<VetoRight<String>> {
  fn check(self, api: &dyn Api) -> Result<Vec<VetoRight<Addr>>, SharedError> {
    self.into_iter().map(|a| a.check(api)).collect::<Result<Vec<_>, SharedError>>()
  }
}

#[cw_serde]
pub enum ExecuteMsg {
  UpdateVetoConfig {
    vetos: Vec<VetoRight<String>>,
  },

  UpdateConfig {
    add_oracle: Option<Vec<(AssetInfoBase<String>, Oracle<String>)>>,
    remove_oracle: Option<Vec<AssetInfoBase<String>>>,
  },

  Setup {
    name: String,
    action: TreasuryActionSetup,
  },
  Cancel {
    id: u64,
  },
  Veto {
    id: u64,
  },
  Claim {
    id: u64,
  },
  Execute {
    id: u64,
    min_received: Option<Uint128>,
  },

  // Privileged functions
  ClaimRewards {},

  AllianceDelegate(AllianceDelegateMsg),
  AllianceUndelegate(AllianceUndelegateMsg),
  AllianceRedelegate(AllianceRedelegateMsg),

  RemoveValidator {
    validator: String,
  },
}

#[cw_serde]
pub enum Oracle<T: AddressLike> {
  Usdc,
  Pair {
    contract: T,
    simulation_amount: Uint128,
  },
  Route {
    contract: T,
    path: Vec<AssetInfoBase<T>>,
    simulation_amount: Uint128,
  },
}

impl Oracle<String> {
  pub fn check(self, api: &dyn Api) -> Result<Oracle<Addr>, SharedError> {
    Ok(match self {
      Oracle::Usdc => Oracle::Usdc,
      Oracle::Pair {
        contract,
        simulation_amount,
      } => Oracle::Pair {
        contract: api.addr_validate(&contract)?,
        simulation_amount,
      },
      Oracle::Route {
        contract,
        path,
        simulation_amount,
      } => Oracle::Route {
        contract: api.addr_validate(&contract)?,
        simulation_amount,
        path: path
          .into_iter()
          .map(|a| a.check(api, None))
          .collect::<Result<Vec<AssetInfo>, AssetError>>()?,
      },
    })
  }
}

#[cw_serde]
pub enum TreasuryActionSetup {
  Payment {
    payments: Vec<(String, Asset)>,
  },
  Dca {
    amount: Asset,
    into: AssetInfo,
    max_per_swap: Option<Uint128>,
    start_unix_s: u64,
    end_unix_s: u64,
  },
  Milestone {
    recipient: String,
    asset: AssetInfo,
    milestones: Vec<Milestone>,
  },
  Vesting {
    recipient: String,
    amount: Asset,
    start_unix_s: u64,
    end_unix_s: u64,
  },
}

#[cw_serde]
pub struct TreasuryAction {
  pub id: u64,
  pub name: String,
  pub reserved: Assets,
  pub cancelled: bool,
  pub done: bool,
  pub setup: TreasuryActionSetup,
  pub claim_active_from: u64,
  pub value_usd: Uint128,
  pub runtime: TreasuryActionRuntime,
}

#[cw_serde]
pub enum TreasuryActionRuntime {
  Payment {
    open: Vec<(String, Asset)>,
  },
  Milestone {
    milestones: Vec<MilestoneRuntime>,
  },
  Vesting {
    last_claim_unix_s: u64,
  },
  Dca {
    last_execution_unix_s: u64,
  },
  Empty {},
}

#[cw_serde]
pub struct Milestone {
  pub text: String,
  pub amount: Uint128,
}

#[cw_serde]
pub struct MilestoneRuntime {
  pub amount: Uint128,
  pub enabled: bool,
  pub claimed: bool,
}

#[cw_serde]
pub struct AllianceDelegateMsg {
  pub delegations: Vec<AllianceDelegation>,
}

#[cw_serde]
pub struct AllianceUndelegateMsg {
  pub undelegations: Vec<AllianceDelegation>,
}

#[cw_serde]
pub struct AllianceDelegation {
  pub validator: String,
  pub amount: Uint128,
}

#[cw_serde]
pub struct AllianceRedelegation {
  pub src_validator: String,
  pub dst_validator: String,
  pub amount: Uint128,
}

#[cw_serde]
pub struct AllianceRedelegateMsg {
  pub redelegations: Vec<AllianceRedelegation>,
}

#[cw_serde]
pub struct Config {
  pub global_config_addr: Addr,
  pub reward_denom: String,
  pub alliance_token_denom: String,
  pub vetos: Vec<VetoRight<Addr>>,
}

impl Config {
  pub fn zapper(&self, querier: &QuerierWrapper) -> Result<Zapper, SharedError> {
    Ok(Zapper(self.get_address(querier, AT_ZAPPER)?))
  }
}

#[cw_serde]
pub struct State {
  pub max_id: u64,
  pub reserved: Assets,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(Config)]
  Config {},

  #[returns(State)]
  State {},

  #[returns(HashSet<Addr>)]
  Validators {},

  #[returns(Vec<TreasuryAction>)]
  Actions {
    start_after: Option<u64>,
    limit: Option<u32>,
  },
  #[returns(Vec<TreasuryAction>)]
  UserActions {
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
  },
}
