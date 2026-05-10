use anchor_lang::prelude::*;

#[error_code]
pub enum LobbyError {
    #[msg("Lobby is not open for joining")]
    LobbyNotOpen,
    #[msg("Lobby is full")]
    LobbyFull,
    #[msg("Player already joined this lobby")]
    AlreadyJoined,
    #[msg("Lobby has not started yet")]
    LobbyNotStarted,
    #[msg("Lobby is already settled")]
    AlreadySettled,
    #[msg("Only the lobby authority can perform this action")]
    Unauthorized,
    #[msg("Leaderboard account owner does not match expected program")]
    InvalidLeaderboardOwner,
    #[msg("Leaderboard account is too small to decode")]
    LeaderboardTooSmall,
    #[msg("Leaderboard is empty — no winners to pay")]
    EmptyLeaderboard,
    #[msg("Winner account does not match leaderboard entry at that index")]
    LeaderboardMismatch,
    #[msg("Not enough accounts passed for distribution")]
    NotEnoughAccounts,
    #[msg("Treasury account does not match lobby authority")]
    InvalidTreasury,
    #[msg("Vault balance insufficient for computed payout (should not happen)")]
    VaultUnderflow,
    #[msg("Player is not in this lobby")]
    NotInLobby,
    #[msg("Lobby is locked — minimum-player threshold reached, no more leaves")]
    LobbyLocked,
}
