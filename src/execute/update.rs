use crate::{
    error::ContractError,
    msg::{ContractSelector, IndexUpdate, IndexValue, TagUpdate, UpdateMsg, UpdateOperation},
    state::{
        build_index_storage_key, build_reverse_mapping_storage_key,
        storage::{
            ContractId, IndexMap, CONTRACT_ADDR_2_ID, CONTRACT_ID_2_ADDR, CONTRACT_NAME_2_ID, CONTRACT_TAG_WEIGHTS,
            CUSTOM_INDEX_NAMES, ID_2_UPDATED_AT, IX_TAG, IX_UPDATED_AT, IX_WEIGHTED_TAG, MANAGED_BY,
        },
    },
};
use cosmwasm_std::{attr, ensure_eq, Response, Storage};
use cw_storage_plus::Map;

use super::Context;

pub fn exec_update(
    ctx: Context,
    msg: UpdateMsg,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;
    let UpdateMsg {
        contract: maybe_contract_selector,
        indices: index_updates,
        tags: tag_updates,
    } = msg;

    // Get ID of contract applying updates. Sender must be either the
    // contract itself, assuming it is managed by this factory, or the factory
    // manager. No one else.
    let contract_id = if let Some(selector) = maybe_contract_selector {
        let manager = MANAGED_BY.load(deps.storage)?;
        ensure_eq!(
            info.sender,
            manager,
            ContractError::NotAuthorized {
                reason: "only manager or a contract itself can apply updates".to_owned()
            }
        );
        match selector {
            ContractSelector::Address(addr) => {
                CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(addr.as_str())?)?
            },
            ContractSelector::Id(id) => {
                if !CONTRACT_ID_2_ADDR.has(deps.storage, id) {
                    return Err(ContractError::NotAuthorized {
                        reason: format!("contract ID not found: {}", id),
                    });
                }
                id
            },
            ContractSelector::Name(name) => CONTRACT_NAME_2_ID.load(deps.storage, &name)?,
        }
    } else {
        CONTRACT_ADDR_2_ID.load(deps.storage, &info.sender)?
    };

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

    // Apply each index update
    for IndexUpdate { name, value } in index_updates.unwrap_or_default().iter() {
        // Normalized received index value to u8 slice
        let bytes = value.to_bytes();

        // Track the fact that this index contains an entry for this contract so
        // we can do things like
        if !CUSTOM_INDEX_NAMES.has(deps.storage, (contract_id, name)) {
            CUSTOM_INDEX_NAMES.save(deps.storage, (contract_id, name), &0)?;
        }

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

    // Apply tag updates
    for TagUpdate { op, tag, weight } in tag_updates.unwrap_or_default().iter() {
        match op {
            UpdateOperation::Set => {
                set_tag(deps.storage, contract_id, tag.to_owned(), weight.to_owned())?;
            },
            UpdateOperation::Remove => {
                let tag_bytes = IndexValue::String(tag.to_owned()).to_bytes();
                remove_tag(deps.storage, contract_id, &tag_bytes)?;
            },
        }
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update")]))
}

fn set_tag(
    store: &mut dyn Storage,
    contract_id: ContractId,
    tag: String,
    weight: Option<u16>,
) -> Result<(), ContractError> {
    let tag_bytes = &IndexValue::String(tag).to_bytes();
    let weight = weight.unwrap_or_default();
    remove_tag(store, contract_id, tag_bytes)?;
    CONTRACT_TAG_WEIGHTS.save(store, (contract_id, tag_bytes), &0)?;
    IX_WEIGHTED_TAG.save(store, (tag_bytes, weight, contract_id), &0)?;
    IX_TAG.save(store, (tag_bytes, contract_id), &0)?;
    Ok(())
}

fn remove_tag(
    store: &mut dyn Storage,
    contract_id: ContractId,
    tag_bytes: &[u8],
) -> Result<(), ContractError> {
    if let Some(weight) = CONTRACT_TAG_WEIGHTS.may_load(store, (contract_id, tag_bytes))? {
        CONTRACT_TAG_WEIGHTS.remove(store, (contract_id, tag_bytes));
        IX_WEIGHTED_TAG.remove(store, (tag_bytes, weight, contract_id));
        IX_TAG.remove(store, (tag_bytes, contract_id));
    }
    Ok(())
}
