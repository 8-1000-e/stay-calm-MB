use anchor_lang::prelude::*;
use crate::constants::MAX_PLAYERS;

/// Lobby metadata. Seeds: ["lobby", lobby_id_le_bytes].
/// Rent-exempt, owned by this program.
#[account]
pub struct Lobby {
    pub lobby_id: u64,
    /// Server keypair — the only one allowed to start/distribute/close.
    pub authority: Pubkey,
    /// Entry fee in lamports. Each `join_lobby` transfers this to the vault.
    pub entry_fee: u64,
    /// Number of registered players.
    pub player_count: u8,
    /// Registered player wallets. Indexes beyond `player_count` are zeroed.
    pub players: [Pubkey; MAX_PLAYERS],
    /// 0 = Open, 1 = Started, 2 = Settled. See `constants.rs`.
    pub status: u8,
    /// Unix timestamp when the lobby was created.
    pub created_at: i64,
    /// Unix timestamp when start_match was called (0 otherwise).
    pub started_at: i64,
    pub bump: u8,
}

impl Lobby {
    /// 8 (disc) + 8 (id) + 32 (auth) + 8 (fee) + 1 (count) + 32*10 (players)
    /// + 1 (status) + 8 (created) + 8 (started) + 1 (bump) = 395.
    pub const LEN: usize = 8 + 8 + 32 + 8 + 1 + (32 * MAX_PLAYERS) + 1 + 8 + 8 + 1;
}

/// Vault PDA that physically holds the entry-fee lamports.
/// Seeds: ["vault", lobby.key()]. Owned by this program so we can
/// sub_lamports() directly in distribute_prize.
#[account]
pub struct Vault {
    pub lobby: Pubkey,
    pub total_pot: u64,
    pub bump: u8,
}

impl Vault {
    /// 8 (disc) + 32 (lobby) + 8 (pot) + 1 (bump) = 49.
    pub const LEN: usize = 8 + 32 + 8 + 1;
}
