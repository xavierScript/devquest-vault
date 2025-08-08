// src/state.rs
// State definitions for the vault program

use anchor_lang::prelude::*;

/// Data structure for a scheduled payout
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct PayoutSchedule {
    pub amount: u64,                 // Amount to be paid
    pub next_payout_time: i64,       // Timestamp for next payout
    pub interval: i64,               // Time between payouts (in seconds)
    pub is_active: bool,             // Whether this schedule is active
}

/// Data structure for tracking epoch-based spending limits
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct EpochSpending {
    pub epoch_start: i64,            // Start time of current epoch
    pub spent_amount: u64,           // Amount spent in current epoch
    pub limit: u64,                  // Maximum amount that can be spent in an epoch
    pub duration: i64,               // Duration of epoch in seconds (e.g., 86400 for daily)
}

/// Main vault state account
#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
    pub admin: Pubkey,
    pub payees: Vec<Pubkey>,         // List of authorized payees
    pub payout_schedules: Vec<PayoutSchedule>,  // Scheduled payouts for each payee
    pub epoch_limits: Vec<(Pubkey, EpochSpending)>,  // Spending limits per payee
    pub is_initialized: bool,
}

impl Space for VaultState {
    // Calculate the required space for the VaultState account
    // 8 discriminator + 1 vault_bump + 1 state_bump + 32 admin + 
    // 4 vec length + (32 * 5) max payees + 
    // 4 vec length + (8 + 8 + 8 + 1) * 5 max schedules + 
    // 4 vec length + (32 + (8 + 8 + 8 + 8)) * 5 max epoch limits +
    // 1 is_initialized
    const INIT_SPACE: usize = 8 + 1 + 1 + 32 + 4 + (32 * 5) + 4 + (25 * 5) + 4 + (64 * 5) + 1;
}