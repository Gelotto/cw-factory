use crate::{
    error::ContractError,
    state::{
        models::Config,
        storage::{CONFIG_ALLOWED_CODE_IDS, CONFIG_DEFAULT_CODE_ID, MANAGED_BY},
    },
};
use cosmwasm_std::{attr, ensure_eq, Response};

use super::Context;

pub fn exec_set_config(
    ctx: Context,
    config: Config,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    let Config {
        allowed_code_ids,
        default_code_id,
        managed_by,
    } = config;

    ensure_eq!(
        info.sender,
        MANAGED_BY.load(deps.storage)?,
        ContractError::NotAuthorized {
            reason: "only manager can update the factory config".to_owned()
        }
    );

    // Upsert manager address
    MANAGED_BY.save(deps.storage, &deps.api.addr_validate(managed_by.as_str())?)?;

    // Re-init allowed code IDs
    CONFIG_ALLOWED_CODE_IDS.clear(deps.storage);
    for code_id in allowed_code_ids.iter() {
        CONFIG_ALLOWED_CODE_IDS.save(deps.storage, code_id.u64(), &0)?;
    }

    // Save or remove default code ID
    if let Some(default_code_id) = default_code_id {
        CONFIG_ALLOWED_CODE_IDS.save(deps.storage, default_code_id.u64(), &0)?;
        CONFIG_DEFAULT_CODE_ID.save(deps.storage, &default_code_id)?;
    } else {
        CONFIG_DEFAULT_CODE_ID.remove(deps.storage);
    }

    Ok(Response::new().add_attributes(vec![attr("action", "set_config")]))
}
