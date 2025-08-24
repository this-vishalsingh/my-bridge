use anchor_lang::prelude::*;

use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
pub struct UpdateInstantBridgeCap<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
}

impl UpdateInstantBridgeCap<'_> {
    pub fn update_instant_bridge_cap(&mut self, instant_bridge_cap: u64) -> Result<()> {
        self.bridge_handler.instant_bridge_cap_remained_dollar = instant_bridge_cap;
        Ok(())
    }
}
