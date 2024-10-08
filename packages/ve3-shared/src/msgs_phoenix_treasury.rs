use crate::{
  adapters::{global_config_adapter::ConfigExt, zapper::Zapper},
  constants::AT_ZAPPER,
  error::SharedError,
  helpers::assets::Assets,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Decimal, QuerierWrapper, Uint128};
use cw_address_like::AddressLike;
use cw_asset::{Asset, AssetError, AssetInfo, AssetInfoBase, AssetInfoUnchecked};
#[allow(unused_imports)]
use std::collections::HashSet;

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
  pub reward_denom: String,
  pub alliance_token_denom: String,
  pub global_config_addr: String,
  pub veto_owner: String,
  pub vetos: Vec<VetoRight<String>>,
  pub oracles: Vec<(AssetInfoBase<String>, Oracle<String>)>,
  pub allowed_actions: Option<Vec<String>>,
}

#[cw_serde]
pub struct VetoRight<T: AddressLike> {
  pub vetoer: T,
  pub spend_above_usd: Uint128,
  pub spend_above_usd_30d: Uint128,
  pub delay_s: u64,
}

impl VetoRight<String> {
  pub fn check(self, api: &dyn Api) -> Result<VetoRight<Addr>, SharedError> {
    Ok(VetoRight {
      vetoer: api.addr_validate(&self.vetoer)?,
      spend_above_usd: self.spend_above_usd,
      spend_above_usd_30d: self.spend_above_usd_30d,
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
pub enum Oracle<T: AddressLike> {
  Usdc,
  Pair {
    contract: T,
    simulation_amount: Uint128,
    #[serde(default)]
    from_decimals: Option<u32>,
  },
  Route {
    contract: T,
    path: Vec<AssetInfoBase<T>>,
    simulation_amount: Uint128,
    #[serde(default)]
    from_decimals: Option<u32>,
  },
}

impl Oracle<String> {
  pub fn check(self, api: &dyn Api) -> Result<Oracle<Addr>, SharedError> {
    Ok(match self {
      Oracle::Usdc => Oracle::Usdc,
      Oracle::Pair {
        contract,
        simulation_amount,
        from_decimals,
      } => Oracle::Pair {
        contract: api.addr_validate(&contract)?,
        simulation_amount,
        from_decimals,
      },
      Oracle::Route {
        contract,
        path,
        simulation_amount,
        from_decimals,
      } => Oracle::Route {
        contract: api.addr_validate(&contract)?,
        simulation_amount,
        path: path
          .into_iter()
          .map(|a| a.check(api, None))
          .collect::<Result<Vec<AssetInfo>, AssetError>>()?,
        from_decimals,
      },
    })
  }
}

#[cw_serde]
pub enum TreasuryActionSetup {
  Payment {
    payments: Vec<PaymentDescription>,
  },
  Otc {
    amount: Asset,
    into: Asset,
  },
  Dca {
    amount: Asset,
    into: AssetInfo,
    max_per_swap: Option<Uint128>,
    start_s: u64,
    end_s: u64,
    cooldown_s: u64,
  },
  Milestone {
    recipient: String,
    asset_info: AssetInfo,
    milestones: Vec<Milestone>,
  },
  Vesting {
    recipient: String,
    amount: Asset,
    start_s: u64,
    end_s: u64,
  },
}

impl TreasuryActionSetup {
  pub fn to_action_str(&self) -> String {
    match self {
      TreasuryActionSetup::Payment {
        ..
      } => "payment".to_string(),
      TreasuryActionSetup::Otc {
        ..
      } => "otc".to_string(),
      TreasuryActionSetup::Dca {
        ..
      } => "dca".to_string(),
      TreasuryActionSetup::Milestone {
        ..
      } => "milestone".to_string(),
      TreasuryActionSetup::Vesting {
        ..
      } => "vesting".to_string(),
    }
  }
}

#[cw_serde]
pub struct TreasuryAction {
  pub id: u64,
  pub name: String,
  pub reserved: Assets,
  pub cancelled: bool,
  pub done: bool,
  pub setup: TreasuryActionSetup,
  pub active_from: u64,
  pub total_usd: Uint128,
  pub total_usd_30d: Uint128,
  pub runtime: TreasuryActionRuntime,
}

#[cw_serde]
pub struct PaymentDescription {
  pub recipient: String,
  pub asset: Asset,
  pub claimable_after_s: Option<u64>,
}

impl From<(String, Asset)> for PaymentDescription {
  fn from(val: (String, Asset)) -> Self {
    PaymentDescription {
      asset: val.1,
      recipient: val.0,
      claimable_after_s: None,
    }
  }
}
impl From<(String, Asset, u64)> for PaymentDescription {
  fn from(val: (String, Asset, u64)) -> Self {
    PaymentDescription {
      asset: val.1,
      recipient: val.0,
      claimable_after_s: Some(val.2),
    }
  }
}

#[cw_serde]
pub enum TreasuryActionRuntime {
  Payment {
    open: Vec<PaymentDescription>,
  },
  Otc {},
  Dca {
    last_execution_s: u64,
  },
  Milestone {
    milestones: Vec<MilestoneRuntime>,
  },
  Vesting {
    last_claim_s: u64,
  },
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
  pub veto_owner: Addr,
  pub vetos: Vec<VetoRight<Addr>>,
  #[serde(default)]
  pub allowed_actions: Option<Vec<String>>,
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
  pub clawback: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
  // only allowed by veto owner to update what can be vetoed in what way
  UpdateVetoConfig {
    vetos: Vec<VetoRight<String>>,
  },

  // controls what on-chain oracles are being used and what assets can be used
  // asset can only be used if there is an oracle
  UpdateConfig {
    add_oracle: Option<Vec<(AssetInfoBase<String>, Oracle<String>)>>,
    remove_oracle: Option<Vec<AssetInfoBase<String>>>,
  },

  // This method permanently stops any payment operation on the contract
  // It will shutdown any open claims and vesting and stops everything. (Can break queries)
  // It will return the specified assets to the specified recipient.
  // It can only be called by the veto owner as a last resort.
  // It can be called multiple times if assets have been forgotten to clawback.
  Clawback {
    recipient: String,
    assets: Vec<AssetInfoUnchecked>,
  },

  // userd by the controller to setup payment actions
  Setup {
    name: String,
    action: TreasuryActionSetup,
  },
  // cancel any unpaid action
  Cancel {
    id: u64,
  },
  // vetoer if they have the right to veto, are allowed to cancel any action
  Veto {
    id: u64,
  },
  // claiming actions is for payment recipients to claim their received amount.
  Claim {
    id: u64,
  },
  // is used to update milestones and enable them for payout.
  UpdateMilestone {
    id: u64,
    index: u64,
    enabled: bool,
  },

  // bot interface for executing DCAs through zapping, using cooldown and max trade size
  ExecuteDca {
    id: u64,
    min_received: Option<Uint128>,
  },

  // interface to accept OTC offers from the PDT
  ExecuteOtc {
    id: u64,
    offer_amount: Uint128,
  },

  // Privileged functions
  // claims staking rewards from alliance
  ClaimRewards {},

  // Manage staked virtual token
  AllianceDelegate(AllianceDelegateMsg),
  AllianceUndelegate(AllianceUndelegateMsg),
  AllianceRedelegate(AllianceRedelegateMsg),

  // Remove validators after undelegation to gas optimize claiming rewards.
  RemoveValidator {
    validator: String,
  },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  /// Returns config
  #[returns(Config)]
  Config {},

  /// Returns config, including reserved amounts and if the clawback is active
  #[returns(State)]
  State {},

  /// Returns validators that the contract stakes to
  #[returns(HashSet<Addr>)]
  Validators {},

  /// Returns a specific action by id
  #[returns(TreasuryAction)]
  Action {
    id: u64,
  },

  /// Returns a list of actions (define id sort direction)
  #[returns(Vec<TreasuryAction>)]
  Actions {
    start_after: Option<u64>,
    limit: Option<u32>,
    direction: Option<Direction>,
  },

  // Returns actions that have a claim associated to the user.
  #[returns(Vec<TreasuryAction>)]
  UserActions {
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
  },

  // query balances of the contract (subtracting reserved amounts)
  #[returns(BalancesResponse)]
  Balances {
    assets: Option<Vec<AssetInfoUnchecked>>,
  },

  #[returns(OraclesResponse)]
  OraclePrices {
    assets: Option<Vec<AssetInfoUnchecked>>,
  },
}

#[cw_serde]
pub enum Direction {
  Asc,
  Desc,
}

#[cw_serde]
pub struct BalancesResponse {
  pub reserved: Assets,
  pub available: Assets,
}

// #[cw_serde]
pub type OraclesResponse = Vec<(AssetInfo, Decimal)>;
