use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use solana_program::hash::hash;

use crate::constants::{MIN_SOL_BRIDGE_AMOUNT, SOL_MINT_PUBKEY};
use crate::states::{BridgeProofSourceChain, Chain};
use crate::{errors::BridgeHandlerError, states::BridgeHandler};

// The below precompile is used to burn SOL on Solayer only
#[allow(dead_code)]
#[cfg(target_os = "solana")]
extern "C" {
    fn sol_burn_native_sol(amount: u64, account_idx: u64) -> u64;
}

#[derive(Accounts)]
#[instruction(bridge_proof_nonce: u64)]
pub struct BridgeAssetSourceChainSol<'info> {
    #[account(mut)]
    signer: Signer<'info>,
    #[account(
        mut,
        has_one = fee_vault,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        init,
        payer = signer,
        space = 8 + BridgeProofSourceChain::INIT_SPACE,
        seeds = [b"bridge_proof", bridge_handler.key().as_ref(), signer.key().as_ref(), bridge_proof_nonce.to_be_bytes().as_ref()],
        bump
    )]
    bridge_proof: Box<Account<'info, BridgeProofSourceChain>>,
    #[account(mut)]
    /// CHECK: no check needed other than address check
    fee_vault: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

impl BridgeAssetSourceChainSol<'_> {
    pub fn bridge_asset_source_chain_sol(&mut self, amount: u64, recipient: Pubkey) -> Result<u64> {
        require!(!self.bridge_handler.pause, BridgeHandlerError::BridgePaused);
        require!(
            amount >= MIN_SOL_BRIDGE_AMOUNT,
            BridgeHandlerError::TooLittleSolBridgeAmount
        );

        self.transfer_sol_to_fee_vault(self.bridge_handler.fee_info.bridge_asset_fee)?;

        // lamports auto reloads after cpi
        require!(
            self.signer.lamports() >= amount,
            BridgeHandlerError::InsufficientAmount
        );
        msg!(
            "bridging sol from {:?} to {:?}",
            self.signer.key(),
            recipient
        );

        if self.bridge_handler.chain == Chain::Solana {
            self.transfer_sol_to_bridge_handler(amount)?;
        } else {
            self.burn_sol(amount)?;
        }

        let nonce = self.bridge_handler.nonce;
        msg!("nonce: {:?}", nonce);
        self.bridge_handler.nonce = nonce.checked_add(1).unwrap();
        Ok(nonce)
    }

    pub fn issue_bridge_proof(
        &mut self,
        bumps: BridgeAssetSourceChainSolBumps,
        bridge_proof_nonce: u64,
        amount: u64,
        recipient: Pubkey,
    ) -> Result<()> {
        self.bridge_proof.bump = bumps.bridge_proof;
        let mut message_data = Vec::new();
        message_data.extend_from_slice(&self.signer.key().to_bytes());
        message_data.extend_from_slice(&recipient.to_bytes());
        message_data.extend_from_slice(&SOL_MINT_PUBKEY.to_bytes());
        message_data.extend_from_slice(&amount.to_be_bytes());
        message_data.extend_from_slice(&bridge_proof_nonce.to_be_bytes());
        self.bridge_proof.msg_hash = hash(message_data.as_ref()).to_bytes();
        self.bridge_proof.user_account = self.signer.key();
        self.bridge_proof.created_at = Clock::get()?.unix_timestamp as u64;
        Ok(())
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
}
