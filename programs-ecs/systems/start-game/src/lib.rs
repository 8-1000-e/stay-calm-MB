use bolt_lang::*;
use game_config::GameConfig;
use shared::{GameError, GAME_DURATION_SEC, MIN_PLAYERS};

declare_id!("4TXxJkPVphdVQkHo17RyNDLeVTBh3TRMBxooHghgcznQ");

/// Closes the lobby and flips the match to Playing. Gates on the lobby
/// being open (`status == Waiting`), at least `MIN_PLAYERS` having
/// spawned, AND `now >= min_start_time` so the lobby window had a
/// chance to fill. Stamps `game_end = now + GAME_DURATION_SEC`.
///
/// Back-only by Bolt design — only the back's keypair has signing
/// authority over the GameConfig entity, so no explicit authority guard
/// is needed (mirrors red-light / trade-fight).
#[system]
pub mod start_game {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> {
        require!(ctx.accounts.game_config.status == 0, GameError::GameNotWaiting);
        require!(
            ctx.accounts.game_config.active_players >= MIN_PLAYERS,
            GameError::NotEnoughPlayers
        );
        let now = Clock::get()?.unix_timestamp;
        require!(
            now >= ctx.accounts.game_config.min_start_time,
            GameError::LobbyNotOver
        );
        ctx.accounts.game_config.status = 1;
        ctx.accounts.game_config.game_end = now + GAME_DURATION_SEC;
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
    }
}
