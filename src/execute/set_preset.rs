use crate::{
    error::ContractError,
    msg::SetPresetMsg,
    state::{
        models::Preset,
        storage::{MANAGED_BY, PRESETS},
    },
};
use cosmwasm_std::{attr, ensure_eq, Response};

use super::Context;

pub fn exec_set_preset(
    ctx: Context,
    msg: SetPresetMsg,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;
    let SetPresetMsg {
        name,
        values,
        overridable,
    } = msg;

    ensure_eq!(
        info.sender,
        MANAGED_BY.load(deps.storage)?,
        ContractError::NotAuthorized {
            reason: "only manager can set presets".to_owned()
        }
    );

    PRESETS.save(
        deps.storage,
        &name,
        &Preset {
            values,
            overridable,
            n_uses: 0,
        },
    )?;

    Ok(Response::new().add_attributes(vec![attr("action", "set_preset"), attr("preset", name)]))
}

pub fn exec_remove_preset(
    ctx: Context,
    name: String,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;

    ensure_eq!(
        info.sender,
        MANAGED_BY.load(deps.storage)?,
        ContractError::NotAuthorized {
            reason: "only manager can set presets".to_owned()
        }
    );

    PRESETS.remove(deps.storage, &name);

    Ok(Response::new().add_attributes(vec![attr("action", "delete_preset"), attr("preset", name)]))
}
