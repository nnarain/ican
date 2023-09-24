//
// mod.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Aug 17 2022
//

pub mod monitor;

use clap::Subcommand;

use crate::CommandContext;

/// CANopen subcommands
#[derive(Debug, Subcommand)]
pub enum CanOpenCommands {
    /// Monitor CAN traffic and decode CANopen data
    Monitor(monitor::Args),
}

pub async fn run(cmd: CanOpenCommands, ctx: CommandContext) -> anyhow::Result<()> {
    match cmd {
        CanOpenCommands::Monitor(args) => Ok(monitor::run(args, ctx).await?),
    }
}
