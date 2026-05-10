use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::constants::*;
use crate::errors::*;
use crate::state::*;

pub fn join_lobby(ctx: Context<JoinLobby>, _lobby_id: u64) -> Result<()> {
    let lobby = &mut ctx.accounts.lobby;

    require!(lobby.status == STATUS_OPEN, LobbyError::LobbyNotOpen);
    require!((lobby.player_count as usize) < MAX_PLAYERS, LobbyError::LobbyFull);

    let player_key = ctx.accounts.player.key();
    let already_in = lobby.players[..lobby.player_count as usize]
        .iter()
        .any(|p| p == &player_key);
    require!(!already_in, LobbyError::AlreadyJoined);

    // 1. Transfer entry fee player → vault (CPI to SystemProgram)
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: ctx.accounts.player.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        },
    );
    system_program::transfer(cpi_ctx, lobby.entry_fee)?;

    // 2. Register player on-chain
    let idx = lobby.player_count as usize;
    lobby.players[idx] = player_key;
    lobby.player_count += 1;

    // 3. Update vault bookkeeping
    let vault = &mut ctx.accounts.vault;
    vault.total_pot = vault
        .total_pot
        .checked_add(lobby.entry_fee)
        .ok_or(LobbyError::VaultUnderflow)?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(lobby_id: u64)]
pub struct JoinLobby<'info> {
    #[account(
        mut,
        seeds = [LOBBY_SEED, lobby_id.to_le_bytes().as_ref()],
        bump = lobby.bump,
    )]
    pub lobby: Box<Account<'info, Lobby>>,

    #[account(
        mut,
        seeds = [VAULT_SEED, lobby.key().as_ref()],
        bump = vault.bump,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// The player joining — signs to authorize the entry_fee transfer.
    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}
