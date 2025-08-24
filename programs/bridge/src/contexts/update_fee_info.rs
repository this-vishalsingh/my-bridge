use anchor_lang::prelude::*;

use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, FixedFeeInfo},
};

#[derive(Accounts)]
pub struct UpdateFeeInfo<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
}

impl UpdateFeeInfo<'_> {
    pub fn update_fee_info(&mut self, fee_info: FixedFeeInfo) -> Result<()> {
        self.bridge_handler.fee_info = fee_info;
        Ok(())
    }
}
