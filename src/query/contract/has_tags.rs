use cosmwasm_std::Storage;

use crate::{
    error::ContractError,
    msg::{BooleanTest, ContractHasTagsQueryParams, IndexValue, TagSelector},
    query::ReadonlyContext,
    state::storage::{ContractId, CONTRACT_ADDR_2_ID, CONTRACT_TAG_WEIGHTS},
};

pub fn query_contract_has_tags(
    ctx: ReadonlyContext,
    msg: ContractHasTagsQueryParams,
) -> Result<bool, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(msg.contract.as_str())?)?;
    match msg.test {
        BooleanTest::And => {
            for selector in msg.tags.iter() {
                if !has_tag(deps.storage, contract_id, selector)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        },
        BooleanTest::Or => {
            for selector in msg.tags.iter() {
                if has_tag(deps.storage, contract_id, selector)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        },
        BooleanTest::Xor => {
            let mut test_passes = false;
            for selector in msg.tags.iter() {
                if has_tag(deps.storage, contract_id, selector)? {
                    if !test_passes {
                        test_passes = true;
                    } else {
                        return Ok(false);
                    }
                }
            }
            return Ok(test_passes);
        },
    }
}

fn has_tag(
    store: &dyn Storage,
    contract_id: ContractId,
    selector: &TagSelector,
) -> Result<bool, ContractError> {
    let tag_bytes_vec = IndexValue::String(selector.tag.to_owned()).to_bytes();
    let tag_bytes = tag_bytes_vec.as_slice();
    if let Some(weight) = CONTRACT_TAG_WEIGHTS.may_load(store, (contract_id, tag_bytes))? {
        if let Some(min_weight) = selector.min_weight {
            if weight < min_weight {
                return Ok(false);
            }
        }
        if let Some(max_weight) = selector.max_weight {
            if weight > max_weight {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(false)
}
