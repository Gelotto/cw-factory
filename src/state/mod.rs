pub mod models;
pub mod storage;

use cosmwasm_std::{Response, Uint64};
use storage::{
    CONFIG_ALLOWED_CODE_IDS, CONFIG_DEFAULT_CODE_ID, CONTRACT_COUNTER, CONTRACT_ID_COUNTER, CREATED_AT, CREATED_BY,
    MANAGED_BY, REPLY_ID_COUNTER,
};

use crate::{error::ContractError, execute::Context, msg::InstantiateMsg};

/// Top-level initialization of contract state
pub fn init(
    ctx: Context,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let InstantiateMsg { config } = msg;

    REPLY_ID_COUNTER.save(deps.storage, &Uint64::zero())?;
    CONTRACT_ID_COUNTER.save(deps.storage, &0)?;
    CONTRACT_COUNTER.save(deps.storage, &0)?;
    MANAGED_BY.save(deps.storage, &deps.api.addr_validate(config.managed_by.as_str())?)?;
    CREATED_BY.save(deps.storage, &info.sender)?;
    CREATED_AT.save(deps.storage, &env.block.time)?;

    for code_id in config.allowed_code_ids.iter() {
        CONFIG_ALLOWED_CODE_IDS.save(deps.storage, code_id.u64(), &0)?;
    }

    if let Some(default_code_id) = config.default_code_id {
        CONFIG_ALLOWED_CODE_IDS.save(deps.storage, default_code_id.u64(), &0)?;
        CONFIG_DEFAULT_CODE_ID.save(deps.storage, &default_code_id)?;
    }

    Ok(Response::new().add_attribute("action", "instantiate"))
}

pub fn build_index_storage_key(index_name: &String) -> String {
    format!("_ix_{}", index_name)
}

pub fn build_reverse_mapping_storage_key(index_name: &String) -> String {
    format!("_id_2_{}", index_name)
}
