// src/lib.rs
// Main program entry point and module declarations

use anchor_lang::prelude::*;

// Import our modules
mod errors;
mod state;
mod instructions;

// Re-export what we want to be public
pub use errors::*;
pub use state::*;
pub use instructions::*;

// Program ID for the deployed contract
// declare_id!("8b4G7EpokrxCpb1BMK5kjFMtLrxG1KBYB5yjVVva2cs8");
declare_id!("cDTxPaqGFcfNwMmcj283j7dNPrYdMfw3J1T2oYzv36U");

/// Main program module for devquest_vault
#[program]
pub mod devquest_vault {
    use super::*;

    /// Initializes the vault and vault state accounts
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Adds a new payee to the vault (admin only)
    pub fn add_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        instructions::payee::add_payee(ctx, payee)
    }

    /// Removes a payee from the vault (admin only)
    pub fn remove_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
        instructions::payee::remove_payee(ctx, payee)
    }

    /// Deposits SOL into the vault
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Sets an epoch spending limit for a payee (admin only)
    pub fn set_epoch_limit(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
        limit: u64,
        duration: i64
    ) -> Result<()> {
        instructions::payee::set_epoch_limit(ctx, payee, limit, duration)
    }

    /// Withdraws SOL from the vault (admin or authorized payee)
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::withdraw(ctx, amount)
    }

    /// Closes the vault and returns remaining funds to the admin
    pub fn close(ctx: Context<Close>) -> Result<()> {
        instructions::close::handler(ctx)
    }

    /// Schedules a recurring payout for a payee (admin only)
    pub fn schedule_payout(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
        amount: u64,
        start_time: i64,
        interval: i64,
    ) -> Result<()> {
        instructions::payee::schedule_payout(ctx, payee, amount, start_time, interval)
    }

    /// Cancels a payout schedule for a payee (admin only)
    pub fn cancel_payout(
        ctx: Context<UpdatePayee>,
        payee: Pubkey,
    ) -> Result<()> {
        instructions::payee::cancel_payout(ctx, payee)
    }

    /// Allows a payee to claim their scheduled payout
    pub fn claim_payout(
        ctx: Context<Withdraw>,
    ) -> Result<()> {
        instructions::withdraw::claim_payout(ctx)
    }
}