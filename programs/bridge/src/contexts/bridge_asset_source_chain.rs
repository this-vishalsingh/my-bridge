use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{burn, transfer_checked, Burn, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use solana_program::hash::hash;

use crate::constants::{MAX_ADDITIONAL_SOL_GAS, METADATA_CREATION_FEE, MIN_SOL_BRIDGE_AMOUNT};
use crate::states::BridgeProofSourceChain;
use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, Chain, TokenInfo},
};

// The below precompile is used to burn SOL on Solayer only
#[allow(dead_code)]
#[cfg(target_os = "solana")]
extern "C" {
    fn sol_burn_native_sol(amount: u64, account_idx: u64) -> u64;
}

#[derive(Accounts)]
#[instruction(bridge_proof_nonce: u64)]
pub struct BridgeAssetSourceChain<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    #[account(
        mut,
        mint::token_program = token_program
    )]
    mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        token::authority = signer,
        token::mint = mint,
        token::token_program = token_program,
    )]
    signer_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        has_one = fee_vault,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(mut)]
    /// CHECK: Instead of checking in macro, we do manual check to reduce stack size
    bridge_handler_vault: AccountInfo<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + BridgeProofSourceChain::INIT_SPACE,
        seeds = [b"bridge_proof", bridge_handler.key().as_ref(), signer.key().as_ref(), bridge_proof_nonce.to_be_bytes().as_ref()],
        bump
    )]
    bridge_proof: Box<Account<'info, BridgeProofSourceChain>>,
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + TokenInfo::INIT_SPACE,
        seeds = [b"token_info", bridge_handler.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    token_info: Box<Account<'info, TokenInfo>>,
    #[account(mut)]
    /// CHECK: no check needed other than address check
    fee_vault: AccountInfo<'info>,
    token_program: Interface<'info, TokenInterface>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

impl<'info> BridgeAssetSourceChain<'info> {
    pub fn bridge_asset_source_chain(
        &mut self,
        token_info_bump: u8,
        amount: u64,
        recipient: Pubkey,
        target_mint: Pubkey,
        additional_sol_gas: u64,
    ) -> Result<u64> {
        self.init_if_needed_and_check_bridge_handler_vault()?;

        require!(!self.bridge_handler.pause, BridgeHandlerError::BridgePaused);
        require!(
            self.signer_vault.amount >= amount,
            BridgeHandlerError::InsufficientAmount,
        );

        self.transfer_sol_to_fee_vault(self.bridge_handler.fee_info.bridge_asset_fee)?;

        require!(
            additional_sol_gas <= MAX_ADDITIONAL_SOL_GAS,
            BridgeHandlerError::TooMuchAdditionalSolGas
        );

        msg!("additional_sol_gas: {:?}", additional_sol_gas);
        if additional_sol_gas > 0 {
            require!(
                additional_sol_gas >= MIN_SOL_BRIDGE_AMOUNT,
                BridgeHandlerError::TooLittleAdditionalSolGas
            );

            if self.bridge_handler.chain == Chain::Solayer {
                self.burn_sol(additional_sol_gas)?;
            } else {
                self.transfer_sol_to_bridge_handler(additional_sol_gas)?;
            }
        }

        let chain = self.bridge_handler.chain;

        if !self.token_info.is_solana_native_token && !self.token_info.is_solayer_native_token {
            // token info not exists before, then it is a native token on current chain
            // token info not exists, then it is a native token on current chain
            self.transfer_token(amount, self.bridge_handler_vault.to_account_info())?;

            if chain == Chain::Solayer {
                self.transfer_sol_to_fee_vault(METADATA_CREATION_FEE)?;
            }

            self.token_info.bump = token_info_bump;

            if chain == Chain::Solana {
                self.token_info.solana_mint = self.mint.key();
                self.token_info.solayer_mint = target_mint;
                self.token_info.is_solana_native_token = true;
                self.token_info.is_solayer_native_token = false;
            } else {
                self.token_info.solana_mint = target_mint;
                self.token_info.solayer_mint = self.mint.key();
                self.token_info.is_solana_native_token = false;
                self.token_info.is_solayer_native_token = true;
            }
            self.token_info.pause = false;
        } else {
            // token info already exists
            require!(!self.token_info.pause, BridgeHandlerError::TokenPaused);
            if (chain == Chain::Solana && self.token_info.is_solana_native_token)
                || (chain == Chain::Solayer && self.token_info.is_solayer_native_token)
            {
                self.transfer_token(amount, self.bridge_handler_vault.to_account_info())?;
            } else if (chain == Chain::Solana && !self.token_info.is_solana_native_token)
                || (chain == Chain::Solayer && !self.token_info.is_solayer_native_token)
            {
                self.burn_token(amount)?;
            }
        }

        msg!(
            "bridging {:?} token of {:?} to {:?}",
            amount,
            self.mint.key(),
            recipient
        );

        let nonce = self.bridge_handler.nonce;
        msg!("nonce: {:?}", nonce);
        self.bridge_handler.nonce = nonce.checked_add(1).unwrap();
        Ok(nonce)
    }

    pub fn issue_bridge_proof(
        &mut self,
        bridge_proof_bump: u8,
        bridge_proof_nonce: u64,
        amount: u64,
        recipient: Pubkey,
    ) -> Result<()> {
        self.bridge_proof.bump = bridge_proof_bump;
        let mut message_data = Vec::new();
        message_data.extend_from_slice(&self.signer.key().to_bytes());
        message_data.extend_from_slice(&recipient.to_bytes());
        message_data.extend_from_slice(&self.mint.key().to_bytes());
        message_data.extend_from_slice(&amount.to_be_bytes());
        message_data.extend_from_slice(&bridge_proof_nonce.to_be_bytes());
        self.bridge_proof.msg_hash = hash(message_data.as_ref()).to_bytes();
        self.bridge_proof.user_account = self.signer.key();
        self.bridge_proof.created_at = Clock::get()?.unix_timestamp as u64;
        Ok(())
    }

    fn burn_token(&mut self, amount: u64) -> Result<()> {
        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.mint.to_account_info(),
                from: self.signer_vault.to_account_info(),
                authority: self.signer.to_account_info(),
            },
        );

        burn(ctx, amount)
    }

    fn transfer_token(&mut self, amount: u64, target_vault: AccountInfo<'info>) -> Result<()> {
        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.signer_vault.to_account_info(),
                to: target_vault,
                mint: self.mint.to_account_info(),
                authority: self.signer.to_account_info(),
            },
        );

        transfer_checked(ctx, amount, self.mint.decimals)
    }

    fn transfer_sol_to_fee_vault(&mut self, lamports: u64) -> Result<()> {
        let ctx = CpiContext::new(
            self.system_program.to_account_info(),
            Transfer {
                from: self.signer.to_account_info(),
                to: self.fee_vault.to_account_info(),
            },
        );

        transfer(ctx, lamports)
    }

    fn transfer_sol_to_bridge_handler(&mut self, lamports: u64) -> Result<()> {
        let ctx = CpiContext::new(
            self.system_program.to_account_info(),
            Transfer {
                from: self.signer.to_account_info(),
                to: self.bridge_handler.to_account_info(),
            },
        );

        transfer(ctx, lamports)
    }

    #[cfg(not(feature = "solayer"))]
    fn burn_sol(&mut self, _lamports: u64) -> Result<()> {
        require!(false, BridgeHandlerError::InvalidOSForBurningSol);
        Ok(())
    }

    #[cfg(feature = "solayer")]
    fn burn_sol(&mut self, lamports: u64) -> Result<()> {
        #[cfg(target_os = "solana")]
        let result = unsafe { sol_burn_native_sol(lamports, 0) };
        #[cfg(not(target_os = "solana"))]
        let result = 0;

        match result {
            0 => {
                msg!(
                    "SUCCESS: Burned {} lamports from {}",
                    lamports,
                    self.signer.key()
                );
                self.signer.sub_lamports(lamports)?;
            }
            _ => {
                msg!(
                    "ERROR: Failed to burn {} lamports from {}",
                    lamports,
                    self.signer.key()
                );
                return Err(BridgeHandlerError::FailToBurnSol.into());
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn init_if_needed_and_check_bridge_handler_vault(&mut self) -> Result<()> {
        let rent = Rent::get()?;
        let owner_program = self.bridge_handler_vault.to_account_info().owner;

        if owner_program == self.system_program.key {
            // do init
            let cpi_program = self.associated_token_program.to_account_info();
            let cpi_accounts = ::anchor_spl::associated_token::Create {
                payer: self.signer.to_account_info(),
                associated_token: self.bridge_handler_vault.to_account_info(),
                authority: self.bridge_handler.to_account_info(),
                mint: self.mint.to_account_info(),
                system_program: self.system_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
            };
            let cpi_ctx = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts);
            ::anchor_spl::associated_token::create(cpi_ctx)?;
        }

        {
            let data = self
                .bridge_handler_vault
                .to_account_info()
                .data
                .borrow()
                .to_vec();
            let token_account = match TokenAccount::try_deserialize(&mut data.as_slice()) {
                Ok(val) => val,
                Err(e) => {
                    return Err(e.with_account_name("bridge_handler_vault"));
                }
            };

            if token_account.mint != self.mint.key() {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintTokenMint,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((token_account.mint, self.mint.key())));
            }

            if token_account.owner != self.bridge_handler.key() {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintTokenOwner,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((token_account.owner, self.bridge_handler.key())));
            }

            if owner_program != self.token_program.key {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintAssociatedTokenTokenProgram,
                )
                .with_account_name("bridge_handler_vault")
                .with_pubkeys((*owner_program, self.token_program.key())));
            }

            if self.bridge_handler_vault.key()
                != ::anchor_spl::associated_token::get_associated_token_address_with_program_id(
                    &self.bridge_handler.key(),
                    &self.mint.key(),
                    &self.token_program.key(),
                )
            {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::AccountNotAssociatedTokenAccount,
                )
                .with_account_name("bridge_handler_vault"));
            }
        }

        if !rent.is_exempt(
            self.bridge_handler_vault.to_account_info().lamports(),
            self.bridge_handler_vault.to_account_info().try_data_len()?,
        ) {
            return Err(anchor_lang::error::Error::from(
                anchor_lang::error::ErrorCode::ConstraintRentExempt,
            )
            .with_account_name("bridge_handler_vault"));
        }

        Ok(())
    }
}
