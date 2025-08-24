use anchor_lang::prelude::*;

use crate::{errors::BridgeHandlerError, states::BridgeHandler};

#[derive(Accounts)]
#[instruction(init_nonce: u64)]
pub struct UpdateOperator<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    /// CHECK: no check needed
    new_operator: AccountInfo<'info>,
}

impl UpdateOperator<'_> {
    pub fn update_operator(&mut self) -> Result<()> {
        self.bridge_handler.operator = self.new_operator.key();
        Ok(())
    }
}
