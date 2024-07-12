pub mod create;
pub mod set_config;
pub mod update;

use cosmwasm_std::{DepsMut, Env, MessageInfo};

pub struct Context<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}
