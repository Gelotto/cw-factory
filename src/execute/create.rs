use crate::{
    error::ContractError,
    math::{add_u32, add_u64},
    msg::CreateMsg,
    state::{
        models::SubMsgContext,
        storage::{
            CONFIG_ALLOWED_CODE_IDS, CONFIG_DEFAULT_CODE_ID, CONTRACT_ADDR_2_ID, CONTRACT_COUNTER, CONTRACT_ID_2_ADDR,
            CONTRACT_ID_COUNTER, IX_ADMIN, IX_CODE_ID, IX_CREATED_AT, IX_CREATED_BY, IX_UPDATED_AT, REPLY_ID_COUNTER,
            SUBMSG_CONTEXTS,
        },
    },
};
use cosmwasm_std::{attr, Addr, DepsMut, Env, Reply, Response, StdError, SubMsg, WasmMsg};
use cw_utils::{parse_reply_instantiate_data, MsgInstantiateContractResponse};

use super::Context;

/// Instantiate a new contract through the factory, adding it to its internal
/// data structures and indices via the SubMsg reply.
pub fn exec_create(
    ctx: Context,
    msg: CreateMsg,
) -> Result<Response, ContractError> {
    let Context { deps, env, info } = ctx;

    let reply_id = REPLY_ID_COUNTER
        .update(deps.storage, |n| -> Result<_, ContractError> { add_u64(n, 1u64) })?
        .u64()
        - 1;

    let code_id = if let Some(code_id) = msg.code_id {
        if CONFIG_ALLOWED_CODE_IDS.has(deps.storage, code_id.into()) {
            code_id
        } else {
            return Err(ContractError::NotAuthorized {
                reason: format!("not allowed to instantiate code ID: {}", code_id.u64()),
            });
        }
    } else {
        CONFIG_DEFAULT_CODE_ID
            .load(deps.storage)
            .map_err(|_| ContractError::ValidationError {
                reason: "no default code ID set in factory".to_owned(),
            })?
    }
    .u64();

    // NOTE: By default, the factory is the admin of the contracts instantiated
    // through it. This is in order to be able to exec admin functions via the
    // factory for things like batch migrations.
    let admin = if let Some(admin) = msg.admin {
        deps.api.addr_validate(admin.as_str())?
    } else {
        env.contract.address
    };

    // Generate contract ID
    let contract_id = CONTRACT_ID_COUNTER.update(deps.storage, |n| -> Result<_, ContractError> { add_u32(n, 1) })? - 1;

    // Save temp state for processing SubMsg reply
    SUBMSG_CONTEXTS.save(
        deps.storage,
        reply_id,
        &SubMsgContext {
            code_id: code_id.into(),
            created_by: info.sender.to_owned(),
            admin: admin.to_owned(),
            contract_id,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            attr("action", "create"),
            attr("code_id", code_id.to_string()),
            attr("label", msg.label.to_owned()),
            attr("admin", admin.to_owned()),
        ])
        .add_submessage(SubMsg::reply_on_success(
            WasmMsg::Instantiate {
                msg: msg.instantiate_msg,
                funds: info.funds.to_owned(),
                label: msg.label,
                admin: Some(admin.into()),
                code_id,
            },
            reply_id,
        )))
}

/// Extract and save created contract address and initialize indexes and other
/// contract-related state data structures.
pub fn exec_create_reply_handler(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let resp = Response::new();

    // Extract created contract address
    let MsgInstantiateContractResponse { contract_address, .. } = parse_reply_instantiate_data(reply.to_owned())
        .map_err(|e| {
            ContractError::Std(StdError::GenericErr {
                msg: format!(
                    "failed to extract newly created contract address from reply: {}",
                    e.to_string()
                ),
            })
        })?;

    let t = env.block.time.nanos();
    let contract_address = Addr::unchecked(contract_address);

    let SubMsgContext {
        contract_id,
        code_id,
        created_by,
        admin,
    } = SUBMSG_CONTEXTS.load(deps.storage, reply.id)?;

    CONTRACT_COUNTER.update(deps.storage, |n| -> Result<_, ContractError> { add_u32(n, 1) })?;
    CONTRACT_ADDR_2_ID.save(deps.storage, &contract_address, &contract_id)?;
    CONTRACT_ID_2_ADDR.save(deps.storage, contract_id, &contract_address)?;
    IX_CODE_ID.save(deps.storage, (&code_id.to_le_bytes(), contract_id), &0)?;
    IX_CREATED_AT.save(deps.storage, (&t.to_le_bytes(), contract_id), &0)?;
    IX_CREATED_BY.save(deps.storage, (created_by.as_bytes(), contract_id), &0)?;
    IX_UPDATED_AT.save(deps.storage, (created_by.as_bytes(), contract_id), &0)?;
    IX_ADMIN.save(deps.storage, (admin.as_bytes(), contract_id), &0)?;

    Ok(resp)
}
