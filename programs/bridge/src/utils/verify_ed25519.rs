use anchor_lang::prelude::*;
use solana_program::ed25519_program;
use solana_program::sysvar::instructions::get_instruction_relative;

use crate::constants::{
    MESSAGE_DATA_SIZE, PUBKEY_SERIALIZED_SIZE, SIGNATURE_OFFSETS_SERIALIZED_SIZE,
    SIGNATURE_OFFSETS_START, SIGNATURE_SERIALIZED_SIZE,
};
use crate::errors::BridgeHandlerError;

#[allow(dead_code)]
struct Ed25519SignatureOffsets {
    signature_offset: u16,             // offset to ed25519 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get message data
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Ed25519Signature {
    pubkey: [u8; PUBKEY_SERIALIZED_SIZE],
    signature: [u8; SIGNATURE_SERIALIZED_SIZE],
    msg: [u8; MESSAGE_DATA_SIZE],
}

fn iter_signature_offsets(
    data: &[u8],
) -> Result<impl Iterator<Item = Ed25519SignatureOffsets> + '_> {
    let num_sigs = *data.first().ok_or(BridgeHandlerError::InvalidEd25519Data)?;
    let all_structs_size = SIGNATURE_OFFSETS_SERIALIZED_SIZE
        .checked_mul(num_sigs as usize)
        .ok_or(BridgeHandlerError::InvalidEd25519Data)?;
    require!(
        all_structs_size + SIGNATURE_OFFSETS_START <= data.len(),
        BridgeHandlerError::InvalidEd25519Data
    );
    let all_structs_slice = data
        .get(SIGNATURE_OFFSETS_START..all_structs_size + SIGNATURE_OFFSETS_START)
        .ok_or(BridgeHandlerError::InvalidEd25519Data)?;

    fn decode_u16(chunk: &[u8], index: usize) -> u16 {
        u16::from_le_bytes(<[u8; 2]>::try_from(&chunk[index..index + 2]).unwrap())
    }

    Ok(all_structs_slice
        .chunks(SIGNATURE_OFFSETS_SERIALIZED_SIZE)
        .map(|chunk| Ed25519SignatureOffsets {
            signature_offset: decode_u16(chunk, 0),
            signature_instruction_index: decode_u16(chunk, 2),
            public_key_offset: decode_u16(chunk, 4),
            public_key_instruction_index: decode_u16(chunk, 6),
            message_data_offset: decode_u16(chunk, 8),
            message_data_size: decode_u16(chunk, 10),
            message_instruction_index: decode_u16(chunk, 12),
        }))
}

fn load_signatures(data: &[u8]) -> Result<Vec<Ed25519Signature>> {
    let mut signatures = vec![];
    for offsets in iter_signature_offsets(data)? {
        let signature = data
            .get(
                offsets.signature_offset as usize
                    ..offsets.signature_offset as usize + SIGNATURE_SERIALIZED_SIZE,
            )
            .ok_or(BridgeHandlerError::InvalidEd25519Data)?;
        let pubkey = data
            .get(
                offsets.public_key_offset as usize
                    ..offsets.public_key_offset as usize + PUBKEY_SERIALIZED_SIZE,
            )
            .ok_or(BridgeHandlerError::InvalidEd25519Data)?;
        let msg = data
            .get(
                offsets.message_data_offset as usize
                    ..offsets.message_data_offset as usize + offsets.message_data_size as usize,
            )
            .ok_or(BridgeHandlerError::InvalidEd25519Data)?;

        signatures.push(Ed25519Signature {
            pubkey: <[u8; PUBKEY_SERIALIZED_SIZE]>::try_from(pubkey).unwrap(),
            signature: <[u8; SIGNATURE_SERIALIZED_SIZE]>::try_from(signature).unwrap(),
            msg: <[u8; MESSAGE_DATA_SIZE]>::try_from(msg).unwrap(),
        });
    }

    Ok(signatures)
}

pub fn verify_ed25519_ix(
    ix_sysvar_account: &AccountInfo,
    signers: Vec<Pubkey>,
    message: [u8; MESSAGE_DATA_SIZE],
) -> Result<()> {
    let ed25519_inst = get_instruction_relative(-1, ix_sysvar_account)?;
    require!(
        ed25519_program::check_id(&ed25519_inst.program_id),
        BridgeHandlerError::InvalidEd25519ProgramId
    );
    require!(
        ed25519_inst.accounts.is_empty(),
        BridgeHandlerError::InvalidEd25519Accounts
    );
    require!(
        ed25519_inst.data.len() > 1,
        BridgeHandlerError::InvalidEd25519Data
    );

    let signatures = load_signatures(&ed25519_inst.data)?;

    require!(
        signatures.len() == signers.len(),
        BridgeHandlerError::InvalidGuardianSigCount
    );

    for (id, sig_bundle) in signatures.iter().enumerate() {
        require!(
            sig_bundle.pubkey == signers[id].to_bytes(),
            BridgeHandlerError::InvalidGuardianSignaturePubKey
        );
        require!(
            sig_bundle.msg == message,
            BridgeHandlerError::InvalidGuardianSignatureMessage
        );
    }

    Ok(())
}
