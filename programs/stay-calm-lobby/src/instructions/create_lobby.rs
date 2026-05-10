use anchor_lang::prelude::*;
use crate::constants::*;
use crate::state::*;

pub fn create_lobby(ctx: Context<CreateLobby>, lobby_id: u64, entry_fee: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let lobby = &mut ctx.accounts.lobby;
    lobby.lobby_id = lobby_id;
    lobby.authority = ctx.accounts.authority.key();
    lobby.entry_fee = entry_fee;
    lobby.player_count = 0;
    lobby.players = [Pubkey::default(); MAX_PLAYERS];
    lobby.status = STATUS_OPEN;
    lobby.created_at = now;
    lobby.started_at = 0;
    lobby.bump = ctx.bumps.lobby;

    let vault = &mut ctx.accounts.vault;
    vault.lobby = ctx.accounts.lobby.key();
    vault.total_pot = 0;
    vault.bump = ctx.bumps.vault;

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct CreateLobby<'info> {
    #[account(
        init,
        payer = authority,
        space = Lobby::LEN,
        seeds = [LOBBY_SEED, lobby_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub lobby: Box<Account<'info, Lobby>>,

    #[account(
        init,
        payer = authority,
        space = Vault::LEN,
        seeds = [VAULT_SEED, lobby.key().as_ref()],
        bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// The backend server keypair — becomes the lobby authority.
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}
