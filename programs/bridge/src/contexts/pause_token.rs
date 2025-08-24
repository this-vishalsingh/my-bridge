use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, TokenInfo},
};

#[derive(Accounts)]
pub struct PauseToken<'info> {
    manager: Signer<'info>,
    #[account(
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        mint::token_program = token_program
    )]
    mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        seeds = [b"token_info", bridge_handler.key().as_ref(), mint.key().as_ref()],
        bump = token_info.bump
    )]
    token_info: Box<Account<'info, TokenInfo>>,
    token_program: Interface<'info, TokenInterface>,
}

impl PauseToken<'_> {
    pub fn pause_token(&mut self) -> Result<()> {
        self.token_info.pause = true;
        Ok(())
    }

    pub fn unpause_token(&mut self) -> Result<()> {
        self.token_info.pause = false;
        Ok(())
    }
}
