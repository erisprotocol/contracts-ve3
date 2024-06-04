use crate::voting_escrow::QueryMsg::{LockInfo, TotalVamp, TotalVampAt, UserVamp, UserVampAt};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Empty, QuerierWrapper, StdResult, Uint128};
use cw20::Expiration;
#[allow(unused_imports)]
use cw20::{
    BalanceResponse, Cw20ReceiveMsg, DownloadLogoResponse, Logo, MarketingInfoResponse,
    TokenInfoResponse,
};
use cw721_base::ExecuteMsg as CW721ExecuteMsg;
use cw_asset::AssetInfo;
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
    pub deposit_assets: HashMap<String, AssetInfoLockConfig>,
}

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// USER
    /// Create a vAMP position and lock ampLP for `time` amount of time
    CreateLock {
        time: u64,
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
        append_deposit_assets: Option<HashMap<String, AssetInfoLockConfig>>,
        remove_deposit_assets: Option<Vec<String>>,

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
    /// Burn an NFT the sender has access to
    Burn {
        token_id: String,
    },
}

pub type VeNftCollection<'a> = cw721_base::Cw721Contract<'a, Metadata, Empty, Empty, Empty>;

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

impl From<ExecuteMsg> for CW721ExecuteMsg<Metadata, Empty> {
    fn from(msg: ExecuteMsg) -> CW721ExecuteMsg<Metadata, Empty> {
        match msg {
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => CW721ExecuteMsg::TransferNft {
                recipient,
                token_id,
            },
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => CW721ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            },
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
        user: String,
        lock_info: LockInfoResponse,
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

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Checks if specified addresses are blacklisted
    #[returns(BlacklistedVotersResponse)]
    CheckVotersAreBlacklisted {
        voters: Vec<String>,
    },
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
    /// Fetch the vAMP token information
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    /// Fetch vAMP's marketing information
    #[returns(MarketingInfoResponse)]
    MarketingInfo {},
    /// Download the vAMP logo
    #[returns(DownloadLogoResponse)]
    DownloadLogo {},
    /// Return the current total amount of vAMP
    #[returns(VotingPowerResponse)]
    TotalVamp {},
    /// Return the total amount of vAMP at some point in the past
    #[returns(VotingPowerResponse)]
    TotalVampAt {
        time: u64,
    },
    /// Return the total voting power at a specific period
    #[returns(VotingPowerResponse)]
    TotalVampAtPeriod {
        period: u64,
    },
    /// Return the user's current voting power (vAMP balance)
    #[returns(VotingPowerResponse)]
    UserVamp {
        user: String,
    },
    /// Return the user's vAMP balance at some point in the past
    #[returns(VotingPowerResponse)]
    UserVampAt {
        user: String,
        time: u64,
    },
    /// Return the user's voting power at a specific period
    #[returns(VotingPowerResponse)]
    UserVampAtPeriod {
        user: String,
        period: u64,
    },
    /// Return information about a user's lock position
    #[returns(LockInfoResponse)]
    LockInfo {
        user: String,
    },
    /// Return user's locked ampLP balance at the given block height
    #[returns(Uint128)]
    UserDepositAtHeight {
        user: String,
        height: u64,
    },
    /// Return the vAMP contract configuration
    #[returns(Config)]
    Config {},
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
    /// The amount of ampLP locked in the position
    pub amount: Uint128,
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

/// This structure stores the main parameters for the voting escrow contract.
#[cw_serde]
pub struct Config {
    // global address config
    pub global_config_addr: Addr,
    // assets that are allowed to be locked including a config of how to calculate base power
    pub allowed_deposit_assets: HashMap<AssetInfo, AssetInfoLockConfig>,
    /// The list of contracts to receive updates on user's lock info changes
    pub push_update_contracts: Vec<Addr>,
    /// Address that can only blacklist vAMP stakers and remove their governance power
    pub decommissioned: Option<bool>,
}

#[cw_serde]
pub enum AssetInfoLockConfig {
    Default,
    ExchangeRate {
        contract: Addr,
    },
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
        &UserVamp {
            user: user.into(),
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
    user: impl Into<String>,
    timestamp: u64,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(
        escrow_addr,
        &UserVampAt {
            user: user.into(),
            time: timestamp,
        },
    )?;

    Ok(vp.vamp)
}

/// Queries current total voting power from the voting escrow contract.
pub fn get_total_voting_power(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(escrow_addr, &TotalVamp {})?;

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
        &TotalVampAt {
            time: timestamp,
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
        &QueryMsg::TotalVampAtPeriod {
            period,
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
    user: impl Into<String>,
) -> StdResult<LockInfoResponse> {
    let lock_info: LockInfoResponse = querier.query_wasm_smart(
        escrow_addr,
        &LockInfo {
            user: user.into(),
        },
    )?;
    Ok(lock_info)
}
