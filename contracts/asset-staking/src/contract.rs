use crate::error::ContractError;
use crate::state::{
    ASSET_REWARD_DISTRIBUTION, ASSET_REWARD_RATE, BALANCES, CONFIG, TOTAL_BALANCES,
    UNCLAIMED_REWARDS, USER_ASSET_REWARD_RATE, WHITELIST,
};
use crate::token_factory::CustomExecuteMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, Addr, Decimal, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Storage, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use cw_asset::{Asset, AssetInfo, AssetInfoBase};
use semver::Version;
use std::collections::HashMap;
use ve3_global_config::global_config_adapter::GlobalConfig;
use ve3_shared::adapters::connector::Connector;
use ve3_shared::alliance_oracle_types::ChainId;
use ve3_shared::alliance_protocol::{
    AssetDistribution, CallbackMsg, Config, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg,
};
use ve3_shared::constants::{
    AT_ASSET_WHITELIST_CONTROLLER, AT_CONNECTOR, AT_REWARD_DISTRIBUTION_CONTROLLER,
};
use ve3_shared::error::SharedError;
use ve3_shared::extensions::asset_info_ext::AssetInfoExt;
use ve3_shared::extensions::env_ext::EnvExt;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    ensure!(storage_version < version, StdError::generic_err("Invalid contract version"));

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<CustomExecuteMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        reward_denom: msg.reward_denom,
        global_config_addr: deps.api.addr_validate(&msg.global_config_addr)?,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![("action", "instantiate")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // user
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Stake {
            recipient,
        } => {
            if info.funds.len() != 1 {
                return Err(ContractError::OnlySingleAssetAllowed {});
            }
            if info.funds[0].amount.is_zero() {
                return Err(ContractError::AmountCannotBeZero {});
            }
            let recipient = if let Some(recipient) = recipient {
                deps.api.addr_validate(&recipient)?
            } else {
                info.sender
            };
            let asset = AssetInfo::native(&info.funds[0].denom);
            stake(deps, env, info.clone(), asset, info.funds[0].amount, recipient)
        },
        ExecuteMsg::Unstake(asset) => unstake(deps, info, asset),
        ExecuteMsg::ClaimRewards(asset) => claim_rewards(deps, info, asset),

        // bot
        ExecuteMsg::UpdateRewards {} => update_rewards(deps, env, info),
        ExecuteMsg::DistributeTakeRate {
            assets,
        } => distribute_take_rate(deps, env, info, assets),

        // controller
        ExecuteMsg::WhitelistAssets(assets) => whitelist_assets(deps, info, assets),
        ExecuteMsg::RemoveAssets(assets) => remove_assets(deps, info, assets),
        ExecuteMsg::SetAssetRewardDistribution(asset_reward_distribution) => {
            set_asset_reward_distribution(deps, info, asset_reward_distribution)
        },

        // contract
        ExecuteMsg::Callback(msg) => callback(deps, env, info, msg),

        _ => Err(ContractError::Std(StdError::generic_err("unsupported action"))),
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    if env.contract.address != info.sender {
        Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
    }

    match msg {
        CallbackMsg::UpdateRewardsCallback {
            initial_balance,
        } => update_reward_callback(deps, env, info, initial_balance),
    }
}

// receive_cw20 routes a cw20 token to the proper handler in this case stake and unstake
fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;

    match from_json(&cw20_msg.msg)? {
        Cw20HookMsg::Stake {
            recipient,
        } => {
            if cw20_msg.amount.is_zero() {
                return Err(ContractError::AmountCannotBeZero {});
            }
            let asset = AssetInfo::Cw20(info.sender.clone());
            let recipient = if let Some(recipient) = recipient {
                deps.api.addr_validate(&recipient)?
            } else {
                sender
            };

            stake(deps, env, info, asset, cw20_msg.amount, recipient)
        },
    }
}

fn set_asset_reward_distribution(
    deps: DepsMut,
    info: MessageInfo,
    asset_reward_distribution: Vec<AssetDistribution>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    assert_distribution_controller(&deps, &info, &config)?;

    // Ensure the dsitributions add up to 100%
    let total_distribution = asset_reward_distribution
        .iter()
        .map(|a| a.distribution)
        .fold(Decimal::zero(), |acc, v| acc + v);

    if total_distribution != Decimal::percent(100) {
        return Err(ContractError::InvalidDistribution {});
    }

    // Simply set the asset_reward_distribution, overwriting any previous settings.
    // This means any updates should include the full existing set of AssetDistributions and not just the newly updated one.
    ASSET_REWARD_DISTRIBUTION.save(deps.storage, &asset_reward_distribution)?;
    Ok(Response::new().add_attributes(vec![("action", "set_asset_reward_distribution")]))
}

fn whitelist_assets(
    deps: DepsMut,
    info: MessageInfo,
    assets_request: HashMap<ChainId, Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    assert_whitelist_controller(&deps, &info, &config)?;
    let mut attrs = vec![("action".to_string(), "whitelist_assets".to_string())];
    for (chain_id, assets) in &assets_request {
        for asset in assets {
            WHITELIST.save(deps.storage, &asset, chain_id)?;
            ASSET_REWARD_RATE.update(deps.storage, asset, |rate| -> StdResult<_> {
                Ok(rate.unwrap_or(Decimal::zero()))
            })?;
        }
        attrs.push(("chain_id".to_string(), chain_id.to_string()));
        let assets_str =
            assets.iter().map(|asset| asset.to_string()).collect::<Vec<String>>().join(",");

        attrs.push(("assets".to_string(), assets_str.to_string()));
    }
    Ok(Response::new().add_attributes(attrs))
}

fn remove_assets(
    deps: DepsMut,
    info: MessageInfo,
    assets: Vec<AssetInfo>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Only allow the governance address to update whitelisted assets
    assert_whitelist_controller(&deps, &info, &config)?;
    for asset in &assets {
        WHITELIST.remove(deps.storage, &asset);
    }
    let assets_str =
        assets.iter().map(|asset| asset.to_string()).collect::<Vec<String>>().join(",");
    Ok(Response::new().add_attributes(vec![("action", "remove_assets"), ("assets", &assets_str)]))
}

fn stake(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    asset: AssetInfoBase<Addr>,
    amount: Uint128,
    recipient: Addr,
) -> Result<Response, ContractError> {
    WHITELIST.load(deps.storage, &asset).map_err(|_| ContractError::AssetNotWhitelisted {})?;

    let rewards = _claim_reward(deps.storage, recipient.clone(), asset.clone())?;
    if !rewards.is_zero() {
        UNCLAIMED_REWARDS.update(
            deps.storage,
            (recipient.clone(), &asset),
            |balance| -> Result<_, ContractError> {
                Ok(balance.unwrap_or(Uint128::zero()) + rewards)
            },
        )?;
    }

    BALANCES.update(
        deps.storage,
        (recipient.clone(), &asset),
        |balance| -> Result<_, ContractError> {
            match balance {
                Some(balance) => Ok(balance + amount),
                None => Ok(amount),
            }
        },
    )?;
    TOTAL_BALANCES.update(deps.storage, &asset, |balance| -> Result<_, ContractError> {
        Ok(balance.unwrap_or(Uint128::zero()) + amount)
    })?;

    let asset_reward_rate = ASSET_REWARD_RATE.load(deps.storage, &asset).unwrap_or(Decimal::zero());
    USER_ASSET_REWARD_RATE.save(deps.storage, (recipient.clone(), &asset), &asset_reward_rate)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "stake"),
        ("user", recipient.as_ref()),
        ("asset", &asset.to_string()),
        ("amount", &amount.to_string()),
    ]))
}

fn unstake(deps: DepsMut, info: MessageInfo, asset: Asset) -> Result<Response, ContractError> {
    let sender = info.sender.clone();
    if asset.amount.is_zero() {
        return Err(ContractError::AmountCannotBeZero {});
    }

    let rewards = _claim_reward(deps.storage, sender.clone(), asset.info.clone())?;
    if !rewards.is_zero() {
        UNCLAIMED_REWARDS.update(
            deps.storage,
            (sender.clone(), &asset.info),
            |balance| -> Result<_, ContractError> {
                Ok(balance.unwrap_or(Uint128::zero()) + rewards)
            },
        )?;
    }

    BALANCES.update(
        deps.storage,
        (sender, &asset.info),
        |balance| -> Result<_, ContractError> {
            match balance {
                Some(balance) => {
                    if balance < asset.amount {
                        return Err(ContractError::InsufficientBalance {});
                    }
                    Ok(balance - asset.amount)
                },
                None => Err(ContractError::InsufficientBalance {}),
            }
        },
    )?;
    TOTAL_BALANCES.update(deps.storage, &asset.info, |balance| -> Result<_, ContractError> {
        let balance = balance.unwrap_or(Uint128::zero());
        if balance < asset.amount {
            return Err(ContractError::InsufficientBalance {});
        }
        Ok(balance - asset.amount)
    })?;

    let msg = asset.transfer_msg(&info.sender)?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "unstake"),
            ("user", info.sender.as_ref()),
            ("asset", &asset.info.to_string()),
            ("amount", &asset.amount.to_string()),
        ])
        .add_message(msg))
}

fn claim_rewards(
    deps: DepsMut,
    info: MessageInfo,
    asset: AssetInfo,
) -> Result<Response, ContractError> {
    let user = info.sender;
    let config = CONFIG.load(deps.storage)?;
    let rewards = _claim_reward(deps.storage, user.clone(), asset.clone())?;
    let unclaimed_rewards =
        UNCLAIMED_REWARDS.load(deps.storage, (user.clone(), &asset)).unwrap_or(Uint128::zero());
    let final_rewards = rewards + unclaimed_rewards;
    UNCLAIMED_REWARDS.remove(deps.storage, (user.clone(), &asset));
    let response = Response::new().add_attributes(vec![
        ("action", "claim_rewards"),
        ("user", user.as_ref()),
        ("asset", &asset.to_string()),
        ("reward_amount", &final_rewards.to_string()),
    ]);
    if !final_rewards.is_zero() {
        let rewards_asset = Asset {
            info: AssetInfo::Native(config.reward_denom),
            amount: final_rewards,
        };
        Ok(response.add_message(rewards_asset.transfer_msg(&user)?))
    } else {
        Ok(response)
    }
}

fn _claim_reward(
    storage: &mut dyn Storage,
    user: Addr,
    asset: AssetInfo,
) -> Result<Uint128, ContractError> {
    let user_reward_rate = USER_ASSET_REWARD_RATE.load(storage, (user.clone(), &asset));
    let asset_reward_rate = ASSET_REWARD_RATE.load(storage, &asset)?;

    if let Ok(user_reward_rate) = user_reward_rate {
        let user_staked = BALANCES.load(storage, (user.clone(), &asset))?;
        let rewards = ((asset_reward_rate - user_reward_rate)
            * Decimal::from_atomics(user_staked, 0)?)
        .to_uint_floor();
        if rewards.is_zero() {
            Ok(Uint128::zero())
        } else {
            USER_ASSET_REWARD_RATE.save(storage, (user, &asset), &asset_reward_rate)?;
            Ok(rewards)
        }
    } else {
        // If cannot find user_reward_rate, assume this is the first time they are staking and set it to the current asset_reward_rate
        USER_ASSET_REWARD_RATE.save(storage, (user, &asset), &asset_reward_rate)?;

        Ok(Uint128::zero())
    }
}

fn distribute_take_rate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Option<Vec<AssetInfo>>,
) -> Result<Response, ContractError> {
    todo!()
}

fn update_rewards(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.funds.len() > 0 {
        Err(SharedError::NoFundsAllowed {})?;
    }

    let connector_addr =
        GlobalConfig(config.global_config_addr).get_address(&deps.querier, AT_CONNECTOR)?;

    let initial_balance: cw_asset::AssetBase<Addr> = AssetInfo::native(config.reward_denom)
        .with_balance_query(&deps.querier, &env.contract.address)?;

    let msgs = vec![
        Connector(connector_addr).claim_rewards_msg()?,
        env.callback_msg(ExecuteMsg::Callback(CallbackMsg::UpdateRewardsCallback {
            initial_balance,
        }))?,
    ];

    Ok(Response::new().add_attributes(vec![("action", "update_rewards")]).add_messages(msgs))
}

fn update_reward_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    initial_balance: Asset,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        Err(SharedError::UnauthorizedCallbackOnlyCallableByContract {})?
    }

    let previous_balance = initial_balance.amount;
    let current_balance =
        initial_balance.info.query_balance(&deps.querier, env.contract.address)?;

    let rewards_collected = current_balance - previous_balance;

    let asset_reward_distribution = ASSET_REWARD_DISTRIBUTION.load(deps.storage)?;
    let total_distribution = asset_reward_distribution
        .iter()
        .map(|a| a.distribution)
        .fold(Decimal::zero(), |acc, v| acc + v);

    for asset_distribution in asset_reward_distribution {
        let total_reward_distributed = Decimal::from_atomics(rewards_collected, 0)?
            * asset_distribution.distribution
            / total_distribution;

        // If there are no balances, we stop updating the rate. This means that the emissions are not directed to any stakers.
        let total_balance =
            TOTAL_BALANCES.load(deps.storage, &asset_distribution.asset).unwrap_or(Uint128::zero());
        if !total_balance.is_zero() {
            let rate_to_update =
                total_reward_distributed / Decimal::from_atomics(total_balance, 0)?;
            if rate_to_update > Decimal::zero() {
                ASSET_REWARD_RATE.update(
                    deps.storage,
                    &asset_distribution.asset,
                    |rate| -> StdResult<_> { Ok(rate.unwrap_or(Decimal::zero()) + rate_to_update) },
                )?;
            }
        }
    }

    Ok(Response::new().add_attributes(vec![("action", "update_rewards_callback")]))
}

// Only governance (through a on-chain prop) can change the whitelisted assets
fn assert_whitelist_controller(
    deps: &DepsMut,
    info: &MessageInfo,
    config: &Config,
) -> Result<(), ContractError> {
    GlobalConfig(config.global_config_addr.clone()).assert_has_access(
        &deps.querier,
        AT_ASSET_WHITELIST_CONTROLLER,
        &info.sender,
    )?;
    Ok(())
}

// Only governance or the operator can pass through this function
fn assert_distribution_controller(
    deps: &DepsMut,
    info: &MessageInfo,
    config: &Config,
) -> Result<(), ContractError> {
    GlobalConfig(config.global_config_addr.clone()).assert_has_access(
        &deps.querier,
        AT_REWARD_DISTRIBUTION_CONTROLLER,
        &info.sender,
    )?;
    Ok(())
}
