use bolt_lang::*;

declare_id!("Eg1TPqh2JQKZPiibUjVgkg4ci6eanci2XV36mAwJxZHV");

#[component(delegate)]
pub struct GameConfig {
    /// 0 = Waiting (lobby open), 1 = Playing, 2 = Finished
    pub status: u8,
    /// Number of players currently in the lobby / game
    pub active_players: u8,
    /// Game start (lobby open) timestamp — set by init-game
    pub min_start_time: i64,
    /// Game ends at this timestamp (set by start-game = now + GAME_DURATION_SEC)
    pub game_end: i64,
    /// ID of the lobby PDA this match was launched from. Set by `init-game`,
    /// read by the lobby program's `distribute_prize` to verify that the
    /// Leaderboard account passed in actually corresponds to the match
    /// that's being settled — without this binding the back could (in
    /// theory) front a stale/foreign leaderboard and skim the pot.
    pub lobby_id: u64,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            status: 0,
            active_players: 0,
            min_start_time: 0,
            game_end: 0,
            lobby_id: 0,
            bolt_metadata: BoltMetadata::default(),
        }
    }
}
