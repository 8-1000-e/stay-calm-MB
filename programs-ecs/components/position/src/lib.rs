use bolt_lang::*;

declare_id!("YzrB4KFX4VQMmwxneogLD9AwdpKfeC5T9Q8dHDtbLvm");

#[component]
#[derive(Default)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
    #[max_len(20)]
    pub description: String,
}