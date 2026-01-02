use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction};
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::solana_program::keccak;
use std::str::FromStr;

// Program ID for Solana
declare_id!("87JSCiht1TyXmT1yHbYZpKGtgJRhKzBYyFrmENvAogef");

// Constants
const RAFFLE_DURATION: i64 = 15 * 60; // 15 minutes in seconds
const TICKET_PRICE: u64 = 10_000_000; // 0.01 SOL in lamports
const RAFFLE_SEED: &[u8] = b"raffle"; // Fixed seed for raffle PDA
const TREASURY_WALLET: &str = "7fbAEwAuTHgBPxxf6dtvr8opw9tzVxYBVu1gXZNUJsAg"; // Replace with actual treasury wallet

#[program]
mod raffle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // Derive the raffle PDA based on the fixed seed
        let (raffle_pda, _bump) = Pubkey::find_program_address(&[RAFFLE_SEED], &ctx.program_id);

        // Ensure that the raffle PDA doesn't already exist
        let raffle_account = &mut ctx.accounts.raffle;
        require!(raffle_account.key() == raffle_pda, RaffleError::InvalidRaffleAccount);

        raffle_account.jackpot = 0;
        raffle_account.start_time = Clock::get()?.unix_timestamp;
        raffle_account.end_time = raffle_account.start_time + RAFFLE_DURATION;
        raffle_account.tickets = vec![];

        msg!("Raffle initialized at PDA: {}", raffle_pda);

        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>, amount: u64) -> Result<()> {
        // Derive the raffle PDA again using the fixed seed
        let (raffle_pda, _bump) = Pubkey::find_program_address(&[RAFFLE_SEED], &ctx.program_id);
    
        // Ensure the raffle PDA is correct
        let raffle = &mut ctx.accounts.raffle;
        require!(raffle.key() == raffle_pda, RaffleError::InvalidRaffleAccount);
    
        // Ensure the user sent enough for at least one ticket
        require!(amount >= TICKET_PRICE, RaffleError::InsufficientFunds);
    
        // Calculate number of tickets and remaining amount
        let tickets_bought = amount / TICKET_PRICE;
        let leftover = amount % TICKET_PRICE;
    
        // Refund leftover lamports if any
        if leftover > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.buyer.key(),
                    &ctx.accounts.buyer.key(),
                    leftover,
                ),
                &[
                    ctx.accounts.buyer.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }
    
        // Add 80% of the payment to the jackpot
        let jackpot_increment = (amount * 80) / 100;
        raffle.jackpot += jackpot_increment;
    
        // Transfer 20% to the treasury
        let treasury_cut = (amount * 20) / 100;
        let treasury_wallet = Pubkey::from_str(TREASURY_WALLET).unwrap();  // Replace with actual treasury wallet
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &treasury_wallet,
                treasury_cut,
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
    
        // Store tickets in the raffle state
        for _ in 0..tickets_bought {
            raffle.tickets.push(ctx.accounts.buyer.key());
        }
    
        msg!("{} tickets bought by {}", tickets_bought, ctx.accounts.buyer.key());
    
        Ok(())
    }
    
    pub fn select_winner(ctx: Context<SelectWinner>) -> Result<()> {
        // Derive the raffle PDA again using the fixed seed
        let (raffle_pda, _bump) = Pubkey::find_program_address(&[RAFFLE_SEED], &ctx.program_id);

        // Ensure the raffle PDA is correct
        let raffle = &mut ctx.accounts.raffle;
        require!(raffle.key() == raffle_pda, RaffleError::InvalidRaffleAccount);

        // Ensure raffle has ended
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time >= raffle.end_time, RaffleError::RaffleStillActive);

        // Ensure there are tickets
        require!(!raffle.tickets.is_empty(), RaffleError::NoTickets);

        // Select random winner using Keccak256 for better randomness
        let seed = Clock::get()?.unix_timestamp.to_le_bytes();
        let hash = keccak::hash(&seed).to_bytes();
        let winner_index = (u64::from_le_bytes(hash[0..8].try_into().unwrap()) as usize) % raffle.tickets.len();
        let winner = raffle.tickets[winner_index];

        // Transfer jackpot to winner
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.program.key(),
                &winner,
                raffle.jackpot,
            ),
            &[ctx.accounts.program.to_account_info()],
        )?;

        // Reset raffle
        raffle.jackpot = 0;
        raffle.start_time = current_time + 10; // 10 seconds to next raffle
        raffle.end_time = raffle.start_time + RAFFLE_DURATION;
        raffle.tickets = vec![];

        msg!("Winner selected: {}", winner);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = admin, space = 8 + 32100)] // Space includes discriminator
    pub raffle: Account<'info, Raffle>,               // PDA for Raffle
    #[account(mut)]
    pub admin: Signer<'info>,                         // Admin initializing the raffle
    pub system_program: Program<'info, System>,       // System program for account creation
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,                         // Ticket buyer
    #[account(mut)]
    pub raffle: Account<'info, Raffle>,               // Raffle account storing ticket details
    #[account(mut)]
    pub program: AccountInfo<'info>,                  // PDA for program fees
    pub system_program: Program<'info, System>,       // System program for funds transfer
}

#[derive(Accounts)]
pub struct SelectWinner<'info> {
    #[account(mut)]
    pub raffle: Account<'info, Raffle>,               // Raffle account for winner selection
    #[account(mut)]
    pub program: AccountInfo<'info>,                  // PDA holding funds
    pub system_program: Program<'info, System>,       // System program for winner payment
}


#[account]
pub struct Raffle {
    pub jackpot: u64,          // Total prize pool
    pub start_time: i64,       // Raffle start timestamp
    pub end_time: i64,         // Raffle end timestamp
    pub tickets: Vec<Pubkey>,  // List of participants' public keys
}


#[error_code]
pub enum RaffleError {
    #[msg("The amount sent is insufficient to buy a ticket.")]
    InsufficientFunds,

    #[msg("The amount sent does not match the ticket price.")]
    IncorrectAmount,

    #[msg("The provided treasury wallet is invalid.")]
    InvalidTreasuryWallet,

    #[msg("Failed to transfer funds.")]
    TransferFailed,

    #[msg("The raffle is still active. Cannot end the raffle before the end time.")]
    RaffleStillActive,

    #[msg("No tickets have been purchased for this raffle.")]
    NoTickets,

    #[msg("Invalid raffle account.")]
    InvalidRaffleAccount,
}
