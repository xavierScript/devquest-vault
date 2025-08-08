// src/instructions/withdraw.rs
// Withdraw instruction implementation

use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::{errors::CustomError, state::VaultState};

/// Accounts required for withdrawing SOL from the vault
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        seeds = [b"state", vault_state.admin.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    ctx.accounts.withdraw(amount)
}

pub fn claim_payout(ctx: Context<Withdraw>) -> Result<()> {
    ctx.accounts.claim_payout()
}

impl<'info> Withdraw<'info> {
    /// Handler for withdrawal logic (admin or authorized payee)
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        // Check if user is admin or authorized payee
        if self.user.key() != self.vault_state.admin &&
           !self.vault_state.payees.contains(&self.user.key()) {
            return err!(CustomError::UnauthorizedPayee);
        }
        // If user is not admin, check epoch spending limits
        if self.user.key() != self.vault_state.admin {
            let now = Clock::get()?.unix_timestamp;
            if let Some((_, epoch_spending)) = self.vault_state.epoch_limits
                .iter_mut()
                .find(|(p, _)| p == &self.user.key())
            {
                // Reset epoch if needed
                if now >= epoch_spending.epoch_start + epoch_spending.duration {
                    epoch_spending.epoch_start = now;
                    epoch_spending.spent_amount = 0;
                }
                // Check if withdrawal exceeds limit
                if epoch_spending.spent_amount + amount > epoch_spending.limit {
                    return err!(CustomError::EpochSpendingLimitReached);
                }
                // Update spent amount
                epoch_spending.spent_amount += amount;
            }
        }
        // Perform the withdrawal from vault to user
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let vault_state_key = self.vault_state.to_account_info().key;
        let vault_bump = self.vault_state.vault_bump;
        let seeds = &[
            b"vault",
            vault_state_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }

    /// Handler for claiming a scheduled payout (payee only)
    pub fn claim_payout(&mut self) -> Result<()> {
        let user_key = self.user.key();
        require!(self.vault_state.payees.contains(&user_key), CustomError::UnauthorizedPayee);
        let current_time = Clock::get()?.unix_timestamp;
        // Find the active payout schedule
        let schedule_index = self.vault_state.payout_schedules
            .iter()
            .position(|s| s.is_active)
            .ok_or(error!(CustomError::ScheduleNotFound))?;
        // Check if it's time for payout
        let schedule = &self.vault_state.payout_schedules[schedule_index];
        require!(current_time >= schedule.next_payout_time, CustomError::PayoutTimeNotReached);
        let amount = schedule.amount;
        // Transfer the scheduled amount from vault to user
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let vault_state_key = self.vault_state.to_account_info().key;
        let vault_bump = self.vault_state.vault_bump;
        let seeds = &[
            b"vault",
            vault_state_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)?;
        // Update next payout time for the schedule
        self.vault_state.payout_schedules[schedule_index].next_payout_time += 
            self.vault_state.payout_schedules[schedule_index].interval;
        Ok(())
    }
}