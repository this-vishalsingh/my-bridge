use anchor_lang::prelude::*;

use crate::constants::MAX_GUARDIAN_SIGNATURES;
use crate::states::{GuardianInfo, VerifiedSignatures};
use crate::utils::verify_ed25519_ix;
use crate::{errors::BridgeHandlerError, states::BridgeHandler};
use solana_program::sysvar::instructions::ID as IX_ID;

#[derive(Accounts)]
#[instruction(msg_hash: [u8; 32])]
pub struct VerifySignature<'info> {
    #[account(mut)]
    operator: Signer<'info>,
    #[account(
        seeds = [b"bridge_handler", bridge_handler.init_nonce.to_be_bytes().as_ref()],
        bump = bridge_handler.bump
    )]
    bridge_handler: Box<Account<'info, BridgeHandler>>,
    #[account(
        seeds = [b"guardian_info", bridge_handler.key().as_ref()],
        bump = guardian_info.bump
    )]
    guardian_info: Box<Account<'info, GuardianInfo>>,
    #[account(
        init_if_needed,
        payer = operator,
        space = 8 + VerifiedSignatures::INIT_SPACE,
        seeds = [b"verified_signatures", bridge_handler.key().as_ref(), msg_hash.as_ref()],
        bump
    )]
    verified_signatures: Box<Account<'info, VerifiedSignatures>>,
    system_program: Program<'info, System>,
    /// CHECK: only address check is needed
    #[account(address = IX_ID)]
    ix_sysvar: AccountInfo<'info>,
}

impl VerifySignature<'_> {
    pub fn verify_signature(
        &mut self,
        bump: VerifySignatureBumps,
        msg_hash: [u8; 32],
        signer_indexes: Vec<u8>,
    ) -> Result<()> {
        require!(
            !signer_indexes.is_empty()
                && signer_indexes.len() <= self.guardian_info.guardians.len(),
            BridgeHandlerError::InvalidSignerCount
        );

        require!(
            !signer_indexes
                .iter()
                .any(|index| self.verified_signatures.pubkey_index.contains(index)),
            BridgeHandlerError::GuardianSignatureAlreadyExists
        );

        // make sure there is no duplicate signer indexes
        let mut unique_signer_indexes = signer_indexes.clone();
        unique_signer_indexes.sort();
        unique_signer_indexes.dedup();
        require!(
            unique_signer_indexes.len() == signer_indexes.len(),
            BridgeHandlerError::InvalidSignerIndexes
        );

        let concat_sig_count = self.verified_signatures.pubkey_index.len() + signer_indexes.len();
        require!(
            concat_sig_count <= self.guardian_info.guardians.len()
                && concat_sig_count <= MAX_GUARDIAN_SIGNATURES,
            BridgeHandlerError::InvalidGuardianSigCount
        );

        let signers = signer_indexes
            .iter()
            .map(|index| self.guardian_info.guardians[*index as usize])
            .collect::<Vec<Pubkey>>();

        msg!("Guardian signers: {:?}", signers);
        verify_ed25519_ix(&self.ix_sysvar, signers, msg_hash)?;

        self.verified_signatures.bump = bump.verified_signatures;
        self.verified_signatures.pubkey_index.extend(signer_indexes);
        self.verified_signatures.created_at = Clock::get()?.unix_timestamp as u64;
        Ok(())
    }
}
