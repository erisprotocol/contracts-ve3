use crate::adapters::eris::ErisHub;
use crate::helpers::governance::get_period;
use crate::voting_escrow::QueryMsg::{LockInfo, LockVamp, TotalVamp};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Empty, Env, QuerierWrapper, StdResult, Uint128};
use cw20::Expiration;
#[allow(unused_imports)]
use cw20::{
  BalanceResponse, Cw20ReceiveMsg, DownloadLogoResponse, Logo, MarketingInfoResponse,
  TokenInfoResponse,
};
use cw721::{
  AllNftInfoResponse, ApprovalResponse, ApprovalsResponse, ContractInfoResponse, NftInfoResponse,
  NumTokensResponse, OperatorsResponse, OwnerOfResponse, TokensResponse,
};
use cw721_base::ExecuteMsg as CW721ExecuteMsg;
use cw721_base::MinterResponse;
use cw721_base::QueryMsg as CW721QueryMsg;
use cw_asset::{Asset, AssetInfo};
use std::{collections::HashMap, fmt};

/// This structure stores marketing information for voting escrow.
#[cw_serde]
pub struct UpdateMarketingInfo {
  /// Project URL
  pub project: Option<String>,
  /// Token description
  pub description: Option<String>,
  /// Token marketing information
  pub marketing: Option<String>,
  /// Token logo
  pub logo: Option<Logo>,
}

/// This structure stores general parameters for the voting escrow contract.
#[cw_serde]
pub struct InstantiateMsg {
  // global address config
  pub global_config_addr: String,
  // assets that are allowed to be locked including a config of how to calculate base power
  pub deposit_assets: HashMap<String, AssetInfoConfig>,
}

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
  /// USER
  /// Create a vAMP position and lock ampLP for `time` amount of time
  CreateLock {
    time: u64,
  },
  MergeLock {
    token_id: Uint128,
    token_id_add: Uint128,
  },
  SplitLock {
    token_id: Uint128,
    amount: Uint128,
    recipient: Option<String>,
  },
  /// Extend the lockup time for your staked ampLP. For an expired lock, it will always start from the current period.
  ExtendLockTime {
    time: u64,
    token_id: Uint128,
  },
  /// Add more ampLP to your vAMP position
  ExtendLockAmount {
    token_id: Uint128,
  },
  /// Withdraw ampLP from the voting escrow contract
  Withdraw {
    token_id: Uint128,
  },
  /// Implements the Cw20 receiver interface
  Receive(Cw20ReceiveMsg),

  /// Add or remove accounts from the blacklist
  UpdateBlacklist {
    append_addrs: Option<Vec<String>>,
    remove_addrs: Option<Vec<String>>,
  },
  /// Update config
  UpdateConfig {
    // assets that are allowed to be locked including a config of how to calculate base power
    // for now removal is not supported
    append_deposit_assets: Option<HashMap<String, AssetInfoConfig>>,

    push_update_contracts: Option<Vec<String>>,
    // allows withdrawals of tokens.
    decommissioned: Option<bool>,
  },

  /// CW721 standard message
  /// Transfer is a base message to move a token to another account without triggering actions
  TransferNft {
    recipient: String,
    token_id: String,
  },
  /// Send is a base message to transfer a token to a contract and trigger an action
  /// on the receiving contract.
  SendNft {
    contract: String,
    token_id: String,
    msg: Binary,
  },
  /// Burn an NFT the sender has access to
  Burn {
    token_id: String,
  },

  /// Allows operator to transfer / send the token from the owner's account.
  /// If expiration is set, then this allowance has a time/height limit
  Approve {
    spender: String,
    token_id: String,
    expires: Option<Expiration>,
  },
  /// Remove previously granted Approval
  Revoke {
    spender: String,
    token_id: String,
  },
  /// Allows operator to transfer / send any token from the owner's account.
  /// If expiration is set, then this allowance has a time/height limit
  ApproveAll {
    operator: String,
    expires: Option<Expiration>,
  },
  /// Remove previously granted ApproveAll permission
  RevokeAll {
    operator: String,
  },
}

pub type VeNftCollection<'a> = cw721_base::Cw721Contract<'a, Extension, Empty, Empty, Empty>;

#[cw_serde]
pub struct Trait {
  pub display_type: Option<String>,
  pub trait_type: String,
  pub value: String,
}

pub type Extension = Metadata;

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
pub struct Metadata {
  pub image: Option<String>,
  // pub image_data: Option<String>,
  // pub external_url: Option<String>,
  pub description: Option<String>,
  pub name: Option<String>,
  pub attributes: Option<Vec<Trait>>,
  // pub background_color: Option<String>,
  // pub animation_url: Option<String>,
  // pub youtube_url: Option<String>,
}

impl From<ExecuteMsg> for CW721ExecuteMsg<Metadata, Empty> {
  fn from(msg: ExecuteMsg) -> CW721ExecuteMsg<Metadata, Empty> {
    match msg {
      ExecuteMsg::Approve {
        spender,
        token_id,
        expires,
      } => CW721ExecuteMsg::Approve {
        spender,
        token_id,
        expires,
      },
      ExecuteMsg::Revoke {
        spender,
        token_id,
      } => CW721ExecuteMsg::Revoke {
        spender,
        token_id,
      },
      ExecuteMsg::ApproveAll {
        operator,
        expires,
      } => CW721ExecuteMsg::ApproveAll {
        operator,
        expires,
      },
      ExecuteMsg::RevokeAll {
        operator,
      } => CW721ExecuteMsg::RevokeAll {
        operator,
      },
      _ => panic!("cannot covert {:?} to CW721ExecuteMsg", msg),
    }
  }
}

#[cw_serde]
pub enum ReceiveMsg {
  ExtendLockAmount {
    token_id: Uint128,
  },
  CreateLock {
    time: u64,
  },
}

#[cw_serde]
pub enum PushExecuteMsg {
  UpdateVote {
    token_id: String,
    lock_info: LockInfoResponse,
    old_owner: Option<Addr>,
  },
}

/// This enum describes voters status.
#[cw_serde]
pub enum BlacklistedVotersResponse {
  /// Voters are blacklisted
  VotersBlacklisted {},
  /// Returns a voter that is not blacklisted.
  VotersNotBlacklisted {
    voter: String,
  },
}

impl fmt::Display for BlacklistedVotersResponse {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      BlacklistedVotersResponse::VotersBlacklisted {} => write!(f, "Voters are blacklisted!"),
      BlacklistedVotersResponse::VotersNotBlacklisted {
        voter,
      } => {
        write!(f, "Voter is not blacklisted: {}", voter)
      },
    }
  }
}

#[cw_serde]
#[derive(Default)]
pub enum Time {
  #[default]
  Current,
  Time(u64),
  Period(u64),
}

pub trait GetPeriod {
  fn get_period(self, env: &Env) -> StdResult<u64>;
}
impl GetPeriod for Option<Time> {
  fn get_period(self, env: &Env) -> StdResult<u64> {
    match self {
      Some(time) => match time {
        Time::Current => get_period(env.block.time.seconds()),
        Time::Time(time) => get_period(time),
        Time::Period(period) => Ok(period),
      },
      None => get_period(env.block.time.seconds()),
    }
  }
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  /// Return the blacklisted voters
  #[returns(Vec<Addr>)]
  BlacklistedVoters {
    start_after: Option<String>,
    limit: Option<u32>,
  },

  /// Return the user's vAMP balance
  #[returns(BalanceResponse)]
  Balance {
    address: String,
  },

  /// Return the current total amount of vAMP
  #[returns(VotingPowerResponse)]
  TotalVamp {
    time: Option<Time>,
  },

  /// Return the user's current voting power (vAMP balance)
  #[returns(VotingPowerResponse)]
  LockVamp {
    token_id: String,
    time: Option<Time>,
  },

  /// Return information about a user's lock position
  #[returns(LockInfoResponse)]
  LockInfo {
    token_id: String,
    time: Option<Time>,
  },
  /// Return the vAMP contract configuration
  #[returns(Config)]
  Config {},

  /// With MetaData Extension.
  /// Returns metadata about one particular token,
  /// based on *ERC721 Metadata JSON Schema*
  /// https://docs.opensea.io/docs/metadata-standards
  ///
  /// {    
  ///    "name": "AllianceNFT # 1",
  ///    "token_uri": null,
  ///    "extension": {
  ///      "image": "https://ipfs.io/ipfs/{hash}",
  ///      "description": "Received for participating on Game Of Alliance",
  ///      "name": "AllianceNFT # 1",
  ///      "attributes": [{
  ///              "display_type" : null,
  ///              "trait_type": "x",
  ///              "value": "1"
  ///          },{
  ///              "display_type" : null,
  ///              "trait_type": "y",
  ///              "value": "1"
  ///          },{
  ///              "display_type" : null,
  ///              "trait_type": "width",
  ///              "value": "120"
  ///          },{
  ///              "display_type" : null,
  ///              "trait_type": "height",
  ///              "value": "120"
  ///          },{
  ///              "display_type" : null,
  ///              "trait_type": "rarity",
  ///              "value": 11
  ///          }],
  ///      "image_data": null,
  ///      "external_url": null,
  ///      "background_color": null,
  ///      "animation_url": null,
  ///      "youtube_url": null
  ///    }
  ///  }
  #[returns(NftInfoResponse<Extension>)]
  NftInfo {
    token_id: String,
  },

  /// With MetaData Extension.
  /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
  #[returns(AllNftInfoResponse<Extension>)]
  AllNftInfo {
    token_id: String,
    /// unset or false will filter out expired approvals, you must set to true to see them
    include_expired: Option<bool>,
  },

  /// CW721 Queries

  /// Return the owner of the given token, error if token does not exist
  #[returns(OwnerOfResponse)]
  OwnerOf {
    token_id: String,
    /// unset or false will filter out expired approvals, you must set to true to see them
    include_expired: Option<bool>,
  },
  /// Return operator that can access all of the owner's tokens.
  /// Return the owner of the given token, error if token does not exist
  #[returns(ApprovalResponse)]
  Approval {
    token_id: String,
    spender: String,
    include_expired: Option<bool>,
  },
  /// Return approvals that a token has
  #[returns(ApprovalsResponse)]
  Approvals {
    token_id: String,
    include_expired: Option<bool>,
  },
  /// List all operators that can access all of the owner's tokens
  #[returns(OperatorsResponse)]
  AllOperators {
    owner: String,
    /// unset or false will filter out expired items, you must set to true to see them
    include_expired: Option<bool>,
    start_after: Option<String>,
    limit: Option<u32>,
  },
  /// Total number of tokens issued
  #[returns(NumTokensResponse)]
  NumTokens {},

  /// With MetaData Extension.
  #[returns(ContractInfoResponse)]
  ContractInfo {},

  /// With Enumerable extension.
  /// Returns all tokens owned by the given address, [] if unset.
  #[returns(TokensResponse)]
  Tokens {
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
  },
  /// With Enumerable extension.
  /// Requires pagination. Lists all token_ids controlled by the contract.
  #[returns(TokensResponse)]
  AllTokens {
    start_after: Option<String>,
    limit: Option<u32>,
  },

  // Return the minter
  #[returns(MinterResponse)]
  Minter {},
}

impl From<QueryMsg> for CW721QueryMsg<Empty> {
  fn from(msg: QueryMsg) -> CW721QueryMsg<Empty> {
    match msg {
      QueryMsg::OwnerOf {
        token_id,
        include_expired,
      } => CW721QueryMsg::OwnerOf {
        token_id,
        include_expired,
      },
      QueryMsg::Approval {
        token_id,
        spender,
        include_expired,
      } => CW721QueryMsg::Approval {
        token_id,
        spender,
        include_expired,
      },
      QueryMsg::Approvals {
        token_id,
        include_expired,
      } => CW721QueryMsg::Approvals {
        token_id,
        include_expired,
      },
      QueryMsg::AllOperators {
        owner,
        include_expired,
        start_after,
        limit,
      } => CW721QueryMsg::AllOperators {
        owner,
        include_expired,
        start_after,
        limit,
      },
      QueryMsg::NumTokens {} => CW721QueryMsg::NumTokens {},
      QueryMsg::ContractInfo {} => CW721QueryMsg::ContractInfo {},
      QueryMsg::NftInfo {
        token_id,
      } => CW721QueryMsg::NftInfo {
        token_id,
      },
      QueryMsg::AllNftInfo {
        token_id,
        include_expired,
      } => CW721QueryMsg::AllNftInfo {
        token_id,
        include_expired,
      },
      QueryMsg::Tokens {
        owner,
        start_after,
        limit,
      } => CW721QueryMsg::Tokens {
        owner,
        start_after,
        limit,
      },
      QueryMsg::AllTokens {
        start_after,
        limit,
      } => CW721QueryMsg::AllTokens {
        start_after,
        limit,
      },
      QueryMsg::Minter {} => CW721QueryMsg::Minter {},
      _ => panic!("cannot covert {:?} to CW721QueryMsg", msg),
    }
  }
}

/// This structure is used to return a user's amount of vAMP.
#[cw_serde]
pub struct VotingPowerResponse {
  /// The vAMP balance
  pub vamp: Uint128,
}

/// This structure is used to return the lock information for a vAMP position.
#[cw_serde]
pub struct LockInfoResponse {
  pub owner: Addr,

  pub period: u64,

  pub asset: Asset,
  /// The underlying_amount locked in the position
  pub underlying_amount: Uint128,
  /// This is the initial boost for the lock position
  pub coefficient: Decimal,
  /// Start time for the vAMP position decay
  pub start: u64,
  /// End time for the vAMP position decay
  pub end: u64,
  /// Slope at which a staker's vAMP balance decreases over time
  pub slope: Uint128,

  /// fixed sockel
  pub fixed_amount: Uint128,
  /// includes only decreasing voting_power, it is the current voting power of the period currently queried.
  pub voting_power: Uint128,
}

impl LockInfoResponse {
  pub fn has_vp(&self) -> bool {
    !self.fixed_amount.is_zero() || !self.voting_power.is_zero()
  }
}

/// This structure stores the main parameters for the voting escrow contract.
#[cw_serde]
pub struct Config {
  // global address config
  pub global_config_addr: Addr,
  // assets that are allowed to be locked including a config of how to calculate base power
  pub allowed_deposit_assets: HashMap<AssetInfo, AssetInfoConfig>,
  /// The list of contracts to receive updates on user's lock info changes
  pub push_update_contracts: Vec<Addr>,
  /// Address that can only blacklist vAMP stakers and remove their governance power
  pub decommissioned: Option<bool>,
}

#[cw_serde]
pub enum AssetInfoConfig {
  Default,
  ExchangeRate {
    contract: Addr,
  },
}

impl AssetInfoConfig {
  pub fn get_exchange_rate(&self, querier: &QuerierWrapper) -> StdResult<Option<Decimal>> {
    match self {
      AssetInfoConfig::Default => Ok(None),
      AssetInfoConfig::ExchangeRate {
        contract,
      } => Ok(Some(ErisHub(contract).query_exchange_rate(querier)?)),
    }
  }

  pub fn get_underlying_amount(
    &self,
    querier: &QuerierWrapper,
    amount: Uint128,
  ) -> StdResult<Uint128> {
    Ok(self.get_exchange_rate(querier)?.map_or(amount, |e| e * amount))
  }
}

/// This structure describes a Migration message.
#[cw_serde]
pub struct MigrateMsg {}

/// Queries current user's voting power from the voting escrow contract.
///
/// * **user** staker for which we calculate the latest vAMP voting power.
pub fn get_voting_power(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
  user: impl Into<String>,
) -> StdResult<Uint128> {
  let vp: VotingPowerResponse = querier.query_wasm_smart(
    escrow_addr,
    &LockVamp {
      token_id: user.into(),
      time: None,
    },
  )?;
  Ok(vp.vamp)
}

/// Queries current user's voting power from the voting escrow contract by timestamp.
///
/// * **user** staker for which we calculate the voting power at a specific time.
///
/// * **timestamp** timestamp at which we calculate the staker's voting power.
pub fn get_voting_power_at(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
  token_id: impl Into<String>,
  timestamp: u64,
) -> StdResult<Uint128> {
  let vp: VotingPowerResponse = querier.query_wasm_smart(
    escrow_addr,
    &LockVamp {
      token_id: token_id.into(),
      time: Some(Time::Time(timestamp)),
    },
  )?;

  Ok(vp.vamp)
}

/// Queries current total voting power from the voting escrow contract.
pub fn get_total_voting_power(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
) -> StdResult<Uint128> {
  let vp: VotingPowerResponse = querier.query_wasm_smart(
    escrow_addr,
    &TotalVamp {
      time: None,
    },
  )?;

  Ok(vp.vamp)
}

/// Queries total voting power from the voting escrow contract by timestamp.
///
/// * **timestamp** time at which we fetch the total voting power.
pub fn get_total_voting_power_at(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
  timestamp: u64,
) -> StdResult<Uint128> {
  let vp: VotingPowerResponse = querier.query_wasm_smart(
    escrow_addr,
    &TotalVamp {
      time: Some(Time::Time(timestamp)),
    },
  )?;

  Ok(vp.vamp)
}

/// Queries total voting power from the voting escrow contract by period.
///
/// * **timestamp** time at which we fetch the total voting power.
pub fn get_total_voting_power_at_by_period(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
  period: u64,
) -> StdResult<Uint128> {
  let vp: VotingPowerResponse = querier.query_wasm_smart(
    escrow_addr,
    &QueryMsg::TotalVamp {
      time: Some(Time::Period(period)),
    },
  )?;

  Ok(vp.vamp)
}

/// Queries user's lockup information from the voting escrow contract.
///
/// * **user** staker for which we return lock position information.
pub fn get_lock_info(
  querier: &QuerierWrapper,
  escrow_addr: impl Into<String>,
  token_id: impl Into<String>,
) -> StdResult<LockInfoResponse> {
  let lock_info: LockInfoResponse = querier.query_wasm_smart(
    escrow_addr,
    &LockInfo {
      token_id: token_id.into(),
      time: None,
    },
  )?;
  Ok(lock_info)
}
