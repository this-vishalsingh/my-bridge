use anchor_lang::prelude::*;

use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, GuardianInfo},
};

#[derive(Accounts)]
pub struct RemoveGuardian<'info> {
    manager: Signer<'info>,
    #[account(
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        mut,
        seeds = [b"guardian_info", bridge_handler.key().as_ref()],
        bump = guardian_info.bump
    )]
    guardian_info: Box<Account<'info, GuardianInfo>>,
    /// CHECK: no need to check
    guardian: AccountInfo<'info>,
}

impl RemoveGuardian<'_> {
    pub fn remove_guardian(&mut self) -> Result<()> {
        require!(
            self.guardian_info.guardians.contains(&self.guardian.key()),
            BridgeHandlerError::GuardianNotFound
        );
        self.guardian_info
            .guardians
            .retain(|&g| g != self.guardian.key());
        require!(
            self.guardian_info.guardians.len() >= self.bridge_handler.guardian_threshold as usize,
            BridgeHandlerError::GuardianThresholdNotMet
        );
        Ok(())
    }
}
