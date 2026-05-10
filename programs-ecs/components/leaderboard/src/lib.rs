use bolt_lang::*;

declare_id!("Gm6tpszWzqzetoAHvTvxyBWAY4wYdREj9mmper7BPrZa");

pub const MAX_LEADERBOARD: usize = 10;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct LeaderboardEntry {
    pub pubkey: [u8; 32],
    /// Sort key: balance + locked margin + unrealized_pnl, in lamports.
    pub net_worth: i64,
    /// Free cash on hand (lamports).
    pub balance: u64,
    /// Flying PnL on the open position (lamports, signed).
    pub unrealized_pnl: i64,
    /// Realized PnL across closed trades (lamports).
    pub realized_pnl: i64,
    pub alive: bool,
}

impl Default for LeaderboardEntry {
    fn default() -> Self {
        Self {
            pubkey: [0u8; 32],
            net_worth: 0,
            balance: 0,
            unrealized_pnl: 0,
            realized_pnl: 0,
            alive: false,
        }
    }
}

/// `entries` is a Vec rather than a fixed array so Borsh can deserialize
/// element-by-element on the stack (~65 bytes per element) instead of
/// allocating the whole array on stack at once. Combined with
/// `Box<Account<Leaderboard>>` on the consumer side, this keeps `update`
/// and `bolt_execute` inside the BPF 4 KB stack frame budget. Cap mirrors
/// PlayerRegistry's MAX_PLAYERS (currently 10) — see that component for the
/// 1024-byte return-data rationale.
///
/// `Default` pre-fills the Vec with `MAX_LEADERBOARD` default entries so
/// systems can keep using positional access (`entries[i] = …`) — same
/// semantics as the original fixed array.
#[component(delegate)]
pub struct Leaderboard {
    /// Entries sorted by net_worth descending (best PnL on top).
    #[max_len(MAX_LEADERBOARD)]
    pub entries: Vec<LeaderboardEntry>,
    pub count: u8,
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self {
            entries: vec![LeaderboardEntry::default(); MAX_LEADERBOARD],
            count: 0,
            bolt_metadata: BoltMetadata::default(),
        }
    }
}
