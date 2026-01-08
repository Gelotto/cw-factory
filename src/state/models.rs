//! Data models for factory configuration and state.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr,
    Uint64,
};
use serde_json::{
    Map as SerdeMap,
    Value,
};

use crate::msg::MigrationParams;

use super::storage::ContractId;

/// Factory configuration set during instantiation and updatable by manager.
#[cw_serde]

pub struct Config {
    /// Address with elevated permissions (migrations, presets, forced updates)
    pub managed_by: Addr,
    /// Code ID used when Create message doesn't specify one
    pub default_code_id: Option<Uint64,>,
    /// Whitelist of code IDs that can be instantiated through this factory
    pub allowed_code_ids: Vec<Uint64,>,
}

/// Temporary context saved between submessage execution and reply handling.
///
/// Since reply handlers only receive the reply_id, we store necessary context
/// here to complete the creation process (save to indices, emit events, etc.)
#[cw_serde]

pub struct SubMsgContext {
    /// Code ID of the contract being created
    pub code_id: Uint64,
    /// Internal contract ID assigned by factory
    pub contract_id: ContractId,
    /// Address that initiated contract creation
    pub created_by: Addr,
    /// Optional human-readable name for the contract
    pub name: Option<String,>,
    /// Admin address for the new contract (factory by default)
    pub admin: Addr,
}

/// Preset template for contract instantiation.
///
/// Presets contain JSON values that are merged with user-provided instantiate messages.
#[cw_serde]

pub struct Preset {
    /// JSON object with preset values (merged with user's instantiate_msg)
    pub values: SerdeMap<String, Value,>,
    /// If true, user values override preset; if false, preset values take precedence
    pub overridable: bool,
    /// Number of times this preset has been used (usage tracking)
    pub n_uses: u32,
}

/// Migration session status.
#[cw_serde]

pub enum MigrationStatus {
    /// Session is actively processing or ready for next step
    Running,
    /// All contracts migrated successfully (or completed with errors in Retry mode)
    Complete,
    /// Session was cancelled by manager before completion
    Aborted,
}

/// Batch migration session state.
///
/// Tracks progress of migrating multiple contracts with cursor-based batching.
#[cw_serde]

pub struct Migration {
    /// Migration parameters (from_code_id, to_code_id, migrate_msg, etc.)
    pub params: MigrationParams,
    /// Current status of the migration session
    pub status: MigrationStatus,
    /// Cursor for next batch of contracts to migrate (None if complete)
    pub cursor: Option<ContractId,>,
    /// Cursor for retrying failed migrations (used in Retry mode)
    pub retry_cursor: Option<ContractId,>,
    /// Number of successful migrations
    pub n_success: u32,
    /// Number of failed migrations
    pub n_error: u32,
}

/// Error details for failed migrations.
///
/// Stored in MIGRATION_ERRORS map for retry logic in Retry strategy.
#[cw_serde]

pub struct MigrationError {
    /// Address of contract that failed to migrate
    pub contract: Addr,
    /// Error message from failed migration attempt
    pub error: String,
    /// Reply ID of the failed migration submessage
    pub reply_id: Uint64,
}

/// Error handling strategy for batch migrations.
#[cw_serde]

pub enum MigrationErrorStrategy {
    /// Fail the entire transaction if any contract migration fails (strict mode)
    Abort,
    /// Track errors and allow retrying failed contracts later (resilient mode)
    Retry,
}
