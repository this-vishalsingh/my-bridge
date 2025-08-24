use anchor_lang::prelude::*;

use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
#[instruction(init_nonce: u64)]
pub struct UpdateManager<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    /// CHECK: no check needed
    new_manager: AccountInfo<'info>,
}

impl UpdateManager<'_> {
    pub fn update_manager(&mut self) -> Result<()> {
        self.bridge_handler.manager = self.new_manager.key();
        Ok(())
    }
}
