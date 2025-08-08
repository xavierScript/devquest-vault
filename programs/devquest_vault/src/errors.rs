// src/errors.rs
// Custom error definitions for the vault program

use anchor_lang::prelude::*;

/// Custom errors for the vault program
#[error_code]
pub enum CustomError {
    #[msg("Vault is already initialized")]
    AlreadyInitialized,
    #[msg("Only admin can perform this action")]
    UnauthorizedAdmin,
    #[msg("Maximum number of payees reached")]
    MaxPayeesReached,
    #[msg("Payee already exists")]
    PayeeAlreadyExists,
    #[msg("Payee not found")]
    PayeeNotFound,
    #[msg("Only authorized payees can withdraw")]
    UnauthorizedPayee,
    #[msg("Invalid payout schedule")]
    InvalidPayoutSchedule,
    #[msg("Maximum number of payout schedules reached")]
    MaxSchedulesReached,
    #[msg("Payout schedule not found")]
    ScheduleNotFound,
    #[msg("Payout time not reached")]
    PayoutTimeNotReached,
    #[msg("Epoch spending limit reached")]
    EpochSpendingLimitReached,
    #[msg("Invalid epoch configuration")]
    InvalidEpochConfig,
}