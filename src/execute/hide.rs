use crate::{
    error::ContractError,
    state::{
        build_index_storage_key,
        storage::{IndexMap, CONTRACT_ADDR_2_ID, CONTRACT_CUSTOM_IX_VALUES, CONTRACT_ID_2_IS_HIDDEN},
    },
    util::ensure_is_manager,
};
use cosmwasm_std::{attr, Addr, Order, Response};
use cw_storage_plus::Map;

use super::Context;

/// Hide a contract from custom index range queries. Effectively, delist it. All
/// tags and relations remain unaffected.
pub fn exec_toggle_hide(
    ctx: Context,
    contract: Option<Addr>,
) -> Result<Response, ContractError> {
    let Context { deps, info, .. } = ctx;

    let contract_addr = if let Some(contract_addr) = contract {
        ensure_is_manager(deps.storage, &info.sender)?;
        contract_addr
    } else {
        info.sender.to_owned()
    };

    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &contract_addr)?;

    let is_hidden = CONTRACT_ID_2_IS_HIDDEN
        .may_load(deps.storage, contract_id)?
        .unwrap_or_default();

    // Get custom index names and values
    let entries: Vec<_> = CONTRACT_CUSTOM_IX_VALUES
        .prefix(contract_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.unwrap())
        .collect();

    if is_hidden {
        // Add entries back to custom indices.
        for (name, value_vec) in entries.iter() {
            let storage_key = build_index_storage_key(name);
            let map: IndexMap = Map::new(&storage_key);
            map.save(deps.storage, (value_vec.as_slice(), contract_id), &0)?;
        }
    } else {
        // Remove contract's entries from custom indices
        for (name, value_vec) in entries.iter() {
            let storage_key = build_index_storage_key(name);
            let map: IndexMap = Map::new(&storage_key);
            map.remove(deps.storage, (value_vec.as_slice(), contract_id));
        }
    }

    // Toggle the is_hidden state in storage
    CONTRACT_ID_2_IS_HIDDEN.save(deps.storage, contract_id, &(!is_hidden))?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "hide"),
        attr("is_hidden", (!is_hidden).to_string()),
    ]))
}
