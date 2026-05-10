use bolt_lang::*;
use game_config::GameConfig;
use shared::GAME_DURATION_SEC;
use shared::GameError;

declare_id!("4TXxJkPVphdVQkHo17RyNDLeVTBh3TRMBxooHghgcznQ");

/// Closes the lobby and switches the game to Playing. Sets the game timer.
#[system]
pub mod start_game {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> 
    {
        require!(ctx.accounts.game_config.status == 0, GameError::GameNotWaiting);
        let now = Clock::get()?.unix_timestamp;
        // The back enforces the 60s lobby countdown off-chain (between the
        // first 2 players joining and `launchMatch` firing). Re-checking the
        // same delay here would force a useless 60s gap between init_game
        // and start_game inside the same `createMatch` flow — same calls
        // run back-to-back. Mirrors red-light's choice (line 17 of its
        // start-game/src/lib.rs is also commented out for the same reason).
        // require!(now >= ctx.accounts.game_config.min_start_time, GameError::LobbyNotOver);
        ctx.accounts.game_config.status = 1;
        ctx.accounts.game_config.game_end = now + GAME_DURATION_SEC;
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
    }
}
