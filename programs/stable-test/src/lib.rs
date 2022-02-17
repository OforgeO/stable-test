use anchor_lang::prelude::*;
use crate::{error::StableTestError};
use anchor_lang::solana_program::{program::invoke, program::invoke_signed, system_instruction };
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, MintTo, TokenAccount, Transfer};
use pyth_client;
use std::mem::size_of;

pub mod error;

declare_id!("DHkf7V1VjWCdJDMGogvvGC2S2H7mSS4Cv6YBM1mw8mK1");

#[program]
pub mod stable_test {
    use super::*;
    pub fn process_escrow(
        ctx: Context<Initialize>,
        _nonce: u8,
        _price_nonce: u8,
        token_authority_bump: u8,
        sol_amount: u64
    ) -> ProgramResult {
        if **ctx.accounts.user_account.lamports.borrow() < sol_amount {
            msg!("No enough SOL");
            return Err(StableTestError::NoEnough.into());
        }

        // Transfer SOL to the escrow account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.escrow_account.key,
                sol_amount,
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.escrow_account.clone(),
                ctx.accounts.system_program.clone(),
            ],
        )?;

        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let sc_usd_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        if sc_usd_price < 0 {
            msg!("The SOL/USD price is wrong.");
            return Err(StableTestError::UsdPriceWrong.into());
        }

        let init_stable_supply = ( sc_usd_price * sol_amount) / (10u32.pow(8) as u64);

        msg!("Total Supply:  {:?}", init_stable_supply);        

        let seeds = &[&b"mint-authority"[..], &[token_authority_bump]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.stable_token.to_account_info(),
            to: ctx.accounts.stable_account.to_account_info(),
            authority: ctx.accounts.token_authority.to_account_info(),
        };

        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
            ).with_signer(&[&seeds[..]]),
            init_stable_supply
        )?;

        ctx.accounts.sol_price_account.last_sol_price = sc_usd_price;
        ctx.accounts.sol_price_account.stable_total_supply = init_stable_supply;

        Ok(())
    }

    pub fn process_mint_burn_token(
        ctx: Context<MintBurnToken>,
        token_authority_bump: u8,
    ) -> ProgramResult {
        let pyth_price_info = &ctx.accounts.pyth_account;
        let pyth_price_data = &pyth_price_info.try_borrow_data()?;
        let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);

        let current_price = pyth_price.agg.price as u64; // Get the SOL/USD price from pyth.network

        let last_sol_price = ctx.accounts.sol_price_account.last_sol_price;

        if current_price < last_sol_price {
            let burn_amount = last_sol_price - current_price;
            let cpi_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.stable_token.to_account_info(),
                    to: ctx.accounts.stable_account.to_account_info(),
                    authority: ctx.accounts.token_authority.to_account_info(),
                },
            );
            token::burn(cpi_ctx, burn_amount)?;

        } else if current_price > last_sol_price {
            let mint_amount = current_price - last_sol_price;

            let seeds = &[&b"mint-authority"[..], &[token_authority_bump]];

            let cpi_accounts = MintTo {
                mint: ctx.accounts.stable_token.to_account_info(),
                to: ctx.accounts.stable_account.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            };
    
            token::mint_to(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    cpi_accounts,
                ).with_signer(&[&seeds[..]]),
                mint_amount
            )?;
        }

        ctx.accounts.sol_price_account.last_sol_price = current_price;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(nonce: u8, price_nonce: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"escrow".as_ref()],
        bump = nonce,
        payer = user_account,
        space = 8
    )]
    pub escrow_account: AccountInfo<'info>,
    #[account(mut)]
    pub stable_account: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        seeds = [b"sol_price".as_ref()],
        bump = price_nonce,
        payer = user_account,
        space = 8 + size_of::<SolPriceAccount>()
    )]
    pub sol_price_account: Box<Account<'info, SolPriceAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct MintBurnToken<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(mut)]
    pub stable_token: Box<Account<'info, Mint>>,
    pub token_authority: AccountInfo<'info>,
    #[account(mut)]
    pub stable_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub sol_price_account: Box<Account<'info, SolPriceAccount>>,
    pub pyth_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

#[account]
pub struct SolPriceAccount {
    pub last_sol_price: u64,
    pub stable_total_supply: u64
}