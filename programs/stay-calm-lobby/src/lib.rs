use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("874rb8bkqaQmYPT3DxSYGbPjrfHzUa2AcyuDGJuXC1AZ");

#[event]
pub struct PrizeDistributed {
    pub lobby_id: u64,
    pub total_pot: u64,
    pub treasury_cut: u64,
    pub winner_count: u8,
    pub winner_pubkeys: Vec<Pubkey>,
    pub winner_amounts: Vec<u64>,
}

#[program]
pub mod stay_calm_lobby {
    use super::*;

    /// Back creates a new lobby. Opens it for joining.
    pub fn create_lobby(ctx: Context<CreateLobby>, lobby_id: u64, entry_fee: u64) -> Result<()> {
        instructions::create_lobby::create_lobby(ctx, lobby_id, entry_fee)
    }

    /// A player joins an open lobby and pays the entry fee into the vault.
    pub fn join_lobby(ctx: Context<JoinLobby>, lobby_id: u64) -> Result<()> {
        instructions::join_lobby::join_lobby(ctx, lobby_id)
    }

    /// Back locks the lobby so no more players can join — called right before
    /// launching the BOLT match.
    pub fn start_match(ctx: Context<StartMatch>, lobby_id: u64) -> Result<()> {
        instructions::start_match::start_match(ctx, lobby_id)
    }

    /// Back distributes the vault to winners + treasury. The leaderboard
    /// account is passed and decoded on-chain; remaining_accounts must match
    /// leaderboard.entries in order (anti-scam).
    ///
    /// remaining_accounts layout:
    ///   [0..count]  winner wallets in leaderboard order (sorted by net_worth DESC)
    pub fn distribute_prize(ctx: Context<DistributePrize>, lobby_id: u64, player_count: u8) -> Result<()> {
        instructions::distribute_prize::distribute_prize(ctx, lobby_id, player_count)
    }

    /// Player leaves an open lobby — refund entry fee. Called by backend (JWT-verified).
    pub fn leave_lobby(ctx: Context<LeaveLobby>, lobby_id: u64, player: Pubkey) -> Result<()> {
        instructions::leave_lobby::leave_lobby(ctx, lobby_id, player)
    }

    /// Refund all players from the vault when a match launch fails.
    /// remaining_accounts: player wallets in lobby.players[] order.
    pub fn refund_lobby(ctx: Context<RefundLobby>, lobby_id: u64) -> Result<()> {
        instructions::refund_lobby::refund_lobby(ctx, lobby_id)
    }

    /// Back closes a settled lobby and reclaims the rent from Lobby + Vault.
    pub fn close_lobby(ctx: Context<CloseLobby>, lobby_id: u64) -> Result<()> {
        instructions::close_lobby::close_lobby(ctx, lobby_id)
    }
}
