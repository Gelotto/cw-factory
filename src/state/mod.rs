pub mod models;
pub mod storage;

use cosmwasm_std::Response;
use storage::{CONTRACT_COUNTER, CREATED_AT, CREATED_BY, MANAGED_BY};

use crate::{error::ContractError, execute::Context, msg::InstantiateMsg};

/// Top-level initialization of contract state
pub fn init(
    ctx: Context,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let InstantiateMsg { config } = msg;

    CONTRACT_COUNTER.save(deps.storage, &0)?;
    MANAGED_BY.save(deps.storage, &deps.api.addr_validate(config.managed_by.as_str())?)?;
    CREATED_BY.save(deps.storage, &info.sender)?;
    CREATED_AT.save(deps.storage, &env.block.time)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

pub fn build_index_storage_key(index_name: &String) -> String {
    format!("_ix_{}", index_name)
}

pub fn build_reverse_mapping_storage_key(index_name: &String) -> String {
    format!("_id_2_{}", index_name)
}
