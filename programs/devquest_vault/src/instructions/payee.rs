// src/instructions/payee.rs
// Payee management instruction implementation

use anchor_lang::prelude::*;
use crate::{errors::CustomError, state::{VaultState, EpochSpending, PayoutSchedule}};

/// Accounts required for updating payees and payout schedules
#[derive(Accounts)]
pub struct UpdatePayee<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"state", vault_state.admin.key().as_ref()],
        bump = vault_state.state_bump,
        constraint = user.key() == vault_state.admin @ CustomError::UnauthorizedAdmin,
    )]
    pub vault_state: Account<'info, VaultState>,
}

pub fn add_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
    ctx.accounts.add_payee(payee)
}

pub fn remove_payee(ctx: Context<UpdatePayee>, payee: Pubkey) -> Result<()> {
    ctx.accounts.remove_payee(payee)
}

pub fn set_epoch_limit(
    ctx: Context<UpdatePayee>,
    payee: Pubkey,
    limit: u64,
    duration: i64
) -> Result<()> {
    require!(duration > 0, CustomError::InvalidEpochConfig);
    require!(limit > 0, CustomError::InvalidEpochConfig);

    let state = &mut ctx.accounts.vault_state;
    // Only admin can set limits
    require!(ctx.accounts.user.key() == state.admin, CustomError::UnauthorizedAdmin);
    // Check if payee exists
    require!(state.payees.contains(&payee), CustomError::PayeeNotFound);
    let now = Clock::get()?.unix_timestamp;
    // Find existing epoch limit or create new one
    if let Some(index) = state.epoch_limits.iter().position(|(p, _)| p == &payee) {
        state.epoch_limits[index].1 = EpochSpending {
            epoch_start: now,
            spent_amount: 0,
            limit,
            duration,
        };
    } else {
        state.epoch_limits.push((
            payee,
            EpochSpending {
                epoch_start: now,
                spent_amount: 0,
                limit,
                duration,
            }
        ));
    }
    Ok(())
}

pub fn schedule_payout(
    ctx: Context<UpdatePayee>,
    payee: Pubkey,
    amount: u64,
    start_time: i64,
    interval: i64,
) -> Result<()> {
    ctx.accounts.schedule_payout(payee, amount, start_time, interval)
}

pub fn cancel_payout(
    ctx: Context<UpdatePayee>,
    payee: Pubkey,
) -> Result<()> {
    ctx.accounts.cancel_payout(payee)
}

impl<'info> UpdatePayee<'info> {
    /// Handler for adding a new payee (admin only)
    pub fn add_payee(&mut self, payee: Pubkey) -> Result<()> {
        require!(self.vault_state.payees.len() < 5, CustomError::MaxPayeesReached);
        require!(!self.vault_state.payees.contains(&payee), CustomError::PayeeAlreadyExists);
        self.vault_state.payees.push(payee);
        Ok(())
    }

    /// Handler for removing a payee (admin only)
    pub fn remove_payee(&mut self, payee: Pubkey) -> Result<()> {
        if let Some(index) = self.vault_state.payees.iter().position(|x| *x == payee) {
            self.vault_state.payees.remove(index);
            // Also remove any associated payout schedule
            let schedule_index = self.vault_state.payout_schedules
                .iter()
                .position(|s| s.is_active);
            if let Some(idx) = schedule_index {
                self.vault_state.payout_schedules.remove(idx);
            }
            Ok(())
        } else {
            err!(CustomError::PayeeNotFound)
        }
    }

    /// Handler for scheduling a payout (admin only)
    pub fn schedule_payout(
        &mut self,
        payee: Pubkey,
        amount: u64,
        start_time: i64,
        interval: i64,
    ) -> Result<()> {
        // Validate inputs
        require!(amount > 0, CustomError::InvalidPayoutSchedule);
        require!(interval > 0, CustomError::InvalidPayoutSchedule);
        require!(start_time > Clock::get()?.unix_timestamp, CustomError::InvalidPayoutSchedule);
        require!(self.vault_state.payees.contains(&payee), CustomError::PayeeNotFound);
        require!(self.vault_state.payout_schedules.len() < 5, CustomError::MaxSchedulesReached);
        let schedule = PayoutSchedule {
            amount,
            next_payout_time: start_time,
            interval,
            is_active: true,
        };
        self.vault_state.payout_schedules.push(schedule);
        Ok(())
    }

    /// Handler for cancelling a payout schedule (admin only)
    pub fn cancel_payout(&mut self, payee: Pubkey) -> Result<()> {
        require!(self.vault_state.payees.contains(&payee), CustomError::PayeeNotFound);
        if let Some(schedule_index) = self.vault_state.payout_schedules.iter().position(|_| true) {
            self.vault_state.payout_schedules[schedule_index].is_active = false;
            Ok(())
        } else {
            err!(CustomError::ScheduleNotFound)
        }
    }
}