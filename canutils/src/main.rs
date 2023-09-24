use anyhow::Context;
//
// main.rs
//
// @author Natesh Narain <nnaraindev@gmail.com>
// @date Jul 15 2022
//

use socketcan::tokio::CanSocket;
use clap::Parser;

use canutils::{CommandContext, Args, Command, action};

use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args = Args::parse();
    let device = args.device;
    let tick_rate = args.tick_rate;

    let socket = CanSocket::open(&device)
                                .with_context(|| format!("Failed to open CAN interface {}", device))?;

    let ctx = CommandContext {socket, device, tick_rate};

    match args.cmd {
        Command::Dump => Ok(action::dump::run(ctx).await?),
        Command::Monitor => Ok(action::monitor::run(ctx).await?),
        Command::Send(args) => Ok(action::send::run(ctx, args).await?),
        Command::Bridge => Ok(()),
        // Command::Canopen(cmd) => Ok(action::canopen::run(cmd, ctx).await?)
    }
}
