// Fixture: `events.rs` — `events` segment.

use anchor_lang::prelude::*;

// === sunscreen:auto-generated:begin segment=events version=1 generator=event ===
#[event]
pub struct Deposited {
    pub depositor: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Withdrawn {
    pub recipient: Pubkey,
    pub amount: u64,
}
// === sunscreen:auto-generated:end segment=events ===
