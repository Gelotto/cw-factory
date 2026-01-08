//! Boolean tag testing with weight constraints.
//!
//! ## Boolean Test Modes
//!
//! Three boolean evaluation strategies for testing multiple tags:
//!
//! - **And**: Returns true if contract has ALL specified tags (with weight constraints)
//!   - Short-circuits on first missing tag
//!   - Use case: "Contract must have 'verified' AND 'premium' AND 'active'"
//!
//! - **Or**: Returns true if contract has ANY of the specified tags
//!   - Short-circuits on first match
//!   - Use case: "Contract has 'beta' OR 'alpha' OR 'staging'"
//!
//! - **Xor**: Returns true if contract has EXACTLY ONE of the specified tags
//!   - Short-circuits when second match is found (immediate failure)
//!   - Use case: "Contract is in exactly one deployment stage"
//!
//! ## Weight Constraints
//!
//! Each tag selector can optionally specify weight bounds:
//! - `min_weight`: Tag weight must be >= this value
//! - `max_weight`: Tag weight must be <= this value
//! - Both: Tag weight must be in range [min, max]
//!
//! If a tag exists but fails weight constraints, it's treated as not present.
//!
//! ## Examples
//!
//! - `And` with ["premium", "active"]: Contract must have both tags
//! - `And` with [("premium", min_weight: 80), "active"]: Contract must have "premium" with weight >= 80 AND "active"
//! - `Or` with ["beta", "alpha"]: Contract has at least one of these tags
//! - `Xor` with ["staging", "prod", "dev"]: Contract is in exactly one environment

use cosmwasm_std::Storage;

use crate::{
    error::ContractError,
    msg::{
        BooleanTest,
        ContractHasTagsQueryParams,
        IndexValue,
        TagSelector,
    },
    query::ReadonlyContext,
    state::storage::{
        ContractId,
        CONTRACT_ADDR_2_ID,
        CONTRACT_TAG_WEIGHTS,
    },
};

/// Test whether a contract has tags matching the specified boolean criteria.
///
/// ## Boolean Modes
///
/// - **And**: True if contract has ALL tags (short-circuits on first failure)
/// - **Or**: True if contract has ANY tag (short-circuits on first match)
/// - **Xor**: True if contract has EXACTLY ONE tag (short-circuits when second match found)
///
/// ## Weight Constraints
///
/// Each TagSelector can optionally specify min_weight/max_weight.
/// Tags failing weight constraints are treated as not present.

pub fn query_contract_has_tags(
    ctx: ReadonlyContext,
    msg: ContractHasTagsQueryParams,
) -> Result<bool, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    let contract_id = CONTRACT_ADDR_2_ID.load(deps.storage, &deps.api.addr_validate(msg.address.as_str(),)?,)?;

    match msg.test {
        BooleanTest::And => {

            // AND: All tags must be present (short-circuit on first failure)
            for selector in msg.tags.iter() {

                if !has_tag(deps.storage, contract_id, selector,)? {

                    return Ok(false,);
                }
            }

            return Ok(true,);
        },
        BooleanTest::Or => {

            // OR: At least one tag must be present (short-circuit on first match)
            for selector in msg.tags.iter() {

                if has_tag(deps.storage, contract_id, selector,)? {

                    return Ok(true,);
                }
            }

            return Ok(false,);
        },
        BooleanTest::Xor => {

            // XOR: Exactly one tag must be present (short-circuit when second match found)
            let mut test_passes = false;

            for selector in msg.tags.iter() {

                if has_tag(deps.storage, contract_id, selector,)? {

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

/// Check if a contract has a specific tag with optional weight constraints.
///
/// ## Logic
///
/// 1. Look up tag in CONTRACT_TAG_WEIGHTS index
/// 2. If tag not found: return false
/// 3. If tag found: validate weight against min_weight/max_weight constraints
/// 4. If weight constraints fail: return false (tag treated as not present)
/// 5. If all constraints pass: return true

fn has_tag(
    store: &dyn Storage,
    contract_id: ContractId,
    selector: &TagSelector,
) -> Result<bool, ContractError,> {

    // Convert tag string to padded bytes (128 bytes for lexicographic ordering)
    let tag_bytes_vec = IndexValue::String(selector.tag.to_owned(),).to_bytes();

    let tag_bytes = tag_bytes_vec.as_slice();

    // Look up tag weight from CONTRACT_TAG_WEIGHTS index
    if let Some(weight,) = CONTRACT_TAG_WEIGHTS.may_load(store, (contract_id, tag_bytes,),)? {

        // Tag exists: check weight constraints if specified
        if let Some(min_weight,) = selector.min_weight {

            if weight < min_weight {

                // Weight too low: treat as tag not present
                return Ok(false,);
            }
        }

        if let Some(max_weight,) = selector.max_weight {

            if weight > max_weight {

                // Weight too high: treat as tag not present
                return Ok(false,);
            }
        }

        // Tag exists and weight constraints (if any) are satisfied
        return Ok(true,);
    }

    // Tag does not exist
    Ok(false,)
}
