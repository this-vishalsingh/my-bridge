use crate::{
    constants::MAX_GUARDIAN_COUNT,
    errors::BridgeHandlerError,
    states::{BridgeHandler, GuardianInfo},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateGuardianThreshold<'info> {
    manager: Signer<'info>,
    #[account(
        mut,
        has_one = manager @ BridgeHandlerError::Unauthorized,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        seeds = [b"guardian_info", bridge_handler.key().as_ref()],
        bump = guardian_info.bump
    )]
    guardian_info: Box<Account<'info, GuardianInfo>>,
}

impl UpdateGuardianThreshold<'_> {
    pub fn update_guardian_threshold(&mut self, guardian_threshold: u8) -> Result<()> {
        require!(
            guardian_threshold > 0 && guardian_threshold <= MAX_GUARDIAN_COUNT as u8,
            BridgeHandlerError::InvalidGuardianThreshold
        );

        let minimum_threshold = self.guardian_info.guardians.len() as u8 / 2 + 1;
        require!(
            guardian_threshold >= minimum_threshold,
            BridgeHandlerError::InvalidGuardianThreshold
        );

        require!(
            self.guardian_info.guardians.len() >= guardian_threshold as usize,
            BridgeHandlerError::GuardianThresholdNotMet
        );

        self.bridge_handler.guardian_threshold = guardian_threshold;
        Ok(())
    }
}
