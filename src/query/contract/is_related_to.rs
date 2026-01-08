//! Boolean relationship testing.
//!
//! ## Boolean Test Modes
//!
//! Similar to has_tags.rs but for relationships:
//!
//! - **And**: Contract has ALL specified relationships to the target address
//! - **Or**: Contract has ANY of the specified relationships to the target address
//! - **Xor**: Contract has EXACTLY ONE of the specified relationships to the target address
//!
//! ## NameValue Matching
//!
//! Each relationship is specified as a NameValue:
//! - Name only: Match relationship by name (any value)
//! - Name + Value: Match exact name-value combination
//!
//! ## Use Cases
//!
//! - Permission checks: "Does contract have 'admin' OR 'owner' relationship to this address?"
//! - Dependency validation: "Contract has BOTH 'depends_on' and 'integrates_with' relationships"
//! - Exclusivity checks: "Contract has EXACTLY ONE deployment relationship (staging XOR prod)"

use cosmwasm_std::{
    Addr,
    Storage,
};

use crate::{
    error::ContractError,
    msg::{
        BooleanTest,
        ContractHasRelationsQueryParams,
        IndexValue,
        NameValue,
    },
    query::ReadonlyContext,
    state::storage::{
        ContractId,
        CONTRACT_ADDR_2_ID,
        IX_REL_CONTRACT_ADDR,
    },
};

/// Test whether a contract has relationships to an address matching boolean criteria.
///
/// ## Boolean Modes
///
/// - **And**: True if contract has ALL relationships (short-circuits on first failure)
/// - **Or**: True if contract has ANY relationship (short-circuits on first match)
/// - **Xor**: True if contract has EXACTLY ONE relationship (short-circuits when second match found)

pub fn query_contract_is_related_to(
    ctx: ReadonlyContext,
    msg: ContractHasRelationsQueryParams,
) -> Result<bool, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    let contract_id =
        CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(msg.contract_address.as_str(),)?,)?;

    let address = deps.api.addr_validate(msg.address.as_str(),)?;

    match msg.test {
        BooleanTest::And => {

            // AND: All relationships must exist (short-circuit on first failure)
            for x in msg.relations.iter() {

                if !has_relation(deps.storage, contract_id, x, &address,) {

                    return Ok(false,);
                }
            }

            return Ok(true,);
        },
        BooleanTest::Or => {

            // OR: At least one relationship must exist (short-circuit on first match)
            for x in msg.relations.iter() {

                if has_relation(deps.storage, contract_id, x, &address,) {

                    return Ok(true,);
                }
            }

            return Ok(false,);
        },
        BooleanTest::Xor => {

            // XOR: Exactly one relationship must exist (short-circuit when second match found)
            let mut test_passes = false;

            for x in msg.relations.iter() {

                if has_relation(deps.storage, contract_id, x, &address,) {

                    if !test_passes {

                        // First match: continue checking
                        test_passes = true;
                    } else {

                        // Second match: XOR fails
                        return Ok(false,);
                    }
                }
            }

            return Ok(test_passes,);
        },
    }
}

/// Check if a specific relationship exists from contract to address.
///
/// Builds edge bytes from NameValue (name || value) and checks existence
/// in the forward relationship index.

fn has_relation(
    store: &dyn Storage,
    contract_id: ContractId,
    name_val: &NameValue,
    addr: &Addr,
) -> bool {

    // Build edge bytes: name_bytes || value_bytes (value is optional)
    let mut edge = IndexValue::String(name_val.name.to_owned(),).to_bytes();

    if let Some(value,) = &name_val.value {

        edge.extend(value.to_bytes(),);
    }

    // Check existence in IX_REL_CONTRACT_ADDR: (contract_id, edge, addr) -> value
    IX_REL_CONTRACT_ADDR.has(store, (contract_id, edge.as_slice(), addr.as_bytes(),),)
}
