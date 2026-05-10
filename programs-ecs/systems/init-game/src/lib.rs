use bolt_lang::*;
use game_config::GameConfig;
use shared::LOBBY_DURATION_SEC;

declare_id!("6HbXyVdNJihLBzZaGibWWqo93a4uXQeVyi3efWyhhakw");

#[system]
pub mod init_game {
    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> 
    {
        ctx.accounts.game_config.status = 0;
        ctx.accounts.game_config.active_players = 0;
        ctx.accounts.game_config.min_start_time = Clock::get()?.unix_timestamp + LOBBY_DURATION_SEC;
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub game_config: GameConfig,
    }
}
