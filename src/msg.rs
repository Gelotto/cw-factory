use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Int128, Int64, Uint128, Uint64};

use crate::state::{models::Config, storage::ContractId};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetConfig(Config),
    Create(CreateMsg),
    Update(UpdateMsg),
}

#[cw_serde]
pub enum ContractsQueryMsg {
    ByIndex(ContractsByIndexParams),
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    Contracts(ContractsQueryMsg),
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
    pub name: Option<String>,
    pub label: String,
    pub admin: Option<Addr>,
    pub tags: Option<Vec<String>>,
}

#[cw_serde]
pub enum IndexValue {
    Bytes(Vec<u8>),
    String(String),
    Bool(bool),
    Binary(Binary),
    Uint128(Uint128),
    Uint64(Uint64),
    Uint32(u32),
    Uint16(u16),
    Uint8(u8),
    Int128(Int128),
    Int64(Int64),
    Int32(i32),
    Int16(i16),
    Int8(i8),
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
                let mut bytes = s.as_bytes().to_vec();
                let n = 500 - bytes.len();
                bytes.extend_from_slice(vec![0; n].as_slice());
                bytes
            },
            IndexValue::Uint128(x) => x.to_le_bytes().to_vec(),
            IndexValue::Uint64(x) => x.to_le_bytes().to_vec(),
            IndexValue::Uint32(x) => x.to_le_bytes().to_vec(),
            IndexValue::Uint16(x) => x.to_le_bytes().to_vec(),
            IndexValue::Uint8(x) => x.to_le_bytes().to_vec(),
            IndexValue::Int128(x) => x.to_le_bytes().to_vec(),
            IndexValue::Int64(x) => x.to_le_bytes().to_vec(),
            IndexValue::Int32(x) => x.to_le_bytes().to_vec(),
            IndexValue::Int16(x) => x.to_le_bytes().to_vec(),
            IndexValue::Int8(x) => x.to_le_bytes().to_vec(),
            IndexValue::Bool(x) => vec![if *x { 1u8 } else { 0u8 }],
            IndexValue::Binary(x) => x.to_vec(),
        }
    }
}

#[cw_serde]
pub enum ContractSelector {
    Address(Addr),
    Id(ContractId),
    Name(String),
}

#[cw_serde]
pub struct UpdateMsg {
    pub contract: Option<ContractSelector>,
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
pub struct ContractsByIndexParams {
    pub index: IndexSelector,
    pub start: Option<PaginationRangeBound>,
    pub stop: Option<PaginationRangeBound>,
    pub cursor: Option<(Vec<u8>, ContractId)>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}
