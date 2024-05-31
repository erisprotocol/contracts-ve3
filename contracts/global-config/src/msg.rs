use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw_ownable::{cw_ownable_execute, Ownership};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    SetAddresses {
        addresses: Vec<(String, String)>,
    },

    SetAdressList {
        address_type: String,
        addresses: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Ownership<String>)]
    Ownership {},

    /// Get a single address
    #[returns(AddressResponse)]
    Address(String),

    /// Get a list of addresses
    #[returns(Vec<AddressResponse>)]
    Addresses(Vec<String>),

    /// Query all stored addresses with pagination
    #[returns(Vec<AddressResponse>)]
    AllAddresses {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get a single address
    #[returns(AddressListResponse)]
    AddressList(String),
}

pub type AddressResponse = (String, Addr);
pub type AddressListResponse = (String, Vec<Addr>);

#[cw_serde]
pub struct MigrateMsg {}
