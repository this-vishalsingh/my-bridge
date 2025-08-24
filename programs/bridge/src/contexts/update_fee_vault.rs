use anchor_lang::prelude::*;

use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
pub struct UpdateFeeVault<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        constraint = new_fee_vault.key() != bridge_handler.fee_vault
    )]
    /// CHECK: no other check needed
    new_fee_vault: AccountInfo<'info>,
}

impl UpdateFeeVault<'_> {
    pub fn update_fee_vault(&mut self) -> Result<()> {
        self.bridge_handler.fee_vault = self.new_fee_vault.key();
        Ok(())
    }
}
