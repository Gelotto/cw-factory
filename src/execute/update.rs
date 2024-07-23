use crate::{
    error::ContractError,
    msg::{ContractSelector, IndexUpdate, IndexValue, RelationUpdate, TagUpdate, UpdateMsg, UpdateOperation},
    state::{
        build_index_storage_key, build_reverse_mapping_storage_key,
        storage::{
            ContractId, IndexMap, CONTRACT_ADDR_2_ID, CONTRACT_CUSTOM_IX_VALUES, CONTRACT_ID_2_ADDR,
            CONTRACT_NAME_2_ID, CONTRACT_TAG_WEIGHTS, ID_2_UPDATED_AT, IX_REL_ADDR, IX_REL_CONTRACT_ADDR, IX_TAG,
            IX_UPDATED_AT, IX_WEIGHTED_TAG,
        },
    },
    util::ensure_is_manager,
};
use cosmwasm_std::{attr, Addr, Response, Storage};
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
        relations: relation_updates,
    } = msg;

    // Get ID of contract applying updates. Sender must be either the
    // contract itself, assuming it is managed by this factory, or the factory
    // manager. No one else.
    let contract_id = if let Some(selector) = maybe_contract_selector {
        ensure_is_manager(deps.storage, &info.sender)?;
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
        // sender is assumed to be the factory-managed contract itself
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
        if !CONTRACT_CUSTOM_IX_VALUES.has(deps.storage, (contract_id, name)) {
            CONTRACT_CUSTOM_IX_VALUES.save(deps.storage, (contract_id, name), &bytes)?;
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

    // Update tags
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

    // Update relations
    for RelationUpdate {
        op,
        name,
        value,
        address,
    } in relation_updates.unwrap_or_default().iter()
    {
        match op {
            UpdateOperation::Set => {
                set_relation(deps.storage, contract_id, name, address, value.to_owned())?;
            },
            UpdateOperation::Remove => {
                let name_bytes = IndexValue::String(name.to_owned()).to_bytes();
                remove_relation(deps.storage, contract_id, &name_bytes, address.as_bytes())?;
            },
        }
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update")]))
}

fn remove_relation(
    store: &mut dyn Storage,
    contract_id: ContractId,
    edge: &[u8],
    rel_addr: &[u8],
) -> Result<(), ContractError> {
    IX_REL_ADDR.remove(store, (rel_addr, edge, contract_id));
    IX_REL_CONTRACT_ADDR.remove(store, (contract_id, edge, rel_addr));
    Ok(())
}

fn set_relation(
    store: &mut dyn Storage,
    contract_id: ContractId,
    rel_name: &String,
    rel_addr: &Addr,
    value: Option<IndexValue>,
) -> Result<(), ContractError> {
    let rel_addr = rel_addr.as_bytes();
    let mut edge = IndexValue::String(rel_name.to_owned()).to_bytes();
    if let Some(value) = &value {
        edge.extend(value.to_bytes());
    }
    remove_relation(store, contract_id, &edge, rel_addr)?;
    IX_REL_ADDR.save(store, (rel_addr, &edge, contract_id), &0)?;
    IX_REL_CONTRACT_ADDR.save(store, (contract_id, &edge, rel_addr), &value)?;
    Ok(())
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
