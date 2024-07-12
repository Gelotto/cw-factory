use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint64};

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
    pub admin: Addr,
}
