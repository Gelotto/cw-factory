//! Query contract metadata.
//!
//! ## Metadata Retrieved
//!
//! Returns all built-in contract metadata:
//! - `created_at`: Timestamp when contract was created through factory
//! - `updated_at`: Timestamp of last Update message
//! - `created_by`: Address that created the contract
//! - `admin`: Current admin address
//! - `code_id`: Contract's code ID
//! - `name`: Optional human-readable name (if specified during creation)
//!
//! ## Use Cases
//!
//! - Contract details page: Display all metadata for a contract
//! - Verification: Check contract's code_id and creation date
//! - Administration: Identify admin and creator addresses

use cosmwasm_std::{
    Addr,
    Timestamp,
};
use cw_storage_plus::KeyDeserialize;

use crate::{
    error::ContractError,
    msg::ContractMetadataResponse,
    query::ReadonlyContext,
    state::storage::{
        CONTRACT_ADDR_2_ID,
        CONTRACT_ID_2_NAME,
        ID_2_ADMIN,
        ID_2_CODE_ID,
        ID_2_CREATED_AT,
        ID_2_CREATED_BY,
        ID_2_UPDATED_AT,
    },
};

/// Query all metadata for a factory-managed contract.
///
/// Retrieves built-in fields from reverse lookup maps (ID_2_*).

pub fn query_contract_metadata(
    ctx: ReadonlyContext,
    contract: Addr,
) -> Result<ContractMetadataResponse, ContractError,> {

    let ReadonlyContext { deps, .. } = ctx;

    let addr = deps.api.addr_validate(contract.as_str(),)?;

    // Resolve contract ID from address
    let id = CONTRACT_ADDR_2_ID.load(deps.storage, &addr,)?;

    let created_at_nanos: u64 = u64::from_le_bytes(
        ID_2_CREATED_AT
            .load(deps.storage, id,)?
            .try_into()
            .expect("incorrect created_at byte length",),
    );

    let updated_at_nanos: u64 = u64::from_le_bytes(
        ID_2_UPDATED_AT
            .load(deps.storage, id,)?
            .try_into()
            .expect("incorrect upated_at byte length",),
    );

    let code_id = u64::from_le_bytes(
        ID_2_CODE_ID
            .load(deps.storage, id,)?
            .try_into()
            .expect("incorrect upated_at byte length",),
    );

    Ok(ContractMetadataResponse {
        created_at: Timestamp::from_nanos(created_at_nanos,),
        created_by: Addr::from_slice(ID_2_CREATED_BY.load(deps.storage, id,)?.as_slice(),)?,
        updated_at: Timestamp::from_nanos(updated_at_nanos,),
        name: CONTRACT_ID_2_NAME.may_load(deps.storage, id,)?,
        admin: Addr::from_slice(ID_2_ADMIN.load(deps.storage, id,)?.as_slice(),)?,
        code_id: code_id.into(),
    },)
}
