use bolt_lang::*;
use game_config::GameConfig;
use player_state::PlayerState;
use player_registry::PlayerRegistry;
use shared::GameError;

declare_id!("AmkrpcXWp54TtJHPSVXvrMCo6dkdXwq1q2BbvR3mJKUY");

#[system]
pub mod spawn_player {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> 
    {
        require!(ctx.accounts.game_config.status == 0, GameError::GameNotWaiting);
        let idx = ctx.accounts.player_registry.count as usize;
        require!(idx < player_registry::MAX_PLAYERS, GameError::TooManyPlayers);

        let owner_idx = ctx.remaining_accounts.len() - 1;
        ctx.accounts.player_state.authority = *ctx.accounts.authority.key;
        ctx.accounts.player_state.owner = *ctx.remaining_accounts[owner_idx].key;

        ctx.accounts.player_state.alive = true;
        ctx.accounts.player_state.balance = shared::STARTING_BALANCE;
        ctx.accounts.player_state.position = shared::POS_FLAT;
        ctx.accounts.player_state.leverage = 0;
        ctx.accounts.player_state.entry_price = 0;
        ctx.accounts.player_state.position_size = 0;
        ctx.accounts.player_state.realized_pnl = 0;
        ctx.accounts.player_state.opened_at = 0;

        ctx.accounts.player_registry.players[idx] = ctx.accounts.player_state.owner.to_bytes();
        ctx.accounts.player_registry.player_states[idx] = ctx.accounts.player_state.key().to_bytes();
        ctx.accounts.player_registry.count += 1;
        ctx.accounts.game_config.active_players += 1;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub player_state: PlayerState,
        pub game_config: GameConfig,
        pub player_registry: PlayerRegistry,
    }
}
