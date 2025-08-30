use anchor_lang::prelude::*;

use crate::{
    constants::SOLANA_DOLLAR_CAP_PER_EPOCH,
    states::{BridgeHandler, Chain, FixedFeeInfo, GuardianInfo},
};

#[derive(Accounts)]
#[instruction(init_nonce: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + BridgeHandler::INIT_SPACE,
        seeds = [b"bridge_handler", init_nonce.to_be_bytes().as_ref()],
        bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        init,
        payer = signer,
        space = 8 + GuardianInfo::INIT_SPACE,
        seeds = [b"guardian_info", bridge_handler.key().as_ref()],
        bump
    )]
    guardian_info: Box<Account<'info, GuardianInfo>>,
    /// CHECK: no check needed
    fee_vault: AccountInfo<'info>,
    /// CHECK: no check needed
    manager: AccountInfo<'info>,
    /// CHECK: no check needed
    operator: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

impl Initialize<'_> {
    pub fn initialize(
        &mut self,
        bumps: InitializeBumps,
        init_nonce: u64,
        chain: Chain,
    ) -> Result<()> {
        self.bridge_handler.bump = bumps.bridge_handler;
        self.bridge_handler.init_nonce = init_nonce;
        self.bridge_handler.pause = false;
        self.bridge_handler.nonce = 0;
        self.bridge_handler.chain = chain;
        self.bridge_handler.fee_vault = self.fee_vault.key();
        self.bridge_handler.manager = self.manager.key();
        self.bridge_handler.operator = self.operator.key();
        self.bridge_handler.guardian_info = self.guardian_info.key();
        self.bridge_handler.guardian_threshold = u8::MAX;
        self.bridge_handler.instant_bridge_cap_remained_dollar = SOLANA_DOLLAR_CAP_PER_EPOCH;
        self.bridge_handler.instant_bridge_cap_epoch = Clock::get()?.epoch;
        self.bridge_handler.fee_info = FixedFeeInfo::default();

        self.guardian_info.bump = bumps.guardian_info;
        self.guardian_info.guardians = vec![];
        Ok(())
    }
}
