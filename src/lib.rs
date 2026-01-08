//! # CW Factory - Advanced Contract Factory with Indexing
//!
//! This contract factory instantiates and manages CosmWasm contracts with sophisticated
//! querying capabilities through a multi-index storage system.
//!
//! ## Core Features
//!
//! - **Contract Lifecycle**: Create contracts from allowed code IDs with preset templates
//! - **Multi-Index Storage**: Query by code_id, creator, timestamps, admin, or custom indices
//! - **Weighted Tags**: Tag contracts with numeric weights for flexible categorization
//! - **Relationship Graphs**: Directed relationships between contracts with optional values
//! - **Batch Migrations**: Migrate multiple contracts with error recovery and retry logic
//! - **Cursor Pagination**: Efficient range queries across all indices
//!
//! ## Storage Architecture
//!
//! The factory uses a dual-map pattern for indexing:
//! - **Forward index**: `IX_* -> (value_bytes, contract_id) -> marker`
//! - **Reverse map**: `ID_2_* -> contract_id -> value_bytes`
//!
//! This enables both efficient range queries and updates without scanning.
//!
//! ## Usage Examples
//!
//! ### Creating a Contract
//!
//! ```ignore
//! use cw_factory::msg::{ExecuteMsg, CreateMsg};
//!
//! let create_msg = CreateMsg {
//!     code_id: Some(Uint64::from(123u64)),
//!     label: "my-contract-1".to_string(),
//!     name: Some("my-contract".to_string()),
//!     admin: None, // Factory becomes admin
//!     tags: Some(vec!["production".to_string()]),
//!     preset: None,
//!     instantiate_msg: json!({ "name": "My Contract" }).as_object().unwrap().clone(),
//! };
//! ```
//!
//! ### Updating Indexes from Child Contract
//!
//! ```ignore
//! // In your child contract
//! let update_msg = WasmMsg::Execute {
//!     contract_addr: factory_addr.to_string(),
//!     msg: to_binary(&FactoryExecuteMsg::Update(UpdateMsg {
//!         contract: None, // Self
//!         indices: Some(vec![
//!             IndexUpdate {
//!                 name: "total_supply".to_string(),
//!                 value: IndexValue::Uint128(Uint128::from(1000000u128)),
//!             }
//!         ]),
//!         tags: None,
//!         relations: None,
//!     }))?,
//!     funds: vec![],
//! };
//! ```
//!
//! ### Querying by Index Range
//!
//! ```ignore
//! let query = QueryMsg::Contracts(
//!     ContractSetQueryMsg::InRange(ContractsInRangeQueryParams {
//!         index: IndexSelector::CreatedBy,
//!         start: Some(IndexRangeBound::Inclusive(IndexValue::String(creator.to_string()))),
//!         stop: Some(IndexRangeBound::Inclusive(IndexValue::String(creator.to_string()))),
//!         limit: Some(100),
//!         desc: Some(true),
//!         cursor: None,
//!     })
//! );
//! ```

#[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
#[cfg(not(feature = "library"))]
pub mod execute;
pub mod math;
pub mod msg;
#[cfg(not(feature = "library"))]
pub mod query;
pub mod state;
pub mod util;
