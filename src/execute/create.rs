//! Contract creation with preset application and automatic indexing.
//!
//! ## Creation Flow
//!
//! 1. **Validation**: Verify code_id against whitelist (or use default)
//! 2. **Context Creation**: Save temporary context with contract metadata
//! 3. **SubMsg Emission**: Emit WasmMsg::Instantiate with reply_on_success
//! 4. **Reply Handling**: Extract contract address, initialize all indices
//!
//! ## SubMsg Reply Pattern
//!
//! Contract creation is asynchronous - we don't know the new contract's address
//! until after instantiation succeeds. The reply handler:
//! - Loads saved context from SUBMSG_CONTEXTS
//! - Extracts the created contract address from reply data
//! - Initializes bidirectional ID/Address mappings
//! - Populates all built-in indices (code_id, created_at, created_by, admin)
//!
//! ## Default Admin Strategy
//!
//! The factory sets itself as admin by default (overridable via CreateMsg).
//! This enables batch migrations via the factory's migration endpoints,
//! allowing managed upgrades of all created contracts.

use crate::{
    error::ContractError,
    math::{
        add_u32,
        add_u64,
    },
    msg::{
        CreateMsg,
        IndexValue,
    },
    state::{
        models::SubMsgContext,
        storage::{
            CONFIG_ALLOWED_CODE_IDS,
            CONFIG_DEFAULT_CODE_ID,
            CONTRACT_ADDR_2_ID,
            CONTRACT_COUNTER,
            CONTRACT_ID_2_ADDR,
            CONTRACT_ID_2_NAME,
            CONTRACT_ID_COUNTER,
            CONTRACT_NAME_2_ID,
            ID_2_CODE_ID,
            IX_ADMIN,
            IX_CODE_ID,
            IX_CREATED_AT,
            IX_CREATED_BY,
            IX_UPDATED_AT,
            REPLY_ID_COUNTER,
            SUBMSG_CONTEXTS,
        },
    },
    util::apply_preset,
};
use cosmwasm_std::{
    attr,
    Addr,
    DepsMut,
    Env,
    Event,
    Reply,
    Response,
    StdError,
    SubMsg,
    WasmMsg,
};
use cw_utils::{
    parse_reply_instantiate_data,
    MsgInstantiateContractResponse,
};

use super::Context;

/// Instantiate a new contract through the factory.
///
/// ## Process
///
/// 1. Validates code_id against whitelist (or uses factory default)
/// 2. Generates unique contract_id and reply_id
/// 3. Saves creation context to SUBMSG_CONTEXTS for reply handler
/// 4. Applies preset template if specified (merges with user's instantiate_msg)
/// 5. Emits WasmMsg::Instantiate submessage
///
/// ## Admin Assignment
///
/// By default, the factory becomes admin of created contracts. This enables:
/// - Batch migrations via factory's migration endpoints
/// - Centralized contract management
/// - Override by passing explicit admin in CreateMsg

pub fn exec_create(
    ctx: Context,
    msg: CreateMsg,
) -> Result<Response, ContractError,> {

    let Context { deps, env, info, } = ctx;

    // Generate unique reply ID for routing the instantiation response
    let reply_id = REPLY_ID_COUNTER
        .update(deps.storage, |n| -> Result<_, ContractError,> {

            add_u64(n, 1u64,)
        },)?
        .u64()
        - 1;

    // Validate code_id: either use provided code_id (if whitelisted) or factory default
    let code_id = if let Some(code_id,) = msg.code_id {

        // Check if the provided code_id is in the allowed list
        if CONFIG_ALLOWED_CODE_IDS.has(deps.storage, code_id.into(),) {

            code_id
        } else {

            return Err(ContractError::NotAuthorized {
                reason: format!("not allowed to instantiate code ID: {}", code_id.u64()),
            },);
        }
    } else {

        // No code_id provided: use factory's default (fails if not set)
        CONFIG_DEFAULT_CODE_ID
            .load(deps.storage,)
            .map_err(|_| ContractError::ValidationError {
                reason: "no default code ID set in factory".to_owned(),
            },)?
    }
    .u64();

    // Determine admin: factory by default (enables batch migrations), or custom admin if specified
    // This is critical for allowing the factory to manage contract lifecycles
    let admin = if let Some(admin,) = msg.admin {

        deps.api.addr_validate(admin.as_str(),)?
    } else {

        env.contract.address
    };

    // Generate sequential contract ID (internal tracking separate from address)
    let contract_id = CONTRACT_ID_COUNTER.update(deps.storage, |n| -> Result<_, ContractError,> {

        add_u32(n, 1,)
    },)? - 1;

    // Save context for reply handler (reply handlers only receive reply_id, so we store metadata here)
    SUBMSG_CONTEXTS.save(
        deps.storage,
        reply_id,
        &SubMsgContext {
            code_id: code_id.into(),
            created_by: info.sender.to_owned(),
            admin: admin.to_owned(),
            name: msg.name,
            contract_id,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "create",)],)
        .add_submessage(SubMsg::reply_on_success(
            WasmMsg::Instantiate {
                // Apply preset template if specified (merges preset values with user's instantiate_msg)
                msg: apply_preset(deps.storage, msg.instantiate_msg, msg.preset,)?,
                funds: info.funds.to_owned(),
                label: msg.label,
                admin: Some(admin.into(),),
                code_id,
            },
            reply_id,
        ),),)
}

/// Handle contract instantiation reply and initialize all indices.
///
/// ## Process
///
/// 1. **Extract Address**: Parse created contract address from reply data
/// 2. **Load Context**: Retrieve saved metadata from SUBMSG_CONTEXTS
/// 3. **Initialize Mappings**: Create bidirectional ID ↔ Address lookups
/// 4. **Initialize Indices**: Populate all built-in forward indices and reverse maps
/// 5. **Cleanup**: Remove temporary context from storage
///
/// ## Index Initialization
///
/// All built-in indices are populated with initial values:
/// - `IX_CODE_ID` / `ID_2_CODE_ID`: Contract's code_id
/// - `IX_CREATED_AT` / `IX_UPDATED_AT`: Creation timestamp (both set initially)
/// - `IX_CREATED_BY`: Creator's address
/// - `IX_ADMIN`: Admin address
///
/// Optional name mapping is created if name was provided in CreateMsg.

pub fn handle_creation_reply(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError,> {

    let resp = Response::new();

    // Extract the newly created contract's address from the reply data
    let MsgInstantiateContractResponse { contract_address, .. } = parse_reply_instantiate_data(reply.to_owned(),)
        .map_err(|e| {

            ContractError::Std(StdError::GenericErr {
                msg: format!(
                    "failed to extract newly created contract address from reply: {}",
                    e.to_string()
                ),
            },)
        },)?;

    let t = env.block.time.nanos();

    let contract_address = Addr::unchecked(contract_address,);

    // Load saved context containing contract metadata
    let SubMsgContext {
        contract_id,
        code_id,
        created_by,
        admin,
        name,
    } = SUBMSG_CONTEXTS.load(deps.storage, reply.id,)?;

    // Clean up temporary context storage
    SUBMSG_CONTEXTS.remove(deps.storage, reply.id,);

    // Increment global contract counter
    CONTRACT_COUNTER.update(deps.storage, |n| -> Result<_, ContractError,> {

        add_u32(n, 1,)
    },)?;

    // Create bidirectional ID ↔ Address mappings
    CONTRACT_ADDR_2_ID.save(deps.storage, &contract_address, &contract_id,)?;

    CONTRACT_ID_2_ADDR.save(deps.storage, contract_id, &contract_address,)?;

    // Convert metadata to bytes for index storage
    let code_id_bytes = IndexValue::Uint64(code_id,).to_bytes();

    let created_at_bytes = IndexValue::Uint64(t.into(),).to_bytes();

    let created_by_bytes = IndexValue::String(created_by.into(),).to_bytes();

    let admin_bytes = IndexValue::String(admin.clone().into(),).to_bytes();

    // Initialize reverse map for code_id (required for updates)
    ID_2_CODE_ID.save(deps.storage, contract_id, &code_id_bytes,)?;

    // Create optional name mapping if provided
    if let Some(contract_name,) = &name {

        CONTRACT_NAME_2_ID.save(deps.storage, contract_name, &contract_id,)?;

        CONTRACT_ID_2_NAME.save(deps.storage, contract_id, contract_name,)?;
    }

    // Initialize all built-in forward indices
    // Each uses composite key (value_bytes, contract_id) for uniqueness and range queries
    IX_CODE_ID.save(deps.storage, (&code_id_bytes, contract_id,), &0,)?;

    IX_CREATED_BY.save(deps.storage, (&created_by_bytes, contract_id,), &0,)?;

    IX_CREATED_AT.save(deps.storage, (&created_at_bytes, contract_id,), &0,)?;

    // Initialize updated_at to creation time (will be updated on first Update message)
    IX_UPDATED_AT.save(deps.storage, (&created_at_bytes, contract_id,), &0,)?;

    IX_ADMIN.save(deps.storage, (&admin_bytes, contract_id,), &0,)?;

    Ok(resp.add_event(Event::new("factory-create",).add_attributes(vec![
        attr("contract_address", contract_address.to_string(),),
        attr("code_id", code_id.to_string(),),
        attr("admin", admin.to_owned(),),
    ],),),)
}
