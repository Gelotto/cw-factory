//! Update contract metadata, indices, tags, and relationships.
//!
//! ## Authorization Model
//!
//! Updates can be initiated by:
//! 1. **The contract itself** (self-updates): No selector needed, sender must be factory-managed contract
//! 2. **Factory manager** (forced updates): Must provide selector, sender must be manager
//!
//! ## Index Update Pattern
//!
//! All index updates follow the remove-then-insert pattern to maintain consistency:
//! 1. Load old value from reverse map (`ID_2_*`)
//! 2. Remove old entry from forward index (`IX_*`)
//! 3. Insert new value into forward index
//! 4. Update reverse map with new value
//!
//! This prevents orphaned index entries when values change.

use crate::{
    error::ContractError,
    msg::{
        ContractSelector,
        IndexUpdate,
        IndexValue,
        RelationUpdate,
        TagUpdate,
        UpdateMsg,
        UpdateOperation,
    },
    state::{
        build_index_storage_key,
        build_reverse_mapping_storage_key,
        storage::{
            ContractId,
            IndexMap,
            CONTRACT_ADDR_2_ID,
            CONTRACT_CUSTOM_IX_VALUES,
            CONTRACT_ID_2_ADDR,
            CONTRACT_NAME_2_ID,
            CONTRACT_TAG_WEIGHTS,
            ID_2_UPDATED_AT,
            IX_REL_ADDR,
            IX_REL_CONTRACT_ADDR,
            IX_TAG,
            IX_UPDATED_AT,
            IX_WEIGHTED_TAG,
        },
    },
    util::ensure_is_manager,
};
use cosmwasm_std::{
    attr,
    Addr,
    Response,
    Storage,
};
use cw_storage_plus::Map;

use super::Context;

/// Update contract indices, tags, and relationships.
///
/// ## Authorization
///
/// - If `contract` selector is None: msg.sender must be a factory-managed contract (self-update)
/// - If `contract` selector is Some: msg.sender must be factory manager (forced update)
///
/// ## Automatic Timestamp Update
///
/// Every update automatically refreshes the `updated_at` index with current block time.

pub fn exec_update(
    ctx: Context,
    msg: UpdateMsg,
) -> Result<Response, ContractError,> {

    let Context { deps, env, info, } = ctx;

    let UpdateMsg {
        contract: maybe_contract_selector,
        indices: index_updates,
        tags: tag_updates,
        relations: relation_updates,
    } = msg;

    // Resolve contract ID and verify authorization
    // Two modes: self-update (contract is sender) or manager-forced update
    let contract_id = if let Some(selector,) = maybe_contract_selector {

        // Manager-forced update: verify sender is manager
        ensure_is_manager(deps.storage, &info.sender,)?;

        // Resolve contract ID from selector
        match selector {
            ContractSelector::Address(addr,) => {
                CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(addr.as_str(),)?,)?
            },
            ContractSelector::Id(id,) => {

                if !CONTRACT_ID_2_ADDR.has(deps.storage, id,) {

                    return Err(ContractError::NotAuthorized {
                        reason: format!("contract ID not found: {}", id),
                    },);
                }

                id
            },
            ContractSelector::Name(name,) => CONTRACT_NAME_2_ID.load(deps.storage, &name,)?,
        }
    } else {

        // Self-update: sender is the contract itself
        // Will fail if sender is not a factory-managed contract
        CONTRACT_ADDR_2_ID.load(deps.storage, &info.sender,)?
    };

    // Automatically update the updated_at index with current block time
    // This happens on every update, providing a "last modified" timestamp
    {

        // Remove old timestamp entry from index
        if let Some(old_bytes,) = ID_2_UPDATED_AT.may_load(deps.storage, contract_id,)? {

            IX_UPDATED_AT.remove(deps.storage, (old_bytes.as_slice(), contract_id,),);
        }

        // Insert new timestamp entry
        let t = env.block.time.nanos().to_le_bytes();

        IX_UPDATED_AT.save(deps.storage, (&t, contract_id,), &0,)?;

        ID_2_UPDATED_AT.save(deps.storage, contract_id, &t.to_vec(),)?;
    }

    // Apply custom index updates using the remove-then-insert pattern
    for IndexUpdate { name, value, } in index_updates.unwrap_or_default().iter() {

        // Convert index value to bytes for storage
        let bytes = value.to_bytes();

        // Track that this contract uses this custom index
        // This enables queries like "list all custom indices for a contract"
        if !CONTRACT_CUSTOM_IX_VALUES.has(deps.storage, (contract_id, name,),) {

            CONTRACT_CUSTOM_IX_VALUES.save(deps.storage, (contract_id, name,), &bytes,)?;
        }

        // Build dynamic storage keys for this custom index
        let storage_key = build_index_storage_key(name,);

        let map: IndexMap = Map::new(&storage_key,);

        // Build dynamic storage key for reverse lookup
        let reverse_mapping_storage_key = build_reverse_mapping_storage_key(name,);

        let reverse_map: Map<ContractId, Vec<u8,>,> = Map::new(&reverse_mapping_storage_key,);

        // Remove old index entry (if exists) before adding new one
        // This is critical to prevent orphaned entries when values change
        if let Some(old_bytes,) = reverse_map.may_load(deps.storage, contract_id,)? {

            map.remove(deps.storage, (old_bytes.as_slice(), contract_id,),);
        }

        // Insert new value into forward index and reverse map
        map.save(deps.storage, (&bytes, contract_id,), &0,)?;

        reverse_map.save(deps.storage, contract_id, &bytes,)?;
    }

    // Update tags
    for TagUpdate { op, tag, weight, } in tag_updates.unwrap_or_default().iter() {

        match op {
            UpdateOperation::Set => {

                set_tag(deps.storage, contract_id, tag.to_owned(), weight.to_owned(),)?;
            },
            UpdateOperation::Remove => {

                let tag_bytes = IndexValue::String(tag.to_owned(),).to_bytes();

                remove_tag(deps.storage, contract_id, &tag_bytes,)?;
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

                set_relation(deps.storage, contract_id, name, address, value.to_owned(),)?;
            },
            UpdateOperation::Remove => {

                let name_bytes = IndexValue::String(name.to_owned(),).to_bytes();

                remove_relation(deps.storage, contract_id, &name_bytes, address.as_bytes(),)?;
            },
        }
    }

    Ok(Response::new().add_attributes(vec![attr("action", "update",)],),)
}

/// Remove a relationship from both forward and reverse indices.

fn remove_relation(
    store: &mut dyn Storage,
    contract_id: ContractId,
    edge: &[u8],
    rel_addr: &[u8],
) -> Result<(), ContractError,> {

    // Remove from reverse index (queries from address to contracts)
    IX_REL_ADDR.remove(store, (rel_addr, edge, contract_id,),);

    // Remove from forward index (queries from contract to addresses)
    IX_REL_CONTRACT_ADDR.remove(store, (contract_id, edge, rel_addr,),);

    Ok((),)
}

/// Set or update a directed relationship with optional value.
///
/// Relationships are stored bidirectionally:
/// - `IX_REL_CONTRACT_ADDR`: from contract's perspective (queries: "who am I related to?")
/// - `IX_REL_ADDR`: from address's perspective (queries: "who relates to me?")
///
/// Edge encoding: `name_bytes || value_bytes` (concatenated for composite key)

fn set_relation(
    store: &mut dyn Storage,
    contract_id: ContractId,
    rel_name: &String,
    rel_addr: &Addr,
    value: Option<IndexValue,>,
) -> Result<(), ContractError,> {

    let rel_addr = rel_addr.as_bytes();

    // Build edge bytes: name + optional value
    // This allows querying by both name and value ranges
    let mut edge = IndexValue::String(rel_name.to_owned(),).to_bytes();

    if let Some(value,) = &value {

        // Append value bytes to edge for composite key
        edge.extend(value.to_bytes(),);
    }

    // Remove old relationship entry if exists (handles updates)
    remove_relation(store, contract_id, &edge, rel_addr,)?;

    // Store bidirectional relationship
    IX_REL_ADDR.save(store, (rel_addr, &edge, contract_id,), &0,)?;

    IX_REL_CONTRACT_ADDR.save(store, (contract_id, &edge, rel_addr,), &value,)?;

    Ok((),)
}

/// Set or update a tag with optional weight.
///
/// Tags use the weighted index for efficient filtering by importance/priority.
/// Weight defaults to 0 if not specified.
///
/// Stores in three indices:
/// - `IX_TAG`: Simple tag presence
/// - `IX_WEIGHTED_TAG`: Tag sorted by weight
/// - `CONTRACT_TAG_WEIGHTS`: Weight lookup for updates

fn set_tag(
    store: &mut dyn Storage,
    contract_id: ContractId,
    tag: String,
    weight: Option<u16,>,
) -> Result<(), ContractError,> {

    let tag_bytes = &IndexValue::String(tag,).to_bytes();

    let weight = weight.unwrap_or_default();

    // Remove old tag entry first (handles weight changes)
    remove_tag(store, contract_id, tag_bytes,)?;

    // Store tag in all three indices
    CONTRACT_TAG_WEIGHTS.save(store, (contract_id, tag_bytes,), &weight,)?;

    IX_WEIGHTED_TAG.save(store, (tag_bytes, weight, contract_id,), &0,)?;

    IX_TAG.save(store, (tag_bytes, contract_id,), &0,)?;

    Ok((),)
}

/// Remove a tag from all tag indices.
///
/// Loads the weight from CONTRACT_TAG_WEIGHTS to remove the correct entry
/// from IX_WEIGHTED_TAG (which requires weight as part of the key).

fn remove_tag(
    store: &mut dyn Storage,
    contract_id: ContractId,
    tag_bytes: &[u8],
) -> Result<(), ContractError,> {

    // Load weight to remove from weighted index
    if let Some(weight,) = CONTRACT_TAG_WEIGHTS.may_load(store, (contract_id, tag_bytes,),)? {

        CONTRACT_TAG_WEIGHTS.remove(store, (contract_id, tag_bytes,),);

        // Remove from weighted index (requires weight as part of composite key)
        IX_WEIGHTED_TAG.remove(store, (tag_bytes, weight, contract_id,),);

        IX_TAG.remove(store, (tag_bytes, contract_id,),);
    }

    Ok((),)
}
