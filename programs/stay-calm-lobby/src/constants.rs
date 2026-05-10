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
// TODO: replace with the deployed leaderboard program ID after the first
// `bolt build` pins it.
pub const LEADERBOARD_COMPONENT_ID: Pubkey =
    pubkey!("11111111111111111111111111111111");

// Layout of the BOLT Leaderboard account we decode manually.
// `entries` is a Vec (Borsh: 4-byte LE length prefix then elements) — switched
// from a fixed array so the auto-generated `update` ix and consuming systems
// stay under the BPF 4 KB stack budget. The Vec is pre-filled to MAX_PLAYERS
// entries by Default, so the prefix is always MAX_PLAYERS.
//
// Entry: pubkey(32) + net_worth(i64=8) + balance(u64=8) + unrealized_pnl(i64=8)
//      + realized_pnl(i64=8) + alive(1) = 65 bytes
//
//   [0..8]                                anchor discriminator
//   [8..12]                               entries.len() (u32 LE) = MAX_PLAYERS
//   [12..(12 + 65*MAX_PLAYERS)]           entries
//   [12 + 65*MAX_PLAYERS]                 count: u8
pub const LEADERBOARD_DISC_LEN: usize = 8;
pub const LEADERBOARD_VEC_LEN_PREFIX: usize = 4;
pub const LEADERBOARD_ENTRIES_OFFSET: usize = LEADERBOARD_DISC_LEN + LEADERBOARD_VEC_LEN_PREFIX;
pub const LEADERBOARD_ENTRY_SIZE: usize = 65;
pub const LEADERBOARD_ENTRY_NET_WORTH_OFFSET: usize = 32; // i64, 8 bytes LE
pub const LEADERBOARD_ENTRIES_LEN: usize = LEADERBOARD_ENTRY_SIZE * MAX_PLAYERS;
pub const LEADERBOARD_COUNT_OFFSET: usize = LEADERBOARD_ENTRIES_OFFSET + LEADERBOARD_ENTRIES_LEN;
