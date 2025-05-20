mod instructions;

use anchor_lang::prelude::*;
use instructions::*;

const PDA_USER: &[u8] = b"user_account";
const PDA_VAULT: &[u8] = b"vault";
// Replace with your actual program ID after deployment
declare_id!("EL3Wpg3SVp5xqEW3SryBwmTsKBNR8Sg3VEfdvejmLMR9");

#[event]
pub struct CustomEvent {
    pub message: String,
}
#[program]
pub mod sol_deposit {
    use super::*;

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let user = &mut ctx.accounts.user_account;
        let clock = Clock::get()?;
        user.owner = ctx.accounts.user.key();
        user.balance = user.balance.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        user.last_deposit = clock.unix_timestamp;

        // Rent-exempt check
        let vault_lamports = ctx.accounts.vault.to_account_info().lamports();
        let rent = Rent::get()?;
        let required_rent = rent.minimum_balance(ctx.accounts.vault.to_account_info().data_len());

        if vault_lamports < required_rent && (vault_lamports + amount) < required_rent {
            return err!(ErrorCode::VaultNotRentExempt);
        }

        // Transfer SOL from user to vault PDA (deposit)
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.vault.key(),
            amount,
        );
        emit!(CustomEvent{message: "Before transfer".to_string()});
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault.to_account_info(),
            ],
            &[],
        )?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let user = &mut ctx.accounts.user_account;
        require!(user.balance >= amount, ErrorCode::InsufficientFunds);
        user.balance = user.balance.checked_sub(amount).ok_or(ErrorCode::Overflow)?;

        // Transfer SOL from vault PDA to user
        let bump = ctx.bumps.vault;
        let seeds = &[PDA_VAULT, &[bump]];
        let signer_seeds = &[&seeds[..]];
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.vault.key(),
            &ctx.accounts.user.key(),
            amount,
        );
        emit!(CustomEvent{message: "Before transfer".to_string()});
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            signer_seeds,
        )?;
        Ok(())
    }
}

#[account]
pub struct UserAccount {
    pub owner: Pubkey,
    pub balance: u64,
    pub last_deposit: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Overflow in balance calculation")]
    Overflow,
    #[msg("Deposit must be enough to make the vault rent-exempt")]
    VaultNotRentExempt,
}
