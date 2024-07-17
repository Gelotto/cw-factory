pub mod create;
pub mod hide;
pub mod migrate;
pub mod set_config;
pub mod set_preset;
pub mod update;

use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub struct Context<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}
