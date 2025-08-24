use anchor_lang::prelude::*;

use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
pub struct PauseBridge<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
}

impl PauseBridge<'_> {
    pub fn pause_bridge(&mut self) -> Result<()> {
        self.bridge_handler.pause = true;
        Ok(())
    }

    pub fn unpause_bridge(&mut self) -> Result<()> {
        self.bridge_handler.pause = false;
        Ok(())
    }
}
