use crate::{
    error::ContractError,
    msg::{IndexUpdate, UpdateMsg},
    state::{
        build_index_storage_key, build_reverse_mapping_storage_key,
        storage::{ContractId, IndexMap, CONTRACT_ADDR_2_ID, ID_2_UPDATED_AT, IX_UPDATED_AT, MANAGED_BY},
    },
};
use cosmwasm_std::{attr, ensure_eq, Response};
use cw_storage_plus::Map;

use super::Context;

pub fn exec_update(
    ctx: Context,
    msg: UpdateMsg,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let UpdateMsg {
        contract: maybe_contract_addr,
        indices: index_updates,
    } = msg;

    // Get addr of contract applying updates. The sender must be either the
    // contract itself, assuming it is managed by this factory, or the factory
    // maanger. Not one else.
    let contract_addr = if let Some(addr) = maybe_contract_addr {
        let manager = MANAGED_BY.load(deps.storage)?;
        ensure_eq!(
            info.sender,
            manager,
            ContractError::NotAuthorized {
                reason: "only manager or a contract itself can apply updates".to_owned()
            }
        );
        deps.api.addr_validate(addr.as_str())?
    } else {
        info.sender.to_owned()
    };

    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(contract_addr.as_str())?)?;

    // Update the contract's entry in the updated_at index
    {
        // update entry in updated_at index
        if let Some(old_bytes) = ID_2_UPDATED_AT.may_load(deps.storage, contract_id)? {
            IX_UPDATED_AT.remove(deps.storage, (old_bytes.as_slice(), contract_id));
        }

        // insert updated values in index and the reverse lookup map
        let t = env.block.time.nanos().to_le_bytes();
        IX_UPDATED_AT.save(deps.storage, (&t, contract_id), &0)?;
        ID_2_UPDATED_AT.save(deps.storage, contract_id, &t.to_vec())?;
    }

    // Apply each index updated received
    for IndexUpdate { name, value } in index_updates.unwrap_or_default().iter() {
        // Normalized received index value to u8 slice
        let bytes = value.to_bytes();

        // Get index map
        let storage_key = build_index_storage_key(name);
        let map: IndexMap = Map::new(&storage_key);

        // Get value reverse lookup map
        let reverse_mapping_storage_key = build_reverse_mapping_storage_key(name);
        let reverse_map: Map<ContractId, Vec<u8>> = Map::new(&reverse_mapping_storage_key);

        // remove previous entry from index, which is now stale
        if let Some(old_bytes) = reverse_map.may_load(deps.storage, contract_id)? {
            map.remove(deps.storage, (old_bytes.as_slice(), contract_id));
        }

        // insert updated values in index and the reverse lookup map
        map.save(deps.storage, (&bytes, contract_id), &0)?;
        reverse_map.save(deps.storage, contract_id, &bytes)?;
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update")]))
}
