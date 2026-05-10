use bolt_lang::*;
use game_config::GameConfig;
use shared::LOBBY_DURATION_SEC;

declare_id!("6HbXyVdNJihLBzZaGibWWqo93a4uXQeVyi3efWyhhakw");

/// Initializes a new match's GameConfig — back-signed, called right after
/// the lobby program launches the match. Sets `status = Waiting`, opens
/// the join window for `LOBBY_DURATION_SEC`, and stamps `lobby_id` so
/// the lobby program's `distribute_prize` can verify the leaderboard
/// it's settling actually corresponds to this match.
///
/// args: JSON `{"lobby_id": <u64>}`
#[system]
pub mod init_game {
    pub fn execute(ctx: Context<Components>, args_p: Vec<u8>) -> Result<Components> {
        ctx.accounts.game_config.status = 0;
        ctx.accounts.game_config.active_players = 0;
        ctx.accounts.game_config.min_start_time =
            Clock::get()?.unix_timestamp + LOBBY_DURATION_SEC;
        ctx.accounts.game_config.game_end = 0;
        ctx.accounts.game_config.lobby_id = shared::parse_json_u64(&args_p, b"lobby_id");

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
    }
}
