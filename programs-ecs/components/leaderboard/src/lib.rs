use bolt_lang::*;

declare_id!("B3cei3GugJWu5xER2gixCQuc9AKC6qPksNxyLj6XggJN");

pub const MAX_LEADERBOARD: usize = 10;

/// One row of the on-chain leaderboard. Stays small (41 bytes) so the
/// BOLT `update` ix + the consuming systems can keep `entries` on the
/// stack without busting the 4 KB BPF frame budget — see the comment on
/// `Leaderboard.entries` for why we use a Vec.
///
/// Sort key is `score` descending (highest first); `attempts_left` is
/// informational (shown as "2/3" next to the player on the front so the
/// viewer can see who's still got tries left) and isn't used for sorting.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct LeaderboardEntry {
    /// Player wallet, stored as raw 32 bytes (avoids Anchor's Pubkey
    /// alignment quirks when the consumer reads us as a flat byte slice).
    pub pubkey: [u8; 32],
    /// Cumulative banked score across resolved attempts in the match.
    /// Mirrors `PlayerState.score`. Sort key — leaderboard is sorted
    /// descending by this field.
    pub score: u64,
    /// LOCK attempts the player has left (0..=3). Mirrors
    /// `PlayerState.attempts_left`. Display-only; doesn't affect ordering.
    pub attempts_left: u8,
}

impl Default for LeaderboardEntry {
    fn default() -> Self {
        Self {
            pubkey: [0u8; 32],
            score: 0,
            attempts_left: 0,
        }
    }
}

/// `entries` is a Vec rather than a fixed array so Borsh can deserialize
/// element-by-element on the stack (~41 bytes per element) instead of
/// allocating the whole array on stack at once. Combined with
/// `Box<Account<Leaderboard>>` on the consumer side, this keeps `update`
/// and `bolt_execute` inside the BPF 4 KB stack frame budget. Cap mirrors
/// PlayerRegistry's MAX_PLAYERS (currently 10) — see that component for
/// the 1024-byte return-data rationale.
///
/// `Default` pre-fills the Vec with `MAX_LEADERBOARD` default entries so
/// systems can keep using positional access (`entries[i] = …`) — same
/// semantics as the original fixed array.
#[component(delegate)]
pub struct Leaderboard {
    /// Entries sorted by `score` descending (highest banked score on top).
    #[max_len(MAX_LEADERBOARD)]
    pub entries: Vec<LeaderboardEntry>,
    /// Number of valid (non-default) entries — refresh-leaderboard writes
    /// up to this index, the rest of the Vec stays zeroed.
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
