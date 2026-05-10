use bolt_lang::*;

declare_id!("DLWJtGTeytHa1t94pa1XPcrWucXeYJbfxPNDMhEJGAP1");

pub const MAX_PLAYERS: usize = 10;

/// Mirrors red-light's registry: parallel arrays of player authority pubkeys
/// and their corresponding PlayerState component PDAs.
///
/// Stored as `Vec` rather than fixed arrays so Borsh can deserialize
/// element-by-element on the stack (32 bytes temp per element) instead of
/// allocating the whole array at once — keeps the auto-generated
/// `update` / `update_with_session` and the systems that read this component
/// inside the BPF 4 KB stack frame budget. Currently capped at 10 to stay
/// under the 1024-byte Bolt return-data limit when `spawn_player` writes
/// PlayerState + GameConfig + PlayerRegistry in a single CPI return.
///
/// `Default` pre-fills both Vecs with `MAX_PLAYERS` zeroed entries so systems
/// can keep using positional writes (`players[idx] = …`) — same semantics as
/// the original fixed arrays. `count` still tracks how many slots are in use.
#[component(delegate)]
pub struct PlayerRegistry {
    #[max_len(MAX_PLAYERS)]
    pub players: Vec<[u8; 32]>,
    #[max_len(MAX_PLAYERS)]
    pub player_states: Vec<[u8; 32]>,
    pub count: u8,
}

impl Default for PlayerRegistry {
    fn default() -> Self {
        Self {
            players: vec![[0u8; 32]; MAX_PLAYERS],
            player_states: vec![[0u8; 32]; MAX_PLAYERS],
            count: 0,
            bolt_metadata: BoltMetadata::default(),
        }
    }
}
