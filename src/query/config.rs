use cosmwasm_std::Order;

use crate::{
    error::ContractError,
    msg::ConfigResponse,
    state::{
        models::Config,
        storage::{CONFIG_ALLOWED_CODE_IDS, CONFIG_DEFAULT_CODE_ID, MANAGED_BY},
    },
};

use super::ReadonlyContext;

pub fn query_config(ctx: ReadonlyContext) -> Result<ConfigResponse, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    Ok(ConfigResponse(Config {
        managed_by: MANAGED_BY.load(deps.storage)?,
        default_code_id: CONFIG_DEFAULT_CODE_ID.may_load(deps.storage)?,
        allowed_code_ids: CONFIG_ALLOWED_CODE_IDS
            .keys(deps.storage, None, None, Order::Ascending)
            .map(|r| r.unwrap().into())
            .collect(),
    }))
}
