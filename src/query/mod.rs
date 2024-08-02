pub mod config;
pub mod contract;
pub mod contracts;
pub mod migrations;
pub mod presets;

use cosmwasm_std::{Deps, Env};

pub struct ReadonlyContext<'a> {
    pub deps: Deps<'a>,
    pub env: Env,
}
