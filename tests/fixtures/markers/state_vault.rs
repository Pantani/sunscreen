// Fixture: `state/<acc>.rs` — `state` segment.

use anchor_lang::prelude::*;

// === sunscreen:auto-generated:begin segment=state version=1 generator=account ===
#[account]
pub struct Vault {
    pub total: u64,
    pub authority: Pubkey,
    pub bump: u8,
}
// === sunscreen:auto-generated:end segment=state ===
