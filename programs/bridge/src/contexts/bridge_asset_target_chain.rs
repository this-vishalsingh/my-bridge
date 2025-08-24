use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

use solana_program::hash::hash;

use crate::constants::{
    get_whitelisted_token_price, is_token_whitelisted, MAX_ADDITIONAL_SOL_GAS,
    MAX_GUARDIAN_SIGNATURES, SOLANA_DOLLAR_CAP_PER_EPOCH,
    SOLANA_POST_INSTANT_CAP_AWAITING_TIME_SECONDS,
};
use crate::states::{BridgeProof, GuardianInfo, TokenInfo, VerifiedSignatures};
use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, Chain},
};

// The below precompile is used to mint SOL on Solana only
#[allow(dead_code)]
#[cfg(target_os = "solana")]
extern "C" {
    fn sol_mint_native_sol(amount: u64, account_idx: u64) -> u64;
}

#[derive(Accounts)]
#[instruction(msg_hash: [u8; 32], source_tx_id: [u8; 64])]
pub struct BridgeAssetTargetChain<'info> {
    #[account(mut)]
    operator: Signer<'info>,
    #[account(
        mut,
        mint::token_program = token_program
    )]
    mint: Box<InterfaceAccount<'info, Mint>>,
    /// CHECKED: checks will be performed agsint signature with hash
    #[account(mut)]
    recipient: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = operator,
        associated_token::authority = recipient,
        associated_token::mint = mint,
        associated_token::token_program = token_program
    )]
    recipient_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        has_one = operator @ BridgeHandlerError::InvalidOperator,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        mut,
        associated_token::authority = bridge_handler,
        associated_token::mint = mint,
        associated_token::token_program = token_program
    )]
    bridge_handler_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = operator,
        space = 8 + BridgeProof::INIT_SPACE,
        seeds = [b"bridge_proof", bridge_handler.key().as_ref(), hash(source_tx_id.as_ref()).to_bytes().as_ref()],
        bump
    )]
    bridge_proof: Box<Account<'info, BridgeProof>>,
    #[account(
        seeds = [b"guardian_info", bridge_handler.key().as_ref()],
        bump = guardian_info.bump
    )]
    guardian_info: Box<Account<'info, GuardianInfo>>,
    #[account(
        mut,
        close = operator,
        seeds = [b"verified_signatures", bridge_handler.key().as_ref(), msg_hash.as_ref()],
        bump = verified_signatures.bump
    )]
    verified_signatures: Box<Account<'info, VerifiedSignatures>>,
    #[account(
        seeds = [b"token_info", bridge_handler.key().as_ref(), mint.key().as_ref()],
        bump = token_info.bump
    )]
    token_info: Box<Account<'info, TokenInfo>>,
    token_program: Interface<'info, TokenInterface>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> BridgeAssetTargetChain<'info> {
    #[allow(clippy::too_many_arguments)]
    pub fn bridge_asset_target_chain(
        &mut self,
        bumps: BridgeAssetTargetChainBumps,
        msg_hash: [u8; 32],
        source_tx_id: [u8; 64],
        sender: Pubkey,
        source_mint: Pubkey,
        receive_amount: u64,
        nonce: u64,
        additional_sol_gas: u64,
    ) -> Result<()> {
        require!(!self.bridge_handler.pause, BridgeHandlerError::BridgePaused);

        let chain = self.bridge_handler.chain;

        // validate instant bridge cap
        if chain == Chain::Solana && is_token_whitelisted(self.mint.key()) {
            let dollar_value =
                get_whitelisted_token_price(self.mint.key(), receive_amount, self.mint.decimals)
                    .ok_or(BridgeHandlerError::TokenNotExists)?;

            if self.bridge_handler.instant_bridge_cap_epoch != Clock::get()?.epoch {
                self.bridge_handler.instant_bridge_cap_remained_dollar =
                    SOLANA_DOLLAR_CAP_PER_EPOCH;
                self.bridge_handler.instant_bridge_cap_epoch = Clock::get()?.epoch;
            }

            if self.bridge_handler.instant_bridge_cap_remained_dollar < dollar_value as u64 {
                let awaiting_time = (Clock::get()?.unix_timestamp as u64)
                    .checked_sub(self.verified_signatures.created_at)
                    .unwrap();
                if awaiting_time < SOLANA_POST_INSTANT_CAP_AWAITING_TIME_SECONDS {
                    return Err(BridgeHandlerError::InstantBridgeCapExceeded.into());
                }
            } else {
                self.bridge_handler.instant_bridge_cap_remained_dollar -= dollar_value as u64;
            }
        }

        require!(
            !self.verified_signatures.pubkey_index.is_empty()
                && self.verified_signatures.pubkey_index.len()
                    <= self.guardian_info.guardians.len()
                && self.verified_signatures.pubkey_index.len() <= MAX_GUARDIAN_SIGNATURES,
            BridgeHandlerError::InvalidSignerCount
        );

        // verify sigs meet threshold
        require!(
            self.verified_signatures.pubkey_index.len()
                >= self.bridge_handler.guardian_threshold as usize,
            BridgeHandlerError::GuardianThresholdNotMet
        );

        require!(
            additional_sol_gas <= MAX_ADDITIONAL_SOL_GAS,
            BridgeHandlerError::TooMuchAdditionalSolGas
        );

        let mut message_data = Vec::new();
        message_data.extend_from_slice(&sender.to_bytes());
        message_data.extend_from_slice(&self.recipient.key().to_bytes());
        message_data.extend_from_slice(&source_mint.to_bytes());
        message_data.extend_from_slice(&self.mint.key().to_bytes());
        message_data.extend_from_slice(&receive_amount.to_be_bytes());
        message_data.extend_from_slice(&nonce.to_be_bytes());
        message_data.extend_from_slice(&source_tx_id);
        message_data.extend_from_slice(&additional_sol_gas.to_be_bytes());

        let message: [u8; 32] = hash(message_data.as_ref()).to_bytes();
        require!(
            message == msg_hash,
            BridgeHandlerError::InvalidGuardianSignatureMessage
        );

        if (chain == Chain::Solana && self.token_info.is_solana_native_token)
            || (chain == Chain::Solayer && self.token_info.is_solayer_native_token)
        {
            require!(
                self.bridge_handler_vault.amount >= receive_amount,
                BridgeHandlerError::InsufficientFunds
            );
            msg!("transfer {} token to recipient", receive_amount);
            self.transfer_token(receive_amount, self.recipient_vault.to_account_info())?;
        } else if (chain == Chain::Solana && !self.token_info.is_solana_native_token)
            || (chain == Chain::Solayer && !self.token_info.is_solayer_native_token)
        {
            msg!("mint {} token to recipient", receive_amount);
            self.mint_token(receive_amount, self.recipient_vault.to_account_info())?;
        } else {
            require!(false, BridgeHandlerError::InvalidTokenInfo);
        }

        self.bridge_proof.bump = bumps.bridge_proof;
        self.bridge_proof.msg_hash = message;
        self.bridge_proof.tx_id = source_tx_id;
        self.bridge_proof.user_account = self.recipient.key();
        self.bridge_proof.created_at = Clock::get()?.unix_timestamp as u64;

        if additional_sol_gas > 0 {
            if chain == Chain::Solayer {
                self.mint_sol_to_recipient(additional_sol_gas)?;
            } else {
                self.transfer_sol_to_recipient(additional_sol_gas)?;
            }
        }

        Ok(())
    }

    fn transfer_token(&mut self, amount: u64, target_vault: AccountInfo<'info>) -> Result<()> {
        let bump = [self.bridge_handler.bump];
        let init_nonce_bytes = self.bridge_handler.init_nonce.to_be_bytes();
        let signer_seeds: [&[&[u8]]; 1] =
            [&[b"bridge_handler", init_nonce_bytes.as_ref(), &bump][..]];

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.bridge_handler_vault.to_account_info(),
                to: target_vault,
                mint: self.mint.to_account_info(),
                authority: self.bridge_handler.to_account_info(),
            },
            &signer_seeds[..],
        );

        transfer_checked(ctx, amount, self.mint.decimals)
    }

    fn mint_token(&mut self, amount: u64, target_vault: AccountInfo<'info>) -> Result<()> {
        let bump = [self.bridge_handler.bump];
        let init_nonce_bytes = self.bridge_handler.init_nonce.to_be_bytes();
        let signer_seeds: [&[&[u8]]; 1] =
            [&[b"bridge_handler", init_nonce_bytes.as_ref(), &bump][..]];

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.mint.to_account_info(),
                to: target_vault,
                authority: self.bridge_handler.to_account_info(),
            },
            &signer_seeds[..],
        );

        mint_to(ctx, amount)
    }

    #[cfg(feature = "solayer")]
    fn mint_sol_to_recipient(&mut self, lamports: u64) -> Result<()> {
        #[cfg(target_os = "solana")]
        let result = unsafe { sol_mint_native_sol(lamports, 2) };
        #[cfg(not(target_os = "solana"))]
        let result = 0;

        match result {
            0 => {
                msg!(
                    "SUCCESS: Minted {} lamports to {}",
                    lamports,
                    self.recipient.key()
                );
                self.recipient.add_lamports(lamports)?;
            }
            _ => {
                msg!(
                    "ERROR: Failed to mint {} lamports to {}",
                    lamports,
                    self.recipient.key()
                );
                return Err(BridgeHandlerError::FailToMintSol.into());
            }
        }

        Ok(())
    }
    #[cfg(not(feature = "solayer"))]
    fn mint_sol_to_recipient(&mut self, _lamports: u64) -> Result<()> {
        require!(false, BridgeHandlerError::InvalidOSForMintingSol);
        Ok(())
    }

    fn transfer_sol_to_recipient(&mut self, lamports: u64) -> Result<()> {
        require!(
            Rent::get()?.minimum_balance(self.bridge_handler.to_account_info().data_len())
                + lamports
                <= self.bridge_handler.to_account_info().lamports(),
            BridgeHandlerError::InsufficientFunds
        );

        **self
            .bridge_handler
            .to_account_info()
            .try_borrow_mut_lamports()? -= lamports;
        **self.recipient.try_borrow_mut_lamports()? += lamports;

        Ok(())
    }
}
