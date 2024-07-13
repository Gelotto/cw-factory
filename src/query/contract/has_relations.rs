use cosmwasm_std::{Addr, Storage};

use crate::{
    error::ContractError,
    msg::{BooleanTest, ContractHasRelationsQueryParams, IndexValue, NameValue},
    query::ReadonlyContext,
    state::storage::{ContractId, CONTRACT_ADDR_2_ID, IX_REL_CONTRACT_ADDR},
};

pub fn query_contract_has_relations(
    ctx: ReadonlyContext,
    msg: ContractHasRelationsQueryParams,
) -> Result<bool, ContractError> {
    let ReadonlyContext { deps, .. } = ctx;
    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(msg.contract_address.as_str())?)?;
    let address = deps.api.addr_validate(msg.address.as_str())?;
    match msg.test {
        BooleanTest::And => {
            for x in msg.relations.iter() {
                if !has_relation(deps.storage, contract_id, x, &address) {
                    return Ok(false);
                }
            }
            return Ok(true);
        },
        BooleanTest::Or => {
            for x in msg.relations.iter() {
                if has_relation(deps.storage, contract_id, x, &address) {
                    return Ok(false);
                }
            }
            return Ok(true);
        },
        BooleanTest::Xor => {
            let mut test_passes = false;
            for x in msg.relations.iter() {
                if has_relation(deps.storage, contract_id, x, &address) {
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

fn has_relation(
    store: &dyn Storage,
    contract_id: ContractId,
    name_val: &NameValue,
    addr: &Addr,
) -> bool {
    let tag_bytes_vec = IndexValue::String(name_val.name.to_owned()).to_bytes();
    let tag_bytes = tag_bytes_vec.as_slice();
    IX_REL_CONTRACT_ADDR.has(store, (contract_id, tag_bytes, addr.as_bytes()))
}
