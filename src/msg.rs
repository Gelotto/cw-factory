use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Coin, Uint64};

use crate::state::{models::Config, storage::ContractId};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetConfig(Config),
    Create(CreateMsg),
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ConfigResponse(pub Config);

#[cw_serde]
pub struct ContractsByIndexResponse {
    pub addresses: Vec<Addr>,
    pub cursor: Option<(Vec<u8>, ContractId)>,
}

#[cw_serde]
pub struct CreateMsg {
    pub code_id: Option<Uint64>,
    pub instantiate_msg: Binary,
    pub label: String,
    pub admin: Option<Addr>,
    pub tags: Option<Vec<String>>,
}

#[cw_serde]
pub enum IndexValue {
    Bytes(Vec<u8>),
    String(String),
}

#[cw_serde]
pub struct IndexUpdate {
    pub name: String,
    pub value: IndexValue,
}

impl IndexValue {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Bytes(bytes) => bytes.to_owned(),
            IndexValue::String(s) => {
                let bytes_vec = s.as_bytes().to_vec();
                // let n = max_sizeof_str - bytes_vec.len();
                // bytes_vec.extend_from_slice(vec![0; n].as_slice());
                bytes_vec
            },
        }
    }
}

#[cw_serde]
pub struct UpdateMsg {
    pub contract: Option<Addr>,
    pub indices: Option<Vec<IndexUpdate>>,
}

#[cw_serde]
pub enum IndexSelector {
    Custom(String),
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    CodeId,
    Admin,
}

#[cw_serde]
pub struct TagSelector {
    pub text: String,
    pub min_weight: Option<u16>,
    pub max_weight: Option<u16>,
}

#[cw_serde]
pub struct RelationshipSelector {
    pub name: String,
    pub value: IndexValue,
}

#[cw_serde]
pub enum ContractPaginationSelector {
    Index(IndexSelector),
    Relationship(RelationshipSelector),
    Tag(TagSelector),
}

#[cw_serde]
pub enum PaginationRangeBound {
    Exclusive(Vec<u8>),
    Inclusive(Vec<u8>),
}

#[cw_serde]
pub struct ContractPaginationParams {
    pub index: IndexSelector,
    pub start: Option<PaginationRangeBound>,
    pub stop: Option<PaginationRangeBound>,
    pub cursor: Option<(Vec<u8>, ContractId)>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}
