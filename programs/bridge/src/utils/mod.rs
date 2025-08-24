pub mod verify_ed25519;
use anchor_lang::prelude::Pubkey;
pub use verify_ed25519::*;

#[inline(always)]
pub const fn pubkey_from_str(s: &str) -> Pubkey {
    Pubkey::new_from_array(five8_const::decode_32_const(s))
}
