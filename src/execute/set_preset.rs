use crate::{
    error::ContractError,
    msg::SetPresetMsg,
    state::{models::Preset, storage::PRESETS},
};
use cosmwasm_std::{attr, Response};

use super::Context;

pub fn exec_set_preset(
    ctx: Context,
    msg: SetPresetMsg,
) -> Result<Response, ContractError> {
    let Context { deps, .. } = ctx;
    let SetPresetMsg {
        name,
        values,
        overridable,
    } = msg;

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
    let Context { deps, .. } = ctx;

    PRESETS.remove(deps.storage, &name);

    Ok(Response::new().add_attributes(vec![attr("action", "delete_preset"), attr("preset", name)]))
}
