use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint64};
use serde_json::{Map as SerdeMap, Value};

use crate::msg::MigrationParams;

use super::storage::ContractId;

#[cw_serde]
pub struct Config {
    pub managed_by: Addr,
    pub default_code_id: Option<Uint64>,
    pub allowed_code_ids: Vec<Uint64>,
}

#[cw_serde]
pub struct SubMsgContext {
    pub code_id: Uint64,
    pub contract_id: ContractId,
    pub created_by: Addr,
    pub name: Option<String>,
    pub admin: Addr,
}

#[cw_serde]
pub struct Preset {
    pub values: SerdeMap<String, Value>,
    pub overridable: bool,
    pub n_uses: u32,
}

#[cw_serde]
pub enum MigrationStatus {
    Running,
    Complete,
    Aborted,
}

#[cw_serde]
pub struct Migration {
    pub params: MigrationParams,
    pub status: MigrationStatus,
    pub cursor: Option<ContractId>,
    pub retry_cursor: Option<ContractId>,
    pub n_success: u32,
    pub n_error: u32,
}

#[cw_serde]
pub struct MigrationError {
    pub contract: Addr,
    pub error: String,
    pub reply_id: Uint64,
}

#[cw_serde]
pub enum MigrationErrorStrategy {
    Abort,
    Retry,
}
