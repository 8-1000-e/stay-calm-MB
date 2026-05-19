use anchor_lang::prelude::*;

// PDA seeds
pub const LOBBY_SEED: &[u8] = b"lobby";
pub const VAULT_SEED: &[u8] = b"vault";

// Caps
pub const MAX_PLAYERS: usize = 10;
/// Minimum players for a match to launch — once the lobby reaches this
/// count, the back's tick will start the launch flow and `leave_lobby`
/// is rejected on-chain (`LobbyLocked`). Mirrors the back's `MIN_PLAYERS`
/// in `stay-calm-chain.service.ts` (when wired).
pub const MIN_PLAYERS: usize = 2;

// Platform rake, in basis points (100 bps = 1%)
pub const PLATFORM_FEE_BPS: u64 = 500; // 5%

// Fee charged on leave to prevent join/leave spam (covers back's tx fee)
pub const LEAVE_FEE: u64 = 200_000; // 0.0001 SOL

// Lobby status values
pub const STATUS_OPEN: u8 = 0;
pub const STATUS_STARTED: u8 = 1;
pub const STATUS_SETTLED: u8 = 2;

// Stay-calm `leaderboard` BOLT component program ID — used to verify that
// the leaderboard account passed to distribute_prize is the real one.
// Must match declare_id! in programs-ecs/components/leaderboard/src/lib.rs.
pub const LEADERBOARD_COMPONENT_ID: Pubkey =
    pubkey!("B3cei3GugJWu5xER2gixCQuc9AKC6qPksNxyLj6XggJN");

// Layout of the BOLT Leaderboard account we decode manually.
// `entries` is a Vec (Borsh: 4-byte LE length prefix then elements). Default
// pre-fills the Vec with MAX_LEADERBOARD entries so the length prefix is
// always MAX_PLAYERS.
//
// Entry: pubkey(32) + score(u64=8) + attempts_left(u8=1) = 41 bytes
//
//   [0..8]                                anchor discriminator
//   [8..12]                               entries.len() (u32 LE) = MAX_PLAYERS
//   [12..(12 + 41*MAX_PLAYERS)]           entries
//   [12 + 41*MAX_PLAYERS]                 count: u8
pub const LEADERBOARD_DISC_LEN: usize = 8;
pub const LEADERBOARD_VEC_LEN_PREFIX: usize = 4;
pub const LEADERBOARD_ENTRIES_OFFSET: usize = LEADERBOARD_DISC_LEN + LEADERBOARD_VEC_LEN_PREFIX;
pub const LEADERBOARD_ENTRY_SIZE: usize = 41;
// `score` lives right after the 32-byte pubkey. Distribute_prize reads it
// as i64 (legacy from the trade-fight port) — for stay-calm `score` is u64
// but the byte layout is identical for non-negative values, and the on-
// chain comparison logic (`==` for ties) works either way.
pub const LEADERBOARD_ENTRY_SCORE_OFFSET: usize = 32;
pub const LEADERBOARD_ENTRIES_LEN: usize = LEADERBOARD_ENTRY_SIZE * MAX_PLAYERS;
pub const LEADERBOARD_COUNT_OFFSET: usize = LEADERBOARD_ENTRIES_OFFSET + LEADERBOARD_ENTRIES_LEN;
