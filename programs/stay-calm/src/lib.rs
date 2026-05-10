use bolt_lang::prelude::*;

declare_id!("FoyBPXV4XEFESAgvFToMuHK3pyP66m5bjZGmEAM9mBhs");

#[program]
pub mod stay_calm {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
