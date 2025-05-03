use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};
use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use std::str::FromStr;

declare_id!("F4YwKDaX4RTHfpN53mpcxMKt5syxqPrUtfyvW2T64wcn");

const ADMIN_PUBKEY: &str = "9B5XszUGdMaxCZ7uSQhPzdks5ZQSmWxrmzCSvtJ6Ns6g";

#[program]
pub mod tp_solana_5ibc {
    use super::*;

    pub fn initialize_joueur(ctx: Context<InitializeJoueur>, pseudo: String) -> Result<()> {
        let joueur = &mut ctx.accounts.joueur;
        joueur.pdv = 100;
        joueur.pseudonyme = pseudo;
        joueur.xp = 0;
        joueur.vivant = true;
        joueur.user_address = *ctx.accounts.signer.key;
        joueur.level = Level::Beginner;

        msg!("Joueur initialized!");
        msg!("pdv {}", joueur.pdv);
        msg!("pseudonyme {}", joueur.pseudonyme);
        msg!("xp {}", joueur.xp);
        msg!("vivant {}", joueur.vivant);
        msg!("user_address {}", joueur.user_address);

        Ok(())
    }

    pub fn buy_experience(ctx: Context<BuyExperience>) -> Result<()> {
        let joueur = &mut ctx.accounts.joueur;
        joueur.xp = joueur.xp.checked_add(10).unwrap();

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.signer.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        let amount = (LAMPORTS_PER_SOL as f64 * 0.1) as u64;
        system_program::transfer(cpi_ctx, amount)?;

        joueur.level = match joueur.xp {
            xp if xp < 5 => Level::Beginner,
            xp if xp < 35 => Level::Explorer,
            xp if xp < 100 => Level::Champion,
            _ => Level::Legend,
        };

        Ok(())
    }

    pub fn withdraw_vault(ctx: Context<WithdrawVault>) -> Result<()> {
        let admin_pubkey = Pubkey::from_str(ADMIN_PUBKEY).unwrap();
        require_keys_eq!(ctx.accounts.admin.key(), admin_pubkey, ErrorCode::NonAdmin);

        let amount = **ctx.accounts.vault.to_account_info().lamports.borrow();

        // Aquí accedemos al bump como struct field, no como índice
        let seeds = &[b"vault".as_ref(), &[ctx.bumps.vault]];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.admin.to_account_info(),
            },
            signer_seeds,
        );
        system_program::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn battle(ctx: Context<Battle>) -> Result<()> {
        let attacker = &mut ctx.accounts.attacker;
        let defender = &mut ctx.accounts.defender;

        require!(attacker.vivant, ErrorCode::JoueurMort);
        require!(defender.vivant, ErrorCode::JoueurMort);

        if attacker.xp >= defender.xp {
            if defender.pdv <= 50 {
                defender.pdv = 0;
                defender.vivant = false;
            } else {
                defender.pdv -= 50;
            }
            attacker.xp = attacker.xp.checked_add(5).unwrap();
        } else {
            if attacker.pdv <= 50 {
                attacker.pdv = 0;
                attacker.vivant = false;
            } else {
                attacker.pdv -= 50;
            }
            defender.xp = defender.xp.checked_add(5).unwrap();
        }

        attacker.level = match attacker.xp {
            xp if xp < 5 => Level::Beginner,
            xp if xp < 35 => Level::Explorer,
            xp if xp < 100 => Level::Champion,
            _ => Level::Legend,
        };
        defender.level = match defender.xp {
            xp if xp < 5 => Level::Beginner,
            xp if xp < 35 => Level::Explorer,
            xp if xp < 100 => Level::Champion,
            _ => Level::Legend,
        };

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeJoueur<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + Joueur::INIT_SPACE,
        seeds = [b"joueur", signer.key().as_ref()],
        bump
    )]
    pub joueur: Account<'info, Joueur>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyExperience<'info> {
    #[account(
        mut,
        seeds = [b"joueur", signer.key().as_ref()],
        bump
    )]
    pub joueur: Account<'info, Joueur>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

    /// CHECK: Esta cuenta `vault` es un PDA derivado con seed ["vault"] y
    /// su bump se valida en `ctx.bumps.vault`.
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WithdrawVault<'info> {
    /// CHECK: La cuenta `vault` es un PDA con seed ["vault"] y su bump
    /// se valida en `ctx.bumps.vault`.
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Battle<'info> {
    #[account(
        mut,
        seeds = [b"joueur", attacker_owner.key().as_ref()],
        bump
    )]
    pub attacker: Account<'info, Joueur>,
    #[account(mut)]
    pub attacker_owner: Signer<'info>,

    #[account(
        mut,
        constraint = attacker.key() != defender.key() @ ErrorCode::InvalidArgument
    )]
    pub defender: Account<'info, Joueur>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Joueur {
    pub user_address: Pubkey,
    #[max_len(50)]
    pub pseudonyme: String,
    pub pdv: u64,
    pub xp: u64,
    pub vivant: bool,
    pub level: Level,
}

#[derive(InitSpace, AnchorDeserialize, AnchorSerialize, Debug, Clone, Copy)]
pub enum Level {
    Beginner,
    Explorer,
    Champion,
    Legend,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Le joueur est mort")]
    JoueurMort,
    #[msg("Expérience insuffisante")]
    ExperienceInsuffisante,
    #[msg("Vous n'êtes pas autorisé à effectuer cette action")]
    NonAdmin,
    #[msg("Argument invalide")]
    InvalidArgument,
}
