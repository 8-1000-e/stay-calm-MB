use bolt_lang::*;
use game_config::GameConfig;
use player_state::PlayerState;
use player_registry::PlayerRegistry;
use shared::{GameError, MAX_ATTEMPTS};

declare_id!("AmkrpcXWp54TtJHPSVXvrMCo6dkdXwq1q2BbvR3mJKUY");

/// Spawns a player into a Waiting match. Initializes their PlayerState
/// (`MAX_ATTEMPTS` LOCKs available, score = 0, no live attempt) and
/// pushes them onto the PlayerRegistry parallel arrays so the keeper /
/// leaderboard refreshers can iterate every active player from a single
/// fixed-size component.
///
/// remaining_accounts: the LAST extra account is the player's wallet
/// pubkey (used to set `PlayerState.owner` so the on-chain leaderboard
/// keys against the real wallet, not the BOLT entity authority).
#[system]
pub mod spawn_player {

    pub fn execute(ctx: Context<Components>, _args_p: Vec<u8>) -> Result<Components> {
        require!(ctx.accounts.game_config.status == 0, GameError::GameNotWaiting);
        let idx = ctx.accounts.player_registry.count as usize;
        require!(idx < player_registry::MAX_PLAYERS, GameError::TooManyPlayers);

        let owner_idx = ctx.remaining_accounts.len() - 1;
        ctx.accounts.player_state.authority = *ctx.accounts.authority.key;
        ctx.accounts.player_state.owner = *ctx.remaining_accounts[owner_idx].key;

        ctx.accounts.player_registry.players[idx] = ctx.accounts.player_state.owner.to_bytes();
        ctx.accounts.player_registry.player_states[idx] =
            ctx.accounts.player_state.key().to_bytes();
        ctx.accounts.player_registry.count += 1;
        ctx.accounts.game_config.active_players += 1;

        ctx.accounts.player_state.attempts_left = MAX_ATTEMPTS;
        ctx.accounts.player_state.score = 0;
        ctx.accounts.player_state.points_this_round = 0;
        ctx.accounts.player_state.leverage = 0;
        ctx.accounts.player_state.low_price = 0;
        ctx.accounts.player_state.high_price = 0;
        ctx.accounts.player_state.attempt_end_ts = 0;
        ctx.accounts.player_state.last_block = 0;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub player_state: PlayerState,
        pub game_config: GameConfig,
        pub player_registry: PlayerRegistry,
    }
}
