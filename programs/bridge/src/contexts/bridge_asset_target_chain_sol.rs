use anchor_lang::prelude::*;
use solana_program::hash::hash;
use solana_program::native_token::LAMPORTS_PER_SOL;

use crate::constants::{
    MAX_GUARDIAN_SIGNATURES, SOLANA_DOLLAR_CAP_PER_EPOCH, SOLANA_DOLLAR_VALUE,
    SOLANA_POST_INSTANT_CAP_AWAITING_TIME_SECONDS, SOL_MINT_PUBKEY,
};
use crate::states::{BridgeProof, GuardianInfo, VerifiedSignatures};
use crate::{
    errors::BridgeHandlerError,
    states::{BridgeHandler, Chain},
};

// The below precompile is used to mint SOL on Solayer only
#[allow(dead_code)]
#[cfg(target_os = "solana")]
extern "C" {
    fn sol_mint_native_sol(amount: u64, account_idx: u64) -> u64;
}

#[derive(Accounts)]
#[instruction(msg_hash: [u8; 32], source_tx_id: [u8; 64])]
pub struct BridgeAssetTargetChainSol<'info> {
    #[account(mut)]
    operator: Signer<'info>,
    /// CHECKED: checks will be performed agsint signature with hash
    #[account(mut)]
    recipient: AccountInfo<'info>,
    #[account(
        mut,
        has_one = operator @ BridgeHandlerError::InvalidOperator,
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
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
    system_program: Program<'info, System>,
}

impl BridgeAssetTargetChainSol<'_> {
    pub fn bridge_asset_target_chain_sol(
        &mut self,
        bumps: BridgeAssetTargetChainSolBumps,
        msg_hash: [u8; 32],
        source_tx_id: [u8; 64],
        sender: Pubkey,
        receive_amount: u64,
        nonce: u64,
    ) -> Result<()> {
        require!(!self.bridge_handler.pause, BridgeHandlerError::BridgePaused);

        let chain = self.bridge_handler.chain;

        // validate instant bridge cap
        if chain == Chain::Solana {
            let dollar_value =
                receive_amount as f64 / LAMPORTS_PER_SOL as f64 * SOLANA_DOLLAR_VALUE as f64;

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
                self.bridge_handler.instant_bridge_cap_remained_dollar =
                    (self.bridge_handler.instant_bridge_cap_remained_dollar)
                        .checked_sub(dollar_value as u64)
                        .unwrap();
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

        let mut message_data = Vec::new();
        message_data.extend_from_slice(&sender.to_bytes());
        message_data.extend_from_slice(&self.recipient.key().to_bytes());
        message_data.extend_from_slice(&SOL_MINT_PUBKEY.to_bytes());
        message_data.extend_from_slice(&receive_amount.to_be_bytes());
        message_data.extend_from_slice(&nonce.to_be_bytes());
        message_data.extend_from_slice(&source_tx_id);

        let message: [u8; 32] = hash(message_data.as_ref()).to_bytes();
        require!(
            message == msg_hash,
            BridgeHandlerError::InvalidGuardianSignatureMessage
        );

        self.bridge_proof.bump = bumps.bridge_proof;
        self.bridge_proof.msg_hash = message;
        self.bridge_proof.tx_id = source_tx_id;
        self.bridge_proof.user_account = self.recipient.key();
        self.bridge_proof.created_at = Clock::get()?.unix_timestamp as u64;

        if chain == Chain::Solayer {
            self.mint_sol_to_recipient(receive_amount)?;
        } else {
            self.transfer_sol_to_recipient(receive_amount)?;
        }

        Ok(())
    }

    #[cfg(feature = "solayer")]
    fn mint_sol_to_recipient(&mut self, lamports: u64) -> Result<()> {
        #[cfg(target_os = "solana")]
        let result = unsafe { sol_mint_native_sol(lamports, 1) };
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
