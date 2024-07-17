use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Int128, Int64, Timestamp, Uint128, Uint64};
use serde_json::{Map as SerdeMap, Value};

use crate::state::{models::Config, storage::ContractId};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config,
}

#[cw_serde]
pub enum ExecuteMsg {
    SetConfig(Config),
    Presets(PresetsExecuteMsg),
    Create(CreateMsg),
    Update(UpdateMsg),
}

#[cw_serde]
pub enum PresetsExecuteMsg {
    Set(SetPresetMsg),
    Remove { name: String },
}

#[cw_serde]
pub enum ContractSetQueryMsg {
    InRange(ContractsInRangeQueryParams),
    WithTag(ContractsByTagQueryParams),
    RelatedTo(ContractsRelatedToParams),
}

#[cw_serde]
pub enum ContractQueryMsg {
    HasRelations(ContractHasRelationsQueryParams),
    Relations(ContractRelationsQueryParams),
    HasTags(ContractHasTagsQueryParams),
    Tags(ContractTagsQueryParams),
    Metadata { address: Addr },
}

#[cw_serde]
pub enum PresetsQueryMsg {
    Get { name: String },
    Paginate { cursor: Option<String> },
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    Contracts(ContractSetQueryMsg),
    Contract(ContractQueryMsg),
    Presets(PresetsQueryMsg),
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum BooleanTest {
    And,
    Or,
    Xor,
}

#[cw_serde]
pub struct ContractHasTagsQueryParams {
    pub address: Addr,
    pub test: BooleanTest,
    pub tags: Vec<TagSelector>,
}

#[cw_serde]
pub struct ContractHasRelationsQueryParams {
    pub test: BooleanTest,
    pub relations: Vec<NameValue>,
    pub contract_address: Addr,
    pub address: Addr,
}

#[cw_serde]
pub struct ContractIsRelatedToResponse {
    pub is_related: bool,
    pub values: Vec<NameValue>,
}

#[cw_serde]
pub struct ContractRelationsQueryParams {
    pub address: Addr,
    pub cursor: Option<(String, Addr)>,
    pub start: Option<RangeQueryBound<String>>,
    pub stop: Option<RangeQueryBound<String>>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub struct NameValue {
    pub name: String,
    pub value: Option<String>,
}

#[cw_serde]
pub struct RelatedAddress {
    pub address: Addr,
    pub name: String,
    pub value: Option<String>,
}

#[cw_serde]
pub struct ContractRelationsResponse {
    pub cursor: Option<(String, Addr)>,
    pub relations: Vec<RelatedAddress>,
}

#[cw_serde]
pub struct ContractTagsQueryParams {
    pub contract: Addr,
    pub cursor: Option<String>,
    pub start: Option<RangeQueryBound<String>>,
    pub stop: Option<RangeQueryBound<String>>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub struct ContractTagsResponse {
    pub cursor: Option<String>,
    pub tags: Vec<WeightedTag>,
}

#[cw_serde]
pub struct ContractMetadataResponse {
    pub created_at: Timestamp,
    pub created_by: Addr,
    pub updated_at: Timestamp,
    pub name: Option<String>,
    pub code_id: Uint64,
    pub admin: Addr,
}

#[cw_serde]
pub struct ConfigResponse(pub Config);

#[cw_serde]
pub struct PresetResponse {
    pub name: String,
    pub values: SerdeMap<String, Value>,
    pub overridable: bool,
    pub n_uses: u32,
}

#[cw_serde]
pub struct PresetPaginationResponse {
    pub cursor: Option<String>,
    pub presets: Vec<PresetResponse>,
}

#[cw_serde]
pub struct ContractsByIndexResponse {
    pub addresses: Vec<Addr>,
    pub cursor: Option<(Vec<u8>, ContractId)>,
}

#[cw_serde]
pub struct ContractsByTagResponse {
    pub addresses: Vec<Addr>,
    pub weights: Vec<u16>,
    pub cursor: Option<(Vec<u8>, u16, ContractId)>,
}

#[cw_serde]
pub struct ContractsRelatedToResponse {
    pub addresses: Vec<Addr>,
    pub values: Vec<Option<String>>,
    pub cursor: Option<ContractId>,
}

#[cw_serde]
pub struct CreateMsg {
    pub preset: Option<String>,
    pub code_id: Option<Uint64>,
    pub instantiate_msg: SerdeMap<String, Value>,
    pub name: Option<String>,
    pub label: String,
    pub admin: Option<Addr>,
    pub tags: Option<Vec<String>>,
}

#[cw_serde]
pub struct SetPresetMsg {
    pub name: String,
    pub values: SerdeMap<String, Value>,
    pub overridable: bool,
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
pub enum UpdateOperation {
    Remove,
    Set,
}

#[cw_serde]
pub struct WeightedTag {
    pub tag: String,
    pub weight: u16,
}

#[cw_serde]
pub struct TagUpdate {
    pub op: UpdateOperation,
    pub tag: String,
    pub weight: Option<u16>,
}

#[cw_serde]
pub struct RelationUpdate {
    pub op: UpdateOperation,
    pub name: String,
    pub value: Option<String>,
    pub address: Addr,
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
    pub relations: Option<Vec<RelationUpdate>>,
    pub tags: Option<Vec<TagUpdate>>,
}

#[cw_serde]
pub enum IndexSelector {
    Custom(String),
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    CodeId,
    Admin,
    Tag,
}

#[cw_serde]
pub struct TagSelector {
    pub tag: String,
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
pub enum RangeQueryBound<T> {
    Exclusive(T),
    Inclusive(T),
}

#[cw_serde]
pub enum IndexRangeBound {
    Exclusive(IndexValue),
    Inclusive(IndexValue),
}

#[cw_serde]
pub enum TagWeightRangeBound {
    Exclusive(u16),
    Inclusive(u16),
}

#[cw_serde]
pub struct ContractsInRangeQueryParams {
    pub cursor: Option<(Vec<u8>, ContractId)>,
    pub index: IndexSelector,
    pub start: Option<IndexRangeBound>,
    pub stop: Option<IndexRangeBound>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub struct ContractsByTagQueryParams {
    pub cursor: Option<(Vec<u8>, u16, ContractId)>,
    pub tag: String,
    pub min_weight: Option<TagWeightRangeBound>,
    pub max_weight: Option<TagWeightRangeBound>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
}

#[cw_serde]
pub struct ContractsRelatedToParams {
    pub cursor: Option<ContractId>,
    pub limit: Option<u16>,
    pub desc: Option<bool>,
    pub name: String,
    pub address: Addr,
}
